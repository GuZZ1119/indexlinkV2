//! Consumer-facing catalog of server-owned, versioned strategies.

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use strategy_dsl::StrategySpecDocument;
use strategy_policy::{PolicyId, PolicyRef, PolicyVersion};

use crate::{official_strategies, ApiError, ApiState};

#[derive(Debug, Serialize)]
struct StrategyCatalogEntry {
    policy: PolicyRef,
    name: &'static str,
    summary: &'static str,
    rule: &'static str,
    limitation: &'static str,
    risk: StrategyRisk,
    supported_symbols: &'static [&'static str],
    default_plan: DefaultPlan,
    data_requirements: &'static [&'static str],
    adoptable: bool,
    research_status: ResearchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    formula: Option<StrategySpecDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    research: Option<strategy_evaluation::StrategyAdmissionReport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum StrategyRisk {
    Stable,
    Balanced,
}

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
    let mut entries = vec![fixed_dca_entry()?];
    entries.push(
        dsl_entry(
            &state,
            official_strategies::MA200_TREND_GUARD_ID,
            "200 日均线趋势保护",
            "保留固定核心投入，在价格低于 200 日均线时暂停当期弹性投入。",
            "每期检查价格相对 200 日均线的位置；低于均线时弹性桶为 0，否则按标准额度。",
            "均线是滞后指标；它不预测底部，也不会取消 70% 核心投入。",
            StrategyRisk::Stable,
            &["daily_close_200"],
        )
        .await?,
    );
    entries.push(
        dsl_entry(
            &state,
            official_strategies::GROWTH_VOLATILITY_BALANCE_ID,
            "增长与波动平衡",
            "用中期增长和近期波动共同调整弹性投入，固定核心投入保持不变。",
            "63 日年化波动不低于 25% 时弹性额度减半；126 日增长高于 5% 且波动低于 20% 时弹性额度为 1.2 倍。",
            "阈值来自固定规则而非预测；震荡行情可能频繁切换，且只调整 30% 弹性桶。",
            StrategyRisk::Balanced,
            &["daily_close_127"],
        )
        .await?,
    );
    Ok(Json(entries))
}

fn fixed_dca_entry() -> Result<StrategyCatalogEntry, ApiError> {
    Ok(StrategyCatalogEntry {
        policy: policy("fixed_dca")?,
        name: "每月稳步投入",
        summary: "不判断行情，在固定日期按固定金额持续投入。",
        rule: "每个计划日建议投入计划金额，不读取市场指标。",
        limitation: "不会主动降低回撤，也可能在市场高位继续买入。",
        risk: StrategyRisk::Stable,
        supported_symbols: &["SPY", "VOO", "QQQ"],
        default_plan: DefaultPlan {
            schedule_kind: "monthly",
            schedule_day: 18,
            core_ratio: "1.0",
            opportunity_ratio: "0.0",
            risk_mode: "fixed",
        },
        data_requirements: &[],
        adoptable: true,
        research_status: ResearchStatus::Reference,
        formula: None,
        research: None,
    })
}

#[allow(clippy::too_many_arguments)]
async fn dsl_entry(
    state: &ApiState,
    id: &str,
    name: &'static str,
    summary: &'static str,
    rule: &'static str,
    limitation: &'static str,
    risk: StrategyRisk,
    data_requirements: &'static [&'static str],
) -> Result<StrategyCatalogEntry, ApiError> {
    let policy = policy(id)?;
    let stored = state.get_strategy_spec(&policy).await?;
    let research = state.strategy_admission_report(&policy).await?;
    let adoptable = research.eligible;
    Ok(StrategyCatalogEntry {
        policy,
        name,
        summary,
        rule,
        limitation,
        risk,
        supported_symbols: &["SPY", "VOO"],
        default_plan: DefaultPlan {
            schedule_kind: "monthly",
            schedule_day: 18,
            core_ratio: "0.7",
            opportunity_ratio: "0.3",
            risk_mode: "approval",
        },
        data_requirements,
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

fn policy(id: &str) -> Result<PolicyRef, ApiError> {
    Ok(PolicyRef::new(
        PolicyId::new(id).map_err(|_| ApiError::ServiceUnavailable)?,
        PolicyVersion::new(1).map_err(|_| ApiError::ServiceUnavailable)?,
    ))
}
