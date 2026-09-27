//! SQLite adapter for the investment-plan repository port.

use async_trait::async_trait;
use investment_plans::{
    BucketAllocationRatio, CreateInvestmentPlan, InvestmentPlan, InvestmentPlanRepository,
    OpportunityCashPolicy, PlanExecutionConfiguration, PlanRepositoryError, PlanRiskMode,
    PlanTimeZone, PlanValidationError, ScheduleKind, TwoBucketAllocationConfig,
    UpdateInvestmentPlan,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::sqlite::{decode_amount, encode_amount};

const RATIO_SCALE: u32 = 8;
const RATIO_UNITS: i64 = 100_000_000;

type ExecutionConfigurationValues = (
    i64,
    i64,
    PlanRiskMode,
    OpportunityCashPolicy,
    Option<String>,
    Option<String>,
);

const INSERT_PLAN_SQL: &str = "INSERT INTO investment_plans \
    (id, name, symbol, base_contribution, currency, schedule_kind, schedule_day, \
     max_single_execution, is_active) \
    VALUES (?1, ?2, ?3, ?4, ?5, 'monthly', ?6, ?7, 1)";
const INSERT_EXECUTION_CONFIGURATION_SQL: &str = "INSERT INTO plan_execution_configurations \
    (plan_id, schedule_kind, schedule_day, schedule_days_json, timezone, core_ratio_units, opportunity_ratio_units, risk_mode, opportunity_cash_policy, opportunity_cash_cap, period_execution_limit, policy_id, policy_version) \
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)";
const LIST_PLANS_SQL: &str = "SELECT p.id, p.name, p.symbol, p.base_contribution, p.currency, \
    c.schedule_kind, c.schedule_day, c.schedule_days_json, c.timezone, c.core_ratio_units, c.opportunity_ratio_units, c.risk_mode, c.opportunity_cash_policy, c.opportunity_cash_cap, c.period_execution_limit, c.policy_id, c.policy_version, \
    p.max_single_execution, p.is_active, p.created_at, p.updated_at \
    FROM investment_plans p JOIN plan_execution_configurations c ON c.plan_id = p.id \
    ORDER BY p.created_at ASC, p.id ASC";
const GET_PLAN_SQL: &str = "SELECT p.id, p.name, p.symbol, p.base_contribution, p.currency, \
    c.schedule_kind, c.schedule_day, c.schedule_days_json, c.timezone, c.core_ratio_units, c.opportunity_ratio_units, c.risk_mode, c.opportunity_cash_policy, c.opportunity_cash_cap, c.period_execution_limit, c.policy_id, c.policy_version, \
    p.max_single_execution, p.is_active, p.created_at, p.updated_at \
    FROM investment_plans p JOIN plan_execution_configurations c ON c.plan_id = p.id \
    WHERE p.id = ?1";
const SELECT_UPDATE_VALUES_SQL: &str = "SELECT p.base_contribution, p.max_single_execution, \
    p.schedule_day AS legacy_schedule_day, c.schedule_kind, c.schedule_day, c.schedule_days_json, c.core_ratio_units, \
    c.opportunity_ratio_units, c.risk_mode, c.opportunity_cash_policy, c.opportunity_cash_cap, c.period_execution_limit, c.policy_id, c.policy_version \
    FROM investment_plans p JOIN plan_execution_configurations c ON c.plan_id = p.id \
    WHERE p.id = ?1";
const UPDATE_PLAN_SQL: &str = "UPDATE investment_plans SET \
    name = COALESCE(?2, name), \
    base_contribution = COALESCE(?3, base_contribution), \
    schedule_day = COALESCE(?4, schedule_day), \
    max_single_execution = COALESCE(?5, max_single_execution), \
    is_active = COALESCE(?6, is_active), \
    updated_at = MAX( \
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), \
        strftime('%Y-%m-%dT%H:%M:%fZ', updated_at, '+0.001 seconds') \
    ) \
    WHERE id = ?1";
const UPDATE_EXECUTION_CONFIGURATION_SQL: &str = "UPDATE plan_execution_configurations SET \
    schedule_day = ?2, schedule_days_json = ?3, core_ratio_units = ?4, opportunity_ratio_units = ?5, risk_mode = ?6, opportunity_cash_policy = ?7, opportunity_cash_cap = ?8, period_execution_limit = ?9, policy_id = ?10, policy_version = ?11 \
    WHERE plan_id = ?1";
const SET_ACTIVE_SQL: &str = "UPDATE investment_plans SET \
    is_active = ?2, \
    updated_at = MAX( \
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), \
        strftime('%Y-%m-%dT%H:%M:%fZ', updated_at, '+0.001 seconds') \
    ) \
    WHERE id = ?1";
const DELETE_PLAN_SQL: &str = "DELETE FROM investment_plans WHERE id = ?1";

/// SQLite implementation of [`InvestmentPlanRepository`].
#[derive(Clone, Debug)]
pub struct SqliteInvestmentPlanRepository {
    pool: SqlitePool,
}

impl SqliteInvestmentPlanRepository {
    /// Build a repository from an existing SQLite pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InvestmentPlanRepository for SqliteInvestmentPlanRepository {
    /// Insert a normalized investment plan and return the persisted row.
    async fn create(
        &self,
        input: CreateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let base_contribution =
            encode_amount(input.base_contribution).ok_or(PlanRepositoryError::Unavailable)?;
        let max_single_execution =
            encode_amount(input.max_single_execution).ok_or(PlanRepositoryError::Unavailable)?;
        let id = Uuid::new_v4();
        let schedule_day = input.schedule_day;
        let schedule_days_json = encode_schedule_days(&input.schedule_days)?;
        let configuration = input.execution_configuration;
        let policy = input
            .policy
            .unwrap_or_else(investment_plans::default_fixed_dca_policy);
        let (
            core_ratio_units,
            opportunity_ratio_units,
            risk_mode,
            opportunity_cash_policy,
            opportunity_cash_cap,
            period_execution_limit,
        ) = execution_configuration_values(configuration)?;
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query(INSERT_PLAN_SQL)
            .bind(id.to_string())
            .bind(input.name)
            .bind(input.symbol)
            .bind(base_contribution)
            .bind(input.currency)
            .bind(schedule_day)
            .bind(max_single_execution)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query(INSERT_EXECUTION_CONFIGURATION_SQL)
            .bind(id.to_string())
            .bind(schedule_kind_name(input.schedule_kind))
            .bind(schedule_day)
            .bind(schedule_days_json)
            .bind(input.timezone.as_str())
            .bind(core_ratio_units)
            .bind(opportunity_ratio_units)
            .bind(risk_mode_name(risk_mode))
            .bind(opportunity_cash_policy_name(opportunity_cash_policy))
            .bind(opportunity_cash_cap)
            .bind(period_execution_limit)
            .bind(policy.id().as_str())
            .bind(i64::from(policy.version().value()))
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let row = sqlx::query(GET_PLAN_SQL)
            .bind(id.to_string())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;

        plan_from_row(row)
    }

    /// List plans in deterministic creation order.
    async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanRepositoryError> {
        let rows = sqlx::query(LIST_PLANS_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        rows.into_iter().map(plan_from_row).collect()
    }

    /// Fetch one plan by ID.
    async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanRepositoryError> {
        let row = sqlx::query(GET_PLAN_SQL)
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(PlanRepositoryError::NotFound)?;

        plan_from_row(row)
    }

    /// Merge, validate, and persist an update within one SQLite write transaction.
    async fn update(
        &self,
        id: Uuid,
        input: UpdateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(map_sqlx_error)?;
        let current = sqlx::query(SELECT_UPDATE_VALUES_SQL)
            .bind(id.to_string())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(PlanRepositoryError::NotFound)?;
        let current_base = decode_amount(
            current
                .try_get::<String, _>("base_contribution")
                .map_err(map_sqlx_error)?
                .as_str(),
        )
        .ok_or(PlanRepositoryError::Unavailable)?;
        let current_max = decode_amount(
            current
                .try_get::<String, _>("max_single_execution")
                .map_err(map_sqlx_error)?
                .as_str(),
        )
        .ok_or(PlanRepositoryError::Unavailable)?;
        let base = input.base_contribution.unwrap_or(current_base);
        let max = input.max_single_execution.unwrap_or(current_max);
        validate_final_amounts(base, max)?;
        let schedule_kind = schedule_kind_from_name(
            current
                .try_get::<String, _>("schedule_kind")
                .map_err(map_sqlx_error)?
                .as_str(),
        )?;
        let current_schedule_day = i16::try_from(
            current
                .try_get::<i64, _>("schedule_day")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| PlanRepositoryError::Unavailable)?;
        let current_schedule_days = decode_schedule_days(
            schedule_kind,
            current
                .try_get::<String, _>("schedule_days_json")
                .map_err(map_sqlx_error)?
                .as_str(),
        )?;
        let schedule_days = match input.schedule_days.clone() {
            Some(days) => normalize_schedule_days(schedule_kind, days)?,
            None => input
                .schedule_day
                .map(|day| normalize_schedule_days(schedule_kind, vec![day]))
                .transpose()?
                .unwrap_or(current_schedule_days),
        };
        let schedule_day = schedule_days[0];
        if input.schedule_days.is_none()
            && input.schedule_day.is_none()
            && schedule_day != current_schedule_day
        {
            return Err(PlanRepositoryError::Unavailable);
        }
        let current_configuration = execution_configuration_from_row(&current)?;
        let policy = input.policy.clone().unwrap_or(policy_from_row(&current)?);
        let bucket_allocation = input
            .bucket_allocation
            .unwrap_or(current_configuration.bucket_allocation());
        let risk_mode = input.risk_mode.unwrap_or(current_configuration.risk_mode());
        let opportunity_cash_policy = input
            .opportunity_cash_policy
            .unwrap_or(current_configuration.opportunity_cash_policy());
        let opportunity_cash_cap = if input.opportunity_cash_policy.is_some()
            && opportunity_cash_policy != OpportunityCashPolicy::CarryWithCap
        {
            None
        } else {
            input
                .opportunity_cash_cap
                .or(current_configuration.opportunity_cash_cap())
        };
        let period_execution_limit = input
            .period_execution_limit
            .or(current_configuration.period_execution_limit());
        let configuration = PlanExecutionConfiguration::new_with_limits(
            bucket_allocation,
            risk_mode,
            opportunity_cash_policy,
            opportunity_cash_cap,
            period_execution_limit,
        )?;
        let (
            core_ratio_units,
            opportunity_ratio_units,
            risk_mode,
            opportunity_cash_policy,
            opportunity_cash_cap,
            period_execution_limit,
        ) = execution_configuration_values(configuration)?;

        let base_contribution = input
            .base_contribution
            .map(|value| encode_amount(value).ok_or(PlanRepositoryError::Unavailable))
            .transpose()?;
        let max_single_execution = input
            .max_single_execution
            .map(|value| encode_amount(value).ok_or(PlanRepositoryError::Unavailable))
            .transpose()?;
        let is_active = input.is_active.map(i64::from);
        sqlx::query(UPDATE_PLAN_SQL)
            .bind(id.to_string())
            .bind(input.name)
            .bind(base_contribution)
            .bind(
                input
                    .schedule_day
                    .or(input.schedule_days.as_ref().map(|_| schedule_day)),
            )
            .bind(max_single_execution)
            .bind(is_active)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query(UPDATE_EXECUTION_CONFIGURATION_SQL)
            .bind(id.to_string())
            .bind(schedule_day)
            .bind(encode_schedule_days(&schedule_days)?)
            .bind(core_ratio_units)
            .bind(opportunity_ratio_units)
            .bind(risk_mode_name(risk_mode))
            .bind(opportunity_cash_policy_name(opportunity_cash_policy))
            .bind(opportunity_cash_cap)
            .bind(period_execution_limit)
            .bind(policy.id().as_str())
            .bind(i64::from(policy.version().value()))
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let row = sqlx::query(GET_PLAN_SQL)
            .bind(id.to_string())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;

        plan_from_row(row)
    }

    /// Persist the active flag through the dedicated toggle use case.
    async fn set_active(
        &self,
        id: Uuid,
        is_active: bool,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let result = sqlx::query(SET_ACTIVE_SQL)
            .bind(id.to_string())
            .bind(i64::from(is_active))
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        if result.rows_affected() == 0 {
            return Err(PlanRepositoryError::NotFound);
        }
        let row = sqlx::query(GET_PLAN_SQL)
            .bind(id.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        plan_from_row(row)
    }

    /// Delete one plan and rely on SQLite foreign-key cascades for its local records.
    async fn delete(&self, id: Uuid) -> Result<(), PlanRepositoryError> {
        let result = sqlx::query(DELETE_PLAN_SQL)
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        if result.rows_affected() == 0 {
            return Err(PlanRepositoryError::NotFound);
        }
        Ok(())
    }
}

/// 将 SQLite 查询结果转换为已验证的领域计划。
fn plan_from_row(row: SqliteRow) -> Result<InvestmentPlan, PlanRepositoryError> {
    let schedule_kind = schedule_kind_from_name(
        row.try_get::<String, _>("schedule_kind")
            .map_err(map_sqlx_error)?
            .as_str(),
    )?;
    let is_active = match row.try_get::<i64, _>("is_active").map_err(map_sqlx_error)? {
        0 => false,
        1 => true,
        _ => return Err(PlanRepositoryError::Unavailable),
    };

    let schedule_day = i16::try_from(
        row.try_get::<i64, _>("schedule_day")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| PlanRepositoryError::Unavailable)?;
    validate_schedule_day(schedule_kind, schedule_day)?;
    let schedule_days = decode_schedule_days(
        schedule_kind,
        row.try_get::<String, _>("schedule_days_json")
            .map_err(map_sqlx_error)?
            .as_str(),
    )?;
    if schedule_days[0] != schedule_day {
        return Err(PlanRepositoryError::Unavailable);
    }
    let timezone = PlanTimeZone::new(
        row.try_get::<String, _>("timezone")
            .map_err(map_sqlx_error)?
            .as_str(),
    )?;

    Ok(InvestmentPlan {
        id: parse_uuid(row.try_get("id").map_err(map_sqlx_error)?)?,
        name: row.try_get("name").map_err(map_sqlx_error)?,
        symbol: row.try_get("symbol").map_err(map_sqlx_error)?,
        base_contribution: parse_amount(row.try_get("base_contribution").map_err(map_sqlx_error)?)?,
        currency: row.try_get("currency").map_err(map_sqlx_error)?,
        schedule_kind,
        schedule_day,
        schedule_days,
        timezone,
        policy: policy_from_row(&row)?,
        execution_configuration: execution_configuration_from_row(&row)?,
        max_single_execution: parse_amount(
            row.try_get("max_single_execution")
                .map_err(map_sqlx_error)?,
        )?,
        is_active,
        created_at: parse_timestamp(row.try_get("created_at").map_err(map_sqlx_error)?)?,
        updated_at: parse_timestamp(row.try_get("updated_at").map_err(map_sqlx_error)?)?,
    })
}

/// 从 SQLite 行重建已校验、不可变的策略引用。
fn policy_from_row(row: &SqliteRow) -> Result<PolicyRef, PlanRepositoryError> {
    let id = PolicyId::new(
        row.try_get::<String, _>("policy_id")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| PlanRepositoryError::Unavailable)?;
    let version = u32::try_from(
        row.try_get::<i64, _>("policy_version")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| PlanRepositoryError::Unavailable)?;
    let version = PolicyVersion::new(version).map_err(|_| PlanRepositoryError::Unavailable)?;

    Ok(PolicyRef::new(id, version))
}

/// 将数据库文本解析为受支持的计划周期。
fn schedule_kind_from_name(value: &str) -> Result<ScheduleKind, PlanRepositoryError> {
    match value {
        "monthly" => Ok(ScheduleKind::Monthly),
        "weekly" => Ok(ScheduleKind::Weekly),
        _ => Err(PlanRepositoryError::Unavailable),
    }
}

/// 将计划周期编码为 SQLite 配置表使用的稳定文本。
fn schedule_kind_name(value: ScheduleKind) -> &'static str {
    match value {
        ScheduleKind::Monthly => "monthly",
        ScheduleKind::Weekly => "weekly",
    }
}

/// 将风险模式编码为 SQLite 配置表使用的稳定文本。
fn risk_mode_name(value: PlanRiskMode) -> &'static str {
    match value {
        PlanRiskMode::Fixed => "fixed",
        PlanRiskMode::Autopilot => "autopilot",
        PlanRiskMode::Approval => "approval",
    }
}

/// 将机会桶现金策略编码为 SQLite 配置表使用的稳定文本。
fn opportunity_cash_policy_name(value: OpportunityCashPolicy) -> &'static str {
    match value {
        OpportunityCashPolicy::ExpireEachPeriod => "expire_each_period",
        OpportunityCashPolicy::CarryForward => "carry_forward",
        OpportunityCashPolicy::CarryWithCap => "carry_with_cap",
    }
}

/// 从 SQLite 行重建受校验的双桶及风险模式配置。
fn execution_configuration_from_row(
    row: &SqliteRow,
) -> Result<PlanExecutionConfiguration, PlanRepositoryError> {
    let core_ratio = decode_ratio_units(
        row.try_get::<i64, _>("core_ratio_units")
            .map_err(map_sqlx_error)?,
    )?;
    let opportunity_ratio = decode_ratio_units(
        row.try_get::<i64, _>("opportunity_ratio_units")
            .map_err(map_sqlx_error)?,
    )?;
    let bucket_allocation = TwoBucketAllocationConfig::new(core_ratio, opportunity_ratio)?;
    let risk_mode = match row
        .try_get::<String, _>("risk_mode")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "fixed" => PlanRiskMode::Fixed,
        "autopilot" => PlanRiskMode::Autopilot,
        "approval" => PlanRiskMode::Approval,
        _ => return Err(PlanRepositoryError::Unavailable),
    };
    let opportunity_cash_policy = match row
        .try_get::<String, _>("opportunity_cash_policy")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "expire_each_period" => OpportunityCashPolicy::ExpireEachPeriod,
        "carry_forward" => OpportunityCashPolicy::CarryForward,
        "carry_with_cap" => OpportunityCashPolicy::CarryWithCap,
        _ => return Err(PlanRepositoryError::Unavailable),
    };
    let opportunity_cash_cap = match row
        .try_get::<Option<String>, _>("opportunity_cash_cap")
        .map_err(map_sqlx_error)?
    {
        Some(value) => Some(decode_amount(&value).ok_or(PlanRepositoryError::Unavailable)?),
        None => None,
    };
    let period_execution_limit = match row
        .try_get::<Option<String>, _>("period_execution_limit")
        .map_err(map_sqlx_error)?
    {
        Some(value) => Some(decode_amount(&value).ok_or(PlanRepositoryError::Unavailable)?),
        None => None,
    };
    PlanExecutionConfiguration::new_with_limits(
        bucket_allocation,
        risk_mode,
        opportunity_cash_policy,
        opportunity_cash_cap,
        period_execution_limit,
    )
    .map_err(Into::into)
}

/// 编码计划执行配置，拒绝无法被 SQLite 以固定八位精度保存的比例。
fn execution_configuration_values(
    configuration: PlanExecutionConfiguration,
) -> Result<ExecutionConfigurationValues, PlanRepositoryError> {
    let bucket_allocation = configuration.bucket_allocation();
    Ok((
        encode_ratio_units(bucket_allocation.core_ratio())?,
        encode_ratio_units(bucket_allocation.opportunity_ratio())?,
        configuration.risk_mode(),
        configuration.opportunity_cash_policy(),
        configuration
            .opportunity_cash_cap()
            .map(|value| encode_amount(value).ok_or(PlanRepositoryError::Unavailable))
            .transpose()?,
        configuration
            .period_execution_limit()
            .map(|value| encode_amount(value).ok_or(PlanRepositoryError::Unavailable))
            .transpose()?,
    ))
}

/// 将一个领域比例编码为 SQLite 的八位精度整数单位。
fn encode_ratio_units(value: BucketAllocationRatio) -> Result<i64, PlanRepositoryError> {
    let mut decimal = value.value();
    decimal.rescale(RATIO_SCALE);
    if decimal != value.value() {
        return Err(PlanRepositoryError::Unavailable);
    }
    let units = (decimal * Decimal::from(RATIO_UNITS))
        .to_i64()
        .ok_or(PlanRepositoryError::Unavailable)?;
    (0..=RATIO_UNITS)
        .contains(&units)
        .then_some(units)
        .ok_or(PlanRepositoryError::Unavailable)
}

/// 将 SQLite 固定精度整数单位解码为受校验的领域比例。
fn decode_ratio_units(value: i64) -> Result<BucketAllocationRatio, PlanRepositoryError> {
    if !(0..=RATIO_UNITS).contains(&value) {
        return Err(PlanRepositoryError::Unavailable);
    }
    BucketAllocationRatio::new(Decimal::new(value, RATIO_SCALE)).map_err(Into::into)
}

/// 根据当前计划周期校验更新后的执行日。
fn validate_schedule_day(kind: ScheduleKind, day: i16) -> Result<(), PlanRepositoryError> {
    match kind {
        ScheduleKind::Monthly if (1..=28).contains(&day) => Ok(()),
        ScheduleKind::Weekly if (1..=7).contains(&day) => Ok(()),
        ScheduleKind::Monthly => Err(PlanValidationError::InvalidScheduleDay.into()),
        ScheduleKind::Weekly => Err(PlanValidationError::InvalidWeeklyScheduleDay.into()),
    }
}

/// Validate and normalize schedule days at the persistence boundary.
fn normalize_schedule_days(
    kind: ScheduleKind,
    mut days: Vec<i16>,
) -> Result<Vec<i16>, PlanRepositoryError> {
    if days.is_empty() {
        return Err(PlanValidationError::InvalidScheduleDays.into());
    }
    let original_len = days.len();
    days.sort_unstable();
    days.dedup();
    if days.len() != original_len
        || days
            .iter()
            .any(|day| validate_schedule_day(kind, *day).is_err())
    {
        return Err(PlanValidationError::InvalidScheduleDays.into());
    }
    Ok(days)
}

/// Encode schedule days as canonical JSON for the local SQLite adapter.
fn encode_schedule_days(days: &[i16]) -> Result<String, PlanRepositoryError> {
    serde_json::to_string(days).map_err(|_| PlanRepositoryError::Unavailable)
}

/// Decode and validate a persisted schedule day array.
fn decode_schedule_days(kind: ScheduleKind, value: &str) -> Result<Vec<i16>, PlanRepositoryError> {
    let days = serde_json::from_str(value).map_err(|_| PlanRepositoryError::Unavailable)?;
    normalize_schedule_days(kind, days)
}

/// 解析数据库存储的 UUID 文本。
fn parse_uuid(value: String) -> Result<Uuid, PlanRepositoryError> {
    value.parse().map_err(|_| PlanRepositoryError::Unavailable)
}

/// 解析 schema 强制的固定精度金额文本。
fn parse_amount(value: String) -> Result<Decimal, PlanRepositoryError> {
    decode_amount(&value).ok_or(PlanRepositoryError::Unavailable)
}

/// 解析 schema 强制的 UTC RFC 3339 时间文本。
fn parse_timestamp(value: String) -> Result<OffsetDateTime, PlanRepositoryError> {
    OffsetDateTime::parse(&value, &Rfc3339).map_err(|_| PlanRepositoryError::Unavailable)
}

/// 校验合并更新后的最终金额关系。
fn validate_final_amounts(base: Decimal, max: Decimal) -> Result<(), PlanRepositoryError> {
    if base <= Decimal::ZERO {
        return Err(PlanValidationError::NonPositiveAmount {
            field: "base_contribution",
        }
        .into());
    }
    if max <= Decimal::ZERO {
        return Err(PlanValidationError::NonPositiveAmount {
            field: "max_single_execution",
        }
        .into());
    }
    if max < base {
        return Err(PlanValidationError::MaxBelowBaseContribution.into());
    }
    Ok(())
}

/// 将底层 SQLite 错误映射为安全的 repository 错误。
fn map_sqlx_error(error: sqlx::Error) -> PlanRepositoryError {
    match error {
        sqlx::Error::RowNotFound => PlanRepositoryError::NotFound,
        _ => PlanRepositoryError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::SqliteStorage;

    /// 创建测试用 Decimal。
    fn amount(value: &str) -> Decimal {
        value.parse().expect("test amount must parse")
    }

    /// 创建合法的测试计划输入。
    fn input() -> CreateInvestmentPlan {
        CreateInvestmentPlan {
            name: "Core plan".to_owned(),
            symbol: "VOO".to_owned(),
            base_contribution: amount("1000.00"),
            currency: "USD".to_owned(),
            schedule_kind: ScheduleKind::Monthly,
            schedule_day: 15,
            schedule_days: vec![15],
            timezone: PlanTimeZone::utc(),
            policy: None,
            execution_configuration: PlanExecutionConfiguration::default(),
            max_single_execution: amount("1500.00"),
        }
    }

    /// 创建已执行 migration 的隔离 SQLite repository。
    async fn repository() -> SqliteInvestmentPlanRepository {
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
        SqliteInvestmentPlanRepository::new(storage.pool().clone())
    }

    /// 验证 SQLite adapter 按固定精度编码金额并实现 create、list、get。
    #[tokio::test]
    async fn creates_lists_and_gets_plans() {
        let repository = repository().await;
        let created = repository.create(input()).await.unwrap();

        assert_eq!(created.base_contribution, amount("1000.00000000"));
        assert_eq!(created.max_single_execution, amount("1500.00000000"));
        assert_eq!(
            created.policy,
            investment_plans::default_fixed_dca_policy(),
            "new SQLite plans must bind the fixed DCA default explicitly"
        );
        assert_eq!(repository.list().await.unwrap(), vec![created.clone()]);
        assert_eq!(repository.get(created.id).await.unwrap(), created);
    }

    /// 验证已绑定计划可原子切换到受支持的兼容策略引用。
    #[tokio::test]
    async fn updates_persisted_policy_binding() {
        let repository = repository().await;
        let created = repository.create(input()).await.unwrap();
        let updated = repository
            .update(
                created.id,
                UpdateInvestmentPlan {
                    policy: Some(investment_plans::legacy_core_opportunity_v1_policy()),
                    ..UpdateInvestmentPlan::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(
            updated.policy,
            investment_plans::legacy_core_opportunity_v1_policy()
        );
        assert_eq!(
            repository.get(created.id).await.unwrap().policy,
            updated.policy
        );
    }

    /// 验证金额型机会滚存上限与周期执行上限可完整持久化。
    #[tokio::test]
    async fn persists_capped_carry_and_period_execution_limit() {
        let repository = repository().await;
        let mut plan = input();
        plan.execution_configuration = PlanExecutionConfiguration::new_with_limits(
            TwoBucketAllocationConfig::new(
                BucketAllocationRatio::new(amount("0.70")).unwrap(),
                BucketAllocationRatio::new(amount("0.30")).unwrap(),
            )
            .unwrap(),
            PlanRiskMode::Autopilot,
            OpportunityCashPolicy::CarryWithCap,
            Some(amount("500.00")),
            Some(amount("1200.00")),
        )
        .unwrap();
        let created = repository.create(plan).await.unwrap();
        assert_eq!(
            created.execution_configuration.opportunity_cash_cap(),
            Some(amount("500.00000000"))
        );
        assert_eq!(
            created.execution_configuration.period_execution_limit(),
            Some(amount("1200.00000000"))
        );
    }

    /// 验证 V1.1 周度、双桶及审批配置可在 SQLite 中完整往返。
    #[tokio::test]
    async fn persists_weekly_bucket_and_approval_configuration() {
        let repository = repository().await;
        let configuration = PlanExecutionConfiguration::new_with_cash_policy(
            TwoBucketAllocationConfig::new(
                BucketAllocationRatio::new(amount("0.75")).unwrap(),
                BucketAllocationRatio::new(amount("0.25")).unwrap(),
            )
            .unwrap(),
            PlanRiskMode::Approval,
            OpportunityCashPolicy::CarryForward,
        )
        .unwrap();
        let created = repository
            .create(CreateInvestmentPlan {
                schedule_kind: ScheduleKind::Weekly,
                schedule_day: 1,
                schedule_days: vec![1, 3, 5],
                timezone: PlanTimeZone::new("Australia/Sydney").unwrap(),
                execution_configuration: configuration,
                ..input()
            })
            .await
            .unwrap();

        assert_eq!(created.schedule_kind, ScheduleKind::Weekly);
        assert_eq!(created.schedule_day, 1);
        assert_eq!(created.schedule_days, vec![1, 3, 5]);
        assert_eq!(created.timezone.as_str(), "Australia/Sydney");
        assert_eq!(
            created
                .execution_configuration
                .bucket_allocation()
                .core_ratio()
                .value(),
            amount("0.75")
        );
        assert_eq!(
            created.execution_configuration.risk_mode(),
            PlanRiskMode::Approval
        );
        assert_eq!(
            created.execution_configuration.opportunity_cash_policy(),
            OpportunityCashPolicy::CarryForward
        );
    }

    /// 验证更新在同一 SQLite 写事务中校验最终金额组合。
    #[tokio::test]
    async fn update_preserves_atomic_amount_validation() {
        let repository = repository().await;
        let created = repository.create(input()).await.unwrap();

        let invalid = repository
            .update(
                created.id,
                UpdateInvestmentPlan {
                    base_contribution: Some(amount("2000.00")),
                    ..Default::default()
                },
            )
            .await;
        assert_eq!(
            invalid,
            Err(PlanRepositoryError::Validation(
                PlanValidationError::MaxBelowBaseContribution
            ))
        );
        assert_eq!(
            repository.get(created.id).await.unwrap().base_contribution,
            amount("1000.00000000")
        );

        let updated = repository
            .update(
                created.id,
                UpdateInvestmentPlan {
                    name: Some("Growth plan".to_owned()),
                    base_contribution: Some(amount("1200.00")),
                    schedule_day: Some(20),
                    schedule_days: None,
                    policy: None,
                    bucket_allocation: None,
                    risk_mode: None,
                    opportunity_cash_policy: None,
                    opportunity_cash_cap: None,
                    period_execution_limit: None,
                    max_single_execution: Some(amount("1800.00")),
                    is_active: Some(false),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "Growth plan");
        assert_eq!(updated.base_contribution, amount("1200.00000000"));
        assert_eq!(updated.schedule_day, 20);
        assert_eq!(updated.max_single_execution, amount("1800.00000000"));
        assert!(!updated.is_active);
    }

    /// 验证 SQLite adapter 持久化启停状态并正确处理金额精度边界。
    #[tokio::test]
    async fn toggles_activity_and_rejects_unrepresentable_amounts() {
        let repository = repository().await;
        let created = repository.create(input()).await.unwrap();

        let inactive = repository.set_active(created.id, false).await.unwrap();
        assert!(!inactive.is_active);
        let trailing_zero_precision = repository
            .create(CreateInvestmentPlan {
                base_contribution: amount("1.000000000"),
                max_single_execution: amount("2.000000000"),
                ..input()
            })
            .await
            .unwrap();
        assert_eq!(
            trailing_zero_precision.base_contribution,
            amount("1.00000000")
        );
        assert_eq!(
            trailing_zero_precision.max_single_execution,
            amount("2.00000000")
        );
        assert_eq!(
            repository
                .create(CreateInvestmentPlan {
                    base_contribution: amount("1.000000001"),
                    ..input()
                })
                .await,
            Err(PlanRepositoryError::Unavailable)
        );
    }

    /// 验证删除计划后 SQLite 不再返回该记录。
    #[tokio::test]
    async fn deletes_existing_plan_and_reports_missing_id() {
        let repository = repository().await;
        let created = repository.create(input()).await.unwrap();

        repository.delete(created.id).await.unwrap();

        assert_eq!(repository.list().await.unwrap(), Vec::new());
        assert_eq!(
            repository.delete(created.id).await,
            Err(PlanRepositoryError::NotFound)
        );
    }

    /// 验证固定精度金额编码拒绝零、负数和超出整数范围的值。
    #[test]
    fn amount_codec_enforces_sqlite_representation() {
        assert_eq!(
            encode_amount(amount("12.5")).as_deref(),
            Some("000000000012.50000000")
        );
        assert_eq!(
            decode_amount("000000000012.50000000"),
            Some(amount("12.50000000"))
        );
        assert_eq!(encode_amount(Decimal::ZERO), None);
        assert_eq!(encode_amount(amount("-1.00")), None);
        assert_eq!(
            encode_amount(amount("1.000000000")).as_deref(),
            Some("000000000001.00000000")
        );
        assert_eq!(encode_amount(amount("1.000000001")), None);
        assert_eq!(encode_amount(amount("1000000000000.00")), None);
    }

    /// 验证快速更新时仍保证 UTC 更新时间严格递增。
    #[tokio::test]
    async fn updates_advance_timestamp_even_when_clock_does_not() {
        let repository = repository().await;
        let created = repository.create(input()).await.unwrap();
        let future_timestamp = "2099-01-01T00:00:00.000Z";
        sqlx::query("UPDATE investment_plans SET updated_at = ?1 WHERE id = ?2")
            .bind(future_timestamp)
            .bind(created.id.to_string())
            .execute(&repository.pool)
            .await
            .expect("test timestamp override must succeed");
        let future_timestamp =
            OffsetDateTime::parse(future_timestamp, &Rfc3339).expect("test timestamp must parse");

        let updated = repository.set_active(created.id, false).await.unwrap();

        assert!(updated.updated_at > future_timestamp);
    }

    /// 验证 SQLite 错误和损坏金额快照映射为安全 repository 错误。
    #[test]
    fn maps_storage_failures_to_safe_repository_errors() {
        assert_eq!(
            map_sqlx_error(sqlx::Error::RowNotFound),
            PlanRepositoryError::NotFound
        );
        assert_eq!(
            map_sqlx_error(sqlx::Error::PoolClosed),
            PlanRepositoryError::Unavailable
        );
        assert_eq!(
            parse_amount("1000.00".to_owned()),
            Err(PlanRepositoryError::Unavailable)
        );
    }

    /// 验证所有 SQLite 查询保持静态并使用 SQLite 参数占位符。
    #[test]
    fn query_strings_are_static_and_sqlite_compatible() {
        for query in [
            INSERT_PLAN_SQL,
            LIST_PLANS_SQL,
            GET_PLAN_SQL,
            SELECT_UPDATE_VALUES_SQL,
            UPDATE_PLAN_SQL,
            SET_ACTIVE_SQL,
            DELETE_PLAN_SQL,
        ] {
            assert!(!query.contains('$'));
        }
        assert!(UPDATE_PLAN_SQL.contains("MAX("));
        assert!(SET_ACTIVE_SQL.contains("+0.001 seconds"));
    }
}
