//! Decision record history and approval HTTP routes.

use std::time::Duration;

use ::decision_records::{
    AttachBrokerOrderRequest, CompleteDecisionRecord, DecisionExecutionStatus, DecisionRecord,
    DecisionRecordListQuery,
};
use axum::{
    extract::{
        rejection::{PathRejection, QueryRejection},
        Path, Query, State,
    },
    routing::{get, post},
    Json, Router,
};
use broker::{BrokerEnvironment, BrokerOrderAck, BrokerOrderRequest, BrokerOrderSide};
use chrono::{NaiveDate, Utc};
use investment_plans::{PlanRiskMode, ScheduleKind};
use rust_decimal::{prelude::ToPrimitive, Decimal, RoundingStrategy};
use serde::Deserialize;
use serde_json::Value;
use tokio::time::timeout;
use uuid::Uuid;

use crate::{ApiError, ApiState};

/// Query parameters accepted by the decision-record history route.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DecisionRecordListRequest {
    /// Optional bounded number of records to return.
    limit: Option<u16>,
}

impl DecisionRecordListRequest {
    /// Convert the HTTP query into the validated domain query.
    fn into_domain(self) -> Result<DecisionRecordListQuery, ApiError> {
        self.limit
            .map(DecisionRecordListQuery::new)
            .transpose()
            .map_err(|_| ApiError::BadRequest)?
            .map_or_else(|| Ok(DecisionRecordListQuery::default()), Ok)
    }
}

/// Build decision-record history routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/decisions", get(list_all_decision_records))
        .route(
            "/investment-plans/:id/decisions",
            get(list_decision_records),
        )
        .route("/decisions/:id", get(get_decision_record))
        .route(
            "/decisions/:id/approve-paper-order",
            post(approve_paper_order),
        )
}

/// List the newest decision records across all plans for client-side review filtering.
async fn list_all_decision_records(
    State(state): State<ApiState>,
    query: Result<Query<DecisionRecordListRequest>, QueryRejection>,
) -> Result<Json<Vec<DecisionRecord>>, ApiError> {
    let Query(query) = query.map_err(|_| ApiError::BadRequest)?;
    Ok(Json(
        state.decision_records().list(query.into_domain()?).await?,
    ))
}

/// List the newest persisted decision records for one existing investment plan.
async fn list_decision_records(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    query: Result<Query<DecisionRecordListRequest>, QueryRejection>,
) -> Result<Json<Vec<DecisionRecord>>, ApiError> {
    let Path(plan_id) = id.map_err(|_| ApiError::BadRequest)?;
    let Query(query) = query.map_err(|_| ApiError::BadRequest)?;
    let query = query.into_domain()?;

    state.plans().get(plan_id).await?;
    Ok(Json(
        state
            .decision_records()
            .list_by_plan_with_query(plan_id, query)
            .await?,
    ))
}

/// Fetch one persisted decision record by its ID.
async fn get_decision_record(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
) -> Result<Json<DecisionRecord>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    Ok(Json(state.decision_records().get(id).await?))
}

/// User-confirmed paper-order request for a persisted approval-mode decision.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovePaperOrderRequest {
    /// Stable client-provided key, retained in the non-secret order audit snapshot.
    idempotency_key: String,
}

/// Submit the already-audited recommended amount for one approval-mode decision exactly once.
///
/// This route never re-evaluates signals. It claims the immutable decision record by writing the
/// order intent before the broker call, so a second click cannot submit a different order.
async fn approve_paper_order(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    input: Result<Json<ApprovePaperOrderRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<BrokerOrderAck>, ApiError> {
    let Path(record_id) = id.map_err(|_| ApiError::BadRequest)?;
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let record = state.decision_records().get(record_id).await?;
    let plan = state.plans().get(record.plan_id).await?;
    if record.execution_status != DecisionExecutionStatus::Due
        || plan.execution_configuration.risk_mode() != PlanRiskMode::Approval
        || record.broker_order_request.is_some()
        || record.broker_order_ack.is_some()
        || !record_requires_approval(&record)
    {
        return Err(ApiError::BadRequest);
    }

    let amount = audited_recommended_contribution(&record)?;
    let portfolio = state.paper_portfolio().await?;
    if portfolio.currency != record.currency || amount > portfolio.buying_power {
        return Err(ApiError::BadRequest);
    }
    let price = state.latest_market_price(&record.symbol).await?;
    let quantity = (amount / price).round_dp_with_strategy(0, RoundingStrategy::ToZero);
    if quantity <= Decimal::ZERO || quantity.to_i64().is_none() {
        return Err(ApiError::BadRequest);
    }
    let request = BrokerOrderRequest::market(
        input.idempotency_key,
        &record.symbol,
        BrokerOrderSide::Buy,
        quantity,
        BrokerEnvironment::Paper,
    )
    .map_err(|_| ApiError::BadRequest)?;

    let limit = plan
        .execution_configuration
        .period_execution_limit()
        .unwrap_or_else(|| {
            plan.max_single_execution * Decimal::from(plan.schedule_days.len() as u32)
        });
    if !state
        .reserve_period_execution(
            plan.id,
            record.id,
            &period_key(plan.schedule_kind, Utc::now().date_naive()),
            limit,
            amount,
        )
        .await?
    {
        return Err(ApiError::BadRequest);
    }
    let claimed = state
        .decision_records()
        .attach_broker_order_request(
            record_id,
            AttachBrokerOrderRequest {
                broker_order_request: snapshot(&request)?,
                summary: format!(
                    "Approval confirmed; paper order for {} {} shares is awaiting broker acknowledgement.",
                    request.symbol(),
                    request.quantity()
                ),
            },
        )
        .await?;

    let acknowledgement = timeout(
        Duration::from_secs(5),
        state.broker()?.submit_order(request.clone()),
    )
    .await
    .map_err(|_| ApiError::OrderOutcomeUnknown)?
    .map_err(ApiError::from)?;
    state.accept_period_execution(claimed.id).await?;
    let summary = format!(
        "Approval confirmed; paper order accepted for {} {} shares.",
        request.symbol(),
        request.quantity()
    );
    if let Err(error) = state
        .decision_records()
        .complete_broker_order(
            claimed.id,
            CompleteDecisionRecord {
                broker_order_ack: snapshot(&acknowledgement)?,
                summary,
            },
        )
        .await
    {
        tracing::error!(error = %error, record_id = %claimed.id, "paper order accepted but approval record completion failed");
    }
    state
        .record_accepted_paper_order(plan.id, claimed.id, &acknowledgement, &request)
        .await?;
    Ok(Json(acknowledgement))
}

/// Read the persisted execution proof rather than trusting any new caller-supplied amount.
fn audited_recommended_contribution(record: &DecisionRecord) -> Result<Decimal, ApiError> {
    let execution = record.execution_snapshot.get("execution");
    let value = execution
        .and_then(|value| value.get("bucket_split"))
        .and_then(|value| value.get("recommended_contribution"))
        .and_then(Value::as_str)
        .or(record.planned_contribution.as_deref())
        .ok_or(ApiError::BadRequest)?;
    value
        .parse::<Decimal>()
        .ok()
        .filter(|amount| *amount > Decimal::ZERO)
        .ok_or(ApiError::BadRequest)
}

/// Return whether this immutable preview was deliberately produced for human approval.
fn record_requires_approval(record: &DecisionRecord) -> bool {
    record
        .execution_snapshot
        .get("execution")
        .and_then(|value| value.get("bucket_split"))
        .and_then(|value| value.get("requires_approval"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// Use the same bounded weekly/monthly key shape as automatic execution reservations.
fn period_key(schedule_kind: ScheduleKind, date: NaiveDate) -> String {
    match schedule_kind {
        ScheduleKind::Monthly => date.format("%Y-%m").to_string(),
        ScheduleKind::Weekly => date.format("%G-W%V").to_string(),
    }
}

/// Serialize trusted order data into a non-secret audit snapshot.
fn snapshot(value: &impl serde::Serialize) -> Result<Value, ApiError> {
    serde_json::to_value(value).map_err(|error| {
        tracing::error!(error = %error, "approval audit snapshot serialization failed");
        ApiError::ServiceUnavailable
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    fn approval_record() -> DecisionRecord {
        DecisionRecord {
            id: Uuid::nil(),
            plan_id: Uuid::nil(),
            symbol: "VOO".to_owned(),
            currency: "USD".to_owned(),
            execution_status: DecisionExecutionStatus::Due,
            planned_contribution: Some("100.00".to_owned()),
            execution_snapshot: serde_json::json!({
                "execution": {
                    "bucket_split": {
                        "recommended_contribution": "72.50",
                        "requires_approval": true
                    }
                }
            }),
            fundamental_snapshot: serde_json::json!({"used": true}),
            trend_snapshot: serde_json::json!({"used": true}),
            sentiment_snapshot: None,
            decision_snapshot: serde_json::json!({"action": "standard"}),
            policy_evidence: None,
            broker_order_request: None,
            broker_order_ack: None,
            summary: "approval required".to_owned(),
            created_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    /// Verify approval executes the immutable recommended amount, not a caller supplied amount.
    #[test]
    fn approval_amount_comes_from_persisted_bucket_snapshot() {
        let record = approval_record();
        assert_eq!(
            audited_recommended_contribution(&record).unwrap(),
            Decimal::new(725, 1)
        );
        assert!(record_requires_approval(&record));
    }

    /// Verify records without explicit approval evidence cannot reach the approval route.
    #[test]
    fn approval_requires_explicit_persisted_evidence() {
        let mut record = approval_record();
        record.execution_snapshot = serde_json::json!({"execution": {"bucket_split": {}}});
        assert!(!record_requires_approval(&record));
    }
}
