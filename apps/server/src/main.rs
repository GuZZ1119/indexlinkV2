#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! IndexLink HTTP 服务进程。

mod config;
mod shutdown;

use ai_client::{AiProvider, QwenClient, RssNewsSource};
use broker::{
    BrokerClient, BrokerError, OpenDConnectionConfig, OpenDPaperBroker, OpenDPaperSession,
    OpenDSessionError,
};
use config::{AiProviderConfiguration, Config, SchedulerConfig};
use indexlink_api::{build_router_with_cors, ApiState, SchedulerStatusHandle};
use indexlink_storage::SqliteStorage;
use market_data::OpenDMarketSignalProvider;
use std::{future::Future, sync::Arc};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    init_tracing()?;

    let config = Config::from_env()?;
    run(config).await
}

/// Run the configured HTTP server until the operating-system shutdown signal arrives.
async fn run(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_with_shutdown(config, shutdown::signal()).await
}

/// Run one fully composed server with an injected shutdown future.
///
/// Keeping the shutdown trigger injectable makes startup, migration, and graceful-stop
/// behavior verifiable without installing a process-wide signal handler in tests.
async fn run_with_shutdown<F>(
    config: Config,
    shutdown_signal: F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: Future<Output = ()> + Send + 'static,
{
    let storage = SqliteStorage::connect_with_options(
        &config.database_url,
        config.database_max_connections,
        config.database_connect_timeout,
    )
    .await?;
    storage.migrate().await?;
    tracing::info!("SQLite migrations applied");
    let market_sentiment_configured = !config.ai_providers.is_empty();
    let paper_broker_configured = config.opend.is_some();
    let scheduler_status = SchedulerStatusHandle::new(
        config.scheduler.enabled,
        config.scheduler.tick_interval.as_secs(),
    );
    let state = build_api_state(
        storage,
        config.ai_providers,
        config.opend,
        scheduler_status.clone(),
        build_opend_paper_broker,
    )
    .await?;
    start_automatic_scheduler(state.clone(), config.scheduler, scheduler_status);
    let app = build_router_with_cors(state, config.cors_allowed_origins);
    let listener = tokio::net::TcpListener::bind(config.address).await?;

    tracing::info!(
        address = %config.address,
        market_sentiment_configured,
        paper_broker_configured,
        scheduler_enabled = config.scheduler.enabled,
        "indexlink server started"
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;
    tracing::info!("indexlink server stopped");

    Ok(())
}

/// Spawn the safe periodic audit scheduler when enabled by local configuration.
///
/// The task reads each plan's monthly/weekly fixed-day set and creates at most one
/// server-sourced decision record per active plan and UTC day.
/// It never submits a broker order; an operator must still explicitly request paper submission.
fn start_automatic_scheduler(
    state: ApiState,
    config: SchedulerConfig,
    scheduler_status: SchedulerStatusHandle,
) {
    if !config.enabled {
        tracing::info!("automatic decision scheduler is disabled");
        return;
    }

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(config.tick_interval);
        loop {
            interval.tick().await;
            match indexlink_api::run_due_decisions(&state).await {
                Ok(summary) => {
                    scheduler_status.record_success(summary);
                    tracing::info!(
                        created = summary.created,
                        already_claimed = summary.already_claimed,
                        unavailable = summary.unavailable,
                        "automatic decision scheduler tick completed"
                    );
                }
                Err(error) => {
                    scheduler_status.record_failure();
                    tracing::error!(error = %error, "automatic decision scheduler tick failed")
                }
            }
        }
    });
}

/// Assemble production API state with optional server-deployed AI profiles and OpenD dependencies.
///
/// Without an OpenD configuration, the state keeps its local paper-only mock broker.
/// A configured OpenD session must initialize successfully before the server starts.
async fn build_api_state<F, Fut>(
    storage: SqliteStorage,
    ai_providers: Vec<AiProviderConfiguration>,
    opend: Option<OpenDConnectionConfig>,
    scheduler_status: SchedulerStatusHandle,
    build_broker: F,
) -> Result<ApiState, BrokerSetupError>
where
    F: FnOnce(OpenDConnectionConfig) -> Fut,
    Fut: Future<Output = Result<Arc<dyn BrokerClient>, BrokerSetupError>>,
{
    let state =
        ApiState::new(storage, env!("CARGO_PKG_VERSION")).with_scheduler_status(scheduler_status);
    let state = if ai_providers.is_empty() {
        state
    } else {
        let default_profile_id = ai_providers
            .iter()
            .find(|provider| provider.is_default)
            .expect("configuration requires exactly one default AI profile")
            .profile
            .id()
            .clone();
        let providers = ai_providers
            .into_iter()
            .map(|provider| {
                Arc::new(QwenClient::with_profile(provider.client, provider.profile))
                    as Arc<dyn AiProvider>
            })
            .collect();
        state
            .with_ai_evidence_providers(
                Arc::new(RssNewsSource::new()),
                providers,
                default_profile_id,
            )
            .expect("configuration pre-validates unique AI profiles")
    };
    match opend {
        Some(config) => {
            let market_data = OpenDMarketSignalProvider::new(config.host(), config.port())
                .map_err(BrokerSetupError::MarketData)?;
            Ok(state
                .with_market_data(Arc::new(market_data))
                .with_broker(build_broker(config).await?))
        }
        None => Ok(state),
    }
}

/// Connect and wrap the configured local OpenD session as the production paper broker.
async fn build_opend_paper_broker(
    config: OpenDConnectionConfig,
) -> Result<Arc<dyn BrokerClient>, BrokerSetupError> {
    let session = OpenDPaperSession::connect(&config)
        .await
        .map_err(BrokerSetupError::Session)?;
    let broker = OpenDPaperBroker::new(config, session).map_err(BrokerSetupError::Adapter)?;

    Ok(Arc::new(broker))
}

/// Safe startup error when an explicitly configured OpenD paper adapter cannot initialize.
#[derive(Debug, thiserror::Error)]
enum BrokerSetupError {
    /// The local OpenD endpoint could not become a read-only market-data provider.
    #[error("configured OpenD market-data provider could not be initialized")]
    MarketData(#[source] market_data::MarketDataError),
    /// The local OpenD session could not be initialized.
    #[error("configured OpenD paper broker is unavailable")]
    Session(#[source] OpenDSessionError),
    /// The validated OpenD session could not become a paper broker adapter.
    #[error("configured OpenD paper broker could not be initialized")]
    Adapter(#[source] BrokerError),
}

fn init_tracing() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,indexlink_server=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init()
}

#[cfg(test)]
mod tests {
    use std::{env, time::Duration};

    use ai_client::{AiConfig, AiProviderProfile};
    use async_trait::async_trait;
    use axum::{
        body::{to_bytes, Body},
        http::{header::CONTENT_TYPE, Request, StatusCode},
        response::Response,
        Router,
    };
    use broker::{BrokerOrderAck, BrokerOrderRequest, BrokerProvider};
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use super::*;

    /// Build an isolated local storage handle for composition-root tests.
    async fn storage() -> SqliteStorage {
        SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
            .await
            .expect("in-memory SQLite storage should connect")
    }

    /// Build a disabled status holder for composition tests that do not run a scheduler task.
    fn scheduler_status() -> SchedulerStatusHandle {
        SchedulerStatusHandle::new(false, 60)
    }

    /// Verify a full local startup applies migrations and accepts an injected graceful stop.
    #[tokio::test]
    async fn run_with_shutdown_starts_and_stops_an_in_memory_server() {
        let config = Config::from_lookup(|name| match name {
            "APP_HOST" => Some("127.0.0.1".to_owned()),
            "APP_PORT" => Some("0".to_owned()),
            "DATABASE_URL" => Some("sqlite::memory:".to_owned()),
            "SCHEDULER_ENABLED" => Some("false".to_owned()),
            _ => None,
        })
        .expect("isolated test server configuration should be valid");
        run_with_shutdown(config, async {}).await.unwrap();
    }

    /// Verify an enabled scheduler records a successful empty-plan audit tick.
    #[tokio::test]
    async fn enabled_scheduler_records_a_successful_tick() {
        let storage = storage().await;
        storage.migrate().await.unwrap();
        let status = SchedulerStatusHandle::new(true, 1);
        let state = ApiState::new(storage, "test").with_scheduler_status(status.clone());
        start_automatic_scheduler(
            state,
            SchedulerConfig {
                enabled: true,
                tick_interval: Duration::from_millis(1),
            },
            status.clone(),
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(status.snapshot().last_tick_at.is_some());
    }

    /// Verify disabled scheduling does not create a background execution task.
    #[test]
    fn disabled_scheduler_returns_without_running_work() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let state = ApiState::new(storage().await, "test");
            start_automatic_scheduler(
                state,
                SchedulerConfig {
                    enabled: false,
                    tick_interval: Duration::from_secs(1),
                },
                SchedulerStatusHandle::new(false, 1),
            );
        });
    }

    /// Build a credential-free configured provider for composition-root tests.
    fn ai_provider(id: &str, is_default: bool) -> AiProviderConfiguration {
        AiProviderConfiguration {
            profile: AiProviderProfile::new(
                ai_client::AiProviderProfileId::new(id).expect("static profile ID is valid"),
                ai_client::AiProviderId::new("test-ai").expect("static provider ID is valid"),
                format!("Test {id}"),
                "test-model".to_owned(),
                ai_client::AiProviderCapabilities::market_evidence_and_restricted_policy_drafts(),
            )
            .expect("static profile is valid"),
            client: AiConfig {
                api_key: "server-test-secret".to_owned(),
                model: "test-model".to_owned(),
                ..Default::default()
            },
            is_default,
        }
    }

    /// Verify the production composition root leaves sentiment unavailable without Qwen config.
    #[tokio::test]
    async fn build_api_state_leaves_market_sentiment_unconfigured_without_qwen() {
        let state = build_api_state(
            storage().await,
            Vec::new(),
            None,
            scheduler_status(),
            build_opend_paper_broker,
        )
        .await
        .expect("mock broker composition should be infallible");

        assert!(format!("{state:?}").contains("market_sentiment: None"));
    }

    /// Verify the production composition root injects configured profiles without exposing keys.
    #[tokio::test]
    async fn build_api_state_injects_multiple_ai_profiles_when_configured() {
        let state = build_api_state(
            storage().await,
            vec![ai_provider("reviewer", true), ai_provider("copilot", false)],
            None,
            scheduler_status(),
            build_opend_paper_broker,
        )
        .await
        .expect("configured AI composition should be infallible");
        let debug = format!("{state:?}");

        assert!(debug.contains("market_sentiment: Some(MarketSentimentDependencies)"));
        assert!(!debug.contains("server-test-secret"));
        let response = build_router_with_cors(state, Vec::new())
            .oneshot(
                Request::builder()
                    .uri("/ai/providers")
                    .body(Body::empty())
                    .expect("provider-list request should build"),
            )
            .await
            .expect("provider-list route should respond");
        assert_eq!(response.status(), StatusCode::OK);
        let providers = response_json(response).await;
        assert_eq!(providers["providers"].as_array().map(Vec::len), Some(2));
        assert_eq!(providers["providers"][0]["id"], "copilot");
        assert_eq!(providers["providers"][1]["id"], "reviewer");
    }

    /// Broker double used to prove the composition root replaces its default mock.
    #[derive(Debug)]
    struct UnavailableBroker;

    #[async_trait]
    impl BrokerClient for UnavailableBroker {
        async fn submit_order(
            &self,
            _request: BrokerOrderRequest,
        ) -> Result<BrokerOrderAck, BrokerError> {
            Err(BrokerError::Unavailable)
        }
    }

    /// Build a validated paper configuration without contacting its local endpoint.
    fn paper_config() -> OpenDConnectionConfig {
        OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", 11111)
            .expect("paper configuration should be valid")
    }

    /// Send a due decision-preview request with a paper order through an app router.
    async fn submit_decision_preview(
        app: Router,
        symbol: &str,
        quantity: &str,
        idempotency_key: &str,
    ) -> Response {
        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/investment-plans")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "name": "OpenD paper smoke",
                            "symbol": symbol,
                            "base_contribution": "100.00",
                            "currency": "USD",
                            "schedule_kind": "monthly",
                            "schedule_day": 15,
                            "max_single_execution": "100.00"
                        })
                        .to_string(),
                    ))
                    .expect("create request should build"),
            )
            .await
            .expect("create route should respond");
        assert_eq!(created.status(), StatusCode::CREATED);
        let created = response_json(created).await;
        let plan_id = created["id"]
            .as_str()
            .expect("created plan should have an ID");

        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/investment-plans/{plan_id}/decision-preview"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "day_of_month": 15,
                        "fundamental": {
                            "score": 0.5,
                            "cape_percentile": 0.5,
                            "erp_percentile": 0.5
                        },
                        "trend": {
                            "score": 0.5,
                            "ma_distance_percentile": 0.5,
                            "rsi_percentile": 0.5,
                            "vix_percentile": 0.5,
                            "regime": "neutral"
                        },
                        "paper_order": {
                            "idempotency_key": idempotency_key,
                            "side": "buy",
                            "order_type": "market",
                            "quantity": quantity
                        }
                    })
                    .to_string(),
                ))
                .expect("decision request should build"),
        )
        .await
        .expect("decision route should respond")
    }

    /// Verify a configured OpenD factory failure prevents server composition.
    #[tokio::test]
    async fn build_api_state_returns_session_error_when_opend_factory_fails() {
        let error = build_api_state(
            storage().await,
            Vec::new(),
            Some(paper_config()),
            scheduler_status(),
            |_| async {
                Err::<Arc<dyn BrokerClient>, _>(BrokerSetupError::Session(
                    OpenDSessionError::Unavailable,
                ))
            },
        )
        .await
        .expect_err("failed OpenD factory must prevent startup");

        assert!(matches!(
            error,
            BrokerSetupError::Session(OpenDSessionError::Unavailable)
        ));
    }

    /// Verify a configured broker factory replaces the default mock in the HTTP route.
    #[tokio::test]
    async fn build_api_state_uses_configured_broker_factory() {
        let storage = storage().await;
        storage
            .migrate()
            .await
            .expect("in-memory SQLite migrations should apply");
        let state = build_api_state(
            storage,
            Vec::new(),
            Some(paper_config()),
            scheduler_status(),
            |_| async {
                Ok::<Arc<dyn BrokerClient>, BrokerSetupError>(Arc::new(UnavailableBroker))
            },
        )
        .await
        .expect("configured factory should compose");
        let response = submit_decision_preview(
            build_router_with_cors(state, Vec::new()),
            "VOO",
            "1.00",
            "configured-broker-factory-test",
        )
        .await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    /// Read an explicitly supplied smoke value without adding a default real order.
    fn smoke_value(name: &str) -> String {
        env::var(name).unwrap_or_else(|_| panic!("{name} must be set for the real OpenD smoke"))
    }

    /// Decode a JSON HTTP response in the manual real-OpenD smoke test.
    async fn response_json(response: axum::response::Response) -> Value {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("smoke response body should be readable");
        serde_json::from_slice(&body).expect("smoke response should be JSON")
    }

    /// Submit one explicitly confirmed order through the production OpenD server wiring.
    ///
    /// This test is ignored by default because it creates a real virtual-account
    /// order. It requires a locally authenticated OpenD process, `OPEND_PROVIDER`,
    /// an explicit paper-account selection, a unique idempotency key, symbol,
    /// quantity, and `OPEND_SMOKE_CONFIRM=submit-paper-order`.
    #[tokio::test]
    #[ignore = "requires an explicitly confirmed local OpenD paper order"]
    async fn real_opend_paper_order_smoke() {
        assert_eq!(
            env::var("OPEND_SMOKE_CONFIRM").as_deref(),
            Ok("submit-paper-order"),
            "set OPEND_SMOKE_CONFIRM=submit-paper-order to acknowledge a real paper order"
        );
        let config = Config::from_env().expect("server configuration should be valid");
        let opend = config
            .opend
            .expect("OPEND_PROVIDER must configure the real paper broker");
        assert!(
            opend.account_id().is_some(),
            "OPEND_ACCOUNT_ID must explicitly select one paper account for the smoke"
        );
        let symbol = smoke_value("OPEND_SMOKE_SYMBOL");
        let quantity = smoke_value("OPEND_SMOKE_QUANTITY");
        let idempotency_key = smoke_value("OPEND_SMOKE_IDEMPOTENCY_KEY");
        let storage = storage().await;
        storage
            .migrate()
            .await
            .expect("in-memory SQLite migrations should apply");
        let app = build_router_with_cors(
            build_api_state(
                storage,
                Vec::new(),
                Some(opend),
                scheduler_status(),
                build_opend_paper_broker,
            )
            .await
            .expect("local OpenD paper broker should initialize"),
            Vec::new(),
        );
        let response = submit_decision_preview(app, &symbol, &quantity, &idempotency_key).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = response_json(response).await;

        assert_eq!(response["paper_order_ack"]["environment"], "paper");
        assert_eq!(response["paper_order_ack"]["status"], "accepted");
        assert!(response["paper_order_ack"]["order_id"].is_string());
    }
}
