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

async fn save_personal_strategy(app: &axum::Router, document: Value) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/strategies")
                .header("content-type", "application/json")
                .body(Body::from(document.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

fn personal_strategy_document(policy_version: u32, name: &str, fixed_amount: bool) -> Value {
    let action = if fixed_amount {
        json!({ "kind": "set_opportunity_fixed_amount", "amount": "1001" })
    } else {
        json!({ "kind": "set_opportunity_multiplier", "multiplier": 0.8 })
    };
    json!({
        "policy_id": "dsl_personal_catalog_test",
        "policy_version": policy_version,
        "name": name,
        "rules": [{
            "condition": {
                "kind": "comparison",
                "expression": { "kind": "indicator", "indicator": { "kind": "close_price" } },
                "operator": "greater_than",
                "threshold": "0"
            },
            "action": action
        }]
    })
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
async fn consumer_catalog_exposes_grouped_versioned_non_ai_presets() {
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

    assert_eq!(entries.len(), 101);
    assert_eq!(ids[0], "fixed_dca");
    assert!(entries.iter().all(|entry| entry["origin"] == "official"));
    assert!(entries
        .iter()
        .all(|entry| entry["lifecycle"] == "published"));
    assert!(entries.iter().all(|entry| entry["status"] == "usable"));
    assert_eq!(
        ids.iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        101
    );
    assert!(ids.contains(&"dsl_ma200_trend_guard"));
    assert!(ids.contains(&"dsl_growth_volatility_balance"));
    assert!(!body.to_string().contains("core_opportunity_v1"));
    assert!(entries.iter().all(|entry| entry["adoptable"] == true));
    assert_eq!(entries[0]["research_status"], "reference");
    assert_eq!(entries[0]["validation_mode"], "reference");
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
    let formula_entries = &entries[1..];
    let family_ids = formula_entries
        .iter()
        .map(|entry| entry["family"]["id"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(family_ids.len(), 20);
    for family_id in family_ids {
        assert_eq!(
            formula_entries
                .iter()
                .filter(|entry| entry["family"]["id"] == family_id)
                .count(),
            5,
            "{family_id}"
        );
    }
    for entry in formula_entries {
        assert_eq!(entry["research_status"], "available");
        assert_eq!(entry["default_plan"]["core_ratio"], "0.7");
        assert_eq!(entry["default_plan"]["opportunity_ratio"], "0.3");
        assert!(entry["data_requirement"]["required_close_observations"]
            .as_u64()
            .is_some_and(|required| required <= 253));
        assert_eq!(entry["supported_symbols"], json!([]));
        assert!(entry["preset"]["order"]
            .as_u64()
            .is_some_and(|order| (1..=5).contains(&order)));
        assert!(!entry["tags"].as_array().unwrap().is_empty());
        assert!(entry["source"]["url"]
            .as_str()
            .unwrap()
            .starts_with("https://"));
    }

    for id in ["dsl_ma200_trend_guard", "dsl_growth_volatility_balance"] {
        let entry = formula_entries
            .iter()
            .find(|entry| entry["policy"]["id"] == id)
            .unwrap();
        assert_eq!(entry["validation_mode"], "fixed_fixture");
        assert_eq!(entry["research"]["eligible"], true);
        assert_eq!(entry["research"]["assets"].as_array().unwrap().len(), 2);
        assert!(entry["formula"]["rules"].as_array().is_some());
    }
    let generated = formula_entries
        .iter()
        .find(|entry| entry["policy"]["id"] == "dsl_price_sma_responsive")
        .unwrap();
    assert_eq!(generated["validation_mode"], "compiled_formula");
    assert_eq!(generated["name"], "价格与简单均线（50日）");
    assert_eq!(generated["preset"]["name"], "50日");
    assert!(generated.get("formula").is_none());
    assert!(generated.get("research").is_none());

    let ema20 = formula_entries
        .iter()
        .find(|entry| entry["policy"]["id"] == "dsl_price_ema_responsive")
        .unwrap();
    assert_eq!(ema20["name"], "价格与指数（20日）");
    assert_eq!(ema20["family"]["name"], "价格与指数");
}

#[tokio::test]
async fn unified_catalog_appends_exact_immutable_personal_versions_with_honest_status() {
    let app = app(None).await;
    assert_eq!(
        save_personal_strategy(
            &app,
            personal_strategy_document(1, "个人价格规则 v1", false)
        )
        .await,
        StatusCode::CREATED
    );
    assert_eq!(
        save_personal_strategy(&app, personal_strategy_document(2, "个人价格规则 v2", true)).await,
        StatusCode::CREATED
    );

    let duplicate =
        save_personal_strategy(&app, personal_strategy_document(1, "试图覆盖 v1", false)).await;
    assert_ne!(duplicate, StatusCode::CREATED);

    let response = app
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
    assert_eq!(entries.len(), 103);

    let personal = entries
        .iter()
        .filter(|entry| entry["origin"] == "personal")
        .collect::<Vec<_>>();
    assert_eq!(personal.len(), 2);
    assert!(personal
        .iter()
        .all(|entry| entry["policy"]["id"] == "dsl_personal_catalog_test"));
    assert_eq!(personal[0]["policy"]["version"], 2);
    assert_eq!(personal[1]["policy"]["version"], 1);
    assert_eq!(personal[1]["name"], "个人价格规则 v1");
    assert!(personal.iter().all(|entry| entry["lifecycle"] == "saved"));
    assert!(personal
        .iter()
        .all(|entry| entry["formula"]["policy_id"] == "dsl_personal_catalog_test"));

    let fixed_amount = personal
        .iter()
        .find(|entry| entry["policy"]["version"] == 2)
        .unwrap();
    assert_eq!(fixed_amount["status"], "validated");
    assert_eq!(fixed_amount["adoptable"], false);
    assert_eq!(fixed_amount["validation_mode"], "compiled_formula");
    assert_eq!(fixed_amount["research_status"], "blocked");
    assert!(fixed_amount.get("research").is_none());

    let plan_usable = personal
        .iter()
        .find(|entry| entry["policy"]["version"] == 1)
        .unwrap();
    assert_eq!(plan_usable["status"], "usable");
    assert_eq!(plan_usable["adoptable"], true);
    assert_eq!(plan_usable["validation_mode"], "fixed_fixture");
    assert_eq!(plan_usable["research"]["eligible"], true);
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
async fn generated_formula_plans_accept_dynamic_us_hk_and_china_instruments() {
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
            plan_request(symbol, currency, "dsl_price_sma_responsive"),
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
