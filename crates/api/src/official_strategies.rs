//! Server-owned immutable Formula V1 strategies exposed by the consumer catalog.

use core_domain::Multiplier;
use indexlink_storage::StoredStrategySpec;
use rust_decimal::Decimal;
use strategy_dsl::{
    ComparisonOperator, Condition, IndicatorSpec, LookbackWindow, PolicyAction, StrategyRule,
    StrategySpec, StrategySpecDocument, ValueExpression,
};
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};
use time::OffsetDateTime;

use crate::ApiError;

pub(crate) const MA200_TREND_GUARD_ID: &str = "dsl_ma200_trend_guard";
pub(crate) const GROWTH_VOLATILITY_BALANCE_ID: &str = "dsl_growth_volatility_balance";

/// Build the immutable server-owned DSL strategy matching an exact policy reference.
pub(crate) fn strategy(policy: &PolicyRef) -> Result<Option<StrategySpec>, ApiError> {
    match (policy.id().as_str(), policy.version().value()) {
        (MA200_TREND_GUARD_ID, 1) => ma200_trend_guard().map(Some),
        (GROWTH_VOLATILITY_BALANCE_ID, 1) => growth_volatility_balance().map(Some),
        _ => Ok(None),
    }
}

/// Return whether an ID/version pair is reserved by the official catalog.
pub(crate) fn is_reserved(policy: &PolicyRef) -> bool {
    matches!(
        (policy.id().as_str(), policy.version().value()),
        (MA200_TREND_GUARD_ID, 1) | (GROWTH_VOLATILITY_BALANCE_ID, 1)
    )
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

fn policy(id: &str) -> Result<PolicyRef, ApiError> {
    Ok(PolicyRef::new(
        PolicyId::new(id).map_err(|_| ApiError::ServiceUnavailable)?,
        PolicyVersion::new(1).map_err(|_| ApiError::ServiceUnavailable)?,
    ))
}

fn ma200_trend_guard() -> Result<StrategySpec, ApiError> {
    let window = LookbackWindow::new(200).map_err(|_| ApiError::ServiceUnavailable)?;
    StrategySpec::new(
        policy(MA200_TREND_GUARD_ID)?,
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
        policy(GROWTH_VOLATILITY_BALANCE_ID)?,
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
    fn official_strategies_are_immutable_valid_formula_v1_specs() {
        for id in [MA200_TREND_GUARD_ID, GROWTH_VOLATILITY_BALANCE_ID] {
            let policy = policy(id).unwrap();
            let first = strategy(&policy).unwrap().unwrap();
            let rebuilt = StrategySpecDocument::from_strategy_spec(&first)
                .into_strategy_spec()
                .unwrap();

            assert_eq!(first, rebuilt);
            assert!(is_reserved(&policy));
            assert!(!first.has_fixed_opportunity_amount_action());
        }
    }
}
