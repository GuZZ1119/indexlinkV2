//! Explicitly triggered, read-only AI assistance for local product facts.

use ai_client::{
    AiExplanationKind, AiExplanationRequest, AiProviderProfile, AiReadOnlyExplanation,
};
use axum::{extract::State, routing::post, Json, Router};
use decision_records::DecisionRecordListQuery;
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiState};

const PERSONAL_DECISION_LIMIT: u16 = 20;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersonalSummaryRequest {
    profile_id: String,
}

#[derive(Debug, Serialize)]
struct PersonalSummaryResponse {
    provider: AiProviderProfile,
    plan_count: usize,
    decision_count: usize,
    explanation: AiReadOnlyExplanation,
}

/// Build the manually triggered personal-summary route.
pub(crate) fn router() -> Router<ApiState> {
    Router::new().route("/personal/ai-summary", post(personal_summary))
}

async fn personal_summary(
    State(state): State<ApiState>,
    input: Result<Json<PersonalSummaryRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<PersonalSummaryResponse>, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let plans = state.plans().list().await?;
    let decisions = state
        .decision_records()
        .list(
            DecisionRecordListQuery::new(PERSONAL_DECISION_LIMIT)
                .expect("static personal-summary limit is valid"),
        )
        .await?;

    let mut decision_facts = Vec::with_capacity(decisions.len());
    for decision in &decisions {
        let manual = state
            .manual_executions()
            .list_by_decision(decision.id)
            .await?;
        decision_facts.push(serde_json::json!({
            "plan_id": decision.plan_id,
            "symbol": decision.symbol,
            "execution_status": decision.execution_status,
            "planned_contribution": decision.planned_contribution,
            "summary": decision.summary,
            "created_at": decision.created_at,
            "manual_outcome": manual.first().map(|event| event.outcome),
        }));
    }

    let plan_facts = plans
        .iter()
        .map(|plan| {
            serde_json::json!({
                "id": plan.id,
                "name": plan.name,
                "symbol": plan.symbol,
                "currency": plan.currency,
                "base_contribution": plan.base_contribution.to_string(),
                "schedule_kind": plan.schedule_kind,
                "schedule_days": plan.schedule_days,
                "policy": plan.policy,
                "is_active": plan.is_active,
            })
        })
        .collect::<Vec<_>>();
    let facts = serde_json::json!({
        "plans": plan_facts,
        "recent_decisions_newest_first": decision_facts,
        "important": "These are local records. A missing manual_outcome means no user confirmation is stored; it does not prove that no broker action occurred.",
    });
    let request = AiExplanationRequest::new(AiExplanationKind::PersonalSummary, facts)
        .map_err(|_| ApiError::ServiceUnavailable)?;
    let (provider, explanation) = state
        .ai_read_only_explanation(Some(&input.profile_id), &request)
        .await?;
    Ok(Json(PersonalSummaryResponse {
        provider,
        plan_count: plans.len(),
        decision_count: decisions.len(),
        explanation,
    }))
}
