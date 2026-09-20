//! Consumer-facing catalog of server-owned, versioned strategies.

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use strategy_dsl::StrategySpecDocument;
use strategy_policy::PolicyRef;

use crate::{official_strategies, ApiError, ApiState};

#[derive(Debug, Serialize)]
struct StrategyCatalogEntry {
    policy: PolicyRef,
    name: String,
    summary: String,
    rule: String,
    limitation: String,
    risk: official_strategies::OfficialStrategyRisk,
    supported_markets: &'static [&'static str],
    /// Deprecated compatibility field. An empty list means symbols are validated dynamically.
    supported_symbols: &'static [&'static str],
    default_plan: DefaultPlan,
    data_requirements: Vec<String>,
    data_requirement: StrategyDataRequirement,
    #[serde(skip_serializing_if = "Option::is_none")]
    family: Option<StrategyFamily>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preset: Option<StrategyPreset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<StrategySource>,
    tags: Vec<String>,
    validation_mode: ValidationMode,
    adoptable: bool,
    research_status: ResearchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    formula: Option<StrategySpecDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    research: Option<strategy_evaluation::StrategyAdmissionReport>,
}

#[derive(Debug, Serialize)]
struct StrategyFamily {
    id: String,
    name: String,
    description: String,
    category: String,
}

#[derive(Debug, Serialize)]
struct StrategyPreset {
    id: String,
    name: String,
    order: u8,
}

#[derive(Debug, Serialize)]
struct StrategySource {
    name: String,
    url: String,
    license: String,
    adaptation: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ValidationMode {
    Reference,
    FixedFixture,
    CompiledFormula,
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
    let family = descriptor.family.as_ref().map(|family| StrategyFamily {
        id: family.id.clone(),
        name: family.name.clone(),
        description: family.description.clone(),
        category: family.category.clone(),
    });
    let preset = descriptor.preset.as_ref().map(|preset| StrategyPreset {
        id: preset.id.clone(),
        name: preset.name.clone(),
        order: preset.order,
    });
    let source = descriptor.source.as_ref().map(|source| StrategySource {
        name: source.name.clone(),
        url: source.url.clone(),
        license: source.license.clone(),
        adaptation: source.adaptation.clone(),
    });
    if !descriptor.is_formula() {
        return Ok(StrategyCatalogEntry {
            policy,
            name: descriptor.display_name.clone(),
            summary: descriptor.summary.clone(),
            rule: descriptor.rule.clone(),
            limitation: descriptor.limitation.clone(),
            risk: descriptor.risk,
            supported_markets: SUPPORTED_MARKETS,
            supported_symbols: &[],
            default_plan,
            data_requirements: descriptor.data_requirements.clone(),
            data_requirement: StrategyDataRequirement {
                required_close_observations: 0,
            },
            family,
            preset,
            source,
            tags: descriptor.tags.clone(),
            validation_mode: ValidationMode::Reference,
            adoptable: true,
            research_status: ResearchStatus::Reference,
            formula: None,
            research: None,
        });
    }
    let strategy = official_strategies::strategy(&policy)?.ok_or(ApiError::ServiceUnavailable)?;
    let required_close_observations = strategy.required_close_observations();
    let (adoptable, validation_mode, formula, research) = if descriptor.include_catalog_research {
        let stored = state.get_strategy_spec(&policy).await?;
        let research = state.strategy_admission_report(&policy).await?;
        (
            research.eligible,
            ValidationMode::FixedFixture,
            Some(stored.document),
            Some(research),
        )
    } else {
        let budget = rust_decimal::Decimal::new(1_000, 0);
        (
            !strategy.has_fixed_opportunity_amount_action()
                && strategy.validate_for_budget(budget).is_ok(),
            ValidationMode::CompiledFormula,
            None,
            None,
        )
    };
    Ok(StrategyCatalogEntry {
        policy,
        name: descriptor.display_name.clone(),
        summary: descriptor.summary.clone(),
        rule: descriptor.rule.clone(),
        limitation: descriptor.limitation.clone(),
        risk: descriptor.risk,
        supported_markets: SUPPORTED_MARKETS,
        supported_symbols: &[],
        default_plan,
        data_requirements: descriptor.data_requirements.clone(),
        data_requirement: StrategyDataRequirement {
            required_close_observations,
        },
        family,
        preset,
        source,
        tags: descriptor.tags.clone(),
        validation_mode,
        adoptable,
        research_status: if adoptable {
            ResearchStatus::Available
        } else {
            ResearchStatus::Blocked
        },
        formula,
        research,
    })
}
