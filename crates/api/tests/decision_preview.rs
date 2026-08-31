use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use ai_client::{
    AiClientError, AiProvider, NewsItem, NewsSource, NewsSourceError, Sentiment, SentimentAnalysis,
};
use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use broker::MockBroker;
use chrono::{Datelike, Utc};
use decision_records::{
    CompleteDecisionRecord, CreateDecisionRecord, DecisionRecord, DecisionRecordListQuery,
    DecisionRecordRepository, DecisionRecordRepositoryError, DecisionRecordService,
};
use http_body_util::BodyExt;
use indexlink_api::{build_router, run_due_decisions, ApiState, ReadinessCheck, ReadinessError};
use indexlink_storage::SqliteStorage;
use investment_plans::{
    BucketAllocationRatio, CreateInvestmentPlan, InvestmentPlan, InvestmentPlanRepository,
    InvestmentPlanService, OpportunityCashPolicy, PlanExecutionConfiguration, PlanRepositoryError,
    PlanRiskMode, ScheduleKind, TwoBucketAllocationConfig, UpdateInvestmentPlan,
};
use market_data::{MarketDataError, MarketPricePoint, MarketSignalInput, MarketSignalProvider};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

/// Readiness stub used by decision preview route tests.
struct Ready;

#[async_trait]
impl ReadinessCheck for Ready {
    /// Always report dependencies as available.
    async fn check(&self) -> Result<(), ReadinessError> {
        Ok(())
    }
}

/// Deterministic news source used to verify the automatic sentiment pipeline.
struct StaticNews;

#[async_trait]
impl NewsSource for StaticNews {
    /// Return one news item without network access.
    async fn fetch(&self) -> Result<Vec<NewsItem>, NewsSourceError> {
        Ok(vec![NewsItem {
            title: "Markets rise on improving inflation data".to_owned(),
            description: "Deterministic decision-preview test news.".to_owned(),
            url: "https://example.com/market-rise".to_owned(),
            pub_date: Utc::now(),
        }])
    }
}

/// AI provider returning a fixed Qwen-equivalent sentiment for route tests.
struct PositiveAi;

#[async_trait]
impl AiProvider for PositiveAi {
    /// Return a bounded positive sentiment without network access.
    async fn analyze(&self, _prompt: &str) -> Result<Sentiment, AiClientError> {
        Ok(Sentiment::new(0.4).expect("test sentiment is in range"))
    }

    /// Return deterministic evidence for decision-preview persistence coverage.
    async fn analyze_with_evidence(
        &self,
        _prompt: &str,
    ) -> Result<SentimentAnalysis, AiClientError> {
        SentimentAnalysis::new(
            Sentiment::new(0.4).expect("test sentiment is in range"),
            "Inflation data improved market sentiment.".to_owned(),
            vec!["Market news can change quickly.".to_owned()],
        )
        .map_err(|_| AiClientError::ParseFailure)
    }
}

/// AI provider that models an unavailable automatic Qwen pipeline.
struct FailingAi;

#[async_trait]
impl AiProvider for FailingAi {
    /// Return a private provider failure that must trigger safe fallback weights.
    async fn analyze(&self, _prompt: &str) -> Result<Sentiment, AiClientError> {
        Err(AiClientError::EmptyResponse)
    }
}

/// Deterministic automatic 70/20 source used without OpenD or public-network access.
struct StaticMarketData;

#[async_trait]
impl MarketSignalProvider for StaticMarketData {
    /// Return a complete same-value fixture that the quant engine can evaluate.
    async fn fetch(&self, symbol: &str) -> Result<MarketSignalInput, MarketDataError> {
        let values = vec![1.0; 60];
        Ok(MarketSignalInput {
            symbol: symbol.to_ascii_uppercase(),
            as_of: "2026-07-19".to_owned(),
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
            vix_as_of: "2026-07-19".to_owned(),
        })
    }

    /// Return one harmless price point because this test only exercises decision orchestration.
    async fn fetch_price_history(
        &self,
        _symbol: &str,
        _lookback_days: i64,
    ) -> Result<Vec<MarketPricePoint>, MarketDataError> {
        Ok(vec![MarketPricePoint {
            date: "2026-07-19".to_owned(),
            close: 100.0,
        }])
    }
}

/// In-memory repository fake for previewing decisions through the API router.
#[derive(Default)]
struct FakeRepository {
    plans: Mutex<Vec<InvestmentPlan>>,
}

/// In-memory decision-record repository for verifying preview audit writes.
#[derive(Default)]
struct FakeDecisionRecordRepository {
    records: Mutex<Vec<DecisionRecord>>,
}

/// Decision-record fake that simulates unavailable local persistence.
struct UnavailableDecisionRecordRepository;

#[async_trait]
impl DecisionRecordRepository for FakeDecisionRecordRepository {
    /// Persist the supplied normalized record snapshot.
    async fn create(
        &self,
        input: CreateDecisionRecord,
    ) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        let mut records = self.records.lock().unwrap();
        let record = DecisionRecord {
            id: Uuid::from_u128((records.len() + 1) as u128),
            plan_id: input.plan_id,
            symbol: input.symbol,
            currency: input.currency,
            execution_status: input.execution_status,
            planned_contribution: input.planned_contribution,
            execution_snapshot: input.execution_snapshot,
            fundamental_snapshot: input.fundamental_snapshot,
            trend_snapshot: input.trend_snapshot,
            sentiment_snapshot: input.sentiment_snapshot,
            decision_snapshot: input.decision_snapshot,
            policy_evidence: input.policy_evidence,
            broker_order_request: input.broker_order_request,
            broker_order_ack: input.broker_order_ack,
            summary: input.summary,
            created_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        };
        records.push(record.clone());
        Ok(record)
    }

    /// Complete the stored order-intention audit record.
    async fn complete_broker_order(
        &self,
        id: Uuid,
        input: CompleteDecisionRecord,
    ) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        let mut records = self.records.lock().unwrap();
        let record = records
            .iter_mut()
            .find(|record| record.id == id)
            .ok_or(DecisionRecordRepositoryError::NotFound)?;
        record.broker_order_ack = Some(input.broker_order_ack);
        record.summary = input.summary;
        Ok(record.clone())
    }

    /// Return a bounded snapshot of matching records.
    async fn list_by_plan(
        &self,
        plan_id: Uuid,
        query: DecisionRecordListQuery,
    ) -> Result<Vec<DecisionRecord>, DecisionRecordRepositoryError> {
        Ok(self
            .records
            .lock()
            .unwrap()
            .iter()
            .filter(|record| record.plan_id == plan_id)
            .take(usize::from(query.limit()))
            .cloned()
            .collect())
    }

    /// Fetch one stored record by ID.
    async fn get(&self, id: Uuid) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .find(|record| record.id == id)
            .cloned()
            .ok_or(DecisionRecordRepositoryError::NotFound)
    }
}

#[async_trait]
impl DecisionRecordRepository for UnavailableDecisionRecordRepository {
    /// Reject creates to model unavailable local persistence before broker submission.
    async fn create(
        &self,
        _input: CreateDecisionRecord,
    ) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }

    /// Reject completions because the local persistence backend is unavailable.
    async fn complete_broker_order(
        &self,
        _id: Uuid,
        _input: CompleteDecisionRecord,
    ) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }

    /// Reject list queries because the local persistence backend is unavailable.
    async fn list_by_plan(
        &self,
        _plan_id: Uuid,
        _query: DecisionRecordListQuery,
    ) -> Result<Vec<DecisionRecord>, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }

    /// Reject reads because the local persistence backend is unavailable.
    async fn get(&self, _id: Uuid) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }
}

#[async_trait]
impl InvestmentPlanRepository for FakeRepository {
    /// Store the normalized create input as a persisted plan.
    async fn create(
        &self,
        input: CreateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let mut plans = self.plans.lock().unwrap();
        let plan = plan_from(Uuid::from_u128((plans.len() + 1) as u128), input);
        plans.push(plan.clone());
        Ok(plan)
    }

    /// Return a snapshot of stored plans.
    async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanRepositoryError> {
        Ok(self.plans.lock().unwrap().clone())
    }

    /// Return one stored plan by ID.
    async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanRepositoryError> {
        self.plans
            .lock()
            .unwrap()
            .iter()
            .find(|plan| plan.id == id)
            .cloned()
            .ok_or(PlanRepositoryError::NotFound)
    }

    /// Merge and store updates through the repository port.
    async fn update(
        &self,
        id: Uuid,
        input: UpdateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let mut plans = self.plans.lock().unwrap();
        let plan = plans
            .iter_mut()
            .find(|plan| plan.id == id)
            .ok_or(PlanRepositoryError::NotFound)?;

        if let Some(name) = input.name {
            plan.name = name;
        }
        if let Some(base_contribution) = input.base_contribution {
            plan.base_contribution = base_contribution;
        }
        if let Some(schedule_day) = input.schedule_day {
            plan.schedule_day = schedule_day;
        }
        if let Some(policy) = input.policy {
            plan.policy = policy;
        }
        if let Some(max_single_execution) = input.max_single_execution {
            plan.max_single_execution = max_single_execution;
        }
        if let Some(is_active) = input.is_active {
            plan.is_active = is_active;
        }

        Ok(plan.clone())
    }

    /// Active-state toggles are outside this route's scope.
    async fn set_active(
        &self,
        _id: Uuid,
        _is_active: bool,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        Err(PlanRepositoryError::Unavailable)
    }
}

/// Convert service input into a stored test plan.
fn plan_from(id: Uuid, input: CreateInvestmentPlan) -> InvestmentPlan {
    let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
    InvestmentPlan {
        id,
        name: input.name,
        symbol: input.symbol,
        base_contribution: input.base_contribution,
        currency: input.currency,
        schedule_kind: input.schedule_kind,
        schedule_day: input.schedule_day,
        schedule_days: input.schedule_days,
        policy: input
            .policy
            .unwrap_or_else(investment_plans::default_fixed_dca_policy),
        execution_configuration: input.execution_configuration,
        max_single_execution: input.max_single_execution,
        is_active: true,
        created_at: now,
        updated_at: now,
    }
}

/// Build an API app wired to fake investment plans and a mock broker.
fn app(repository: Arc<FakeRepository>, broker: Arc<MockBroker>) -> axum::Router {
    app_with_records(repository, broker).0
}

/// Build an API app and expose its local audit repository for assertions.
fn app_with_records(
    repository: Arc<FakeRepository>,
    broker: Arc<MockBroker>,
) -> (axum::Router, Arc<FakeDecisionRecordRepository>) {
    let records = Arc::new(FakeDecisionRecordRepository::default());
    let app = app_with_decision_records(
        repository,
        broker,
        Arc::clone(&records) as Arc<dyn DecisionRecordRepository>,
    );
    (app, records)
}

/// Build an API app with a caller-selected decision-record persistence port.
fn app_with_decision_records(
    repository: Arc<FakeRepository>,
    broker: Arc<MockBroker>,
    records: Arc<dyn DecisionRecordRepository>,
) -> axum::Router {
    app_with_sentiment_provider(repository, broker, records, Arc::new(PositiveAi))
}

/// Build an API app with a selected automatic market-sentiment provider.
fn app_with_sentiment_provider(
    repository: Arc<FakeRepository>,
    broker: Arc<MockBroker>,
    records: Arc<dyn DecisionRecordRepository>,
    provider: Arc<dyn AiProvider>,
) -> axum::Router {
    let state = ApiState::with_readiness_plans_broker_and_decision_records(
        Arc::new(Ready),
        InvestmentPlanService::new(repository),
        broker,
        DecisionRecordService::new(records),
        "0.1.0",
    )
    .with_market_sentiment(Arc::new(StaticNews), provider);
    build_router(state)
}

/// Build an app with source-backed 70/20 inputs for automatic-preview coverage.
fn app_with_automatic_sources(
    repository: Arc<FakeRepository>,
    broker: Arc<MockBroker>,
    records: Arc<dyn DecisionRecordRepository>,
) -> axum::Router {
    let state = ApiState::with_readiness_plans_broker_and_decision_records(
        Arc::new(Ready),
        InvestmentPlanService::new(repository),
        broker,
        DecisionRecordService::new(records),
        "0.1.0",
    )
    .with_market_sentiment(Arc::new(StaticNews), Arc::new(PositiveAi))
    .with_market_data(Arc::new(StaticMarketData));
    build_router(state)
}

/// Build an API app that exercises Decision Preview without a Qwen provider.
fn app_without_sentiment_provider(
    repository: Arc<FakeRepository>,
    broker: Arc<MockBroker>,
    records: Arc<dyn DecisionRecordRepository>,
) -> axum::Router {
    build_router(ApiState::with_readiness_plans_broker_and_decision_records(
        Arc::new(Ready),
        InvestmentPlanService::new(repository),
        broker,
        DecisionRecordService::new(records),
        "0.1.0",
    ))
}

/// Parse an HTTP response body as JSON.
async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Build a normalized domain input for seeding the fake repository.
fn create_input() -> CreateInvestmentPlan {
    CreateInvestmentPlan {
        name: "Core ETF".to_owned(),
        symbol: "VOO".to_owned(),
        base_contribution: Decimal::new(1000, 0),
        currency: "USD".to_owned(),
        schedule_kind: ScheduleKind::Monthly,
        schedule_day: 15,
        schedule_days: vec![15],
        policy: Some(investment_plans::legacy_core_opportunity_v1_policy()),
        execution_configuration: PlanExecutionConfiguration::new_with_cash_policy(
            TwoBucketAllocationConfig::new(
                BucketAllocationRatio::new(Decimal::new(80, 2)).unwrap(),
                BucketAllocationRatio::new(Decimal::new(20, 2)).unwrap(),
            )
            .unwrap(),
            PlanRiskMode::Autopilot,
            OpportunityCashPolicy::ExpireEachPeriod,
        )
        .unwrap(),
        max_single_execution: Decimal::new(1500, 0),
    }
}

/// Build a valid decision preview payload.
fn preview_payload(day_of_month: i16, regime: &str) -> Value {
    json!({
        "day_of_month": day_of_month,
        "bucket_allocation": {
            "core_ratio": "0.80",
            "opportunity_ratio": "0.20"
        },
        "fundamental": {
            "score": 0.10,
            "cape_percentile": 0.10,
            "erp_percentile": 0.90
        },
        "trend": {
            "score": 0.50,
            "ma_distance_percentile": 0.50,
            "rsi_percentile": 0.50,
            "vix_percentile": 0.50,
            "regime": regime
        },
        "paper_order": {
            "idempotency_key": "decision-preview-demo-1",
            "side": "buy",
            "order_type": "market",
            "quantity": "1.00"
        }
    })
}

/// Verify a fixed-DCA plan reaches the shared preview/audit path without market or Qwen inputs.
#[tokio::test]
async fn fixed_dca_automatic_preview_does_not_require_market_signals() {
    let repository = Arc::new(FakeRepository::default());
    let broker = Arc::new(MockBroker::paper_only());
    let mut input = create_input();
    input.policy = None;
    let created = repository.create(input).await.unwrap();
    let records = Arc::new(FakeDecisionRecordRepository::default());
    let app = app_without_sentiment_provider(
        repository,
        broker,
        Arc::clone(&records) as Arc<dyn DecisionRecordRepository>,
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/investment-plans/{}/automatic-decision-preview",
                    created.id
                ))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["decision"]["policy"]["id"], json!("fixed_dca"));
    assert_eq!(body["decision"]["market_signals_used"], json!(false));
    assert!(body["decision"].get("final_score").is_none());
    let persisted = records.records.lock().unwrap();
    let evidence = persisted[0]
        .policy_evidence
        .as_ref()
        .expect("new audit records must retain structured policy evidence");
    assert_eq!(evidence.policy.to_string(), "fixed_dca@1");
    assert_eq!(
        evidence.recommendation_snapshot["market_signals_used"],
        json!(false)
    );
}

/// Verify a due executable decision submits one MockBroker paper order.
#[tokio::test]
async fn decision_preview_submits_mock_paper_order_when_due() {
    let repository = Arc::new(FakeRepository::default());
    let broker = Arc::new(MockBroker::paper_only());
    let created = repository.create(create_input()).await.unwrap();
    let (app, records) = app_with_records(repository, Arc::clone(&broker));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/investment-plans/{}/decision-preview", created.id))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(preview_payload(15, "neutral").to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["execution"]["status"], json!("due"));
    assert_eq!(
        body["execution"]["bucket_split"]["core_contribution"],
        json!("800.00")
    );
    assert_eq!(body["decision"]["action"], json!("overweight"));
    assert_eq!(body["decision"]["sentiment_score"], json!(0.7));
    assert_eq!(body["market_sentiment"]["score"], json!(0.4));
    assert_eq!(
        body["market_sentiment"]["rationale"],
        json!("Inflation data improved market sentiment.")
    );
    assert_eq!(
        body["market_sentiment"]["headlines"][0]["url"],
        json!("https://example.com/market-rise")
    );
    assert_eq!(body["paper_order_ack"]["status"], json!("accepted"));
    let summary = body["summary"].as_str().unwrap();
    assert!(summary.contains("fundamental_investability=0.90 (supportive)"));
    assert!(summary.contains("trend_timing=0.50 (neutral, regime=Neutral)"));
    assert!(summary.contains("market_sentiment=0.70"));
    assert!(summary.contains(
        "bucket_split=core=800.00 USD, opportunity_budget=200.00 USD, carried_opportunity_cash=0 USD, opportunity=200.00 USD, recommended=1000.00 USD"
    ));
    assert_eq!(broker.accepted_orders().len(), 1);
    let persisted = records.records.lock().unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(
        persisted[0].execution_snapshot["trigger"],
        json!("manual_input")
    );
    assert_eq!(
        persisted[0].execution_snapshot["execution"]["status"],
        json!("due")
    );
    assert_eq!(
        persisted[0].execution_snapshot["execution"]["bucket_split"]["recommended_contribution"],
        json!("1000.00")
    );
    assert_eq!(
        persisted[0].fundamental_snapshot["signal"]["score"],
        json!(0.10)
    );
    assert_eq!(
        persisted[0].trend_snapshot["signal"]["regime"],
        json!("neutral")
    );
    let sentiment_snapshot = persisted[0].sentiment_snapshot.as_ref().unwrap();
    assert_eq!(sentiment_snapshot["source"], json!("ai_evidence"));
    assert_eq!(
        sentiment_snapshot["provider"]["id"],
        json!("external-default")
    );
    assert_eq!(sentiment_snapshot["score"], json!(0.4));
    assert_eq!(
        sentiment_snapshot["rationale"],
        json!("Inflation data improved market sentiment.")
    );
    assert_eq!(
        sentiment_snapshot["warnings"],
        json!(["Market news can change quickly."])
    );
    assert_eq!(
        sentiment_snapshot["headlines"][0]["url"],
        json!("https://example.com/market-rise")
    );
    assert!(sentiment_snapshot["headlines"][0]["published_at"].is_string());
    assert_eq!(
        persisted[0].broker_order_ack.as_ref().unwrap()["status"],
        json!("accepted")
    );
}

/// Verify automatic preview hides caller signal fields and persists automatic-source disclosure.
#[tokio::test]
async fn automatic_decision_preview_uses_server_sources_and_writes_readable_audit() {
    let repository = Arc::new(FakeRepository::default());
    let broker = Arc::new(MockBroker::paper_only());
    let mut input = create_input();
    input.schedule_day = i16::try_from(Utc::now().day()).unwrap();
    input.schedule_days = vec![input.schedule_day];
    let created = repository.create(input).await.unwrap();
    let records = Arc::new(FakeDecisionRecordRepository::default());
    let app = app_with_automatic_sources(
        repository,
        broker,
        Arc::clone(&records) as Arc<dyn DecisionRecordRepository>,
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/investment-plans/{}/automatic-decision-preview",
                    created.id
                ))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert!(body["audit_record_id"].is_string());
    assert_eq!(body["execution"]["status"], json!("due"));
    assert!(body.get("paper_order_ack").is_none());
    let persisted = records.records.lock().unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(
        persisted[0].execution_snapshot["trigger"],
        json!("automatic_preview")
    );
    assert_eq!(
        persisted[0].fundamental_snapshot["source"]["kind"],
        json!("automatic_market_data")
    );
    assert_eq!(
        persisted[0].trend_snapshot["source"]["symbol"],
        json!("VOO")
    );
}

/// Verify the persisted scheduler claim prevents duplicate automatic records on a second tick.
#[tokio::test]
async fn scheduler_creates_one_due_audit_record_per_plan_and_utc_day() {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    let state = ApiState::new(storage, "0.1.0")
        .with_market_sentiment(Arc::new(StaticNews), Arc::new(PositiveAi))
        .with_market_data(Arc::new(StaticMarketData));
    let app = build_router(state.clone());
    let day = Utc::now().weekday().number_from_monday();
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "Scheduled VOO",
                        "symbol": "VOO",
                        "base_contribution": "100.00",
                        "currency": "USD",
                        "schedule_kind": "weekly",
                        "schedule_day": day,
                        "max_single_execution": "100.00"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let created = response_json(created).await;
    let plan_id = created["id"].as_str().unwrap();

    let first = run_due_decisions(&state).await.unwrap();
    let second = run_due_decisions(&state).await.unwrap();
    assert_eq!(first.created, 1);
    assert_eq!(first.already_claimed, 0);
    assert_eq!(second.created, 0);
    assert_eq!(second.already_claimed, 1);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/investment-plans/{plan_id}/decisions"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let records = response_json(response).await;
    assert_eq!(records.as_array().unwrap().len(), 1);
    assert_eq!(
        records[0]["execution_snapshot"]["trigger"],
        json!("automatic_scheduler")
    );
}

/// Verify the scheduler executes a weekly plan once when today's weekday is one of several dates.
#[tokio::test]
async fn scheduler_executes_weekly_multi_day_plan() {
    let storage = SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
        .await
        .unwrap();
    storage.migrate().await.unwrap();
    let state = ApiState::new(storage, "0.1.0")
        .with_market_sentiment(Arc::new(StaticNews), Arc::new(PositiveAi))
        .with_market_data(Arc::new(StaticMarketData));
    let app = build_router(state.clone());
    let weekday = Utc::now().weekday().num_days_from_monday() + 1;
    let schedule_days = if weekday == 1 {
        vec![1]
    } else {
        vec![1, weekday]
    };
    let created = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "Weekly multi-day VOO",
                        "symbol": "VOO",
                        "base_contribution": "100.00",
                        "currency": "USD",
                        "schedule_kind": "weekly",
                        "schedule_day": 1,
                        "schedule_days": schedule_days,
                        "max_single_execution": "100.00"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    assert_eq!(run_due_decisions(&state).await.unwrap().created, 1);
}

/// Verify an unavailable Qwen pipeline uses the documented 90/10/0 fallback.
#[tokio::test]
async fn decision_preview_uses_fallback_weights_when_qwen_is_unavailable() {
    let repository = Arc::new(FakeRepository::default());
    let broker = Arc::new(MockBroker::paper_only());
    let created = repository.create(create_input()).await.unwrap();
    let records = Arc::new(FakeDecisionRecordRepository::default());
    let app = app_with_sentiment_provider(
        repository,
        broker,
        Arc::clone(&records) as Arc<dyn DecisionRecordRepository>,
        Arc::new(FailingAi),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/investment-plans/{}/decision-preview", created.id))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(preview_payload(16, "neutral").to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(
        body["decision"]["weight_mode"],
        json!("sentiment_unavailable")
    );
    assert!(body["decision"].get("sentiment_score").is_none());
    assert!(body["summary"]
        .as_str()
        .unwrap()
        .contains("market_sentiment=unavailable"));
    assert!(records.records.lock().unwrap()[0]
        .sentiment_snapshot
        .is_none());
}

/// Verify an absent Qwen configuration uses the same explicit fallback mode.
#[tokio::test]
async fn decision_preview_uses_fallback_weights_without_qwen_configuration() {
    let repository = Arc::new(FakeRepository::default());
    let broker = Arc::new(MockBroker::paper_only());
    let created = repository.create(create_input()).await.unwrap();
    let app = app_without_sentiment_provider(
        repository,
        broker,
        Arc::new(FakeDecisionRecordRepository::default()),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/investment-plans/{}/decision-preview", created.id))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(preview_payload(16, "neutral").to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await["decision"]["weight_mode"],
        json!("sentiment_unavailable")
    );
}

/// Verify non-due previews never submit paper orders.
#[tokio::test]
async fn decision_preview_waiting_does_not_submit_order() {
    let repository = Arc::new(FakeRepository::default());
    let broker = Arc::new(MockBroker::paper_only());
    let created = repository.create(create_input()).await.unwrap();
    let app = app(repository, Arc::clone(&broker));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/investment-plans/{}/decision-preview", created.id))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(preview_payload(16, "neutral").to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["execution"]["status"], json!("waiting"));
    assert!(body.get("paper_order_ack").is_none());
    assert!(broker.accepted_orders().is_empty());
}

/// Verify a tactical delay submits the preserved core bucket when the plan is due.
#[tokio::test]
async fn decision_preview_tactical_delay_submits_preserved_core_bucket() {
    let repository = Arc::new(FakeRepository::default());
    let broker = Arc::new(MockBroker::paper_only());
    let created = repository.create(create_input()).await.unwrap();
    let app = app(repository, Arc::clone(&broker));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/investment-plans/{}/decision-preview", created.id))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(preview_payload(15, "overheated").to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["execution"]["status"], json!("due"));
    assert_eq!(body["decision"]["action"], json!("tactical_delay"));
    assert_eq!(
        body["execution"]["bucket_split"]["core_contribution"],
        json!("800.00")
    );
    assert_eq!(
        body["execution"]["bucket_split"]["opportunity_contribution"],
        json!("0")
    );
    assert_eq!(body["paper_order_ack"]["status"], json!("accepted"));
    assert_eq!(broker.accepted_orders().len(), 1);
}

/// Verify a skip also preserves and submits the due core bucket.
#[tokio::test]
async fn decision_preview_skip_submits_preserved_core_bucket() {
    let repository = Arc::new(FakeRepository::default());
    let broker = Arc::new(MockBroker::paper_only());
    let created = repository.create(create_input()).await.unwrap();
    let app = app_without_sentiment_provider(
        repository,
        Arc::clone(&broker),
        Arc::new(FakeDecisionRecordRepository::default()),
    );
    let mut payload = preview_payload(15, "neutral");
    payload["fundamental"] = json!({
        "score": 1.0,
        "cape_percentile": 1.0,
        "erp_percentile": 0.0
    });
    payload["trend"] = json!({
        "score": 0.0,
        "ma_distance_percentile": 0.0,
        "rsi_percentile": 0.0,
        "vix_percentile": 0.0,
        "regime": "neutral"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/investment-plans/{}/decision-preview", created.id))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["decision"]["action"], json!("skip"));
    assert_eq!(
        body["execution"]["bucket_split"]["recommended_contribution"],
        json!("800.00")
    );
    assert_eq!(body["paper_order_ack"]["status"], json!("accepted"));
    assert_eq!(broker.accepted_orders().len(), 1);
}

/// Verify an all-opportunity plan never sends an empty order after a skip.
#[tokio::test]
async fn decision_preview_skip_does_not_submit_zero_recommendation() {
    let repository = Arc::new(FakeRepository::default());
    let broker = Arc::new(MockBroker::paper_only());
    let mut input = create_input();
    input.execution_configuration = PlanExecutionConfiguration::new_with_cash_policy(
        TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(Decimal::ZERO).unwrap(),
            BucketAllocationRatio::new(Decimal::ONE).unwrap(),
        )
        .unwrap(),
        PlanRiskMode::Autopilot,
        OpportunityCashPolicy::ExpireEachPeriod,
    )
    .unwrap();
    let created = repository.create(input).await.unwrap();
    let app = app_without_sentiment_provider(
        repository,
        Arc::clone(&broker),
        Arc::new(FakeDecisionRecordRepository::default()),
    );
    let mut payload = preview_payload(15, "neutral");
    payload["fundamental"] = json!({
        "score": 1.0,
        "cape_percentile": 1.0,
        "erp_percentile": 0.0
    });
    payload["trend"] = json!({
        "score": 0.0,
        "ma_distance_percentile": 0.0,
        "rsi_percentile": 0.0,
        "vix_percentile": 0.0,
        "regime": "neutral"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/investment-plans/{}/decision-preview", created.id))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["decision"]["action"], json!("skip"));
    assert_eq!(
        body["execution"]["bucket_split"]["recommended_contribution"],
        json!("0")
    );
    assert!(body.get("paper_order_ack").is_none());
    assert!(broker.accepted_orders().is_empty());
}

/// Verify unavailable audit persistence blocks the broker call before its side effect.
#[tokio::test]
async fn decision_preview_does_not_submit_when_audit_persistence_is_unavailable() {
    let repository = Arc::new(FakeRepository::default());
    let broker = Arc::new(MockBroker::paper_only());
    let created = repository.create(create_input()).await.unwrap();
    let app = app_with_decision_records(
        repository,
        Arc::clone(&broker),
        Arc::new(UnavailableDecisionRecordRepository),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/investment-plans/{}/decision-preview", created.id))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(preview_payload(15, "neutral").to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(broker.accepted_orders().is_empty());
}

/// Verify malformed previews return the shared bad-request envelope.
#[tokio::test]
async fn decision_preview_maps_bad_input_to_safe_bad_request() {
    let repository = Arc::new(FakeRepository::default());
    let broker = Arc::new(MockBroker::paper_only());
    let created = repository.create(create_input()).await.unwrap();
    let app = app(repository, broker);
    let mut waiting_invalid_order = preview_payload(16, "neutral");
    waiting_invalid_order["paper_order"]["limit_price"] = json!("10.00");
    let mut tactical_delay_invalid_order = preview_payload(15, "overheated");
    tactical_delay_invalid_order["paper_order"]["limit_price"] = json!("10.00");

    for (uri, body) in [
        (
            "/investment-plans/not-a-uuid/decision-preview".to_owned(),
            preview_payload(15, "neutral").to_string(),
        ),
        (
            format!("/investment-plans/{}/decision-preview", created.id),
            json!({"day_of_month": 32}).to_string(),
        ),
        (
            format!("/investment-plans/{}/decision-preview", created.id),
            json!({
                "day_of_month": 15,
                "fundamental": {
                    "score": 0.10,
                    "cape_percentile": 0.10,
                    "erp_percentile": 0.90
                },
                "trend": {
                    "score": 0.50,
                    "ma_distance_percentile": 0.50,
                    "rsi_percentile": 0.50,
                    "vix_percentile": 0.50,
                    "regime": "neutral"
                },
                "sentiment": {"score": 0.80}
            })
            .to_string(),
        ),
        (
            format!("/investment-plans/{}/decision-preview", created.id),
            json!({
                "day_of_month": 15,
                "fundamental": {
                    "score": 1.20,
                    "cape_percentile": 0.10,
                    "erp_percentile": 0.90
                },
                "trend": {
                    "score": 0.50,
                    "ma_distance_percentile": 0.50,
                    "rsi_percentile": 0.50,
                    "vix_percentile": 0.50,
                    "regime": "neutral"
                }
            })
            .to_string(),
        ),
        (
            format!("/investment-plans/{}/decision-preview", created.id),
            json!({
                "day_of_month": 15,
                "fundamental": {
                    "score": 0.10,
                    "cape_percentile": 0.10,
                    "erp_percentile": 0.90
                },
                "trend": {
                    "score": 0.50,
                    "ma_distance_percentile": 0.50,
                    "rsi_percentile": 0.50,
                    "vix_percentile": 0.50,
                    "regime": "neutral"
                },
                "paper_order": {
                    "idempotency_key": "bad-market-limit",
                    "side": "buy",
                    "order_type": "market",
                    "quantity": "1.00",
                    "limit_price": "10.00"
                }
            })
            .to_string(),
        ),
        (
            format!("/investment-plans/{}/decision-preview", created.id),
            waiting_invalid_order.to_string(),
        ),
        (
            format!("/investment-plans/{}/decision-preview", created.id),
            tactical_delay_invalid_order.to_string(),
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response_json(response).await,
            json!({"error": {"code": "bad_request", "message": "invalid request"}})
        );
    }
}
