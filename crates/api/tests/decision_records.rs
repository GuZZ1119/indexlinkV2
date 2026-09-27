use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use broker::MockBroker;
use decision_records::{
    CreateDecisionRecord, DecisionExecutionStatus, DecisionRecord, DecisionRecordListQuery,
    DecisionRecordRepository, DecisionRecordRepositoryError, DecisionRecordService,
};
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState, ReadinessCheck, ReadinessError};
use investment_plans::{
    CreateInvestmentPlan, InvestmentPlan, InvestmentPlanRepository, InvestmentPlanService,
    PlanRepositoryError, ScheduleKind, UpdateInvestmentPlan,
};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

/// Readiness stub used by decision-record history route tests.
struct Ready;

#[async_trait]
impl ReadinessCheck for Ready {
    /// Always report dependencies as available.
    async fn check(&self) -> Result<(), ReadinessError> {
        Ok(())
    }
}

/// In-memory plan repository that exposes exactly one existing plan.
struct PlanRepository {
    plan: InvestmentPlan,
}

#[async_trait]
impl InvestmentPlanRepository for PlanRepository {
    /// Reject creates because this fake only serves read checks for history routes.
    async fn create(
        &self,
        _input: CreateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        Err(PlanRepositoryError::Unavailable)
    }

    /// Reject list queries because this fake only serves read checks for history routes.
    async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanRepositoryError> {
        Err(PlanRepositoryError::Unavailable)
    }

    /// Return the known plan or report that the requested plan does not exist.
    async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanRepositoryError> {
        (id == self.plan.id)
            .then(|| self.plan.clone())
            .ok_or(PlanRepositoryError::NotFound)
    }

    /// Reject updates because this fake only serves read checks for history routes.
    async fn update(
        &self,
        _id: Uuid,
        _input: UpdateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        Err(PlanRepositoryError::Unavailable)
    }

    /// Reject activation changes because this fake only serves read checks for history routes.
    async fn set_active(
        &self,
        _id: Uuid,
        _is_active: bool,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        Err(PlanRepositoryError::Unavailable)
    }
}

/// In-memory decision-record repository with optional unavailability behavior.
struct RecordRepository {
    records: Mutex<Vec<DecisionRecord>>,
    unavailable: bool,
}

impl RecordRepository {
    /// Build an available repository from persisted record snapshots.
    fn available(records: Vec<DecisionRecord>) -> Self {
        Self {
            records: Mutex::new(records),
            unavailable: false,
        }
    }

    /// Build a repository that fails all reads as an unavailable dependency.
    fn unavailable() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
            unavailable: true,
        }
    }
}

#[async_trait]
impl DecisionRecordRepository for RecordRepository {
    /// Reject creates because decision-record writes are outside this query-route test scope.
    async fn create(
        &self,
        _input: CreateDecisionRecord,
    ) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }

    /// Reject completions because this query-route fake only supports reads.
    async fn complete_broker_order(
        &self,
        _id: Uuid,
        _input: decision_records::CompleteDecisionRecord,
    ) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }

    /// Return newest matching records, bounded by the validated domain query.
    async fn list_by_plan(
        &self,
        plan_id: Uuid,
        query: DecisionRecordListQuery,
    ) -> Result<Vec<DecisionRecord>, DecisionRecordRepositoryError> {
        if self.unavailable {
            return Err(DecisionRecordRepositoryError::Unavailable);
        }

        let mut records = self
            .records
            .lock()
            .unwrap()
            .iter()
            .filter(|record| record.plan_id == plan_id)
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        records.truncate(usize::from(query.limit()));
        Ok(records)
    }

    /// Return one record by ID or report that it does not exist.
    async fn get(&self, id: Uuid) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        if self.unavailable {
            return Err(DecisionRecordRepositoryError::Unavailable);
        }

        self.records
            .lock()
            .unwrap()
            .iter()
            .find(|record| record.id == id)
            .cloned()
            .ok_or(DecisionRecordRepositoryError::NotFound)
    }
}

/// Build a router with one persisted plan and a replaceable record repository.
fn app(plan: InvestmentPlan, records: Arc<RecordRepository>) -> axum::Router {
    build_router(ApiState::with_readiness_plans_broker_and_decision_records(
        Arc::new(Ready),
        InvestmentPlanService::new(Arc::new(PlanRepository { plan })),
        Arc::new(MockBroker::paper_only()),
        DecisionRecordService::new(records),
        "0.1.0",
    ))
}

/// Parse an HTTP response body as JSON.
async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Build a persisted investment plan used by the history route's existence check.
fn plan(id: Uuid) -> InvestmentPlan {
    let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
    InvestmentPlan {
        id,
        name: "Core ETF".to_owned(),
        symbol: "VOO".to_owned(),
        base_contribution: Decimal::new(1000, 0),
        currency: "USD".to_owned(),
        schedule_kind: ScheduleKind::Monthly,
        schedule_day: 15,
        schedule_days: vec![15],
        timezone: investment_plans::PlanTimeZone::utc(),
        policy: investment_plans::legacy_core_opportunity_v1_policy(),
        execution_configuration: investment_plans::PlanExecutionConfiguration::default(),
        max_single_execution: Decimal::new(1500, 0),
        is_active: true,
        created_at: now,
        updated_at: now,
    }
}

/// Build one audit-ready decision record for a plan and creation time.
fn decision_record(id: Uuid, plan_id: Uuid, created_at: i64) -> DecisionRecord {
    DecisionRecord {
        id,
        plan_id,
        symbol: "VOO".to_owned(),
        currency: "USD".to_owned(),
        execution_status: DecisionExecutionStatus::Due,
        planned_contribution: Some("1000.00".to_owned()),
        execution_snapshot: json!({"status": "due"}),
        fundamental_snapshot: json!({"score": 0.1}),
        trend_snapshot: json!({"score": 0.5}),
        sentiment_snapshot: Some(json!({"score": 0.2})),
        decision_snapshot: json!({"action": "standard"}),
        policy_evidence: None,
        broker_order_request: None,
        broker_order_ack: None,
        summary: "Decision preview completed.".to_owned(),
        created_at: OffsetDateTime::from_unix_timestamp(created_at).unwrap(),
    }
}

/// Verify plan history is newest-first, excludes other plans, and honors its limit.
#[tokio::test]
async fn list_history_returns_bounded_newest_records_for_existing_plan() {
    let plan_id = Uuid::from_u128(1);
    let app = app(
        plan(plan_id),
        Arc::new(RecordRepository::available(vec![
            decision_record(Uuid::from_u128(1), plan_id, 1_700_000_000),
            decision_record(Uuid::from_u128(2), plan_id, 1_700_000_100),
            decision_record(Uuid::from_u128(3), Uuid::from_u128(2), 1_700_000_200),
        ])),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/investment-plans/{plan_id}/decisions?limit=1"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!([{
            "id": Uuid::from_u128(2),
            "plan_id": plan_id,
            "symbol": "VOO",
            "currency": "USD",
            "execution_status": "due",
            "planned_contribution": "1000.00",
            "execution_snapshot": {"status": "due"},
            "fundamental_snapshot": {"score": 0.1},
            "trend_snapshot": {"score": 0.5},
            "sentiment_snapshot": {"score": 0.2},
            "decision_snapshot": {"action": "standard"},
            "summary": "Decision preview completed.",
            "created_at": "2023-11-14T22:15:00Z"
        }])
    );
}

/// Verify the individual history endpoint returns the persisted audit snapshot.
#[tokio::test]
async fn get_history_returns_one_record_by_id() {
    let plan_id = Uuid::from_u128(1);
    let record = decision_record(Uuid::from_u128(2), plan_id, 1_700_000_100);
    let app = app(
        plan(plan_id),
        Arc::new(RecordRepository::available(vec![record.clone()])),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/decisions/{}", record.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["id"], json!(record.id));
    assert_eq!(body, json!(record));
}

/// Verify malformed paths, invalid limits, absent resources, and unavailable storage use safe errors.
#[tokio::test]
async fn history_routes_map_invalid_and_unavailable_paths_to_safe_errors() {
    let plan_id = Uuid::from_u128(1);
    let available_app = app(
        plan(plan_id),
        Arc::new(RecordRepository::available(Vec::new())),
    );

    for uri in [
        "/investment-plans/not-a-uuid/decisions".to_owned(),
        format!("/investment-plans/{plan_id}/decisions?limit=0"),
        format!("/investment-plans/{plan_id}/decisions?limit=201"),
        format!("/investment-plans/{plan_id}/decisions?unknown=true"),
    ] {
        let response = available_app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response_json(response).await,
            json!({"error": {"code": "bad_request", "message": "invalid request"}})
        );
    }

    let missing_plan = available_app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/investment-plans/{}/decisions",
                    Uuid::from_u128(2)
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_plan.status(), StatusCode::NOT_FOUND);

    let missing_record = available_app
        .oneshot(
            Request::builder()
                .uri(format!("/decisions/{}", Uuid::from_u128(99)))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_record.status(), StatusCode::NOT_FOUND);

    let unavailable_app = app(plan(plan_id), Arc::new(RecordRepository::unavailable()));
    let unavailable = unavailable_app
        .oneshot(
            Request::builder()
                .uri(format!("/investment-plans/{plan_id}/decisions"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unavailable.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response_json(unavailable).await,
        json!({
            "error": {
                "code": "service_unavailable",
                "message": "service is unavailable"
            }
        })
    );
}
