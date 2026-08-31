//! Decision preview HTTP route.

use std::time::Duration;

use ai_client::MarketSentimentReport;
use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection},
        Path, State,
    },
    routing::post,
    Json, Router,
};
use broker::{BrokerOrderAck, BrokerOrderRequest, BrokerOrderSide, BrokerOrderStatus};
use builtin_policies::{
    BuiltinPolicyDecision, BuiltinPolicyError, BuiltinPolicyEvidence, BuiltinPolicyEvidenceKind,
    CoreOpportunityEvidence,
};
use chrono::{Datelike, NaiveDate, Utc};
use core_domain::{Action, Percentile};
use decision_engine::{DecisionInput, DecisionSentiment, DecisionWeightMode};
use decision_records::{
    CompleteDecisionRecord, CreateDecisionRecord, DecisionExecutionStatus, DecisionPolicyEvidence,
};
use indexlink_storage::OpportunityCashSettlementInput;
use investment_plans::{
    BucketAllocationRatio, ExecutionPreviewStatus, InvestmentPlan, InvestmentPlanExecutionPreview,
    PreviewInvestmentPlanExecution, ScheduleKind, TwoBucketAllocationConfig,
};
use market_data::MarketSignalInput;
use quant_engine::{
    evaluate_fundamental, evaluate_trend, FundamentalConfig, FundamentalSignal,
    FundamentalSnapshot, TrendConfig, TrendRegime, TrendSignal, TrendSnapshot,
};
use rust_decimal::{prelude::ToPrimitive, Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use strategy_dsl::{
    DslEvidence, DslRuntimeAction, StrategySpec, TechnicalClose, TechnicalMarketSnapshot,
    TechnicalVix,
};
use strategy_policy::DecisionContext;
use time::{Date, Month};
use tokio::time::timeout;
use uuid::Uuid;

use crate::{ApiError, ApiState};

use super::market_sentiment::MarketSentimentResponse;

const BROKER_SUBMIT_TIMEOUT: Duration = Duration::from_secs(5);

/// Decision preview request DTO.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DecisionPreviewRequest {
    /// Month day used by the execution preview.
    day_of_month: i16,
    /// Legacy bucket allocation retained for request compatibility.
    ///
    /// The persisted plan configuration is authoritative and this value is not
    /// used to override it.
    bucket_allocation: Option<TwoBucketAllocationRequest>,
    /// Fundamental signal snapshot; only legacy Core/Opportunity V1 requires it.
    fundamental: Option<FundamentalSignalRequest>,
    /// Trend signal snapshot; only legacy Core/Opportunity V1 requires it.
    trend: Option<TrendSignalRequest>,
    /// Optional paper order to submit when the execution date is due and the request is valid.
    paper_order: Option<PaperOrderRequest>,
    /// Trusted source disclosure attached only by server-side automatic orchestration.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    input_source: Option<Value>,
    /// Runtime evidence constructed exclusively by the server for a bound DSL strategy.
    #[serde(skip_deserializing, skip_serializing, default)]
    dsl_evidence: Option<DslEvidence>,
}

/// Server-sourced decision request that deliberately excludes 70/20 signal fields.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AutomaticDecisionPreviewRequest {
    /// Legacy bucket allocation retained for request compatibility.
    ///
    /// The persisted plan configuration is authoritative and this value is not
    /// used to override it.
    bucket_allocation: Option<TwoBucketAllocationRequest>,
    /// Optional operator-confirmed paper order submitted only after automatic evaluation.
    paper_order: Option<PaperOrderRequest>,
}

/// Origin of an audit record's 70/20 inputs.
#[derive(Debug, Clone, Copy)]
enum DecisionTrigger {
    /// An operator supplied validated signal values through the legacy preview endpoint.
    ManualInput,
    /// An operator explicitly requested server-sourced automatic inputs.
    AutomaticPreview,
    /// The configured periodic background scheduler created the automatic decision.
    AutomaticScheduler,
}

/// Bucket allocation request DTO.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
struct TwoBucketAllocationRequest {
    /// Core bucket ratio.
    #[serde(with = "rust_decimal::serde::str")]
    core_ratio: Decimal,
    /// Opportunity bucket ratio.
    #[serde(with = "rust_decimal::serde::str")]
    opportunity_ratio: Decimal,
}

/// Fundamental signal request DTO.
#[derive(Debug, Deserialize, Serialize)]
struct FundamentalSignalRequest {
    /// Composite fundamental score in `[0.0, 1.0]`.
    score: f64,
    /// Raw CAPE percentile in `[0.0, 1.0]`.
    cape_percentile: f64,
    /// Raw ERP percentile in `[0.0, 1.0]`.
    erp_percentile: f64,
}

/// Trend signal request DTO.
#[derive(Debug, Deserialize, Serialize)]
struct TrendSignalRequest {
    /// Composite trend score in `[0.0, 1.0]`.
    score: f64,
    /// Raw MA distance percentile in `[0.0, 1.0]`.
    ma_distance_percentile: f64,
    /// Raw RSI percentile in `[0.0, 1.0]`.
    rsi_percentile: f64,
    /// Raw VIX percentile in `[0.0, 1.0]`.
    vix_percentile: f64,
    /// Discrete trend regime.
    regime: TrendRegimeRequest,
}

/// Optional paper order request DTO.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct PaperOrderRequest {
    /// Stable idempotency key for this preview-triggered paper order.
    idempotency_key: String,
    /// Buy or sell side.
    side: BrokerOrderSideRequest,
    /// Market or limit order type.
    order_type: BrokerOrderTypeRequest,
    /// Legacy client-provided quantity. When present it must match the server-calculated
    /// whole-share quantity from the audited recommended amount.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    quantity: Option<Decimal>,
    /// Positive limit price when `order_type` is limit.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    limit_price: Option<Decimal>,
}

/// API trend regime values.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum TrendRegimeRequest {
    /// Overheated market regime.
    Overheated,
    /// Neutral market regime.
    Neutral,
    /// Falling-knife market regime.
    FallingKnife,
}

/// API broker order side values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum BrokerOrderSideRequest {
    /// Buy side.
    Buy,
    /// Sell side.
    Sell,
}

/// API broker order type values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum BrokerOrderTypeRequest {
    /// Market order.
    Market,
    /// Limit order.
    Limit,
}

/// Decision preview response DTO.
#[derive(Debug, Serialize)]
struct DecisionPreviewResponse {
    /// Persisted local audit-record ID for this decision.
    audit_record_id: Uuid,
    /// Execution preview from the investment-plan service.
    execution: InvestmentPlanExecutionPreview,
    /// Decision result safe for API clients.
    decision: DecisionResponse,
    /// AI rationale, risk warnings, and RSS sources used for this decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    market_sentiment: Option<MarketSentimentResponse>,
    /// Paper order acknowledgement when an executable due preview submitted an order.
    #[serde(skip_serializing_if = "Option::is_none")]
    paper_order_ack: Option<BrokerOrderAck>,
    /// Human-readable summary for demo UI.
    summary: String,
}

/// Result counters emitted by one periodic automatic scheduler tick.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ScheduledDecisionRunSummary {
    /// Due active plans for which a new automatic audit record was created.
    pub created: u32,
    /// Earlier fixed dates in the current period that were safely caught up after downtime.
    pub catch_up_created: u32,
    /// Due plans already claimed for the same UTC calendar day.
    pub already_claimed: u32,
    /// Due plans skipped because automatic market data was unavailable or invalid.
    pub unavailable: u32,
}

/// API-facing decision response.
#[derive(Debug, Serialize)]
struct DecisionResponse {
    /// 策略版本引用。
    policy: PolicyResponse,
    /// 本次策略是否读取了市场或 AI 信号。
    market_signals_used: bool,
    /// Final investability score.
    #[serde(skip_serializing_if = "Option::is_none")]
    final_score: Option<f64>,
    /// Contribution multiplier.
    multiplier: f64,
    /// Final action label.
    action: ActionResponse,
    /// Weight mode used by the decision engine.
    #[serde(skip_serializing_if = "Option::is_none")]
    weight_mode: Option<DecisionWeightModeResponse>,
    /// Fundamental contribution score after direction normalization.
    #[serde(skip_serializing_if = "Option::is_none")]
    fundamental_score: Option<f64>,
    /// Trend timing contribution score after safety normalization.
    #[serde(skip_serializing_if = "Option::is_none")]
    trend_score: Option<f64>,
    /// Sentiment contribution score when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    sentiment_score: Option<f64>,
}

/// API 中的不可变策略版本标识。
#[derive(Debug, Serialize)]
struct PolicyResponse {
    /// 稳定策略 ID。
    id: String,
    /// 不可变版本号。
    version: u32,
}

/// API action values.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ActionResponse {
    /// Increase contribution.
    Overweight,
    /// Standard contribution.
    Standard,
    /// Delay execution tactically.
    TacticalDelay,
    /// Reduce contribution.
    Underweight,
    /// Skip this execution.
    Skip,
}

/// API decision weight mode values.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum DecisionWeightModeResponse {
    /// Normal 70/20/10 weights.
    Normal,
    /// Sentiment-unavailable fallback weights.
    SentimentUnavailable,
}

/// Build decision preview routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route(
            "/investment-plans/:id/decision-preview",
            post(preview_decision),
        )
        .route(
            "/investment-plans/:id/automatic-decision-preview",
            post(preview_automatic_decision),
        )
}

/// Preview one investment decision and optionally submit a configured paper order.
async fn preview_decision(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    input: Result<Json<DecisionPreviewRequest>, JsonRejection>,
) -> Result<Json<DecisionPreviewResponse>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;

    Ok(Json(
        preview_decision_input(
            &state,
            id,
            input,
            DecisionTrigger::ManualInput,
            Utc::now().date_naive(),
        )
        .await?,
    ))
}

/// Run one server-sourced decision preview without accepting caller-supplied 70/20 inputs.
async fn preview_automatic_decision(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    input: Result<Json<AutomaticDecisionPreviewRequest>, JsonRejection>,
) -> Result<Json<DecisionPreviewResponse>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let day_of_month = i16::try_from(Utc::now().day()).map_err(|_| ApiError::ServiceUnavailable)?;
    Ok(Json(
        preview_automatic_for_plan(
            &state,
            id,
            day_of_month,
            DecisionTrigger::AutomaticPreview,
            input,
        )
        .await?,
    ))
}

/// Execute one due-plan scheduler tick, including still-unclaimed fixed dates in this period.
pub(crate) async fn run_due_decisions(
    state: &ApiState,
) -> Result<ScheduledDecisionRunSummary, ApiError> {
    let today = Utc::now().date_naive();
    let mut summary = ScheduledDecisionRunSummary::default();

    for plan in state.plans().list().await? {
        if !plan.is_active {
            continue;
        }
        for scheduled_date in due_dates_in_current_period(&plan, today) {
            match preview_automatic_for_plan_with_claim(state, plan.id, scheduled_date).await {
                Ok(Some(_)) => {
                    if scheduled_date == today {
                        summary.created += 1;
                    } else {
                        summary.catch_up_created += 1;
                    }
                }
                Ok(None) => summary.already_claimed += 1,
                Err(ApiError::ServiceUnavailable | ApiError::BadRequest) => {
                    summary.unavailable += 1;
                    tracing::warn!(plan_id = %plan.id, scheduled_for = %scheduled_date, "automatic decision skipped because market inputs were unavailable");
                }
                Err(error) => return Err(error),
            }
        }
    }

    Ok(summary)
}

/// Build source-labelled automatic inputs, claim the plan/day key, then persist one audit record.
async fn preview_automatic_for_plan_with_claim(
    state: &ApiState,
    plan_id: Uuid,
    scheduled_for: NaiveDate,
) -> Result<Option<DecisionPreviewResponse>, ApiError> {
    let plan = state.plans().get(plan_id).await?;
    let day_of_month =
        i16::try_from(scheduled_for.day()).map_err(|_| ApiError::ServiceUnavailable)?;
    let input = automatic_decision_input(
        state,
        &plan,
        day_of_month,
        AutomaticDecisionPreviewRequest {
            bucket_allocation: None,
            paper_order: None,
        },
    )
    .await?;
    if !state
        .claim_scheduled_decision(plan_id, &scheduled_for.to_string())
        .await?
    {
        return Ok(None);
    }
    let result = preview_decision_input(
        state,
        plan_id,
        input,
        DecisionTrigger::AutomaticScheduler,
        scheduled_for,
    )
    .await;
    match result {
        Ok(response) => Ok(Some(response)),
        Err(error) => {
            state
                .release_scheduled_decision(plan_id, &scheduled_for.to_string())
                .await;
            Err(error)
        }
    }
}

/// Build one automatic preview from trusted provider data for a selected investment symbol.
async fn preview_automatic_for_plan(
    state: &ApiState,
    plan_id: Uuid,
    day_of_month: i16,
    trigger: DecisionTrigger,
    options: AutomaticDecisionPreviewRequest,
) -> Result<DecisionPreviewResponse, ApiError> {
    let plan = state.plans().get(plan_id).await?;
    let input = automatic_decision_input(state, &plan, day_of_month, options).await?;
    preview_decision_input(state, plan_id, input, trigger, Utc::now().date_naive()).await
}

/// Resolve automatic market snapshots into the same validated request shape used by the engine.
async fn automatic_decision_input(
    state: &ApiState,
    plan: &InvestmentPlan,
    day_of_month: i16,
    options: AutomaticDecisionPreviewRequest,
) -> Result<DecisionPreviewRequest, ApiError> {
    if state.policy_resolver().supports(&plan.policy)
        && state
            .policy_resolver()
            .evidence_kind(&plan.policy)
            .map_err(map_policy_error)?
            == BuiltinPolicyEvidenceKind::FixedDca
    {
        return Ok(DecisionPreviewRequest {
            day_of_month,
            bucket_allocation: options.bucket_allocation,
            fundamental: None,
            trend: None,
            paper_order: options.paper_order,
            input_source: Some(json!({
                "kind": "fixed_dca",
                "description": "fixed_dca does not read market or AI signals",
            })),
            dsl_evidence: None,
        });
    }

    let input = state.market_signal_input(&plan.symbol).await?;
    let fundamental = evaluate_fundamental(
        &FundamentalSnapshot {
            cape_history: input.cape_history.clone(),
            cape_current: input.cape_current,
            erp_history: input.erp_history.clone(),
            erp_current: input.erp_current,
        },
        &FundamentalConfig::default(),
    )
    .map_err(|_| ApiError::ServiceUnavailable)?;
    let trend = evaluate_trend(
        &TrendSnapshot {
            ma_distance_history: input.ma_distance_history.clone(),
            ma_distance_current: input.ma_distance_current,
            rsi_history: input.rsi_history.clone(),
            rsi_current: input.rsi_current,
            vix_history: input.vix_history.clone(),
            vix_current: input.vix_current,
        },
        &TrendConfig::default(),
    )
    .map_err(|_| ApiError::ServiceUnavailable)?;

    let (dsl_evidence, input_source) = if state.policy_resolver().supports(&plan.policy) {
        (None, automatic_source_snapshot(&input))
    } else {
        let strategy = state
            .get_strategy_spec(&plan.policy)
            .await?
            .document
            .into_strategy_spec()
            .map_err(|_| ApiError::ServiceUnavailable)?;
        let mut source = automatic_source_snapshot(&input);
        source["dsl_runtime"] = json!({
            "as_of": input.as_of,
            "price_source": "local OpenD daily closes through the decision as_of date",
            "volatility_source": "Cboe VIX latest validated observation",
            "indicators": strategy.required_indicators().into_iter().map(|indicator| format!("{indicator:?}")).collect::<Vec<_>>(),
        });
        (
            Some(dsl_evidence_for_live_runtime(state, &strategy, &plan.symbol, &input).await?),
            source,
        )
    };

    Ok(DecisionPreviewRequest {
        day_of_month,
        bucket_allocation: options.bucket_allocation,
        fundamental: Some(FundamentalSignalRequest::from(fundamental)),
        trend: Some(TrendSignalRequest::from(trend)),
        paper_order: options.paper_order,
        input_source: Some(input_source),
        dsl_evidence,
    })
}

/// Build the current online Runtime evidence profile from one trusted market snapshot.
///
/// The Studio intentionally exposes only RSI(14) and VIX because those are the two raw values
/// supplied together by the existing automatic market-data adapter.  Other DSL indicators remain
/// valid for offline research but cannot be activated until a dedicated data adapter is added.
async fn dsl_evidence_for_live_runtime(
    state: &ApiState,
    strategy: &StrategySpec,
    symbol: &str,
    input: &MarketSignalInput,
) -> Result<DslEvidence, ApiError> {
    let as_of = evidence_as_of(&input.as_of)?;
    let vix = Decimal::from_f64_retain(input.vix_current).ok_or(ApiError::ServiceUnavailable)?;
    let prices = state.market_price_history(symbol, 366).await?;
    let closes = prices
        .iter()
        .map(|point| {
            let close =
                Decimal::from_f64_retain(point.close).ok_or(ApiError::ServiceUnavailable)?;
            TechnicalClose::new(evidence_as_of(&point.date)?, close)
                .map_err(|_| ApiError::ServiceUnavailable)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let snapshot = TechnicalMarketSnapshot::new(
        as_of,
        closes,
        TechnicalVix::new(evidence_as_of(&input.vix_as_of)?, vix)
            .map_err(|_| ApiError::ServiceUnavailable)?,
    )
    .map_err(|_| ApiError::ServiceUnavailable)?;
    DslEvidence::from_as_of_market_snapshot(strategy, &snapshot).map_err(|_| ApiError::BadRequest)
}

/// Parse a provider ISO date into the single DSL evidence cutoff type.
fn evidence_as_of(value: &str) -> Result<Date, ApiError> {
    let date =
        NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| ApiError::ServiceUnavailable)?;
    Date::from_calendar_date(
        date.year(),
        Month::try_from(date.month() as u8).map_err(|_| ApiError::ServiceUnavailable)?,
        date.day() as u8,
    )
    .map_err(|_| ApiError::ServiceUnavailable)
}

/// Execute a resolved decision and create its audit record before any optional broker side effect.
async fn preview_decision_input(
    state: &ApiState,
    id: Uuid,
    input: DecisionPreviewRequest,
    trigger: DecisionTrigger,
    execution_date: NaiveDate,
) -> Result<DecisionPreviewResponse, ApiError> {
    let execution_input = PreviewInvestmentPlanExecution::for_date(
        input.day_of_month,
        i16::try_from(execution_date.weekday().num_days_from_monday() + 1)
            .map_err(|_| ApiError::ServiceUnavailable)?,
    )?;
    let plan = state.plans().get(id).await?;
    let legacy_bucket_config = input
        .bucket_allocation
        .map(TwoBucketAllocationRequest::into_domain)
        .transpose()?;
    if legacy_bucket_config.is_some() {
        tracing::warn!(plan_id = %id, "legacy decision-preview bucket allocation ignored; persisted plan configuration is authoritative");
    }
    let is_legacy_core = state.policy_resolver().evidence_kind(&plan.policy).ok()
        == Some(BuiltinPolicyEvidenceKind::CoreOpportunity);
    let market_sentiment = if is_legacy_core {
        market_sentiment_for_decision(state).await
    } else {
        None
    };
    let market_sentiment_response = market_sentiment.as_ref().map(MarketSentimentResponse::from);
    let decision = resolve_policy_decision(
        state,
        &plan,
        execution_date,
        &input,
        market_sentiment.as_ref(),
    )
    .await?;
    let carried_opportunity_cash = state.opportunity_cash_balance(id).await?;
    let execution = state
        .plans()
        .preview_execution_with_decision_and_cash(
            id,
            execution_input,
            decision.recommendation().action(),
            decision.recommendation().multiplier(),
            carried_opportunity_cash,
        )
        .await
        .inspect_err(|error| tracing::warn!(%error, policy = %plan.policy, "plan execution preview rejected policy recommendation"))?;
    let paper_order = match input.paper_order.clone() {
        Some(order)
            if matches!(
                trigger,
                DecisionTrigger::AutomaticPreview | DecisionTrigger::AutomaticScheduler
            ) && execution.status == ExecutionPreviewStatus::Due
                && has_positive_recommended_contribution(&execution) =>
        {
            let quantity = recommended_quantity(state, &execution, &order).await?;
            Some(order.into_domain(&execution.symbol, quantity)?)
        }
        Some(order) => {
            let quantity = order.quantity.ok_or(ApiError::BadRequest)?;
            Some(order.into_domain(&execution.symbol, quantity)?)
        }
        None => None,
    };
    // Approval-mode plans must retain a decision-only record first. The later approval endpoint
    // derives an order from this immutable snapshot instead of accepting a second preview's input.
    let paper_order = if execution
        .bucket_split
        .is_some_and(|split| split.requires_approval())
    {
        None
    } else {
        paper_order
    };
    let decision_response = DecisionResponse::from_policy_decision(&decision);
    let should_submit = should_submit_paper_order(&execution, paper_order.as_ref());
    let preliminary_summary = summarize_decision(&execution, &decision, None);
    let persisted = state
        .decision_records()
        .create(record_input(DecisionRecordContext {
            plan_id: id,
            input: &input,
            execution: &execution,
            policy: &plan.policy,
            decision: &decision,
            market_sentiment: market_sentiment.as_ref(),
            trigger,
            paper_order: paper_order.as_ref(),
            paper_order_ack: None,
            summary: preliminary_summary,
        })?)
        .await?;
    let paper_order_ack = if should_submit {
        let request = paper_order.as_ref().ok_or(ApiError::ServiceUnavailable)?;
        let automatic_trigger = matches!(
            trigger,
            DecisionTrigger::AutomaticPreview | DecisionTrigger::AutomaticScheduler
        );
        if automatic_trigger {
            let plan = state.plans().get(id).await?;
            let scheduled_for = execution_date;
            let limit = plan
                .execution_configuration
                .period_execution_limit()
                .unwrap_or_else(|| {
                    plan.max_single_execution * Decimal::from(plan.schedule_days.len() as u32)
                });
            let amount = execution
                .bucket_split
                .map_or(plan.base_contribution, |split| {
                    split.recommended_contribution()
                });
            if !state
                .reserve_period_execution(
                    id,
                    persisted.id,
                    &period_key(plan.schedule_kind, scheduled_for),
                    limit,
                    amount,
                )
                .await?
            {
                return Err(ApiError::BadRequest);
            }
        }
        let ack = match submit_paper_order(state, request).await {
            Ok(ack) => ack,
            Err(error) => {
                if automatic_trigger {
                    state.release_period_execution(persisted.id).await;
                }
                return Err(error);
            }
        };
        if automatic_trigger {
            state.accept_period_execution(persisted.id).await?;
        }
        let summary = summarize_decision(&execution, &decision, Some(&ack));
        if let Err(error) = state
            .decision_records()
            .complete_broker_order(
                persisted.id,
                CompleteDecisionRecord {
                    broker_order_ack: snapshot(&ack)?,
                    summary: summary.clone(),
                },
            )
            .await
        {
            tracing::error!(error = %error, record_id = %persisted.id, "paper order accepted but decision record completion failed");
        }
        state
            .record_accepted_paper_order(id, persisted.id, &ack, request)
            .await?;
        if matches!(
            trigger,
            DecisionTrigger::AutomaticPreview | DecisionTrigger::AutomaticScheduler
        ) {
            if let Some(split) = execution.bucket_split {
                let scheduled_for = execution_date.to_string();
                state
                    .settle_opportunity_cash(OpportunityCashSettlementInput {
                        plan_id: id,
                        decision_record_id: persisted.id,
                        scheduled_for: &scheduled_for,
                        policy: split.opportunity_cash_policy(),
                        cash_cap: state
                            .plans()
                            .get(id)
                            .await?
                            .execution_configuration
                            .opportunity_cash_cap(),
                        period_budget: split.opportunity_budget(),
                        core_contribution: split.core_contribution(),
                        allocated_amount: split.opportunity_contribution(),
                    })
                    .await?;
            }
        }
        Some(ack)
    } else {
        None
    };
    let summary = summarize_decision(&execution, &decision, paper_order_ack.as_ref());

    Ok(DecisionPreviewResponse {
        audit_record_id: persisted.id,
        execution,
        decision: decision_response,
        market_sentiment: market_sentiment_response,
        paper_order_ack,
        summary,
    })
}

/// 通过计划绑定的策略解析一次统一推荐；策略本身不进行 IO。
async fn resolve_policy_decision(
    state: &ApiState,
    plan: &InvestmentPlan,
    execution_date: NaiveDate,
    input: &DecisionPreviewRequest,
    market_sentiment: Option<&MarketSentimentReport>,
) -> Result<BuiltinPolicyDecision, ApiError> {
    let date = Date::from_calendar_date(
        execution_date.year(),
        Month::try_from(execution_date.month() as u8).map_err(|_| ApiError::BadRequest)?,
        execution_date.day() as u8,
    )
    .map_err(|_| ApiError::BadRequest)?;
    if !state.policy_resolver().supports(&plan.policy) {
        let strategy = state
            .get_strategy_spec(&plan.policy)
            .await?
            .document
            .into_strategy_spec()
            .map_err(|_| ApiError::ServiceUnavailable)?;
        let evidence = input.dsl_evidence.clone().ok_or(ApiError::BadRequest)?;
        let context = DecisionContext::new(date, plan.base_contribution, evidence)
            .map_err(|_| ApiError::BadRequest)?;
        let evaluation = strategy.evaluate(&context).map_err(|error| {
            tracing::warn!(%error, policy = %plan.policy, "DSL policy evidence could not be evaluated");
            ApiError::BadRequest
        })?;
        if matches!(
            evaluation.action(),
            DslRuntimeAction::OpportunityFixedAmount(_)
        ) {
            // Fixed-dollar opportunity actions will become executable only when the execution
            // layer can represent them without converting through a multiplier.
            return Err(ApiError::BadRequest);
        }
        return Ok(BuiltinPolicyDecision::FixedDca {
            recommendation: evaluation.recommendation().clone(),
        });
    }
    let evidence = match state
        .policy_resolver()
        .evidence_kind(&plan.policy)
        .map_err(map_policy_error)?
    {
        BuiltinPolicyEvidenceKind::FixedDca => BuiltinPolicyEvidence::FixedDca,
        BuiltinPolicyEvidenceKind::CoreOpportunity => {
            let fundamental = input
                .fundamental
                .as_ref()
                .ok_or(ApiError::BadRequest)?
                .clone_signal()?;
            let trend = input
                .trend
                .as_ref()
                .ok_or(ApiError::BadRequest)?
                .clone_signal()?;
            BuiltinPolicyEvidence::CoreOpportunity(CoreOpportunityEvidence::new(DecisionInput {
                fundamental,
                trend,
                sentiment: market_sentiment.map_or(DecisionSentiment::Unavailable, |report| {
                    DecisionSentiment::Available(report.analysis.sentiment())
                }),
            }))
        }
    };

    state
        .policy_resolver()
        .evaluate(&plan.policy, date, plan.base_contribution, evidence)
        .map_err(map_policy_error)
}

/// 将策略 resolver 的安全领域错误映射为不暴露实现细节的 HTTP 错误。
fn map_policy_error(error: BuiltinPolicyError) -> ApiError {
    tracing::warn!(%error, "plan policy could not be resolved");
    match error {
        BuiltinPolicyError::UnsupportedPolicy(_)
        | BuiltinPolicyError::EvidenceDoesNotMatchPolicy => ApiError::BadRequest,
        BuiltinPolicyError::InvalidContext(_) => ApiError::BadRequest,
    }
}

/// Return a stable weekly or monthly UTC period key for the atomic budget ledger.
fn period_key(schedule_kind: ScheduleKind, date: NaiveDate) -> String {
    match schedule_kind {
        ScheduleKind::Monthly => date.format("%Y-%m").to_string(),
        ScheduleKind::Weekly => date.format("%G-W%V").to_string(),
    }
}

/// Return the due dates from the current configured week or month up to `today`.
///
/// The persisted `(plan_id, scheduled_for)` claim keeps restarts idempotent. Catch-up only
/// covers the current plan period, so a long outage never silently replays old market days.
fn due_dates_in_current_period(plan: &InvestmentPlan, today: NaiveDate) -> Vec<NaiveDate> {
    match plan.schedule_kind {
        ScheduleKind::Monthly => plan
            .schedule_days
            .iter()
            .filter_map(|day| NaiveDate::from_ymd_opt(today.year(), today.month(), *day as u32))
            .filter(|date| *date <= today)
            .collect(),
        ScheduleKind::Weekly => {
            let monday =
                today - chrono::Duration::days(today.weekday().num_days_from_monday().into());
            plan.schedule_days
                .iter()
                .filter_map(|weekday| {
                    monday.checked_add_signed(chrono::Duration::days(i64::from(*weekday - 1)))
                })
                .filter(|date| *date <= today)
                .collect()
        }
    }
}

impl TwoBucketAllocationRequest {
    fn into_domain(self) -> Result<TwoBucketAllocationConfig, ApiError> {
        TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(self.core_ratio)?,
            BucketAllocationRatio::new(self.opportunity_ratio)?,
        )
        .map_err(|_| ApiError::BadRequest)
    }
}

impl FundamentalSignalRequest {
    fn clone_signal(&self) -> Result<FundamentalSignal, ApiError> {
        Ok(FundamentalSignal {
            score: percentile(self.score)?,
            cape_percentile: percentile(self.cape_percentile)?,
            erp_percentile: percentile(self.erp_percentile)?,
        })
    }
}

impl From<FundamentalSignal> for FundamentalSignalRequest {
    fn from(signal: FundamentalSignal) -> Self {
        Self {
            score: signal.score.value(),
            cape_percentile: signal.cape_percentile.value(),
            erp_percentile: signal.erp_percentile.value(),
        }
    }
}

impl TrendSignalRequest {
    fn clone_signal(&self) -> Result<TrendSignal, ApiError> {
        Ok(TrendSignal {
            score: percentile(self.score)?,
            ma_distance_percentile: percentile(self.ma_distance_percentile)?,
            rsi_percentile: percentile(self.rsi_percentile)?,
            vix_percentile: percentile(self.vix_percentile)?,
            regime: self.regime.to_domain(),
        })
    }
}

impl From<TrendSignal> for TrendSignalRequest {
    fn from(signal: TrendSignal) -> Self {
        Self {
            score: signal.score.value(),
            ma_distance_percentile: signal.ma_distance_percentile.value(),
            rsi_percentile: signal.rsi_percentile.value(),
            vix_percentile: signal.vix_percentile.value(),
            regime: signal.regime.into(),
        }
    }
}

impl PaperOrderRequest {
    fn into_domain(self, symbol: &str, quantity: Decimal) -> Result<BrokerOrderRequest, ApiError> {
        match self.order_type {
            BrokerOrderTypeRequest::Market => {
                if self.limit_price.is_some() {
                    return Err(ApiError::BadRequest);
                }
                BrokerOrderRequest::market(
                    self.idempotency_key,
                    symbol,
                    self.side.into(),
                    quantity,
                    broker::BrokerEnvironment::Paper,
                )
            }
            BrokerOrderTypeRequest::Limit => BrokerOrderRequest::limit(
                self.idempotency_key,
                symbol,
                self.side.into(),
                quantity,
                self.limit_price.ok_or(ApiError::BadRequest)?,
                broker::BrokerEnvironment::Paper,
            ),
        }
        .map_err(|_| ApiError::BadRequest)
    }
}

/// Convert the persisted decision recommendation into a conservative whole-share paper order.
///
/// A caller cannot enlarge the order by supplying its own quantity. Limit orders use their
/// limit price; market orders use the newest local OpenD close only as a budgeting estimate.
async fn recommended_quantity(
    state: &ApiState,
    execution: &InvestmentPlanExecutionPreview,
    order: &PaperOrderRequest,
) -> Result<Decimal, ApiError> {
    if order.side != BrokerOrderSideRequest::Buy || execution.status != ExecutionPreviewStatus::Due
    {
        return Err(ApiError::BadRequest);
    }
    let amount = execution
        .bucket_split
        .map(|split| split.recommended_contribution())
        .or(execution.planned_contribution)
        .ok_or(ApiError::BadRequest)?;
    let portfolio = state.paper_portfolio().await?;
    if portfolio.currency != execution.currency || amount > portfolio.buying_power {
        return Err(ApiError::BadRequest);
    }
    let price = match order.order_type {
        BrokerOrderTypeRequest::Limit => order.limit_price.ok_or(ApiError::BadRequest)?,
        BrokerOrderTypeRequest::Market => state.latest_market_price(&execution.symbol).await?,
    };
    let quantity = (amount / price).round_dp_with_strategy(0, RoundingStrategy::ToZero);
    if quantity <= Decimal::ZERO || quantity.to_i64().is_none() {
        return Err(ApiError::BadRequest);
    }
    if let Some(supplied) = order.quantity {
        if supplied != quantity {
            return Err(ApiError::BadRequest);
        }
    }
    Ok(quantity)
}

impl TrendRegimeRequest {
    fn to_domain(&self) -> TrendRegime {
        match self {
            Self::Overheated => TrendRegime::Overheated,
            Self::Neutral => TrendRegime::Neutral,
            Self::FallingKnife => TrendRegime::FallingKnife,
        }
    }
}

impl From<TrendRegime> for TrendRegimeRequest {
    fn from(value: TrendRegime) -> Self {
        match value {
            TrendRegime::Overheated => Self::Overheated,
            TrendRegime::Neutral => Self::Neutral,
            TrendRegime::FallingKnife => Self::FallingKnife,
        }
    }
}

impl From<BrokerOrderSideRequest> for BrokerOrderSide {
    fn from(value: BrokerOrderSideRequest) -> Self {
        match value {
            BrokerOrderSideRequest::Buy => Self::Buy,
            BrokerOrderSideRequest::Sell => Self::Sell,
        }
    }
}

impl DecisionResponse {
    fn from_policy_decision(decision: &BuiltinPolicyDecision) -> Self {
        let recommendation = decision.recommendation();
        let policy = PolicyResponse {
            id: recommendation.policy().id().as_str().to_owned(),
            version: recommendation.policy().version().value(),
        };
        match decision {
            BuiltinPolicyDecision::CoreOpportunity { signal, .. } => Self {
                policy,
                market_signals_used: true,
                final_score: Some(signal.final_score.value()),
                multiplier: signal.multiplier.value(),
                action: signal.action.into(),
                weight_mode: Some(signal.weight_mode.into()),
                fundamental_score: Some(signal.fundamental_score.value()),
                trend_score: Some(signal.trend_score.value()),
                sentiment_score: signal.sentiment_score.map(Percentile::value),
            },
            BuiltinPolicyDecision::FixedDca { .. } => Self {
                market_signals_used: policy.id.starts_with("dsl_"),
                policy,
                final_score: None,
                multiplier: recommendation.multiplier().value(),
                action: recommendation.action().into(),
                weight_mode: None,
                fundamental_score: None,
                trend_score: None,
                sentiment_score: None,
            },
        }
    }
}

impl From<Action> for ActionResponse {
    fn from(value: Action) -> Self {
        match value {
            Action::Overweight => Self::Overweight,
            Action::Standard => Self::Standard,
            Action::TacticalDelay => Self::TacticalDelay,
            Action::Underweight => Self::Underweight,
            Action::Skip => Self::Skip,
        }
    }
}

impl From<DecisionWeightMode> for DecisionWeightModeResponse {
    fn from(value: DecisionWeightMode) -> Self {
        match value {
            DecisionWeightMode::Normal => Self::Normal,
            DecisionWeightMode::SentimentUnavailable => Self::SentimentUnavailable,
        }
    }
}

fn percentile(value: f64) -> Result<Percentile, ApiError> {
    Percentile::new(value).ok_or(ApiError::BadRequest)
}

/// Return whether the validated order is safe and eligible to submit.
///
/// The domain split already reduces the opportunity bucket to zero for `Skip`
/// and `TacticalDelay`; a due paper order must still carry the preserved core
/// bucket rather than treating either action as a global veto.
fn should_submit_paper_order(
    execution: &InvestmentPlanExecutionPreview,
    paper_order: Option<&BrokerOrderRequest>,
) -> bool {
    paper_order.is_some()
        && execution.status == ExecutionPreviewStatus::Due
        && has_positive_recommended_contribution(execution)
}

/// Return whether the domain preview contains a positive safe order amount.
fn has_positive_recommended_contribution(execution: &InvestmentPlanExecutionPreview) -> bool {
    execution
        .bucket_split
        .map(|split| split.recommended_contribution())
        .or(execution.planned_contribution)
        .is_some_and(|amount| amount > Decimal::ZERO)
}

/// Submit one already-validated paper order through the configured broker.
async fn submit_paper_order(
    state: &ApiState,
    request: &BrokerOrderRequest,
) -> Result<BrokerOrderAck, ApiError> {
    timeout(
        BROKER_SUBMIT_TIMEOUT,
        state.broker().submit_order(request.clone()),
    )
    .await
    .map_err(|_| ApiError::ServiceUnavailable)?
    .map_err(Into::into)
}

/// Fetch Qwen market sentiment and safely fall back to the engine's 90/10/0 mode.
async fn market_sentiment_for_decision(state: &ApiState) -> Option<MarketSentimentReport> {
    match state.market_sentiment().await {
        Ok(sentiment) => Some(sentiment),
        Err(ApiError::ServiceUnavailable) => {
            tracing::warn!("market sentiment unavailable; decision preview uses fallback weights");
            None
        }
        Err(error) => {
            tracing::error!(error = %error, "unexpected market sentiment error; decision preview uses fallback weights");
            None
        }
    }
}

/// Borrowed decision material required to create one local audit record.
struct DecisionRecordContext<'a> {
    plan_id: Uuid,
    input: &'a DecisionPreviewRequest,
    execution: &'a InvestmentPlanExecutionPreview,
    policy: &'a strategy_policy::PolicyRef,
    decision: &'a BuiltinPolicyDecision,
    market_sentiment: Option<&'a MarketSentimentReport>,
    trigger: DecisionTrigger,
    paper_order: Option<&'a BrokerOrderRequest>,
    paper_order_ack: Option<&'a BrokerOrderAck>,
    summary: String,
}

/// Build a complete local audit snapshot before any optional broker side effect.
fn record_input(context: DecisionRecordContext<'_>) -> Result<CreateDecisionRecord, ApiError> {
    Ok(CreateDecisionRecord {
        plan_id: context.plan_id,
        symbol: context.execution.symbol.clone(),
        currency: context.execution.currency.clone(),
        execution_status: execution_status(context.execution.status),
        planned_contribution: context
            .execution
            .planned_contribution
            .map(|value| value.to_string()),
        execution_snapshot: json!({
            "trigger": trigger_label(context.trigger),
            "policy": {
                "id": context.policy.id().as_str(),
                "version": context.policy.version().value(),
            },
            "execution": snapshot(context.execution)?,
        }),
        fundamental_snapshot: policy_signal_snapshot(
            "fundamental",
            context.input.fundamental.as_ref(),
            context.input.input_source.as_ref(),
            context.policy,
        )?,
        trend_snapshot: policy_signal_snapshot(
            "trend",
            context.input.trend.as_ref(),
            context.input.input_source.as_ref(),
            context.policy,
        )?,
        sentiment_snapshot: context.market_sentiment.map(market_sentiment_snapshot),
        decision_snapshot: decision_snapshot(context.decision),
        policy_evidence: Some(DecisionPolicyEvidence {
            policy: context.policy.clone(),
            recommendation_snapshot: recommendation_snapshot(context.decision),
        }),
        broker_order_request: context.paper_order.map(snapshot).transpose()?,
        broker_order_ack: context.paper_order_ack.map(snapshot).transpose()?,
        summary: context.summary,
    })
}

/// Build the policy-neutral recommendation evidence retained with every new audit record.
fn recommendation_snapshot(decision: &BuiltinPolicyDecision) -> Value {
    let recommendation = decision.recommendation();
    json!({
        "action": action_label(recommendation.action()),
        "multiplier": recommendation.multiplier().value(),
        "scheduled_contribution": recommendation.scheduled_contribution().to_string(),
        "market_signals_used": decision.legacy_signal().is_some(),
    })
}

/// 保存实际使用的信号，或明确记录固定策略为何没有读取该信号层。
fn policy_signal_snapshot(
    layer: &'static str,
    signal: Option<&impl Serialize>,
    automatic_source: Option<&Value>,
    policy: &strategy_policy::PolicyRef,
) -> Result<Value, ApiError> {
    match signal {
        Some(signal) => signal_snapshot(layer, signal, automatic_source),
        None => Ok(json!({
            "layer": layer,
            "used": false,
            "policy": policy.to_string(),
            "reason": "the selected policy does not expose legacy 70/20 signals",
        })),
    }
}

/// Build a source-labelled 70% or 20% audit snapshot without retaining credentials.
fn signal_snapshot(
    layer: &'static str,
    signal: &impl Serialize,
    automatic_source: Option<&Value>,
) -> Result<Value, ApiError> {
    Ok(json!({
        "layer": layer,
        "source": automatic_source.cloned().unwrap_or_else(|| json!({
            "kind": "operator_input",
            "description": "validated values supplied through the legacy decision-preview endpoint",
        })),
        "signal": snapshot(signal)?,
    }))
}

/// Build the auditable, non-secret provider disclosure for automatic 70/20 inputs.
fn automatic_source_snapshot(input: &MarketSignalInput) -> Value {
    json!({
        "kind": "automatic_market_data",
        "symbol": input.symbol,
        "as_of": input.as_of,
        "vix_as_of": input.vix_as_of,
        "fundamental": "Shiller CAPE monthly table; ERP proxy = 100 / CAPE - US Treasury 10-year yield",
        "trend": "local OpenD daily close with locally computed MA200 and RSI; Cboe VIX last available observation",
    })
}

/// Return a stable trigger label for a persisted execution snapshot.
fn trigger_label(trigger: DecisionTrigger) -> &'static str {
    match trigger {
        DecisionTrigger::ManualInput => "manual_input",
        DecisionTrigger::AutomaticPreview => "automatic_preview",
        DecisionTrigger::AutomaticScheduler => "automatic_scheduler",
    }
}

/// Return a stable audit snapshot for an automatically retrieved market sentiment.
fn market_sentiment_snapshot(report: &MarketSentimentReport) -> Value {
    json!({
        "source": "market_sentiment",
        "score": report.analysis.sentiment().value(),
        "rationale": report.analysis.rationale(),
        "warnings": report.analysis.warnings(),
        "headlines": report.headlines.iter().map(|headline| json!({
            "title": headline.title,
            "url": headline.url,
            "published_at": headline.published_at.to_rfc3339(),
        })).collect::<Vec<_>>(),
    })
}

/// Convert an execution preview status into its persisted audit representation.
fn execution_status(status: ExecutionPreviewStatus) -> DecisionExecutionStatus {
    match status {
        ExecutionPreviewStatus::Due => DecisionExecutionStatus::Due,
        ExecutionPreviewStatus::Waiting => DecisionExecutionStatus::Waiting,
        ExecutionPreviewStatus::Inactive => DecisionExecutionStatus::Inactive,
    }
}

/// Serialize one trusted in-process value into a JSON audit snapshot.
fn snapshot(value: &impl Serialize) -> Result<Value, ApiError> {
    serde_json::to_value(value).map_err(|error| {
        tracing::error!(error = %error, "decision preview audit snapshot serialization failed");
        ApiError::ServiceUnavailable
    })
}

/// Build the decision-output snapshot, including the effective weights.
fn decision_snapshot(decision: &BuiltinPolicyDecision) -> Value {
    let recommendation = decision.recommendation();
    let base = json!({
        "policy": {
            "id": recommendation.policy().id().as_str(),
            "version": recommendation.policy().version().value(),
        },
        "multiplier": recommendation.multiplier().value(),
        "action": action_label(recommendation.action()),
    });
    let Some(decision) = decision.legacy_signal() else {
        return json!({
            "policy": base["policy"].clone(),
            "multiplier": recommendation.multiplier().value(),
            "action": action_label(recommendation.action()),
            "market_signals_used": false,
        });
    };
    json!({
        "policy": base["policy"].clone(),
        "market_signals_used": true,
        "final_score": decision.final_score.value(),
        "multiplier": decision.multiplier.value(),
        "action": action_label(decision.action),
        "weight_mode": weight_mode_label(decision.weight_mode),
        "weights": {
            "fundamental_weight": decision.weights.fundamental_weight.value(),
            "trend_weight": decision.weights.trend_weight.value(),
            "sentiment_weight": decision.weights.sentiment_weight.value(),
        },
        "fundamental_score": decision.fundamental_score.value(),
        "trend_score": decision.trend_score.value(),
        "sentiment_score": decision.sentiment_score.map(Percentile::value),
    })
}

/// Return the stable persisted label for a decision action.
fn action_label(action: Action) -> &'static str {
    match action {
        Action::Overweight => "overweight",
        Action::Standard => "standard",
        Action::TacticalDelay => "tactical_delay",
        Action::Underweight => "underweight",
        Action::Skip => "skip",
    }
}

/// Return the stable persisted label for the selected decision-weight mode.
fn weight_mode_label(mode: DecisionWeightMode) -> &'static str {
    match mode {
        DecisionWeightMode::Normal => "normal",
        DecisionWeightMode::SentimentUnavailable => "sentiment_unavailable",
    }
}

fn summarize_decision(
    execution: &InvestmentPlanExecutionPreview,
    decision: &BuiltinPolicyDecision,
    ack: Option<&BrokerOrderAck>,
) -> String {
    let execution_status = match execution.status {
        ExecutionPreviewStatus::Due => "due",
        ExecutionPreviewStatus::Waiting => "waiting",
        ExecutionPreviewStatus::Inactive => "inactive",
    };
    let contribution = execution
        .planned_contribution
        .map_or_else(|| "none".to_owned(), |value| value.to_string());
    let bucket_split = execution.bucket_split.map_or_else(
        || "none".to_owned(),
        |split| {
            format!(
                "core={} {}, opportunity_budget={} {}, carried_opportunity_cash={} {}, opportunity={} {}, recommended={} {}, unallocated_opportunity={} {}, policy={:?}, requires_approval={}",
                split.core_contribution(),
                execution.currency,
                split.opportunity_budget(),
                execution.currency,
                split.carried_opportunity_cash(),
                execution.currency,
                split.opportunity_contribution(),
                execution.currency,
                split.recommended_contribution(),
                execution.currency,
                split.unallocated_opportunity_contribution(),
                execution.currency,
                split.opportunity_cash_policy(),
                split.requires_approval(),
            )
        },
    );
    let order = match ack.map(BrokerOrderAck::status) {
        Some(BrokerOrderStatus::Accepted) => "paper order accepted",
        Some(BrokerOrderStatus::Duplicate) => "paper order deduplicated",
        None => "no paper order submitted",
    };

    let recommendation = decision.recommendation();
    let Some(decision) = decision.legacy_signal() else {
        return format!(
            "Decision preview for {}: execution={}; planned_contribution={} {}; policy={}; the selected policy produced a bounded opportunity-bucket recommendation without a legacy 70/20 signal payload; multiplier={:.2}; action={}; bucket_split={}; {}.",
            execution.symbol,
            execution_status,
            contribution,
            execution.currency,
            recommendation.policy(),
            recommendation.multiplier().value(),
            action_label(recommendation.action()),
            bucket_split,
            order,
        );
    };
    let fundamental = score_interpretation(decision.fundamental_score.value());
    let trend = score_interpretation(decision.trend_score.value());
    let sentiment = decision.sentiment_score.map_or_else(
        || "unavailable".to_owned(),
        |value| format!("{:.2}", value.value()),
    );
    format!(
        "Decision preview for {}: execution={}; planned_contribution={} {}; fundamental_investability={:.2} ({}); trend_timing={:.2} ({}, regime={:?}); market_sentiment={}; weight_mode={}; final_score={:.2}; multiplier={:.2}; action={}; bucket_split={}; {}.",
        execution.symbol,
        execution_status,
        contribution,
        execution.currency,
        decision.fundamental_score.value(),
        fundamental,
        decision.trend_score.value(),
        trend,
        decision.input.trend.regime,
        sentiment,
        weight_mode_label(decision.weight_mode),
        decision.final_score.value(),
        decision.multiplier.value(),
        action_label(decision.action),
        bucket_split,
        order
    )
}

/// Return a stable, intentionally coarse explanation for a normalized score.
fn score_interpretation(score: f64) -> &'static str {
    if score <= 0.33 {
        "cautious"
    } else if score >= 0.67 {
        "supportive"
    } else {
        "neutral"
    }
}

#[cfg(test)]
mod scheduler_tests {
    use super::*;
    use investment_plans::{
        BucketAllocationRatio, PlanExecutionConfiguration, PlanRiskMode, TwoBucketAllocationConfig,
    };
    use time::OffsetDateTime;

    fn plan(kind: ScheduleKind, days: Vec<i16>) -> InvestmentPlan {
        InvestmentPlan {
            id: Uuid::new_v4(),
            name: "test".to_owned(),
            symbol: "VOO".to_owned(),
            base_contribution: Decimal::new(100, 0),
            currency: "USD".to_owned(),
            schedule_kind: kind,
            schedule_day: days[0],
            schedule_days: days,
            policy: investment_plans::legacy_core_opportunity_v1_policy(),
            execution_configuration: PlanExecutionConfiguration::new(
                TwoBucketAllocationConfig::new(
                    BucketAllocationRatio::new(Decimal::ONE).unwrap(),
                    BucketAllocationRatio::new(Decimal::ZERO).unwrap(),
                )
                .unwrap(),
                PlanRiskMode::Fixed,
            )
            .unwrap(),
            max_single_execution: Decimal::new(100, 0),
            is_active: true,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    /// Verify restart catch-up is bounded to the current monthly period.
    #[test]
    fn monthly_catch_up_only_includes_past_and_current_dates() {
        let dates = due_dates_in_current_period(
            &plan(ScheduleKind::Monthly, vec![1, 15, 25]),
            NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        );
        assert_eq!(
            dates,
            vec![
                NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 8, 15).unwrap(),
            ]
        );
    }

    /// Verify weekly restart catch-up stops at the current ISO weekday.
    #[test]
    fn weekly_catch_up_only_includes_current_week_past_dates() {
        let dates = due_dates_in_current_period(
            &plan(ScheduleKind::Weekly, vec![1, 3, 6]),
            NaiveDate::from_ymd_opt(2026, 8, 19).unwrap(),
        );
        assert_eq!(
            dates,
            vec![
                NaiveDate::from_ymd_opt(2026, 8, 17).unwrap(),
                NaiveDate::from_ymd_opt(2026, 8, 19).unwrap(),
            ]
        );
    }
}
