use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState, ReadinessCheck, ReadinessError};
use investment_plans::{
    BucketAllocationRatio, CreateInvestmentPlan, InvestmentPlan, InvestmentPlanRepository,
    InvestmentPlanService, OpportunityCashPolicy, PlanExecutionConfiguration, PlanRepositoryError,
    PlanRiskMode, ScheduleKind, TwoBucketAllocationConfig, UpdateInvestmentPlan,
};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

/// Readiness stub used by investment plan route tests.
struct Ready;

#[async_trait]
impl ReadinessCheck for Ready {
    /// Always report dependencies as available.
    async fn check(&self) -> Result<(), ReadinessError> {
        Ok(())
    }
}

/// In-memory repository fake for exercising HTTP routes through the service.
#[derive(Default)]
struct FakeRepository {
    plans: Mutex<Vec<InvestmentPlan>>,
}

#[async_trait]
impl InvestmentPlanRepository for FakeRepository {
    /// Store the normalized create input as a persisted plan.
    async fn create(
        &self,
        input: CreateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let mut plans = self.plans.lock().unwrap();
        let plan = plan_from(Uuid::from_u128((plans.len() + 1) as u128), input);
        plans.push(plan.clone());
        Ok(plan)
    }

    /// Return a snapshot of stored plans.
    async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanRepositoryError> {
        Ok(self.plans.lock().unwrap().clone())
    }

    /// Return one stored plan by ID.
    async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanRepositoryError> {
        self.plans
            .lock()
            .unwrap()
            .iter()
            .find(|plan| plan.id == id)
            .cloned()
            .ok_or(PlanRepositoryError::NotFound)
    }

    /// Merge and store updates through the repository port.
    async fn update(
        &self,
        id: Uuid,
        input: UpdateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let mut plans = self.plans.lock().unwrap();
        let plan = plans
            .iter_mut()
            .find(|plan| plan.id == id)
            .ok_or(PlanRepositoryError::NotFound)?;
        let base = input.base_contribution.unwrap_or(plan.base_contribution);
        let max = input
            .max_single_execution
            .unwrap_or(plan.max_single_execution);
        if max < base {
            return Err(PlanRepositoryError::Validation(
                investment_plans::PlanValidationError::MaxBelowBaseContribution,
            ));
        }

        if let Some(name) = input.name {
            plan.name = name;
        }
        if let Some(base_contribution) = input.base_contribution {
            plan.base_contribution = base_contribution;
        }
        if let Some(schedule_day) = input.schedule_day {
            plan.schedule_day = schedule_day;
        }
        if let Some(policy) = input.policy {
            plan.policy = policy;
        }
        if let Some(max_single_execution) = input.max_single_execution {
            plan.max_single_execution = max_single_execution;
        }
        if let Some(is_active) = input.is_active {
            plan.is_active = is_active;
        }
        plan.updated_at = OffsetDateTime::from_unix_timestamp(1_700_000_100).unwrap();

        Ok(plan.clone())
    }

    /// Active-state toggles are outside this PR's route scope.
    async fn set_active(
        &self,
        _id: Uuid,
        _is_active: bool,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        Err(PlanRepositoryError::Unavailable)
    }

    /// Delete one stored plan for the HTTP deletion contract.
    async fn delete(&self, id: Uuid) -> Result<(), PlanRepositoryError> {
        let mut plans = self.plans.lock().unwrap();
        let index = plans
            .iter()
            .position(|plan| plan.id == id)
            .ok_or(PlanRepositoryError::NotFound)?;
        plans.remove(index);
        Ok(())
    }
}

/// Convert service input into a stored test plan.
fn plan_from(id: Uuid, input: CreateInvestmentPlan) -> InvestmentPlan {
    let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
    InvestmentPlan {
        id,
        name: input.name,
        symbol: input.symbol,
        base_contribution: input.base_contribution,
        currency: input.currency,
        schedule_kind: input.schedule_kind,
        schedule_day: input.schedule_day,
        schedule_days: input.schedule_days,
        timezone: input.timezone,
        policy: input
            .policy
            .unwrap_or_else(investment_plans::default_fixed_dca_policy),
        execution_configuration: input.execution_configuration,
        max_single_execution: input.max_single_execution,
        is_active: true,
        created_at: now,
        updated_at: now,
    }
}

/// Build an API app wired to the investment plan fake repository.
fn app(repository: Arc<FakeRepository>) -> axum::Router {
    build_router(ApiState::with_readiness_and_plans(
        Arc::new(Ready),
        InvestmentPlanService::new(repository),
        "0.1.0",
    ))
}

/// Parse an HTTP response body as JSON.
async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Build a normalized domain input for seeding the fake repository.
fn create_input() -> CreateInvestmentPlan {
    CreateInvestmentPlan {
        name: "Core ETF".to_owned(),
        symbol: "VOO".to_owned(),
        base_contribution: Decimal::new(1000, 0),
        currency: "USD".to_owned(),
        schedule_kind: ScheduleKind::Monthly,
        schedule_day: 15,
        schedule_days: vec![15],
        timezone: investment_plans::PlanTimeZone::utc(),
        policy: None,
        execution_configuration: PlanExecutionConfiguration::new_with_cash_policy(
            TwoBucketAllocationConfig::new(
                BucketAllocationRatio::new(Decimal::new(80, 2)).unwrap(),
                BucketAllocationRatio::new(Decimal::new(20, 2)).unwrap(),
            )
            .unwrap(),
            PlanRiskMode::Autopilot,
            OpportunityCashPolicy::ExpireEachPeriod,
        )
        .unwrap(),
        max_single_execution: Decimal::new(1500, 0),
    }
}

/// Verify create route uses DTO conversion and returns string-encoded money.
#[tokio::test]
async fn create_plan_returns_normalized_plan_json() {
    let app = app(Arc::new(FakeRepository::default()));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "  Core ETF  ",
                        "symbol": " voo ",
                        "base_contribution": "1000.00",
                        "currency": " usd ",
                        "schedule_kind": "monthly",
                        "schedule_day": 15,
                        "timezone": "Australia/Sydney",
                        "max_single_execution": "1500.00"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    assert_eq!(body["name"], json!("Core ETF"));
    assert_eq!(body["symbol"], json!("VOO"));
    assert!(body["base_contribution"].is_string());
    assert_eq!(body["execution_configuration"]["risk_mode"], json!("fixed"));
    assert_eq!(body["policy"]["id"], json!("fixed_dca"));
    assert_eq!(body["policy"]["version"], json!(1));
    assert_eq!(body["timezone"], json!("Australia/Sydney"));
}

/// Verify arbitrary labels and UTC offsets cannot bypass the IANA timezone boundary.
#[tokio::test]
async fn create_plan_rejects_invalid_timezone() {
    let response = app(Arc::new(FakeRepository::default()))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "Invalid timezone",
                        "symbol": "VOO",
                        "base_contribution": "100.00",
                        "currency": "USD",
                        "schedule_kind": "monthly",
                        "schedule_day": 15,
                        "timezone": "GMT+8 maybe",
                        "max_single_execution": "100.00"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Verify API accepts a persisted weekly plan with an explicit approval-mode bucket split.
#[tokio::test]
async fn create_plan_accepts_weekly_approval_bucket_configuration() {
    let response = app(Arc::new(FakeRepository::default()))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "Weekly VOO",
                        "symbol": "VOO",
                        "base_contribution": "100.00",
                        "currency": "USD",
                        "schedule_kind": "weekly",
                        "schedule_day": 3,
                        "bucket_allocation": {
                            "core_ratio": "0.70",
                            "opportunity_ratio": "0.30"
                        },
                        "risk_mode": "approval",
                        "opportunity_cash_policy": "carry_forward",
                        "max_single_execution": "150.00"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    assert_eq!(body["schedule_kind"], json!("weekly"));
    assert_eq!(body["schedule_day"], json!(3));
    assert_eq!(
        body["execution_configuration"]["bucket_allocation"]["core_ratio"],
        json!("0.70")
    );
    assert_eq!(
        body["execution_configuration"]["risk_mode"],
        json!("approval")
    );
    assert_eq!(
        body["execution_configuration"]["opportunity_cash_policy"],
        json!("carry_forward")
    );
}

/// Verify API rejects incomplete or contradictory bucket authorization choices.
#[tokio::test]
async fn create_plan_rejects_incomplete_bucket_authorization() {
    let response = app(Arc::new(FakeRepository::default()))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "Incomplete config",
                        "symbol": "VOO",
                        "base_contribution": "100.00",
                        "currency": "USD",
                        "schedule_kind": "monthly",
                        "schedule_day": 15,
                        "bucket_allocation": {
                            "core_ratio": "0.80",
                            "opportunity_ratio": "0.20"
                        },
                        "max_single_execution": "150.00"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Verify JSON extractor failures return the shared error envelope.
#[tokio::test]
async fn invalid_create_json_returns_safe_bad_request() {
    let response = app(Arc::new(FakeRepository::default()))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"base_contribution": 1000.0}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(response).await,
        json!({"error": {"code": "bad_request", "message": "invalid request"}})
    );
}

/// Verify list/get routes read plans and map bad IDs safely.
#[tokio::test]
async fn list_get_and_bad_id_routes_use_service() {
    let repository = Arc::new(FakeRepository::default());
    let created = repository.create(create_input()).await.unwrap();
    let app = app(repository);

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/investment-plans")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response_json(list).await[0]["id"], json!(created.id));

    let get = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/investment-plans/{}", created.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response_json(get).await["name"], json!("Core ETF"));

    let missing = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/investment-plans/{}", Uuid::from_u128(99)))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response_json(missing).await,
        json!({"error": {"code": "not_found", "message": "resource not found"}})
    );

    let bad_id = app
        .oneshot(
            Request::builder()
                .uri("/investment-plans/not-a-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_id.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(bad_id).await,
        json!({"error": {"code": "bad_request", "message": "invalid request"}})
    );
}

/// Verify update route uses DTO conversion and safe rejection mapping.
#[tokio::test]
async fn update_plan_merges_fields_and_maps_bad_input() {
    let repository = Arc::new(FakeRepository::default());
    let created = repository.create(create_input()).await.unwrap();
    let app = app(repository);

    let updated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/investment-plans/{}", created.id))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "  Core ETF Plus  ",
                        "base_contribution": "1200.00",
                        "schedule_day": 20,
                        "is_active": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(updated.status(), StatusCode::OK);
    let body = response_json(updated).await;
    assert_eq!(body["name"], json!("Core ETF Plus"));
    assert_eq!(body["base_contribution"], json!("1200.00"));
    assert_eq!(body["schedule_day"], json!(20));
    assert_eq!(body["is_active"], json!(false));

    let invalid_amounts = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/investment-plans/{}", created.id))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"max_single_execution": "1000.00"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid_amounts.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(invalid_amounts).await,
        json!({"error": {"code": "bad_request", "message": "invalid request"}})
    );

    let empty_patch = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/investment-plans/{}", created.id))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(empty_patch.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(empty_patch).await,
        json!({"error": {"code": "bad_request", "message": "invalid request"}})
    );

    let bad_id = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/investment-plans/not-a-uuid")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"name": "Core"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_id.status(), StatusCode::BAD_REQUEST);

    let bad_json = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/investment-plans/{}", created.id))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"base_contribution": 1200.0}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_json.status(), StatusCode::BAD_REQUEST);
}

/// Verify execution preview route exposes due bucket splits through HTTP.
#[tokio::test]
async fn preview_execution_returns_bucket_split_when_due() {
    let repository = Arc::new(FakeRepository::default());
    let created = repository.create(create_input()).await.unwrap();
    let app = app(repository);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/investment-plans/{}/execution-preview",
                    created.id
                ))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"day_of_month": 15}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["plan_id"], json!(created.id));
    assert_eq!(body["status"], json!("due"));
    assert_eq!(body["planned_contribution"], json!("1000"));
    assert_eq!(body["bucket_split"]["planned_contribution"], json!("1000"));
    assert_eq!(body["bucket_split"]["core_contribution"], json!("800.00"));
    assert_eq!(
        body["bucket_split"]["opportunity_contribution"],
        json!("200.00")
    );
}

/// Verify execution preview omits bucket split outside execution day.
#[tokio::test]
async fn preview_execution_omits_bucket_split_when_waiting() {
    let repository = Arc::new(FakeRepository::default());
    let created = repository.create(create_input()).await.unwrap();
    let app = app(repository);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/investment-plans/{}/execution-preview",
                    created.id
                ))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"day_of_month": 16}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["status"], json!("waiting"));
    assert!(body.get("planned_contribution").is_none());
    assert!(body.get("bucket_split").is_none());
}

/// Verify execution preview rejects malformed IDs, JSON, days, and bucket ratios.
#[tokio::test]
async fn preview_execution_maps_bad_input_to_safe_bad_request() {
    let repository = Arc::new(FakeRepository::default());
    let created = repository.create(create_input()).await.unwrap();
    let app = app(repository);

    for (uri, body) in [
        (
            "/investment-plans/not-a-uuid/execution-preview".to_owned(),
            json!({"day_of_month": 15}).to_string(),
        ),
        (
            format!("/investment-plans/{}/execution-preview", created.id),
            json!({"day_of_month": 32}).to_string(),
        ),
        (
            format!("/investment-plans/{}/execution-preview", created.id),
            json!({
                "day_of_month": 15,
                "bucket_allocation": {
                    "core_ratio": "0.80",
                    "opportunity_ratio": "0.30"
                }
            })
            .to_string(),
        ),
        (
            format!("/investment-plans/{}/execution-preview", created.id),
            json!({
                "day_of_month": 15,
                "bucket_allocation": {
                    "core_ratio": "-0.20",
                    "opportunity_ratio": "1.20"
                }
            })
            .to_string(),
        ),
        (
            format!("/investment-plans/{}/execution-preview", created.id),
            json!({"day_of_month": "15"}).to_string(),
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response_json(response).await,
            json!({"error": {"code": "bad_request", "message": "invalid request"}})
        );
    }
}

/// Verify deleting a holding removes it from subsequent list responses.
#[tokio::test]
async fn delete_plan_returns_no_content_and_removes_holding() {
    let repository = Arc::new(FakeRepository::default());
    let created = repository.create(create_input()).await.unwrap();
    let app = app(repository);

    let deleted = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/investment-plans/{}", created.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let listed = app
        .oneshot(
            Request::builder()
                .uri("/investment-plans")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response_json(listed).await, json!([]));
}
