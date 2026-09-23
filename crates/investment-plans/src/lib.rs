#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Investment Plan 领域与应用层基础。
//!
//! 本 crate 采用模块化单体内的轻量六边形边界：这里定义投资计划的领域模型、
//! 输入校验、执行预览、应用服务和 repository port；SQLite、Axum、Broker、AI、
//! Scheduler 与订单意图均属于外部 adapter。
//!
//! MVP 假设：单用户系统、无计划级 timezone、不验证 symbol 是否真实可交易、不生成
//! 任何真实订单。计划可持久化月度或周度配置，并可为同一周期保存多个固定执行日。
//!
//! 金额统一使用 [`rust_decimal::Decimal`]。HTTP/JSON 边界必须以字符串编码金额，
//! 避免 JavaScript Number 或 JSON 浮点转换造成精度损失。领域类型不直接实现
//! `Deserialize`；入站 adapter 应先反序列化到 DTO，再调用 `normalize()` 进入领域模型。

use std::sync::Arc;

use async_trait::async_trait;
use core_domain::{Action, Multiplier};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};
use time::OffsetDateTime;
use uuid::Uuid;

const MAX_NAME_LEN: usize = 100;
const MAX_SYMBOL_LEN: usize = 32;

/// 新计划默认使用的固定定投策略标识。
pub const FIXED_DCA_POLICY_ID: &str = "fixed_dca";
/// 新计划默认使用的固定定投策略版本。
pub const FIXED_DCA_POLICY_VERSION: u32 = 1;
/// 旧计划兼容使用的 70/20/10 策略标识。
pub const CORE_OPPORTUNITY_V1_POLICY_ID: &str = "core_opportunity_v1";
/// 旧计划兼容使用的 70/20/10 策略版本。
pub const CORE_OPPORTUNITY_V1_POLICY_VERSION: u32 = 1;

/// 返回新建计划默认绑定的固定定投策略版本。
#[must_use]
pub fn default_fixed_dca_policy() -> PolicyRef {
    PolicyRef::new(
        PolicyId::new(FIXED_DCA_POLICY_ID).expect("fixed DCA policy id is valid"),
        PolicyVersion::new(FIXED_DCA_POLICY_VERSION).expect("fixed DCA policy version is valid"),
    )
}

/// 返回迁移前已有计划应绑定的 Legacy 70/20/10 策略版本。
#[must_use]
pub fn legacy_core_opportunity_v1_policy() -> PolicyRef {
    PolicyRef::new(
        PolicyId::new(CORE_OPPORTUNITY_V1_POLICY_ID)
            .expect("legacy CoreOpportunityV1 policy id is valid"),
        PolicyVersion::new(CORE_OPPORTUNITY_V1_POLICY_VERSION)
            .expect("legacy CoreOpportunityV1 policy version is valid"),
    )
}

/// 投资计划双桶类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InvestmentBucket {
    /// 常规定投桶，用于稳定执行计划基准金额。
    Core,
    /// 机会桶，用于后续根据市场条件增加投入。
    Opportunity,
}

/// 双桶分配比例，范围为 0..=1。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct BucketAllocationRatio {
    /// 已校验的比例值。
    #[serde(with = "rust_decimal::serde::str")]
    value: Decimal,
}

impl BucketAllocationRatio {
    /// 创建已校验的双桶分配比例。
    pub fn new(value: Decimal) -> Result<Self, PlanValidationError> {
        if (Decimal::ZERO..=Decimal::ONE).contains(&value) {
            Ok(Self { value })
        } else {
            Err(PlanValidationError::InvalidBucketAllocationRatio)
        }
    }

    /// 返回已校验的比例值。
    #[must_use]
    pub fn value(self) -> Decimal {
        self.value
    }
}

/// 投资计划双桶分配配置。
///
/// 该配置只表达目标比例，不读取余额、不判断市场信号，也不生成本次分配金额。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TwoBucketAllocationConfig {
    /// 常规定投桶目标比例。
    core_ratio: BucketAllocationRatio,
    /// 机会桶目标比例。
    opportunity_ratio: BucketAllocationRatio,
}

impl TwoBucketAllocationConfig {
    /// 创建已校验的双桶分配配置。
    pub fn new(
        core_ratio: BucketAllocationRatio,
        opportunity_ratio: BucketAllocationRatio,
    ) -> Result<Self, PlanValidationError> {
        if core_ratio.value() + opportunity_ratio.value() == Decimal::ONE {
            Ok(Self {
                core_ratio,
                opportunity_ratio,
            })
        } else {
            Err(PlanValidationError::BucketAllocationRatiosMustSumToOne)
        }
    }

    /// 返回常规定投桶目标比例。
    #[must_use]
    pub fn core_ratio(self) -> BucketAllocationRatio {
        self.core_ratio
    }

    /// 返回机会桶目标比例。
    #[must_use]
    pub fn opportunity_ratio(self) -> BucketAllocationRatio {
        self.opportunity_ratio
    }

    /// 返回是否将全部计划金额分配到常规定投桶。
    #[must_use]
    pub fn is_core_only(self) -> bool {
        self.core_ratio.value() == Decimal::ONE
    }
}

/// 计划中机会桶的执行授权方式。
///
/// 该枚举只保存用户选择的策略边界；本 PR 不改变订单或 scheduler 的实际执行行为。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanRiskMode {
    /// 全部资金位于核心桶，只保留固定定投配置。
    Fixed,
    /// 机会桶由后续已审计的策略链路自动决定是否执行。
    Autopilot,
    /// 机会桶计算完成后必须由用户确认，后续才可提交 paper order。
    Approval,
}

/// 机会桶当期未使用金额的后续处理策略。
///
/// `CarryForward` 当前只作为已持久化的用户意图。实际现金池余额、滚存上限和账本
/// 流水将在后续执行账本阶段实现；本阶段不会伪造历史滚存余额。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpportunityCashPolicy {
    /// 当期未使用机会预算不参与后续自动补投。
    ExpireEachPeriod,
    /// 当期未使用机会预算计划在后续账本阶段滚存。
    CarryForward,
    /// 未使用机会预算滚存，但本地余额不得超过用户设置的金额上限。
    CarryWithCap,
}

/// 已校验的计划级双桶与风险模式配置。
///
/// 核心桶为 `100%` 时只能使用 [`PlanRiskMode::Fixed`]；存在机会桶时必须显式选择
/// [`PlanRiskMode::Autopilot`] 或 [`PlanRiskMode::Approval`]，避免隐式授权。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PlanExecutionConfiguration {
    /// 核心/机会桶的目标分配比例。
    bucket_allocation: TwoBucketAllocationConfig,
    /// 机会桶的执行授权方式。
    risk_mode: PlanRiskMode,
    /// 机会桶未使用金额的后续处理策略。
    opportunity_cash_policy: OpportunityCashPolicy,
    /// `carry_with_cap` 的本地机会现金余额上限。
    #[serde(
        default,
        with = "rust_decimal::serde::str_option",
        skip_serializing_if = "Option::is_none"
    )]
    opportunity_cash_cap: Option<Decimal>,
    /// 一个调度周期内所有已保留订单的合计金额上限。
    #[serde(
        default,
        with = "rust_decimal::serde::str_option",
        skip_serializing_if = "Option::is_none"
    )]
    period_execution_limit: Option<Decimal>,
}

impl PlanExecutionConfiguration {
    /// 创建已校验的计划级执行配置。
    pub fn new(
        bucket_allocation: TwoBucketAllocationConfig,
        risk_mode: PlanRiskMode,
    ) -> Result<Self, PlanValidationError> {
        Self::new_with_cash_policy(
            bucket_allocation,
            risk_mode,
            OpportunityCashPolicy::ExpireEachPeriod,
        )
    }

    /// 创建带机会桶未使用金额处理策略的已校验计划执行配置。
    pub fn new_with_cash_policy(
        bucket_allocation: TwoBucketAllocationConfig,
        risk_mode: PlanRiskMode,
        opportunity_cash_policy: OpportunityCashPolicy,
    ) -> Result<Self, PlanValidationError> {
        Self::new_with_limits(
            bucket_allocation,
            risk_mode,
            opportunity_cash_policy,
            None,
            None,
        )
    }

    /// 创建带机会现金与周期执行上限的已校验计划级执行配置。
    pub fn new_with_limits(
        bucket_allocation: TwoBucketAllocationConfig,
        risk_mode: PlanRiskMode,
        opportunity_cash_policy: OpportunityCashPolicy,
        opportunity_cash_cap: Option<Decimal>,
        period_execution_limit: Option<Decimal>,
    ) -> Result<Self, PlanValidationError> {
        if bucket_allocation.is_core_only()
            && opportunity_cash_policy != OpportunityCashPolicy::ExpireEachPeriod
        {
            return Err(PlanValidationError::CoreOnlyRequiresExpireEachPeriodPolicy);
        }
        if opportunity_cash_policy == OpportunityCashPolicy::CarryWithCap {
            let Some(cap) = opportunity_cash_cap else {
                return Err(PlanValidationError::CarryWithCapRequiresCashCap);
            };
            validate_positive("opportunity_cash_cap", cap)?;
        } else if opportunity_cash_cap.is_some() {
            return Err(PlanValidationError::CashCapRequiresCarryWithCapPolicy);
        }
        if let Some(limit) = period_execution_limit {
            validate_positive("period_execution_limit", limit)?;
        }

        match (bucket_allocation.is_core_only(), risk_mode) {
            (true, PlanRiskMode::Fixed)
            | (false, PlanRiskMode::Autopilot | PlanRiskMode::Approval) => Ok(Self {
                bucket_allocation,
                risk_mode,
                opportunity_cash_policy,
                opportunity_cash_cap,
                period_execution_limit,
            }),
            (true, _) => Err(PlanValidationError::CoreOnlyRequiresFixedRiskMode),
            (false, PlanRiskMode::Fixed) => {
                Err(PlanValidationError::OpportunityBucketRequiresRiskMode)
            }
        }
    }

    /// 返回计划的核心/机会桶目标比例。
    #[must_use]
    pub fn bucket_allocation(self) -> TwoBucketAllocationConfig {
        self.bucket_allocation
    }

    /// 返回机会桶的执行授权方式。
    #[must_use]
    pub fn risk_mode(self) -> PlanRiskMode {
        self.risk_mode
    }

    /// 返回机会桶当期未使用金额的处理策略。
    #[must_use]
    pub fn opportunity_cash_policy(self) -> OpportunityCashPolicy {
        self.opportunity_cash_policy
    }

    /// 返回机会现金滚存上限；仅 `carry_with_cap` 配置时存在。
    #[must_use]
    pub fn opportunity_cash_cap(self) -> Option<Decimal> {
        self.opportunity_cash_cap
    }

    /// 返回用户配置的周期累计执行金额上限。
    #[must_use]
    pub fn period_execution_limit(self) -> Option<Decimal> {
        self.period_execution_limit
    }

    /// 重新校验已构造配置，供持久化边界验证外部数据使用。
    pub fn validate(self) -> Result<(), PlanValidationError> {
        Self::new_with_limits(
            self.bucket_allocation,
            self.risk_mode,
            self.opportunity_cash_policy,
            self.opportunity_cash_cap,
            self.period_execution_limit,
        )
        .map(|_| ())
    }
}

impl Default for PlanExecutionConfiguration {
    fn default() -> Self {
        Self::new_with_limits(
            TwoBucketAllocationConfig::new(
                BucketAllocationRatio::new(Decimal::ONE).expect("one is a valid ratio"),
                BucketAllocationRatio::new(Decimal::ZERO).expect("zero is a valid ratio"),
            )
            .expect("core-only ratios must sum to one"),
            PlanRiskMode::Fixed,
            OpportunityCashPolicy::ExpireEachPeriod,
            None,
            None,
        )
        .expect("core-only configuration must be valid")
    }
}

/// 投资计划双桶投入拆分结果。
///
/// 该结果只表达本次计划投入金额在两个桶之间的拆分，不读取余额、不生成订单。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TwoBucketContributionSplit {
    /// 本周期可用的计划预算，已经受单次执行上限限制。
    #[serde(with = "rust_decimal::serde::str")]
    planned_contribution: Decimal,
    /// 常规定投桶投入金额。
    #[serde(with = "rust_decimal::serde::str")]
    core_contribution: Decimal,
    /// 机会桶的当期预算，尚未应用决策倍率。
    #[serde(with = "rust_decimal::serde::str")]
    opportunity_budget: Decimal,
    /// 机会桶实际采用的有界倍率。
    #[serde(with = "rust_decimal::serde::str")]
    opportunity_multiplier: Decimal,
    /// 已从此前接受订单滚存的机会现金。
    #[serde(with = "rust_decimal::serde::str")]
    carried_opportunity_cash: Decimal,
    /// 机会桶本次建议投入金额。
    #[serde(with = "rust_decimal::serde::str")]
    opportunity_contribution: Decimal,
    /// 机会桶本次未建议投入金额，不等同于已持久化的现金池余额。
    #[serde(with = "rust_decimal::serde::str")]
    unallocated_opportunity_contribution: Decimal,
    /// 核心桶与机会桶合计的本次建议金额。
    #[serde(with = "rust_decimal::serde::str")]
    recommended_contribution: Decimal,
    /// 机会桶未使用金额的后续处理策略。
    opportunity_cash_policy: OpportunityCashPolicy,
    /// 当前计划是否要求操作者明确提交 paper order 才能执行。
    requires_approval: bool,
}

impl TwoBucketContributionSplit {
    /// 按双桶配置拆分本次计划投入金额。
    pub fn new(
        planned_contribution: Decimal,
        config: TwoBucketAllocationConfig,
    ) -> Result<Self, PlanValidationError> {
        Self::baseline(
            planned_contribution,
            PlanExecutionConfiguration::new(
                config,
                if config.is_core_only() {
                    PlanRiskMode::Fixed
                } else {
                    PlanRiskMode::Autopilot
                },
            )?,
        )
    }

    /// 使用计划的持久化配置生成不带市场信号的基准拆分。
    pub fn baseline(
        planned_contribution: Decimal,
        configuration: PlanExecutionConfiguration,
    ) -> Result<Self, PlanValidationError> {
        validate_positive("planned_contribution", planned_contribution)?;
        let allocation = configuration.bucket_allocation();
        let core_contribution = planned_contribution * allocation.core_ratio().value();
        let opportunity_contribution = planned_contribution - core_contribution;

        Ok(Self {
            planned_contribution,
            core_contribution,
            opportunity_budget: opportunity_contribution,
            opportunity_multiplier: Decimal::ONE,
            carried_opportunity_cash: Decimal::ZERO,
            opportunity_contribution,
            unallocated_opportunity_contribution: Decimal::ZERO,
            recommended_contribution: planned_contribution,
            opportunity_cash_policy: configuration.opportunity_cash_policy(),
            requires_approval: configuration.risk_mode() == PlanRiskMode::Approval,
        })
    }

    /// 使用已计算的决策动作和倍率生成双桶执行建议。
    ///
    /// 核心桶始终按比例保留。`Skip` 与 `TacticalDelay` 只会将机会桶降为零，
    /// 不会否决核心桶。机会桶的建议金额不会超过其当期预算；后续现金池阶段
    /// 才会允许使用经审计的滚存余额实现额外加码。
    pub fn from_decision(
        planned_contribution: Decimal,
        configuration: PlanExecutionConfiguration,
        action: Action,
        multiplier: Multiplier,
    ) -> Result<Self, PlanValidationError> {
        Self::from_decision_with_carry(
            planned_contribution,
            planned_contribution,
            configuration,
            action,
            multiplier,
            Decimal::ZERO,
        )
    }

    /// 使用经审计的滚存余额生成双桶建议。
    ///
    /// `max_single_execution` 仍是单次建议的硬上限；滚存资金只能填补机会桶的
    /// 可用上限，不能绕过该安全边界。
    pub fn from_decision_with_carry(
        planned_contribution: Decimal,
        max_single_execution: Decimal,
        configuration: PlanExecutionConfiguration,
        action: Action,
        multiplier: Multiplier,
        carried_opportunity_cash: Decimal,
    ) -> Result<Self, PlanValidationError> {
        validate_positive("planned_contribution", planned_contribution)?;
        validate_positive("max_single_execution", max_single_execution)?;
        if carried_opportunity_cash < Decimal::ZERO || max_single_execution < planned_contribution {
            return Err(PlanValidationError::InvalidOpportunityCash);
        }
        let allocation = configuration.bucket_allocation();
        let core_contribution = planned_contribution * allocation.core_ratio().value();
        let opportunity_budget = planned_contribution - core_contribution;
        let opportunity_multiplier =
            opportunity_multiplier(configuration.risk_mode(), action, multiplier)?;
        let requested_opportunity = opportunity_budget * opportunity_multiplier;
        let maximum_opportunity = max_single_execution - core_contribution;
        let available_opportunity = opportunity_budget + carried_opportunity_cash;
        let opportunity_contribution = requested_opportunity
            .min(available_opportunity)
            .min(maximum_opportunity);
        let unallocated_opportunity_contribution = opportunity_budget - opportunity_contribution;

        Ok(Self {
            planned_contribution,
            core_contribution,
            opportunity_budget,
            opportunity_multiplier,
            carried_opportunity_cash,
            opportunity_contribution,
            unallocated_opportunity_contribution,
            recommended_contribution: core_contribution + opportunity_contribution,
            opportunity_cash_policy: configuration.opportunity_cash_policy(),
            requires_approval: configuration.risk_mode() == PlanRiskMode::Approval,
        })
    }

    /// 返回本次计划投入总金额。
    #[must_use]
    pub fn planned_contribution(self) -> Decimal {
        self.planned_contribution
    }

    /// 返回常规定投桶投入金额。
    #[must_use]
    pub fn core_contribution(self) -> Decimal {
        self.core_contribution
    }

    /// 返回机会桶投入金额。
    #[must_use]
    pub fn opportunity_contribution(self) -> Decimal {
        self.opportunity_contribution
    }

    /// 返回机会桶当期预算。
    #[must_use]
    pub fn opportunity_budget(self) -> Decimal {
        self.opportunity_budget
    }

    /// 返回机会桶实际采用的有界倍率。
    #[must_use]
    pub fn opportunity_multiplier(self) -> Decimal {
        self.opportunity_multiplier
    }

    /// 返回本次建议可使用的此前滚存机会现金。
    #[must_use]
    pub fn carried_opportunity_cash(self) -> Decimal {
        self.carried_opportunity_cash
    }

    /// 返回机会桶本次未建议投入金额。
    #[must_use]
    pub fn unallocated_opportunity_contribution(self) -> Decimal {
        self.unallocated_opportunity_contribution
    }

    /// 返回核心桶与机会桶合计的建议金额。
    #[must_use]
    pub fn recommended_contribution(self) -> Decimal {
        self.recommended_contribution
    }

    /// 返回机会桶未使用金额处理策略。
    #[must_use]
    pub fn opportunity_cash_policy(self) -> OpportunityCashPolicy {
        self.opportunity_cash_policy
    }

    /// 返回本次建议是否等待用户明确确认。
    #[must_use]
    pub fn requires_approval(self) -> bool {
        self.requires_approval
    }

    /// 返回指定桶的投入金额。
    #[must_use]
    pub fn contribution_for(self, bucket: InvestmentBucket) -> Decimal {
        match bucket {
            InvestmentBucket::Core => self.core_contribution,
            InvestmentBucket::Opportunity => self.opportunity_contribution,
        }
    }
}

/// 根据当前策略动作得出机会桶可采用的有界倍率。
fn opportunity_multiplier(
    risk_mode: PlanRiskMode,
    action: Action,
    multiplier: Multiplier,
) -> Result<Decimal, PlanValidationError> {
    if risk_mode == PlanRiskMode::Fixed || matches!(action, Action::Skip | Action::TacticalDelay) {
        return Ok(Decimal::ZERO);
    }

    Decimal::from_f64(multiplier.value()).ok_or(PlanValidationError::InvalidOpportunityMultiplier)
}

/// 投资计划周期。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleKind {
    /// 每月固定日期触发。
    Monthly,
    /// 每周固定星期触发；本 PR 仅持久化配置，尚未接入自动调度器。
    Weekly,
}

/// 持久化后的投资计划。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InvestmentPlan {
    /// 计划 ID。
    pub id: Uuid,
    /// 用户可读计划名称。
    pub name: String,
    /// 投资标的代码，已规范化为大写。
    pub symbol: String,
    /// 基准定投金额，不是本期实际执行金额。
    #[serde(with = "rust_decimal::serde::str")]
    pub base_contribution: Decimal,
    /// ISO 风格三位币种代码，已规范化为大写。
    pub currency: String,
    /// 用户选择的周期。
    pub schedule_kind: ScheduleKind,
    /// 月度计划为月内日期 `1..=28`，周度计划为 ISO 星期 `1..=7`。
    pub schedule_day: i16,
    /// 所有固定执行日，已去重并按升序排列。
    ///
    /// `schedule_day` 保留为第一项，以兼容旧客户端和旧数据。
    pub schedule_days: Vec<i16>,
    /// 已激活、不可变的策略版本引用。
    pub policy: PolicyRef,
    /// 已校验的核心/机会桶与风险模式配置。
    pub execution_configuration: PlanExecutionConfiguration,
    /// 单次执行金额硬上限，不是 planner 输出。
    #[serde(with = "rust_decimal::serde::str")]
    pub max_single_execution: Decimal,
    /// 是否启用计划。
    pub is_active: bool,
    /// 创建时间。
    pub created_at: OffsetDateTime,
    /// 最近更新时间。
    pub updated_at: OffsetDateTime,
}

/// 创建投资计划的领域输入。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CreateInvestmentPlan {
    /// 用户可读计划名称。
    pub name: String,
    /// 投资标的代码。
    pub symbol: String,
    /// 基准定投金额，不是本期实际执行金额。
    #[serde(with = "rust_decimal::serde::str")]
    pub base_contribution: Decimal,
    /// ISO 风格三位币种代码。
    pub currency: String,
    /// 用户选择的周期。
    pub schedule_kind: ScheduleKind,
    /// 月度计划为月内日期，周度计划为 ISO 星期。
    pub schedule_day: i16,
    /// 所有固定执行日；至少包含一个有效日期。
    pub schedule_days: Vec<i16>,
    /// 可选的初始策略版本；省略时绑定 [`default_fixed_dca_policy`]。
    pub policy: Option<PolicyRef>,
    /// 已校验的核心/机会桶与风险模式配置。
    pub execution_configuration: PlanExecutionConfiguration,
    /// 单次执行金额硬上限。
    #[serde(with = "rust_decimal::serde::str")]
    pub max_single_execution: Decimal,
}

impl CreateInvestmentPlan {
    /// 规范化并校验创建输入。
    ///
    /// 本方法只处理计划配置本身，不计算任何本期买入金额。
    pub fn normalize(self) -> Result<Self, PlanValidationError> {
        let name = normalize_non_empty(self.name, MAX_NAME_LEN, PlanValidationError::InvalidName)?;
        let symbol = normalize_symbol(self.symbol)?;
        let currency = normalize_currency(self.currency)?;
        validate_schedule_day(self.schedule_kind, self.schedule_day)?;
        let schedule_days = normalize_schedule_days(self.schedule_kind, self.schedule_days)?;
        if self.schedule_day != schedule_days[0] {
            return Err(PlanValidationError::ScheduleDayMustMatchFirst);
        }
        self.execution_configuration.validate()?;
        validate_amounts(self.base_contribution, self.max_single_execution)?;

        Ok(Self {
            name,
            symbol,
            currency,
            schedule_days,
            policy: Some(self.policy.clone().unwrap_or_else(default_fixed_dca_policy)),
            ..self
        })
    }
}

/// 更新投资计划的领域输入。
///
/// 不包含 `symbol`、`currency` 或 `schedule_kind`；更换标的、币种或周期应创建新计划并停用旧计划。
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct UpdateInvestmentPlan {
    /// 可选的新名称。
    pub name: Option<String>,
    /// 可选的新基准定投金额。
    #[serde(
        default,
        with = "rust_decimal::serde::str_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub base_contribution: Option<Decimal>,
    /// 可选的新每月执行日。
    pub schedule_day: Option<i16>,
    /// 可选的新固定执行日集合；第一项将写入兼容字段 `schedule_day`。
    pub schedule_days: Option<Vec<i16>>,
    /// 可选的新已激活策略版本。
    pub policy: Option<PolicyRef>,
    /// 可选的新核心/机会桶配置。
    pub bucket_allocation: Option<TwoBucketAllocationConfig>,
    /// 可选的新机会桶执行授权方式。
    pub risk_mode: Option<PlanRiskMode>,
    /// 可选的新机会桶未使用金额处理策略。
    pub opportunity_cash_policy: Option<OpportunityCashPolicy>,
    /// 可选的机会现金滚存金额上限，仅 `carry_with_cap` 使用。
    #[serde(
        default,
        with = "rust_decimal::serde::str_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub opportunity_cash_cap: Option<Decimal>,
    /// 可选的周期累计执行金额上限。
    #[serde(
        default,
        with = "rust_decimal::serde::str_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub period_execution_limit: Option<Decimal>,
    /// 可选的新单次执行金额硬上限。
    #[serde(
        default,
        with = "rust_decimal::serde::str_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_single_execution: Option<Decimal>,
    /// 可选启停状态。
    pub is_active: Option<bool>,
}

impl UpdateInvestmentPlan {
    /// 规范化并校验更新输入。
    ///
    /// 当同时更新 `base_contribution` 与 `max_single_execution` 时，会校验二者关系；
    /// 只更新其中一项时，repository update 路径需结合当前计划再次校验最终状态。
    pub fn normalize(self) -> Result<Self, PlanValidationError> {
        if self.name.is_none()
            && self.base_contribution.is_none()
            && self.schedule_day.is_none()
            && self.schedule_days.is_none()
            && self.policy.is_none()
            && self.bucket_allocation.is_none()
            && self.risk_mode.is_none()
            && self.opportunity_cash_policy.is_none()
            && self.opportunity_cash_cap.is_none()
            && self.period_execution_limit.is_none()
            && self.max_single_execution.is_none()
            && self.is_active.is_none()
        {
            return Err(PlanValidationError::EmptyUpdate);
        }

        let name = self
            .name
            .map(|name| normalize_non_empty(name, MAX_NAME_LEN, PlanValidationError::InvalidName))
            .transpose()?;
        if let Some(base) = self.base_contribution {
            validate_positive("base_contribution", base)?;
        }
        if let Some(max) = self.max_single_execution {
            validate_positive("max_single_execution", max)?;
        }
        if let Some(cap) = self.opportunity_cash_cap {
            validate_positive("opportunity_cash_cap", cap)?;
        }
        if let Some(limit) = self.period_execution_limit {
            validate_positive("period_execution_limit", limit)?;
        }
        if let (Some(base), Some(max)) = (self.base_contribution, self.max_single_execution) {
            validate_amounts(base, max)?;
        }
        if let (Some(bucket_allocation), Some(risk_mode), Some(opportunity_cash_policy)) = (
            self.bucket_allocation,
            self.risk_mode,
            self.opportunity_cash_policy,
        ) {
            PlanExecutionConfiguration::new_with_cash_policy(
                bucket_allocation,
                risk_mode,
                opportunity_cash_policy,
            )?;
        }

        Ok(Self { name, ..self })
    }
}

/// 投资计划执行预览输入。
///
/// 这里只表达调度日判断所需的 UTC 日历字段；timezone 和交易所日历由 scheduler adapter 负责。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PreviewInvestmentPlanExecution {
    /// 当前月内日期，范围为 1..=31。
    day_of_month: i16,
    iso_weekday: i16,
}

impl PreviewInvestmentPlanExecution {
    /// 创建已校验的执行预览输入。
    pub fn new(day_of_month: i16) -> Result<Self, PlanValidationError> {
        validate_calendar_day(day_of_month)?;
        Ok(Self {
            day_of_month,
            iso_weekday: 0,
        })
    }

    /// 根据 UTC 月内日期和 ISO 星期创建已校验预览输入。
    pub fn for_date(day_of_month: i16, iso_weekday: i16) -> Result<Self, PlanValidationError> {
        validate_calendar_day(day_of_month)?;
        validate_schedule_day(ScheduleKind::Weekly, iso_weekday)?;
        Ok(Self {
            day_of_month,
            iso_weekday,
        })
    }

    /// 返回已校验的月内日期。
    #[must_use]
    pub fn day_of_month(&self) -> i16 {
        self.day_of_month
    }

    /// 返回 ISO 星期；仅使用旧月度构造函数时为 `0`。
    #[must_use]
    pub fn iso_weekday(&self) -> i16 {
        self.iso_weekday
    }
}

/// 投资计划执行预览状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPreviewStatus {
    /// 计划启用且当前日期命中计划执行日。
    Due,
    /// 计划启用，但当前日期不是计划执行日。
    Waiting,
    /// 计划已停用，本次不会执行。
    Inactive,
}

/// 投资计划执行预览结果。
///
/// 这是执行编排前的轻量领域结果，不包含 broker order 或成交状态。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InvestmentPlanExecutionPreview {
    /// 计划 ID。
    pub plan_id: Uuid,
    /// 投资标的代码。
    pub symbol: String,
    /// ISO 风格三位币种代码。
    pub currency: String,
    /// 计划周期。
    pub schedule_kind: ScheduleKind,
    /// 计划每月执行日。
    pub schedule_day: i16,
    /// 计划所有固定执行日。
    pub schedule_days: Vec<i16>,
    /// 本次预览使用的月内日期。
    pub day_of_month: i16,
    /// 本次预览使用的 ISO 星期。
    pub iso_weekday: i16,
    /// 执行预览状态。
    pub status: ExecutionPreviewStatus,
    /// 命中执行条件时的计划投入金额，已受单次执行上限限制。
    #[serde(
        default,
        with = "rust_decimal::serde::str_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub planned_contribution: Option<Decimal>,
    /// 命中执行条件且调用方提供双桶配置时的投入拆分。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_split: Option<TwoBucketContributionSplit>,
}

/// 投资计划字段校验错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanValidationError {
    /// 名称为空或超过长度限制。
    #[error("investment plan name must be 1..=100 characters after trimming")]
    InvalidName,
    /// 标的为空或超过长度限制。
    #[error("investment plan symbol must be 1..=32 characters after trimming")]
    InvalidSymbol,
    /// 币种不是三位 ASCII 大写字母。
    #[error("currency must be exactly 3 ASCII uppercase letters")]
    InvalidCurrency,
    /// 月度执行日不在 1..=28。
    #[error("monthly schedule day must be between 1 and 28")]
    InvalidScheduleDay,
    /// 周度执行日不在 ISO 星期 1..=7。
    #[error("weekly schedule day must be an ISO weekday between 1 and 7")]
    InvalidWeeklyScheduleDay,
    /// 固定执行日集合为空、包含重复值或含有无效日期。
    #[error("schedule days must be a non-empty unique valid set for the selected schedule")]
    InvalidScheduleDays,
    /// 兼容字段 `schedule_day` 不是固定执行日集合的第一项。
    #[error("schedule_day must match the first schedule day")]
    ScheduleDayMustMatchFirst,
    /// 执行预览日期不在 1..=31。
    #[error("execution preview day must be between 1 and 31")]
    InvalidExecutionPreviewDay,
    /// 双桶分配比例不在 0..=1。
    #[error("bucket allocation ratio must be between 0 and 1")]
    InvalidBucketAllocationRatio,
    /// 双桶分配比例合计不等于 1。
    #[error("bucket allocation ratios must sum to 1")]
    BucketAllocationRatiosMustSumToOne,
    /// 仅核心桶计划未使用固定定投风险模式。
    #[error("a core-only plan must use fixed risk mode")]
    CoreOnlyRequiresFixedRiskMode,
    /// 存在机会桶但未选择自动或审批风险模式。
    #[error("a plan with an opportunity bucket must use autopilot or approval risk mode")]
    OpportunityBucketRequiresRiskMode,
    /// 全部资金位于核心桶时不允许设置没有作用的机会桶滚存策略。
    #[error("a core-only plan must use the expire_each_period opportunity cash policy")]
    CoreOnlyRequiresExpireEachPeriodPolicy,
    /// `carry_with_cap` 未提供正的机会现金余额上限。
    #[error("carry_with_cap requires a positive opportunity cash cap")]
    CarryWithCapRequiresCashCap,
    /// 非 `carry_with_cap` 策略包含无意义的机会现金余额上限。
    #[error("an opportunity cash cap requires the carry_with_cap policy")]
    CashCapRequiresCarryWithCapPolicy,
    /// 倍率无法安全转换为精确金额计算使用的 Decimal。
    #[error("opportunity multiplier cannot be represented as a decimal")]
    InvalidOpportunityMultiplier,
    /// 机会现金池余额或其与单次金额上限的组合无效。
    #[error("opportunity cash balance or execution limit is invalid")]
    InvalidOpportunityCash,
    /// 金额不是正数。
    #[error("{field} must be greater than zero")]
    NonPositiveAmount {
        /// 字段名。
        field: &'static str,
    },
    /// 单次执行上限低于基准定投金额。
    #[error("max_single_execution must be greater than or equal to base_contribution")]
    MaxBelowBaseContribution,
    /// PATCH 没有任何字段。
    #[error("update must include at least one field")]
    EmptyUpdate,
}

/// 投资计划 repository port。
///
/// 这是应用层依赖的 outbound port；V2.1 生产组合根使用本地 SQLite adapter。
#[async_trait]
pub trait InvestmentPlanRepository: Send + Sync {
    /// 创建并持久化投资计划。
    async fn create(
        &self,
        input: CreateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError>;

    /// 按固定顺序列出投资计划。
    async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanRepositoryError>;

    /// 按 ID 查询投资计划。
    async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanRepositoryError>;

    /// 更新投资计划。
    ///
    /// 实现方必须在同一个原子写入路径中读取当前计划、合并已规范化的更新输入、
    /// 校验最终金额组合，并写入结果。
    async fn update(
        &self,
        id: Uuid,
        input: UpdateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError>;

    /// 设置投资计划启停状态。
    async fn set_active(
        &self,
        id: Uuid,
        is_active: bool,
    ) -> Result<InvestmentPlan, PlanRepositoryError>;

    /// 删除一个投资计划及其由持久化层级联管理的本地记录。
    async fn delete(&self, _id: Uuid) -> Result<(), PlanRepositoryError> {
        Err(PlanRepositoryError::Unavailable)
    }
}

/// Repository port 的安全错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanRepositoryError {
    /// 输入未通过领域校验。
    #[error(transparent)]
    Validation(#[from] PlanValidationError),
    /// 计划不存在。
    #[error("investment plan not found")]
    NotFound,
    /// 持久化后端暂不可用；不得包含数据库连接、SQL 或内部错误文本。
    #[error("investment plan persistence is unavailable")]
    Unavailable,
}

/// Investment Plan 应用服务。
#[derive(Clone)]
pub struct InvestmentPlanService {
    repository: Arc<dyn InvestmentPlanRepository>,
}

impl InvestmentPlanService {
    /// 创建服务。
    #[must_use]
    pub fn new(repository: Arc<dyn InvestmentPlanRepository>) -> Self {
        Self { repository }
    }

    /// 创建投资计划。
    ///
    /// 先执行领域规范化与校验，再调用 repository port；不计算执行金额。
    pub async fn create(
        &self,
        input: CreateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanApplicationError> {
        let input = input.normalize()?;
        self.repository.create(input).await.map_err(Into::into)
    }

    /// 列出投资计划。
    pub async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanApplicationError> {
        self.repository.list().await.map_err(Into::into)
    }

    /// 获取单个投资计划。
    pub async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanApplicationError> {
        self.repository.get(id).await.map_err(Into::into)
    }

    /// 更新投资计划。
    ///
    /// 先规范化输入；最终金额组合校验由 repository 在原子写入路径内完成。
    pub async fn update(
        &self,
        id: Uuid,
        input: UpdateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanApplicationError> {
        let input = input.normalize()?;
        self.repository.update(id, input).await.map_err(Into::into)
    }

    /// 设置投资计划启停状态。
    pub async fn set_active(
        &self,
        id: Uuid,
        is_active: bool,
    ) -> Result<InvestmentPlan, PlanApplicationError> {
        self.repository
            .set_active(id, is_active)
            .await
            .map_err(Into::into)
    }

    /// 删除一个投资计划。
    ///
    /// 本服务只发起删除；关联的决策、账本与快照由 repository 的原子持久化边界
    /// 负责级联清理，避免留下脱离计划的本地审计记录。
    pub async fn delete(&self, id: Uuid) -> Result<(), PlanApplicationError> {
        self.repository.delete(id).await.map_err(Into::into)
    }

    /// 预览计划在指定月内日期的执行状态。
    ///
    /// 该用例只做启停与调度日判断，并在 due 时返回不超过单次执行上限的计划预算及
    /// 持久化双桶配置的基准拆分；真实订单、成交和现金池由后续阶段处理。
    pub async fn preview_execution(
        &self,
        id: Uuid,
        input: PreviewInvestmentPlanExecution,
    ) -> Result<InvestmentPlanExecutionPreview, PlanApplicationError> {
        let plan = self.repository.get(id).await?;
        preview_execution(&plan, input, None).map_err(Into::into)
    }

    /// 使用当前策略动作与倍率预览计划在指定月内日期的双桶执行建议。
    ///
    /// 核心桶不受普通策略动作否决；`Skip` 与 `TacticalDelay` 只会使机会桶本次不建议
    /// 投入。该用例仍不读取余额、不生成订单，也不维护机会现金池。
    pub async fn preview_execution_with_decision(
        &self,
        id: Uuid,
        input: PreviewInvestmentPlanExecution,
        action: Action,
        multiplier: Multiplier,
    ) -> Result<InvestmentPlanExecutionPreview, PlanApplicationError> {
        let plan = self.repository.get(id).await?;
        preview_execution(&plan, input, Some((action, multiplier))).map_err(Into::into)
    }

    /// 使用已持久化机会现金池余额预览本次建议。
    pub async fn preview_execution_with_decision_and_cash(
        &self,
        id: Uuid,
        input: PreviewInvestmentPlanExecution,
        action: Action,
        multiplier: Multiplier,
        carried_opportunity_cash: Decimal,
    ) -> Result<InvestmentPlanExecutionPreview, PlanApplicationError> {
        let plan = self.repository.get(id).await?;
        preview_execution_with_cash(&plan, input, action, multiplier, carried_opportunity_cash)
            .map_err(Into::into)
    }
}

/// 应用服务错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanApplicationError {
    /// 输入未通过领域校验。
    #[error(transparent)]
    Validation(#[from] PlanValidationError),
    /// 计划不存在。
    #[error("investment plan not found")]
    NotFound,
    /// 持久化后端暂不可用。
    #[error("investment plan service is unavailable")]
    Unavailable,
}

impl From<PlanRepositoryError> for PlanApplicationError {
    fn from(error: PlanRepositoryError) -> Self {
        match error {
            PlanRepositoryError::Validation(error) => Self::Validation(error),
            PlanRepositoryError::NotFound => Self::NotFound,
            PlanRepositoryError::Unavailable => Self::Unavailable,
        }
    }
}

fn normalize_non_empty(
    value: String,
    max_len: usize,
    error: PlanValidationError,
) -> Result<String, PlanValidationError> {
    let normalized = value.trim().to_owned();
    if normalized.is_empty() || normalized.chars().count() > max_len {
        Err(error)
    } else {
        Ok(normalized)
    }
}

fn normalize_symbol(value: String) -> Result<String, PlanValidationError> {
    let symbol = normalize_non_empty(value, MAX_SYMBOL_LEN, PlanValidationError::InvalidSymbol)?;
    if symbol.is_ascii() {
        Ok(symbol.to_ascii_uppercase())
    } else {
        Err(PlanValidationError::InvalidSymbol)
    }
}

fn normalize_currency(value: String) -> Result<String, PlanValidationError> {
    let currency = value.trim().to_ascii_uppercase();
    if currency.len() == 3 && currency.bytes().all(|byte| byte.is_ascii_uppercase()) {
        Ok(currency)
    } else {
        Err(PlanValidationError::InvalidCurrency)
    }
}

fn validate_schedule_day(kind: ScheduleKind, day: i16) -> Result<(), PlanValidationError> {
    match kind {
        ScheduleKind::Monthly if (1..=28).contains(&day) => Ok(()),
        ScheduleKind::Monthly => Err(PlanValidationError::InvalidScheduleDay),
        ScheduleKind::Weekly if (1..=7).contains(&day) => Ok(()),
        ScheduleKind::Weekly => Err(PlanValidationError::InvalidWeeklyScheduleDay),
    }
}

/// 规范化固定执行日集合并保持稳定排序。
fn normalize_schedule_days(
    kind: ScheduleKind,
    mut days: Vec<i16>,
) -> Result<Vec<i16>, PlanValidationError> {
    if days.is_empty() {
        return Err(PlanValidationError::InvalidScheduleDays);
    }
    let supplied_len = days.len();
    days.sort_unstable();
    days.dedup();
    if days.len() != supplied_len
        || days
            .iter()
            .any(|day| validate_schedule_day(kind, *day).is_err())
    {
        return Err(PlanValidationError::InvalidScheduleDays);
    }
    Ok(days)
}

fn validate_calendar_day(day: i16) -> Result<(), PlanValidationError> {
    if (1..=31).contains(&day) {
        Ok(())
    } else {
        Err(PlanValidationError::InvalidExecutionPreviewDay)
    }
}

fn validate_positive(field: &'static str, value: Decimal) -> Result<(), PlanValidationError> {
    if value > Decimal::ZERO {
        Ok(())
    } else {
        Err(PlanValidationError::NonPositiveAmount { field })
    }
}

fn preview_execution(
    plan: &InvestmentPlan,
    input: PreviewInvestmentPlanExecution,
    decision: Option<(Action, Multiplier)>,
) -> Result<InvestmentPlanExecutionPreview, PlanValidationError> {
    let is_due = match plan.schedule_kind {
        ScheduleKind::Monthly => plan.schedule_days.contains(&input.day_of_month()),
        ScheduleKind::Weekly => {
            input.iso_weekday() != 0 && plan.schedule_days.contains(&input.iso_weekday())
        }
    };
    let status = if !plan.is_active {
        ExecutionPreviewStatus::Inactive
    } else if is_due {
        ExecutionPreviewStatus::Due
    } else {
        ExecutionPreviewStatus::Waiting
    };
    let contribution = if plan.base_contribution <= plan.max_single_execution {
        plan.base_contribution
    } else {
        plan.max_single_execution
    };
    let planned_contribution = (status == ExecutionPreviewStatus::Due).then_some(contribution);
    let bucket_split = match (planned_contribution, decision) {
        (Some(contribution), Some((action, multiplier))) => {
            Some(TwoBucketContributionSplit::from_decision(
                contribution,
                plan.execution_configuration,
                action,
                multiplier,
            )?)
        }
        (Some(contribution), None) => Some(TwoBucketContributionSplit::baseline(
            contribution,
            plan.execution_configuration,
        )?),
        _ => None,
    };

    Ok(InvestmentPlanExecutionPreview {
        plan_id: plan.id,
        symbol: plan.symbol.clone(),
        currency: plan.currency.clone(),
        schedule_kind: plan.schedule_kind,
        schedule_day: plan.schedule_day,
        schedule_days: plan.schedule_days.clone(),
        day_of_month: input.day_of_month(),
        iso_weekday: input.iso_weekday(),
        status,
        planned_contribution,
        bucket_split,
    })
}

fn preview_execution_with_cash(
    plan: &InvestmentPlan,
    input: PreviewInvestmentPlanExecution,
    action: Action,
    multiplier: Multiplier,
    carried_opportunity_cash: Decimal,
) -> Result<InvestmentPlanExecutionPreview, PlanValidationError> {
    let mut preview = preview_execution(plan, input, None)?;
    if let Some(planned_contribution) = preview.planned_contribution {
        preview.bucket_split = Some(TwoBucketContributionSplit::from_decision_with_carry(
            planned_contribution,
            plan.max_single_execution,
            plan.execution_configuration,
            action,
            multiplier,
            carried_opportunity_cash,
        )?);
    }
    Ok(preview)
}

fn validate_amounts(base: Decimal, max: Decimal) -> Result<(), PlanValidationError> {
    validate_positive("base_contribution", base)?;
    validate_positive("max_single_execution", max)?;
    if max < base {
        Err(PlanValidationError::MaxBelowBaseContribution)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};
    use std::sync::Mutex;

    /// 构造测试用 Decimal，避免测试中出现浮点字面量。
    fn money(value: &str) -> Decimal {
        value.parse().unwrap()
    }

    /// 构造一份尚未规范化的创建输入，供领域和服务测试复用。
    fn create_input() -> CreateInvestmentPlan {
        CreateInvestmentPlan {
            name: "  VOO monthly DCA  ".to_owned(),
            symbol: " voo ".to_owned(),
            base_contribution: money("1000.00"),
            currency: " usd ".to_owned(),
            schedule_kind: ScheduleKind::Monthly,
            schedule_day: 15,
            schedule_days: vec![15],
            policy: None,
            execution_configuration: PlanExecutionConfiguration::default(),
            max_single_execution: money("1500.00"),
        }
    }

    /// 构造带机会桶的测试执行配置。
    fn mixed_configuration(
        core_ratio: &str,
        risk_mode: PlanRiskMode,
        cash_policy: OpportunityCashPolicy,
    ) -> PlanExecutionConfiguration {
        let core_ratio = BucketAllocationRatio::new(money(core_ratio)).unwrap();
        let opportunity_ratio =
            BucketAllocationRatio::new(Decimal::ONE - core_ratio.value()).unwrap();
        PlanExecutionConfiguration::new_with_cash_policy(
            TwoBucketAllocationConfig::new(core_ratio, opportunity_ratio).unwrap(),
            risk_mode,
            cash_policy,
        )
        .unwrap()
    }

    /// 将已规范化的创建输入转换成 fake repository 中的持久化计划。
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
            policy: input.policy.unwrap_or_else(default_fixed_dca_policy),
            execution_configuration: input.execution_configuration,
            max_single_execution: input.max_single_execution,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// 基于内存锁的 repository fake，用于验证应用服务与 repository port 契约。
    #[derive(Default)]
    struct FakeRepository {
        plans: Mutex<Vec<InvestmentPlan>>,
        fail: bool,
    }

    /// 在 fake repository 的锁保护数据中按 ID 找到可变计划。
    fn find_plan_mut(
        plans: &mut [InvestmentPlan],
        id: Uuid,
    ) -> Result<&mut InvestmentPlan, PlanRepositoryError> {
        plans
            .iter_mut()
            .find(|plan| plan.id == id)
            .ok_or(PlanRepositoryError::NotFound)
    }

    #[async_trait]
    impl InvestmentPlanRepository for FakeRepository {
        /// 创建计划并追加到内存集合。
        async fn create(
            &self,
            input: CreateInvestmentPlan,
        ) -> Result<InvestmentPlan, PlanRepositoryError> {
            if self.fail {
                return Err(PlanRepositoryError::Unavailable);
            }
            let id = Uuid::from_u128((self.plans.lock().unwrap().len() + 1) as u128);
            let plan = plan_from(id, input);
            self.plans.lock().unwrap().push(plan.clone());
            Ok(plan)
        }

        /// 返回当前内存集合快照。
        async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanRepositoryError> {
            if self.fail {
                return Err(PlanRepositoryError::Unavailable);
            }
            Ok(self.plans.lock().unwrap().clone())
        }

        /// 按 ID 读取计划，失败开关用于覆盖 unavailable 映射。
        async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanRepositoryError> {
            if self.fail {
                return Err(PlanRepositoryError::Unavailable);
            }
            self.plans
                .lock()
                .unwrap()
                .iter()
                .find(|plan| plan.id == id)
                .cloned()
                .ok_or(PlanRepositoryError::NotFound)
        }

        /// 在同一把锁内合并、校验并写入计划更新。
        async fn update(
            &self,
            id: Uuid,
            input: UpdateInvestmentPlan,
        ) -> Result<InvestmentPlan, PlanRepositoryError> {
            if self.fail {
                return Err(PlanRepositoryError::Unavailable);
            }
            let mut plans = self.plans.lock().unwrap();
            let plan = find_plan_mut(&mut plans, id)?;
            let base = input.base_contribution.unwrap_or(plan.base_contribution);
            let max = input
                .max_single_execution
                .unwrap_or(plan.max_single_execution);
            validate_amounts(base, max)?;
            if let Some(day) = input.schedule_day {
                validate_schedule_day(plan.schedule_kind, day)?;
            }
            let bucket_allocation = input
                .bucket_allocation
                .unwrap_or(plan.execution_configuration.bucket_allocation());
            let risk_mode = input
                .risk_mode
                .unwrap_or(plan.execution_configuration.risk_mode());
            let execution_configuration =
                PlanExecutionConfiguration::new(bucket_allocation, risk_mode)?;

            if let Some(name) = input.name {
                plan.name = name;
            }
            if let Some(base) = input.base_contribution {
                plan.base_contribution = base;
            }
            if let Some(day) = input.schedule_day {
                plan.schedule_day = day;
            }
            if let Some(policy) = input.policy {
                plan.policy = policy;
            }
            plan.execution_configuration = execution_configuration;
            if let Some(max) = input.max_single_execution {
                plan.max_single_execution = max;
            }
            if let Some(is_active) = input.is_active {
                plan.is_active = is_active;
            }
            plan.updated_at = OffsetDateTime::from_unix_timestamp(1_700_000_001).unwrap();
            Ok(plan.clone())
        }

        /// 在同一把锁内切换计划启停状态。
        async fn set_active(
            &self,
            id: Uuid,
            is_active: bool,
        ) -> Result<InvestmentPlan, PlanRepositoryError> {
            if self.fail {
                return Err(PlanRepositoryError::Unavailable);
            }
            let mut plans = self.plans.lock().unwrap();
            let plan = find_plan_mut(&mut plans, id)?;
            plan.is_active = is_active;
            plan.updated_at = OffsetDateTime::from_unix_timestamp(1_700_000_001).unwrap();
            Ok(plan.clone())
        }

        /// 从内存集合中删除一个计划。
        async fn delete(&self, id: Uuid) -> Result<(), PlanRepositoryError> {
            if self.fail {
                return Err(PlanRepositoryError::Unavailable);
            }
            let mut plans = self.plans.lock().unwrap();
            let index = plans
                .iter()
                .position(|plan| plan.id == id)
                .ok_or(PlanRepositoryError::NotFound)?;
            plans.remove(index);
            Ok(())
        }
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct DecimalContract {
        #[serde(with = "rust_decimal::serde::str")]
        amount: Decimal,
    }

    /// 验证 JSON 字符串金额可以无浮点损失地反序列化。
    #[test]
    fn decimal_deserializes_from_json_string_without_float() {
        let payload = r#"{"amount":"1000.0001"}"#;

        let decoded: DecimalContract = serde_json::from_str(payload).unwrap();

        assert_eq!(decoded.amount.to_string(), "1000.0001");
    }

    /// 验证金额序列化到 JSON 时仍保持字符串形态。
    #[test]
    fn decimal_serializes_to_json_string_without_float() {
        let payload = DecimalContract {
            amount: "1500.25".parse().unwrap(),
        };

        let encoded = serde_json::to_value(payload).unwrap();

        assert_eq!(encoded, json!({"amount": "1500.25"}));
        assert!(matches!(encoded["amount"], Value::String(_)));
    }

    /// 验证 API 边界拒绝 JSON number，避免引入浮点精度风险。
    #[test]
    fn decimal_rejects_json_number_at_api_boundary() {
        let result = serde_json::from_value::<DecimalContract>(json!({"amount": 1000.00}));

        assert!(result.is_err());
    }

    /// 验证双桶比例只能通过构造器进入有效范围。
    #[test]
    fn bucket_allocation_ratio_accepts_only_closed_unit_interval() {
        assert_eq!(
            BucketAllocationRatio::new(money("0")).unwrap().value(),
            Decimal::ZERO
        );
        assert_eq!(
            BucketAllocationRatio::new(money("1")).unwrap().value(),
            Decimal::ONE
        );
        assert_eq!(
            BucketAllocationRatio::new(money("-0.01")),
            Err(PlanValidationError::InvalidBucketAllocationRatio)
        );
        assert_eq!(
            BucketAllocationRatio::new(money("1.01")),
            Err(PlanValidationError::InvalidBucketAllocationRatio)
        );
    }

    /// 验证双桶配置要求常规定投桶和机会桶比例合计为 1。
    #[test]
    fn two_bucket_config_requires_ratios_to_sum_to_one() {
        let config = TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(money("0.80")).unwrap(),
            BucketAllocationRatio::new(money("0.20")).unwrap(),
        )
        .unwrap();

        assert_eq!(config.core_ratio().value(), money("0.80"));
        assert_eq!(config.opportunity_ratio().value(), money("0.20"));
        assert_eq!(
            TwoBucketAllocationConfig::new(
                BucketAllocationRatio::new(money("0.80")).unwrap(),
                BucketAllocationRatio::new(money("0.30")).unwrap(),
            ),
            Err(PlanValidationError::BucketAllocationRatiosMustSumToOne)
        );
    }

    /// 验证计划级风险模式必须与核心/机会桶占比同时满足不变量。
    #[test]
    fn execution_configuration_requires_explicit_mode_for_opportunity_bucket() {
        let core_only = TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(Decimal::ONE).unwrap(),
            BucketAllocationRatio::new(Decimal::ZERO).unwrap(),
        )
        .unwrap();
        let mixed = TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(money("0.80")).unwrap(),
            BucketAllocationRatio::new(money("0.20")).unwrap(),
        )
        .unwrap();

        assert_eq!(
            PlanExecutionConfiguration::new(core_only, PlanRiskMode::Autopilot),
            Err(PlanValidationError::CoreOnlyRequiresFixedRiskMode)
        );
        assert_eq!(
            PlanExecutionConfiguration::new(mixed, PlanRiskMode::Fixed),
            Err(PlanValidationError::OpportunityBucketRequiresRiskMode)
        );
        assert_eq!(
            PlanExecutionConfiguration::new(mixed, PlanRiskMode::Approval)
                .unwrap()
                .risk_mode(),
            PlanRiskMode::Approval
        );
        assert_eq!(
            PlanExecutionConfiguration::new_with_cash_policy(
                core_only,
                PlanRiskMode::Fixed,
                OpportunityCashPolicy::CarryForward,
            ),
            Err(PlanValidationError::CoreOnlyRequiresExpireEachPeriodPolicy)
        );
    }

    /// 验证周度计划可保存，但当前月度预览不会误将其视为到期计划。
    #[tokio::test]
    async fn weekly_schedule_is_valid_configuration_but_not_yet_monthly_due() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let mut input = create_input();
        input.schedule_kind = ScheduleKind::Weekly;
        input.schedule_day = 1;
        input.schedule_days = vec![1];
        let created = service.create(input).await.unwrap();

        let preview = service
            .preview_execution(created.id, PreviewInvestmentPlanExecution::new(1).unwrap())
            .await
            .unwrap();

        assert_eq!(preview.schedule_kind, ScheduleKind::Weekly);
        assert_eq!(preview.status, ExecutionPreviewStatus::Waiting);
        assert_eq!(preview.planned_contribution, None);
    }

    /// 验证双桶比例以字符串形式序列化，避免浮点比例进入 JSON 边界。
    #[test]
    fn bucket_allocation_ratio_serializes_as_json_string() {
        let encoded =
            serde_json::to_value(BucketAllocationRatio::new(money("0.25")).unwrap()).unwrap();

        assert_eq!(encoded, json!("0.25"));
    }

    /// 验证双桶投入拆分会按比例拆分并保持总额不变。
    #[test]
    fn two_bucket_contribution_split_preserves_total_amount() {
        let config = TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(money("0.80")).unwrap(),
            BucketAllocationRatio::new(money("0.20")).unwrap(),
        )
        .unwrap();

        let split = TwoBucketContributionSplit::new(money("1000.00"), config).unwrap();

        assert_eq!(split.core_contribution(), money("800.0000"));
        assert_eq!(split.opportunity_contribution(), money("200.0000"));
        assert_eq!(
            split.core_contribution() + split.opportunity_contribution(),
            split.planned_contribution()
        );
    }

    /// 验证趋势延迟只使机会桶暂不投入，不会否决核心桶。
    #[test]
    fn tactical_delay_keeps_core_bucket_and_leaves_opportunity_unallocated() {
        let configuration = mixed_configuration(
            "0.70",
            PlanRiskMode::Autopilot,
            OpportunityCashPolicy::CarryForward,
        );

        let split = TwoBucketContributionSplit::from_decision(
            money("1000.00"),
            configuration,
            Action::TacticalDelay,
            Multiplier::new_clamped(1.5),
        )
        .unwrap();

        assert_eq!(split.core_contribution(), money("700.0000"));
        assert_eq!(split.opportunity_budget(), money("300.0000"));
        assert_eq!(split.opportunity_multiplier(), Decimal::ZERO);
        assert_eq!(split.opportunity_contribution(), Decimal::ZERO);
        assert_eq!(
            split.unallocated_opportunity_contribution(),
            money("300.0000")
        );
        assert_eq!(split.recommended_contribution(), money("700.0000"));
        assert_eq!(
            split.opportunity_cash_policy(),
            OpportunityCashPolicy::CarryForward
        );
    }

    /// 验证当前期没有已审计滚存余额时，机会桶加码不会越过当期计划预算。
    #[test]
    fn opportunity_overweight_is_capped_at_current_period_budget() {
        let configuration = mixed_configuration(
            "0.70",
            PlanRiskMode::Autopilot,
            OpportunityCashPolicy::ExpireEachPeriod,
        );

        let split = TwoBucketContributionSplit::from_decision(
            money("1000.00"),
            configuration,
            Action::Overweight,
            Multiplier::new_clamped(1.5),
        )
        .unwrap();

        assert_eq!(split.opportunity_multiplier(), money("1.5"));
        assert_eq!(split.opportunity_contribution(), money("300.00000"));
        assert_eq!(split.unallocated_opportunity_contribution(), Decimal::ZERO);
        assert_eq!(split.recommended_contribution(), money("1000.00000"));
    }

    /// 验证审批模式会将确认要求随执行建议返回。
    #[test]
    fn approval_mode_marks_decision_as_requiring_confirmation() {
        let configuration = mixed_configuration(
            "0.50",
            PlanRiskMode::Approval,
            OpportunityCashPolicy::ExpireEachPeriod,
        );

        let split = TwoBucketContributionSplit::from_decision(
            money("1000.00"),
            configuration,
            Action::Underweight,
            Multiplier::new_clamped(0.5),
        )
        .unwrap();

        assert!(split.requires_approval());
        assert_eq!(split.core_contribution(), money("500.0000"));
        assert_eq!(split.opportunity_contribution(), money("250.00000"));
        assert_eq!(split.recommended_contribution(), money("750.00000"));
    }

    /// 验证双桶投入拆分可以按桶读取金额。
    #[test]
    fn two_bucket_contribution_split_returns_amount_by_bucket() {
        let config = TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(money("0.25")).unwrap(),
            BucketAllocationRatio::new(money("0.75")).unwrap(),
        )
        .unwrap();

        let split = TwoBucketContributionSplit::new(money("40.00"), config).unwrap();

        assert_eq!(
            split.contribution_for(InvestmentBucket::Core),
            money("10.0000")
        );
        assert_eq!(
            split.contribution_for(InvestmentBucket::Opportunity),
            money("30.0000")
        );
    }

    /// 验证双桶投入拆分拒绝非正计划金额。
    #[test]
    fn two_bucket_contribution_split_rejects_non_positive_amount() {
        let config = TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(money("0.50")).unwrap(),
            BucketAllocationRatio::new(money("0.50")).unwrap(),
        )
        .unwrap();

        assert_eq!(
            TwoBucketContributionSplit::new(Decimal::ZERO, config),
            Err(PlanValidationError::NonPositiveAmount {
                field: "planned_contribution",
            })
        );
    }

    /// 验证双桶投入拆分以字符串形式序列化金额。
    #[test]
    fn two_bucket_contribution_split_serializes_amounts_as_json_strings() {
        let config = TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(money("0.50")).unwrap(),
            BucketAllocationRatio::new(money("0.50")).unwrap(),
        )
        .unwrap();
        let split = TwoBucketContributionSplit::new(money("1000.00"), config).unwrap();

        let encoded = serde_json::to_value(split).unwrap();

        assert!(matches!(encoded["planned_contribution"], Value::String(_)));
        assert!(matches!(encoded["core_contribution"], Value::String(_)));
        assert!(matches!(
            encoded["opportunity_contribution"],
            Value::String(_)
        ));
    }

    /// 验证创建输入会裁剪文本并规范化 symbol/currency。
    #[test]
    fn create_plan_normalizes_text_fields() {
        let input = create_input().normalize().unwrap();

        assert_eq!(input.name, "VOO monthly DCA");
        assert_eq!(input.symbol, "VOO");
        assert_eq!(input.currency, "USD");
    }

    /// 验证创建输入拒绝低于基准金额的单次执行上限。
    #[test]
    fn create_plan_rejects_invalid_amount_relationship() {
        let mut input = create_input();
        input.max_single_execution = money("999.99");

        assert_eq!(
            input.normalize(),
            Err(PlanValidationError::MaxBelowBaseContribution)
        );
    }

    /// 验证创建输入拒绝非法执行日和币种。
    #[test]
    fn create_plan_rejects_invalid_schedule_day_and_currency() {
        let mut bad_day = create_input();
        bad_day.schedule_day = 29;
        assert_eq!(
            bad_day.normalize(),
            Err(PlanValidationError::InvalidScheduleDay)
        );

        let mut bad_currency = create_input();
        bad_currency.currency = "US1".to_owned();
        assert_eq!(
            bad_currency.normalize(),
            Err(PlanValidationError::InvalidCurrency)
        );
    }

    /// 验证 symbol 只能使用 ASCII 字符。
    #[test]
    fn create_plan_rejects_non_ascii_symbol() {
        let mut input = create_input();
        input.symbol = "åapl".to_owned();

        assert_eq!(input.normalize(), Err(PlanValidationError::InvalidSymbol));
    }

    /// 验证空 PATCH 不会进入更新流程。
    #[test]
    fn update_plan_rejects_empty_patch() {
        assert_eq!(
            UpdateInvestmentPlan::default().normalize(),
            Err(PlanValidationError::EmptyUpdate)
        );
    }

    /// 验证更新输入会规范化名称并校验同时提交的金额组合。
    #[test]
    fn update_plan_normalizes_name_and_validates_amounts() {
        let update = UpdateInvestmentPlan {
            name: Some("  Core plan  ".to_owned()),
            base_contribution: Some(money("1000.00")),
            max_single_execution: Some(money("1500.00")),
            ..Default::default()
        }
        .normalize()
        .unwrap();

        assert_eq!(update.name.as_deref(), Some("Core plan"));
    }

    /// 验证创建输入中的金额字段保持 JSON 字符串契约。
    #[test]
    fn decimal_fields_remain_json_strings_on_create_input() {
        let encoded = serde_json::to_value(create_input()).unwrap();

        assert!(matches!(encoded["base_contribution"], Value::String(_)));
        assert!(matches!(encoded["max_single_execution"], Value::String(_)));
    }

    /// 验证 service 在持久化前会规范化创建输入。
    #[tokio::test]
    async fn service_normalizes_create_input_before_persisting() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));

        let plan = service.create(create_input()).await.unwrap();

        assert_eq!(plan.name, "VOO monthly DCA");
        assert_eq!(plan.symbol, "VOO");
        assert_eq!(plan.currency, "USD");
    }

    /// 验证非法创建输入会在进入 repository 前被拒绝。
    #[tokio::test]
    async fn service_rejects_invalid_create_before_repository() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let mut input = create_input();
        input.schedule_day = 0;

        assert_eq!(
            service.create(input).await,
            Err(PlanApplicationError::Validation(
                PlanValidationError::InvalidScheduleDay
            ))
        );
    }

    /// 验证 service 通过 repository port 完成列表和单条读取。
    #[tokio::test]
    async fn service_lists_and_gets_plans_through_repository() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));

        let created = service.create(create_input()).await.unwrap();

        assert_eq!(service.list().await.unwrap(), vec![created.clone()]);
        assert_eq!(service.get(created.id).await.unwrap(), created);
    }

    /// 验证 repository 错误会映射为安全的 application 错误。
    #[tokio::test]
    async fn service_maps_repository_errors_to_safe_application_errors() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));

        assert_eq!(
            service.get(Uuid::from_u128(99)).await,
            Err(PlanApplicationError::NotFound)
        );

        let unavailable = InvestmentPlanService::new(Arc::new(FakeRepository {
            plans: Mutex::default(),
            fail: true,
        }));
        assert_eq!(
            unavailable.list().await,
            Err(PlanApplicationError::Unavailable)
        );
        assert_eq!(
            unavailable.get(Uuid::from_u128(1)).await,
            Err(PlanApplicationError::Unavailable)
        );
    }

    /// 验证 service update 能更新允许变更的计划字段。
    #[tokio::test]
    async fn service_updates_plan_fields() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let created = service.create(create_input()).await.unwrap();

        let updated = service
            .update(
                created.id,
                UpdateInvestmentPlan {
                    name: Some("  Core ETF  ".to_owned()),
                    schedule_day: Some(20),
                    max_single_execution: Some(money("2000.00")),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.name, "Core ETF");
        assert_eq!(updated.schedule_day, 20);
        assert_eq!(updated.max_single_execution, money("2000.00"));
        assert!(updated.updated_at > created.updated_at);
    }

    /// 验证 repository update 路径会拒绝最终金额组合非法的更新。
    #[tokio::test]
    async fn service_rejects_update_that_breaks_final_amount_limit() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let created = service.create(create_input()).await.unwrap();

        assert_eq!(
            service
                .update(
                    created.id,
                    UpdateInvestmentPlan {
                        base_contribution: Some(money("2000.00")),
                        ..Default::default()
                    },
                )
                .await,
            Err(PlanApplicationError::Validation(
                PlanValidationError::MaxBelowBaseContribution
            ))
        );
        assert_eq!(
            service.get(created.id).await.unwrap().base_contribution,
            money("1000.00")
        );
    }

    /// 验证 service 能通过专用用例切换计划启停状态。
    #[tokio::test]
    async fn service_sets_active_state() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let created = service.create(create_input()).await.unwrap();

        let disabled = service.set_active(created.id, false).await.unwrap();

        assert!(!disabled.is_active);
        assert!(disabled.updated_at > created.updated_at);
    }

    /// 验证执行预览会在执行日返回 due、受上限限制的预算和持久化基准拆分。
    #[tokio::test]
    async fn service_previews_due_execution_with_persisted_baseline_split() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let created = service.create(create_input()).await.unwrap();

        let preview = service
            .preview_execution(created.id, PreviewInvestmentPlanExecution::new(15).unwrap())
            .await
            .unwrap();

        assert_eq!(preview.plan_id, created.id);
        assert_eq!(preview.status, ExecutionPreviewStatus::Due);
        assert_eq!(preview.planned_contribution, Some(money("1000.00")));
        let split = preview.bucket_split.unwrap();
        assert_eq!(split.core_contribution(), money("1000.00"));
        assert_eq!(split.opportunity_contribution(), Decimal::ZERO);
    }

    /// 验证执行预览会区分等待执行和计划停用。
    #[tokio::test]
    async fn service_previews_waiting_and_inactive_execution() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let created = service.create(create_input()).await.unwrap();

        let waiting = service
            .preview_execution(created.id, PreviewInvestmentPlanExecution::new(16).unwrap())
            .await
            .unwrap();
        assert_eq!(waiting.status, ExecutionPreviewStatus::Waiting);
        assert_eq!(waiting.planned_contribution, None);
        assert_eq!(waiting.bucket_split, None);

        service.set_active(created.id, false).await.unwrap();
        let inactive = service
            .preview_execution(created.id, PreviewInvestmentPlanExecution::new(15).unwrap())
            .await
            .unwrap();
        assert_eq!(inactive.status, ExecutionPreviewStatus::Inactive);
        assert_eq!(inactive.planned_contribution, None);
        assert_eq!(inactive.bucket_split, None);
    }

    /// 验证 due 执行预览可以附带双桶投入拆分。
    #[tokio::test]
    async fn service_previews_due_execution_with_bucket_split() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let mut input = create_input();
        input.execution_configuration = mixed_configuration(
            "0.80",
            PlanRiskMode::Autopilot,
            OpportunityCashPolicy::CarryForward,
        );
        let created = service.create(input).await.unwrap();

        let preview = service
            .preview_execution(created.id, PreviewInvestmentPlanExecution::new(15).unwrap())
            .await
            .unwrap();

        let split = preview.bucket_split.unwrap();
        assert_eq!(preview.status, ExecutionPreviewStatus::Due);
        assert_eq!(split.planned_contribution(), money("1000.00"));
        assert_eq!(split.core_contribution(), money("800.0000"));
        assert_eq!(split.opportunity_budget(), money("200.0000"));
        assert_eq!(split.opportunity_contribution(), money("200.0000"));
        assert_eq!(
            split.opportunity_cash_policy(),
            OpportunityCashPolicy::CarryForward
        );
    }

    /// 验证非 due 执行预览不会附带双桶投入拆分。
    #[tokio::test]
    async fn service_omits_bucket_split_when_execution_is_not_due() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let mut input = create_input();
        input.execution_configuration = mixed_configuration(
            "0.50",
            PlanRiskMode::Autopilot,
            OpportunityCashPolicy::ExpireEachPeriod,
        );
        let created = service.create(input).await.unwrap();

        let waiting = service
            .preview_execution(created.id, PreviewInvestmentPlanExecution::new(16).unwrap())
            .await
            .unwrap();

        assert_eq!(waiting.status, ExecutionPreviewStatus::Waiting);
        assert_eq!(waiting.planned_contribution, None);
        assert_eq!(waiting.bucket_split, None);
    }

    /// 验证执行预览中的双桶拆分保持 JSON 字符串金额契约。
    #[tokio::test]
    async fn bucket_split_preview_serializes_amounts_as_json_strings() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let mut input = create_input();
        input.execution_configuration = mixed_configuration(
            "0.50",
            PlanRiskMode::Approval,
            OpportunityCashPolicy::CarryForward,
        );
        let created = service.create(input).await.unwrap();

        let preview = service
            .preview_execution_with_decision(
                created.id,
                PreviewInvestmentPlanExecution::new(15).unwrap(),
                Action::Underweight,
                Multiplier::new_clamped(0.5),
            )
            .await
            .unwrap();
        let encoded = serde_json::to_value(preview).unwrap();

        assert!(matches!(
            encoded["bucket_split"]["planned_contribution"],
            Value::String(_)
        ));
        assert!(matches!(
            encoded["bucket_split"]["core_contribution"],
            Value::String(_)
        ));
        assert!(matches!(
            encoded["bucket_split"]["opportunity_contribution"],
            Value::String(_)
        ));
        assert!(matches!(
            encoded["bucket_split"]["recommended_contribution"],
            Value::String(_)
        ));
    }

    /// 验证执行预览不会超过单次执行金额硬上限。
    #[test]
    fn execution_preview_never_exceeds_single_execution_cap() {
        let mut plan = plan_from(Uuid::from_u128(1), create_input().normalize().unwrap());
        plan.base_contribution = money("2000.00");
        plan.max_single_execution = money("1500.00");

        let preview = preview_execution(
            &plan,
            PreviewInvestmentPlanExecution::new(15).unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(preview.status, ExecutionPreviewStatus::Due);
        assert_eq!(preview.planned_contribution, Some(money("1500.00")));
    }

    /// 验证执行预览输入构造器拒绝非法月内日期。
    #[test]
    fn preview_execution_input_rejects_invalid_day() {
        assert_eq!(
            PreviewInvestmentPlanExecution::new(32),
            Err(PlanValidationError::InvalidExecutionPreviewDay)
        );
    }

    /// 验证带上限的滚存策略必须显式保留一个正的金额边界。
    #[test]
    fn carry_with_cap_requires_a_positive_cash_cap() {
        let allocation = TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(money("0.70")).unwrap(),
            BucketAllocationRatio::new(money("0.30")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            PlanExecutionConfiguration::new_with_limits(
                allocation,
                PlanRiskMode::Autopilot,
                OpportunityCashPolicy::CarryWithCap,
                None,
                Some(money("300.00")),
            ),
            Err(PlanValidationError::CarryWithCapRequiresCashCap)
        );
        let configuration = PlanExecutionConfiguration::new_with_limits(
            allocation,
            PlanRiskMode::Autopilot,
            OpportunityCashPolicy::CarryWithCap,
            Some(money("500.00")),
            Some(money("300.00")),
        )
        .unwrap();
        assert_eq!(configuration.opportunity_cash_cap(), Some(money("500.00")));
        assert_eq!(
            configuration.period_execution_limit(),
            Some(money("300.00"))
        );
    }
}
