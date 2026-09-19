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
    max_bars: usize,
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
            .take(self.max_bars)
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

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn create_plan(app: axum::Router, body: Value) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/investment-plans")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

fn plan_request(symbol: &str, currency: &str, policy_id: &str) -> Value {
    json!({
        "name": format!("{symbol} {policy_id} plan"),
        "symbol": symbol,
        "base_contribution": "1000.00",
        "currency": currency,
        "schedule_kind": "monthly",
        "schedule_day": 18,
        "schedule_days": [18],
        "policy": { "id": policy_id, "version": 1 },
        "bucket_allocation": if policy_id == "fixed_dca" {
            json!({ "core_ratio": "1.0", "opportunity_ratio": "0.0" })
        } else {
            json!({ "core_ratio": "0.7", "opportunity_ratio": "0.3" })
        },
        "risk_mode": if policy_id == "fixed_dca" { "fixed" } else { "approval" },
        "opportunity_cash_policy": "expire_each_period",
        "max_single_execution": "1000.00"
    })
}

#[tokio::test]
async fn consumer_catalog_exposes_only_adoptable_versioned_non_ai_strategies() {
    let response = app(None)
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
    assert_eq!(
        entries[0]["supported_markets"],
        json!(["us", "hong_kong", "china_shanghai", "china_shenzhen"])
    );
    assert!(entries
        .iter()
        .all(|entry| entry["supported_symbols"] == json!([])));
    assert_eq!(
        entries[0]["data_requirement"]["required_close_observations"],
        0
    );
    assert_eq!(
        entries[1]["data_requirement"]["required_close_observations"],
        200
    );
    assert_eq!(
        entries[2]["data_requirement"]["required_close_observations"],
        127
    );
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
    let app = app(None).await;
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

#[tokio::test]
async fn official_formula_plans_accept_dynamic_us_hk_and_china_instruments() {
    for (symbol, currency, stored_symbol) in [
        ("US.AAPL", "USD", "US.AAPL"),
        ("HK.00700", "HKD", "HK.00700"),
        ("SH.600519", "CNY", "SH.600519"),
        ("SZ.000001", "CNY", "SZ.000001"),
    ] {
        let response = create_plan(
            app(Some(Arc::new(StaticHistory {
                fails: false,
                max_bars: 500,
            })))
            .await,
            plan_request(symbol, currency, "dsl_ma200_trend_guard"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED, "{symbol}");
        assert_eq!(response_json(response).await["symbol"], stored_symbol);
    }
}

#[tokio::test]
async fn fixed_dca_accepts_a_dynamic_market_without_requiring_history() {
    let response = create_plan(
        app(None).await,
        plan_request("HK.09988", "HKD", "fixed_dca"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    assert_eq!(body["symbol"], "HK.09988");
    assert_eq!(body["currency"], "HKD");
}

#[tokio::test]
async fn official_plans_reject_malformed_symbols_and_market_currency_mismatches() {
    for body in [
        plan_request("JP.7974", "JPY", "fixed_dca"),
        plan_request("HK.00700", "USD", "fixed_dca"),
        plan_request("SH.600519", "HKD", "fixed_dca"),
    ] {
        let response = create_plan(app(None).await, body).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response_json(response).await["error"]["code"],
            "bad_request"
        );
    }
}

#[tokio::test]
async fn official_formula_plan_fails_closed_without_sufficient_usable_history() {
    let providers: [Option<Arc<dyn HistoricalPriceProvider>>; 4] = [
        None,
        Some(Arc::new(StaticHistory {
            fails: true,
            max_bars: 500,
        })),
        Some(Arc::new(StaticHistory {
            fails: false,
            max_bars: 199,
        })),
        Some(Arc::new(StaticHistory {
            fails: false,
            max_bars: 200,
        })),
    ];
    let expected = [
        StatusCode::SERVICE_UNAVAILABLE,
        StatusCode::SERVICE_UNAVAILABLE,
        StatusCode::BAD_REQUEST,
        StatusCode::BAD_REQUEST,
    ];
    for (provider, expected_status) in providers.into_iter().zip(expected) {
        let response = create_plan(
            app(provider).await,
            plan_request("US.AAPL", "USD", "dsl_ma200_trend_guard"),
        )
        .await;
        assert_eq!(response.status(), expected_status);
    }
}
