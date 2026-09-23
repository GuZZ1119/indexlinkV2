use std::{sync::Arc, time::Duration};

use ai_client::{MockAiProvider, NewsItem, NewsSource, NewsSourceError};
use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState};
use indexlink_storage::SqliteStorage;
use serde_json::{json, Value};
use tower::ServiceExt;

struct NoopNews;

#[async_trait]
impl NewsSource for NoopNews {
    async fn fetch(&self) -> Result<Vec<NewsItem>, NewsSourceError> {
        Ok(Vec::new())
    }
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn personal_summary_is_explicit_read_only_and_uses_local_counts() {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    let app = build_router(
        ApiState::new(storage, "0.1.0")
            .with_market_sentiment(Arc::new(NoopNews), Arc::new(MockAiProvider::new())),
    );
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "Local plan",
                        "symbol": "US.SPY",
                        "base_contribution": "1000.00",
                        "currency": "USD",
                        "schedule_kind": "monthly",
                        "schedule_day": 18,
                        "max_single_execution": "1000.00"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/personal/ai-summary")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"profile_id": "mock-default"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["plan_count"], 1);
    assert_eq!(body["decision_count"], 0);
    assert_eq!(body["provider"]["id"], "mock-default");
    assert_eq!(
        body["explanation"]["headline"],
        "Mock read-only explanation"
    );
}
