use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState};
use indexlink_storage::SqliteStorage;
use serde_json::Value;
use tower::ServiceExt;

async fn app() -> axum::Router {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    build_router(ApiState::new(storage, "0.1.0"))
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn consumer_catalog_exposes_only_adoptable_versioned_non_ai_strategies() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/strategy-catalog")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let entries = body.as_array().unwrap();
    let ids = entries
        .iter()
        .map(|entry| entry["policy"]["id"].as_str().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            "fixed_dca",
            "dsl_ma200_trend_guard",
            "dsl_growth_volatility_balance"
        ]
    );
    assert!(!body.to_string().contains("core_opportunity_v1"));
    assert!(entries.iter().all(|entry| entry["adoptable"] == true));
    assert_eq!(entries[0]["research_status"], "reference");
    for entry in &entries[1..] {
        assert_eq!(entry["research_status"], "available");
        assert_eq!(entry["default_plan"]["core_ratio"], "0.7");
        assert_eq!(entry["default_plan"]["opportunity_ratio"], "0.3");
        assert_eq!(entry["research"]["eligible"], true);
        assert_eq!(entry["research"]["assets"].as_array().unwrap().len(), 2);
        assert!(entry["formula"]["rules"].as_array().is_some());
    }
}

#[tokio::test]
async fn official_formula_version_is_readable_but_cannot_be_overwritten() {
    let app = app().await;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/strategies/dsl_ma200_trend_guard/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let official = response_json(response).await;
    assert_eq!(official["document"]["policy_id"], "dsl_ma200_trend_guard");

    let overwrite = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/strategies")
                .header("content-type", "application/json")
                .body(Body::from(official["document"].to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(overwrite.status(), StatusCode::CONFLICT);
}
