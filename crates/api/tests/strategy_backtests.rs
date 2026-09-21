use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration as ChronoDuration, Utc};
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState};
use indexlink_storage::{SqliteStorage, SqliteStrategySpecRepository};
use market_data::{
    Adjustment, DatasetSource, HistoricalPriceBar, HistoricalPriceDataset, HistoricalPriceProvider,
    HistoricalPriceRequest, Market, MarketDataError,
};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use strategy_dsl::{
    ComparisonOperator, Condition, IndicatorSpec, LookbackWindow, PolicyAction, StrategyRule,
    StrategySpec, ValueExpression,
};
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};
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

async fn app_with_personal_strategy(
    provider: Arc<dyn HistoricalPriceProvider>,
    strategy: &StrategySpec,
) -> axum::Router {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    SqliteStrategySpecRepository::new(storage.pool().clone())
        .save(strategy)
        .await
        .unwrap();
    build_router(ApiState::new(storage, "0.1.0").with_historical_price_provider(provider))
}

async fn app_with_corrupt_personal_strategy(
    provider: Arc<dyn HistoricalPriceProvider>,
    strategy: &StrategySpec,
) -> axum::Router {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    SqliteStrategySpecRepository::new(storage.pool().clone())
        .save(strategy)
        .await
        .unwrap();
    sqlx::query("UPDATE strategy_specs SET spec_json = '{\"corrupt\":true}' WHERE policy_id = ?1 AND policy_version = ?2")
        .bind(strategy.policy().id().as_str())
        .bind(i64::from(strategy.policy().version().value()))
        .execute(storage.pool())
        .await
        .unwrap();
    build_router(ApiState::new(storage, "0.1.0").with_historical_price_provider(provider))
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

fn personal_strategy() -> StrategySpec {
    StrategySpec::new(
        PolicyRef::new(
            PolicyId::new("dsl_personal_price_guard").unwrap(),
            PolicyVersion::new(7).unwrap(),
        ),
        "My price guard",
        vec![StrategyRule::new(
            Condition::compare(
                ValueExpression::indicator(IndicatorSpec::PriceReturn(
                    LookbackWindow::new(20).unwrap(),
                )),
                ComparisonOperator::LessThan,
                Decimal::ZERO,
            ),
            PolicyAction::set_opportunity_multiplier(core_domain::Multiplier::new_clamped(0.5)),
        )],
    )
    .unwrap()
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
        assert!(body["result"]["market_points"]
            .as_array()
            .is_some_and(|points| !points.is_empty()));
        assert_eq!(body["result"]["series"][0]["strategy_id"], "fixed_dca");
        assert!(body["result"]["series"][0]["normalized_points"]
            .as_array()
            .is_some_and(|points| !points.is_empty()));
        let executions = body["result"]["series"][0]["execution_points"]
            .as_array()
            .unwrap();
        assert_eq!(
            executions.len() as u64,
            body["result"]["contribution_count"].as_u64().unwrap()
        );
        assert_eq!(executions[0]["invested_amount"], 1_000.0);
        assert_eq!(executions[0]["budget_utilisation_percent"], 100.0);
        assert_eq!(executions[0]["scheduled_contribution_amount"], 1_000.0);
        assert_eq!(executions[0]["core_invested_amount"], 1_000.0);
        assert_eq!(executions[0]["opportunity_invested_amount"], 0.0);
        assert_eq!(executions[0]["unallocated_amount"], 0.0);
        assert!(executions[0]["transaction_cost"].as_f64().unwrap() > 0.0);
        assert_eq!(executions[0]["strategy_rule_matched"], false);
        assert!(body["result"]["series"][0]["drawdown_points"]
            .as_array()
            .is_some_and(|points| !points.is_empty()));
        assert_eq!(
            body["result"]["series"][0]["calculation_details"]["buy_cost_bps"],
            5.0
        );
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
async fn generated_formula_preset_runs_against_provider_history() {
    let response = post(
        app(Some(Arc::new(StaticHistory { fails: false }))).await,
        json!({
            "symbol": "SH.600519",
            "strategy_ids": ["fixed_dca", "dsl_price_sma_responsive"],
            "range": "1y",
            "monthly_day": 18,
            "contribution": "1000.00"
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["market"], "china_shanghai");
    assert_eq!(body["data"]["currency"], "CNY");
    assert_eq!(body["data"]["provider"], "test-history");
    assert_eq!(body["data"]["dataset_version"], "fixture-v1");
    let series = body["result"]["series"].as_array().unwrap();
    assert_eq!(series.len(), 2);
    assert_eq!(series[1]["strategy_id"], "dsl_price_sma_responsive");
    assert!(series.iter().all(|item| item["normalized_points"]
        .as_array()
        .is_some_and(|points| !points.is_empty())));
}

#[tokio::test]
async fn personal_and_official_exact_versions_share_one_backtest_window() {
    let response = post(
        app_with_personal_strategy(
            Arc::new(StaticHistory { fails: false }),
            &personal_strategy(),
        )
        .await,
        json!({
            "symbol": "US.SPY",
            "strategy_refs": [
                {"policy_id": "fixed_dca", "policy_version": 1},
                {"policy_id": "dsl_personal_price_guard", "policy_version": 7}
            ],
            "range": "1y",
            "monthly_day": 18,
            "contribution": "1000.00"
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    let series = body["result"]["series"].as_array().unwrap();
    assert_eq!(series.len(), 2);
    assert_eq!(series[0]["strategy_id"], "fixed_dca");
    assert_eq!(series[0]["strategy_version"], 1);
    assert_eq!(series[1]["strategy_id"], "dsl_personal_price_guard");
    assert_eq!(series[1]["strategy_version"], 7);
    assert_eq!(series[1]["strategy_name"], "My price guard");
    assert_eq!(
        series[0]["normalized_points"].as_array().unwrap().len(),
        series[1]["normalized_points"].as_array().unwrap().len()
    );
}

#[tokio::test]
async fn missing_or_wrong_personal_strategy_version_is_rejected_without_fallback() {
    for strategy_ref in [
        json!({"policy_id": "dsl_personal_price_guard", "policy_version": 8}),
        json!({"policy_id": "dsl_missing_personal", "policy_version": 1}),
    ] {
        let response = post(
            app_with_personal_strategy(
                Arc::new(StaticHistory { fails: false }),
                &personal_strategy(),
            )
            .await,
            json!({
                "symbol": "US.SPY",
                "strategy_refs": [strategy_ref],
                "range": "1y",
                "monthly_day": 18,
                "contribution": "1000.00"
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json_body(response).await["error"]["code"], "bad_request");
    }
}

#[tokio::test]
async fn corrupt_personal_strategy_is_unavailable_instead_of_falling_back() {
    let response = post(
        app_with_corrupt_personal_strategy(
            Arc::new(StaticHistory { fails: false }),
            &personal_strategy(),
        )
        .await,
        json!({
            "symbol": "US.SPY",
            "strategy_refs": [
                {"policy_id": "dsl_personal_price_guard", "policy_version": 7}
            ],
            "range": "1y",
            "monthly_day": 18,
            "contribution": "1000.00"
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        json_body(response).await["error"]["code"],
        "service_unavailable"
    );
}

#[tokio::test]
async fn malformed_or_unsafe_requests_use_the_existing_bad_request_envelope() {
    let cases = [
        json!({"symbol":"US.SPY","strategy_ids":["fixed_dca"],"range":"2y","monthly_day":18,"contribution":"1000"}),
        json!({"symbol":"US.SPY","strategy_ids":[],"range":"1y","monthly_day":18,"contribution":"1000"}),
        json!({"symbol":"US.SPY","strategy_ids":["fixed_dca","fixed_dca"],"range":"1y","monthly_day":18,"contribution":"1000"}),
        json!({"symbol":"US.SPY","strategy_refs":[{"policy_id":"fixed_dca","policy_version":1},{"policy_id":"fixed_dca","policy_version":1}],"range":"1y","monthly_day":18,"contribution":"1000"}),
        json!({"symbol":"US.SPY","strategy_ids":["fixed_dca"],"strategy_refs":[{"policy_id":"fixed_dca","policy_version":1}],"range":"1y","monthly_day":18,"contribution":"1000"}),
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
