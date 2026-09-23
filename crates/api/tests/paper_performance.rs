//! HTTP coverage for the local paper-performance ledger routes.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use broker::{
    BrokerClient, BrokerError, BrokerOrderAck, BrokerOrderRequest, PaperPortfolioSnapshot,
};
use indexlink_api::{build_router_with_cors, ApiState};
use rust_decimal::Decimal;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tower::ServiceExt;

/// Read-only broker fixture with an empty, internally consistent paper account.
struct EmptyPaperBroker;

#[async_trait]
impl BrokerClient for EmptyPaperBroker {
    async fn submit_order(
        &self,
        _request: BrokerOrderRequest,
    ) -> Result<BrokerOrderAck, BrokerError> {
        Err(BrokerError::Unavailable)
    }

    async fn read_paper_portfolio(&self) -> Result<PaperPortfolioSnapshot, BrokerError> {
        Ok(PaperPortfolioSnapshot {
            currency: "USD".to_owned(),
            cash: Decimal::new(1_000, 0),
            buying_power: Decimal::new(1_000, 0),
            total_assets: Decimal::new(1_000, 0),
            market_value: Decimal::ZERO,
            positions: Vec::new(),
            orders: Vec::new(),
        })
    }
}

/// Build one SQLite-backed application state for local-ledger route coverage.
async fn app() -> axum::Router {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .in_memory(true)
                .foreign_keys(true),
        )
        .await
        .expect("in-memory SQLite must connect");
    let storage = indexlink_storage::SqliteStorage::from_pool(pool);
    storage.migrate().await.expect("schema must migrate");
    let state = ApiState::new(storage, "test").with_broker(Arc::new(EmptyPaperBroker));
    build_router_with_cors(state, Vec::new())
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body must read");
    serde_json::from_slice(&body).expect("response must be JSON")
}

/// Verify a confirmed baseline unlocks a local zero-fill performance snapshot.
#[tokio::test]
async fn opening_balance_and_performance_routes_use_local_sqlite_ledger() {
    let app = app().await;
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "name": "Core ETF",
                        "symbol": "VOO",
                        "base_contribution": "1000.00",
                        "currency": "USD",
                        "schedule_kind": "monthly",
                        "schedule_day": 15,
                        "max_single_execution": "1500.00"
                    })
                    .to_string(),
                ))
                .expect("create request must build"),
        )
        .await
        .expect("create request must complete");
    assert_eq!(created.status(), StatusCode::CREATED);
    let id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let baseline = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/investment-plans/{id}/paper-performance/opening-balance"
                ))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "amount": "1000.00",
                        "occurred_at": "2026-07-19T00:00:00.000Z"
                    })
                    .to_string(),
                ))
                .expect("baseline request must build"),
        )
        .await
        .expect("baseline request must complete");
    assert_eq!(baseline.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/investment-plans/{id}/paper-performance"))
                .body(Body::empty())
                .expect("performance request must build"),
        )
        .await
        .expect("performance request must complete");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["has_opening_balance"], true);
    assert_eq!(body["data_complete"], true);
    assert_eq!(body["net_contributions"], "1000.00000000");
    assert_eq!(body["total_return"], "0.00000000");
    assert_eq!(body["points"].as_array().unwrap().len(), 1);
}

/// The hard-coded MA200 compatibility replay is no longer part of the public product API.
#[tokio::test]
async fn legacy_historical_backtest_route_is_removed() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/paper-performance/historical-backtest")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
