use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration as ChronoDuration, Utc};
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState};
use indexlink_storage::SqliteStorage;
use market_data::{
    Adjustment, DatasetSource, HistoricalPriceBar, HistoricalPriceDataset, HistoricalPriceProvider,
    HistoricalPriceRequest, Market, MarketDataError,
};
use serde_json::{json, Value};
use tower::ServiceExt;

#[derive(Debug)]
struct StaticHistory {
    fails: bool,
}

#[async_trait]
impl HistoricalPriceProvider for StaticHistory {
    fn provider_id(&self) -> &'static str {
        "test-history"
    }

    fn preferred_adjustment(&self, market: Market) -> Result<Adjustment, MarketDataError> {
        Ok(match market {
            Market::Us => Adjustment::All,
            Market::HongKong | Market::ChinaShanghai | Market::ChinaShenzhen => Adjustment::Forward,
        })
    }

    async fn fetch_history(
        &self,
        request: &HistoricalPriceRequest,
    ) -> Result<HistoricalPriceDataset, MarketDataError> {
        if self.fails {
            return Err(MarketDataError::ProviderUnavailable);
        }
        let first = std::cmp::max(request.start(), request.end() - ChronoDuration::days(499));
        let bars = (0_i64..=499)
            .map(|offset| first + ChronoDuration::days(offset))
            .take_while(|date| *date <= request.end())
            .enumerate()
            .map(|(index, date)| HistoricalPriceBar::new(date, 100.0 + index as f64 * 0.05))
            .collect::<Result<Vec<_>, _>>()?;
        HistoricalPriceDataset::new(
            request,
            DatasetSource::new("test-history", "fixture-v1")?,
            Utc::now(),
            bars,
        )
    }
}

async fn app(provider: Option<Arc<dyn HistoricalPriceProvider>>) -> axum::Router {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    let state = ApiState::new(storage, "0.1.0");
    build_router(match provider {
        Some(provider) => state.with_historical_price_provider(provider),
        None => state,
    })
}

async fn post(app: axum::Router, body: Value) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/strategy-backtests")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn request(range: &str) -> Value {
    json!({
        "symbol": "US.SPY",
        "strategy_ids": ["fixed_dca"],
        "range": range,
        "monthly_day": 18,
        "contribution": "1000.00"
    })
}

#[tokio::test]
async fn all_documented_ranges_return_provenance_and_real_series() {
    for range in ["1m", "3m", "6m", "1y", "3y", "5y", "all"] {
        let response = post(
            app(Some(Arc::new(StaticHistory { fails: false }))).await,
            request(range),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK, "range {range}");
        let body = json_body(response).await;
        assert_eq!(body["requested_range"], range);
        assert_eq!(body["data"]["provider"], "test-history");
        assert_eq!(body["data"]["market"], "us");
        assert_eq!(body["data"]["adjustment"], "all");
        assert_eq!(body["data"]["dataset_version"], "fixture-v1");
        assert_eq!(body["data"]["checksum"].as_str().unwrap().len(), 64);
        assert_eq!(body["result"]["symbol"], "US.SPY");
        assert_eq!(body["result"]["series"][0]["strategy_id"], "fixed_dca");
        assert!(body["result"]["series"][0]["normalized_points"]
            .as_array()
            .is_some_and(|points| !points.is_empty()));
    }
}

#[tokio::test]
async fn up_to_three_unique_official_strategies_share_one_result_window() {
    let response = post(
        app(Some(Arc::new(StaticHistory { fails: false }))).await,
        json!({
            "symbol": "HK.00700",
            "strategy_ids": [
                "fixed_dca",
                "dsl_ma200_trend_guard",
                "dsl_growth_volatility_balance"
            ],
            "range": "1y",
            "monthly_day": 18,
            "contribution": "1000.00"
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["market"], "hong_kong");
    assert_eq!(body["data"]["currency"], "HKD");
    assert_eq!(body["data"]["adjustment"], "forward");
    let series = body["result"]["series"].as_array().unwrap();
    assert_eq!(series.len(), 3);
    let point_counts = series
        .iter()
        .map(|item| item["normalized_points"].as_array().unwrap().len())
        .collect::<Vec<_>>();
    assert!(point_counts.windows(2).all(|pair| pair[0] == pair[1]));
}

#[tokio::test]
async fn malformed_or_unsafe_requests_use_the_existing_bad_request_envelope() {
    let cases = [
        json!({"symbol":"US.SPY","strategy_ids":["fixed_dca"],"range":"2y","monthly_day":18,"contribution":"1000"}),
        json!({"symbol":"US.SPY","strategy_ids":[],"range":"1y","monthly_day":18,"contribution":"1000"}),
        json!({"symbol":"US.SPY","strategy_ids":["fixed_dca","fixed_dca"],"range":"1y","monthly_day":18,"contribution":"1000"}),
        json!({"symbol":"US.SPY","strategy_ids":["fixed_dca","dsl_ma200_trend_guard","dsl_growth_volatility_balance","fourth"],"range":"1y","monthly_day":18,"contribution":"1000"}),
        json!({"symbol":"JP.7974","strategy_ids":["fixed_dca"],"range":"1y","monthly_day":18,"contribution":"1000"}),
        json!({"symbol":"US.SPY","strategy_ids":["unknown"],"range":"1y","monthly_day":18,"contribution":"1000"}),
        json!({"symbol":"US.SPY","strategy_ids":["fixed_dca"],"range":"1y","monthly_day":29,"contribution":"1000"}),
        json!({"symbol":"US.SPY","strategy_ids":["fixed_dca"],"range":"1y","monthly_day":18,"contribution":"zero"}),
    ];
    for case in cases {
        let response = post(
            app(Some(Arc::new(StaticHistory { fails: false }))).await,
            case,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json_body(response).await["error"]["code"], "bad_request");
    }
}

#[tokio::test]
async fn missing_or_failed_optional_history_provider_is_explicitly_unavailable() {
    for provider in [
        None,
        Some(Arc::new(StaticHistory { fails: true }) as Arc<dyn HistoricalPriceProvider>),
    ] {
        let response = post(app(provider).await, request("1y")).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            json_body(response).await["error"]["code"],
            "service_unavailable"
        );
    }
}
