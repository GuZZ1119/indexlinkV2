use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState, ReadinessCheck, ReadinessError};
use serde_json::{json, Value};
use tower::ServiceExt;

struct Ready;

#[async_trait]
impl ReadinessCheck for Ready {
    async fn check(&self) -> Result<(), ReadinessError> {
        Ok(())
    }
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn frontend_opend_configuration_is_loopback_read_only_and_clearable() {
    let app = build_router(ApiState::with_readiness(Arc::new(Ready), "0.1.0"));
    let configured = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/market-data/session-opend")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"host": "127.0.0.1", "port": 11111}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(configured.status(), StatusCode::OK);
    let body = response_json(configured).await;
    assert_eq!(body["provider"], "opend");
    assert_eq!(body["access"], "read_only_market_data");
    assert_eq!(body["storage"], "process_memory");

    let runtime = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/runtime-status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let runtime = response_json(runtime).await;
    assert_eq!(runtime["market_data"], "configured");
    assert_eq!(runtime["historical_prices"], "configured");

    let cleared = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/market-data/session-opend")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cleared.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn frontend_opend_configuration_rejects_non_loopback_hosts() {
    let response = build_router(ApiState::with_readiness(Arc::new(Ready), "0.1.0"))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/market-data/session-opend")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"host": "192.0.2.10", "port": 11111}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
