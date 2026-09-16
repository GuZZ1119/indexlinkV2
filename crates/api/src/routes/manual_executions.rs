//! Append-only user-reported execution journal routes.

use axum::{
    extract::{rejection::PathRejection, Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use decision_records::{
    CreateManualExecutionEvent, DecisionExecutionStatus, ManualExecutionEvent,
    ManualExecutionOutcome,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::{ApiError, ApiState};

/// HTTP-only representation of the closed manual outcome vocabulary.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ManualExecutionOutcomeRequest {
    /// The intended action was completed.
    Executed,
    /// The user intentionally took no action.
    Skipped,
    /// Only part of the intended action was completed.
    Partial,
}

impl From<ManualExecutionOutcomeRequest> for ManualExecutionOutcome {
    fn from(value: ManualExecutionOutcomeRequest) -> Self {
        match value {
            ManualExecutionOutcomeRequest::Executed => Self::Executed,
            ManualExecutionOutcomeRequest::Skipped => Self::Skipped,
            ManualExecutionOutcomeRequest::Partial => Self::Partial,
        }
    }
}

/// JSON body accepted when appending a user-reported event.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateManualExecutionRequest {
    /// Client-generated retry-safe event ID.
    event_id: Uuid,
    /// Closed user-reported outcome.
    outcome: ManualExecutionOutcomeRequest,
    /// Exact decimal string; required except for skipped events.
    actual_amount: Option<String>,
    /// UTC or offset-aware RFC 3339 occurrence time.
    occurred_at: String,
    /// Optional bounded user note.
    note: Option<String>,
}

impl CreateManualExecutionRequest {
    fn into_domain(self, decision_record_id: Uuid) -> Result<CreateManualExecutionEvent, ApiError> {
        let actual_amount = self
            .actual_amount
            .map(|value| value.parse::<Decimal>())
            .transpose()
            .map_err(|_| ApiError::BadRequest)?;
        let occurred_at =
            OffsetDateTime::parse(&self.occurred_at, &Rfc3339).map_err(|_| ApiError::BadRequest)?;
        Ok(CreateManualExecutionEvent {
            id: self.event_id,
            decision_record_id,
            outcome: self.outcome.into(),
            actual_amount,
            note: self.note,
            occurred_at,
        })
    }
}

/// Build read and append routes for a decision's manual execution journal.
pub(crate) fn router() -> Router<ApiState> {
    Router::new().route(
        "/decisions/:id/manual-executions",
        get(list_manual_executions).post(append_manual_execution),
    )
}

/// Append one user-reported event without changing the referenced decision.
async fn append_manual_execution(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    input: Result<Json<CreateManualExecutionRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<ManualExecutionEvent>), ApiError> {
    let Path(decision_record_id) = id.map_err(|_| ApiError::BadRequest)?;
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let decision = state.decision_records().get(decision_record_id).await?;
    if decision.execution_status != DecisionExecutionStatus::Due {
        return Err(ApiError::BadRequest);
    }
    let event = state
        .manual_executions()
        .append(input.into_domain(decision_record_id)?)
        .await?;
    Ok((StatusCode::CREATED, Json(event)))
}

/// List the complete append-only event history for one decision.
async fn list_manual_executions(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
) -> Result<Json<Vec<ManualExecutionEvent>>, ApiError> {
    let Path(decision_record_id) = id.map_err(|_| ApiError::BadRequest)?;
    state.decision_records().get(decision_record_id).await?;
    Ok(Json(
        state
            .manual_executions()
            .list_by_decision(decision_record_id)
            .await?,
    ))
}
