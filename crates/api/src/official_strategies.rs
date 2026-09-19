//! Server-owned immutable strategies exposed by the consumer catalog.

use core_domain::Multiplier;
use indexlink_storage::StoredStrategySpec;
use rust_decimal::Decimal;
use serde::Serialize;
use strategy_dsl::{
    ComparisonOperator, Condition, IndicatorSpec, LookbackWindow, PolicyAction, StrategyRule,
    StrategySpec, StrategySpecDocument, ValueExpression,
};
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};
use time::OffsetDateTime;

use crate::ApiError;

pub(crate) const MA200_TREND_GUARD_ID: &str = "dsl_ma200_trend_guard";
pub(crate) const GROWTH_VOLATILITY_BALANCE_ID: &str = "dsl_growth_volatility_balance";
pub(crate) const FIXED_DCA_ID: &str = "fixed_dca";

/// Consumer-facing risk label owned by the official registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OfficialStrategyRisk {
    Stable,
    Balanced,
}

/// Immutable plan defaults published with one official strategy version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OfficialDefaultPlan {
    pub(crate) schedule_kind: &'static str,
    pub(crate) schedule_day: i16,
    pub(crate) core_ratio: &'static str,
    pub(crate) opportunity_ratio: &'static str,
    pub(crate) risk_mode: &'static str,
}

#[derive(Clone, Copy)]
enum OfficialStrategyKind {
    FixedDca,
    Formula(fn() -> Result<StrategySpec, ApiError>),
}

/// Single source of truth for one consumer-visible official strategy version.
#[derive(Clone, Copy)]
pub(crate) struct OfficialStrategyDescriptor {
    pub(crate) id: &'static str,
    pub(crate) version: u32,
    pub(crate) name: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) rule: &'static str,
    pub(crate) limitation: &'static str,
    pub(crate) risk: OfficialStrategyRisk,
    pub(crate) data_requirements: &'static [&'static str],
    pub(crate) default_plan: OfficialDefaultPlan,
    kind: OfficialStrategyKind,
}

impl OfficialStrategyDescriptor {
    pub(crate) fn policy(self) -> Result<PolicyRef, ApiError> {
        policy_version(self.id, self.version)
    }

    pub(crate) fn is_formula(self) -> bool {
        matches!(self.kind, OfficialStrategyKind::Formula(_))
    }
}

const FIXED_PLAN: OfficialDefaultPlan = OfficialDefaultPlan {
    schedule_kind: "monthly",
    schedule_day: 18,
    core_ratio: "1.0",
    opportunity_ratio: "0.0",
    risk_mode: "fixed",
};

const FORMULA_PLAN: OfficialDefaultPlan = OfficialDefaultPlan {
    schedule_kind: "monthly",
    schedule_day: 18,
    core_ratio: "0.7",
    opportunity_ratio: "0.3",
    risk_mode: "approval",
};

static OFFICIAL_STRATEGIES: [OfficialStrategyDescriptor; 3] = [
    OfficialStrategyDescriptor {
        id: FIXED_DCA_ID,
        version: 1,
        name: "每月稳步投入",
        summary: "不判断行情，在固定日期按固定金额持续投入。",
        rule: "每个计划日建议投入计划金额，不读取市场指标。",
        limitation: "不会主动降低回撤，也可能在市场高位继续买入。",
        risk: OfficialStrategyRisk::Stable,
        data_requirements: &[],
        default_plan: FIXED_PLAN,
        kind: OfficialStrategyKind::FixedDca,
    },
    OfficialStrategyDescriptor {
        id: MA200_TREND_GUARD_ID,
        version: 1,
        name: "200 日均线趋势保护",
        summary: "保留固定核心投入，在价格低于 200 日均线时暂停当期弹性投入。",
        rule: "每期检查价格相对 200 日均线的位置；低于均线时弹性桶为 0，否则按标准额度。",
        limitation: "均线是滞后指标；它不预测底部，也不会取消 70% 核心投入。",
        risk: OfficialStrategyRisk::Stable,
        data_requirements: &["daily_close_200"],
        default_plan: FORMULA_PLAN,
        kind: OfficialStrategyKind::Formula(ma200_trend_guard),
    },
    OfficialStrategyDescriptor {
        id: GROWTH_VOLATILITY_BALANCE_ID,
        version: 1,
        name: "增长与波动平衡",
        summary: "用中期增长和近期波动共同调整弹性投入，固定核心投入保持不变。",
        rule: "63 日年化波动不低于 25% 时弹性额度减半；126 日增长高于 5% 且波动低于 20% 时弹性额度为 1.2 倍。",
        limitation: "阈值来自固定规则而非预测；震荡行情可能频繁切换，且只调整 30% 弹性桶。",
        risk: OfficialStrategyRisk::Balanced,
        data_requirements: &["daily_close_127"],
        default_plan: FORMULA_PLAN,
        kind: OfficialStrategyKind::Formula(growth_volatility_balance),
    },
];

/// Return the complete consumer catalog registry in stable display order.
pub(crate) fn registry() -> &'static [OfficialStrategyDescriptor] {
    &OFFICIAL_STRATEGIES
}

/// Resolve one exact immutable official version.
pub(crate) fn descriptor(policy: &PolicyRef) -> Option<&'static OfficialStrategyDescriptor> {
    registry().iter().find(|descriptor| {
        descriptor.id == policy.id().as_str() && descriptor.version == policy.version().value()
    })
}

/// Resolve the catalog version for a public policy ID.
pub(crate) fn policy_by_id(id: &str) -> Result<Option<PolicyRef>, ApiError> {
    registry()
        .iter()
        .find(|descriptor| descriptor.id == id)
        .copied()
        .map(OfficialStrategyDescriptor::policy)
        .transpose()
}

/// Build the immutable server-owned DSL strategy matching an exact policy reference.
pub(crate) fn strategy(policy: &PolicyRef) -> Result<Option<StrategySpec>, ApiError> {
    let Some(descriptor) = descriptor(policy) else {
        return Ok(None);
    };
    match descriptor.kind {
        OfficialStrategyKind::FixedDca => Ok(None),
        OfficialStrategyKind::Formula(build) => build().map(Some),
    }
}

/// Return whether an ID/version pair is reserved by an official Formula entry.
pub(crate) fn is_reserved(policy: &PolicyRef) -> bool {
    descriptor(policy).is_some_and(|descriptor| descriptor.is_formula())
}

/// Return whether a policy is one of the consumer catalog versions.
pub(crate) fn is_catalog_policy(policy: &PolicyRef) -> bool {
    descriptor(policy).is_some()
}

/// Rebuild an official strategy through the persisted-document boundary used by stored DSL specs.
pub(crate) fn stored_strategy(policy: &PolicyRef) -> Result<Option<StoredStrategySpec>, ApiError> {
    let Some(strategy) = strategy(policy)? else {
        return Ok(None);
    };
    Ok(Some(StoredStrategySpec {
        policy: strategy.policy().clone(),
        name: strategy.name().to_owned(),
        document: StrategySpecDocument::from_strategy_spec(&strategy),
        created_at: OffsetDateTime::UNIX_EPOCH,
    }))
}

fn policy_version(id: &str, version: u32) -> Result<PolicyRef, ApiError> {
    Ok(PolicyRef::new(
        PolicyId::new(id).map_err(|_| ApiError::ServiceUnavailable)?,
        PolicyVersion::new(version).map_err(|_| ApiError::ServiceUnavailable)?,
    ))
}

fn ma200_trend_guard() -> Result<StrategySpec, ApiError> {
    let window = LookbackWindow::new(200).map_err(|_| ApiError::ServiceUnavailable)?;
    StrategySpec::new(
        policy_version(MA200_TREND_GUARD_ID, 1)?,
        "200 日均线趋势保护",
        vec![StrategyRule::new(
            Condition::compare(
                ValueExpression::indicator(IndicatorSpec::MovingAverageDistance(window)),
                ComparisonOperator::LessThan,
                Decimal::ZERO,
            ),
            PolicyAction::skip_opportunity(),
        )],
    )
    .map_err(|_| ApiError::ServiceUnavailable)
}

fn growth_volatility_balance() -> Result<StrategySpec, ApiError> {
    let growth_window = LookbackWindow::new(126).map_err(|_| ApiError::ServiceUnavailable)?;
    let volatility_window = LookbackWindow::new(63).map_err(|_| ApiError::ServiceUnavailable)?;
    let high_volatility = Condition::compare(
        ValueExpression::indicator(IndicatorSpec::AnnualizedVolatility(volatility_window)),
        ComparisonOperator::GreaterThanOrEqual,
        Decimal::new(25, 2),
    );
    let positive_growth_and_calm = Condition::all(vec![
        Condition::compare(
            ValueExpression::indicator(IndicatorSpec::PriceReturn(growth_window)),
            ComparisonOperator::GreaterThan,
            Decimal::new(5, 2),
        ),
        Condition::compare(
            ValueExpression::indicator(IndicatorSpec::AnnualizedVolatility(volatility_window)),
            ComparisonOperator::LessThan,
            Decimal::new(20, 2),
        ),
    ])
    .map_err(|_| ApiError::ServiceUnavailable)?;

    StrategySpec::new(
        policy_version(GROWTH_VOLATILITY_BALANCE_ID, 1)?,
        "增长与波动平衡",
        vec![
            StrategyRule::new(
                high_volatility,
                PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(0.5)),
            ),
            StrategyRule::new(
                positive_growth_and_calm,
                PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(1.2)),
            ),
        ],
    )
    .map_err(|_| ApiError::ServiceUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_the_single_source_for_catalog_and_formula_versions() {
        assert_eq!(registry().len(), 3);
        for descriptor in registry() {
            let policy = descriptor.policy().unwrap();
            assert!(is_catalog_policy(&policy));
            assert_eq!(policy_by_id(descriptor.id).unwrap(), Some(policy.clone()));
            if descriptor.is_formula() {
                let first = strategy(&policy).unwrap().unwrap();
                let rebuilt = StrategySpecDocument::from_strategy_spec(&first)
                    .into_strategy_spec()
                    .unwrap();
                assert_eq!(first, rebuilt);
                assert!(is_reserved(&policy));
                assert!(!first.has_fixed_opportunity_amount_action());
            } else {
                assert!(strategy(&policy).unwrap().is_none());
                assert!(!is_reserved(&policy));
            }
        }
    }

    #[test]
    fn unknown_policy_is_not_accidentally_admitted() {
        assert!(policy_by_id("dsl_missing").unwrap().is_none());
        let missing = policy_version("dsl_missing", 1).unwrap();
        assert!(descriptor(&missing).is_none());
        assert!(!is_catalog_policy(&missing));
        assert!(!is_reserved(&missing));
    }
}
