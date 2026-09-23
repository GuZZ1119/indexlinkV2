//! Local paper-performance routes.

use axum::{
    extract::{rejection::PathRejection, Path, State},
    routing::{get, put},
    Json, Router,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::{ApiError, ApiState};

/// Build local paper-performance routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/paper-performance/actual", get(read_actual_performance))
        .route(
            "/investment-plans/:id/paper-performance",
            get(read_performance),
        )
        .route(
            "/investment-plans/:id/paper-performance/opening-balance",
            put(set_opening_balance),
        )
}

/// Refresh all active holdings from one read-only paper-account snapshot and return their lines.
async fn read_actual_performance(
    State(state): State<ApiState>,
) -> Result<Json<crate::state::ActualPerformance>, ApiError> {
    Ok(Json(state.actual_performance().await?))
}

/// User-confirmed opening balance used as the local return calculation baseline.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpeningBalanceRequest {
    /// Positive or zero virtual-account opening balance.
    #[serde(with = "rust_decimal::serde::str")]
    amount: Decimal,
    /// UTC RFC3339 timestamp with millisecond precision and a `Z` suffix.
    occurred_at: String,
}

/// Refresh the local ledger from read-only OpenD state and return chart points.
async fn read_performance(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
) -> Result<Json<indexlink_storage::PaperPerformance>, ApiError> {
    let Path(plan_id) = id.map_err(|_| ApiError::BadRequest)?;
    Ok(Json(state.paper_performance(plan_id).await?))
}

/// Store the user-confirmed local starting balance without contacting the broker.
async fn set_opening_balance(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    input: Result<Json<OpeningBalanceRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<(), ApiError> {
    let Path(plan_id) = id.map_err(|_| ApiError::BadRequest)?;
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    state
        .set_paper_opening_balance(plan_id, input.amount, &input.occurred_at)
        .await
}
