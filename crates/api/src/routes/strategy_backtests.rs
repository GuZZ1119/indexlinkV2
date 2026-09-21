//! Product-facing, provider-neutral strategy backtests for one selected instrument.

use std::{collections::BTreeSet, str::FromStr};

use crate::{official_strategies, ApiError, ApiState};
use axum::{
    extract::{rejection::JsonRejection, State},
    routing::post,
    Json, Router,
};
use chrono::{Months, NaiveDate, Utc};
use market_data::{HistoricalPriceRequest, Instrument, MarketDataError};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use strategy_evaluation::{
    run_dynamic_backtest, BacktestPrice, BacktestStrategy, DynamicBacktestError,
    DynamicBacktestRequest, DynamicBacktestResult,
};
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};

const MAX_STRATEGIES: usize = 3;
const WARMUP_CALENDAR_DAYS: i64 = 400;
const ALL_HISTORY_START_YEAR: i32 = 1900;

/// Supported user-facing comparison windows.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum BacktestRange {
    /// One month.
    #[serde(rename = "1m")]
    OneMonth,
    /// Three months.
    #[serde(rename = "3m")]
    ThreeMonths,
    /// Six months.
    #[serde(rename = "6m")]
    SixMonths,
    /// One year.
    #[serde(rename = "1y")]
    OneYear,
    /// Three years.
    #[serde(rename = "3y")]
    ThreeYears,
    /// Five years.
    #[serde(rename = "5y")]
    FiveYears,
    /// All available history within the provider safety bound.
    All,
}

impl BacktestRange {
    fn display_start(self, end: NaiveDate) -> Result<NaiveDate, ApiError> {
        let months = match self {
            Self::OneMonth => Some(1),
            Self::ThreeMonths => Some(3),
            Self::SixMonths => Some(6),
            Self::OneYear => Some(12),
            Self::ThreeYears => Some(36),
            Self::FiveYears => Some(60),
            Self::All => None,
        };
        match months {
            Some(months) => end
                .checked_sub_months(Months::new(months))
                .ok_or(ApiError::BadRequest),
            None => NaiveDate::from_ymd_opt(ALL_HISTORY_START_YEAR, 1, 1)
                .ok_or(ApiError::ServiceUnavailable),
        }
    }
}

#[derive(Debug, Deserialize)]
struct StrategyBacktestRequest {
    symbol: String,
    #[serde(default)]
    strategy_ids: Vec<String>,
    #[serde(default)]
    strategy_refs: Vec<StrategyBacktestReference>,
    range: BacktestRange,
    monthly_day: u8,
    contribution: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
struct StrategyBacktestReference {
    policy_id: String,
    policy_version: u32,
}

#[derive(Debug, Serialize)]
struct StrategyBacktestResponse {
    requested_range: BacktestRange,
    data: BacktestDataProvenance,
    result: DynamicBacktestResult,
}

#[derive(Debug, Serialize)]
struct BacktestDataProvenance {
    provider: String,
    market: String,
    instrument_type: String,
    currency: String,
    timezone: String,
    adjustment: String,
    fetched_at: String,
    requested_start: String,
    requested_end: String,
    dataset_version: String,
    checksum: String,
}

pub(crate) fn router() -> Router<ApiState> {
    Router::new().route("/strategy-backtests", post(run_backtest))
}

async fn run_backtest(
    State(state): State<ApiState>,
    input: Result<Json<StrategyBacktestRequest>, JsonRejection>,
) -> Result<Json<StrategyBacktestResponse>, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    validate_selection(&input)?;
    let instrument = Instrument::parse(&input.symbol).map_err(map_market_request_error)?;
    let contribution = Decimal::from_str(&input.contribution).map_err(|_| ApiError::BadRequest)?;
    if contribution <= Decimal::ZERO || !(1..=28).contains(&input.monthly_day) {
        return Err(ApiError::BadRequest);
    }
    let strategies = resolve_strategies(&state, &input).await?;

    let end = Utc::now().date_naive();
    let display_start = input.range.display_start(end)?;
    let import_start = display_start
        .checked_sub_signed(chrono::Duration::days(WARMUP_CALENDAR_DAYS))
        .unwrap_or(display_start);
    let history_provider = state.historical_price_provider()?;
    let adjustment = history_provider
        .preferred_adjustment(instrument.market())
        .map_err(map_market_request_error)?;
    let price_request = HistoricalPriceRequest::new(instrument, import_start, end, adjustment)
        .map_err(map_market_request_error)?;
    let dataset = history_provider
        .fetch_history(&price_request)
        .await
        .map_err(map_market_provider_error)?;

    let prices = dataset
        .bars()
        .iter()
        .map(|bar| {
            Decimal::from_f64(bar.close())
                .ok_or(ApiError::ServiceUnavailable)
                .and_then(|close| {
                    BacktestPrice::new(bar.date(), close).map_err(|_| ApiError::ServiceUnavailable)
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let effective_display_start = if matches!(input.range, BacktestRange::All) {
        dataset
            .bars()
            .first()
            .map(|bar| bar.date())
            .ok_or(ApiError::BadRequest)?
    } else {
        display_start
    };
    let result = run_dynamic_backtest(DynamicBacktestRequest {
        symbol: dataset.instrument().qualified_symbol(),
        start: effective_display_start,
        end,
        monthly_day: input.monthly_day,
        contribution_amount: contribution,
        prices,
        strategies,
    })
    .map_err(map_backtest_error)?;

    let instrument = dataset.instrument();
    let source = dataset.source();
    Ok(Json(StrategyBacktestResponse {
        requested_range: input.range,
        data: BacktestDataProvenance {
            provider: source.provider().to_owned(),
            market: instrument.market().as_str().to_owned(),
            instrument_type: instrument.instrument_type().as_str().to_owned(),
            currency: instrument.currency().to_owned(),
            timezone: instrument.timezone().to_owned(),
            adjustment: dataset.adjustment().as_str().to_owned(),
            fetched_at: dataset.fetched_at().to_rfc3339(),
            requested_start: dataset.requested_start().to_string(),
            requested_end: dataset.requested_end().to_string(),
            dataset_version: source.dataset_version().to_owned(),
            checksum: dataset.checksum().to_owned(),
        },
        result,
    }))
}

fn validate_selection(input: &StrategyBacktestRequest) -> Result<(), ApiError> {
    if !input.strategy_ids.is_empty() && !input.strategy_refs.is_empty() {
        return Err(ApiError::BadRequest);
    }
    let selection_len = if input.strategy_refs.is_empty() {
        input.strategy_ids.len()
    } else {
        input.strategy_refs.len()
    };
    if selection_len == 0 || selection_len > MAX_STRATEGIES {
        return Err(ApiError::BadRequest);
    }
    if input.strategy_refs.is_empty() {
        let unique = input.strategy_ids.iter().collect::<BTreeSet<_>>();
        if unique.len() != input.strategy_ids.len() {
            return Err(ApiError::BadRequest);
        }
    } else {
        let unique = input.strategy_refs.iter().collect::<BTreeSet<_>>();
        if unique.len() != input.strategy_refs.len() {
            return Err(ApiError::BadRequest);
        }
    }
    Ok(())
}

async fn resolve_strategies(
    state: &ApiState,
    input: &StrategyBacktestRequest,
) -> Result<Vec<BacktestStrategy>, ApiError> {
    if !input.strategy_refs.is_empty() {
        let mut strategies = Vec::with_capacity(input.strategy_refs.len());
        for strategy_ref in &input.strategy_refs {
            let policy = PolicyRef::new(
                PolicyId::new(strategy_ref.policy_id.clone()).map_err(|_| ApiError::BadRequest)?,
                PolicyVersion::new(strategy_ref.policy_version)
                    .map_err(|_| ApiError::BadRequest)?,
            );
            strategies.push(resolve_exact_strategy(state, &policy).await?);
        }
        return Ok(strategies);
    }

    let mut strategies = Vec::with_capacity(input.strategy_ids.len());
    for id in &input.strategy_ids {
        if id == "fixed_dca" {
            strategies.push(BacktestStrategy::FixedDca);
            continue;
        }
        let policy = official_strategies::policy_by_id(id)?.ok_or(ApiError::BadRequest)?;
        strategies.push(resolve_exact_strategy(state, &policy).await?);
    }
    Ok(strategies)
}

async fn resolve_exact_strategy(
    state: &ApiState,
    policy: &PolicyRef,
) -> Result<BacktestStrategy, ApiError> {
    if let Some(official_policy) = official_strategies::policy_by_id(policy.id().as_str())? {
        if official_policy.version() != policy.version() {
            return Err(ApiError::BadRequest);
        }
        if policy.id().as_str() == "fixed_dca" {
            return Ok(BacktestStrategy::FixedDca);
        }
    }

    let stored = match state.get_strategy_spec(policy).await {
        Ok(stored) => stored,
        Err(ApiError::NotFound) => return Err(ApiError::BadRequest),
        Err(error) => return Err(error),
    };
    Ok(BacktestStrategy::Formula(
        stored
            .document
            .into_strategy_spec()
            .map_err(|_| ApiError::ServiceUnavailable)?,
    ))
}

fn map_market_request_error(error: MarketDataError) -> ApiError {
    match error {
        MarketDataError::InvalidSymbol
        | MarketDataError::InvalidRange
        | MarketDataError::UnsupportedRequest => ApiError::BadRequest,
        _ => ApiError::ServiceUnavailable,
    }
}

fn map_market_provider_error(error: MarketDataError) -> ApiError {
    match error {
        MarketDataError::InvalidSymbol
        | MarketDataError::InvalidRange
        | MarketDataError::UnsupportedRequest
        | MarketDataError::InsufficientHistory => ApiError::BadRequest,
        _ => ApiError::ServiceUnavailable,
    }
}

fn map_backtest_error(error: DynamicBacktestError) -> ApiError {
    match error {
        DynamicBacktestError::EmptySelection
        | DynamicBacktestError::InvalidRequest
        | DynamicBacktestError::ExternalIndicatorRequired
        | DynamicBacktestError::UnsupportedFormulaAction
        | DynamicBacktestError::InsufficientHistory => ApiError::BadRequest,
        DynamicBacktestError::InvalidPrice
        | DynamicBacktestError::NonChronologicalPrices
        | DynamicBacktestError::Dsl(_)
        | DynamicBacktestError::Policy(_)
        | DynamicBacktestError::Plan(_) => ApiError::ServiceUnavailable,
    }
}
