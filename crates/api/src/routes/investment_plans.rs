//! Investment Plan HTTP routes.

use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection},
        Path, State,
    },
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration as ChronoDuration, Utc};
use investment_plans::{
    default_fixed_dca_policy, BucketAllocationRatio, CreateInvestmentPlan, InvestmentPlan,
    InvestmentPlanExecutionPreview, OpportunityCashPolicy, PlanExecutionConfiguration,
    PlanRiskMode, PreviewInvestmentPlanExecution, ScheduleKind, TwoBucketAllocationConfig,
    UpdateInvestmentPlan,
};
use market_data::{HistoricalPriceRequest, Instrument, MarketDataError};
use rust_decimal::Decimal;
use serde::Deserialize;
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};
use uuid::Uuid;

use crate::{ApiError, ApiState};

const MAX_OFFICIAL_HISTORY_STALENESS_DAYS: i64 = 10;

/// 创建 investment plan 的入站 DTO。
#[derive(Debug, Deserialize)]
struct CreateInvestmentPlanRequest {
    /// 用户可读计划名称。
    name: String,
    /// 投资标的代码。
    symbol: String,
    /// 基准定投金额，JSON 中必须是字符串。
    #[serde(with = "rust_decimal::serde::str")]
    base_contribution: Decimal,
    /// 三位币种代码。
    currency: String,
    /// 每月或每周固定定投日。
    schedule_kind: ScheduleKindRequest,
    /// 月度为月内日期，周度为 ISO 星期。
    schedule_day: i16,
    /// 同一周期内的所有固定执行日；缺省时兼容为仅 `schedule_day`。
    #[serde(default)]
    schedule_days: Vec<i16>,
    /// 可选的内置策略版本；省略时新计划默认绑定 `fixed_dca@1`。
    policy: Option<PolicyReferenceRequest>,
    /// 可选核心/机会桶比例；未提供时兼容旧计划，默认全部核心桶。
    bucket_allocation: Option<TwoBucketAllocationRequest>,
    /// 可选风险模式；未提供时兼容旧计划，默认固定模式。
    risk_mode: Option<PlanRiskModeRequest>,
    /// 可选机会桶未使用金额处理策略；未提供时默认当期到期。
    opportunity_cash_policy: Option<OpportunityCashPolicyRequest>,
    /// `carry_with_cap` 的机会现金余额上限，JSON 中必须是字符串。
    #[serde(default, with = "rust_decimal::serde::str_option")]
    opportunity_cash_cap: Option<Decimal>,
    /// 同一周或月内所有订单的累计金额上限，JSON 中必须是字符串。
    #[serde(default, with = "rust_decimal::serde::str_option")]
    period_execution_limit: Option<Decimal>,
    /// 单次执行金额硬上限，JSON 中必须是字符串。
    #[serde(with = "rust_decimal::serde::str")]
    max_single_execution: Decimal,
}

/// 更新 investment plan 的入站 DTO。
#[derive(Debug, Deserialize)]
struct UpdateInvestmentPlanRequest {
    /// 可选的新用户可读计划名称。
    name: Option<String>,
    /// 可选的新基准定投金额，JSON 中必须是字符串。
    #[serde(default, with = "rust_decimal::serde::str_option")]
    base_contribution: Option<Decimal>,
    /// 可选的新每月执行日。
    schedule_day: Option<i16>,
    /// 可选的新固定执行日集合。
    schedule_days: Option<Vec<i16>>,
    /// 可选的新内置策略版本。
    policy: Option<PolicyReferenceRequest>,
    /// 可选的新核心/机会桶比例。
    bucket_allocation: Option<TwoBucketAllocationRequest>,
    /// 可选的新机会桶风险模式。
    risk_mode: Option<PlanRiskModeRequest>,
    /// 可选的新机会桶未使用金额处理策略。
    opportunity_cash_policy: Option<OpportunityCashPolicyRequest>,
    /// 可选的新机会现金余额上限。
    #[serde(default, with = "rust_decimal::serde::str_option")]
    opportunity_cash_cap: Option<Decimal>,
    /// 可选的新周期累计执行金额上限。
    #[serde(default, with = "rust_decimal::serde::str_option")]
    period_execution_limit: Option<Decimal>,
    /// 可选的新单次执行金额硬上限，JSON 中必须是字符串。
    #[serde(default, with = "rust_decimal::serde::str_option")]
    max_single_execution: Option<Decimal>,
    /// 可选启停状态。
    is_active: Option<bool>,
}

/// 执行预览的入站 DTO。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviewInvestmentPlanExecutionRequest {
    /// 本次预览使用的月内日期。
    day_of_month: i16,
    /// 可选 ISO 星期；提供时可预览周度计划。
    iso_weekday: Option<i16>,
}

/// 双桶分配配置的入站 DTO。
#[derive(Debug, Deserialize)]
struct TwoBucketAllocationRequest {
    /// 常规定投桶比例，JSON 中必须是字符串。
    #[serde(with = "rust_decimal::serde::str")]
    core_ratio: Decimal,
    /// 机会桶比例，JSON 中必须是字符串。
    #[serde(with = "rust_decimal::serde::str")]
    opportunity_ratio: Decimal,
}

/// 策略版本引用的 HTTP 表示。
#[derive(Debug, Deserialize)]
struct PolicyReferenceRequest {
    /// 稳定策略标识，例如 `fixed_dca`。
    id: String,
    /// 不可变策略版本，必须大于零。
    version: u32,
}

/// Explicit user-confirmed activation of one immutable validated strategy version.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivatePolicyRequest {
    /// Strategy version selected by the user from the Strategy Studio.
    policy: PolicyReferenceRequest,
}

impl PolicyReferenceRequest {
    /// 转换为已校验的领域策略引用。
    fn into_domain(self) -> Result<PolicyRef, ApiError> {
        Ok(PolicyRef::new(
            PolicyId::new(self.id).map_err(|_| ApiError::BadRequest)?,
            PolicyVersion::new(self.version).map_err(|_| ApiError::BadRequest)?,
        ))
    }
}

/// API 边界支持的 schedule kind。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ScheduleKindRequest {
    /// 每月固定日期触发。
    Monthly,
    /// 每周固定 ISO 星期触发；本 PR 暂只保存配置。
    Weekly,
}

impl From<ScheduleKindRequest> for ScheduleKind {
    /// Convert the API schedule value into the domain schedule kind.
    fn from(value: ScheduleKindRequest) -> Self {
        match value {
            ScheduleKindRequest::Monthly => Self::Monthly,
            ScheduleKindRequest::Weekly => Self::Weekly,
        }
    }
}

/// API 边界支持的机会桶风险模式。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PlanRiskModeRequest {
    /// 仅核心桶的固定定投模式。
    Fixed,
    /// 后续由策略链路自动决定机会桶是否执行。
    Autopilot,
    /// 后续由用户确认机会桶执行。
    Approval,
}

/// API 边界支持的机会桶未使用金额处理策略。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OpportunityCashPolicyRequest {
    /// 当期未使用机会预算不自动补投。
    ExpireEachPeriod,
    /// 当期未使用机会预算待后续账本阶段滚存。
    CarryForward,
    /// 未使用机会预算滚存，但余额不超过用户配置的金额上限。
    CarryWithCap,
}

impl From<OpportunityCashPolicyRequest> for OpportunityCashPolicy {
    /// Convert the API cash-policy value into the domain policy.
    fn from(value: OpportunityCashPolicyRequest) -> Self {
        match value {
            OpportunityCashPolicyRequest::ExpireEachPeriod => Self::ExpireEachPeriod,
            OpportunityCashPolicyRequest::CarryForward => Self::CarryForward,
            OpportunityCashPolicyRequest::CarryWithCap => Self::CarryWithCap,
        }
    }
}

impl From<PlanRiskModeRequest> for PlanRiskMode {
    /// Convert the API risk-mode value into the domain risk mode.
    fn from(value: PlanRiskModeRequest) -> Self {
        match value {
            PlanRiskModeRequest::Fixed => Self::Fixed,
            PlanRiskModeRequest::Autopilot => Self::Autopilot,
            PlanRiskModeRequest::Approval => Self::Approval,
        }
    }
}

impl CreateInvestmentPlanRequest {
    /// Convert the API DTO into validated domain input with legacy-safe defaults.
    fn into_domain(self) -> Result<CreateInvestmentPlan, ApiError> {
        let Self {
            name,
            symbol,
            base_contribution,
            currency,
            schedule_kind,
            schedule_day,
            schedule_days,
            policy,
            bucket_allocation,
            risk_mode,
            opportunity_cash_policy,
            opportunity_cash_cap,
            period_execution_limit,
            max_single_execution,
        } = self;
        let execution_configuration = execution_configuration_from_request(
            bucket_allocation,
            risk_mode,
            opportunity_cash_policy,
            opportunity_cash_cap,
            period_execution_limit,
            PlanExecutionConfiguration::default(),
        )?;
        Ok(CreateInvestmentPlan {
            name,
            symbol,
            base_contribution,
            currency,
            schedule_kind: schedule_kind.into(),
            schedule_day,
            schedule_days: if schedule_days.is_empty() {
                vec![schedule_day]
            } else {
                schedule_days
            },
            policy: policy
                .map(PolicyReferenceRequest::into_domain)
                .transpose()?,
            max_single_execution,
            execution_configuration,
        })
    }
}

impl UpdateInvestmentPlanRequest {
    /// Convert the API update DTO into a domain partial update.
    fn into_domain(self) -> Result<UpdateInvestmentPlan, ApiError> {
        let Self {
            name,
            base_contribution,
            schedule_day,
            schedule_days,
            policy,
            bucket_allocation,
            risk_mode,
            opportunity_cash_policy,
            opportunity_cash_cap,
            period_execution_limit,
            max_single_execution,
            is_active,
        } = self;
        if bucket_allocation.is_some() != risk_mode.is_some() {
            return Err(ApiError::BadRequest);
        }
        let bucket_allocation = bucket_allocation
            .map(TwoBucketAllocationRequest::into_domain)
            .transpose()?;
        Ok(UpdateInvestmentPlan {
            name,
            base_contribution,
            schedule_day,
            schedule_days,
            policy: policy
                .map(PolicyReferenceRequest::into_domain)
                .transpose()?,
            bucket_allocation,
            risk_mode: risk_mode.map(Into::into),
            opportunity_cash_policy: opportunity_cash_policy.map(Into::into),
            opportunity_cash_cap,
            period_execution_limit,
            max_single_execution,
            is_active,
        })
    }
}

/// Combine optional HTTP bucket settings with a legacy-safe default configuration.
fn execution_configuration_from_request(
    bucket_allocation: Option<TwoBucketAllocationRequest>,
    risk_mode: Option<PlanRiskModeRequest>,
    opportunity_cash_policy: Option<OpportunityCashPolicyRequest>,
    opportunity_cash_cap: Option<Decimal>,
    period_execution_limit: Option<Decimal>,
    default: PlanExecutionConfiguration,
) -> Result<PlanExecutionConfiguration, ApiError> {
    match (
        bucket_allocation,
        risk_mode,
        opportunity_cash_policy,
        opportunity_cash_cap,
        period_execution_limit,
    ) {
        (None, None, None, None, None) => Ok(default),
        (
            Some(bucket_allocation),
            Some(risk_mode),
            opportunity_cash_policy,
            opportunity_cash_cap,
            period_execution_limit,
        ) => PlanExecutionConfiguration::new_with_limits(
            bucket_allocation.into_domain()?,
            risk_mode.into(),
            opportunity_cash_policy
                .map(Into::into)
                .unwrap_or(OpportunityCashPolicy::ExpireEachPeriod),
            opportunity_cash_cap,
            period_execution_limit,
        )
        .map_err(Into::into),
        _ => Err(ApiError::BadRequest),
    }
}

impl PreviewInvestmentPlanExecutionRequest {
    /// Convert the API preview DTO into validated domain inputs.
    fn into_domain(self) -> Result<PreviewInvestmentPlanExecution, ApiError> {
        match self.iso_weekday {
            Some(weekday) => PreviewInvestmentPlanExecution::for_date(self.day_of_month, weekday),
            None => PreviewInvestmentPlanExecution::new(self.day_of_month),
        }
        .map_err(Into::into)
    }
}

impl TwoBucketAllocationRequest {
    /// Convert API ratio strings into a validated domain bucket config.
    fn into_domain(self) -> Result<TwoBucketAllocationConfig, ApiError> {
        TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(self.core_ratio)?,
            BucketAllocationRatio::new(self.opportunity_ratio)?,
        )
        .map_err(Into::into)
    }
}

/// 构建 investment plan routes。
pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/investment-plans", post(create_plan).get(list_plans))
        .route(
            "/investment-plans/:id",
            get(get_plan).patch(update_plan).delete(delete_plan),
        )
        .route(
            "/investment-plans/:id/execution-preview",
            post(preview_plan_execution),
        )
        .route(
            "/investment-plans/:id/activate-policy",
            post(activate_policy),
        )
}

/// 创建 investment plan。
async fn create_plan(
    State(state): State<ApiState>,
    input: Result<Json<CreateInvestmentPlanRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<InvestmentPlan>), ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let mut input = input.into_domain()?.normalize()?;
    let policy = input
        .policy
        .clone()
        .unwrap_or_else(default_fixed_dca_policy);
    if crate::official_strategies::is_catalog_policy(&policy) {
        let instrument =
            validate_and_normalize_official_instrument(&mut input.symbol, &mut input.currency)?;
        validate_official_formula_history(&state, &policy, &instrument).await?;
    }
    if !state
        .is_plan_policy_eligible_for_activation(&policy)
        .await?
    {
        return Err(ApiError::BadRequest);
    }
    Ok((
        StatusCode::CREATED,
        Json(state.plans().create(input).await?),
    ))
}

/// 列出 investment plans。
async fn list_plans(State(state): State<ApiState>) -> Result<Json<Vec<InvestmentPlan>>, ApiError> {
    Ok(Json(state.plans().list().await?))
}

/// 按 ID 获取 investment plan。
async fn get_plan(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
) -> Result<Json<InvestmentPlan>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    Ok(Json(state.plans().get(id).await?))
}

/// 更新 investment plan。
async fn update_plan(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    input: Result<Json<UpdateInvestmentPlanRequest>, JsonRejection>,
) -> Result<Json<InvestmentPlan>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let input = input.into_domain()?;
    if let Some(policy) = &input.policy {
        let plan = state.plans().get(id).await?;
        if crate::official_strategies::is_catalog_policy(policy) {
            let instrument = validate_official_instrument(&plan.symbol, &plan.currency)?;
            validate_official_formula_history(&state, policy, &instrument).await?;
        }
        if !state.is_plan_policy_eligible_for_activation(policy).await? {
            return Err(ApiError::BadRequest);
        }
    }
    Ok(Json(state.plans().update(id, input).await?))
}

/// Bind a user-confirmed immutable built-in or validated DSL policy version to one plan.
async fn activate_policy(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    input: Result<Json<ActivatePolicyRequest>, JsonRejection>,
) -> Result<Json<InvestmentPlan>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let policy = input.policy.into_domain()?;
    let plan = state.plans().get(id).await?;
    if crate::official_strategies::is_catalog_policy(&policy) {
        let instrument = validate_official_instrument(&plan.symbol, &plan.currency)?;
        validate_official_formula_history(&state, &policy, &instrument).await?;
    }
    if !state
        .is_plan_policy_eligible_for_activation(&policy)
        .await?
    {
        return Err(ApiError::BadRequest);
    }
    Ok(Json(
        state
            .plans()
            .update(
                id,
                UpdateInvestmentPlan {
                    policy: Some(policy),
                    ..Default::default()
                },
            )
            .await?,
    ))
}

/// Validate one official-plan instrument and normalize its storage representation.
fn validate_and_normalize_official_instrument(
    symbol: &mut String,
    currency: &mut String,
) -> Result<Instrument, ApiError> {
    let raw_symbol = symbol.trim().to_ascii_uppercase();
    let instrument = validate_official_instrument(&raw_symbol, currency)?;
    *symbol = if instrument.market() == market_data::Market::Us && !raw_symbol.starts_with("US.") {
        instrument.symbol().to_owned()
    } else {
        instrument.qualified_symbol()
    };
    *currency = instrument.currency().to_owned();
    Ok(instrument)
}

/// Reject malformed symbols and client-supplied currencies that disagree with market metadata.
fn validate_official_instrument(symbol: &str, currency: &str) -> Result<Instrument, ApiError> {
    let instrument = Instrument::parse(symbol).map_err(map_plan_market_request_error)?;
    if currency.trim().to_ascii_uppercase() != instrument.currency() {
        return Err(ApiError::BadRequest);
    }
    Ok(instrument)
}

/// Fail closed before binding an official Formula version to a plan without usable history.
async fn validate_official_formula_history(
    state: &ApiState,
    policy: &PolicyRef,
    instrument: &Instrument,
) -> Result<(), ApiError> {
    let Some(strategy) = crate::official_strategies::strategy(policy)? else {
        return Ok(());
    };
    let required_closes = strategy.required_close_observations();
    if required_closes == 0 {
        return Ok(());
    }
    let lookback_days = i64::try_from(required_closes.saturating_mul(2).max(30))
        .map_err(|_| ApiError::ServiceUnavailable)?;
    let request_end = Utc::now().date_naive();
    let request_start = request_end
        .checked_sub_signed(ChronoDuration::days(lookback_days))
        .ok_or(ApiError::ServiceUnavailable)?;
    let provider = state.historical_price_provider()?;
    let adjustment = provider
        .preferred_adjustment(instrument.market())
        .map_err(map_plan_market_provider_error)?;
    let request =
        HistoricalPriceRequest::new(instrument.clone(), request_start, request_end, adjustment)
            .map_err(map_plan_market_request_error)?;
    let dataset = provider
        .fetch_history(&request)
        .await
        .map_err(map_plan_market_provider_error)?;
    if dataset.instrument() != instrument
        || dataset.adjustment() != adjustment
        || dataset.requested_start() != request_start
        || dataset.requested_end() != request_end
    {
        return Err(ApiError::ServiceUnavailable);
    }
    if dataset.bars().len() < required_closes {
        return Err(ApiError::BadRequest);
    }
    let latest_close = dataset
        .bars()
        .last()
        .map(|bar| bar.date())
        .ok_or(ApiError::BadRequest)?;
    if request_end.signed_duration_since(latest_close).num_days()
        > MAX_OFFICIAL_HISTORY_STALENESS_DAYS
    {
        return Err(ApiError::BadRequest);
    }
    Ok(())
}

fn map_plan_market_request_error(error: MarketDataError) -> ApiError {
    match error {
        MarketDataError::InvalidSymbol
        | MarketDataError::InvalidRange
        | MarketDataError::UnsupportedRequest => ApiError::BadRequest,
        _ => ApiError::ServiceUnavailable,
    }
}

fn map_plan_market_provider_error(error: MarketDataError) -> ApiError {
    match error {
        MarketDataError::InvalidSymbol
        | MarketDataError::InvalidRange
        | MarketDataError::UnsupportedRequest
        | MarketDataError::InsufficientHistory => ApiError::BadRequest,
        _ => ApiError::ServiceUnavailable,
    }
}

/// 删除一个定投标的及其本地关联记录。
async fn delete_plan(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    state.plans().delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// 预览 investment plan 在指定日期的执行状态。
async fn preview_plan_execution(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    input: Result<Json<PreviewInvestmentPlanExecutionRequest>, JsonRejection>,
) -> Result<Json<InvestmentPlanExecutionPreview>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let input = input.into_domain()?;
    let preview = state.plans().preview_execution(id, input).await?;

    Ok(Json(preview))
}
