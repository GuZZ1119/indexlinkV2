use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{
        header::{ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_REQUEST_METHOD, ORIGIN},
        HeaderValue, Request, StatusCode,
    },
};
use http_body_util::BodyExt;
use indexlink_api::{
    build_router, build_router_with_cors, ApiState, ReadinessCheck, ReadinessError,
};
use serde_json::{json, Value};
use tower::ServiceExt;

struct FakeReadiness {
    available: bool,
}

struct PanicReadiness;

#[async_trait]
impl ReadinessCheck for PanicReadiness {
    async fn check(&self) -> Result<(), ReadinessError> {
        panic!("health endpoint must not call readiness checker")
    }
}

#[async_trait]
impl ReadinessCheck for FakeReadiness {
    async fn check(&self) -> Result<(), ReadinessError> {
        if self.available {
            Ok(())
        } else {
            Err(ReadinessError::new(
                "postgres://secret:password@internal/database: connection refused",
            ))
        }
    }
}

fn app(available: bool) -> axum::Router {
    build_router(ApiState::with_readiness(
        Arc::new(FakeReadiness { available }),
        "0.1.0",
    ))
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("response body should be readable")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("response should be valid JSON")
}

#[tokio::test]
async fn health_returns_ok_with_expected_json() {
    let response = app(true)
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({
            "status": "ok",
            "service": "indexlink-server",
            "version": "0.1.0"
        })
    );
}

#[tokio::test]
async fn health_does_not_call_failing_readiness_dependency() {
    let app = build_router(ApiState::with_readiness(Arc::new(PanicReadiness), "0.1.0"));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn health_reports_version_supplied_by_application_state() {
    let app = build_router(ApiState::with_readiness(
        Arc::new(PanicReadiness),
        "2026.06-test",
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response_json(response).await["version"],
        json!("2026.06-test")
    );
}

#[tokio::test]
async fn ready_returns_ok_when_database_is_available() {
    let response = app(true)
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"status": "ready", "database": "ok"})
    );
}

#[tokio::test]
async fn ready_hides_internal_error_when_database_is_unavailable() {
    let response = app(false)
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response_json(response).await;
    assert_eq!(
        body,
        json!({
            "error": {
                "code": "service_unavailable",
                "message": "service is unavailable"
            }
        })
    );
    let serialized = body.to_string();
    assert!(!serialized.contains("secret"));
    assert!(!serialized.contains("internal"));
    assert!(!serialized.contains("password"));
}

#[tokio::test]
async fn runtime_status_distinguishes_ready_database_from_optional_unconfigured_dependencies() {
    let response = app(true)
        .oneshot(
            Request::builder()
                .uri("/runtime-status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({
            "service": "running",
            "database": "ready",
            "market_data": "not_configured",
            "qwen": "not_configured",
            "ai_provider_profiles": [],
            "paper_broker": "not_configured",
            "scheduler": {
                "enabled": false,
                "tick_interval_seconds": 0,
                "last_tick_at": null,
                "last_summary": null,
                "last_error_at": null
            }
        })
    );
}

#[tokio::test]
async fn configured_cors_origin_is_returned_for_preflight_request() {
    let app = build_router_with_cors(
        ApiState::with_readiness(Arc::new(FakeReadiness { available: true }), "0.1.0"),
        vec![HeaderValue::from_static("https://app.example")],
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/health")
                .header(ORIGIN, "https://app.example")
                .header(ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&HeaderValue::from_static("https://app.example"))
    );
}

#[tokio::test]
async fn unconfigured_cors_origin_is_not_reflected() {
    let app = build_router_with_cors(
        ApiState::with_readiness(Arc::new(FakeReadiness { available: true }), "0.1.0"),
        vec![HeaderValue::from_static("https://app.example")],
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/health")
                .header(ORIGIN, "https://attacker.example")
                .header(ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response
        .headers()
        .get(ACCESS_CONTROL_ALLOW_ORIGIN)
        .is_none());
}

#[tokio::test]
async fn same_origin_router_does_not_grant_cross_origin_access() {
    let response = app(true)
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/health")
                .header(ORIGIN, "https://app.example")
                .header(ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response
        .headers()
        .get(ACCESS_CONTROL_ALLOW_ORIGIN)
        .is_none());
}
