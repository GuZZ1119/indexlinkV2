//! Consumer-facing catalog of server-owned, versioned strategies.

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use strategy_dsl::StrategySpecDocument;
use strategy_policy::PolicyRef;

use crate::{official_strategies, ApiError, ApiState};

#[derive(Debug, Serialize)]
struct StrategyCatalogEntry {
    policy: PolicyRef,
    name: &'static str,
    summary: &'static str,
    rule: &'static str,
    limitation: &'static str,
    risk: official_strategies::OfficialStrategyRisk,
    supported_markets: &'static [&'static str],
    /// Deprecated compatibility field. An empty list means symbols are validated dynamically.
    supported_symbols: &'static [&'static str],
    default_plan: DefaultPlan,
    data_requirements: &'static [&'static str],
    data_requirement: StrategyDataRequirement,
    adoptable: bool,
    research_status: ResearchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    formula: Option<StrategySpecDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    research: Option<strategy_evaluation::StrategyAdmissionReport>,
}

#[derive(Debug, Serialize)]
struct StrategyDataRequirement {
    required_close_observations: usize,
}

const SUPPORTED_MARKETS: &[&str] = &["us", "hong_kong", "china_shanghai", "china_shenzhen"];

#[derive(Debug, Serialize)]
struct DefaultPlan {
    schedule_kind: &'static str,
    schedule_day: i16,
    core_ratio: &'static str,
    opportunity_ratio: &'static str,
    risk_mode: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ResearchStatus {
    Reference,
    Available,
    Blocked,
}

pub(crate) fn router() -> Router<ApiState> {
    Router::new().route("/strategy-catalog", get(list_strategy_catalog))
}

async fn list_strategy_catalog(
    State(state): State<ApiState>,
) -> Result<Json<Vec<StrategyCatalogEntry>>, ApiError> {
    let mut entries = Vec::with_capacity(official_strategies::registry().len());
    for descriptor in official_strategies::registry() {
        entries.push(strategy_entry(&state, descriptor).await?);
    }
    Ok(Json(entries))
}

async fn strategy_entry(
    state: &ApiState,
    descriptor: &'static official_strategies::OfficialStrategyDescriptor,
) -> Result<StrategyCatalogEntry, ApiError> {
    let policy = descriptor.policy()?;
    let default_plan = DefaultPlan {
        schedule_kind: descriptor.default_plan.schedule_kind,
        schedule_day: descriptor.default_plan.schedule_day,
        core_ratio: descriptor.default_plan.core_ratio,
        opportunity_ratio: descriptor.default_plan.opportunity_ratio,
        risk_mode: descriptor.default_plan.risk_mode,
    };
    if !descriptor.is_formula() {
        return Ok(StrategyCatalogEntry {
            policy,
            name: descriptor.name,
            summary: descriptor.summary,
            rule: descriptor.rule,
            limitation: descriptor.limitation,
            risk: descriptor.risk,
            supported_markets: SUPPORTED_MARKETS,
            supported_symbols: &[],
            default_plan,
            data_requirements: descriptor.data_requirements,
            data_requirement: StrategyDataRequirement {
                required_close_observations: 0,
            },
            adoptable: true,
            research_status: ResearchStatus::Reference,
            formula: None,
            research: None,
        });
    }
    let stored = state.get_strategy_spec(&policy).await?;
    let required_close_observations = official_strategies::strategy(&policy)?
        .ok_or(ApiError::ServiceUnavailable)?
        .required_close_observations();
    let research = state.strategy_admission_report(&policy).await?;
    let adoptable = research.eligible;
    Ok(StrategyCatalogEntry {
        policy,
        name: descriptor.name,
        summary: descriptor.summary,
        rule: descriptor.rule,
        limitation: descriptor.limitation,
        risk: descriptor.risk,
        supported_markets: SUPPORTED_MARKETS,
        supported_symbols: &[],
        default_plan,
        data_requirements: descriptor.data_requirements,
        data_requirement: StrategyDataRequirement {
            required_close_observations,
        },
        adoptable,
        research_status: if adoptable {
            ResearchStatus::Available
        } else {
            ResearchStatus::Blocked
        },
        formula: Some(stored.document),
        research: Some(research),
    })
}
