use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState, ReadinessCheck, ReadinessError};
use market_data::{MarketDataError, MarketPricePoint, MarketSignalInput, MarketSignalProvider};
use serde_json::{json, Value};
use tower::ServiceExt;

/// Ready dependency used by isolated automatic-market-data route tests.
struct Ready;

#[async_trait]
impl ReadinessCheck for Ready {
    /// Always report a ready test dependency.
    async fn check(&self) -> Result<(), ReadinessError> {
        Ok(())
    }
}

/// Deterministic automatic market-data source used without network access.
struct StaticMarketData;

#[async_trait]
impl MarketSignalProvider for StaticMarketData {
    /// Return a complete validated-length input fixture for the selected symbol.
    async fn fetch(&self, symbol: &str) -> Result<MarketSignalInput, MarketDataError> {
        let values = vec![1.0; 60];
        Ok(MarketSignalInput {
            symbol: symbol.to_ascii_uppercase(),
            as_of: "2026-07-17".to_owned(),
            cape_history: values.clone(),
            cape_current: 1.0,
            erp_history: values.clone(),
            erp_current: 1.0,
            ma_distance_history: values.clone(),
            ma_distance_current: 1.0,
            rsi_history: values.clone(),
            rsi_current: 1.0,
            vix_history: values,
            vix_current: 1.0,
            vix_as_of: "2026-07-17".to_owned(),
        })
    }

    /// Return a short deterministic price fixture for the independent chart endpoint.
    async fn fetch_price_history(
        &self,
        _symbol: &str,
        _lookback_days: i64,
    ) -> Result<Vec<MarketPricePoint>, MarketDataError> {
        Ok(vec![MarketPricePoint {
            date: "2026-07-17".to_owned(),
            close: 100.0,
        }])
    }
}

/// Decode an HTTP JSON response body.
async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Verify automatic data is exposed as editable existing signal API fields with source disclosure.
#[tokio::test]
async fn market_input_returns_injected_source_snapshot() {
    let state = ApiState::with_readiness(Arc::new(Ready), "0.1.0")
        .with_market_data(Arc::new(StaticMarketData));
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .uri("/signals/market-input/voo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["symbol"], json!("VOO"));
    assert_eq!(
        body["fundamental"]["cape_history"]
            .as_array()
            .unwrap()
            .len(),
        60
    );
    assert!(body["sources"]["fundamental"]
        .as_str()
        .unwrap()
        .contains("ERP proxy"));
}

/// Verify an absent automatic source does not leak composition details.
#[tokio::test]
async fn market_input_without_provider_is_unavailable() {
    let response = build_router(ApiState::with_readiness(Arc::new(Ready), "0.1.0"))
        .oneshot(
            Request::builder()
                .uri("/signals/market-input/VOO")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response_json(response).await,
        json!({"error": {"code": "service_unavailable", "message": "service is unavailable"}})
    );
}
