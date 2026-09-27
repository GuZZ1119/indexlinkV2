//! SQLite adapter for the local paper-trading performance ledger.

use std::collections::HashMap;

use broker::{
    BrokerOrderAck, BrokerOrderRequest, BrokerOrderSide, PaperOrder, PaperOrderState,
    PaperPortfolioSnapshot,
};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

const SCALE: u32 = 8;
const INTEGER_DIGITS: usize = 12;

/// Plan information required to calculate one tracked paper-performance series.
#[derive(Debug, Clone)]
pub struct PaperPerformancePlan {
    /// Local investment-plan identifier.
    pub id: Uuid,
    /// Normalized tracked symbol.
    pub symbol: String,
    /// Display currency.
    pub currency: String,
    /// Plain-DCA contribution used for each observed buy execution.
    pub base_contribution: Decimal,
}

/// One local, auditable paper-performance point.
#[derive(Debug, Clone, Serialize)]
pub struct PaperPerformancePoint {
    /// UTC RFC3339 timestamp at which the broker was read.
    pub observed_at: String,
    /// Locally reconstructed adaptive strategy value.
    #[serde(with = "rust_decimal::serde::str")]
    pub adaptive_value: Decimal,
    /// Hypothetical plain-DCA value at the same observed price.
    #[serde(with = "rust_decimal::serde::str")]
    pub plain_dca_value: Decimal,
    /// User-confirmed opening balance plus later local cash flows.
    #[serde(with = "rust_decimal::serde::str")]
    pub net_contributions: Decimal,
}

/// 一笔已由本地账本确认的模拟成交标记。
#[derive(Debug, Clone, Serialize)]
pub struct PaperTradeMarker {
    /// 本地归属的定投标的 ID。
    pub plan_id: Uuid,
    /// 买入或卖出方向。
    pub side: String,
    /// 成交量。
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    /// 本地观察到的成交均价。
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    /// 本地观察到成交状态变化的 UTC RFC3339 时间。
    pub observed_at: String,
}

/// Current local paper-performance result for one plan.
#[derive(Debug, Clone, Serialize)]
pub struct PaperPerformance {
    /// Currency of all returned amounts.
    pub currency: String,
    /// Whether a user-confirmed opening balance exists.
    pub has_opening_balance: bool,
    /// Whether local fills reconcile with the current provider position.
    pub data_complete: bool,
    /// Total user-confirmed opening balance, deposits and withdrawals.
    #[serde(with = "rust_decimal::serde::str")]
    pub net_contributions: Decimal,
    /// Current value of the locally tracked adaptive strategy.
    #[serde(with = "rust_decimal::serde::str")]
    pub adaptive_value: Decimal,
    /// Current value of the same-date plain-DCA benchmark.
    #[serde(with = "rust_decimal::serde::str")]
    pub plain_dca_value: Decimal,
    /// FIFO realized profit or loss from locally observed fills.
    #[serde(with = "rust_decimal::serde::str")]
    pub realized_pnl: Decimal,
    /// Mark-to-market unrealized profit or loss from locally observed fills.
    #[serde(with = "rust_decimal::serde::str")]
    pub unrealized_pnl: Decimal,
    /// Adaptive value minus net contributions when a baseline is configured.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub total_return: Option<Decimal>,
    /// Ordered local snapshots for charting.
    pub points: Vec<PaperPerformancePoint>,
}

/// Safe storage failure for the paper-performance ledger.
#[derive(Debug, thiserror::Error)]
pub enum PaperPerformanceError {
    /// Caller supplied invalid local ledger input.
    #[error("invalid paper performance input")]
    InvalidInput,
    /// SQLite could not read or write the local ledger.
    #[error("paper performance storage is unavailable")]
    Unavailable,
}

/// SQLite repository for the local-only paper-performance ledger.
#[derive(Clone, Debug)]
pub struct SqlitePaperPerformanceRepository {
    pool: SqlitePool,
}

impl SqlitePaperPerformanceRepository {
    /// Build the repository from an existing migrated SQLite pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Store or replace the one user-confirmed opening balance for a plan.
    pub async fn set_opening_balance(
        &self,
        plan_id: Uuid,
        amount: Decimal,
        occurred_at: &str,
    ) -> Result<(), PaperPerformanceError> {
        let amount = encode_non_negative(amount).ok_or(PaperPerformanceError::InvalidInput)?;
        if !is_millisecond_utc(occurred_at) {
            return Err(PaperPerformanceError::InvalidInput);
        }
        sqlx::query(
            "INSERT INTO cash_flows (id, plan_id, kind, amount, occurred_at) \
             VALUES (?1, ?2, 'opening_balance', ?3, ?4) \
             ON CONFLICT(plan_id) WHERE kind = 'opening_balance' \
             DO UPDATE SET amount = excluded.amount, occurred_at = excluded.occurred_at",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(plan_id.to_string())
        .bind(amount)
        .bind(occurred_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    /// Record a broker-accepted paper-order intention before it can later fill.
    pub async fn record_accepted_order(
        &self,
        plan_id: Uuid,
        decision_record_id: Uuid,
        acknowledgement: &BrokerOrderAck,
        request: &BrokerOrderRequest,
    ) -> Result<(), PaperPerformanceError> {
        let quantity =
            encode_positive(request.quantity()).ok_or(PaperPerformanceError::InvalidInput)?;
        sqlx::query(
            "INSERT INTO paper_orders \
             (order_id, plan_id, decision_record_id, symbol, side, requested_quantity, state, filled_quantity, average_fill_price, submitted_at, observed_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', '000000000000.00000000', '000000000000.00000000', \
                     strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
             ON CONFLICT(order_id) DO NOTHING",
        )
        .bind(acknowledgement.order_id())
        .bind(plan_id.to_string())
        .bind(decision_record_id.to_string())
        .bind(request.symbol())
        .bind(side_name(request.side()))
        .bind(quantity)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    /// Reconcile locally known orders with a read-only OpenD snapshot and return performance.
    pub async fn refresh(
        &self,
        plan: &PaperPerformancePlan,
        portfolio: &PaperPortfolioSnapshot,
    ) -> Result<PaperPerformance, PaperPerformanceError> {
        if portfolio.currency != plan.currency {
            return Err(PaperPerformanceError::InvalidInput);
        }
        self.sync_orders(plan.id, &portfolio.orders).await?;
        let ledger = self.ledger(plan.id, plan.base_contribution).await?;
        let position = portfolio
            .positions
            .iter()
            .find(|item| item.symbol == plan.symbol);
        let price = position.map_or(Decimal::ZERO, |item| item.price);
        let provider_quantity = position.map_or(Decimal::ZERO, |item| item.quantity);
        let market_value = position.map_or(Decimal::ZERO, |item| item.market_value);
        let data_complete = ledger.has_opening_balance
            && ledger.unmatched_sell.is_zero()
            && ledger.quantity == provider_quantity;
        let adaptive_value = (ledger.cash + market_value).max(Decimal::ZERO);
        let unrealized_pnl = market_value - ledger.open_cost;
        let plain_dca_value = ledger.plain_units * price;
        let total_return = ledger
            .has_opening_balance
            .then_some(adaptive_value - ledger.net_contributions);

        sqlx::query(
            "INSERT INTO portfolio_snapshots \
             (id, plan_id, currency, symbol_price, tracked_quantity, adaptive_value, plain_dca_value, net_contributions, realized_pnl, observed_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(plan.id.to_string())
        .bind(&plan.currency)
        .bind(encode_non_negative(price).ok_or(PaperPerformanceError::Unavailable)?)
        .bind(encode_non_negative(ledger.quantity).ok_or(PaperPerformanceError::Unavailable)?)
        .bind(encode_non_negative(adaptive_value).ok_or(PaperPerformanceError::Unavailable)?)
        .bind(encode_non_negative(plain_dca_value).ok_or(PaperPerformanceError::Unavailable)?)
        .bind(encode_signed(ledger.net_contributions).ok_or(PaperPerformanceError::Unavailable)?)
        .bind(encode_signed(ledger.realized_pnl).ok_or(PaperPerformanceError::Unavailable)?)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;

        Ok(PaperPerformance {
            currency: plan.currency.clone(),
            has_opening_balance: ledger.has_opening_balance,
            data_complete,
            net_contributions: ledger.net_contributions,
            adaptive_value,
            plain_dca_value,
            realized_pnl: ledger.realized_pnl,
            unrealized_pnl,
            total_return,
            points: self.points(plan.id).await?,
        })
    }

    /// 返回一个计划的完整本地快照序列，不读取 broker。
    pub async fn history(
        &self,
        plan_id: Uuid,
    ) -> Result<Vec<PaperPerformancePoint>, PaperPerformanceError> {
        self.points(plan_id).await
    }

    /// 返回本地账本已确认的成交标记，不读取或伪造 provider 成交历史。
    pub async fn trade_markers(
        &self,
        plan_id: Uuid,
    ) -> Result<Vec<PaperTradeMarker>, PaperPerformanceError> {
        let rows = sqlx::query(
            "SELECT paper_orders.plan_id, paper_orders.side, paper_fills.quantity, \
             paper_fills.price, paper_fills.observed_at \
             FROM paper_fills JOIN paper_orders ON paper_orders.order_id = paper_fills.order_id \
             WHERE paper_orders.plan_id = ?1 ORDER BY paper_fills.observed_at, paper_fills.id",
        )
        .bind(plan_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        rows.into_iter()
            .map(|row| {
                Ok(PaperTradeMarker {
                    plan_id: row
                        .try_get::<String, _>("plan_id")
                        .map_err(map_sqlx)?
                        .parse()
                        .map_err(|_| PaperPerformanceError::Unavailable)?,
                    side: row.try_get("side").map_err(map_sqlx)?,
                    quantity: decode_non_negative(
                        row.try_get::<String, _>("quantity").map_err(map_sqlx)?,
                    )?,
                    price: decode_non_negative(
                        row.try_get::<String, _>("price").map_err(map_sqlx)?,
                    )?,
                    observed_at: row.try_get("observed_at").map_err(map_sqlx)?,
                })
            })
            .collect()
    }

    async fn sync_orders(
        &self,
        plan_id: Uuid,
        orders: &[PaperOrder],
    ) -> Result<(), PaperPerformanceError> {
        let stored = sqlx::query("SELECT order_id, filled_quantity, average_fill_price FROM paper_orders WHERE plan_id = ?1")
            .bind(plan_id.to_string()).fetch_all(&self.pool).await.map_err(map_sqlx)?;
        let known = stored
            .into_iter()
            .map(|row| {
                let id: String = row.try_get("order_id").map_err(map_sqlx)?;
                let quantity = decode_non_negative(
                    row.try_get::<String, _>("filled_quantity")
                        .map_err(map_sqlx)?,
                )?;
                let price = decode_non_negative(
                    row.try_get::<String, _>("average_fill_price")
                        .map_err(map_sqlx)?,
                )?;
                Ok((id, (quantity, price)))
            })
            .collect::<Result<HashMap<_, _>, PaperPerformanceError>>()?;
        for order in orders {
            let Some((previous_quantity, previous_average)) = known.get(&order.order_id).copied()
            else {
                continue;
            };
            if order.filled_quantity < previous_quantity {
                continue;
            }
            let delta = order.filled_quantity - previous_quantity;
            if delta > Decimal::ZERO && order.average_fill_price > Decimal::ZERO {
                let incremental_cost = order.average_fill_price * order.filled_quantity
                    - previous_average * previous_quantity;
                let price = if incremental_cost > Decimal::ZERO {
                    incremental_cost / delta
                } else {
                    order.average_fill_price
                };
                sqlx::query("INSERT INTO paper_fills (id, order_id, quantity, price, observed_at) VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))")
                    .bind(Uuid::new_v4().to_string()).bind(&order.order_id)
                    .bind(encode_positive(delta).ok_or(PaperPerformanceError::Unavailable)?)
                    .bind(encode_positive(price).ok_or(PaperPerformanceError::Unavailable)?)
                    .execute(&self.pool).await.map_err(map_sqlx)?;
            }
            sqlx::query("UPDATE paper_orders SET state = ?1, filled_quantity = ?2, average_fill_price = ?3, observed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE order_id = ?4")
                .bind(state_name(order.state))
                .bind(encode_non_negative(order.filled_quantity).ok_or(PaperPerformanceError::Unavailable)?)
                .bind(encode_non_negative(order.average_fill_price).ok_or(PaperPerformanceError::Unavailable)?)
                .bind(&order.order_id).execute(&self.pool).await.map_err(map_sqlx)?;
        }
        Ok(())
    }

    async fn ledger(
        &self,
        plan_id: Uuid,
        base_contribution: Decimal,
    ) -> Result<Ledger, PaperPerformanceError> {
        let flows = sqlx::query(
            "SELECT kind, amount FROM cash_flows WHERE plan_id = ?1 ORDER BY occurred_at, id",
        )
        .bind(plan_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        let mut net_contributions = Decimal::ZERO;
        let mut has_opening_balance = false;
        for row in flows {
            let kind: String = row.try_get("kind").map_err(map_sqlx)?;
            let amount =
                decode_non_negative(row.try_get::<String, _>("amount").map_err(map_sqlx)?)?;
            if kind == "opening_balance" {
                has_opening_balance = true;
            }
            net_contributions += if kind == "withdrawal" {
                -amount
            } else {
                amount
            };
        }
        let fills = sqlx::query("SELECT paper_orders.side, paper_fills.quantity, paper_fills.price, paper_fills.order_id FROM paper_fills JOIN paper_orders ON paper_orders.order_id = paper_fills.order_id WHERE paper_orders.plan_id = ?1 ORDER BY paper_fills.observed_at, paper_fills.id")
            .bind(plan_id.to_string()).fetch_all(&self.pool).await.map_err(map_sqlx)?;
        let mut lots: Vec<(Decimal, Decimal)> = Vec::new();
        let mut cash = net_contributions;
        let mut realized = Decimal::ZERO;
        let mut unmatched_sell = Decimal::ZERO;
        let mut plain_seen = HashMap::<String, Decimal>::new();
        for row in fills {
            let side: String = row.try_get("side").map_err(map_sqlx)?;
            let quantity =
                decode_non_negative(row.try_get::<String, _>("quantity").map_err(map_sqlx)?)?;
            let price = decode_non_negative(row.try_get::<String, _>("price").map_err(map_sqlx)?)?;
            let id: String = row.try_get("order_id").map_err(map_sqlx)?;
            if side == "buy" {
                cash -= quantity * price;
                lots.push((quantity, price));
                plain_seen.entry(id).or_insert(price);
            } else {
                cash += quantity * price;
                let mut remaining = quantity;
                while remaining > Decimal::ZERO {
                    let Some((lot_quantity, lot_price)) = lots.first_mut() else {
                        unmatched_sell += remaining;
                        break;
                    };
                    let matched = remaining.min(*lot_quantity);
                    realized += (price - *lot_price) * matched;
                    *lot_quantity -= matched;
                    remaining -= matched;
                    if lot_quantity.is_zero() {
                        lots.remove(0);
                    }
                }
            }
        }
        let quantity = lots.iter().map(|(quantity, _)| *quantity).sum();
        let open_cost = lots
            .iter()
            .map(|(quantity, price)| *quantity * *price)
            .sum();
        let plain_units = plain_seen
            .values()
            .filter(|price| **price > Decimal::ZERO)
            .map(|price| base_contribution / *price)
            .sum();
        Ok(Ledger {
            has_opening_balance,
            net_contributions,
            cash,
            quantity,
            open_cost,
            realized_pnl: realized,
            unmatched_sell,
            plain_units,
        })
    }

    async fn points(
        &self,
        plan_id: Uuid,
    ) -> Result<Vec<PaperPerformancePoint>, PaperPerformanceError> {
        let rows = sqlx::query("SELECT observed_at, adaptive_value, plain_dca_value, net_contributions FROM portfolio_snapshots WHERE plan_id = ?1 ORDER BY observed_at, id")
            .bind(plan_id.to_string()).fetch_all(&self.pool).await.map_err(map_sqlx)?;
        rows.into_iter()
            .map(|row| {
                Ok(PaperPerformancePoint {
                    observed_at: row.try_get("observed_at").map_err(map_sqlx)?,
                    adaptive_value: decode_non_negative(
                        row.try_get::<String, _>("adaptive_value")
                            .map_err(map_sqlx)?,
                    )?,
                    plain_dca_value: decode_non_negative(
                        row.try_get::<String, _>("plain_dca_value")
                            .map_err(map_sqlx)?,
                    )?,
                    net_contributions: decode_signed(
                        row.try_get::<String, _>("net_contributions")
                            .map_err(map_sqlx)?,
                    )?,
                })
            })
            .collect()
    }
}

struct Ledger {
    has_opening_balance: bool,
    net_contributions: Decimal,
    cash: Decimal,
    quantity: Decimal,
    open_cost: Decimal,
    realized_pnl: Decimal,
    unmatched_sell: Decimal,
    plain_units: Decimal,
}

fn map_sqlx(error: sqlx::Error) -> PaperPerformanceError {
    tracing::warn!(%error, "paper performance SQLite operation failed");
    PaperPerformanceError::Unavailable
}
fn side_name(side: BrokerOrderSide) -> &'static str {
    match side {
        BrokerOrderSide::Buy => "buy",
        BrokerOrderSide::Sell => "sell",
    }
}
fn state_name(state: PaperOrderState) -> &'static str {
    match state {
        PaperOrderState::Pending => "pending",
        PaperOrderState::PartiallyFilled => "partially_filled",
        PaperOrderState::Filled => "filled",
        PaperOrderState::Closed => "closed",
        PaperOrderState::Unknown => "unknown",
    }
}
fn encode_positive(value: Decimal) -> Option<String> {
    (value > Decimal::ZERO)
        .then(|| encode_fixed(value))
        .flatten()
}
fn encode_non_negative(value: Decimal) -> Option<String> {
    (value >= Decimal::ZERO)
        .then(|| encode_fixed(value))
        .flatten()
}
fn encode_signed(value: Decimal) -> Option<String> {
    let negative = value < Decimal::ZERO;
    encode_fixed(value.abs()).map(|encoded| {
        if negative {
            format!("-{encoded}")
        } else {
            encoded
        }
    })
}
fn encode_fixed(value: Decimal) -> Option<String> {
    let mut value = value;
    value.rescale(SCALE);
    let rendered = value.to_string();
    let (integer, fractional) = rendered.split_once('.')?;
    (integer.len() <= INTEGER_DIGITS
        && fractional.len() == SCALE as usize
        && integer.bytes().all(|byte| byte.is_ascii_digit())
        && fractional.bytes().all(|byte| byte.is_ascii_digit()))
    .then(|| format!("{:0>width$}.{fractional}", integer, width = INTEGER_DIGITS))
}
fn decode_non_negative(value: String) -> Result<Decimal, PaperPerformanceError> {
    let decimal: Decimal = value
        .parse()
        .map_err(|_| PaperPerformanceError::Unavailable)?;
    (decimal >= Decimal::ZERO && encode_non_negative(decimal).as_deref() == Some(&value))
        .then_some(decimal)
        .ok_or(PaperPerformanceError::Unavailable)
}

fn decode_signed(value: String) -> Result<Decimal, PaperPerformanceError> {
    let decimal: Decimal = value
        .parse()
        .map_err(|_| PaperPerformanceError::Unavailable)?;
    (encode_signed(decimal).as_deref() == Some(&value))
        .then_some(decimal)
        .ok_or(PaperPerformanceError::Unavailable)
}
fn is_millisecond_utc(value: &str) -> bool {
    value.len() == 24
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value.as_bytes().get(10) == Some(&b'T')
        && value.as_bytes().get(13) == Some(&b':')
        && value.as_bytes().get(16) == Some(&b':')
        && value.as_bytes().get(19) == Some(&b'.')
        && value.ends_with('Z')
        && value.bytes().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19 | 23) || byte.is_ascii_digit()
        })
}

#[cfg(test)]
mod tests {
    use broker::{
        BrokerEnvironment, BrokerOrderAck, BrokerOrderRequest, BrokerOrderStatus, PaperOrder,
        PaperPosition,
    };
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::{SqliteInvestmentPlanRepository, SqliteStorage};

    fn money(value: i64) -> Decimal {
        Decimal::new(value, 0)
    }

    async fn repository() -> (SqlitePaperPerformanceRepository, PaperPerformancePlan) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .in_memory(true)
                    .foreign_keys(true),
            )
            .await
            .expect("in-memory SQLite must connect");
        let storage = SqliteStorage::from_pool(pool);
        storage.migrate().await.expect("schema must migrate");
        let plans = SqliteInvestmentPlanRepository::new(storage.pool().clone());
        let plan = investment_plans::InvestmentPlanRepository::create(
            &plans,
            investment_plans::CreateInvestmentPlan {
                name: "Core".to_owned(),
                symbol: "VOO".to_owned(),
                base_contribution: money(1000),
                currency: "USD".to_owned(),
                schedule_kind: investment_plans::ScheduleKind::Monthly,
                schedule_day: 15,
                schedule_days: vec![15],
                timezone: investment_plans::PlanTimeZone::utc(),
                policy: None,
                execution_configuration: investment_plans::PlanExecutionConfiguration::default(),
                max_single_execution: money(1500),
            },
        )
        .await
        .expect("plan must persist");
        (
            SqlitePaperPerformanceRepository::new(storage.pool().clone()),
            PaperPerformancePlan {
                id: plan.id,
                symbol: plan.symbol,
                currency: plan.currency,
                base_contribution: plan.base_contribution,
            },
        )
    }

    fn portfolio(filled_quantity: Decimal, price: Decimal) -> PaperPortfolioSnapshot {
        PaperPortfolioSnapshot {
            currency: "USD".to_owned(),
            cash: money(900),
            buying_power: money(900),
            total_assets: money(1010),
            market_value: money(110),
            positions: vec![PaperPosition {
                symbol: "VOO".to_owned(),
                name: Some("Vanguard S&P 500 ETF".to_owned()),
                quantity: filled_quantity,
                price,
                cost_price: money(100),
                market_value: filled_quantity * price,
                unrealized_pnl: filled_quantity * (price - money(100)),
            }],
            orders: vec![PaperOrder {
                order_id: "paper-order-1".to_owned(),
                symbol: "VOO".to_owned(),
                side: BrokerOrderSide::Buy,
                state: PaperOrderState::Filled,
                quantity: Decimal::ONE,
                filled_quantity,
                average_fill_price: money(100),
            }],
        }
    }

    /// Verify one observed OpenD fill creates an idempotent local FIFO ledger and chart point.
    #[tokio::test]
    async fn refresh_reconciles_fill_and_calculates_local_performance() {
        let (repository, plan) = repository().await;
        repository
            .set_opening_balance(plan.id, money(1000), "2026-07-19T00:00:00.000Z")
            .await
            .expect("opening balance must persist");
        let request = BrokerOrderRequest::market(
            "paper-order-1",
            "VOO",
            BrokerOrderSide::Buy,
            Decimal::ONE,
            BrokerEnvironment::Paper,
        )
        .expect("order must normalize");
        let acknowledgement = BrokerOrderAck::new(
            "paper-order-1",
            BrokerEnvironment::Paper,
            BrokerOrderStatus::Accepted,
        )
        .expect("ack must normalize");
        repository
            .record_accepted_order(plan.id, Uuid::new_v4(), &acknowledgement, &request)
            .await
            .expect("accepted order must persist");

        let performance = repository
            .refresh(&plan, &portfolio(Decimal::ONE, money(110)))
            .await
            .expect("portfolio must reconcile");
        assert!(performance.data_complete);
        assert_eq!(performance.net_contributions, money(1000));
        assert_eq!(performance.adaptive_value, money(1010));
        assert_eq!(performance.total_return, Some(money(10)));
        assert_eq!(performance.plain_dca_value, money(1100));
        assert_eq!(performance.points.len(), 1);

        let repeated = repository
            .refresh(&plan, &portfolio(Decimal::ONE, money(110)))
            .await
            .expect("same broker state must stay idempotent");
        assert_eq!(repeated.realized_pnl, Decimal::ZERO);
        assert_eq!(repeated.points.len(), 2);
    }
}
