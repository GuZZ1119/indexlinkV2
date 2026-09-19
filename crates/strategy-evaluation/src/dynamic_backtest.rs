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

use crate::{maximum_drawdown, xirr};

const BUY_COST_BPS: f64 = 5.0;
const MAX_SINGLE_EXECUTION_MULTIPLIER: i64 = 15;

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
    /// Total amount converted into asset units, before transaction costs.
    pub total_invested: f64,
    /// Share of contributed cash converted into asset units.
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
    /// Metrics calculated from this exact trajectory and its contribution ledger.
    pub metrics: BacktestMetrics,
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
    let mut opportunity_cash = Decimal::ZERO;
    let mut schedule_cursor = 0;
    for index in start_index..=end_index {
        let price = request.prices[index];
        if schedule.get(schedule_cursor).copied() == Some(index) {
            state.deposit(price.date, request.contribution_amount)?;
            let spend = strategy_spend(
                strategy,
                request.contribution_amount,
                opportunity_cash,
                &request.prices[..index],
            )?;
            opportunity_cash = spend.next_opportunity_cash;
            state.buy(spend.amount, price.adjusted_close)?;
            schedule_cursor += 1;
        }
        state.mark(price.date, price.adjusted_close)?;
    }
    state.finish(strategy)
}

struct StrategySpend {
    amount: Decimal,
    next_opportunity_cash: Decimal,
}

fn strategy_spend(
    strategy: &BacktestStrategy,
    contribution: Decimal,
    opportunity_cash: Decimal,
    history: &[BacktestPrice],
) -> Result<StrategySpend, DynamicBacktestError> {
    if matches!(strategy, BacktestStrategy::FixedDca) {
        return Ok(StrategySpend {
            amount: contribution,
            next_opportunity_cash: Decimal::ZERO,
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
    let recommendation = spec.evaluate(&context)?.recommendation().clone();
    let configuration = formula_configuration()?;
    let maximum = contribution * Decimal::new(MAX_SINGLE_EXECUTION_MULTIPLIER, 1);
    let split = TwoBucketContributionSplit::from_decision_with_carry(
        contribution,
        maximum,
        configuration,
        recommendation.action(),
        recommendation.multiplier(),
        opportunity_cash,
    )?;
    Ok(StrategySpend {
        amount: split.recommended_contribution(),
        next_opportunity_cash: (opportunity_cash + split.opportunity_budget()
            - split.opportunity_contribution())
        .max(Decimal::ZERO),
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
        OpportunityCashPolicy::CarryForward,
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
    last_value: f64,
    pending_flow: f64,
    nav: f64,
    points: Vec<(NaiveDate, f64)>,
    daily_returns: Vec<f64>,
    flows: Vec<(NaiveDate, f64)>,
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

    fn buy(&mut self, amount: Decimal, price: Decimal) -> Result<(), DynamicBacktestError> {
        let amount = amount
            .to_f64()
            .filter(|value| value.is_finite())
            .ok_or(DynamicBacktestError::InvalidRequest)?
            .clamp(0.0, self.cash);
        let price = price
            .to_f64()
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or(DynamicBacktestError::InvalidPrice)?;
        self.cash -= amount;
        self.invested += amount;
        self.units += amount / (price * (1.0 + BUY_COST_BPS / 10_000.0));
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
        let nav_values = self
            .points
            .iter()
            .map(|(_, value)| *value)
            .collect::<Vec<_>>();
        let total_return = last_nav / first_nav - 1.0;
        let elapsed_days = (last_date - first_date).num_days();
        let annualized_return = (elapsed_days > 0)
            .then(|| ((last_nav / first_nav).powf(365.25 / elapsed_days as f64) - 1.0) * 100.0);
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
            metrics: BacktestMetrics {
                total_return_percent: total_return * 100.0,
                annualized_return_percent: annualized_return,
                xirr_percent: xirr(&self.flows).map(|value| value * 100.0),
                maximum_drawdown_percent: maximum_drawdown(&nav_values) * 100.0,
                annualized_volatility_percent: annualized_volatility_daily(&self.daily_returns)
                    .map(|value| value * 100.0),
                sortino_ratio: sortino_ratio_daily(&self.daily_returns),
                total_contributed: self.contributed,
                total_invested: self.invested,
                cash_utilisation_percent: self.invested / self.contributed * 100.0,
                terminal_wealth: self.last_value,
                terminal_cash: self.cash,
            },
        })
    }
}

fn annualized_volatility_daily(returns: &[f64]) -> Option<f64> {
    if returns.len() < 2 {
        return None;
    }
    let average = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns
        .iter()
        .map(|value| (value - average).powi(2))
        .sum::<f64>()
        / (returns.len() - 1) as f64;
    Some(variance.sqrt() * 252.0_f64.sqrt())
}

fn sortino_ratio_daily(returns: &[f64]) -> Option<f64> {
    if returns.len() < 2 {
        return None;
    }
    let average = returns.iter().sum::<f64>() / returns.len() as f64;
    let downside_deviation = (returns
        .iter()
        .map(|value| value.min(0.0).powi(2))
        .sum::<f64>()
        / returns.len() as f64)
        .sqrt();
    (downside_deviation > 0.0).then(|| average / downside_deviation * 252.0_f64.sqrt())
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
        assert_eq!(result.series[0].normalized_points[0].value, 100.0);
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
        assert_eq!(
            baseline_result.series[0].metrics.total_invested,
            shocked_result.series[0].metrics.total_invested
        );
    }
}
