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
async fn frontend_ai_configuration_is_process_memory_only_and_clearable() {
    let app = build_router(ApiState::with_readiness(Arc::new(Ready), "0.1.0"));
    let configured = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ai/session-provider")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "provider": "gpt",
                        "model": "gpt-test",
                        "api_key": "private-test-key"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(configured.status(), StatusCode::OK);
    let configured_body = response_json(configured).await;
    assert_eq!(configured_body["provider"]["id"], "session-gpt");
    assert_eq!(configured_body["provider"]["model"], "gpt-test");
    assert_eq!(configured_body["storage"], "process_memory");
    assert!(!configured_body.to_string().contains("private-test-key"));

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/ai/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    assert_eq!(
        response_json(listed).await["providers"][0]["id"],
        "session-gpt"
    );

    let cleared = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/ai/session-provider")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cleared.status(), StatusCode::NO_CONTENT);

    let empty = app
        .oneshot(
            Request::builder()
                .uri("/ai/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response_json(empty).await["providers"], json!([]));
}

#[tokio::test]
async fn frontend_ai_configuration_rejects_blank_credentials() {
    let response = build_router(ApiState::with_readiness(Arc::new(Ready), "0.1.0"))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ai/session-provider")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"provider": "claude", "model": "claude-test", "api_key": " "})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn provider_probe_requires_an_explicit_process_memory_connection() {
    let response = build_router(ApiState::with_readiness(Arc::new(Ready), "0.1.0"))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ai/session-provider/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn frontend_can_select_qwen_cloud_without_exposing_its_key() {
    let response = build_router(ApiState::with_readiness(Arc::new(Ready), "0.1.0"))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ai/session-provider")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "provider": "qwen_cloud",
                        "model": "qwen3.8-max",
                        "api_key": "unit-test-private-key"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["provider"]["id"], "session-qwen-cloud");
    assert_eq!(body["provider"]["provider"], "qwen-cloud");
    assert_eq!(body["provider"]["display_name"], "QwenCloud（本次运行）");
    assert_eq!(body["provider"]["model"], "qwen3.8-max");
    assert!(!body.to_string().contains("unit-test-private-key"));
}
