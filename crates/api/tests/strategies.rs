use std::{sync::Arc, time::Duration};

use ai_client::{
    AiClientError, AiCopilotDraft, AiCopilotDraftRequest, AiProvider, AiProviderCapabilities,
    AiProviderId, AiProviderProfile, AiProviderProfileId, MockAiProvider, NewsItem, NewsSource,
    NewsSourceError, Sentiment,
};
use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Datelike;
use core_domain::Multiplier;
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState};
use indexlink_storage::{SqliteStorage, SqliteStrategySpecRepository};
use market_data::{MarketDataError, MarketPricePoint, MarketSignalInput, MarketSignalProvider};
use rust_decimal::Decimal;
use serde_json::Value;
use strategy_dsl::{
    ComparisonOperator, Condition, IndicatorSpec, LookbackWindow, PolicyAction, StrategyRule,
    StrategySpec, ValueExpression,
};
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};
use tower::ServiceExt;

/// Unused news source retained only because the shared AI registry owns both adapters.
struct NoopNews;

#[async_trait]
impl NewsSource for NoopNews {
    /// Return no news because Copilot-draft tests never invoke the evidence pipeline.
    async fn fetch(&self) -> Result<Vec<NewsItem>, NewsSourceError> {
        Ok(Vec::new())
    }
}

/// Provider that deliberately cites an unknown reference to verify the API rejects hallucinations.
struct UntrustedCopilotAi;

#[async_trait]
impl AiProvider for UntrustedCopilotAi {
    /// Expose a test-only profile that declares the draft capability.
    fn profile(&self) -> AiProviderProfile {
        AiProviderProfile::new(
            AiProviderProfileId::new("untrusted-copilot").unwrap(),
            AiProviderId::new("test").unwrap(),
            "Untrusted Copilot".to_owned(),
            "fixture".to_owned(),
            AiProviderCapabilities::market_evidence_and_restricted_policy_drafts(),
        )
        .unwrap()
    }

    /// This route test does not request sentiment.
    async fn analyze(&self, _prompt: &str) -> Result<Sentiment, AiClientError> {
        Err(AiClientError::UnsupportedCapability)
    }

    /// Return a valid-looking document with an untrusted evidence-reference ID.
    async fn generate_policy_draft(
        &self,
        request: &AiCopilotDraftRequest,
    ) -> Result<AiCopilotDraft, AiClientError> {
        AiCopilotDraft::new(
            serde_json::json!({
                "policy_id": request.policy_id(),
                "policy_version": request.policy_version(),
                "name": "Untrusted candidate",
                "rules": [{
                    "condition": {
                        "kind": "comparison",
                        "expression": {"kind": "indicator", "indicator": {"kind": "vix"}},
                        "operator": "less_than",
                        "threshold": "30"
                    },
                    "action": {"kind": "set_opportunity_multiplier", "multiplier": 1.0}
                }]
            }),
            "This model attempted to cite an unknown source.".to_owned(),
            Vec::new(),
            vec!["invented_source".to_owned()],
        )
        .map_err(|_| AiClientError::ParseFailure)
    }
}

/// Deterministic automatic data source covering the online DSL RSI(14)/VIX profile.
struct StaticMarketData;

#[async_trait]
impl MarketSignalProvider for StaticMarketData {
    /// Return a complete source-labelled snapshot without network access.
    async fn fetch(&self, symbol: &str) -> Result<MarketSignalInput, MarketDataError> {
        let values = vec![1.0; 60];
        Ok(MarketSignalInput {
            symbol: symbol.to_ascii_uppercase(),
            as_of: "2026-08-25".to_owned(),
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
            vix_as_of: "2026-08-25".to_owned(),
        })
    }

    /// Return one harmless close because this test does not submit an order.
    async fn fetch_price_history(
        &self,
        _symbol: &str,
        _lookback_days: i64,
    ) -> Result<Vec<MarketPricePoint>, MarketDataError> {
        Ok((1..=20)
            .map(|day| MarketPricePoint {
                date: format!("2026-08-{day:02}"),
                close: 100.0 + f64::from(day),
            })
            .collect())
    }
}

/// Build a valid immutable strategy version for read-only API tests.
fn strategy() -> StrategySpec {
    StrategySpec::new(
        PolicyRef::new(
            PolicyId::new("dsl_api_test").unwrap(),
            PolicyVersion::new(1).unwrap(),
        ),
        "API RSI guard",
        vec![StrategyRule::new(
            Condition::compare(
                ValueExpression::indicator(IndicatorSpec::RelativeStrengthIndex(
                    LookbackWindow::new(14).unwrap(),
                )),
                ComparisonOperator::LessThan,
                Decimal::new(35, 0),
            ),
            PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(0.8)),
        )],
    )
    .unwrap()
}

/// Build a structurally valid version that exceeds the fixed-sample period budget.
fn over_budget_strategy() -> StrategySpec {
    StrategySpec::new(
        PolicyRef::new(
            PolicyId::new("dsl_over_budget_rejected").unwrap(),
            PolicyVersion::new(1).unwrap(),
        ),
        "Over-budget admission rejection",
        vec![StrategyRule::new(
            Condition::compare(
                ValueExpression::indicator(IndicatorSpec::ClosePrice),
                ComparisonOperator::GreaterThan,
                Decimal::ONE,
            ),
            PolicyAction::set_opportunity_fixed_amount(Decimal::new(1_001, 0)).unwrap(),
        )],
    )
    .unwrap()
}

/// Read a JSON HTTP response without leaking internal repository errors into assertions.
async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Verify Copilot returns only one validated read-only document and does not save it.
#[tokio::test]
async fn copilot_draft_is_validated_but_never_persisted_or_activated() {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    let app = build_router(
        ApiState::new(storage, "0.1.0")
            .with_market_sentiment(Arc::new(NoopNews), Arc::new(MockAiProvider::new())),
    );
    let request = serde_json::json!({
        "profile_id": "mock-default",
        "policy_id": "dsl_copilot_rsi_guard",
        "policy_version": 1,
        "objective": "Only increase the opportunity bucket when RSI is oversold."
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/strategies/copilot-draft")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["provider"]["id"], "mock-default");
    assert_eq!(body["document"]["policy_id"], "dsl_copilot_rsi_guard");
    assert_eq!(body["document"]["policy_version"], 1);
    assert_eq!(
        body["document"]["rules"][0]["action"]["kind"],
        "set_opportunity_multiplier"
    );
    assert!(body["explanation"].as_str().unwrap().contains("bounded"));
    assert!(body["evidence"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));

    let stored = app
        .oneshot(
            Request::builder()
                .uri("/strategies")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stored.status(), StatusCode::OK);
    assert_eq!(response_json(stored).await, serde_json::json!([]));
}

/// Verify neither invalid custom policy references nor invented evidence can cross the draft boundary.
#[tokio::test]
async fn copilot_draft_rejects_non_dsl_policy_and_invented_evidence() {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    let app = build_router(
        ApiState::new(storage, "0.1.0")
            .with_market_sentiment(Arc::new(NoopNews), Arc::new(UntrustedCopilotAi)),
    );

    let invalid_policy = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/strategies/copilot-draft")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "policy_id": "fixed_dca",
                        "policy_version": 1,
                        "objective": "A draft"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid_policy.status(), StatusCode::BAD_REQUEST);

    let invented_evidence = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/strategies/copilot-draft?unused=query")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "profile_id": "untrusted-copilot",
                        "policy_id": "dsl_untrusted_candidate",
                        "policy_version": 1,
                        "objective": "A conservative VIX rule"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invented_evidence.status(), StatusCode::SERVICE_UNAVAILABLE);
}

/// Verify stored strategy versions are discoverable but not mutable through this API surface.
#[tokio::test]
async fn lists_and_reads_persisted_strategy_versions() {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    SqliteStrategySpecRepository::new(storage.pool().clone())
        .save(&strategy())
        .await
        .unwrap();
    let app = build_router(ApiState::new(storage, "0.1.0"));

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/strategies")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = response_json(listed).await;
    assert_eq!(listed[0]["policy"]["id"], "dsl_api_test");
    assert_eq!(listed[0]["policy"]["version"], 1);
    assert_eq!(listed[0]["document"]["rules"].as_array().unwrap().len(), 1);

    let fetched = app
        .oneshot(
            Request::builder()
                .uri("/strategies/dsl_api_test/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(fetched.status(), StatusCode::OK);
    assert_eq!(response_json(fetched).await["name"], "API RSI guard");
}

/// Verify malformed or absent policy references use the established safe error envelope.
#[tokio::test]
async fn rejects_invalid_or_unknown_strategy_references() {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    let app = build_router(ApiState::new(storage, "0.1.0"));

    let invalid = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/strategies/fixed-dca/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    let unknown = app
        .oneshot(
            Request::builder()
                .uri("/strategies/dsl_missing/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
}

/// Verify a Studio form can validate and persist one immutable DSL version through safe routes.
#[tokio::test]
async fn validates_then_saves_a_restricted_strategy_document() {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    let app = build_router(ApiState::new(storage, "0.1.0"));
    let document = serde_json::to_value(strategy_dsl::StrategySpecDocument::from_strategy_spec(
        &strategy(),
    ))
    .unwrap();

    let validated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/strategies/validate")
                .header("content-type", "application/json")
                .body(Body::from(document.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(validated.status(), StatusCode::OK);
    assert_eq!(response_json(validated).await["valid"], true);

    let saved = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/strategies")
                .header("content-type", "application/json")
                .body(Body::from(document.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(saved.status(), StatusCode::CREATED);
    assert_eq!(response_json(saved).await["policy"]["id"], "dsl_api_test");
}

/// Verify explicit activation makes automatic preview execute the persisted DSL runtime version.
#[tokio::test]
async fn activates_a_validated_strategy_and_uses_it_for_automatic_audit() {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    SqliteStrategySpecRepository::new(storage.pool().clone())
        .save(&strategy())
        .await
        .unwrap();
    let app = build_router(
        ApiState::new(storage, "0.1.0").with_market_data(std::sync::Arc::new(StaticMarketData)),
    );
    let day = chrono::Utc::now().weekday().number_from_monday();
    let plan = serde_json::json!({
        "name": "DSL holding", "symbol": "VOO", "base_contribution": "100.00", "currency": "USD",
        "schedule_kind": "weekly", "schedule_day": day, "max_single_execution": "100.00",
        "bucket_allocation": {"core_ratio":"0.70", "opportunity_ratio":"0.30"}, "risk_mode": "autopilot"
    });
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header("content-type", "application/json")
                .body(Body::from(plan.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let plan_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let admission = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/strategies/dsl_api_test/1/admission")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(admission.status(), StatusCode::OK);
    let admission = response_json(admission).await;
    assert_eq!(admission["eligible"], true);
    assert_eq!(admission["core_bucket_safe"], true);
    assert_eq!(admission["budget_safe"], true);
    assert_eq!(admission["assets"].as_array().unwrap().len(), 2);
    assert!(admission["assets"][0]["fixed_dca"]["terminal_wealth_usd"]
        .as_f64()
        .is_some());

    let activated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/investment-plans/{plan_id}/activate-policy"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"policy":{"id":"dsl_api_test","version":1}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(activated.status(), StatusCode::OK);
    assert_eq!(
        response_json(activated).await["policy"]["id"],
        "dsl_api_test"
    );

    let simulation = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/strategies/dsl_api_test/1/simulate")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"symbol":"VOO"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(simulation.status(), StatusCode::OK);
    let simulation = response_json(simulation).await;
    assert_eq!(simulation["policy"]["id"], "dsl_api_test");
    assert!(simulation["evidence"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));

    let preview = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/investment-plans/{plan_id}/automatic-decision-preview"
                ))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(preview.status(), StatusCode::OK);
    let preview = response_json(preview).await;
    assert_eq!(preview["decision"]["policy"]["id"], "dsl_api_test");
    assert_eq!(preview["decision"]["action"], "standard");
}

/// Verify structural validity alone cannot bind a strategy exceeding its fixed-sample budget.
#[tokio::test]
async fn refuses_activation_when_fixed_sample_budget_is_ineligible() {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    SqliteStrategySpecRepository::new(storage.pool().clone())
        .save(&over_budget_strategy())
        .await
        .unwrap();
    let app = build_router(ApiState::new(storage, "0.1.0"));
    let plan = serde_json::json!({
        "name": "Admission guarded holding", "symbol": "VOO", "base_contribution": "100.00", "currency": "USD",
        "schedule_kind": "monthly", "schedule_day": 1, "max_single_execution": "100.00",
        "bucket_allocation": {"core_ratio":"0.70", "opportunity_ratio":"0.30"}, "risk_mode": "autopilot"
    });
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header("content-type", "application/json")
                .body(Body::from(plan.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let plan_id = response_json(created).await["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let admission = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/strategies/dsl_over_budget_rejected/1/admission")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(admission.status(), StatusCode::OK);
    assert_eq!(response_json(admission).await["eligible"], false);

    let activation = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/investment-plans/{plan_id}/activate-policy"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"policy":{"id":"dsl_over_budget_rejected","version":1}})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(activation.status(), StatusCode::BAD_REQUEST);
}
