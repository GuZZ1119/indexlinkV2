//! SQLite 本地存储基础设施与 migration runner。

use std::{str::FromStr, time::Duration};

use rust_decimal::Decimal;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};

use crate::StorageError;

static SQLITE_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations/sqlite");

const SQLITE_AMOUNT_INTEGER_DIGITS: usize = 12;
const SQLITE_AMOUNT_SCALE: u32 = 8;

/// 将正 Decimal 编码为 SQLite schema 所需的固定宽度文本。
pub(crate) fn encode_amount(value: Decimal) -> Option<String> {
    if value <= Decimal::ZERO {
        return None;
    }

    let mut normalized = value;
    normalized.rescale(SQLITE_AMOUNT_SCALE);
    if normalized != value {
        return None;
    }
    let rendered = normalized.to_string();
    let (integer, fractional) = rendered.split_once('.')?;
    if integer.len() > SQLITE_AMOUNT_INTEGER_DIGITS
        || fractional.len() != SQLITE_AMOUNT_SCALE as usize
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || !fractional.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }

    let padding = "0".repeat(SQLITE_AMOUNT_INTEGER_DIGITS - integer.len());
    Some(format!("{padding}{integer}.{fractional}"))
}

/// 将 SQLite schema 的固定宽度金额文本解码为 Decimal。
pub(crate) fn decode_amount(value: &str) -> Option<Decimal> {
    let decimal = value.parse().ok()?;
    (encode_amount(decimal).as_deref() == Some(value)).then_some(decimal)
}

/// SQLite 本地存储连接。
///
/// 此类型为本地 SQLite production wiring 提供连接、迁移与健康检查基础设施。
#[derive(Clone, Debug)]
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    /// 使用调用方提供的 SQLite 连接池构建存储句柄。
    #[must_use]
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 使用指定连接池参数连接本地 SQLite 数据库。
    ///
    /// 连接会自动创建尚不存在的数据库文件、启用外键约束，并设置 WAL journal 模式。
    ///
    /// # 错误
    ///
    /// 配置无效、URL 无效、连接超时或 SQLite 无法打开数据库时返回 [`StorageError`]。
    pub async fn connect_with_options(
        database_url: &str,
        max_connections: u32,
        connect_timeout: Duration,
    ) -> Result<Self, StorageError> {
        if max_connections == 0 {
            return Err(StorageError::InvalidConfiguration(
                "database max connections must be greater than zero",
            ));
        }
        if connect_timeout.is_zero() {
            return Err(StorageError::InvalidConfiguration(
                "database connect timeout must be greater than zero",
            ));
        }

        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(StorageError::InvalidDatabaseUrl)?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(connect_timeout);
        let connect = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(connect_timeout)
            .connect_with(options);

        let pool = tokio::time::timeout(connect_timeout, connect)
            .await
            .map_err(|_| StorageError::ConnectionTimeout {
                seconds: connect_timeout.as_secs(),
            })?
            .map_err(StorageError::Connection)?;

        Ok(Self { pool })
    }

    /// 执行编译进二进制的全部 SQLite migration。
    ///
    /// 调用方应在开始提供 HTTP 服务前调用此方法；失败时返回
    /// [`StorageError::Migration`]，以阻止服务运行在不完整 schema 上。
    pub async fn migrate(&self) -> Result<(), StorageError> {
        SQLITE_MIGRATOR
            .run(&self.pool)
            .await
            .map_err(StorageError::Migration)
    }

    /// 检查 SQLite 是否可响应查询。
    ///
    /// # 错误
    ///
    /// 数据库不可用时返回 [`StorageError::Ping`]。
    pub async fn ping(&self) -> Result<(), StorageError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(StorageError::Ping)?;
        Ok(())
    }

    /// 返回底层 SQLite 连接池。
    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;

    /// 验证 migration 可在隔离的本地 SQLite 数据库中创建 MVP schema。
    #[tokio::test]
    async fn migration_creates_mvp_schema_in_memory() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .in_memory(true)
                    .foreign_keys(true),
            )
            .await
            .expect("in-memory SQLite pool must connect");
        let storage = SqliteStorage::from_pool(pool);

        storage
            .migrate()
            .await
            .expect("SQLite migration must apply");
        let tables = sqlx::query_scalar::<_, String>(
            "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
        )
        .fetch_all(storage.pool())
        .await
        .expect("SQLite schema tables must be queryable");

        assert_eq!(
            tables,
            vec![
                "_sqlx_migrations".to_owned(),
                "cash_flows".to_owned(),
                "decision_records".to_owned(),
                "investment_plans".to_owned(),
                "manual_execution_events".to_owned(),
                "market_price_bars".to_owned(),
                "market_price_datasets".to_owned(),
                "opportunity_cash_balances".to_owned(),
                "opportunity_cash_events".to_owned(),
                "paper_fills".to_owned(),
                "paper_orders".to_owned(),
                "plan_execution_configurations".to_owned(),
                "plan_period_execution_reservations".to_owned(),
                "portfolio_snapshots".to_owned(),
                "scheduled_decision_runs".to_owned(),
                "strategy_specs".to_owned(),
            ]
        );
    }

    /// 验证 SQLite 连接配置在接触数据库前拒绝零连接数。
    #[tokio::test]
    async fn connect_rejects_zero_max_connections() {
        let error =
            SqliteStorage::connect_with_options("sqlite::memory:", 0, Duration::from_secs(1))
                .await
                .expect_err("zero max connections must fail");

        assert!(matches!(error, StorageError::InvalidConfiguration(_)));
    }

    /// 验证 SQLite schema 拒绝非规范金额、错误金额关系、非 UTC 时间和空必填快照。
    #[tokio::test]
    async fn schema_enforces_amount_timestamp_and_snapshot_invariants() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .in_memory(true)
                    .foreign_keys(true),
            )
            .await
            .expect("in-memory SQLite pool must connect");
        let storage = SqliteStorage::from_pool(pool);
        storage
            .migrate()
            .await
            .expect("SQLite migration must apply");

        let invalid_amount = sqlx::query(
            "INSERT INTO investment_plans \
             (id, name, symbol, base_contribution, currency, schedule_day, max_single_execution) \
             VALUES ('plan-1', 'Core plan', 'VOO', '1000.00', 'USD', 15, '000000001000.00000000')",
        )
        .execute(storage.pool())
        .await;
        assert!(invalid_amount.is_err());

        let invalid_relationship = sqlx::query(
            "INSERT INTO investment_plans \
             (id, name, symbol, base_contribution, currency, schedule_day, max_single_execution) \
             VALUES ('plan-2', 'Core plan', 'VOO', '000000001000.00000000', 'USD', 15, '000000000900.00000000')",
        )
        .execute(storage.pool())
        .await;
        assert!(invalid_relationship.is_err());

        let invalid_timestamp = sqlx::query(
            "INSERT INTO investment_plans \
             (id, name, symbol, base_contribution, currency, schedule_day, max_single_execution, created_at, updated_at) \
             VALUES ('plan-3', 'Core plan', 'VOO', '000000001000.00000000', 'USD', 15, '000000001000.00000000', '2026-07-15T01:02:03.456+00:00', '2026-07-15T01:02:03.456+00:00')",
        )
        .execute(storage.pool())
        .await;
        assert!(invalid_timestamp.is_err());

        sqlx::query(
            "INSERT INTO investment_plans \
             (id, name, symbol, base_contribution, currency, schedule_day, max_single_execution) \
             VALUES ('plan-4', 'Core plan', 'VOO', '000000001000.00000000', 'USD', 15, '000000001000.00000000')",
        )
        .execute(storage.pool())
        .await
        .expect("valid decision-record parent plan must persist");
        sqlx::query(
            "INSERT INTO plan_execution_configurations \
             (plan_id, schedule_kind, schedule_day, core_ratio_units, opportunity_ratio_units, risk_mode, opportunity_cash_policy, schedule_days_json) \
             VALUES ('plan-4', 'monthly', 15, 100000000, 0, 'fixed', 'expire_each_period', '[15]')",
        )
        .execute(storage.pool())
        .await
        .expect("valid execution configuration must persist");
        let invalid_execution_configuration = sqlx::query(
            "UPDATE plan_execution_configurations SET schedule_kind = 'weekly', schedule_day = 8 \
             WHERE plan_id = 'plan-4'",
        )
        .execute(storage.pool())
        .await;
        assert!(invalid_execution_configuration.is_err());
        let null_required_snapshot = sqlx::query(
            "INSERT INTO decision_records \
             (id, plan_id, symbol, currency, execution_status, execution_snapshot, \
              fundamental_snapshot, trend_snapshot, decision_snapshot, summary) \
             VALUES ('record-1', 'plan-4', 'VOO', 'USD', 'due', 'null', '{}', '{}', '{}', 'summary')",
        )
        .execute(storage.pool())
        .await;
        assert!(null_required_snapshot.is_err());

        sqlx::query(
            "INSERT INTO decision_records \
             (id, plan_id, symbol, currency, execution_status, execution_snapshot, \
              fundamental_snapshot, trend_snapshot, decision_snapshot, summary) \
             VALUES ('record-2', 'plan-4', 'VOO', 'USD', 'due', '{}', '{}', '{}', '{}', 'summary')",
        )
        .execute(storage.pool())
        .await
        .expect("valid decision record must persist");
        let null_required_snapshot_update = sqlx::query(
            "UPDATE decision_records SET trend_snapshot = 'null' WHERE id = 'record-2'",
        )
        .execute(storage.pool())
        .await;
        assert!(null_required_snapshot_update.is_err());
    }
}
