use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use decision_records::{CreateDecisionRecord, DecisionExecutionStatus, DecisionRecordRepository};
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState};
use indexlink_storage::{
    SqliteDecisionRecordRepository, SqliteInvestmentPlanRepository, SqliteStorage,
};
use investment_plans::{
    CreateInvestmentPlan, InvestmentPlanRepository, PlanExecutionConfiguration, ScheduleKind,
};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

async fn app_with_decision() -> (axum::Router, Uuid) {
    app_with_decision_status(DecisionExecutionStatus::Due).await
}

async fn app_with_decision_status(
    execution_status: DecisionExecutionStatus,
) -> (axum::Router, Uuid) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .in_memory(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    let storage = SqliteStorage::from_pool(pool);
    storage.migrate().await.unwrap();
    let plans = SqliteInvestmentPlanRepository::new(storage.pool().clone());
    let decisions = SqliteDecisionRecordRepository::new(storage.pool().clone());
    let plan = plans
        .create(CreateInvestmentPlan {
            name: "Manual journal API plan".to_owned(),
            symbol: "VOO".to_owned(),
            base_contribution: Decimal::new(1000, 0),
            currency: "USD".to_owned(),
            schedule_kind: ScheduleKind::Monthly,
            schedule_day: 15,
            schedule_days: vec![15],
            policy: None,
            execution_configuration: PlanExecutionConfiguration::default(),
            max_single_execution: Decimal::new(1500, 0),
        })
        .await
        .unwrap();
    let decision = decisions
        .create(CreateDecisionRecord {
            plan_id: plan.id,
            symbol: plan.symbol,
            currency: plan.currency,
            execution_status,
            planned_contribution: Some("1000.00".to_owned()),
            execution_snapshot: json!({"status": "due"}),
            fundamental_snapshot: json!({"used": false}),
            trend_snapshot: json!({"used": false}),
            sentiment_snapshot: None,
            decision_snapshot: json!({"action": "standard"}),
            policy_evidence: None,
            broker_order_request: None,
            broker_order_ack: None,
            summary: "Invest the scheduled amount.".to_owned(),
        })
        .await
        .unwrap();
    (build_router(ApiState::new(storage, "0.1.0")), decision.id)
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn post_event(app: axum::Router, decision_id: Uuid, body: Value) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri(format!("/decisions/{decision_id}/manual-executions"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn appends_and_lists_user_reported_partial_execution() {
    let (app, decision_id) = app_with_decision().await;
    let event_id = Uuid::from_u128(301);
    let response = post_event(
        app.clone(),
        decision_id,
        json!({
            "event_id": event_id,
            "outcome": "partial",
            "actual_amount": "750.00",
            "occurred_at": "2026-09-16T08:30:00+10:00",
            "note": "  Bought fewer shares than planned.  "
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let created = response_json(response).await;
    assert_eq!(created["id"], json!(event_id));
    assert_eq!(created["decision_record_id"], json!(decision_id));
    assert_eq!(created["outcome"], "partial");
    assert_eq!(created["actual_amount"], "750.00");
    assert_eq!(created["currency"], "USD");
    assert_eq!(created["note"], "Bought fewer shares than planned.");
    assert_eq!(created["occurred_at"], "2026-09-15T22:30:00Z");
    assert_eq!(created["source"], "user_reported");

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/decisions/{decision_id}/manual-executions"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await, json!([created]));
}

#[tokio::test]
async fn validates_outcome_contract_and_rejects_duplicate_event_ids() {
    let (app, decision_id) = app_with_decision().await;
    let event_id = Uuid::from_u128(302);
    let valid = json!({
        "event_id": event_id,
        "outcome": "skipped",
        "occurred_at": "2026-09-16T00:00:00Z"
    });
    assert_eq!(
        post_event(app.clone(), decision_id, valid.clone())
            .await
            .status(),
        StatusCode::CREATED
    );
    let duplicate = post_event(app.clone(), decision_id, valid).await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    assert_eq!(response_json(duplicate).await["error"]["code"], "conflict");

    let skipped_with_amount = json!({
        "event_id": Uuid::from_u128(303),
        "outcome": "skipped",
        "actual_amount": "1.00",
        "occurred_at": "2026-09-16T00:00:00Z"
    });
    assert_eq!(
        post_event(app.clone(), decision_id, skipped_with_amount)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );

    let partial_without_amount = json!({
        "event_id": Uuid::from_u128(304),
        "outcome": "partial",
        "occurred_at": "2026-09-16T00:00:00Z"
    });
    assert_eq!(
        post_event(app.clone(), decision_id, partial_without_amount)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );

    assert_eq!(
        post_event(
            app.clone(),
            Uuid::from_u128(999),
            json!({
                "event_id": Uuid::from_u128(305),
                "outcome": "skipped",
                "occurred_at": "2026-09-16T00:00:00Z"
            }),
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );

    let missing_history = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/decisions/{}/manual-executions",
                    Uuid::from_u128(999)
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_history.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rejects_manual_execution_for_a_decision_that_is_not_due() {
    let (app, decision_id) = app_with_decision_status(DecisionExecutionStatus::Waiting).await;
    let response = post_event(
        app,
        decision_id,
        json!({
            "event_id": Uuid::from_u128(306),
            "outcome": "skipped",
            "occurred_at": "2026-09-16T00:00:00Z"
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
