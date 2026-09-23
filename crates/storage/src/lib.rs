#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! SQLx 连接、migration 与 repository adapter 基础设施。
//!
//! 此 crate 负责连接池的建立、存活检查，以及面向领域 crate 的 outbound adapter。
//! V2.1 只支持本地 SQLite；未接入生产组合根的 PostgreSQL 草稿适配器已移出公开构建。

mod sqlite;
mod sqlite_decision_records;
mod sqlite_investment_plans;
mod sqlite_manual_executions;
mod sqlite_market_price_history;
mod sqlite_opportunity_cash;
mod sqlite_paper_performance;
mod sqlite_period_execution;
mod sqlite_scheduled_decisions;
mod sqlite_strategy_specs;

/// SQLite 本地存储连接与 migration runner。
pub use sqlite::SqliteStorage;
/// Decision Record repository 的 SQLite adapter。
pub use sqlite_decision_records::SqliteDecisionRecordRepository;
/// Investment Plan repository 的 SQLite adapter。
pub use sqlite_investment_plans::SqliteInvestmentPlanRepository;
/// Append-only SQLite journal adapter for user-reported execution events.
pub use sqlite_manual_executions::SqliteManualExecutionRepository;
/// SQLite canonical snapshot store for historical daily prices.
pub use sqlite_market_price_history::SqlitePriceHistoryStore;
/// SQLite local opportunity-bucket cash ledger adapter.
pub use sqlite_opportunity_cash::{
    OpportunityCashSettlement, OpportunityCashSettlementInput, SqliteOpportunityCashRepository,
};
/// SQLite local paper-trading performance ledger adapter.
pub use sqlite_paper_performance::{
    PaperPerformance, PaperPerformanceError, PaperPerformancePlan, PaperPerformancePoint,
    PaperTradeMarker, SqlitePaperPerformanceRepository,
};
/// SQLite atomic per-period execution-budget reservation adapter.
pub use sqlite_period_execution::SqlitePeriodExecutionRepository;
/// SQLite idempotency ledger for periodic automatic decision runs.
pub use sqlite_scheduled_decisions::SqliteScheduledDecisionRepository;
/// SQLite repository for immutable restricted DSL strategy versions.
pub use sqlite_strategy_specs::{
    SqliteStrategySpecRepository, StoredStrategySpec, StrategySpecRepositoryError,
};

/// 存储基础设施错误。
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// 连接池配置无效。
    #[error("invalid storage configuration: {0}")]
    InvalidConfiguration(&'static str),
    /// 数据库 URL 格式无效。
    #[error("database URL is invalid")]
    InvalidDatabaseUrl(#[source] sqlx::Error),
    /// 在配置的时限内未建立连接。
    #[error("database connection timed out after {seconds} seconds")]
    ConnectionTimeout {
        /// 超时秒数。
        seconds: u64,
    },
    /// 建立数据库连接失败。
    #[error("failed to connect to database")]
    Connection(#[source] sqlx::Error),
    /// 数据库存活检查失败。
    #[error("database ping failed")]
    Ping(#[source] sqlx::Error),
    /// 数据库 schema migration 执行失败。
    #[error("failed to apply database migrations")]
    Migration(#[source] sqlx::migrate::MigrateError),
}
