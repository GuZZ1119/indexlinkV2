//! On-demand, symbol-agnostic historical evaluation over caller-provided daily closes.
//!
//! This module deliberately performs no network or storage IO. The API layer supplies an
//! adjusted daily-close snapshot, and this evaluator applies the same restricted Formula V1
//! interpreter used by live decisions. That keeps provider concerns out of strategy math and
//! makes US, A-share, and Hong Kong inputs comparable under one contract.

use chrono::{Datelike, NaiveDate};
use investment_plans::{
    BucketAllocationRatio, OpportunityCashPolicy, PlanExecutionConfiguration, PlanRiskMode,
    TwoBucketAllocationConfig, TwoBucketContributionSplit,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::Serialize;
use strategy_dsl::{DslEvidence, IndicatorSpec, StrategyDslRuntimeError, StrategySpec};
use strategy_policy::{DecisionContext, PolicyValidationError};
use thiserror::Error;
use time::{Date, Month};

use crate::xirr;

const BUY_COST_BPS: f64 = 5.0;

/// One validated adjusted daily closing price supplied to the pure evaluator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BacktestPrice {
    date: NaiveDate,
    adjusted_close: Decimal,
}

impl BacktestPrice {
    /// Construct a positive adjusted close for one trading date.
    pub fn new(date: NaiveDate, adjusted_close: Decimal) -> Result<Self, DynamicBacktestError> {
        if adjusted_close <= Decimal::ZERO {
            return Err(DynamicBacktestError::InvalidPrice);
        }
        Ok(Self {
            date,
            adjusted_close,
        })
    }

    /// Return the trading date.
    #[must_use]
    pub fn date(self) -> NaiveDate {
        self.date
    }

    /// Return the split/dividend-adjusted close.
    #[must_use]
    pub fn adjusted_close(self) -> Decimal {
        self.adjusted_close
    }
}

/// One strategy selected for an on-demand comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum BacktestStrategy {
    /// Invest the full contribution on every scheduled execution date.
    FixedDca,
    /// Apply one immutable restricted Formula V1 strategy to the 30% opportunity bucket.
    Formula(StrategySpec),
}

impl BacktestStrategy {
    fn id(&self) -> &str {
        match self {
            Self::FixedDca => "fixed_dca",
            Self::Formula(strategy) => strategy.policy().id().as_str(),
        }
    }

    fn version(&self) -> u32 {
        match self {
            Self::FixedDca => 1,
            Self::Formula(strategy) => strategy.policy().version().value(),
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::FixedDca => "Fixed DCA",
            Self::Formula(strategy) => strategy.name(),
        }
    }
}

/// Complete input for a pure on-demand comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicBacktestRequest {
    /// Canonical market-qualified symbol, for example `US.SPY`, `SH.510300`, or `HK.02800`.
    pub symbol: String,
    /// First date visible in the requested comparison period.
    pub start: NaiveDate,
    /// Last date visible in the requested comparison period.
    pub end: NaiveDate,
    /// Monthly calendar day; the next available trading session is used for execution.
    pub monthly_day: u8,
    /// Equal external cash contribution made for every evaluated strategy.
    pub contribution_amount: Decimal,
    /// Adjusted daily closes, including any warm-up history before `start`.
    pub prices: Vec<BacktestPrice>,
    /// Unique strategy versions evaluated over the exact same dates and cash flows.
    pub strategies: Vec<BacktestStrategy>,
}

/// A normalized time-series point shared by intuitive and professional views.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NormalizedBacktestPoint {
    /// Trading date in ISO `YYYY-MM-DD` format.
    pub date: String,
    /// Time-weighted wealth index rebased to 100 at the first common observation.
    pub value: f64,
}

/// One adjusted daily close displayed beside the strategy comparison.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BacktestMarketPoint {
    /// Trading date in ISO `YYYY-MM-DD` format.
    pub date: String,
    /// Provider-supplied adjusted closing price in the instrument's trading currency.
    pub adjusted_close: f64,
}

/// One simulated purchase made by a strategy on a scheduled evaluation date.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BacktestExecutionPoint {
    /// Simulated execution date in ISO `YYYY-MM-DD` format.
    pub date: String,
    /// Adjusted close used as the simulated execution price.
    pub adjusted_close: f64,
    /// Cash debited for the simulated purchase, including transaction cost.
    pub invested_amount: f64,
    /// Share of that period's external contribution consumed by the simulated purchase.
    pub budget_utilisation_percent: f64,
    /// External cash assigned to this scheduled period before strategy adjustments.
    pub scheduled_contribution_amount: f64,
    /// Cash assigned to the always-on core bucket.
    pub core_invested_amount: f64,
    /// Cash assigned to the Formula-controlled opportunity bucket.
    pub opportunity_invested_amount: f64,
    /// This period's external cash left uninvested after the simulated decision.
    pub unallocated_amount: f64,
    /// Transaction cost implied by the fixed basis-point purchase model.
    pub transaction_cost: f64,
    /// Whether a Formula rule matched and changed the opportunity-bucket decision.
    ///
    /// Fixed DCA and Formula periods that keep the standard opportunity allocation are `false`.
    pub strategy_rule_matched: bool,
}

/// One daily peak-relative drawdown observation for the professional view.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BacktestDrawdownPoint {
    /// Trading date in ISO `YYYY-MM-DD` format.
    pub date: String,
    /// Percentage change from the running peak; zero at a new peak and otherwise negative.
    pub value_percent: f64,
}

/// Auditable intermediate values used by the published professional metrics.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BacktestCalculationDetails {
    /// Calendar-day span used by CAGR-style annualization.
    pub elapsed_days: i64,
    /// Count of daily time-weighted returns used by volatility and Sortino.
    pub daily_return_count: usize,
    /// Arithmetic mean of daily time-weighted returns, in percent.
    pub mean_daily_return_percent: Option<f64>,
    /// Sample standard deviation of daily time-weighted returns, in percent.
    pub daily_standard_deviation_percent: Option<f64>,
    /// Root mean square of returns below zero, in percent.
    pub downside_deviation_percent: Option<f64>,
    /// Peak date immediately preceding the maximum drawdown trough.
    pub drawdown_peak_date: Option<String>,
    /// Date of the maximum drawdown trough.
    pub drawdown_trough_date: Option<String>,
    /// First later date on which the prior peak was recovered, when observed.
    pub drawdown_recovery_date: Option<String>,
    /// Total simulated purchase cost across the common result window.
    pub total_transaction_cost: f64,
    /// Number of Formula periods in which a rule actually matched.
    pub rule_matched_count: usize,
    /// Number of periods that used the standard, unmatched decision path.
    pub standard_execution_count: usize,
    /// Trading-period annualization constant used by volatility and Sortino.
    pub trading_periods_per_year: u16,
    /// Calendar-day annualization constant used by annualized return and XIRR.
    pub calendar_days_per_year: f64,
    /// Fixed simulated purchase cost in basis points.
    pub buy_cost_bps: f64,
}

/// Comparable non-promotional metrics for one strategy on one symbol and period.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BacktestMetrics {
    /// Time-weighted total return over the evaluated period.
    pub total_return_percent: f64,
    /// Annualized time-weighted return when the period spans at least one day.
    pub annualized_return_percent: Option<f64>,
    /// Money-weighted return based on actual contribution dates, when solvable.
    pub xirr_percent: Option<f64>,
    /// Maximum peak-to-trough decline of the normalized trajectory.
    pub maximum_drawdown_percent: f64,
    /// Annualized daily-return volatility, when enough observations exist.
    pub annualized_volatility_percent: Option<f64>,
    /// Daily downside-risk-adjusted return ratio, when observable.
    pub sortino_ratio: Option<f64>,
    /// Total external cash contributed during the common evaluation window.
    pub total_contributed: f64,
    /// Total cash debited for simulated purchases, including transaction costs.
    pub total_invested: f64,
    /// Share of contributed cash consumed by simulated purchases.
    pub cash_utilisation_percent: f64,
    /// Final marked-to-market portfolio value, including uninvested cash.
    pub terminal_wealth: f64,
    /// Final uninvested cash retained by the simulated plan.
    pub terminal_cash: f64,
}

/// One strategy result under the shared symbol, dates, data revision, and cash-flow schedule.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BacktestSeries {
    /// Stable policy identifier.
    pub strategy_id: String,
    /// Immutable strategy version.
    pub strategy_version: u32,
    /// Human-readable strategy name.
    pub strategy_name: String,
    /// Daily normalized trajectory rebased to 100.
    pub normalized_points: Vec<NormalizedBacktestPoint>,
    /// Scheduled simulated purchases generated from the same causal decisions.
    pub execution_points: Vec<BacktestExecutionPoint>,
    /// Daily running-peak drawdown series derived from the same normalized trajectory.
    pub drawdown_points: Vec<BacktestDrawdownPoint>,
    /// Metrics calculated from this exact trajectory and its contribution ledger.
    pub metrics: BacktestMetrics,
    /// Intermediate values and assumptions that make the published metrics reproducible.
    pub calculation_details: BacktestCalculationDetails,
}

/// Result of one fair, common-window strategy comparison.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DynamicBacktestResult {
    /// Canonical symbol used by every returned strategy.
    pub symbol: String,
    /// First trading date on which every selected strategy had complete causal evidence.
    pub effective_start: String,
    /// Last marked trading date.
    pub effective_end: String,
    /// Number of equal scheduled contributions applied to every strategy.
    pub contribution_count: usize,
    /// Adjusted daily closes over the exact common visible result window.
    pub market_points: Vec<BacktestMarketPoint>,
    /// Per-strategy results under identical market inputs.
    pub series: Vec<BacktestSeries>,
}

/// Validation or evaluation failure from the pure dynamic backtest boundary.
#[derive(Debug, Error)]
pub enum DynamicBacktestError {
    /// Symbol is empty or no strategies were selected.
    #[error("backtest selection is empty")]
    EmptySelection,
    /// Requested dates, monthly day, or contribution are invalid.
    #[error("backtest request is invalid")]
    InvalidRequest,
    /// A price is non-positive or cannot be represented safely.
    #[error("backtest price is invalid")]
    InvalidPrice,
    /// Price dates are duplicated or not strictly increasing.
    #[error("backtest prices must be strictly chronological")]
    NonChronologicalPrices,
    /// A selected Formula V1 strategy requires a non-price series such as VIX.
    #[error("selected strategy requires an unsupported external indicator")]
    ExternalIndicatorRequired,
    /// A selected formula contains an exact-amount action not supported by this comparison mode.
    #[error("selected strategy uses an unsupported exact-amount action")]
    UnsupportedFormulaAction,
    /// Supplied warm-up history cannot satisfy all selected strategies.
    #[error("price history does not contain enough causal warm-up observations")]
    InsufficientHistory,
    /// The strategy interpreter rejected one causal evidence snapshot.
    #[error(transparent)]
    Dsl(#[from] StrategyDslRuntimeError),
    /// The strategy policy context rejected the contribution or date.
    #[error(transparent)]
    Policy(#[from] PolicyValidationError),
    /// The shared two-bucket execution contract rejected its fixed safe configuration.
    #[error(transparent)]
    Plan(#[from] investment_plans::PlanValidationError),
}

/// Evaluate selected strategies on caller-provided adjusted daily closes.
///
/// All strategies receive identical contribution dates. Formula strategies may read only closes
/// strictly before the simulated execution session, preventing same-close look-ahead. Comparison
/// begins only once every selected strategy has enough warm-up history.
pub fn run_dynamic_backtest(
    request: DynamicBacktestRequest,
) -> Result<DynamicBacktestResult, DynamicBacktestError> {
    validate_request(&request)?;
    let schedule = common_schedule(&request)?;
    let effective_start_index = *schedule
        .first()
        .ok_or(DynamicBacktestError::InsufficientHistory)?;
    let effective_start = request.prices[effective_start_index].date;
    let effective_end_index = request
        .prices
        .iter()
        .rposition(|price| price.date <= request.end)
        .ok_or(DynamicBacktestError::InsufficientHistory)?;
    let market_points = request.prices[effective_start_index..=effective_end_index]
        .iter()
        .map(|price| {
            Ok(BacktestMarketPoint {
                date: price.date.to_string(),
                adjusted_close: decimal_price_to_f64(price.adjusted_close)?,
            })
        })
        .collect::<Result<Vec<_>, DynamicBacktestError>>()?;

    let mut series = Vec::with_capacity(request.strategies.len());
    for strategy in &request.strategies {
        series.push(simulate_strategy(
            &request,
            strategy,
            &schedule,
            effective_start_index,
            effective_end_index,
        )?);
    }

    Ok(DynamicBacktestResult {
        symbol: request.symbol.trim().to_ascii_uppercase(),
        effective_start: effective_start.to_string(),
        effective_end: request.prices[effective_end_index].date.to_string(),
        contribution_count: schedule.len(),
        market_points,
        series,
    })
}

fn validate_request(request: &DynamicBacktestRequest) -> Result<(), DynamicBacktestError> {
    if request.symbol.trim().is_empty() || request.strategies.is_empty() {
        return Err(DynamicBacktestError::EmptySelection);
    }
    if request.start > request.end
        || !(1..=28).contains(&request.monthly_day)
        || request.contribution_amount <= Decimal::ZERO
    {
        return Err(DynamicBacktestError::InvalidRequest);
    }
    if request.prices.is_empty()
        || request
            .prices
            .windows(2)
            .any(|pair| pair[0].date >= pair[1].date)
    {
        return Err(DynamicBacktestError::NonChronologicalPrices);
    }
    let mut identities = std::collections::BTreeSet::new();
    for strategy in &request.strategies {
        if !identities.insert((strategy.id().to_owned(), strategy.version())) {
            return Err(DynamicBacktestError::InvalidRequest);
        }
        if matches!(strategy, BacktestStrategy::Formula(spec) if spec.required_indicators().contains(&IndicatorSpec::Vix))
        {
            return Err(DynamicBacktestError::ExternalIndicatorRequired);
        }
        if matches!(strategy, BacktestStrategy::Formula(spec) if spec.has_fixed_opportunity_amount_action())
        {
            return Err(DynamicBacktestError::UnsupportedFormulaAction);
        }
    }
    Ok(())
}

fn common_schedule(request: &DynamicBacktestRequest) -> Result<Vec<usize>, DynamicBacktestError> {
    let required_history = request
        .strategies
        .iter()
        .map(|strategy| match strategy {
            BacktestStrategy::FixedDca => 0,
            BacktestStrategy::Formula(spec) => spec.required_close_observations(),
        })
        .max()
        .unwrap_or(0);
    let mut schedule = Vec::new();
    let mut previous_month = None;
    for (index, price) in request.prices.iter().enumerate() {
        if price.date < request.start || price.date > request.end {
            continue;
        }
        let month = (price.date.year(), price.date.month());
        if previous_month == Some(month) || price.date.day() < u32::from(request.monthly_day) {
            continue;
        }
        previous_month = Some(month);
        if index >= required_history {
            schedule.push(index);
        }
    }
    if schedule.is_empty() {
        Err(DynamicBacktestError::InsufficientHistory)
    } else {
        Ok(schedule)
    }
}

fn simulate_strategy(
    request: &DynamicBacktestRequest,
    strategy: &BacktestStrategy,
    schedule: &[usize],
    start_index: usize,
    end_index: usize,
) -> Result<BacktestSeries, DynamicBacktestError> {
    let mut state = SimulationState::default();
    let mut schedule_cursor = 0;
    for index in start_index..=end_index {
        let price = request.prices[index];
        if schedule.get(schedule_cursor).copied() == Some(index) {
            state.deposit(price.date, request.contribution_amount)?;
            let spend = strategy_spend(
                strategy,
                request.contribution_amount,
                &request.prices[..index],
            )?;
            let purchase = state.buy(spend.amount, price.adjusted_close)?;
            state.record_execution(
                price.date,
                price.adjusted_close,
                request.contribution_amount,
                &spend,
                purchase,
            )?;
            schedule_cursor += 1;
        }
        state.mark(price.date, price.adjusted_close)?;
    }
    state.finish(strategy)
}

struct StrategySpend {
    amount: Decimal,
    core_amount: Decimal,
    opportunity_amount: Decimal,
    strategy_rule_matched: bool,
}

fn strategy_spend(
    strategy: &BacktestStrategy,
    contribution: Decimal,
    history: &[BacktestPrice],
) -> Result<StrategySpend, DynamicBacktestError> {
    if matches!(strategy, BacktestStrategy::FixedDca) {
        return Ok(StrategySpend {
            amount: contribution,
            core_amount: contribution,
            opportunity_amount: Decimal::ZERO,
            strategy_rule_matched: false,
        });
    }
    let BacktestStrategy::Formula(spec) = strategy else {
        unreachable!("fixed DCA returned above")
    };
    let closes = history
        .iter()
        .map(|price| price.adjusted_close)
        .collect::<Vec<_>>();
    let evidence = DslEvidence::from_market_snapshot(spec, &closes, Decimal::ZERO)?;
    let as_of = history
        .last()
        .ok_or(DynamicBacktestError::InsufficientHistory)?
        .date;
    let context = DecisionContext::new(to_time_date(as_of)?, contribution, evidence)?;
    let evaluation = spec.evaluate(&context)?;
    let strategy_rule_matched = evaluation.matched_rule_index().is_some();
    let recommendation = evaluation.recommendation().clone();
    let configuration = formula_configuration()?;
    let split = TwoBucketContributionSplit::from_decision(
        contribution,
        configuration,
        recommendation.action(),
        recommendation.multiplier(),
    )?;
    Ok(StrategySpend {
        amount: split.recommended_contribution(),
        core_amount: split.core_contribution(),
        opportunity_amount: split.opportunity_contribution(),
        strategy_rule_matched,
    })
}

fn formula_configuration() -> Result<PlanExecutionConfiguration, DynamicBacktestError> {
    let allocation = TwoBucketAllocationConfig::new(
        BucketAllocationRatio::new(Decimal::new(7, 1))?,
        BucketAllocationRatio::new(Decimal::new(3, 1))?,
    )?;
    Ok(PlanExecutionConfiguration::new_with_cash_policy(
        allocation,
        PlanRiskMode::Approval,
        OpportunityCashPolicy::ExpireEachPeriod,
    )?)
}

fn to_time_date(value: NaiveDate) -> Result<Date, DynamicBacktestError> {
    let month =
        Month::try_from(value.month() as u8).map_err(|_| DynamicBacktestError::InvalidRequest)?;
    Date::from_calendar_date(value.year(), month, value.day() as u8)
        .map_err(|_| DynamicBacktestError::InvalidRequest)
}

#[derive(Default)]
struct SimulationState {
    cash: f64,
    units: f64,
    contributed: f64,
    invested: f64,
    transaction_cost: f64,
    last_value: f64,
    pending_flow: f64,
    nav: f64,
    points: Vec<(NaiveDate, f64)>,
    execution_points: Vec<BacktestExecutionPoint>,
    daily_returns: Vec<f64>,
    flows: Vec<(NaiveDate, f64)>,
}

struct PurchaseOutcome {
    invested_amount: f64,
    transaction_cost: f64,
}

impl SimulationState {
    fn deposit(&mut self, date: NaiveDate, amount: Decimal) -> Result<(), DynamicBacktestError> {
        let amount = amount
            .to_f64()
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or(DynamicBacktestError::InvalidRequest)?;
        self.cash += amount;
        self.contributed += amount;
        self.pending_flow += amount;
        self.flows.push((date, -amount));
        Ok(())
    }

    fn buy(
        &mut self,
        amount: Decimal,
        price: Decimal,
    ) -> Result<PurchaseOutcome, DynamicBacktestError> {
        let amount = amount
            .to_f64()
            .filter(|value| value.is_finite())
            .ok_or(DynamicBacktestError::InvalidRequest)?
            .clamp(0.0, self.cash);
        let price = price
            .to_f64()
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or(DynamicBacktestError::InvalidPrice)?;
        let purchased_value = amount / (1.0 + BUY_COST_BPS / 10_000.0);
        let transaction_cost = amount - purchased_value;
        self.cash -= amount;
        self.invested += amount;
        self.transaction_cost += transaction_cost;
        self.units += purchased_value / price;
        Ok(PurchaseOutcome {
            invested_amount: amount,
            transaction_cost,
        })
    }

    fn record_execution(
        &mut self,
        date: NaiveDate,
        price: Decimal,
        contribution: Decimal,
        spend: &StrategySpend,
        purchase: PurchaseOutcome,
    ) -> Result<(), DynamicBacktestError> {
        let contribution = contribution
            .to_f64()
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or(DynamicBacktestError::InvalidRequest)?;
        let core_invested_amount = spend
            .core_amount
            .to_f64()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or(DynamicBacktestError::InvalidRequest)?;
        let opportunity_invested_amount = spend
            .opportunity_amount
            .to_f64()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or(DynamicBacktestError::InvalidRequest)?;
        self.execution_points.push(BacktestExecutionPoint {
            date: date.to_string(),
            adjusted_close: decimal_price_to_f64(price)?,
            invested_amount: purchase.invested_amount,
            budget_utilisation_percent: purchase.invested_amount / contribution * 100.0,
            scheduled_contribution_amount: contribution,
            core_invested_amount,
            opportunity_invested_amount,
            unallocated_amount: (contribution - purchase.invested_amount).max(0.0),
            transaction_cost: purchase.transaction_cost,
            strategy_rule_matched: spend.strategy_rule_matched,
        });
        Ok(())
    }

    fn mark(&mut self, date: NaiveDate, price: Decimal) -> Result<(), DynamicBacktestError> {
        if self.contributed == 0.0 {
            return Ok(());
        }
        let close = price
            .to_f64()
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or(DynamicBacktestError::InvalidPrice)?;
        let value = self.cash + self.units * close;
        let denominator = self.last_value + self.pending_flow;
        let period_return = if denominator > 0.0 {
            value / denominator - 1.0
        } else {
            0.0
        };
        if self.points.is_empty() {
            self.nav = 1.0;
        } else {
            self.nav *= 1.0 + period_return;
            self.daily_returns.push(period_return);
        }
        self.points.push((date, self.nav));
        self.last_value = value;
        self.pending_flow = 0.0;
        Ok(())
    }

    fn finish(
        mut self,
        strategy: &BacktestStrategy,
    ) -> Result<BacktestSeries, DynamicBacktestError> {
        let (first_date, first_nav) = self
            .points
            .first()
            .copied()
            .ok_or(DynamicBacktestError::InsufficientHistory)?;
        let (last_date, last_nav) = self
            .points
            .last()
            .copied()
            .ok_or(DynamicBacktestError::InsufficientHistory)?;
        self.flows.push((last_date, self.last_value));
        let drawdown = drawdown_analysis(&self.points);
        let total_return = last_nav / first_nav - 1.0;
        let elapsed_days = (last_date - first_date).num_days();
        let annualized_return = (elapsed_days > 0)
            .then(|| ((last_nav / first_nav).powf(365.25 / elapsed_days as f64) - 1.0) * 100.0);
        let return_details = daily_return_details(&self.daily_returns);
        let rule_matched_count = self
            .execution_points
            .iter()
            .filter(|point| point.strategy_rule_matched)
            .count();
        let standard_execution_count = self.execution_points.len() - rule_matched_count;
        let normalized_points = self
            .points
            .into_iter()
            .map(|(date, value)| NormalizedBacktestPoint {
                date: date.to_string(),
                value: value / first_nav * 100.0,
            })
            .collect();
        Ok(BacktestSeries {
            strategy_id: strategy.id().to_owned(),
            strategy_version: strategy.version(),
            strategy_name: strategy.name().to_owned(),
            normalized_points,
            execution_points: self.execution_points,
            drawdown_points: drawdown.points,
            metrics: BacktestMetrics {
                total_return_percent: total_return * 100.0,
                annualized_return_percent: annualized_return,
                xirr_percent: xirr(&self.flows).map(|value| value * 100.0),
                maximum_drawdown_percent: drawdown.maximum * 100.0,
                annualized_volatility_percent: return_details
                    .sample_standard_deviation
                    .map(|value| value * 252.0_f64.sqrt() * 100.0),
                sortino_ratio: match (return_details.mean, return_details.downside_deviation) {
                    (Some(mean), Some(downside)) if downside > 0.0 => {
                        Some(mean / downside * 252.0_f64.sqrt())
                    }
                    _ => None,
                },
                total_contributed: self.contributed,
                total_invested: self.invested,
                cash_utilisation_percent: self.invested / self.contributed * 100.0,
                terminal_wealth: self.last_value,
                terminal_cash: self.cash,
            },
            calculation_details: BacktestCalculationDetails {
                elapsed_days,
                daily_return_count: self.daily_returns.len(),
                mean_daily_return_percent: return_details.mean.map(|value| value * 100.0),
                daily_standard_deviation_percent: return_details
                    .sample_standard_deviation
                    .map(|value| value * 100.0),
                downside_deviation_percent: return_details
                    .downside_deviation
                    .map(|value| value * 100.0),
                drawdown_peak_date: drawdown.peak_date,
                drawdown_trough_date: drawdown.trough_date,
                drawdown_recovery_date: drawdown.recovery_date,
                total_transaction_cost: self.transaction_cost,
                rule_matched_count,
                standard_execution_count,
                trading_periods_per_year: 252,
                calendar_days_per_year: 365.25,
                buy_cost_bps: BUY_COST_BPS,
            },
        })
    }
}

fn decimal_price_to_f64(price: Decimal) -> Result<f64, DynamicBacktestError> {
    price
        .to_f64()
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or(DynamicBacktestError::InvalidPrice)
}

struct DailyReturnDetails {
    mean: Option<f64>,
    sample_standard_deviation: Option<f64>,
    downside_deviation: Option<f64>,
}

fn daily_return_details(returns: &[f64]) -> DailyReturnDetails {
    if returns.is_empty() {
        return DailyReturnDetails {
            mean: None,
            sample_standard_deviation: None,
            downside_deviation: None,
        };
    }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let sample_standard_deviation = (returns.len() >= 2).then(|| {
        (returns
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / (returns.len() - 1) as f64)
            .sqrt()
    });
    let downside_deviation = (returns
        .iter()
        .map(|value| value.min(0.0).powi(2))
        .sum::<f64>()
        / returns.len() as f64)
        .sqrt();
    DailyReturnDetails {
        mean: Some(mean),
        sample_standard_deviation,
        downside_deviation: Some(downside_deviation),
    }
}

struct DrawdownAnalysis {
    points: Vec<BacktestDrawdownPoint>,
    maximum: f64,
    peak_date: Option<String>,
    trough_date: Option<String>,
    recovery_date: Option<String>,
}

fn drawdown_analysis(points: &[(NaiveDate, f64)]) -> DrawdownAnalysis {
    let mut running_peak = f64::NEG_INFINITY;
    let mut running_peak_index = 0_usize;
    let mut maximum = 0.0_f64;
    let mut maximum_peak_index = None;
    let mut maximum_trough_index = None;
    let drawdown_points = points
        .iter()
        .enumerate()
        .map(|(index, (date, value))| {
            if *value >= running_peak {
                running_peak = *value;
                running_peak_index = index;
            }
            let drawdown = if running_peak > 0.0 {
                (running_peak - value) / running_peak
            } else {
                0.0
            };
            if drawdown > maximum {
                maximum = drawdown;
                maximum_peak_index = Some(running_peak_index);
                maximum_trough_index = Some(index);
            }
            BacktestDrawdownPoint {
                date: date.to_string(),
                value_percent: -drawdown * 100.0,
            }
        })
        .collect::<Vec<_>>();
    let recovery_index =
        maximum_peak_index
            .zip(maximum_trough_index)
            .and_then(|(peak_index, trough_index)| {
                let peak_value = points[peak_index].1;
                points
                    .iter()
                    .enumerate()
                    .skip(trough_index + 1)
                    .find(|(_, (_, value))| *value >= peak_value)
                    .map(|(index, _)| index)
            });
    DrawdownAnalysis {
        points: drawdown_points,
        maximum,
        peak_date: maximum_peak_index.map(|index| points[index].0.to_string()),
        trough_date: maximum_trough_index.map(|index| points[index].0.to_string()),
        recovery_date: recovery_index.map(|index| points[index].0.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use core_domain::Multiplier;
    use strategy_dsl::{
        ComparisonOperator, Condition, LookbackWindow, PolicyAction, StrategyRule, ValueExpression,
    };
    use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};

    use super::*;

    fn prices(days: i64) -> Vec<BacktestPrice> {
        let start = NaiveDate::from_ymd_opt(2023, 1, 2).unwrap();
        (0..days)
            .filter_map(|offset| {
                let date = start + Duration::days(offset);
                (date.weekday().number_from_monday() <= 5).then_some(date)
            })
            .enumerate()
            .map(|(index, date)| {
                BacktestPrice::new(date, Decimal::new(10_000 + index as i64 * 5, 2)).unwrap()
            })
            .collect()
    }

    fn formula() -> StrategySpec {
        let window = LookbackWindow::new(20).unwrap();
        StrategySpec::new(
            PolicyRef::new(
                PolicyId::new("dsl_test_trend").unwrap(),
                PolicyVersion::new(1).unwrap(),
            ),
            "Test trend",
            vec![StrategyRule::new(
                Condition::compare(
                    ValueExpression::indicator(IndicatorSpec::MovingAverageDistance(window)),
                    ComparisonOperator::LessThan,
                    Decimal::ZERO,
                ),
                PolicyAction::set_opportunity_multiplier(Multiplier::MIN),
            )],
        )
        .unwrap()
    }

    fn always_overweight_formula() -> StrategySpec {
        StrategySpec::new(
            PolicyRef::new(
                PolicyId::new("dsl_test_overweight").unwrap(),
                PolicyVersion::new(1).unwrap(),
            ),
            "Always overweight",
            vec![StrategyRule::new(
                Condition::compare(
                    ValueExpression::indicator(IndicatorSpec::ClosePrice),
                    ComparisonOperator::GreaterThan,
                    Decimal::ZERO,
                ),
                PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(1.2)),
            )],
        )
        .unwrap()
    }

    fn always_skip_opportunity_formula() -> StrategySpec {
        StrategySpec::new(
            PolicyRef::new(
                PolicyId::new("dsl_test_core_only").unwrap(),
                PolicyVersion::new(1).unwrap(),
            ),
            "Always core only",
            vec![StrategyRule::new(
                Condition::compare(
                    ValueExpression::indicator(IndicatorSpec::ClosePrice),
                    ComparisonOperator::GreaterThan,
                    Decimal::ZERO,
                ),
                PolicyAction::skip_opportunity(),
            )],
        )
        .unwrap()
    }

    #[test]
    fn compares_formula_and_dca_on_one_common_normalized_window() {
        let history = prices(500);
        let start = NaiveDate::from_ymd_opt(2023, 3, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 4, 30).unwrap();
        let result = run_dynamic_backtest(DynamicBacktestRequest {
            symbol: "us.spy".to_owned(),
            start,
            end,
            monthly_day: 18,
            contribution_amount: Decimal::new(1_000, 0),
            prices: history,
            strategies: vec![
                BacktestStrategy::FixedDca,
                BacktestStrategy::Formula(formula()),
            ],
        })
        .unwrap();

        assert_eq!(result.symbol, "US.SPY");
        assert_eq!(result.series.len(), 2);
        assert!(result.contribution_count >= 12);
        assert_eq!(result.market_points[0].date, result.effective_start);
        assert_eq!(
            result.market_points.len(),
            result.series[0].normalized_points.len()
        );
        assert_eq!(result.series[0].normalized_points[0].value, 100.0);
        assert_eq!(
            result.series[0].execution_points.len(),
            result.contribution_count
        );
        assert_eq!(
            result.series[0].execution_points[0].invested_amount,
            1_000.0
        );
        assert_eq!(
            result.series[0].execution_points[0].budget_utilisation_percent,
            100.0
        );
        assert!(!result.series[0].execution_points[0].strategy_rule_matched);
        assert_eq!(
            result.series[0].execution_points[0].core_invested_amount,
            1_000.0
        );
        assert_eq!(
            result.series[0].execution_points[0].opportunity_invested_amount,
            0.0
        );
        assert_eq!(
            result.series[0].drawdown_points.len(),
            result.market_points.len()
        );
        assert_eq!(result.series[0].calculation_details.buy_cost_bps, 5.0);
        assert_eq!(
            result.series[0].calculation_details.daily_return_count,
            result.series[0].normalized_points.len() - 1
        );
        assert!(result.series[1]
            .execution_points
            .iter()
            .all(|point| !point.strategy_rule_matched));
        assert_eq!(
            result.series[0].normalized_points.len(),
            result.series[1].normalized_points.len()
        );
        assert_eq!(
            result.series[0].metrics.total_contributed,
            result.series[1].metrics.total_contributed
        );
    }

    #[test]
    fn rejects_non_price_indicator_in_price_only_backtest() {
        let spec = StrategySpec::new(
            PolicyRef::new(
                PolicyId::new("dsl_test_vix").unwrap(),
                PolicyVersion::new(1).unwrap(),
            ),
            "VIX",
            vec![StrategyRule::new(
                Condition::compare(
                    ValueExpression::indicator(IndicatorSpec::Vix),
                    ComparisonOperator::GreaterThan,
                    Decimal::new(20, 0),
                ),
                PolicyAction::skip_opportunity(),
            )],
        )
        .unwrap();
        let history = prices(100);
        let error = run_dynamic_backtest(DynamicBacktestRequest {
            symbol: "US.SPY".to_owned(),
            start: history[20].date(),
            end: history.last().unwrap().date(),
            monthly_day: 18,
            contribution_amount: Decimal::new(1_000, 0),
            prices: history,
            strategies: vec![BacktestStrategy::Formula(spec)],
        })
        .unwrap_err();

        assert!(matches!(
            error,
            DynamicBacktestError::ExternalIndicatorRequired
        ));
    }

    #[test]
    fn formula_never_reads_execution_close_as_decision_evidence() {
        let baseline = prices(300);
        let mut shocked = baseline.clone();
        let start = NaiveDate::from_ymd_opt(2023, 8, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2023, 8, 31).unwrap();
        let execution = shocked
            .iter_mut()
            .find(|price| {
                price.date.year() == 2023 && price.date.month() == 8 && price.date.day() >= 18
            })
            .unwrap();
        execution.adjusted_close = Decimal::new(1_000_000, 0);

        let evaluate = |history| {
            run_dynamic_backtest(DynamicBacktestRequest {
                symbol: "US.SPY".to_owned(),
                start,
                end,
                monthly_day: 18,
                contribution_amount: Decimal::new(1_000, 0),
                prices: history,
                strategies: vec![BacktestStrategy::Formula(formula())],
            })
            .unwrap()
        };
        let baseline_result = evaluate(baseline);
        let shocked_result = evaluate(shocked);

        assert_eq!(baseline_result.contribution_count, 1);
        assert_eq!(shocked_result.contribution_count, 1);
        assert_ne!(
            baseline_result.series[0].execution_points[0].adjusted_close,
            shocked_result.series[0].execution_points[0].adjusted_close
        );
        assert_eq!(
            baseline_result.series[0].metrics.total_invested,
            shocked_result.series[0].metrics.total_invested
        );
    }

    #[test]
    fn formula_backtest_uses_the_same_expiring_one_period_budget_as_new_plans() {
        let history = prices(40);
        let spend = strategy_spend(
            &BacktestStrategy::Formula(always_overweight_formula()),
            Decimal::new(1_000, 0),
            &history,
        )
        .unwrap();

        assert_eq!(spend.amount, Decimal::new(1_000, 0));
        assert!(spend.strategy_rule_matched);
        assert_eq!(
            formula_configuration().unwrap().opportunity_cash_policy(),
            OpportunityCashPolicy::ExpireEachPeriod
        );
    }

    #[test]
    fn execution_points_expose_the_simulated_price_and_budget_share() {
        let history = prices(100);
        let result = run_dynamic_backtest(DynamicBacktestRequest {
            symbol: "US.SPY".to_owned(),
            start: NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
            end: history.last().unwrap().date(),
            monthly_day: 18,
            contribution_amount: Decimal::new(1_000, 0),
            prices: history,
            strategies: vec![BacktestStrategy::Formula(always_skip_opportunity_formula())],
        })
        .unwrap();

        let execution = &result.series[0].execution_points[0];
        let market = result
            .market_points
            .iter()
            .find(|point| point.date == execution.date)
            .unwrap();
        assert_eq!(execution.adjusted_close, market.adjusted_close);
        assert_eq!(execution.invested_amount, 700.0);
        assert_eq!(execution.budget_utilisation_percent, 70.0);
        assert_eq!(execution.scheduled_contribution_amount, 1_000.0);
        assert_eq!(execution.core_invested_amount, 700.0);
        assert_eq!(execution.opportunity_invested_amount, 0.0);
        assert_eq!(execution.unallocated_amount, 300.0);
        assert!((execution.transaction_cost - 0.349_825).abs() < 0.000_001);
        assert!(execution.strategy_rule_matched);
        assert!(result.series[0].calculation_details.total_transaction_cost > 0.0);
        assert_eq!(
            result.series[0].calculation_details.rule_matched_count,
            result.series[0].execution_points.len()
        );
        assert_eq!(
            result.series[0]
                .calculation_details
                .standard_execution_count,
            0
        );
    }

    #[test]
    fn drawdown_details_track_peak_trough_and_recovery_dates() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
        let points = [100.0, 120.0, 90.0, 110.0, 121.0]
            .into_iter()
            .enumerate()
            .map(|(index, value)| (start + Duration::days(index as i64), value))
            .collect::<Vec<_>>();

        let analysis = drawdown_analysis(&points);

        assert!((analysis.maximum - 0.25).abs() < f64::EPSILON);
        assert_eq!(analysis.peak_date.as_deref(), Some("2026-01-03"));
        assert_eq!(analysis.trough_date.as_deref(), Some("2026-01-04"));
        assert_eq!(analysis.recovery_date.as_deref(), Some("2026-01-06"));
        assert_eq!(analysis.points[2].value_percent, -25.0);
    }
}
