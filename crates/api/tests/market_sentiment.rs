use std::sync::Arc;

use ai_client::{
    AiClientError, AiProvider, NewsItem, NewsSource, NewsSourceError, Sentiment, SentimentAnalysis,
};
use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState, ReadinessCheck, ReadinessError};
use serde_json::{json, Value};
use tower::ServiceExt;

/// Ready dependency used by isolated route tests.
struct Ready;

#[async_trait]
impl ReadinessCheck for Ready {
    /// Always report the test dependency as ready.
    async fn check(&self) -> Result<(), ReadinessError> {
        Ok(())
    }
}

/// Deterministic news source for HTTP route tests.
struct StaticNews;

#[async_trait]
impl NewsSource for StaticNews {
    /// Return one representative news item without network access.
    async fn fetch(&self) -> Result<Vec<NewsItem>, NewsSourceError> {
        Ok(vec![NewsItem {
            title: "Markets rise on improving inflation data".to_owned(),
            description: "A compact deterministic test item.".to_owned(),
            url: "https://example.com/inflation".to_owned(),
            pub_date: Utc::now(),
        }])
    }
}

/// AI provider that returns a fixed positive signal.
struct PositiveAi;

#[async_trait]
impl AiProvider for PositiveAi {
    /// Return a bounded sentiment without network access.
    async fn analyze(&self, _prompt: &str) -> Result<Sentiment, AiClientError> {
        Ok(Sentiment::new(0.4).expect("constant sentiment is in range"))
    }

    /// Return deterministic explanation fields for the HTTP contract.
    async fn analyze_with_evidence(
        &self,
        _prompt: &str,
    ) -> Result<SentimentAnalysis, AiClientError> {
        SentimentAnalysis::new(
            Sentiment::new(0.4).expect("constant sentiment is in range"),
            "Cooling inflation supports risk appetite.".to_owned(),
            vec!["Single headlines can be noisy.".to_owned()],
        )
        .map_err(|_| AiClientError::ParseFailure)
    }
}

/// AI provider that simulates a private provider failure.
struct FailingAi;

#[async_trait]
impl AiProvider for FailingAi {
    /// Return a provider error whose internal details must not reach HTTP clients.
    async fn analyze(&self, _prompt: &str) -> Result<Sentiment, AiClientError> {
        Err(AiClientError::EmptyResponse)
    }
}

/// Build an app with an optional, fully injected market-sentiment pipeline.
fn app(provider: Option<Arc<dyn AiProvider>>) -> axum::Router {
    let state = ApiState::with_readiness(Arc::new(Ready), "0.1.0");
    let state = match provider {
        Some(provider) => state.with_market_sentiment(Arc::new(StaticNews), provider),
        None => state,
    };
    build_router(state)
}

/// Decode an HTTP JSON response body.
async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Verify a fake AI provider is injected and produces the documented response.
#[tokio::test]
async fn preview_returns_sentiment_from_injected_provider() {
    let response = app(Some(Arc::new(PositiveAi)))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/market-sentiment/preview?profile_id=external-default")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["score"], json!(0.4));
    assert_eq!(body["provider"]["id"], json!("external-default"));
    assert_eq!(body["provider"]["provider"], json!("external"));
    assert_eq!(
        body["provider"]["capabilities"]["market_evidence"],
        json!(true)
    );
    assert_eq!(body["label"], json!("positive"));
    assert_eq!(
        body["rationale"],
        json!("Cooling inflation supports risk appetite.")
    );
    assert_eq!(body["warnings"], json!(["Single headlines can be noisy."]));
    assert_eq!(
        body["headlines"][0]["title"],
        json!("Markets rise on improving inflation data")
    );
    assert_eq!(
        body["headlines"][0]["url"],
        json!("https://example.com/inflation")
    );
}

/// Verify the registry exposes only deployed credential-free profiles.
#[tokio::test]
async fn lists_only_deployed_ai_profiles() {
    let response = app(Some(Arc::new(PositiveAi)))
        .oneshot(
            Request::builder()
                .uri("/ai/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["providers"].as_array().unwrap().len(), 1);
    assert_eq!(body["providers"][0]["id"], json!("external-default"));
    assert!(body.to_string().contains("External AI provider"));
    assert!(!body.to_string().contains("secret"));
}

/// Verify an unknown selected profile is rejected before the evidence pipeline runs.
#[tokio::test]
async fn preview_rejects_an_unknown_ai_profile() {
    let response = app(Some(Arc::new(PositiveAi)))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/market-sentiment/preview?profile_id=not-deployed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Verify a missing provider follows the standard unavailable JSON contract.
#[tokio::test]
async fn preview_without_configured_provider_is_unavailable() {
    let response = app(None)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/market-sentiment/preview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response_json(response).await,
        json!({
            "error": {
                "code": "service_unavailable",
                "message": "service is unavailable"
            }
        })
    );
}

/// Verify provider errors are mapped without exposing provider internals.
#[tokio::test]
async fn preview_provider_error_uses_safe_unavailable_response() {
    let response = app(Some(Arc::new(FailingAi)))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/market-sentiment/preview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response_json(response).await;
    assert_eq!(body["error"]["code"], json!("service_unavailable"));
    assert_eq!(body["error"]["message"], json!("service is unavailable"));
    assert!(!body.to_string().contains("EmptyResponse"));
}
