//! Read-only HTTP routes for persisted restricted DSL strategy versions.

use ai_client::{AiCopilotDraftRequest, AiCopilotEvidenceReference, AiProviderProfile};
use axum::{
    extract::{rejection::PathRejection, Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{Datelike, NaiveDate};
use indexlink_storage::StoredStrategySpec;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use strategy_dsl::{
    DslEvidence, StrategySpecDocument, TechnicalClose, TechnicalMarketSnapshot, TechnicalVix,
};
use strategy_policy::DecisionContext;
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};
use time::Date;

use crate::{ApiError, ApiState};

/// A safe validation result for one form-authored restricted DSL strategy.
#[derive(Debug, Serialize)]
struct StrategyValidationResponse {
    /// Whether the submitted document rebuilt into a validated immutable strategy.
    valid: bool,
    /// Human-readable validation failure without transport, database, or credential details.
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    /// Canonical validated document, returned only when validation succeeds.
    #[serde(skip_serializing_if = "Option::is_none")]
    document: Option<StrategySpecDocument>,
}

/// Current-data simulation output that explains the first matched rule without creating an audit or order.
#[derive(Debug, Serialize)]
struct StrategySimulationResponse {
    /// Immutable version that was interpreted.
    policy: PolicyRef,
    /// Market data cutoff used for this pure simulation.
    as_of: String,
    /// First matching rule index, or `null` when default opportunity behaviour applies.
    matched_rule_index: Option<usize>,
    /// Opportunity-only runtime action selected by the strategy.
    action: String,
    /// Bounded opportunity multiplier selected by the strategy.
    multiplier: f64,
    /// Stable, source-labelled values read by the condition evaluator.
    evidence: Vec<StrategyEvidenceValue>,
}

/// One readable metric used by a current-data strategy simulation.
#[derive(Debug, Serialize)]
struct StrategyEvidenceValue {
    /// Whitelisted indicator and calculation window.
    indicator: String,
    /// Decimal value calculated at `as_of`.
    value: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StrategySimulationRequest {
    /// US symbol whose current local OpenD history should be interpreted.
    symbol: String,
}

/// Read-only request for one server-generated restricted DSL candidate.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CopilotDraftRequest {
    /// Optional deployed AI profile ID; unknown or unsupported profiles are rejected safely.
    profile_id: Option<String>,
    /// Caller-selected immutable custom policy identifier, which must start with `dsl_`.
    policy_id: String,
    /// Caller-selected positive policy version to repeat in the candidate document.
    policy_version: u32,
    /// Bounded natural-language goal used only to draft a reviewable opportunity-bucket rule.
    objective: String,
}

/// One trustworthy evidence reference selected by the model from the server-supplied closed list.
#[derive(Debug, Serialize)]
struct CopilotEvidenceReferenceResponse {
    /// Stable server-supplied reference identifier.
    id: String,
    /// Display-safe explanation of what the reference represents.
    label: String,
}

/// Read-only validated Copilot candidate; it is not persisted, activated, or executable.
#[derive(Debug, Serialize)]
struct CopilotDraftResponse {
    /// Credential-free profile that generated the candidate.
    provider: AiProviderProfile,
    /// Canonical, domain-validated restricted DSL document.
    document: StrategySpecDocument,
    /// Short model explanation, never a trading instruction.
    explanation: String,
    /// Bounded risks and limitations accompanying the candidate.
    warnings: Vec<String>,
    /// Only references selected from the trusted server-supplied evidence set.
    evidence: Vec<CopilotEvidenceReferenceResponse>,
}

/// Build restricted strategy discovery, validation, and immutable-save routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/strategies", get(list_strategies).post(create_strategy))
        .route("/strategies/validate", post(validate_strategy))
        .route("/strategies/copilot-draft", post(generate_copilot_draft))
        .route(
            "/strategies/:policy_id/:policy_version/simulate",
            post(simulate_strategy),
        )
        .route(
            "/strategies/:policy_id/:policy_version/admission",
            get(strategy_admission),
        )
        .route("/strategies/:policy_id/:policy_version", get(get_strategy))
}

/// Generate one validated, read-only restricted DSL candidate without storage or execution.
async fn generate_copilot_draft(
    State(state): State<ApiState>,
    Json(request): Json<CopilotDraftRequest>,
) -> Result<Json<CopilotDraftResponse>, ApiError> {
    let policy_id = PolicyId::new(request.policy_id.clone()).map_err(|_| ApiError::BadRequest)?;
    let policy_version =
        PolicyVersion::new(request.policy_version).map_err(|_| ApiError::BadRequest)?;
    if !policy_id.as_str().starts_with("dsl_") {
        return Err(ApiError::BadRequest);
    }
    let evidence = copilot_evidence_references(&request.objective)?;
    let provider_request = AiCopilotDraftRequest::new(
        policy_id.as_str().to_owned(),
        policy_version.value(),
        request.objective,
        evidence.clone(),
    )
    .map_err(|_| ApiError::BadRequest)?;
    let (provider, draft) = state
        .ai_policy_draft(request.profile_id.as_deref(), &provider_request)
        .await?;

    let document = serde_json::from_value::<StrategySpecDocument>(draft.document().clone())
        .map_err(|_| ApiError::ServiceUnavailable)?;
    if document.policy_id != provider_request.policy_id()
        || document.policy_version != provider_request.policy_version()
    {
        return Err(ApiError::ServiceUnavailable);
    }
    let strategy = document
        .into_strategy_spec()
        .map_err(|_| ApiError::ServiceUnavailable)?;
    let document = StrategySpecDocument::from_strategy_spec(&strategy);
    let selected_evidence = draft
        .evidence_reference_ids()
        .iter()
        .map(|id| {
            evidence
                .iter()
                .find(|reference| reference.id() == id)
                .map(|reference| CopilotEvidenceReferenceResponse {
                    id: reference.id().to_owned(),
                    label: reference.label().to_owned(),
                })
                .ok_or(ApiError::ServiceUnavailable)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(CopilotDraftResponse {
        provider,
        document,
        explanation: draft.explanation().to_owned(),
        warnings: draft.warnings().to_vec(),
        evidence: selected_evidence,
    }))
}

/// Build the closed, provider-visible evidence list for one draft request.
fn copilot_evidence_references(
    objective: &str,
) -> Result<Vec<AiCopilotEvidenceReference>, ApiError> {
    [
        (
            "operator_objective",
            format!("Operator objective: {objective}"),
        ),
        (
            "dsl_allowlist_v1",
            "Server-enforced DSL allowlist: indicators and opportunity-bucket actions only."
                .to_owned(),
        ),
        (
            "review_workflow_v1",
            "Drafts require deterministic validation, fixed-sample admission, and explicit user save/activation."
                .to_owned(),
        ),
    ]
    .into_iter()
    .map(|(id, label)| {
        AiCopilotEvidenceReference::new(id.to_owned(), label).map_err(|_| ApiError::BadRequest)
    })
    .collect()
}

/// Simulate one stored version on current provider data without persisting or submitting anything.
async fn simulate_strategy(
    State(state): State<ApiState>,
    path: Result<Path<(String, u32)>, PathRejection>,
    Json(request): Json<StrategySimulationRequest>,
) -> Result<Json<StrategySimulationResponse>, ApiError> {
    let policy = policy_from_path(path)?;
    let strategy = state
        .get_strategy_spec(&policy)
        .await?
        .document
        .into_strategy_spec()
        .map_err(|_| ApiError::ServiceUnavailable)?;
    if strategy.has_fixed_opportunity_amount_action() {
        return Err(ApiError::BadRequest);
    }
    let input = state.market_signal_input(&request.symbol).await?;
    let as_of = api_date(&input.as_of)?;
    let closes = technical_closes(state.market_price_history(&request.symbol, 366).await?)?;
    let vix = Decimal::from_f64_retain(input.vix_current).ok_or(ApiError::ServiceUnavailable)?;
    let snapshot = TechnicalMarketSnapshot::new(
        as_of,
        closes,
        TechnicalVix::new(api_date(&input.vix_as_of)?, vix)
            .map_err(|_| ApiError::ServiceUnavailable)?,
    )
    .map_err(|_| ApiError::ServiceUnavailable)?;
    let evidence = DslEvidence::from_as_of_market_snapshot(&strategy, &snapshot)
        .map_err(|_| ApiError::BadRequest)?;
    let context = DecisionContext::new(as_of, Decimal::ONE, evidence.clone())
        .map_err(|_| ApiError::BadRequest)?;
    let result = strategy
        .evaluate(&context)
        .map_err(|_| ApiError::BadRequest)?;
    Ok(Json(StrategySimulationResponse {
        policy,
        as_of: input.as_of,
        matched_rule_index: result.matched_rule_index(),
        action: format!("{:?}", result.action()),
        multiplier: result.recommendation().multiplier().value(),
        evidence: evidence
            .values()
            .map(|(indicator, value)| StrategyEvidenceValue {
                indicator: format!("{indicator:?}"),
                value: value.to_string(),
            })
            .collect(),
    }))
}

/// Convert one ISO market-data cutoff to the shared causal evidence date.
fn api_date(value: &str) -> Result<Date, ApiError> {
    let date =
        NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| ApiError::ServiceUnavailable)?;
    Date::from_calendar_date(
        date.year(),
        time::Month::try_from(date.month() as u8).map_err(|_| ApiError::ServiceUnavailable)?,
        date.day() as u8,
    )
    .map_err(|_| ApiError::ServiceUnavailable)
}

/// Convert trusted provider prices into the DSL's date-bounded technical observations.
fn technical_closes(
    prices: Vec<market_data::MarketPricePoint>,
) -> Result<Vec<TechnicalClose>, ApiError> {
    prices
        .into_iter()
        .map(|point| {
            let date = api_date(&point.date)?;
            let close =
                Decimal::from_f64_retain(point.close).ok_or(ApiError::ServiceUnavailable)?;
            TechnicalClose::new(date, close).map_err(|_| ApiError::ServiceUnavailable)
        })
        .collect()
}

fn policy_from_path(
    path: Result<Path<(String, u32)>, PathRejection>,
) -> Result<PolicyRef, ApiError> {
    let Path((policy_id, policy_version)) = path.map_err(|_| ApiError::BadRequest)?;
    Ok(PolicyRef::new(
        PolicyId::new(policy_id).map_err(|_| ApiError::BadRequest)?,
        PolicyVersion::new(policy_version).map_err(|_| ApiError::BadRequest)?,
    ))
}

/// Validate one form-authored restricted DSL document without persisting it.
async fn validate_strategy(
    Json(document): Json<StrategySpecDocument>,
) -> Json<StrategyValidationResponse> {
    match document.into_strategy_spec() {
        Ok(strategy) => Json(StrategyValidationResponse {
            valid: true,
            error: None,
            document: Some(StrategySpecDocument::from_strategy_spec(&strategy)),
        }),
        Err(error) => Json(StrategyValidationResponse {
            valid: false,
            error: Some(error.to_string()),
            document: None,
        }),
    }
}

/// Persist one new immutable validated DSL strategy version.
async fn create_strategy(
    State(state): State<ApiState>,
    Json(document): Json<StrategySpecDocument>,
) -> Result<(StatusCode, Json<StoredStrategySpec>), ApiError> {
    let strategy = document
        .into_strategy_spec()
        .map_err(|_| ApiError::BadRequest)?;
    Ok((
        StatusCode::CREATED,
        Json(state.save_strategy_spec(&strategy).await?),
    ))
}

/// List all immutable persisted DSL strategy versions.
async fn list_strategies(
    State(state): State<ApiState>,
) -> Result<Json<Vec<StoredStrategySpec>>, ApiError> {
    Ok(Json(state.list_strategy_specs().await?))
}

/// Fetch one immutable persisted DSL strategy version by its policy reference.
async fn get_strategy(
    State(state): State<ApiState>,
    path: Result<Path<(String, u32)>, PathRejection>,
) -> Result<Json<StoredStrategySpec>, ApiError> {
    let policy = policy_from_path(path)?;
    Ok(Json(state.get_strategy_spec(&policy).await?))
}

/// Evaluate one stored DSL version against the committed fixed sample before activation.
///
/// This route never changes the selected policy or submits an order. Its comparison uses the
/// same contribution schedule, execution timing, and costs for the candidate and Fixed DCA.
async fn strategy_admission(
    State(state): State<ApiState>,
    path: Result<Path<(String, u32)>, PathRejection>,
) -> Result<Json<strategy_evaluation::StrategyAdmissionReport>, ApiError> {
    let policy = policy_from_path(path)?;
    Ok(Json(state.strategy_admission_report(&policy).await?))
}
