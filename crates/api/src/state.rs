use std::{
    fmt,
    sync::{Arc, Mutex},
};

use ai_client::{
    fetch_market_sentiment_report, AiEvidence, AiProvider, AiProviderProfile,
    AiProviderProfileError, AiProviderProfileId, AiProviderRegistry, NewsSource,
};
use async_trait::async_trait;
use broker::{
    BrokerClient, BrokerOrderAck, BrokerOrderRequest, MockBroker, PaperPortfolioSnapshot,
};
use builtin_policies::BuiltinPolicyResolver;
use chrono::Datelike;
use decision_records::{
    DecisionRecord, DecisionRecordListQuery, DecisionRecordRepository,
    DecisionRecordRepositoryError, DecisionRecordService,
};
use indexlink_storage::{
    OpportunityCashSettlementInput, PaperPerformance, PaperPerformanceError, PaperPerformancePlan,
    PaperPerformancePoint, PaperTradeMarker, SqliteDecisionRecordRepository,
    SqliteInvestmentPlanRepository, SqliteOpportunityCashRepository,
    SqlitePaperPerformanceRepository, SqlitePeriodExecutionRepository,
    SqliteScheduledDecisionRepository, SqliteStorage, SqliteStrategySpecRepository,
    StoredStrategySpec,
};
use investment_plans::InvestmentPlanService;
use market_data::{MarketDataError, MarketPricePoint, MarketSignalInput, MarketSignalProvider};
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use serde::Serialize;
use std::collections::BTreeMap;
use strategy_dsl::StrategySpec;

use crate::ApiError;

/// Shared, display-safe state for the in-process automatic decision scheduler.
///
/// The status intentionally records counters and timestamps only; it never contains plan input,
/// account IDs, provider credentials, or market payloads.
#[derive(Debug, Clone, Serialize)]
pub struct SchedulerStatus {
    /// Whether the server was configured to run periodic automatic decisions.
    pub enabled: bool,
    /// Configured tick interval in whole seconds.
    pub tick_interval_seconds: u64,
    /// UTC timestamp of the most recent completed scheduler tick.
    pub last_tick_at: Option<String>,
    /// Safe counters from the most recent successful tick.
    pub last_summary: Option<crate::ScheduledDecisionRunSummary>,
    /// UTC timestamp of the most recent failed scheduler tick.
    pub last_error_at: Option<String>,
}

impl SchedulerStatus {
    /// Build an initial status snapshot before the first scheduler tick.
    #[must_use]
    pub fn new(enabled: bool, tick_interval_seconds: u64) -> Self {
        Self {
            enabled,
            tick_interval_seconds,
            last_tick_at: None,
            last_summary: None,
            last_error_at: None,
        }
    }
}

/// Cloneable writer/reader for scheduler status shared between the server task and HTTP API.
#[derive(Clone, Debug)]
pub struct SchedulerStatusHandle(Arc<Mutex<SchedulerStatus>>);

impl SchedulerStatusHandle {
    /// Create one shared status holder with its configured scheduler settings.
    #[must_use]
    pub fn new(enabled: bool, tick_interval_seconds: u64) -> Self {
        Self(Arc::new(Mutex::new(SchedulerStatus::new(
            enabled,
            tick_interval_seconds,
        ))))
    }

    /// Capture a completed scheduler tick without retaining decision payloads.
    pub fn record_success(&self, summary: crate::ScheduledDecisionRunSummary) {
        if let Ok(mut status) = self.0.lock() {
            status.last_tick_at = Some(time::OffsetDateTime::now_utc().to_string());
            status.last_summary = Some(summary);
            status.last_error_at = None;
        }
    }

    /// Capture a failed scheduler tick without serializing its internal error detail.
    pub fn record_failure(&self) {
        if let Ok(mut status) = self.0.lock() {
            status.last_error_at = Some(time::OffsetDateTime::now_utc().to_string());
        }
    }

    /// Return a consistent display snapshot for the runtime-status API.
    #[must_use]
    pub fn snapshot(&self) -> SchedulerStatus {
        self.0
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| SchedulerStatus::new(false, 0))
    }
}

/// Display-safe configured dependency capabilities for the web runtime status page.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct RuntimeCapabilities {
    /// Whether automatic market-data input has been composed from OpenD.
    pub market_data_configured: bool,
    /// Whether a Qwen/news provider has been composed from local configuration.
    pub qwen_configured: bool,
    /// Credential-free AI profiles registered by the server operator.
    pub ai_provider_profiles: Vec<AiProviderProfile>,
    /// Whether the production OpenD paper broker replaced the local mock broker.
    pub paper_broker_configured: bool,
    /// Current in-process scheduler counters and timestamps.
    pub scheduler: SchedulerStatus,
}

enum ReadinessBackend {
    SqliteStorage(SqliteStorage),
    Custom(Arc<dyn ReadinessCheck>),
}

struct MarketSentimentDependencies {
    news_source: Arc<dyn NewsSource>,
    registry: AiProviderRegistry,
    providers: BTreeMap<AiProviderProfileId, Arc<dyn AiProvider>>,
    default_profile_id: AiProviderProfileId,
}

/// One real local-paper series belonging to a configured recurring holding.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ActualPerformanceSeries {
    /// Local holding identifier.
    pub plan_id: uuid::Uuid,
    /// User-facing holding name.
    pub name: String,
    /// Normalized symbol.
    pub symbol: String,
    /// Local snapshot points in chronological order.
    pub points: Vec<PaperPerformancePoint>,
}

/// Combined real local-paper trajectory across all configured recurring holdings.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ActualPerformance {
    /// Shared display currency. MVP only aggregates one currency at a time.
    pub currency: String,
    /// Per-holding trajectories.
    pub series: Vec<ActualPerformanceSeries>,
    /// Sum of per-holding adaptive values at each shared observation timestamp.
    pub total_points: Vec<PaperPerformancePoint>,
}

/// Read-only price history and local trade markers for one configured holding.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct HoldingPriceHistory {
    /// Local holding identifier.
    pub plan_id: uuid::Uuid,
    /// User-facing holding name.
    pub name: String,
    /// Normalized symbol.
    pub symbol: String,
    /// Actual OpenD daily closes in the requested window.
    pub prices: Vec<MarketPricePoint>,
    /// Locally confirmed paper fills within the requested window.
    pub trades: Vec<PaperTradeMarker>,
}

/// One monthly point from the transparent historical price-only DCA replay.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct HistoricalBacktestPoint {
    /// Last available trading date in the replay month.
    pub date: String,
    /// Value of equal scheduled contributions without adaptation.
    pub plain_dca_value: f64,
    /// Value of the same schedule after the documented price-distance adjustment.
    pub adaptive_value: f64,
}

/// One-year historical comparison of plain and adaptive contribution schedules.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct HistoricalBacktest {
    /// Shared display currency. MVP only aggregates one currency at a time.
    pub currency: String,
    /// Explains the exact first-version replay boundary without presenting it as realised return.
    pub methodology: &'static str,
    /// Monthly points, oldest first.
    pub points: Vec<HistoricalBacktestPoint>,
}

impl fmt::Debug for MarketSentimentDependencies {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("MarketSentimentDependencies")
    }
}

impl fmt::Debug for ReadinessBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SqliteStorage(_) => formatter.write_str("SqliteStorage"),
            Self::Custom(_) => formatter.write_str("CustomReadinessCheck"),
        }
    }
}

/// HTTP API 的共享应用状态。
#[derive(Clone)]
pub struct ApiState {
    readiness: Arc<ReadinessBackend>,
    plans: InvestmentPlanService,
    decision_records: DecisionRecordService,
    broker: Arc<dyn BrokerClient>,
    market_sentiment: Option<Arc<MarketSentimentDependencies>>,
    market_data: Option<Arc<dyn MarketSignalProvider>>,
    paper_performance: Option<SqlitePaperPerformanceRepository>,
    scheduled_decisions: Option<SqliteScheduledDecisionRepository>,
    opportunity_cash: Option<SqliteOpportunityCashRepository>,
    period_execution: Option<SqlitePeriodExecutionRepository>,
    strategy_specs: Option<SqliteStrategySpecRepository>,
    scheduler_status: SchedulerStatusHandle,
    paper_broker_configured: bool,
    policy_resolver: Arc<BuiltinPolicyResolver>,
    version: Arc<str>,
}

impl fmt::Debug for ApiState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApiState")
            .field("readiness", &self.readiness)
            .field("plans", &"InvestmentPlanService")
            .field("decision_records", &"DecisionRecordService")
            .field("broker", &"BrokerClient")
            .field("market_sentiment", &self.market_sentiment)
            .field(
                "market_data",
                &self.market_data.as_ref().map(|_| "MarketSignalProvider"),
            )
            .field("version", &self.version)
            .finish()
    }
}

impl ApiState {
    /// 使用生产 SQLite 本地存储构建应用状态。
    #[must_use]
    pub fn new(storage: SqliteStorage, version: impl Into<Arc<str>>) -> Self {
        let pool = storage.pool().clone();
        let plans =
            InvestmentPlanService::new(Arc::new(SqliteInvestmentPlanRepository::new(pool.clone())));
        let decision_records =
            DecisionRecordService::new(Arc::new(SqliteDecisionRecordRepository::new(pool.clone())));
        let scheduled_decisions = SqliteScheduledDecisionRepository::new(pool.clone());
        let opportunity_cash = SqliteOpportunityCashRepository::new(pool.clone());
        let period_execution = SqlitePeriodExecutionRepository::new(pool.clone());
        let strategy_specs = SqliteStrategySpecRepository::new(pool.clone());
        Self {
            readiness: Arc::new(ReadinessBackend::SqliteStorage(storage)),
            plans,
            decision_records,
            broker: Arc::new(MockBroker::paper_only()),
            market_sentiment: None,
            market_data: None,
            paper_performance: Some(SqlitePaperPerformanceRepository::new(pool)),
            scheduled_decisions: Some(scheduled_decisions),
            opportunity_cash: Some(opportunity_cash),
            period_execution: Some(period_execution),
            strategy_specs: Some(strategy_specs),
            scheduler_status: SchedulerStatusHandle::new(false, 0),
            paper_broker_configured: false,
            policy_resolver: Arc::new(BuiltinPolicyResolver::default()),
            version: version.into(),
        }
    }

    /// 使用可替换的 readiness 检查构建状态，供隔离测试和受控适配器使用。
    #[must_use]
    pub fn with_readiness(
        readiness: Arc<dyn ReadinessCheck>,
        version: impl Into<Arc<str>>,
    ) -> Self {
        Self::with_readiness_and_plans(
            readiness,
            InvestmentPlanService::new(Arc::new(UnavailableInvestmentPlans)),
            version,
        )
    }

    /// 使用可替换的 readiness 与 investment plan service 构建状态。
    #[must_use]
    pub fn with_readiness_and_plans(
        readiness: Arc<dyn ReadinessCheck>,
        plans: InvestmentPlanService,
        version: impl Into<Arc<str>>,
    ) -> Self {
        Self::with_readiness_plans_and_broker(
            readiness,
            plans,
            Arc::new(MockBroker::paper_only()),
            version,
        )
    }

    /// 使用可替换的 readiness、investment plan service 与 broker 构建状态。
    #[must_use]
    pub fn with_readiness_plans_and_broker(
        readiness: Arc<dyn ReadinessCheck>,
        plans: InvestmentPlanService,
        broker: Arc<dyn BrokerClient>,
        version: impl Into<Arc<str>>,
    ) -> Self {
        Self::with_readiness_plans_broker_and_decision_records(
            readiness,
            plans,
            broker,
            DecisionRecordService::new(Arc::new(UnavailableDecisionRecords)),
            version,
        )
    }

    /// 使用可替换的 readiness、计划、broker 与 decision record 服务构建状态。
    #[must_use]
    pub fn with_readiness_plans_broker_and_decision_records(
        readiness: Arc<dyn ReadinessCheck>,
        plans: InvestmentPlanService,
        broker: Arc<dyn BrokerClient>,
        decision_records: DecisionRecordService,
        version: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            readiness: Arc::new(ReadinessBackend::Custom(readiness)),
            plans,
            decision_records,
            broker,
            market_sentiment: None,
            market_data: None,
            paper_performance: None,
            scheduled_decisions: None,
            opportunity_cash: None,
            period_execution: None,
            strategy_specs: None,
            scheduler_status: SchedulerStatusHandle::new(false, 0),
            paper_broker_configured: false,
            policy_resolver: Arc::new(BuiltinPolicyResolver::default()),
            version: version.into(),
        }
    }

    /// 注入市场新闻源与 AI provider，启用真实市场情绪预览。
    ///
    /// provider 的凭据必须只由 server 配置层持有，不能进入 HTTP 请求、响应或审计快照。
    #[must_use]
    pub fn with_market_sentiment(
        self,
        news_source: Arc<dyn NewsSource>,
        provider: Arc<dyn AiProvider>,
    ) -> Self {
        let default_profile_id = provider.profile().id().clone();
        self.with_ai_evidence_providers(news_source, vec![provider], default_profile_id)
            .expect("one provider profile must register exactly once")
    }

    /// Inject server-deployed AI profiles for generic evidence generation.
    ///
    /// The registry accepts clients supplied only by the composition root. Its
    /// HTTP representation is credential-free metadata, and a user can select
    /// only a profile present in this exact registry.
    pub fn with_ai_evidence_providers(
        mut self,
        news_source: Arc<dyn NewsSource>,
        providers: Vec<Arc<dyn AiProvider>>,
        default_profile_id: AiProviderProfileId,
    ) -> Result<Self, AiProviderProfileError> {
        let mut registry = AiProviderRegistry::default();
        let mut configured_providers = BTreeMap::new();
        for provider in providers {
            let profile = provider.profile();
            let profile_id = profile.id().clone();
            registry.register(profile)?;
            configured_providers.insert(profile_id, provider);
        }
        if registry.get(&default_profile_id).is_none() {
            return Err(AiProviderProfileError::UnregisteredProfile);
        }
        self.market_sentiment = Some(Arc::new(MarketSentimentDependencies {
            news_source,
            registry,
            providers: configured_providers,
            default_profile_id,
        }));
        Ok(self)
    }

    /// 注入只读市场信号 provider，启用自动数据刷新。
    ///
    /// provider 只返回可审计的指标输入，不得持有交易账户、下单权限或任何密钥快照。
    #[must_use]
    pub fn with_market_data(mut self, provider: Arc<dyn MarketSignalProvider>) -> Self {
        self.market_data = Some(provider);
        self
    }

    /// 注入受配置保护的 broker 实现，替换默认的本地 mock broker。
    ///
    /// 生产装配只能传入已验证的 paper-only adapter；凭据和账户标识不得进入
    /// HTTP 请求、响应、审计快照或日志。
    #[must_use]
    pub fn with_broker(mut self, broker: Arc<dyn BrokerClient>) -> Self {
        self.broker = broker;
        self.paper_broker_configured = true;
        self
    }

    /// Inject the server-owned scheduler status holder used by the runtime-status endpoint.
    #[must_use]
    pub fn with_scheduler_status(mut self, scheduler_status: SchedulerStatusHandle) -> Self {
        self.scheduler_status = scheduler_status;
        self
    }

    /// Return a display-safe runtime capability snapshot without probing paid or trading APIs.
    #[must_use]
    pub(crate) fn runtime_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            market_data_configured: self.market_data.is_some(),
            qwen_configured: self.market_sentiment.is_some(),
            ai_provider_profiles: self.ai_provider_profiles(),
            paper_broker_configured: self.paper_broker_configured,
            scheduler: self.scheduler_status.snapshot(),
        }
    }

    /// 检查 API 依赖是否可用。
    pub(crate) async fn check_readiness(&self) -> Result<(), ReadinessError> {
        match self.readiness.as_ref() {
            ReadinessBackend::SqliteStorage(storage) => storage
                .ping()
                .await
                .map_err(|error| ReadinessError::new(error.to_string())),
            ReadinessBackend::Custom(check) => check.check().await,
        }
    }

    /// 返回运行中的服务版本。
    pub(crate) fn version(&self) -> &str {
        self.version.as_ref()
    }

    /// 返回 investment plan 应用服务。
    pub(crate) fn plans(&self) -> &InvestmentPlanService {
        &self.plans
    }

    /// 返回统一执行入口使用的内置策略 resolver。
    #[must_use]
    pub(crate) fn policy_resolver(&self) -> &BuiltinPolicyResolver {
        self.policy_resolver.as_ref()
    }

    /// 读取已保存的受限 DSL 策略版本；该入口不提供创建、激活或执行能力。
    pub(crate) async fn list_strategy_specs(&self) -> Result<Vec<StoredStrategySpec>, ApiError> {
        self.strategy_specs
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?
            .list()
            .await
            .map_err(|error| match error {
                indexlink_storage::StrategySpecRepositoryError::NotFound => ApiError::NotFound,
                indexlink_storage::StrategySpecRepositoryError::Unavailable => {
                    ApiError::ServiceUnavailable
                }
            })
    }

    /// 读取一个已保存的不可变 DSL 策略版本。
    pub(crate) async fn get_strategy_spec(
        &self,
        policy: &strategy_policy::PolicyRef,
    ) -> Result<StoredStrategySpec, ApiError> {
        self.strategy_specs
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?
            .get(policy)
            .await
            .map_err(|error| match error {
                indexlink_storage::StrategySpecRepositoryError::NotFound => ApiError::NotFound,
                indexlink_storage::StrategySpecRepositoryError::Unavailable => {
                    ApiError::ServiceUnavailable
                }
            })
    }

    /// 保存一份已通过领域校验的不可变受限 DSL 策略版本。
    pub(crate) async fn save_strategy_spec(
        &self,
        strategy: &StrategySpec,
    ) -> Result<StoredStrategySpec, ApiError> {
        self.strategy_specs
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?
            .save(strategy)
            .await
            .map_err(|_| ApiError::ServiceUnavailable)
    }

    /// 判断一个计划策略引用是否为内置策略或已保存的可执行 DSL 版本。
    pub(crate) async fn supports_plan_policy(
        &self,
        policy: &strategy_policy::PolicyRef,
    ) -> Result<bool, ApiError> {
        if self.policy_resolver().supports(policy) {
            return Ok(true);
        }
        match self.get_strategy_spec(policy).await {
            Ok(strategy) => {
                let strategy = strategy
                    .document
                    .into_strategy_spec()
                    .map_err(|_| ApiError::ServiceUnavailable)?;
                Ok(!strategy.has_fixed_opportunity_amount_action())
            }
            Err(ApiError::NotFound) => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// 对已保存 DSL 策略运行固定样本准入评估；内置策略不使用此研究门槛。
    pub(crate) async fn strategy_admission_report(
        &self,
        policy: &strategy_policy::PolicyRef,
    ) -> Result<strategy_evaluation::StrategyAdmissionReport, ApiError> {
        let stored = self.get_strategy_spec(policy).await?;
        let strategy = stored
            .document
            .into_strategy_spec()
            .map_err(|_| ApiError::ServiceUnavailable)?;
        strategy_evaluation::evaluate_strategy_admission(&strategy)
            .inspect_err(|error| tracing::error!(%error, "strategy admission evaluation failed"))
            .map_err(|_| ApiError::ServiceUnavailable)
    }

    /// 判断策略是否可被绑定到计划并进入统一执行入口。
    ///
    /// 内置版本已随二进制发布并具备各自的回归覆盖；用户保存的 DSL 版本必须
    /// 通过固定样本回测、预算和核心桶安全门槛，不能仅因结构合法就直接激活。
    pub(crate) async fn is_plan_policy_eligible_for_activation(
        &self,
        policy: &strategy_policy::PolicyRef,
    ) -> Result<bool, ApiError> {
        if self.policy_resolver().supports(policy) {
            return Ok(true);
        }
        if !self.supports_plan_policy(policy).await? {
            return Ok(false);
        }
        Ok(self.strategy_admission_report(policy).await?.eligible)
    }

    /// 返回受配置保护的 broker port。
    pub(crate) fn broker(&self) -> &dyn BrokerClient {
        self.broker.as_ref()
    }

    /// 从已配置的 paper broker 读取账户、持仓和订单快照。
    ///
    /// 读取失败仅对客户端返回统一不可用错误；OpenD 协议细节、账户标识和
    /// provider 错误文本只保留在服务端日志中。
    pub(crate) async fn paper_portfolio(&self) -> Result<PaperPortfolioSnapshot, ApiError> {
        self.broker
            .read_paper_portfolio()
            .await
            .inspect_err(|error| tracing::error!(%error, "paper portfolio refresh failed"))
            .map_err(|_| ApiError::ServiceUnavailable)
    }

    /// 保存一个由用户确认的本地模拟账户起始资金基准。
    pub(crate) async fn set_paper_opening_balance(
        &self,
        plan_id: uuid::Uuid,
        amount: rust_decimal::Decimal,
        occurred_at: &str,
    ) -> Result<(), ApiError> {
        self.plans().get(plan_id).await?;
        self.paper_performance
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?
            .set_opening_balance(plan_id, amount, occurred_at)
            .await
            .map_err(|error| match error {
                PaperPerformanceError::InvalidInput => ApiError::BadRequest,
                PaperPerformanceError::Unavailable => ApiError::ServiceUnavailable,
            })
    }

    /// 刷新并返回一个计划的本地模拟账户收益与对比曲线。
    pub(crate) async fn paper_performance(
        &self,
        plan_id: uuid::Uuid,
    ) -> Result<PaperPerformance, ApiError> {
        let plan = self.plans().get(plan_id).await?;
        let portfolio = self.paper_portfolio().await?;
        let performance = self
            .paper_performance
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?
            .refresh(
                &PaperPerformancePlan {
                    id: plan.id,
                    symbol: plan.symbol,
                    currency: plan.currency,
                    base_contribution: plan.base_contribution,
                },
                &portfolio,
            )
            .await
            .inspect_err(|error| tracing::error!(%error, "paper performance refresh failed"))
            .map_err(|_| ApiError::ServiceUnavailable)?;
        self.reconcile_execution_ledgers(plan_id).await?;
        Ok(performance)
    }

    /// Refresh every configured holding from one read-only paper-account snapshot and return
    /// their local trajectories plus an explicitly summed total line.
    pub(crate) async fn actual_performance(&self) -> Result<ActualPerformance, ApiError> {
        let plans = self.plans().list().await?;
        let active: Vec<_> = plans.into_iter().filter(|plan| plan.is_active).collect();
        let currency = active
            .first()
            .map(|plan| plan.currency.clone())
            .unwrap_or_else(|| "USD".to_owned());
        if active.iter().any(|plan| plan.currency != currency) {
            return Err(ApiError::BadRequest);
        }
        if active.is_empty() {
            return Ok(ActualPerformance {
                currency,
                series: Vec::new(),
                total_points: Vec::new(),
            });
        }
        let portfolio = self.paper_portfolio().await?;
        let repository = self
            .paper_performance
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        let mut series = Vec::with_capacity(active.len());
        for plan in active {
            repository
                .refresh(
                    &PaperPerformancePlan {
                        id: plan.id,
                        symbol: plan.symbol.clone(),
                        currency: plan.currency,
                        base_contribution: plan.base_contribution,
                    },
                    &portfolio,
                )
                .await
                .inspect_err(|error| tracing::error!(%error, "actual performance refresh failed"))
                .map_err(|_| ApiError::ServiceUnavailable)?;
            self.reconcile_execution_ledgers(plan.id).await?;
            series.push(ActualPerformanceSeries {
                plan_id: plan.id,
                name: plan.name,
                symbol: plan.symbol,
                points: repository
                    .history(plan.id)
                    .await
                    .map_err(|_| ApiError::ServiceUnavailable)?,
            });
        }
        let mut daily = BTreeMap::<String, BTreeMap<uuid::Uuid, PaperPerformancePoint>>::new();
        for item in &series {
            for point in &item.points {
                let day = point
                    .observed_at
                    .get(..10)
                    .unwrap_or(&point.observed_at)
                    .to_owned();
                // A manual refresh may create several same-day snapshots.  Keep the newest
                // point per holding so the aggregate is never a sum of duplicate states.
                daily
                    .entry(day)
                    .or_default()
                    .insert(item.plan_id, point.clone());
            }
        }
        let total_points = daily
            .into_iter()
            .map(|(day, per_plan)| {
                let (adaptive_value, plain_dca_value, net_contributions) =
                    per_plan.into_values().fold(
                        (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO),
                        |total, point| {
                            (
                                total.0 + point.adaptive_value,
                                total.1 + point.plain_dca_value,
                                total.2 + point.net_contributions,
                            )
                        },
                    );
                PaperPerformancePoint {
                    observed_at: format!("{day}T00:00:00.000Z"),
                    adaptive_value,
                    plain_dca_value,
                    net_contributions,
                }
            })
            .collect();
        Ok(ActualPerformance {
            currency,
            series,
            total_points,
        })
    }

    /// Return read-only price histories and local buy/sell markers for all active holdings.
    pub(crate) async fn holding_price_history(
        &self,
        lookback_days: i64,
    ) -> Result<Vec<HoldingPriceHistory>, ApiError> {
        let provider = self
            .market_data
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        let repository = self
            .paper_performance
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        let cutoff = chrono::Utc::now() - chrono::Duration::days(lookback_days);
        let mut output = Vec::new();
        for plan in self
            .plans()
            .list()
            .await?
            .into_iter()
            .filter(|plan| plan.is_active)
        {
            let prices = provider
                .fetch_price_history(&plan.symbol, lookback_days)
                .await
                .inspect_err(|error| tracing::error!(%error, symbol = %plan.symbol, "price history refresh failed"))
                .map_err(|_| ApiError::ServiceUnavailable)?;
            let trades = repository
                .trade_markers(plan.id)
                .await
                .map_err(|_| ApiError::ServiceUnavailable)?
                .into_iter()
                .filter(|trade| {
                    chrono::DateTime::parse_from_rfc3339(&trade.observed_at)
                        .is_ok_and(|at| at.with_timezone(&chrono::Utc) >= cutoff)
                })
                .collect();
            output.push(HoldingPriceHistory {
                plan_id: plan.id,
                name: plan.name,
                symbol: plan.symbol,
                prices,
                trades,
            });
        }
        Ok(output)
    }

    /// Simulate one historical year for all active holdings using actual OpenD prices.
    ///
    /// This first MVP replay deliberately does not invent unavailable historical AI output or
    /// macro snapshots.  It applies a bounded contribution adjustment from each symbol's
    /// real 200-day moving-average distance and compares it with the same-date plain schedule.
    pub(crate) async fn historical_backtest(&self) -> Result<HistoricalBacktest, ApiError> {
        let provider = self
            .market_data
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        let plans: Vec<_> = self
            .plans()
            .list()
            .await?
            .into_iter()
            .filter(|plan| plan.is_active)
            .collect();
        let currency = plans
            .first()
            .map(|plan| plan.currency.clone())
            .unwrap_or_else(|| "USD".to_owned());
        if plans.iter().any(|plan| plan.currency != currency) {
            return Err(ApiError::BadRequest);
        }
        let cutoff = chrono::Utc::now().date_naive() - chrono::Duration::days(366);
        let mut totals = BTreeMap::<String, (f64, f64)>::new();
        for plan in plans {
            let prices = provider
                .fetch_price_history(&plan.symbol, 365 * 3 + 1)
                .await
                .inspect_err(|error| tracing::error!(%error, symbol = %plan.symbol, "historical replay data refresh failed"))
                .map_err(|_| ApiError::ServiceUnavailable)?;
            let parsed: Vec<_> = prices
                .iter()
                .filter_map(|point| {
                    chrono::NaiveDate::parse_from_str(&point.date, "%Y-%m-%d")
                        .ok()
                        .map(|date| (date, point.close))
                })
                .collect();
            if parsed.len() < 201 {
                return Err(ApiError::ServiceUnavailable);
            }
            let mut monthly = BTreeMap::<(i32, u32), (usize, chrono::NaiveDate, f64)>::new();
            for (index, (date, close)) in parsed.iter().enumerate() {
                if *date >= cutoff && index >= 199 {
                    monthly.insert((date.year(), date.month()), (index, *date, *close));
                }
            }
            let mut plain_units = 0.0;
            let mut adaptive_units = 0.0;
            let base = plan
                .base_contribution
                .to_f64()
                .ok_or(ApiError::BadRequest)?;
            for (_, (index, date, close)) in monthly {
                let average = parsed[index + 1 - 200..=index]
                    .iter()
                    .map(|(_, value)| *value)
                    .sum::<f64>()
                    / 200.0;
                let distance = close / average - 1.0;
                let multiplier = (1.0 - distance * 2.5).clamp(0.5, 1.5);
                plain_units += base / close;
                adaptive_units += base * multiplier / close;
                let entry = totals
                    .entry(date.format("%Y-%m-%d").to_string())
                    .or_insert((0.0, 0.0));
                entry.0 += plain_units * close;
                entry.1 += adaptive_units * close;
            }
        }
        Ok(HistoricalBacktest {
            currency,
            methodology: "一年前开始的真实 OpenD 日线月度回放；普通定投每月固定投入，自适应定投仅按当月相对 MA200 距离在 0.5x–1.5x 调整。历史 AI 情绪与宏观快照未被伪造，因此这不是已实现收益，也不是完整 70/20/10 审计回放。",
            points: totals
                .into_iter()
                .map(|(date, (plain_dca_value, adaptive_value))| HistoricalBacktestPoint {
                    date,
                    plain_dca_value,
                    adaptive_value,
                })
                .collect(),
        })
    }

    /// 记录已被 broker 接受的订单意图，供后续只读对账生成本地成交账本。
    pub(crate) async fn record_accepted_paper_order(
        &self,
        plan_id: uuid::Uuid,
        decision_record_id: uuid::Uuid,
        acknowledgement: &BrokerOrderAck,
        request: &BrokerOrderRequest,
    ) -> Result<(), ApiError> {
        let Some(repository) = &self.paper_performance else {
            return Ok(());
        };
        repository
            .record_accepted_order(plan_id, decision_record_id, acknowledgement, request)
            .await
            .inspect_err(|error| tracing::error!(%error, order_id = %acknowledgement.order_id(), "accepted paper order was not added to local ledger"))
            .map_err(|_| ApiError::ServiceUnavailable)
    }

    /// 返回 decision record 应用服务。
    pub(crate) fn decision_records(&self) -> &DecisionRecordService {
        &self.decision_records
    }

    /// List only credential-free AI profiles deployed by this server.
    #[must_use]
    pub(crate) fn ai_provider_profiles(&self) -> Vec<AiProviderProfile> {
        self.market_sentiment
            .as_ref()
            .map_or_else(Vec::new, |dependencies| dependencies.registry.profiles())
    }

    /// 拉取新闻并调用默认已部署 AI profile 生成通用 AI 证据。
    pub(crate) async fn ai_evidence(&self) -> Result<AiEvidence, ApiError> {
        self.ai_evidence_for_profile(None).await
    }

    /// Generate generic evidence through one explicitly deployed profile only.
    ///
    /// An unknown profile is rejected before any network call. A profile choice
    /// changes only which evidence adapter is used; it does not authorize a
    /// policy, change a budget, or submit an order.
    pub(crate) async fn ai_evidence_for_profile(
        &self,
        requested_profile: Option<&str>,
    ) -> Result<AiEvidence, ApiError> {
        let dependencies = self
            .market_sentiment
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        let profile_id = match requested_profile {
            Some(value) => {
                AiProviderProfileId::new(value.to_owned()).map_err(|_| ApiError::BadRequest)?
            }
            None => dependencies.default_profile_id.clone(),
        };
        let provider = dependencies
            .providers
            .get(&profile_id)
            .ok_or(ApiError::BadRequest)?;
        fetch_market_sentiment_report(
            dependencies.news_source.as_ref(),
            provider.as_ref(),
        )
        .await
        .inspect_err(|error| tracing::error!(%error, profile_id = %profile_id, "AI evidence pipeline failed"))
        .map_err(Into::into)
    }

    /// 拉取一份自动市场信号输入，并在边界保留内部失败日志。
    pub(crate) async fn market_signal_input(
        &self,
        symbol: &str,
    ) -> Result<MarketSignalInput, ApiError> {
        let provider = self
            .market_data
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        provider
            .fetch(symbol)
            .await
            .inspect_err(|error| tracing::error!(%error, "market signal refresh failed"))
            .map_err(market_data_error)
    }

    /// 读取只含决策日及以前价格的日线序列，供受限 DSL Runtime 构造可审计技术证据。
    pub(crate) async fn market_price_history(
        &self,
        symbol: &str,
        lookback_days: i64,
    ) -> Result<Vec<MarketPricePoint>, ApiError> {
        let provider = self
            .market_data
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        provider
            .fetch_price_history(symbol, lookback_days)
            .await
            .inspect_err(
                |error| tracing::error!(%error, symbol, "DSL market history refresh failed"),
            )
            .map_err(|_| ApiError::ServiceUnavailable)
    }

    /// Return the newest trusted local close for safe budget-to-quantity conversion.
    pub(crate) async fn latest_market_price(&self, symbol: &str) -> Result<Decimal, ApiError> {
        let provider = self
            .market_data
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        let point = provider
            .fetch_price_history(symbol, 7)
            .await
            .inspect_err(|error| tracing::error!(%error, symbol, "latest price refresh failed"))
            .map_err(|_| ApiError::ServiceUnavailable)?
            .into_iter()
            .last()
            .ok_or(ApiError::ServiceUnavailable)?;
        let price = Decimal::from_f64(point.close).ok_or(ApiError::ServiceUnavailable)?;
        (price > Decimal::ZERO)
            .then_some(price)
            .ok_or(ApiError::ServiceUnavailable)
    }

    /// Atomically claim one automatic decision run for a plan and UTC calendar day.
    ///
    /// This ledger is deliberately claimed immediately before record creation. Failed data
    /// refreshes remain retryable, while a persisted automatic decision cannot be duplicated
    /// by the next scheduler tick or after a process restart.
    pub(crate) async fn claim_scheduled_decision(
        &self,
        plan_id: uuid::Uuid,
        scheduled_for: &str,
    ) -> Result<bool, ApiError> {
        self.scheduled_decisions
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?
            .claim(plan_id, scheduled_for)
            .await
            .inspect_err(|error| tracing::error!(%error, plan_id = %plan_id, "scheduled decision claim failed"))
            .map_err(|_| ApiError::ServiceUnavailable)
    }

    /// Release an unpersisted scheduler claim so a later tick can retry it safely.
    pub(crate) async fn release_scheduled_decision(
        &self,
        plan_id: uuid::Uuid,
        scheduled_for: &str,
    ) {
        let Some(repository) = self.scheduled_decisions.as_ref() else {
            return;
        };
        if let Err(error) = repository.release(plan_id, scheduled_for).await {
            tracing::error!(%error, plan_id = %plan_id, scheduled_for, "scheduled decision claim release failed");
        }
    }

    /// Return the locally carried opportunity cash for one plan.
    pub(crate) async fn opportunity_cash_balance(
        &self,
        plan_id: uuid::Uuid,
    ) -> Result<Decimal, ApiError> {
        let Some(repository) = self.opportunity_cash.as_ref() else {
            return Ok(Decimal::ZERO);
        };
        repository
            .balance(plan_id)
            .await
            .inspect_err(|error| tracing::error!(%error, plan_id = %plan_id, "opportunity cash balance read failed"))
            .map_err(|_| ApiError::ServiceUnavailable)
    }

    /// Persist one accepted paper-order opportunity-cash settlement exactly once.
    pub(crate) async fn settle_opportunity_cash(
        &self,
        input: OpportunityCashSettlementInput<'_>,
    ) -> Result<(), ApiError> {
        let Some(repository) = self.opportunity_cash.as_ref() else {
            return Ok(());
        };
        repository
            .settle(input)
            .await
            .inspect_err(|error| tracing::error!(%error, plan_id = %input.plan_id, "opportunity cash settlement failed"))
            .map(|_| ())
            .map_err(|_| ApiError::ServiceUnavailable)
    }

    /// Atomically reserve the plan's configured weekly/monthly budget before broker submission.
    pub(crate) async fn reserve_period_execution(
        &self,
        plan_id: uuid::Uuid,
        decision_record_id: uuid::Uuid,
        period_key: &str,
        limit: Decimal,
        amount: Decimal,
    ) -> Result<bool, ApiError> {
        let Some(repository) = self.period_execution.as_ref() else {
            return Ok(true);
        };
        repository
            .reserve(plan_id, decision_record_id, period_key, limit, amount)
            .await
            .inspect_err(|error| tracing::error!(%error, plan_id = %plan_id, "period execution reservation failed"))
            .map_err(|_| ApiError::ServiceUnavailable)
    }

    /// Confirm that a reserved period amount reached the broker.
    pub(crate) async fn accept_period_execution(
        &self,
        decision_record_id: uuid::Uuid,
    ) -> Result<(), ApiError> {
        let Some(repository) = self.period_execution.as_ref() else {
            return Ok(());
        };
        repository
            .accept(decision_record_id)
            .await
            .map_err(|_| ApiError::ServiceUnavailable)
    }

    /// Release a period reservation when broker submission fails before acknowledgement.
    pub(crate) async fn release_period_execution(&self, decision_record_id: uuid::Uuid) {
        if let Some(repository) = self.period_execution.as_ref() {
            if let Err(error) = repository.release(decision_record_id).await {
                tracing::error!(%error, record_id = %decision_record_id, "period execution reservation release failed");
            }
        }
    }

    /// Apply terminal fills to the opportunity-cash and period-budget ledgers.
    async fn reconcile_execution_ledgers(&self, plan_id: uuid::Uuid) -> Result<(), ApiError> {
        if let Some(repository) = self.opportunity_cash.as_ref() {
            repository
                .reconcile_completed_fills(plan_id)
                .await
                .inspect_err(|error| tracing::error!(%error, plan_id = %plan_id, "opportunity cash fill reconciliation failed"))
                .map_err(|_| ApiError::ServiceUnavailable)?;
        }
        if let Some(repository) = self.period_execution.as_ref() {
            repository
                .reconcile_completed_orders(plan_id)
                .await
                .inspect_err(|error| tracing::error!(%error, plan_id = %plan_id, "period execution fill reconciliation failed"))
                .map_err(|_| ApiError::ServiceUnavailable)?;
        }
        Ok(())
    }
}

fn market_data_error(error: MarketDataError) -> ApiError {
    match error {
        MarketDataError::InvalidSymbol => ApiError::BadRequest,
        _ => ApiError::ServiceUnavailable,
    }
}

/// 可替换的服务就绪检查。
#[async_trait]
pub trait ReadinessCheck: Send + Sync {
    /// 检查依赖是否可用。
    async fn check(&self) -> Result<(), ReadinessError>;
}

/// 未配置计划存储时使用的显式不可用 repository。
struct UnavailableInvestmentPlans;

/// Fallback repository used when decision records are not configured in isolated tests.
struct UnavailableDecisionRecords;

#[async_trait]
impl investment_plans::InvestmentPlanRepository for UnavailableInvestmentPlans {
    async fn create(
        &self,
        _input: investment_plans::CreateInvestmentPlan,
    ) -> Result<investment_plans::InvestmentPlan, investment_plans::PlanRepositoryError> {
        Err(investment_plans::PlanRepositoryError::Unavailable)
    }

    async fn list(
        &self,
    ) -> Result<Vec<investment_plans::InvestmentPlan>, investment_plans::PlanRepositoryError> {
        Err(investment_plans::PlanRepositoryError::Unavailable)
    }

    async fn get(
        &self,
        _id: uuid::Uuid,
    ) -> Result<investment_plans::InvestmentPlan, investment_plans::PlanRepositoryError> {
        Err(investment_plans::PlanRepositoryError::Unavailable)
    }

    async fn update(
        &self,
        _id: uuid::Uuid,
        _input: investment_plans::UpdateInvestmentPlan,
    ) -> Result<investment_plans::InvestmentPlan, investment_plans::PlanRepositoryError> {
        Err(investment_plans::PlanRepositoryError::Unavailable)
    }

    async fn set_active(
        &self,
        _id: uuid::Uuid,
        _is_active: bool,
    ) -> Result<investment_plans::InvestmentPlan, investment_plans::PlanRepositoryError> {
        Err(investment_plans::PlanRepositoryError::Unavailable)
    }
}

#[async_trait]
impl DecisionRecordRepository for UnavailableDecisionRecords {
    /// Reject creates because no decision-record backend is configured.
    async fn create(
        &self,
        _input: decision_records::CreateDecisionRecord,
    ) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }

    /// Reject broker completions because no decision-record backend is configured.
    async fn complete_broker_order(
        &self,
        _id: uuid::Uuid,
        _input: decision_records::CompleteDecisionRecord,
    ) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }

    /// Reject list queries because no decision-record backend is configured.
    async fn list_by_plan(
        &self,
        _plan_id: uuid::Uuid,
        _query: DecisionRecordListQuery,
    ) -> Result<Vec<DecisionRecord>, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }

    /// Reject record lookups because no decision-record backend is configured.
    async fn get(&self, _id: uuid::Uuid) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }
}

/// readiness 检查的内部错误。
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ReadinessError {
    message: String,
}

impl ReadinessError {
    /// 创建内部 readiness 错误。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use async_trait::async_trait;
    use broker::{BrokerEnvironment, BrokerError, BrokerOrderRequest, BrokerOrderSide};
    use investment_plans::{
        BucketAllocationRatio, CreateInvestmentPlan, OpportunityCashPolicy,
        PlanExecutionConfiguration, PlanRiskMode, ScheduleKind, TwoBucketAllocationConfig,
    };
    use market_data::{MarketDataError, MarketPricePoint, MarketSignalInput, MarketSignalProvider};
    use rust_decimal::Decimal;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;

    struct SecretChecker {
        secret: &'static str,
    }

    /// Broker double that proves composition can replace the default mock safely.
    #[derive(Debug)]
    struct UnavailableBroker;

    /// Deterministic market-data adapter for state composition tests.
    struct StaticMarketData {
        failure: Option<StaticMarketFailure>,
        prices: Vec<MarketPricePoint>,
    }

    /// Public market-data error variants exercised by the test adapter.
    #[derive(Clone, Copy)]
    enum StaticMarketFailure {
        /// Simulate caller-supplied invalid symbols.
        InvalidSymbol,
        /// Simulate an unavailable local OpenD market-data dependency.
        Unavailable,
    }

    impl StaticMarketData {
        /// Build a deterministic successful adapter with an arbitrary price sequence.
        fn available(prices: Vec<MarketPricePoint>) -> Self {
            Self {
                failure: None,
                prices,
            }
        }

        /// Build an adapter that fails through the requested public error mapping.
        fn failing(error: StaticMarketFailure) -> Self {
            Self {
                failure: Some(error),
                prices: Vec::new(),
            }
        }
    }

    #[async_trait]
    impl ReadinessCheck for SecretChecker {
        async fn check(&self) -> Result<(), ReadinessError> {
            Err(ReadinessError::new(self.secret))
        }
    }

    #[async_trait]
    impl BrokerClient for UnavailableBroker {
        async fn submit_order(
            &self,
            _request: BrokerOrderRequest,
        ) -> Result<broker::BrokerOrderAck, BrokerError> {
            Err(BrokerError::Unavailable)
        }
    }

    #[async_trait]
    impl MarketSignalProvider for StaticMarketData {
        async fn fetch(&self, symbol: &str) -> Result<MarketSignalInput, MarketDataError> {
            if let Some(error) = self.failure {
                return Err(match error {
                    StaticMarketFailure::InvalidSymbol => MarketDataError::InvalidSymbol,
                    StaticMarketFailure::Unavailable => MarketDataError::OpenDUnavailable,
                });
            }
            let values = vec![1.0; 60];
            Ok(MarketSignalInput {
                symbol: symbol.to_ascii_uppercase(),
                as_of: "2026-08-27".to_owned(),
                cape_history: values.clone(),
                cape_current: 1.0,
                erp_history: values.clone(),
                erp_current: 1.0,
                ma_distance_history: values.clone(),
                ma_distance_current: 0.0,
                rsi_history: values.clone(),
                rsi_current: 50.0,
                vix_history: values,
                vix_current: 20.0,
                vix_as_of: "2026-08-27".to_owned(),
            })
        }

        async fn fetch_price_history(
            &self,
            _symbol: &str,
            _lookback_days: i64,
        ) -> Result<Vec<MarketPricePoint>, MarketDataError> {
            if let Some(error) = self.failure {
                return Err(match error {
                    StaticMarketFailure::InvalidSymbol => MarketDataError::InvalidSymbol,
                    StaticMarketFailure::Unavailable => MarketDataError::OpenDUnavailable,
                });
            }
            Ok(self.prices.clone())
        }
    }

    /// Build a valid plan input for migrated-state tests.
    fn plan_input() -> CreateInvestmentPlan {
        CreateInvestmentPlan {
            name: "State coverage ETF".to_owned(),
            symbol: "VOO".to_owned(),
            base_contribution: Decimal::new(100, 0),
            currency: "USD".to_owned(),
            schedule_kind: ScheduleKind::Monthly,
            schedule_day: 15,
            schedule_days: vec![15],
            policy: None,
            execution_configuration: PlanExecutionConfiguration::new_with_cash_policy(
                TwoBucketAllocationConfig::new(
                    BucketAllocationRatio::new(Decimal::new(80, 2)).unwrap(),
                    BucketAllocationRatio::new(Decimal::new(20, 2)).unwrap(),
                )
                .unwrap(),
                PlanRiskMode::Autopilot,
                OpportunityCashPolicy::CarryForward,
            )
            .unwrap(),
            max_single_execution: Decimal::new(150, 0),
        }
    }

    /// Create a migrated in-memory production state for repository composition tests.
    async fn migrated_state() -> ApiState {
        let storage =
            SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
                .await
                .unwrap();
        storage.migrate().await.unwrap();
        ApiState::new(storage, "coverage")
    }

    #[test]
    fn readiness_error_display_preserves_internal_diagnostic_for_logs() {
        let error = ReadinessError::new("database connection refused");

        assert_eq!(error.to_string(), "database connection refused");
    }

    #[test]
    fn custom_backend_debug_hides_checker_fields() {
        let state = ApiState::with_readiness(
            Arc::new(SecretChecker {
                secret: "private-checker-detail",
            }),
            "0.1.0",
        );
        let debug = format!("{state:?}");

        assert!(debug.contains("CustomReadinessCheck"));
        assert!(!debug.contains("private-checker-detail"));
        assert!(!debug.contains("secret"));
    }

    #[tokio::test]
    async fn sqlite_backend_debug_and_error_hide_pool_details() {
        let pool = SqlitePoolOptions::new()
            .connect_lazy_with(SqliteConnectOptions::new().filename("secret-database.sqlite"));
        pool.close().await;
        let state = ApiState::new(SqliteStorage::from_pool(pool), "0.1.0");
        let debug = format!("{state:?}");

        assert!(debug.contains("SqliteStorage"));
        assert!(!debug.contains("secret-database"));

        let error = state
            .check_readiness()
            .await
            .expect_err("closed pool must fail readiness");
        assert_eq!(error.to_string(), "database ping failed");
        assert!(!error.to_string().contains("secret"));
    }

    /// Verify a configured adapter replaces the local mock at the broker port.
    #[tokio::test]
    async fn with_broker_replaces_default_mock_broker() {
        let pool =
            SqlitePoolOptions::new().connect_lazy_with(SqliteConnectOptions::new().in_memory(true));
        let state = ApiState::new(SqliteStorage::from_pool(pool), "0.1.0")
            .with_broker(Arc::new(UnavailableBroker));
        let request = BrokerOrderRequest::market(
            "configured-broker-test",
            "VOO",
            BrokerOrderSide::Buy,
            Decimal::ONE,
            BrokerEnvironment::Paper,
        )
        .expect("paper order fixture should be valid");

        assert_eq!(
            state.broker().submit_order(request).await,
            Err(BrokerError::Unavailable)
        );
    }

    /// Verify absent optional adapters fail safely while local no-op ledgers remain deterministic.
    #[tokio::test]
    async fn isolated_state_maps_missing_optional_dependencies_safely() {
        let state = ApiState::with_readiness(Arc::new(SecretChecker { secret: "ready" }), "0.1.0");
        let plan_id = uuid::Uuid::new_v4();
        let record_id = uuid::Uuid::new_v4();

        assert!(state.check_readiness().await.is_err());
        assert_eq!(state.version(), "0.1.0");
        assert!(matches!(
            state.list_strategy_specs().await,
            Err(ApiError::ServiceUnavailable)
        ));
        assert!(matches!(
            state.ai_evidence().await,
            Err(ApiError::ServiceUnavailable)
        ));
        assert!(matches!(
            state.market_signal_input("VOO").await,
            Err(ApiError::ServiceUnavailable)
        ));
        assert!(matches!(
            state.market_price_history("VOO", 30).await,
            Err(ApiError::ServiceUnavailable)
        ));
        assert!(matches!(
            state.latest_market_price("VOO").await,
            Err(ApiError::ServiceUnavailable)
        ));
        assert!(matches!(
            state.paper_performance(plan_id).await,
            Err(ApiError::ServiceUnavailable)
        ));
        assert_eq!(
            state.opportunity_cash_balance(plan_id).await.unwrap(),
            Decimal::ZERO
        );
        assert!(state
            .reserve_period_execution(
                plan_id,
                record_id,
                "2026-08",
                Decimal::new(100, 0),
                Decimal::new(10, 0),
            )
            .await
            .unwrap());
        state
            .release_scheduled_decision(plan_id, "2026-08-27")
            .await;
        state.release_period_execution(record_id).await;
    }

    /// Verify market-data success and error mappings remain independent of HTTP routes.
    #[tokio::test]
    async fn market_data_injection_returns_prices_and_maps_provider_errors() {
        let prices = vec![
            MarketPricePoint {
                date: "2026-08-25".to_owned(),
                close: 99.5,
            },
            MarketPricePoint {
                date: "2026-08-26".to_owned(),
                close: 101.25,
            },
        ];
        let available =
            ApiState::with_readiness(Arc::new(SecretChecker { secret: "ready" }), "0.1.0")
                .with_market_data(Arc::new(StaticMarketData::available(prices.clone())));

        assert_eq!(
            available.market_signal_input("voo").await.unwrap().symbol,
            "VOO"
        );
        assert_eq!(
            available.market_price_history("VOO", 30).await.unwrap(),
            prices
        );
        assert_eq!(
            available.latest_market_price("VOO").await.unwrap(),
            Decimal::new(10125, 2)
        );

        let invalid =
            ApiState::with_readiness(Arc::new(SecretChecker { secret: "ready" }), "0.1.0")
                .with_market_data(Arc::new(StaticMarketData::failing(
                    StaticMarketFailure::InvalidSymbol,
                )));
        assert!(matches!(
            invalid.market_signal_input("bad symbol").await,
            Err(ApiError::BadRequest)
        ));
        assert!(matches!(
            invalid.market_price_history("VOO", 30).await,
            Err(ApiError::ServiceUnavailable)
        ));

        let unavailable =
            ApiState::with_readiness(Arc::new(SecretChecker { secret: "ready" }), "0.1.0")
                .with_market_data(Arc::new(StaticMarketData::failing(
                    StaticMarketFailure::Unavailable,
                )));
        assert!(matches!(
            unavailable.market_signal_input("VOO").await,
            Err(ApiError::ServiceUnavailable)
        ));
    }

    /// Verify migrated production wiring supports plans, ledgers and empty portfolio views.
    #[tokio::test]
    async fn migrated_state_composes_local_plan_and_execution_ledgers() {
        let state = migrated_state().await;
        let plan = state.plans().create(plan_input()).await.unwrap();
        let record_id = uuid::Uuid::new_v4();

        state.check_readiness().await.unwrap();
        assert!(state.supports_plan_policy(&plan.policy).await.unwrap());
        assert!(state
            .is_plan_policy_eligible_for_activation(&plan.policy)
            .await
            .unwrap());
        assert!(state
            .claim_scheduled_decision(plan.id, "2026-08-27")
            .await
            .unwrap());
        assert!(!state
            .claim_scheduled_decision(plan.id, "2026-08-27")
            .await
            .unwrap());
        state
            .release_scheduled_decision(plan.id, "2026-08-27")
            .await;
        assert!(state
            .claim_scheduled_decision(plan.id, "2026-08-27")
            .await
            .unwrap());

        state.accept_period_execution(record_id).await.unwrap();
        state.release_period_execution(record_id).await;
        assert_eq!(
            state.opportunity_cash_balance(plan.id).await.unwrap(),
            Decimal::ZERO
        );
        assert!(matches!(
            state.actual_performance().await,
            Err(ApiError::ServiceUnavailable)
        ));
    }

    /// Verify local price history and the transparent replay use the injected read-only source.
    #[tokio::test]
    async fn migrated_state_builds_holding_chart_and_historical_replay() {
        let prices = (0..460)
            .map(|offset| {
                let date = chrono::Utc::now().date_naive() - chrono::Duration::days(459 - offset);
                MarketPricePoint {
                    date: date.format("%Y-%m-%d").to_string(),
                    close: 100.0 + offset as f64 * 0.1,
                }
            })
            .collect();
        let state = migrated_state()
            .await
            .with_market_data(Arc::new(StaticMarketData::available(prices)));
        let plan = state.plans().create(plan_input()).await.unwrap();

        let holding = state.holding_price_history(365).await.unwrap();
        assert_eq!(holding.len(), 1);
        assert_eq!(holding[0].plan_id, plan.id);
        assert_eq!(holding[0].symbol, "VOO");
        assert!(holding[0].prices.len() >= 365);
        assert!(holding[0].trades.is_empty());

        let replay = state.historical_backtest().await.unwrap();
        assert_eq!(replay.currency, "USD");
        assert!(!replay.points.is_empty());
        assert!(replay
            .points
            .iter()
            .all(|point| point.plain_dca_value > 0.0 && point.adaptive_value > 0.0));
    }
}
