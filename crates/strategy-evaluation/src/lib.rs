#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Reproducible, offline-only calibration of the current IndexLink strategy.
//!
//! This crate is intentionally separate from production composition roots. It
//! reads only committed fixture data, invokes the real quant, decision, and
//! two-bucket domain functions, and never performs network or broker IO.

use std::collections::{BTreeMap, BTreeSet};

use ai_client::Sentiment;
use chrono::{Datelike, NaiveDate};
use core_domain::{Action, Multiplier, Percentile};
use decision_engine::{
    evaluate_decision, DecisionConfig, DecisionInput, DecisionSentiment, DecisionSignal,
};
use investment_plans::{
    BucketAllocationRatio, OpportunityCashPolicy, PlanExecutionConfiguration, PlanRiskMode,
    TwoBucketAllocationConfig, TwoBucketContributionSplit,
};
use quant_engine::{
    evaluate_fundamental, evaluate_trend, FundamentalConfig, FundamentalSignal,
    FundamentalSnapshot, TrendSignal, TrendSnapshot,
};
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use strategy_dsl::{
    ComparisonOperator, Condition, DslEvidence, IndicatorSpec, LookbackWindow, PolicyAction,
    StrategyDslRuntimeError, StrategyDslValidationError, StrategyRule, StrategySpec,
    ValueExpression,
};
use strategy_policy::{DecisionContext, PolicyId, PolicyRef, PolicyValidationError, PolicyVersion};
use thiserror::Error;
use time::{Date, Month};

const PERIOD_BUDGET: i64 = 1_000;
const MAX_SINGLE_EXECUTION: i64 = 1_500;
const CORE_RATIO: i64 = 7;
const OPPORTUNITY_RATIO: i64 = 3;
const BUY_COST_BPS: f64 = 5.0;
const OOS_WINDOW_MONTHS: usize = 24;
const OOS_STEP_MONTHS: usize = 12;
const TREND_CAP_FLOOR: f64 = 0.25;
const TREND_CAP_RISK_START: f64 = 0.50;
const C3_FUNDAMENTAL_FLOOR: f64 = 0.75;
const C3_ADDITION_CEILING: f64 = 1.25;
const C3_TAIL_RISK_START: f64 = 0.75;
const C4_FUNDAMENTAL_FLOOR: f64 = 0.85;
const C4_ADDITION_CEILING: f64 = 1.15;
const C4_FUNDAMENTAL_LOOKBACK_MONTHS: usize = 12;
const C4_OPPORTUNITY_CARRY_PERIODS: usize = 3;
const TECHNICAL_FIXTURE_MANIFEST: &str =
    include_str!("../data/generated/technical-v1.manifest.json");
const FRED_SP500_DAILY: &str = include_str!("../data/raw/fred_sp500_daily.csv");
const FRED_NASDAQCOM_DAILY: &str = include_str!("../data/raw/fred_nasdaqcom_daily.csv");
const CBOE_VIX_DAILY: &str = include_str!("../data/raw/cboe_vix_daily.csv");

/// Errors returned while reading or evaluating the committed calibration fixture.
#[derive(Debug, Error)]
pub enum EvaluationError {
    /// The committed fixture JSON is malformed.
    #[error("calibration fixture is invalid")]
    Fixture(#[from] serde_json::Error),
    /// A fixture date cannot be parsed as ISO `YYYY-MM-DD`.
    #[error("calibration fixture contains an invalid date")]
    InvalidDate,
    /// The fixture does not contain enough earlier monthly observations.
    #[error("calibration fixture has insufficient history")]
    InsufficientHistory,
    /// One of the real quant functions rejected a fixture observation.
    #[error(transparent)]
    Quant(#[from] quant_engine::QuantError),
    /// The two-bucket domain function rejected a fixed baseline configuration.
    #[error(transparent)]
    Plan(#[from] investment_plans::PlanValidationError),
    /// A versioned DSL strategy definition is invalid.
    #[error(transparent)]
    DslValidation(#[from] StrategyDslValidationError),
    /// The deterministic DSL interpreter rejected an evidence snapshot.
    #[error(transparent)]
    DslRuntime(#[from] StrategyDslRuntimeError),
    /// A strategy policy identifier, version, or context is invalid.
    #[error(transparent)]
    Policy(#[from] PolicyValidationError),
    /// A fixture number cannot be represented by the Decimal-based DSL evidence.
    #[error("calibration fixture contains an unsupported decimal value")]
    InvalidDecimal,
    /// The committed technical-fixture manifest cannot be parsed or violates its schema.
    #[error("technical fixture manifest is invalid")]
    TechnicalFixtureManifest,
    /// One committed technical-fixture source violates its recorded integrity contract.
    #[error("technical fixture integrity check failed")]
    TechnicalFixtureIntegrity,
}

/// Summary produced after validating the committed technical historical fixture.
///
/// Validation reads compile-time embedded files only. It deliberately performs
/// no network I/O, so repeated tests use exactly the snapshot reviewed in Git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechnicalFixtureSummary {
    /// Immutable fixture version declared by its manifest.
    pub dataset_version: String,
    /// Calendar date on which all listed raw snapshots were captured.
    pub captured_on: String,
    /// Inclusive common date range guaranteed by every validated series.
    pub coverage_start: String,
    /// Inclusive common date range guaranteed by every validated series.
    pub coverage_end: String,
    /// Per-series validated observation and source-gap counts.
    pub series: Vec<TechnicalFixtureSeriesSummary>,
}

/// Validation summary for one committed raw technical series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechnicalFixtureSeriesSummary {
    /// Stable manifest identifier for the series.
    pub id: String,
    /// Source series identifier, such as `SP500`, `NASDAQCOM`, or `VIX`.
    pub source_symbol: String,
    /// Optional ETF name for which an index series is used only as a proxy.
    pub proxy_for: Option<String>,
    /// Number of valid positive close observations retained from the raw snapshot.
    pub observations: usize,
    /// Number of source rows with an explicit blank or `.` close that were excluded without fill.
    pub source_gap_rows: usize,
    /// First valid observation date in the source snapshot.
    pub first_observation: String,
    /// Last valid observation date in the source snapshot.
    pub last_observation: String,
}

/// Validate the immutable daily US-equity-proxy and VIX research inputs.
///
/// The fixture contains FRED S&P 500 and NASDAQ Composite **index proxies**
/// for SPY and QQQ, not ETF total-return execution prices. Every raw file is
/// embedded with [`include_str!`], checked against its manifest SHA-256, and
/// parsed without silently filling source gaps or reading the network.
pub fn validate_technical_fixture() -> Result<TechnicalFixtureSummary, EvaluationError> {
    validate_technical_fixture_contents(
        TECHNICAL_FIXTURE_MANIFEST,
        &[
            ("fred_sp500_daily.csv", FRED_SP500_DAILY),
            ("fred_nasdaqcom_daily.csv", FRED_NASDAQCOM_DAILY),
            ("cboe_vix_daily.csv", CBOE_VIX_DAILY),
        ],
    )
}

/// A complete machine-readable result for calibration-v2.
#[derive(Debug, Serialize)]
pub struct CalibrationReport {
    dataset_version: String,
    assumptions: Assumptions,
    strategy_catalog: Vec<StrategyDefinition>,
    assets: Vec<AssetReport>,
    qwen_sensitivity: QwenSensitivityReport,
}

/// Serialize an evaluation report as deterministic human-readable JSON.
pub fn report_json(report: &CalibrationReport) -> Result<String, EvaluationError> {
    serde_json::to_string_pretty(report).map_err(EvaluationError::from)
}

/// Evaluate the committed calibration-v2 fixture using unmodified production defaults.
pub fn evaluate_fixture() -> Result<CalibrationReport, EvaluationError> {
    let dataset: FixtureDataset =
        serde_json::from_str(include_str!("../data/generated/calibration-v2.json"))?;
    let configuration = execution_configuration(Decimal::new(CORE_RATIO, 1))?;
    let mut evaluated_assets = Vec::new();
    let mut fallback_samples = Vec::new();

    for asset in &dataset.assets {
        let samples = evaluate_asset(asset)?;
        fallback_samples.extend(samples.iter().cloned());
        evaluated_assets.push(asset_report(asset, &samples, configuration)?);
    }

    Ok(CalibrationReport {
        dataset_version: dataset.dataset_version,
        assumptions: Assumptions {
            contribution_schedule: "monthly decision after the final available observation; execution at the first strictly later daily observation".to_owned(),
            period_budget_usd: PERIOD_BUDGET as f64,
            core_ratio: CORE_RATIO as f64 / 10.0,
            opportunity_ratio: OPPORTUNITY_RATIO as f64 / 10.0,
            buy_cost_bps: BUY_COST_BPS,
            cost_model: "Each strategy buys at the first strictly later daily close × (1 + buy_cost_bps / 10,000); cash, including unallocated opportunity cash, remains in terminal wealth and earns no interest.".to_owned(),
            historical_ai_policy: "Historical causal results use DecisionSentiment::Unavailable and the current 90/10/0 fallback.".to_owned(),
        },
        strategy_catalog: strategy_catalog(),
        assets: evaluated_assets,
        qwen_sensitivity: qwen_sensitivity(&fallback_samples),
    })
}

/// Fixed-fixture admission report required before a DSL version can be activated online.
#[derive(Debug, Serialize)]
pub struct StrategyAdmissionReport {
    /// Whether the strategy passed deterministic safety and fixed-sample evaluation gates.
    pub eligible: bool,
    /// Human-readable reason when activation must remain blocked.
    pub reason: Option<String>,
    /// Core-bucket safety result; this must always be true for a DSL strategy.
    pub core_bucket_safe: bool,
    /// Period-budget validation result for the fixed sample budget.
    pub budget_safe: bool,
    /// Matched fixed-fixture comparison with Fixed DCA, when the needed inputs exist.
    pub assets: Vec<StrategyAdmissionAsset>,
}

/// One asset-level fixed-sample comparison used by the activation gate.
#[derive(Debug, Serialize)]
pub struct StrategyAdmissionAsset {
    /// Fixture symbol.
    pub symbol: String,
    /// Number of matched contribution observations.
    pub observations: usize,
    /// Candidate metrics under the same contribution schedule as Fixed DCA.
    pub strategy: StrategyAdmissionMetrics,
    /// Fixed DCA reference metrics.
    pub fixed_dca: StrategyAdmissionMetrics,
}

/// Public subset of comparable, non-promotional admission metrics.
#[derive(Debug, Serialize)]
pub struct StrategyAdmissionMetrics {
    /// Terminal wealth after all fixed fixture observations.
    pub terminal_wealth_usd: f64,
    /// Maximum peak-to-trough drawdown percentage.
    pub maximum_drawdown_percent: f64,
    /// Annualized monthly-return volatility when observable.
    pub annualized_volatility_percent: Option<f64>,
    /// Share of external cash invested by the evaluated strategy.
    pub cash_utilisation_percent: f64,
}

/// Evaluate one restricted strategy against the committed calibration fixture for activation.
///
/// The fixture only includes causal RSI-14 and VIX inputs.  A strategy that requires other
/// indicators remains valid for online simulation but is intentionally ineligible until an
/// equally versioned historical fixture is added; no synthetic backtest is substituted.
pub fn evaluate_strategy_admission(
    strategy: &StrategySpec,
) -> Result<StrategyAdmissionReport, EvaluationError> {
    let core_bucket_safe = strategy
        .rules()
        .iter()
        .all(|rule| rule.action().is_opportunity_only());
    let budget = Decimal::new(PERIOD_BUDGET, 0);
    let budget_safe = strategy.validate_for_budget(budget).is_ok();
    let rsi = IndicatorSpec::RelativeStrengthIndex(LookbackWindow::new(14)?);
    let fixture_supported = strategy
        .required_indicators()
        .iter()
        .all(|indicator| *indicator == rsi || matches!(indicator, IndicatorSpec::Vix));
    if !core_bucket_safe || !budget_safe || !fixture_supported {
        let reason = if !core_bucket_safe {
            "the strategy is not opportunity-bucket-only and would be able to affect the fixed core contribution"
        } else if !budget_safe {
            "the strategy can recommend an opportunity amount above the fixed-sample period budget"
        } else {
            "fixed calibration fixture currently supports only RSI(14) and VIX; add versioned historical inputs before activating this strategy"
        };
        return Ok(StrategyAdmissionReport {
            eligible: false,
            reason: Some(reason.to_owned()),
            core_bucket_safe,
            budget_safe,
            assets: Vec::new(),
        });
    }
    let dataset: FixtureDataset =
        serde_json::from_str(include_str!("../data/generated/calibration-v2.json"))?;
    let configuration = execution_configuration(Decimal::new(CORE_RATIO, 1))?;
    let mut assets = Vec::new();
    for asset in &dataset.assets {
        let samples = evaluate_asset(asset)?;
        let strategy_metrics = simulate_admission_dsl(&samples, configuration, strategy)?;
        let dca = simulate(&samples, configuration, ExecutionMode::FixedDca)?;
        assets.push(StrategyAdmissionAsset {
            symbol: asset.source_symbol.clone(),
            observations: samples.len(),
            strategy: admission_metrics(strategy_metrics),
            fixed_dca: admission_metrics(dca),
        });
    }
    Ok(StrategyAdmissionReport {
        eligible: true,
        reason: None,
        core_bucket_safe,
        budget_safe,
        assets,
    })
}

fn simulate_admission_dsl(
    samples: &[DecisionMonth],
    configuration: PlanExecutionConfiguration,
    strategy: &StrategySpec,
) -> Result<PerformanceMetrics, EvaluationError> {
    let budget = Decimal::new(PERIOD_BUDGET, 0);
    let maximum = Decimal::new(MAX_SINGLE_EXECUTION, 0);
    let rsi = IndicatorSpec::RelativeStrengthIndex(LookbackWindow::new(14)?);
    let mut cash = Decimal::ZERO;
    let mut state = PortfolioState::default();
    for row in samples {
        state.deposit(row.decision_date, PERIOD_BUDGET as f64);
        let evidence = DslEvidence::new([
            (
                rsi,
                Decimal::from_f64(row.rsi14).ok_or(EvaluationError::InvalidDecimal)?,
            ),
            (
                IndicatorSpec::Vix,
                Decimal::from_f64(row.vix).ok_or(EvaluationError::InvalidDecimal)?,
            ),
        ])?;
        let evaluation = strategy.evaluate(&DecisionContext::new(
            fixture_date(row.decision_date)?,
            budget,
            evidence,
        )?)?;
        let split = TwoBucketContributionSplit::from_decision_with_carry(
            budget,
            maximum,
            configuration,
            evaluation.recommendation().action(),
            evaluation.recommendation().multiplier(),
            cash,
        )?;
        cash = (cash + split.opportunity_budget() - split.opportunity_contribution())
            .max(Decimal::ZERO);
        state.buy(
            row.execution_date,
            split
                .recommended_contribution()
                .to_f64()
                .ok_or(EvaluationError::InvalidDecimal)?,
            row.execution_close,
        );
        state.mark_to_market(row.execution_date, row.execution_close);
    }
    Ok(state.metrics(cash.to_f64().unwrap_or_default()))
}

fn admission_metrics(metrics: PerformanceMetrics) -> StrategyAdmissionMetrics {
    StrategyAdmissionMetrics {
        terminal_wealth_usd: metrics.terminal_wealth_usd,
        maximum_drawdown_percent: metrics.maximum_drawdown_percent,
        annualized_volatility_percent: metrics.annualized_volatility_percent,
        cash_utilisation_percent: metrics.cash_utilisation_percent,
    }
}

#[derive(Debug, Deserialize)]
struct TechnicalFixtureManifest {
    schema_version: u8,
    dataset_version: String,
    captured_on: String,
    coverage: TechnicalFixtureCoverage,
    market_timezone: String,
    frequency: String,
    missing_value_rule: String,
    network_rule: String,
    series: Vec<TechnicalFixtureSeries>,
}

#[derive(Debug, Deserialize)]
struct TechnicalFixtureCoverage {
    start: String,
    end: String,
}

#[derive(Debug, Deserialize)]
struct TechnicalFixtureSeries {
    id: String,
    source_symbol: String,
    proxy_for: Option<String>,
    file: String,
    source_url: String,
    source_terms: String,
    date_format: String,
    date_column: String,
    close_column: String,
    sha256: String,
}

fn validate_technical_fixture_contents(
    manifest_source: &str,
    raw_sources: &[(&str, &str)],
) -> Result<TechnicalFixtureSummary, EvaluationError> {
    let manifest: TechnicalFixtureManifest = serde_json::from_str(manifest_source)
        .map_err(|_| EvaluationError::TechnicalFixtureManifest)?;
    if manifest.schema_version != 1
        || manifest.dataset_version.trim().is_empty()
        || manifest.market_timezone != "America/New_York"
        || manifest.frequency != "daily close"
        || manifest.missing_value_rule.trim().is_empty()
        || manifest.network_rule != "offline-only; source files are embedded at compile time"
        || manifest.series.is_empty()
        || manifest.series.len() != raw_sources.len()
    {
        return Err(EvaluationError::TechnicalFixtureManifest);
    }

    let captured_on = parse_technical_date(&manifest.captured_on, "%Y-%m-%d")?;
    let coverage_start = parse_technical_date(&manifest.coverage.start, "%Y-%m-%d")?;
    let coverage_end = parse_technical_date(&manifest.coverage.end, "%Y-%m-%d")?;
    if captured_on < coverage_end || coverage_start > coverage_end {
        return Err(EvaluationError::TechnicalFixtureManifest);
    }

    let mut ids = BTreeSet::new();
    let mut files = BTreeSet::new();
    let mut summaries = Vec::with_capacity(manifest.series.len());
    for series in &manifest.series {
        if series.id.trim().is_empty()
            || series.source_symbol.trim().is_empty()
            || series.source_url.trim().is_empty()
            || series.source_terms.trim().is_empty()
            || series.sha256.len() != 64
            || !ids.insert(&series.id)
            || !files.insert(&series.file)
        {
            return Err(EvaluationError::TechnicalFixtureManifest);
        }
        let source = raw_sources
            .iter()
            .find(|(file, _)| *file == series.file)
            .map(|(_, contents)| *contents)
            .ok_or(EvaluationError::TechnicalFixtureManifest)?;
        let actual_hash = format!("{:x}", Sha256::digest(source.as_bytes()));
        if actual_hash != series.sha256 {
            return Err(EvaluationError::TechnicalFixtureIntegrity);
        }
        let parsed = parse_technical_series(series, source)?;
        if parsed.first_observation > coverage_start || parsed.last_observation < coverage_end {
            return Err(EvaluationError::TechnicalFixtureIntegrity);
        }
        summaries.push(TechnicalFixtureSeriesSummary {
            id: series.id.clone(),
            source_symbol: series.source_symbol.clone(),
            proxy_for: series.proxy_for.clone(),
            observations: parsed.observations,
            source_gap_rows: parsed.source_gap_rows,
            first_observation: parsed.first_observation.to_string(),
            last_observation: parsed.last_observation.to_string(),
        });
    }
    if files.len() != raw_sources.len() {
        return Err(EvaluationError::TechnicalFixtureManifest);
    }

    Ok(TechnicalFixtureSummary {
        dataset_version: manifest.dataset_version,
        captured_on: manifest.captured_on,
        coverage_start: manifest.coverage.start,
        coverage_end: manifest.coverage.end,
        series: summaries,
    })
}

struct ParsedTechnicalSeries {
    observations: usize,
    source_gap_rows: usize,
    first_observation: NaiveDate,
    last_observation: NaiveDate,
}

fn parse_technical_series(
    series: &TechnicalFixtureSeries,
    source: &str,
) -> Result<ParsedTechnicalSeries, EvaluationError> {
    let mut rows = source.lines();
    let header = rows
        .next()
        .ok_or(EvaluationError::TechnicalFixtureIntegrity)?;
    let columns = header.split(',').collect::<Vec<_>>();
    let date_index = columns
        .iter()
        .position(|column| *column == series.date_column)
        .ok_or(EvaluationError::TechnicalFixtureIntegrity)?;
    let close_index = columns
        .iter()
        .position(|column| *column == series.close_column)
        .ok_or(EvaluationError::TechnicalFixtureIntegrity)?;

    let mut observations = 0;
    let mut source_gap_rows = 0;
    let mut first_observation = None;
    let mut last_valid_observation = None;
    let mut last_source_observation = None;
    for row in rows.filter(|row| !row.trim().is_empty()) {
        let values = row.split(',').collect::<Vec<_>>();
        if values.len() != columns.len() {
            return Err(EvaluationError::TechnicalFixtureIntegrity);
        }
        let observed = parse_technical_date(values[date_index].trim(), &series.date_format)?;
        if last_source_observation.is_some_and(|previous| observed <= previous) {
            return Err(EvaluationError::TechnicalFixtureIntegrity);
        }
        last_source_observation = Some(observed);

        let close = values[close_index].trim();
        if close.is_empty() || close == "." {
            source_gap_rows += 1;
            continue;
        }
        let close = close
            .parse::<f64>()
            .map_err(|_| EvaluationError::TechnicalFixtureIntegrity)?;
        if !close.is_finite() || close <= 0.0 {
            return Err(EvaluationError::TechnicalFixtureIntegrity);
        }
        first_observation.get_or_insert(observed);
        last_valid_observation = Some(observed);
        observations += 1;
    }

    Ok(ParsedTechnicalSeries {
        observations,
        source_gap_rows,
        first_observation: first_observation.ok_or(EvaluationError::TechnicalFixtureIntegrity)?,
        last_observation: last_valid_observation
            .ok_or(EvaluationError::TechnicalFixtureIntegrity)?,
    })
}

fn parse_technical_date(value: &str, format: &str) -> Result<NaiveDate, EvaluationError> {
    NaiveDate::parse_from_str(value, format).map_err(|_| EvaluationError::TechnicalFixtureIntegrity)
}

#[derive(Debug, Deserialize)]
struct FixtureDataset {
    dataset_version: String,
    assets: Vec<FixtureAsset>,
}

#[derive(Debug, Deserialize)]
struct FixtureAsset {
    id: String,
    display_name: String,
    source_symbol: String,
    observations: Vec<FixtureObservation>,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureObservation {
    decision_as_of: String,
    execution_as_of: String,
    execution_close: f64,
    cape: f64,
    erp_proxy: f64,
    ma200_distance: f64,
    rsi14: f64,
    vix: f64,
}

#[derive(Debug, Serialize)]
struct Assumptions {
    contribution_schedule: String,
    period_budget_usd: f64,
    core_ratio: f64,
    opportunity_ratio: f64,
    buy_cost_bps: f64,
    cost_model: String,
    historical_ai_policy: String,
}

#[derive(Debug, Serialize)]
struct StrategyDefinition {
    id: String,
    label: String,
    status: String,
    mechanical_difference: String,
}

#[derive(Debug, Serialize)]
struct AssetReport {
    asset_id: String,
    display_name: String,
    source_symbol: String,
    decision_observations: usize,
    first_decision_date: String,
    last_decision_date: String,
    fallback_score_distribution: Distribution,
    fallback_action_distribution: BTreeMap<String, u32>,
    fallback_layer_calibration: LayerCalibration,
    core_opportunity_intent: PerformanceMetrics,
    current_api_effective: PerformanceMetrics,
    fixed_dca: PerformanceMetrics,
    intent_vs_dca_terminal_difference_percent: f64,
    current_api_vs_intent_terminal_difference_percent: f64,
    rolling_out_of_sample: Vec<RollingWindow>,
    experimental_candidates: Vec<CandidateReport>,
    allocation_sensitivity: Vec<AllocationSensitivity>,
}

#[derive(Debug, Serialize)]
struct Distribution {
    mean: f64,
    median: f64,
    p10: f64,
    p25: f64,
    p75: f64,
    p90: f64,
}

#[derive(Debug, Serialize)]
struct LayerCalibration {
    fundamental_raw_mean: f64,
    fundamental_directional_mean: f64,
    fundamental_weighted_contribution_mean: f64,
    trend_raw_mean: f64,
    trend_timing_mean: f64,
    trend_weighted_contribution_mean: f64,
    sentiment_input_mean_when_unavailable: f64,
    sentiment_weighted_contribution_mean: f64,
    final_score_mean: f64,
}

#[derive(Debug, Serialize)]
struct PerformanceMetrics {
    xirr_percent: Option<f64>,
    terminal_wealth_usd: f64,
    maximum_drawdown_percent: f64,
    annualized_volatility_percent: Option<f64>,
    sortino_ratio: Option<f64>,
    maximum_drawdown_recovery_months: Option<u32>,
    total_external_cash_usd: f64,
    total_invested_usd: f64,
    cash_utilisation_percent: f64,
    terminal_cash_usd: f64,
    terminal_opportunity_cash_usd: f64,
}

#[derive(Debug, Serialize)]
struct RollingWindow {
    start: String,
    end: String,
    months: usize,
    core_opportunity_intent: PerformanceMetrics,
    fixed_dca: PerformanceMetrics,
    terminal_difference_percent: f64,
}

#[derive(Debug, Serialize)]
struct CandidateReport {
    id: String,
    status: String,
    rule: String,
    performance: PerformanceMetrics,
    terminal_difference_vs_dca_percent: f64,
    rolling_out_of_sample: Vec<CandidateRollingWindow>,
    worst_rolling_window: Option<CandidateRollingWindow>,
    c4_cash_diagnostics: Option<C4CashDiagnostics>,
}

#[derive(Debug, Clone, Serialize)]
struct CandidateRollingWindow {
    start: String,
    end: String,
    months: usize,
    terminal_difference_vs_dca_percent: f64,
}

#[derive(Debug, Serialize)]
struct AllocationSensitivity {
    core_ratio: f64,
    opportunity_ratio: f64,
    fixed_dca: PerformanceMetrics,
    current_core_opportunity: PerformanceMetrics,
    c1_bounded_continuous: PerformanceMetrics,
    c2_trend_continuous_cap: PerformanceMetrics,
    c3_fundamental_with_trend_addition_cap: PerformanceMetrics,
    c4_bounded_budget_with_deadline: PerformanceMetrics,
}

#[derive(Debug, Serialize)]
struct C4CashDiagnostics {
    fundamental_warmup_months: usize,
    maximum_carry_periods: usize,
    forced_catch_up_usd: f64,
    released_for_additional_investment_usd: f64,
    matured_lot_count: usize,
    maximum_deferred_opportunity_cash_usd: f64,
}

#[derive(Debug, Serialize)]
struct QwenSensitivityReport {
    scope: String,
    frozen_score_version: String,
    sample_count: usize,
    fallback_action_distribution: BTreeMap<String, u32>,
    normal_70_20_10_action_distribution: BTreeMap<String, u32>,
    fallback_score_distribution: Distribution,
    normal_70_20_10_score_distribution: Distribution,
    mean_normal_minus_fallback_score: f64,
}

#[derive(Debug, Deserialize)]
struct FrozenQwenSensitivity {
    version: String,
    purpose: String,
    scores: Vec<f64>,
}

#[derive(Clone)]
struct DecisionMonth {
    decision_date: NaiveDate,
    execution_date: NaiveDate,
    execution_close: f64,
    rsi14: f64,
    vix: f64,
    fundamental: FundamentalSignal,
    trend: TrendSignal,
    fallback: DecisionSignal,
}

fn execution_configuration(
    core_ratio: Decimal,
) -> Result<PlanExecutionConfiguration, EvaluationError> {
    let opportunity_ratio = Decimal::ONE - core_ratio;
    let allocation = TwoBucketAllocationConfig::new(
        BucketAllocationRatio::new(core_ratio)?,
        BucketAllocationRatio::new(opportunity_ratio)?,
    )?;
    PlanExecutionConfiguration::new_with_cash_policy(
        allocation,
        if opportunity_ratio.is_zero() {
            PlanRiskMode::Fixed
        } else {
            PlanRiskMode::Autopilot
        },
        if opportunity_ratio.is_zero() {
            OpportunityCashPolicy::ExpireEachPeriod
        } else {
            OpportunityCashPolicy::CarryForward
        },
    )
    .map_err(EvaluationError::from)
}

fn strategy_catalog() -> Vec<StrategyDefinition> {
    vec![
        StrategyDefinition {
            id: "fixed_dca".to_owned(),
            label: "Fixed DCA / 固定定投".to_owned(),
            status: "benchmark".to_owned(),
            mechanical_difference: "Invest the full period budget on every scheduled execution date; it does not use fundamental, trend, AI, or opportunity cash.".to_owned(),
        },
        StrategyDefinition {
            id: "current_core_opportunity".to_owned(),
            label: "Current Core/Opportunity / 当前双桶".to_owned(),
            status: "current production-shaped research line".to_owned(),
            mechanical_difference: "Keep the core bucket, but let the current composite decision multiplier and action reduce the opportunity bucket; unspent opportunity cash rolls forward in this research configuration.".to_owned(),
        },
        StrategyDefinition {
            id: "bounded_continuous_opportunity_v1".to_owned(),
            label: "C1 bounded continuous / C1 连续有界".to_owned(),
            status: "experimental; not a production default".to_owned(),
            mechanical_difference: "Keep the core bucket and replace only the opportunity multiplier with 0.75 + 0.50 × current final score, bounded to [0.75, 1.25]; it removes the global trend veto.".to_owned(),
        },
        StrategyDefinition {
            id: "trend_continuous_opportunity_cap_v1".to_owned(),
            label: "C2 trend continuous cap / C2 趋势连续压低".to_owned(),
            status: "experimental; negative control".to_owned(),
            mechanical_difference: "Keep the core bucket, then multiply the fundamental-derived opportunity amount by a trend-risk cap in [0.25, 1.00]; it can still materially defer opportunity cash.".to_owned(),
        },
        StrategyDefinition {
            id: "fundamental_with_trend_addition_cap_v1".to_owned(),
            label: "C3 trend caps additions / C3 趋势只限加码".to_owned(),
            status: "experimental; not a production default".to_owned(),
            mechanical_difference: "Keep the core bucket; fundamental sets the opportunity multiplier in [0.75, 1.25], while trend only caps an overweight down to 1.00 and never emits TacticalDelay.".to_owned(),
        },
        StrategyDefinition {
            id: "bounded_budget_with_deadline_v1".to_owned(),
            label: "C4 bounded budget with deadline / C4 有期限预算调度".to_owned(),
            status: "experimental; predeclared; not a production default".to_owned(),
            mechanical_difference: "Keep the core bucket fixed. Rank only prior directional fundamental scores to set a [0.85, 1.15] opportunity tilt; trend caps only the part above 1.00. Deferred opportunity cash expires after three periods and is forcibly caught up, so it cannot accumulate indefinitely. AI remains explanatory-only.".to_owned(),
        },
        StrategyDefinition {
            id: "dsl_rsi_opportunity_guard_v1".to_owned(),
            label: "DSL RSI opportunity guard / DSL RSI 机会桶守卫".to_owned(),
            status: "experimental; deterministic runtime-backed; not a production default".to_owned(),
            mechanical_difference: "Evaluate the saved, restricted strategy-dsl AST in first-match order using only the decision-date RSI-14 evidence: below 35 applies a 1.10 opportunity multiplier, above 65 applies 0.85, otherwise the opportunity bucket remains standard. The core bucket remains fixed and execution uses the next observed trading day.".to_owned(),
        },
    ]
}

fn evaluate_asset(asset: &FixtureAsset) -> Result<Vec<DecisionMonth>, EvaluationError> {
    let mut output = Vec::new();
    for index in 60..asset.observations.len() {
        let current = &asset.observations[index];
        let history = &asset.observations[..index];
        let fundamental = evaluate_fundamental(
            &FundamentalSnapshot {
                cape_history: history.iter().map(|row| row.cape).collect(),
                cape_current: current.cape,
                erp_history: history.iter().map(|row| row.erp_proxy).collect(),
                erp_current: current.erp_proxy,
            },
            &FundamentalConfig::default(),
        )?;
        let trend = evaluate_trend(
            &TrendSnapshot {
                ma_distance_history: history.iter().map(|row| row.ma200_distance).collect(),
                ma_distance_current: current.ma200_distance,
                rsi_history: history.iter().map(|row| row.rsi14).collect(),
                rsi_current: current.rsi14,
                vix_history: history.iter().map(|row| row.vix).collect(),
                vix_current: current.vix,
            },
            &quant_engine::TrendConfig::default(),
        )?;
        let fallback = evaluate_decision(
            &DecisionInput {
                fundamental: fundamental.clone(),
                trend: trend.clone(),
                sentiment: DecisionSentiment::Unavailable,
            },
            &DecisionConfig::default(),
        );
        output.push(DecisionMonth {
            decision_date: NaiveDate::parse_from_str(&current.decision_as_of, "%Y-%m-%d")
                .map_err(|_| EvaluationError::InvalidDate)?,
            execution_date: NaiveDate::parse_from_str(&current.execution_as_of, "%Y-%m-%d")
                .map_err(|_| EvaluationError::InvalidDate)?,
            execution_close: current.execution_close,
            rsi14: current.rsi14,
            vix: current.vix,
            fundamental,
            trend,
            fallback,
        });
    }
    (!output.is_empty())
        .then_some(output)
        .ok_or(EvaluationError::InsufficientHistory)
}

fn asset_report(
    asset: &FixtureAsset,
    samples: &[DecisionMonth],
    configuration: PlanExecutionConfiguration,
) -> Result<AssetReport, EvaluationError> {
    let intent = simulate(samples, configuration, ExecutionMode::CoreOpportunityIntent)?;
    let current_api = simulate(samples, configuration, ExecutionMode::CurrentApiEffective)?;
    let dca = simulate(samples, configuration, ExecutionMode::FixedDca)?;
    let first = samples
        .first()
        .ok_or(EvaluationError::InsufficientHistory)?;
    let last = samples.last().ok_or(EvaluationError::InsufficientHistory)?;
    let rolling_out_of_sample = (0..samples.len())
        .step_by(OOS_STEP_MONTHS)
        .filter_map(|start| samples.get(start..start + OOS_WINDOW_MONTHS))
        .map(|window| {
            let intent = simulate(window, configuration, ExecutionMode::CoreOpportunityIntent)?;
            let dca = simulate(window, configuration, ExecutionMode::FixedDca)?;
            Ok(RollingWindow {
                start: window[0].decision_date.to_string(),
                end: window[window.len() - 1].decision_date.to_string(),
                months: window.len(),
                terminal_difference_percent: relative_difference(
                    intent.terminal_wealth_usd,
                    dca.terminal_wealth_usd,
                ),
                core_opportunity_intent: intent,
                fixed_dca: dca,
            })
        })
        .collect::<Result<Vec<_>, EvaluationError>>()?;
    let experimental_candidates = vec![
        bounded_continuous_candidate(samples, configuration, &dca)?,
        trend_continuous_cap_candidate(samples, configuration, &dca)?,
        fundamental_trend_addition_cap_candidate(samples, configuration, &dca)?,
        bounded_budget_with_deadline_candidate(samples, configuration, &dca)?,
        dsl_rsi_opportunity_candidate(samples, configuration, &dca)?,
    ];

    Ok(AssetReport {
        asset_id: asset.id.clone(),
        display_name: asset.display_name.clone(),
        source_symbol: asset.source_symbol.clone(),
        decision_observations: samples.len(),
        first_decision_date: first.decision_date.to_string(),
        last_decision_date: last.decision_date.to_string(),
        fallback_score_distribution: distribution(
            samples.iter().map(|row| row.fallback.final_score.value()),
        ),
        fallback_action_distribution: action_distribution(samples.iter().map(|row| &row.fallback)),
        fallback_layer_calibration: layer_calibration(samples),
        intent_vs_dca_terminal_difference_percent: relative_difference(
            intent.terminal_wealth_usd,
            dca.terminal_wealth_usd,
        ),
        current_api_vs_intent_terminal_difference_percent: relative_difference(
            current_api.terminal_wealth_usd,
            intent.terminal_wealth_usd,
        ),
        core_opportunity_intent: intent,
        current_api_effective: current_api,
        fixed_dca: dca,
        rolling_out_of_sample,
        experimental_candidates,
        allocation_sensitivity: allocation_sensitivity(samples)?,
    })
}

#[derive(Clone, Copy)]
enum ExecutionMode {
    FixedDca,
    CoreOpportunityIntent,
    CurrentApiEffective,
    CandidateBoundedContinuous,
    CandidateTrendContinuousCap,
    CandidateFundamentalTrendAdditionCap,
    CandidateDslRsiOpportunity,
}

fn simulate(
    samples: &[DecisionMonth],
    configuration: PlanExecutionConfiguration,
    mode: ExecutionMode,
) -> Result<PerformanceMetrics, EvaluationError> {
    let budget = Decimal::new(PERIOD_BUDGET, 0);
    let maximum = Decimal::new(MAX_SINGLE_EXECUTION, 0);
    let mut opportunity_cash = Decimal::ZERO;
    let mut state = PortfolioState::default();
    let dsl_strategy = dsl_rsi_opportunity_strategy()?;
    for row in samples {
        state.deposit(row.decision_date, PERIOD_BUDGET as f64);
        let (action, multiplier) = match mode {
            ExecutionMode::CandidateBoundedContinuous => {
                let multiplier = core_domain::Multiplier::new_clamped(
                    0.75 + 0.5 * row.fallback.final_score.value(),
                );
                (multiplier.to_action(), multiplier)
            }
            ExecutionMode::CandidateTrendContinuousCap => {
                let multiplier =
                    trend_continuous_cap_multiplier(row.fallback.fundamental_score, &row.trend);
                (multiplier.to_action(), multiplier)
            }
            ExecutionMode::CandidateFundamentalTrendAdditionCap => {
                let multiplier = fundamental_trend_addition_cap_multiplier(
                    row.fallback.fundamental_score,
                    &row.trend,
                );
                (multiplier.to_action(), multiplier)
            }
            ExecutionMode::CandidateDslRsiOpportunity => {
                dsl_rsi_opportunity_recommendation(&dsl_strategy, row, budget)?
            }
            _ => (row.fallback.action, row.fallback.multiplier),
        };
        let intended = TwoBucketContributionSplit::from_decision_with_carry(
            budget,
            maximum,
            configuration,
            action,
            multiplier,
            opportunity_cash,
        )?;
        let spend = match mode {
            ExecutionMode::FixedDca => budget,
            ExecutionMode::CoreOpportunityIntent => intended.recommended_contribution(),
            // The API submits the preserved core bucket even when the
            // opportunity bucket is reduced to zero by Skip/TacticalDelay.
            ExecutionMode::CurrentApiEffective => intended.recommended_contribution(),
            ExecutionMode::CandidateBoundedContinuous => intended.recommended_contribution(),
            ExecutionMode::CandidateTrendContinuousCap => intended.recommended_contribution(),
            ExecutionMode::CandidateFundamentalTrendAdditionCap => {
                intended.recommended_contribution()
            }
            ExecutionMode::CandidateDslRsiOpportunity => intended.recommended_contribution(),
        };
        if !matches!(mode, ExecutionMode::FixedDca) {
            opportunity_cash = (opportunity_cash + intended.opportunity_budget()
                - intended.opportunity_contribution())
            .max(Decimal::ZERO);
        }
        let spend = spend.to_f64().ok_or(EvaluationError::InsufficientHistory)?;
        state.buy(row.execution_date, spend, row.execution_close);
        state.mark_to_market(row.execution_date, row.execution_close);
    }
    Ok(state.metrics(opportunity_cash.to_f64().unwrap_or_default()))
}

fn dsl_rsi_opportunity_candidate(
    samples: &[DecisionMonth],
    configuration: PlanExecutionConfiguration,
    fixed_dca: &PerformanceMetrics,
) -> Result<CandidateReport, EvaluationError> {
    let performance = simulate(
        samples,
        configuration,
        ExecutionMode::CandidateDslRsiOpportunity,
    )?;
    let rolling_out_of_sample = (0..samples.len())
        .step_by(OOS_STEP_MONTHS)
        .filter_map(|start| samples.get(start..start + OOS_WINDOW_MONTHS))
        .map(|window| {
            let candidate = simulate(
                window,
                configuration,
                ExecutionMode::CandidateDslRsiOpportunity,
            )?;
            let dca = simulate(window, configuration, ExecutionMode::FixedDca)?;
            Ok(CandidateRollingWindow {
                start: window[0].decision_date.to_string(),
                end: window[window.len() - 1].decision_date.to_string(),
                months: window.len(),
                terminal_difference_vs_dca_percent: relative_difference(
                    candidate.terminal_wealth_usd,
                    dca.terminal_wealth_usd,
                ),
            })
        })
        .collect::<Result<Vec<_>, EvaluationError>>()?;
    Ok(CandidateReport {
        id: "dsl_rsi_opportunity_guard_v1".to_owned(),
        status: "experimental; deterministic runtime-backed; not a production default".to_owned(),
        rule: "Run the same restricted strategy-dsl runtime that future preview and scheduler integrations will call. At the decision date, RSI-14 < 35 sets a 1.10 opportunity multiplier; RSI-14 > 65 sets 0.85; otherwise no DSL rule matches and the opportunity bucket remains at 1.00. The 70% core bucket is fixed, no rule can emit TacticalDelay, and buys occur at the first strictly later observed close.".to_owned(),
        terminal_difference_vs_dca_percent: relative_difference(
            performance.terminal_wealth_usd,
            fixed_dca.terminal_wealth_usd,
        ),
        worst_rolling_window: worst_rolling_window(&rolling_out_of_sample),
        performance,
        rolling_out_of_sample,
        c4_cash_diagnostics: None,
    })
}

fn dsl_rsi_opportunity_strategy() -> Result<StrategySpec, EvaluationError> {
    let rsi = IndicatorSpec::RelativeStrengthIndex(LookbackWindow::new(14)?);
    StrategySpec::new(
        PolicyRef::new(
            PolicyId::new("dsl_rsi_opportunity_guard")?,
            PolicyVersion::new(1)?,
        ),
        "DSL RSI opportunity guard",
        vec![
            StrategyRule::new(
                Condition::compare(
                    ValueExpression::indicator(rsi),
                    ComparisonOperator::LessThan,
                    Decimal::new(35, 0),
                ),
                PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(1.10)),
            ),
            StrategyRule::new(
                Condition::compare(
                    ValueExpression::indicator(rsi),
                    ComparisonOperator::GreaterThan,
                    Decimal::new(65, 0),
                ),
                PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(0.85)),
            ),
        ],
    )
    .map_err(EvaluationError::from)
}

fn dsl_rsi_opportunity_recommendation(
    strategy: &StrategySpec,
    row: &DecisionMonth,
    budget: Decimal,
) -> Result<(Action, Multiplier), EvaluationError> {
    let as_of = fixture_date(row.decision_date)?;
    let rsi = IndicatorSpec::RelativeStrengthIndex(LookbackWindow::new(14)?);
    let evidence = DslEvidence::new([(
        rsi,
        Decimal::from_f64(row.rsi14).ok_or(EvaluationError::InvalidDecimal)?,
    )])?;
    let context = DecisionContext::new(as_of, budget, evidence)?;
    let evaluation = strategy.evaluate(&context)?;
    Ok((
        evaluation.recommendation().action(),
        evaluation.recommendation().multiplier(),
    ))
}

fn fixture_date(value: NaiveDate) -> Result<Date, EvaluationError> {
    let month = Month::try_from(value.month() as u8).map_err(|_| EvaluationError::InvalidDate)?;
    Date::from_calendar_date(value.year(), month, value.day() as u8)
        .map_err(|_| EvaluationError::InvalidDate)
}

fn bounded_continuous_candidate(
    samples: &[DecisionMonth],
    configuration: PlanExecutionConfiguration,
    fixed_dca: &PerformanceMetrics,
) -> Result<CandidateReport, EvaluationError> {
    let performance = simulate(
        samples,
        configuration,
        ExecutionMode::CandidateBoundedContinuous,
    )?;
    let rolling_out_of_sample = (0..samples.len())
        .step_by(OOS_STEP_MONTHS)
        .filter_map(|start| samples.get(start..start + OOS_WINDOW_MONTHS))
        .map(|window| {
            let candidate = simulate(
                window,
                configuration,
                ExecutionMode::CandidateBoundedContinuous,
            )?;
            let dca = simulate(window, configuration, ExecutionMode::FixedDca)?;
            Ok(CandidateRollingWindow {
                start: window[0].decision_date.to_string(),
                end: window[window.len() - 1].decision_date.to_string(),
                months: window.len(),
                terminal_difference_vs_dca_percent: relative_difference(
                    candidate.terminal_wealth_usd,
                    dca.terminal_wealth_usd,
                ),
            })
        })
        .collect::<Result<Vec<_>, EvaluationError>>()?;
    Ok(CandidateReport {
        id: "bounded_continuous_opportunity_v1".to_owned(),
        status: "experimental; not a production default".to_owned(),
        rule: "Keep the 70% core bucket; replace only opportunity execution with multiplier = 0.75 + 0.50 × current final_score, bounded to [0.75, 1.25]. Do not turn non-neutral trend regimes into a global order veto in this evaluation-only candidate.".to_owned(),
        terminal_difference_vs_dca_percent: relative_difference(
            performance.terminal_wealth_usd,
            fixed_dca.terminal_wealth_usd,
        ),
        performance,
        worst_rolling_window: worst_rolling_window(&rolling_out_of_sample),
        rolling_out_of_sample,
        c4_cash_diagnostics: None,
    })
}

fn trend_continuous_cap_candidate(
    samples: &[DecisionMonth],
    configuration: PlanExecutionConfiguration,
    fixed_dca: &PerformanceMetrics,
) -> Result<CandidateReport, EvaluationError> {
    let performance = simulate(
        samples,
        configuration,
        ExecutionMode::CandidateTrendContinuousCap,
    )?;
    let rolling_out_of_sample = (0..samples.len())
        .step_by(OOS_STEP_MONTHS)
        .filter_map(|start| samples.get(start..start + OOS_WINDOW_MONTHS))
        .map(|window| {
            let candidate = simulate(
                window,
                configuration,
                ExecutionMode::CandidateTrendContinuousCap,
            )?;
            let dca = simulate(window, configuration, ExecutionMode::FixedDca)?;
            Ok(CandidateRollingWindow {
                start: window[0].decision_date.to_string(),
                end: window[window.len() - 1].decision_date.to_string(),
                months: window.len(),
                terminal_difference_vs_dca_percent: relative_difference(
                    candidate.terminal_wealth_usd,
                    dca.terminal_wealth_usd,
                ),
            })
        })
        .collect::<Result<Vec<_>, EvaluationError>>()?;
    Ok(CandidateReport {
        id: "trend_continuous_opportunity_cap_v1".to_owned(),
        status: "experimental; not a production default".to_owned(),
        rule: "Keep the 70% core bucket fixed. Derive the opportunity multiplier from the directional fundamental score, then multiply it by a continuous trend-risk cap in [0.25, 1.00]. The cap starts above a 0.50 raw tail-risk percentile and uses max(MA200-distance, RSI, VIX) risk. Trend regime never emits TacticalDelay in this evaluation-only candidate.".to_owned(),
        terminal_difference_vs_dca_percent: relative_difference(
            performance.terminal_wealth_usd,
            fixed_dca.terminal_wealth_usd,
        ),
        performance,
        worst_rolling_window: worst_rolling_window(&rolling_out_of_sample),
        rolling_out_of_sample,
        c4_cash_diagnostics: None,
    })
}

fn fundamental_trend_addition_cap_candidate(
    samples: &[DecisionMonth],
    configuration: PlanExecutionConfiguration,
    fixed_dca: &PerformanceMetrics,
) -> Result<CandidateReport, EvaluationError> {
    let performance = simulate(
        samples,
        configuration,
        ExecutionMode::CandidateFundamentalTrendAdditionCap,
    )?;
    let rolling_out_of_sample = (0..samples.len())
        .step_by(OOS_STEP_MONTHS)
        .filter_map(|start| samples.get(start..start + OOS_WINDOW_MONTHS))
        .map(|window| {
            let candidate = simulate(
                window,
                configuration,
                ExecutionMode::CandidateFundamentalTrendAdditionCap,
            )?;
            let dca = simulate(window, configuration, ExecutionMode::FixedDca)?;
            Ok(CandidateRollingWindow {
                start: window[0].decision_date.to_string(),
                end: window[window.len() - 1].decision_date.to_string(),
                months: window.len(),
                terminal_difference_vs_dca_percent: relative_difference(
                    candidate.terminal_wealth_usd,
                    dca.terminal_wealth_usd,
                ),
            })
        })
        .collect::<Result<Vec<_>, EvaluationError>>()?;
    Ok(CandidateReport {
        id: "fundamental_with_trend_addition_cap_v1".to_owned(),
        status: "experimental; predeclared; not a production default".to_owned(),
        rule: "Keep the core bucket fixed. Use the directional fundamental score alone for an opportunity multiplier bounded to [0.75, 1.25]. Trend never reduces normal opportunity investment: it only caps a fundamental-driven overweight from 1.25 down continuously to 1.00 when max(MA200-distance, RSI, VIX) raw tail risk rises from 0.75 to 1.00. Trend never emits TacticalDelay in this evaluation-only candidate.".to_owned(),
        terminal_difference_vs_dca_percent: relative_difference(
            performance.terminal_wealth_usd,
            fixed_dca.terminal_wealth_usd,
        ),
        worst_rolling_window: worst_rolling_window(&rolling_out_of_sample),
        performance,
        rolling_out_of_sample,
        c4_cash_diagnostics: None,
    })
}

struct C4Simulation {
    performance: PerformanceMetrics,
    diagnostics: C4CashDiagnostics,
}

#[derive(Debug)]
struct DeferredOpportunityLot {
    amount_usd: f64,
    deadline_period: usize,
}

fn bounded_budget_with_deadline_candidate(
    samples: &[DecisionMonth],
    configuration: PlanExecutionConfiguration,
    fixed_dca: &PerformanceMetrics,
) -> Result<CandidateReport, EvaluationError> {
    let simulation = simulate_bounded_budget_with_deadline(samples, configuration)?;
    let rolling_out_of_sample = (0..samples.len())
        .step_by(OOS_STEP_MONTHS)
        .filter_map(|start| samples.get(start..start + OOS_WINDOW_MONTHS))
        .map(|window| {
            let candidate = simulate_bounded_budget_with_deadline(window, configuration)?;
            let dca = simulate(window, configuration, ExecutionMode::FixedDca)?;
            Ok(CandidateRollingWindow {
                start: window[0].decision_date.to_string(),
                end: window[window.len() - 1].decision_date.to_string(),
                months: window.len(),
                terminal_difference_vs_dca_percent: relative_difference(
                    candidate.performance.terminal_wealth_usd,
                    dca.terminal_wealth_usd,
                ),
            })
        })
        .collect::<Result<Vec<_>, EvaluationError>>()?;
    Ok(CandidateReport {
        id: "bounded_budget_with_deadline_v1".to_owned(),
        status: "experimental; predeclared; not a production default".to_owned(),
        rule: "Keep the core bucket fixed. For the opportunity bucket, rank the current directional fundamental score only against the preceding 12 decision scores; use the resulting rank to set a [0.85, 1.15] multiplier. Trend may cap only the part above 1.00. If a multiplier is below 1.00, the difference enters a dated opportunity-cash lot; each lot must be invested after at most three scheduled periods. Higher multipliers may spend existing non-expired lots but never borrow future cash. AI remains explanatory-only.".to_owned(),
        terminal_difference_vs_dca_percent: relative_difference(
            simulation.performance.terminal_wealth_usd,
            fixed_dca.terminal_wealth_usd,
        ),
        worst_rolling_window: worst_rolling_window(&rolling_out_of_sample),
        performance: simulation.performance,
        rolling_out_of_sample,
        c4_cash_diagnostics: Some(simulation.diagnostics),
    })
}

fn simulate_bounded_budget_with_deadline(
    samples: &[DecisionMonth],
    configuration: PlanExecutionConfiguration,
) -> Result<C4Simulation, EvaluationError> {
    let allocation = configuration.bucket_allocation();
    let core_budget = PERIOD_BUDGET as f64
        * allocation
            .core_ratio()
            .value()
            .to_f64()
            .ok_or(EvaluationError::InsufficientHistory)?;
    let opportunity_budget = PERIOD_BUDGET as f64
        * allocation
            .opportunity_ratio()
            .value()
            .to_f64()
            .ok_or(EvaluationError::InsufficientHistory)?;
    let mut state = PortfolioState::default();
    let mut deferred_lots = Vec::new();
    let mut forced_catch_up_usd = 0.0_f64;
    let mut released_for_additional_investment_usd = 0.0_f64;
    let mut matured_lot_count = 0;
    let mut maximum_deferred_opportunity_cash_usd = 0.0_f64;

    for (period, row) in samples.iter().enumerate() {
        state.deposit(row.decision_date, PERIOD_BUDGET as f64);
        let multiplier = c4_opportunity_multiplier(samples, period, &row.trend);
        let base_opportunity = opportunity_budget * multiplier.min(1.0);
        let deferred_current = (opportunity_budget - base_opportunity).max(0.0);
        let forced_catch_up = take_expired_lots(&mut deferred_lots, period);
        forced_catch_up_usd += forced_catch_up.amount_usd;
        matured_lot_count += forced_catch_up.count;

        let additional_requested = opportunity_budget * (multiplier - 1.0).max(0.0);
        let additional_released = take_oldest_lots(&mut deferred_lots, additional_requested);
        released_for_additional_investment_usd += additional_released;
        if deferred_current > 0.0 {
            deferred_lots.push(DeferredOpportunityLot {
                amount_usd: deferred_current,
                deadline_period: period + C4_OPPORTUNITY_CARRY_PERIODS,
            });
        }
        let deferred_total = deferred_lots.iter().map(|lot| lot.amount_usd).sum::<f64>();
        maximum_deferred_opportunity_cash_usd =
            maximum_deferred_opportunity_cash_usd.max(deferred_total);

        let requested =
            core_budget + base_opportunity + forced_catch_up.amount_usd + additional_released;
        let spend = requested.min(MAX_SINGLE_EXECUTION as f64);
        state.buy(row.execution_date, spend, row.execution_close);
        state.mark_to_market(row.execution_date, row.execution_close);
    }

    let terminal_opportunity_cash_usd = deferred_lots.iter().map(|lot| lot.amount_usd).sum();
    Ok(C4Simulation {
        performance: state.metrics(terminal_opportunity_cash_usd),
        diagnostics: C4CashDiagnostics {
            fundamental_warmup_months: C4_FUNDAMENTAL_LOOKBACK_MONTHS,
            maximum_carry_periods: C4_OPPORTUNITY_CARRY_PERIODS,
            forced_catch_up_usd,
            released_for_additional_investment_usd,
            matured_lot_count,
            maximum_deferred_opportunity_cash_usd,
        },
    })
}

struct ExpiredLots {
    amount_usd: f64,
    count: usize,
}

fn take_expired_lots(lots: &mut Vec<DeferredOpportunityLot>, period: usize) -> ExpiredLots {
    let mut amount_usd = 0.0;
    let mut count = 0;
    lots.retain(|lot| {
        if lot.deadline_period <= period {
            amount_usd += lot.amount_usd;
            count += 1;
            false
        } else {
            true
        }
    });
    ExpiredLots { amount_usd, count }
}

fn take_oldest_lots(lots: &mut Vec<DeferredOpportunityLot>, requested: f64) -> f64 {
    let mut remaining = requested;
    let mut released = 0.0;
    for lot in lots.iter_mut() {
        if remaining <= 0.0 {
            break;
        }
        let amount = lot.amount_usd.min(remaining);
        lot.amount_usd -= amount;
        remaining -= amount;
        released += amount;
    }
    lots.retain(|lot| lot.amount_usd > f64::EPSILON);
    released
}

fn c4_opportunity_multiplier(samples: &[DecisionMonth], index: usize, trend: &TrendSignal) -> f64 {
    let fundamental_multiplier = c4_fundamental_multiplier(samples, index);
    fundamental_multiplier.min(c4_trend_addition_cap(trend))
}

fn c4_fundamental_multiplier(samples: &[DecisionMonth], index: usize) -> f64 {
    if index < C4_FUNDAMENTAL_LOOKBACK_MONTHS {
        return 1.0;
    }
    let current = samples[index].fallback.fundamental_score.value();
    let history = &samples[index - C4_FUNDAMENTAL_LOOKBACK_MONTHS..index];
    let rank = history
        .iter()
        .filter(|row| row.fallback.fundamental_score.value() <= current)
        .count() as f64
        / history.len() as f64;
    C4_FUNDAMENTAL_FLOOR + (C4_ADDITION_CEILING - C4_FUNDAMENTAL_FLOOR) * rank
}

fn c4_trend_addition_cap(trend: &TrendSignal) -> f64 {
    let raw_tail_risk = trend
        .ma_distance_percentile
        .value()
        .max(trend.rsi_percentile.value())
        .max(trend.vix_percentile.value());
    let normalised_tail_risk =
        ((raw_tail_risk - C3_TAIL_RISK_START) / (1.0 - C3_TAIL_RISK_START)).clamp(0.0, 1.0);
    C4_ADDITION_CEILING - (C4_ADDITION_CEILING - 1.0) * normalised_tail_risk
}

fn trend_continuous_cap_multiplier(
    fundamental_score: Percentile,
    trend: &TrendSignal,
) -> Multiplier {
    let fundamental_multiplier = multiplier_from_score(fundamental_score);
    let raw_tail_risk = trend
        .ma_distance_percentile
        .value()
        .max(trend.rsi_percentile.value())
        .max(trend.vix_percentile.value());
    let normalised_tail_risk =
        ((raw_tail_risk - TREND_CAP_RISK_START) / (1.0 - TREND_CAP_RISK_START)).clamp(0.0, 1.0);
    let trend_cap = 1.0 - (1.0 - TREND_CAP_FLOOR) * normalised_tail_risk;
    Multiplier::new_clamped(fundamental_multiplier.value() * trend_cap)
}

fn fundamental_trend_addition_cap_multiplier(
    fundamental_score: Percentile,
    trend: &TrendSignal,
) -> Multiplier {
    let fundamental_multiplier = multiplier_from_score(fundamental_score)
        .value()
        .clamp(C3_FUNDAMENTAL_FLOOR, C3_ADDITION_CEILING);
    let raw_tail_risk = trend
        .ma_distance_percentile
        .value()
        .max(trend.rsi_percentile.value())
        .max(trend.vix_percentile.value());
    let normalised_tail_risk =
        ((raw_tail_risk - C3_TAIL_RISK_START) / (1.0 - C3_TAIL_RISK_START)).clamp(0.0, 1.0);
    let trend_addition_cap =
        C3_ADDITION_CEILING - (C3_ADDITION_CEILING - 1.0) * normalised_tail_risk;
    Multiplier::new_clamped(fundamental_multiplier.min(trend_addition_cap))
}

fn allocation_sensitivity(
    samples: &[DecisionMonth],
) -> Result<Vec<AllocationSensitivity>, EvaluationError> {
    [100_i64, 80, 70, 50]
        .into_iter()
        .map(|core_percentage| {
            let core_ratio = Decimal::new(core_percentage, 2);
            let configuration = execution_configuration(core_ratio)?;
            Ok(AllocationSensitivity {
                core_ratio: core_ratio.to_f64().unwrap_or_default(),
                opportunity_ratio: (Decimal::ONE - core_ratio).to_f64().unwrap_or_default(),
                fixed_dca: simulate(samples, configuration, ExecutionMode::FixedDca)?,
                current_core_opportunity: simulate(
                    samples,
                    configuration,
                    ExecutionMode::CoreOpportunityIntent,
                )?,
                c1_bounded_continuous: simulate(
                    samples,
                    configuration,
                    ExecutionMode::CandidateBoundedContinuous,
                )?,
                c2_trend_continuous_cap: simulate(
                    samples,
                    configuration,
                    ExecutionMode::CandidateTrendContinuousCap,
                )?,
                c3_fundamental_with_trend_addition_cap: simulate(
                    samples,
                    configuration,
                    ExecutionMode::CandidateFundamentalTrendAdditionCap,
                )?,
                c4_bounded_budget_with_deadline: simulate_bounded_budget_with_deadline(
                    samples,
                    configuration,
                )?
                .performance,
            })
        })
        .collect()
}

fn worst_rolling_window(windows: &[CandidateRollingWindow]) -> Option<CandidateRollingWindow> {
    windows
        .iter()
        .min_by(|left, right| {
            left.terminal_difference_vs_dca_percent
                .total_cmp(&right.terminal_difference_vs_dca_percent)
        })
        .cloned()
}

fn multiplier_from_score(score: Percentile) -> Multiplier {
    let raw = if score.value() <= 0.5 {
        score.value() * 2.0
    } else {
        1.0 + (score.value() - 0.5)
    };
    Multiplier::new_clamped(raw)
}

struct PortfolioState {
    cash: f64,
    units: f64,
    external_cash: f64,
    invested: f64,
    flows: Vec<(NaiveDate, f64)>,
    last_value: f64,
    pending_external_flow: f64,
    time_weighted_nav: f64,
    nav_values: Vec<f64>,
    period_returns: Vec<f64>,
    last_mark_date: Option<NaiveDate>,
}

impl Default for PortfolioState {
    fn default() -> Self {
        Self {
            cash: 0.0,
            units: 0.0,
            external_cash: 0.0,
            invested: 0.0,
            flows: Vec::new(),
            last_value: 0.0,
            pending_external_flow: 0.0,
            time_weighted_nav: 1.0,
            nav_values: Vec::new(),
            period_returns: Vec::new(),
            last_mark_date: None,
        }
    }
}

impl PortfolioState {
    fn deposit(&mut self, date: NaiveDate, amount: f64) {
        self.cash += amount;
        self.external_cash += amount;
        self.pending_external_flow += amount;
        self.flows.push((date, -amount));
    }

    fn buy(&mut self, _date: NaiveDate, amount: f64, close: f64) {
        let amount = amount.min(self.cash).max(0.0);
        self.cash -= amount;
        self.invested += amount;
        self.units += amount / (close * (1.0 + BUY_COST_BPS / 10_000.0));
    }

    fn mark_to_market(&mut self, date: NaiveDate, close: f64) {
        let value = self.cash + self.units * close;
        let denominator = self.last_value + self.pending_external_flow;
        if denominator > 0.0 {
            let period_return = value / denominator - 1.0;
            self.time_weighted_nav *= 1.0 + period_return;
            self.nav_values.push(self.time_weighted_nav);
            if self.last_value > 0.0 {
                self.period_returns.push(period_return);
            }
        }
        self.last_value = value;
        self.pending_external_flow = 0.0;
        self.last_mark_date = Some(date);
    }

    fn metrics(mut self, terminal_opportunity_cash_usd: f64) -> PerformanceMetrics {
        let terminal = self.last_value;
        if let Some(last_mark_date) = self.last_mark_date {
            self.flows.push((last_mark_date, terminal));
        }
        PerformanceMetrics {
            xirr_percent: xirr(&self.flows).map(|value| value * 100.0),
            terminal_wealth_usd: terminal,
            maximum_drawdown_percent: maximum_drawdown(&self.nav_values) * 100.0,
            annualized_volatility_percent: annualized_volatility(&self.period_returns)
                .map(|value| value * 100.0),
            sortino_ratio: sortino_ratio(&self.period_returns),
            maximum_drawdown_recovery_months: maximum_drawdown_recovery_months(&self.nav_values),
            total_external_cash_usd: self.external_cash,
            total_invested_usd: self.invested,
            cash_utilisation_percent: if self.external_cash == 0.0 {
                0.0
            } else {
                self.invested / self.external_cash * 100.0
            },
            terminal_cash_usd: self.cash,
            terminal_opportunity_cash_usd,
        }
    }
}

fn qwen_sensitivity(samples: &[DecisionMonth]) -> QwenSensitivityReport {
    let frozen: FrozenQwenSensitivity =
        serde_json::from_str(include_str!("../data/generated/qwen-sensitivity-v1.json"))
            .expect("the committed Qwen sensitivity fixture must be valid JSON");
    let normal: Vec<_> = samples
        .iter()
        .enumerate()
        .map(|(index, row)| {
            evaluate_decision(
                &DecisionInput {
                    fundamental: row.fundamental.clone(),
                    trend: row.trend.clone(),
                    sentiment: DecisionSentiment::Available(
                        Sentiment::new(frozen.scores[index % frozen.scores.len()])
                            .expect("frozen sensitivity scores are bounded"),
                    ),
                },
                &DecisionConfig::default(),
            )
        })
        .collect();
    let fallback_scores: Vec<_> = samples
        .iter()
        .map(|row| row.fallback.final_score.value())
        .collect();
    let normal_scores: Vec<_> = normal.iter().map(|row| row.final_score.value()).collect();
    QwenSensitivityReport {
        scope: frozen.purpose,
        frozen_score_version: frozen.version,
        sample_count: samples.len(),
        fallback_action_distribution: action_distribution(samples.iter().map(|row| &row.fallback)),
        normal_70_20_10_action_distribution: action_distribution(normal.iter()),
        fallback_score_distribution: distribution(fallback_scores.clone()),
        normal_70_20_10_score_distribution: distribution(normal_scores.clone()),
        mean_normal_minus_fallback_score: mean(normal_scores) - mean(fallback_scores),
    }
}

fn distribution(values: impl IntoIterator<Item = f64>) -> Distribution {
    let mut values: Vec<_> = values.into_iter().collect();
    values.sort_by(f64::total_cmp);
    Distribution {
        mean: mean(values.clone()),
        median: percentile(&values, 0.5),
        p10: percentile(&values, 0.1),
        p25: percentile(&values, 0.25),
        p75: percentile(&values, 0.75),
        p90: percentile(&values, 0.9),
    }
}

fn percentile(values: &[f64], probability: f64) -> f64 {
    let index = ((values.len().saturating_sub(1)) as f64 * probability).round() as usize;
    values.get(index).copied().unwrap_or_default()
}

fn mean(values: impl IntoIterator<Item = f64>) -> f64 {
    let values: Vec<_> = values.into_iter().collect();
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn action_distribution<'a>(
    signals: impl IntoIterator<Item = &'a DecisionSignal>,
) -> BTreeMap<String, u32> {
    signals
        .into_iter()
        .fold(BTreeMap::new(), |mut output, signal| {
            *output.entry(format!("{:?}", signal.action)).or_default() += 1;
            output
        })
}

fn layer_calibration(samples: &[DecisionMonth]) -> LayerCalibration {
    let values = samples.iter().map(|row| {
        let signal = &row.fallback;
        (
            row.fundamental.score.value(),
            signal.fundamental_score.value(),
            signal.weights.fundamental_weight.value() * signal.fundamental_score.value(),
            row.trend.score.value(),
            signal.trend_score.value(),
            signal.weights.trend_weight.value() * signal.trend_score.value(),
            signal.sentiment_score.map_or(0.5, |value| value.value()),
            signal.weights.sentiment_weight.value()
                * signal.sentiment_score.map_or(0.5, |value| value.value()),
            signal.final_score.value(),
        )
    });
    let mut fundamental_raw = Vec::new();
    let mut fundamental_directional = Vec::new();
    let mut fundamentals = Vec::new();
    let mut trend_raw = Vec::new();
    let mut trend_timing = Vec::new();
    let mut trends = Vec::new();
    let mut sentiment_input = Vec::new();
    let mut sentiments = Vec::new();
    let mut finals = Vec::new();
    for (
        raw_fundamental,
        directional_fundamental,
        fundamental,
        raw_trend,
        timing_trend,
        trend,
        sentiment_value,
        sentiment,
        final_score,
    ) in values
    {
        fundamental_raw.push(raw_fundamental);
        fundamental_directional.push(directional_fundamental);
        fundamentals.push(fundamental);
        trend_raw.push(raw_trend);
        trend_timing.push(timing_trend);
        trends.push(trend);
        sentiment_input.push(sentiment_value);
        sentiments.push(sentiment);
        finals.push(final_score);
    }
    LayerCalibration {
        fundamental_raw_mean: mean(fundamental_raw),
        fundamental_directional_mean: mean(fundamental_directional),
        fundamental_weighted_contribution_mean: mean(fundamentals),
        trend_raw_mean: mean(trend_raw),
        trend_timing_mean: mean(trend_timing),
        trend_weighted_contribution_mean: mean(trends),
        sentiment_input_mean_when_unavailable: mean(sentiment_input),
        sentiment_weighted_contribution_mean: mean(sentiments),
        final_score_mean: mean(finals),
    }
}

fn relative_difference(actual: f64, benchmark: f64) -> f64 {
    if benchmark == 0.0 {
        0.0
    } else {
        (actual / benchmark - 1.0) * 100.0
    }
}

fn maximum_drawdown(values: &[f64]) -> f64 {
    let mut peak = 0.0_f64;
    values.iter().fold(0.0_f64, |maximum, value| {
        peak = peak.max(*value);
        if peak == 0.0 {
            maximum
        } else {
            maximum.max((peak - value) / peak)
        }
    })
}

fn annualized_volatility(returns: &[f64]) -> Option<f64> {
    if returns.len() < 2 {
        return None;
    }
    let average = mean(returns.iter().copied());
    let variance = returns
        .iter()
        .map(|value| (value - average).powi(2))
        .sum::<f64>()
        / (returns.len() - 1) as f64;
    Some(variance.sqrt() * 12.0_f64.sqrt())
}

fn sortino_ratio(returns: &[f64]) -> Option<f64> {
    if returns.len() < 2 {
        return None;
    }
    let downside_deviation = (returns
        .iter()
        .map(|value| value.min(0.0).powi(2))
        .sum::<f64>()
        / returns.len() as f64)
        .sqrt();
    (downside_deviation > 0.0)
        .then(|| mean(returns.iter().copied()) / downside_deviation * 12.0_f64.sqrt())
}

fn maximum_drawdown_recovery_months(values: &[f64]) -> Option<u32> {
    let mut peak_value = 0.0_f64;
    let mut peak_index = 0_usize;
    let mut maximum_drawdown = 0.0_f64;
    let mut drawdown_peak = 0_usize;
    let mut drawdown_trough = 0_usize;
    for (index, value) in values.iter().enumerate() {
        if *value > peak_value {
            peak_value = *value;
            peak_index = index;
        }
        if peak_value > 0.0 {
            let drawdown = (peak_value - value) / peak_value;
            if drawdown > maximum_drawdown {
                maximum_drawdown = drawdown;
                drawdown_peak = peak_index;
                drawdown_trough = index;
            }
        }
    }
    (maximum_drawdown > 0.0).then(|| {
        values
            .iter()
            .enumerate()
            .skip(drawdown_trough + 1)
            .find(|(_, value)| **value >= values[drawdown_peak])
            .map(|(recovery_index, _)| (recovery_index - drawdown_trough) as u32)
    })?
}

fn xirr(flows: &[(NaiveDate, f64)]) -> Option<f64> {
    let first = flows.first()?.0;
    let npv = |rate: f64| -> f64 {
        flows
            .iter()
            .map(|(date, cash)| {
                let years = (*date - first).num_days() as f64 / 365.25;
                cash / (1.0 + rate).powf(years)
            })
            .sum()
    };
    let mut lower = -0.9999;
    let mut upper = 10.0;
    let lower_value = npv(lower);
    let upper_value = npv(upper);
    if lower_value.signum() == upper_value.signum() {
        return None;
    }
    for _ in 0..100 {
        let middle = (lower + upper) / 2.0;
        if npv(middle).signum() == lower_value.signum() {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    Some((lower + upper) / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::Action;

    /// Verify the committed fixture produces a deterministic non-empty offline report.
    #[test]
    fn fixture_evaluation_is_reproducible() {
        let first = report_json(&evaluate_fixture().unwrap()).unwrap();
        let second = report_json(&evaluate_fixture().unwrap()).unwrap();
        assert_eq!(first, second);
        assert!(first.contains("calibration-v2"));
        assert!(first.contains("sp500_index_proxy"));
        assert!(first.contains("nasdaq_composite_proxy"));
    }

    /// Verify the versioned technical inputs are embedded, hashed, ordered, and offline-only.
    #[test]
    fn technical_fixture_validates_daily_proxy_and_vix_snapshots() {
        let summary = validate_technical_fixture().unwrap();

        assert_eq!(summary.dataset_version, "technical-v1");
        assert_eq!(summary.captured_on, "2026-08-21");
        assert_eq!(summary.coverage_start, "2016-08-22");
        assert_eq!(summary.coverage_end, "2026-08-19");
        assert_eq!(summary.series.len(), 3);
        assert_eq!(summary.series[0].proxy_for.as_deref(), Some("SPY"));
        assert_eq!(summary.series[1].proxy_for.as_deref(), Some("QQQ"));
        assert_eq!(summary.series[2].source_symbol, "VIX");
        assert_eq!(summary.series[0].source_gap_rows, 96);
        assert_eq!(summary.series[1].source_gap_rows, 487);
        assert_eq!(summary.series[2].source_gap_rows, 0);
        assert!(summary
            .series
            .iter()
            .all(|series| series.observations > 2_000));
    }

    /// Verify changing a committed raw hash blocks use of the technical fixture.
    #[test]
    fn technical_fixture_rejects_manifest_hash_tampering() {
        let tampered = TECHNICAL_FIXTURE_MANIFEST.replace(
            "94635e135f4aab22a7e77fd1c297ddf5a04cd28e592e4988e67bcf440b291416",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );

        assert!(matches!(
            validate_technical_fixture_contents(
                &tampered,
                &[
                    ("fred_sp500_daily.csv", FRED_SP500_DAILY),
                    ("fred_nasdaqcom_daily.csv", FRED_NASDAQCOM_DAILY),
                    ("cboe_vix_daily.csv", CBOE_VIX_DAILY),
                ],
            ),
            Err(EvaluationError::TechnicalFixtureIntegrity)
        ));
    }

    /// Verify a malformed, non-positive, or non-monotonic source row is rejected instead of filled.
    #[test]
    fn technical_fixture_rejects_invalid_daily_observations() {
        let series = TechnicalFixtureSeries {
            id: "test_price".to_owned(),
            source_symbol: "TEST".to_owned(),
            proxy_for: None,
            file: "test.csv".to_owned(),
            source_url: "https://example.invalid/test.csv".to_owned(),
            source_terms: "test-only".to_owned(),
            date_format: "%Y-%m-%d".to_owned(),
            date_column: "date".to_owned(),
            close_column: "close".to_owned(),
            sha256: "0".repeat(64),
        };
        assert!(matches!(
            parse_technical_series(&series, "date,close\n2026-01-02,100\n2026-01-01,99\n",),
            Err(EvaluationError::TechnicalFixtureIntegrity)
        ));
        assert!(matches!(
            parse_technical_series(&series, "date,close\n2026-01-02,0\n"),
            Err(EvaluationError::TechnicalFixtureIntegrity)
        ));
    }

    /// Verify every decision uses prior history and a strictly later execution price.
    #[test]
    fn decisions_require_sixty_prior_observations_and_next_day_execution() {
        let dataset: FixtureDataset =
            serde_json::from_str(include_str!("../data/generated/calibration-v2.json")).unwrap();
        for asset in dataset.assets {
            let samples = evaluate_asset(&asset).unwrap();
            assert!(samples.len() <= asset.observations.len() - 60);
            assert!(samples
                .iter()
                .all(|sample| sample.execution_date > sample.decision_date));
        }
    }

    /// Verify the research trend cap is continuous and bounded without a delay action.
    #[test]
    fn continuous_trend_cap_bounds_only_the_opportunity_multiplier() {
        let neutral = quant_engine::TrendSignal {
            score: Percentile::new(0.5).unwrap(),
            ma_distance_percentile: Percentile::new(0.5).unwrap(),
            rsi_percentile: Percentile::new(0.5).unwrap(),
            vix_percentile: Percentile::new(0.5).unwrap(),
            regime: quant_engine::TrendRegime::Neutral,
        };
        let severe = quant_engine::TrendSignal {
            score: Percentile::new(0.0).unwrap(),
            ma_distance_percentile: Percentile::new(1.0).unwrap(),
            rsi_percentile: Percentile::new(1.0).unwrap(),
            vix_percentile: Percentile::new(1.0).unwrap(),
            regime: quant_engine::TrendRegime::FallingKnife,
        };
        let fundamental = Percentile::new(0.5).unwrap();

        assert_eq!(
            trend_continuous_cap_multiplier(fundamental, &neutral).value(),
            1.0
        );
        assert_eq!(
            trend_continuous_cap_multiplier(fundamental, &severe).value(),
            TREND_CAP_FLOOR
        );
    }

    /// Verify C3 only removes an overweight and never uses trend to cut normal input.
    #[test]
    fn c3_trend_caps_only_fundamental_additions() {
        let neutral = quant_engine::TrendSignal {
            score: Percentile::new(0.5).unwrap(),
            ma_distance_percentile: Percentile::new(0.5).unwrap(),
            rsi_percentile: Percentile::new(0.5).unwrap(),
            vix_percentile: Percentile::new(0.5).unwrap(),
            regime: quant_engine::TrendRegime::Neutral,
        };
        let severe = quant_engine::TrendSignal {
            score: Percentile::new(0.0).unwrap(),
            ma_distance_percentile: Percentile::new(1.0).unwrap(),
            rsi_percentile: Percentile::new(1.0).unwrap(),
            vix_percentile: Percentile::new(1.0).unwrap(),
            regime: quant_engine::TrendRegime::FallingKnife,
        };

        assert_eq!(
            fundamental_trend_addition_cap_multiplier(Percentile::new(0.9).unwrap(), &neutral)
                .value(),
            C3_ADDITION_CEILING
        );
        assert_eq!(
            fundamental_trend_addition_cap_multiplier(Percentile::new(0.9).unwrap(), &severe)
                .value(),
            1.0
        );
        assert_eq!(
            fundamental_trend_addition_cap_multiplier(Percentile::new(0.1).unwrap(), &severe)
                .value(),
            C3_FUNDAMENTAL_FLOOR
        );
    }

    /// Verify C4 uses only completed factor history and trend never cuts its base input.
    #[test]
    fn c4_preserves_base_opportunity_and_uses_a_bounded_prior_rank() {
        let dataset: FixtureDataset =
            serde_json::from_str(include_str!("../data/generated/calibration-v2.json")).unwrap();
        let samples = evaluate_asset(&dataset.assets[0]).unwrap();
        let severe = quant_engine::TrendSignal {
            score: Percentile::new(0.0).unwrap(),
            ma_distance_percentile: Percentile::new(1.0).unwrap(),
            rsi_percentile: Percentile::new(1.0).unwrap(),
            vix_percentile: Percentile::new(1.0).unwrap(),
            regime: quant_engine::TrendRegime::FallingKnife,
        };

        assert_eq!(c4_fundamental_multiplier(&samples, 0), 1.0);
        for index in C4_FUNDAMENTAL_LOOKBACK_MONTHS..samples.len() {
            let fundamental = c4_fundamental_multiplier(&samples, index);
            assert!((C4_FUNDAMENTAL_FLOOR..=C4_ADDITION_CEILING).contains(&fundamental));
            let multiplier = c4_opportunity_multiplier(&samples, index, &severe);
            assert!(multiplier >= C4_FUNDAMENTAL_FLOOR);
            assert!(multiplier <= 1.0);
        }
    }

    /// Verify C4 expired opportunity cash is released on its deadline, not discarded.
    #[test]
    fn c4_expired_cash_is_forced_to_catch_up() {
        let mut lots = vec![
            DeferredOpportunityLot {
                amount_usd: 30.0,
                deadline_period: 2,
            },
            DeferredOpportunityLot {
                amount_usd: 40.0,
                deadline_period: 3,
            },
        ];
        let expired = take_expired_lots(&mut lots, 2);

        assert_eq!(expired.count, 1);
        assert_eq!(expired.amount_usd, 30.0);
        assert_eq!(lots.len(), 1);
        assert_eq!(lots[0].amount_usd, 40.0);
    }

    /// Verify a trend cap never removes the fixed core contribution.
    #[test]
    fn trend_candidate_preserves_core_for_every_scored_observation() {
        let configuration = execution_configuration(Decimal::new(CORE_RATIO, 1)).unwrap();
        let dataset: FixtureDataset =
            serde_json::from_str(include_str!("../data/generated/calibration-v2.json")).unwrap();
        for asset in dataset.assets {
            for sample in evaluate_asset(&asset).unwrap() {
                let multiplier = trend_continuous_cap_multiplier(
                    sample.fallback.fundamental_score,
                    &sample.trend,
                );
                let split = TwoBucketContributionSplit::from_decision_with_carry(
                    Decimal::new(PERIOD_BUDGET, 0),
                    Decimal::new(MAX_SINGLE_EXECUTION, 0),
                    configuration,
                    multiplier.to_action(),
                    multiplier,
                    Decimal::ZERO,
                )
                .unwrap();
                assert_eq!(split.core_contribution(), Decimal::new(700, 0));
                assert_ne!(multiplier.to_action(), Action::TacticalDelay);

                let c3_multiplier = fundamental_trend_addition_cap_multiplier(
                    sample.fallback.fundamental_score,
                    &sample.trend,
                );
                let c3_split = TwoBucketContributionSplit::from_decision_with_carry(
                    Decimal::new(PERIOD_BUDGET, 0),
                    Decimal::new(MAX_SINGLE_EXECUTION, 0),
                    configuration,
                    c3_multiplier.to_action(),
                    c3_multiplier,
                    Decimal::ZERO,
                )
                .unwrap();
                assert_eq!(c3_split.core_contribution(), Decimal::new(700, 0));
                assert_ne!(c3_multiplier.to_action(), Action::TacticalDelay);
            }
        }
    }

    /// Verify the historical candidate calls the shared DSL interpreter and cannot veto core.
    #[test]
    fn dsl_runtime_candidate_preserves_core_on_every_fixture_observation() {
        let configuration = execution_configuration(Decimal::new(CORE_RATIO, 1)).unwrap();
        let strategy = dsl_rsi_opportunity_strategy().unwrap();
        let dataset: FixtureDataset =
            serde_json::from_str(include_str!("../data/generated/calibration-v2.json")).unwrap();

        for asset in dataset.assets {
            for sample in evaluate_asset(&asset).unwrap() {
                let (action, multiplier) = dsl_rsi_opportunity_recommendation(
                    &strategy,
                    &sample,
                    Decimal::new(PERIOD_BUDGET, 0),
                )
                .unwrap();
                let split = TwoBucketContributionSplit::from_decision_with_carry(
                    Decimal::new(PERIOD_BUDGET, 0),
                    Decimal::new(MAX_SINGLE_EXECUTION, 0),
                    configuration,
                    action,
                    multiplier,
                    Decimal::ZERO,
                )
                .unwrap();

                assert_eq!(split.core_contribution(), Decimal::new(700, 0));
                assert_ne!(action, Action::TacticalDelay);
            }
        }
    }

    /// Verify historical research can call the same price-to-evidence builder as the online runtime.
    #[test]
    fn shared_technical_evidence_builder_is_available_to_offline_research() {
        let strategy = dsl_rsi_opportunity_strategy().unwrap();
        let closes = (1..=15).map(Decimal::from).collect::<Vec<_>>();
        let evidence =
            DslEvidence::from_market_snapshot(&strategy, &closes, Decimal::new(20, 0)).unwrap();
        let rsi = IndicatorSpec::RelativeStrengthIndex(LookbackWindow::new(14).unwrap());

        assert!(evidence.value(rsi).unwrap() > Decimal::ZERO);
    }

    /// Verify the fixed calibration fixture admits a safe RSI/VIX opportunity-only policy.
    #[test]
    fn admission_evaluation_compares_safe_dsl_policy_with_fixed_dca() {
        let report = evaluate_strategy_admission(&dsl_rsi_opportunity_strategy().unwrap()).unwrap();

        assert!(report.eligible);
        assert!(report.core_bucket_safe);
        assert!(report.budget_safe);
        assert_eq!(report.assets.len(), 2);
        assert!(report
            .assets
            .iter()
            .all(|asset| asset.observations > 0 && asset.fixed_dca.terminal_wealth_usd > 0.0));
    }

    /// Verify a policy needing unsupported historical evidence stays saved but cannot activate.
    #[test]
    fn admission_rejects_indicators_missing_from_the_versioned_fixture() {
        let close = IndicatorSpec::ClosePrice;
        let strategy = StrategySpec::new(
            PolicyRef::new(
                PolicyId::new("dsl_close_not_yet_calibrated").unwrap(),
                PolicyVersion::new(1).unwrap(),
            ),
            "Close-price strategy awaiting fixture support",
            vec![StrategyRule::new(
                Condition::compare(
                    ValueExpression::indicator(close),
                    ComparisonOperator::GreaterThan,
                    Decimal::ONE,
                ),
                PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(1.0)),
            )],
        )
        .unwrap();

        let report = evaluate_strategy_admission(&strategy).unwrap();
        assert!(!report.eligible);
        assert!(report.assets.is_empty());
        assert!(report.reason.unwrap().contains("RSI(14) and VIX"));
    }

    /// Verify the core/opportunity simulation treats retained money as terminal cash.
    #[test]
    fn strategy_never_spends_more_than_external_cash() {
        let report = report_json(&evaluate_fixture().unwrap()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        for asset in value["assets"].as_array().unwrap() {
            let mut metrics = vec![&asset["core_opportunity_intent"], &asset["fixed_dca"]];
            metrics.extend(
                asset["experimental_candidates"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|candidate| &candidate["performance"]),
            );
            for metric in metrics {
                assert!(
                    metric["total_invested_usd"].as_f64().unwrap()
                        <= metric["total_external_cash_usd"].as_f64().unwrap()
                );
            }
        }
    }
}
