//! Server-owned immutable strategies exposed by the consumer catalog.

use std::sync::OnceLock;

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
pub(crate) const FORMULA_PRESET_COUNT: usize = 100;

const PROFILE_IDS: [&str; 5] = ["responsive", "short", "balanced", "steady", "patient"];
const PROFILE_NAMES: [&str; 5] = ["灵敏", "偏短期", "均衡", "稳健", "长期"];

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

/// Product grouping for one family of parameterised strategy presets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OfficialStrategyFamily {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) category: String,
}

/// Human-readable parameter profile within one family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OfficialStrategyPreset {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) order: u8,
}

/// Traceable public reference used to explain where a rule family comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OfficialStrategySource {
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) license: String,
    pub(crate) adaptation: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FormulaBlueprint {
    PriceSma {
        window: u16,
        severe: i64,
    },
    PriceEma {
        window: u16,
    },
    SmaCross {
        fast: u16,
        slow: u16,
    },
    EmaCross {
        fast: u16,
        slow: u16,
    },
    SmaRibbon {
        fast: u16,
        medium: u16,
        slow: u16,
    },
    AbsoluteMomentum {
        window: u16,
        severe: i64,
    },
    ReturnBand {
        window: u16,
        warn: i64,
        severe: i64,
    },
    DualMomentum {
        short: u16,
        long: u16,
    },
    MomentumAcceleration {
        short: u16,
        long: u16,
        severe: i64,
    },
    RsiHeat {
        window: u16,
        warn: i64,
        severe: i64,
    },
    PricePercentile {
        window: u16,
        warn: i64,
        severe: i64,
    },
    VolatilityBrake {
        window: u16,
        warn: i64,
        severe: i64,
    },
    VolatilityExpansion {
        short: u16,
        long: u16,
        warn: i64,
        severe: i64,
    },
    DrawdownGuard {
        window: u16,
        warn: i64,
        severe: i64,
    },
    NearHighCooling {
        window: u16,
        warn: i64,
        severe: i64,
    },
    TrendVolatility {
        trend: u16,
        volatility: u16,
        high: i64,
    },
    TrendMomentum {
        trend: u16,
        momentum: u16,
    },
    MomentumVolatility {
        momentum: u16,
        volatility: u16,
        high: i64,
    },
    TrendDrawdown {
        trend: u16,
        drawdown: u16,
        severe: i64,
    },
    GrowthVolatility {
        growth: u16,
        volatility: u16,
        high: i64,
        calm: i64,
    },
    LegacyMa200,
    LegacyGrowthVolatility,
}

/// Single source of truth for one consumer-visible official strategy version.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OfficialStrategyDescriptor {
    pub(crate) id: String,
    pub(crate) version: u32,
    pub(crate) name: String,
    pub(crate) summary: String,
    pub(crate) rule: String,
    pub(crate) limitation: String,
    pub(crate) risk: OfficialStrategyRisk,
    pub(crate) data_requirements: Vec<String>,
    pub(crate) default_plan: OfficialDefaultPlan,
    pub(crate) family: Option<OfficialStrategyFamily>,
    pub(crate) preset: Option<OfficialStrategyPreset>,
    pub(crate) source: Option<OfficialStrategySource>,
    pub(crate) tags: Vec<String>,
    pub(crate) include_catalog_research: bool,
    formula: Option<FormulaBlueprint>,
}

impl OfficialStrategyDescriptor {
    pub(crate) fn policy(&self) -> Result<PolicyRef, ApiError> {
        policy_version(&self.id, self.version)
    }

    pub(crate) fn is_formula(&self) -> bool {
        self.formula.is_some()
    }

    pub(crate) fn strategy(&self) -> Result<Option<StrategySpec>, ApiError> {
        self.formula
            .map(|formula| formula.build(self.policy()?, &self.name))
            .transpose()
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

/// Return the complete consumer catalog registry in stable display order.
pub(crate) fn registry() -> &'static [OfficialStrategyDescriptor] {
    static REGISTRY: OnceLock<Vec<OfficialStrategyDescriptor>> = OnceLock::new();
    REGISTRY.get_or_init(build_registry)
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
        .map(OfficialStrategyDescriptor::policy)
        .transpose()
}

/// Build the immutable server-owned DSL strategy matching an exact policy reference.
pub(crate) fn strategy(policy: &PolicyRef) -> Result<Option<StrategySpec>, ApiError> {
    descriptor(policy)
        .map(OfficialStrategyDescriptor::strategy)
        .transpose()
        .map(Option::flatten)
}

/// Return whether an ID/version pair is reserved by an official Formula entry.
pub(crate) fn is_reserved(policy: &PolicyRef) -> bool {
    descriptor(policy).is_some_and(OfficialStrategyDescriptor::is_formula)
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

fn build_registry() -> Vec<OfficialStrategyDescriptor> {
    let mut registry = vec![fixed_dca_descriptor()];
    for family in family_seeds() {
        for (index, blueprint) in family.blueprints.into_iter().enumerate() {
            registry.push(formula_descriptor(&family, index, blueprint));
        }
    }
    debug_assert_eq!(
        registry.iter().filter(|entry| entry.is_formula()).count(),
        FORMULA_PRESET_COUNT
    );
    registry
}

fn fixed_dca_descriptor() -> OfficialStrategyDescriptor {
    OfficialStrategyDescriptor {
        id: FIXED_DCA_ID.to_owned(),
        version: 1,
        name: "每月稳步投入".to_owned(),
        summary: "不判断行情，在固定日期按固定金额持续投入。".to_owned(),
        rule: "每个计划日建议投入计划金额，不读取市场指标。".to_owned(),
        limitation: "不会主动降低回撤，也可能在市场高位继续买入。".to_owned(),
        risk: OfficialStrategyRisk::Stable,
        data_requirements: Vec::new(),
        default_plan: FIXED_PLAN,
        family: None,
        preset: None,
        source: None,
        tags: vec!["基准".to_owned(), "固定投入".to_owned()],
        include_catalog_research: false,
        formula: None,
    }
}

#[derive(Clone)]
struct FamilySeed {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    category: &'static str,
    limitation: &'static str,
    risk: OfficialStrategyRisk,
    source_name: &'static str,
    source_url: &'static str,
    source_license: &'static str,
    tags: &'static [&'static str],
    blueprints: [FormulaBlueprint; 5],
}

fn formula_descriptor(
    family: &FamilySeed,
    index: usize,
    blueprint: FormulaBlueprint,
) -> OfficialStrategyDescriptor {
    let (id, name, include_catalog_research) = match blueprint {
        FormulaBlueprint::LegacyMa200 => (
            MA200_TREND_GUARD_ID.to_owned(),
            "200 日均线趋势保护".to_owned(),
            true,
        ),
        FormulaBlueprint::LegacyGrowthVolatility => (
            GROWTH_VOLATILITY_BALANCE_ID.to_owned(),
            "增长与波动平衡".to_owned(),
            true,
        ),
        _ => (
            format!("dsl_{}_{}", family.id, PROFILE_IDS[index]),
            format!("{} · {}", family.name, PROFILE_NAMES[index]),
            false,
        ),
    };
    let required = blueprint.required_close_observations();
    OfficialStrategyDescriptor {
        id,
        version: 1,
        name,
        summary: family.description.to_owned(),
        rule: blueprint.rule_text(),
        limitation: family.limitation.to_owned(),
        risk: family.risk,
        data_requirements: vec![format!("daily_close_{required}")],
        default_plan: FORMULA_PLAN,
        family: Some(OfficialStrategyFamily {
            id: family.id.to_owned(),
            name: family.name.to_owned(),
            description: family.description.to_owned(),
            category: family.category.to_owned(),
        }),
        preset: Some(OfficialStrategyPreset {
            id: PROFILE_IDS[index].to_owned(),
            name: PROFILE_NAMES[index].to_owned(),
            order: u8::try_from(index + 1).expect("five presets fit in u8"),
        }),
        source: Some(OfficialStrategySource {
            name: family.source_name.to_owned(),
            url: family.source_url.to_owned(),
            license: family.source_license.to_owned(),
            adaptation: "仅参考公开指标或研究思想；Formula V1 规则由 IndexLink 独立实现为定期检查下的机会桶调整，不复制第三方交易代码。".to_owned(),
        }),
        tags: family.tags.iter().map(|tag| (*tag).to_owned()).collect(),
        include_catalog_research,
        formula: Some(blueprint),
    }
}

fn family_seeds() -> Vec<FamilySeed> {
    use FormulaBlueprint as F;
    vec![
        family("price_sma", "价格与简单均线", "用价格相对长期简单均线的位置控制弹性投入。", "趋势", "均线确认较慢，快速反转时可能延后恢复弹性投入。", OfficialStrategyRisk::Stable, "Meb Faber — A Quantitative Approach to Tactical Asset Allocation", "https://mebfaber.com/white-papers/", "research reference", &["均线", "趋势"], [F::PriceSma { window: 50, severe: -8 }, F::PriceSma { window: 100, severe: -9 }, F::PriceSma { window: 150, severe: -10 }, F::LegacyMa200, F::PriceSma { window: 252, severe: -12 }]),
        family("price_ema", "价格与指数均线", "用价格相对指数均线的位置更快识别趋势转弱。", "趋势", "指数均线更敏感，也更容易在震荡区间反复切换。", OfficialStrategyRisk::Balanced, "QuantConnect LEAN indicator examples", "https://github.com/QuantConnect/Lean", "Apache-2.0 reference", &["均线", "趋势", "EMA"], [20, 50, 100, 150, 200].map(|window| F::PriceEma { window })),
        family("sma_cross", "双简单均线确认", "比较快慢两条简单均线，只在中长期方向一致时保留全部弹性额度。", "趋势", "双均线交叉仍是滞后规则，横盘期可能出现来回切换。", OfficialStrategyRisk::Stable, "QuantConnect canonical moving-average cross", "https://github.com/QuantConnect/Lean/blob/master/Algorithm.CSharp/MovingAverageCrossAlgorithm.cs", "Apache-2.0 reference", &["均线交叉", "趋势"], [F::SmaCross { fast: 10, slow: 50 }, F::SmaCross { fast: 20, slow: 50 }, F::SmaCross { fast: 20, slow: 100 }, F::SmaCross { fast: 50, slow: 150 }, F::SmaCross { fast: 50, slow: 200 }]),
        family("ema_cross", "双指数均线确认", "比较快慢两条指数均线，以更灵敏的方式确认趋势方向。", "趋势", "较高灵敏度会增加震荡期的错误切换。", OfficialStrategyRisk::Balanced, "QuantConnect canonical moving-average cross", "https://github.com/QuantConnect/Lean/blob/master/Algorithm.CSharp/MovingAverageCrossAlgorithm.cs", "Apache-2.0 reference", &["均线交叉", "趋势", "EMA"], [F::EmaCross { fast: 8, slow: 21 }, F::EmaCross { fast: 12, slow: 26 }, F::EmaCross { fast: 20, slow: 50 }, F::EmaCross { fast: 30, slow: 90 }, F::EmaCross { fast: 50, slow: 200 }]),
        family("sma_ribbon", "三均线排列", "同时观察短、中、长期简单均线，按趋势破坏程度分档减少弹性投入。", "趋势", "三条均线提高确认要求，也可能更晚识别趋势恢复。", OfficialStrategyRisk::Stable, "TA-Lib overlap studies", "https://ta-lib.org/", "BSD indicator reference", &["均线排列", "趋势"], [F::SmaRibbon { fast: 5, medium: 20, slow: 60 }, F::SmaRibbon { fast: 10, medium: 30, slow: 90 }, F::SmaRibbon { fast: 20, medium: 50, slow: 100 }, F::SmaRibbon { fast: 20, medium: 60, slow: 120 }, F::SmaRibbon { fast: 50, medium: 100, slow: 200 }]),
        family("absolute_momentum", "绝对动量保护", "检查一段时间的累计涨跌，负动量时减少弹性投入。", "动量", "动量规则会在快速反转时滞后，也可能在下跌后减少投入。", OfficialStrategyRisk::Balanced, "TA-Lib Momentum / ROC", "https://ta-lib.org/functions/mom.html", "BSD indicator reference", &["动量", "收益率"], [(21, -5), (63, -7), (126, -9), (189, -11), (252, -12)].map(|(window, severe)| F::AbsoluteMomentum { window, severe })),
        family("return_band", "收益区间分档", "把区间收益分成正常、警戒和明显走弱三档。", "动量", "固定收益阈值不能适应所有证券的波动特征。", OfficialStrategyRisk::Balanced, "TA-Lib Rate of Change", "https://ta-lib.org/functions/roc.html", "BSD indicator reference", &["动量", "分档"], [(21, -2, -6), (63, -3, -8), (126, -4, -10), (189, -5, -12), (252, -6, -15)].map(|(window, warn, severe)| F::ReturnBand { window, warn, severe })),
        family("dual_momentum", "双周期动量确认", "同时观察短周期和长周期收益，两个方向都弱时暂停弹性投入。", "动量", "这是单标的双周期确认，不是跨资产轮动意义上的 Dual Momentum。", OfficialStrategyRisk::Balanced, "Time-series momentum research family", "https://github.com/paperswithbacktest/awesome-systematic-trading", "research index reference", &["双周期", "动量"], [(21, 63), (21, 126), (42, 126), (63, 189), (126, 252)].map(|(short, long)| F::DualMomentum { short, long })),
        family("momentum_accel", "动量变化", "比较短长期收益差，观察近期动量是否明显弱于长期状态。", "动量", "不同周期收益直接比较只是透明规则，不代表价格会按该差值继续运行。", OfficialStrategyRisk::Balanced, "TA-Lib Rate of Change", "https://ta-lib.org/functions/roc.html", "BSD indicator reference", &["动量", "变化"], [(21, 63, -4), (21, 126, -5), (42, 126, -5), (63, 189, -6), (126, 252, -7)].map(|(short, long, severe)| F::MomentumAcceleration { short, long, severe })),
        family("rsi_heat", "RSI 过热保护", "在 RSI 进入较高区间时降低当期弹性投入。", "价格位置", "RSI 过热可以持续很久，较早减量可能错过上涨。", OfficialStrategyRisk::Balanced, "TA-Lib RSI", "https://ta-lib.org/functions/rsi.html", "BSD indicator reference", &["RSI", "过热"], [(7, 65, 75), (10, 67, 77), (14, 70, 80), (21, 72, 82), (28, 75, 85)].map(|(window, warn, severe)| F::RsiHeat { window, warn, severe })),
        family("price_percentile", "历史价格分位节奏", "只衡量当前价格在自身历史窗口中的位置，高分位时减少弹性投入。", "价格位置", "价格分位不是估值，高分位也可能继续上涨。", OfficialStrategyRisk::Stable, "IndexLink historical-position formula", "https://github.com/GuZZ1119/indexlinkV2", "MIT", &["价格分位", "节奏"], [(63, 75, 90), (90, 77, 91), (126, 80, 92), (189, 82, 94), (252, 85, 95)].map(|(window, warn, severe)| F::PricePercentile { window, warn, severe })),
        family("vol_brake", "波动率刹车", "近期年化波动率升高时逐档减少弹性投入。", "风险", "高波动并不必然意味着继续下跌，减量可能降低反弹参与度。", OfficialStrategyRisk::Stable, "QuantConnect volatility indicators", "https://www.quantconnect.com/docs/v2/writing-algorithms/indicators/supported-indicators", "documentation reference", &["波动率", "风险"], [(21, 20, 30), (42, 22, 32), (63, 25, 35), (126, 28, 38), (252, 30, 40)].map(|(window, warn, severe)| F::VolatilityBrake { window, warn, severe })),
        family("vol_expansion", "波动扩张保护", "比较短长期波动率，短期风险快速扩张时减少弹性投入。", "风险", "波动扩张可能来自上涨或下跌，本规则不判断方向。", OfficialStrategyRisk::Balanced, "QuantConnect volatility indicators", "https://www.quantconnect.com/docs/v2/writing-algorithms/indicators/supported-indicators", "documentation reference", &["波动率", "变化"], [(21, 63, 5, 10), (21, 126, 6, 12), (42, 126, 6, 13), (63, 189, 7, 14), (126, 252, 8, 15)].map(|(short, long, warn, severe)| F::VolatilityExpansion { short, long, warn, severe })),
        family("drawdown_guard", "回撤风险保护", "当价格相对窗口高点出现较深回撤时降低弹性投入。", "风险", "在深度回撤中减少投入可能错过低位恢复，本规则偏向风险控制。", OfficialStrategyRisk::Stable, "QuantConnect drawdown and risk examples", "https://github.com/QuantConnect/Lean", "Apache-2.0 reference", &["回撤", "风险"], [(63, -6, -12), (90, -8, -15), (126, -10, -18), (189, -12, -22), (252, -15, -25)].map(|(window, warn, severe)| F::DrawdownGuard { window, warn, severe })),
        family("near_high", "接近高点降速", "价格非常接近窗口高点时减少弹性投入，避免在局部高位一次投入过快。", "价格位置", "接近高点不等于价格昂贵，强趋势中可能长期保持接近高点。", OfficialStrategyRisk::Stable, "IndexLink historical-position formula", "https://github.com/GuZZ1119/indexlinkV2", "MIT", &["回撤", "高位"], [(63, -4, -1), (90, -5, -1), (126, -6, -2), (189, -7, -2), (252, -8, -3)].map(|(window, warn, severe)| F::NearHighCooling { window, warn, severe })),
        family("trend_vol", "趋势与波动保护", "同时检查长期趋势和近期波动，两个风险信号叠加时暂停弹性投入。", "复合", "复合条件更严格，可能增加现金闲置并错过快速反转。", OfficialStrategyRisk::Stable, "IndexLink Formula V1 composition", "https://github.com/GuZZ1119/indexlinkV2", "MIT", &["趋势", "波动率", "复合"], [(50, 21, 30), (100, 42, 32), (150, 63, 35), (200, 126, 38), (252, 252, 40)].map(|(trend, volatility, high)| F::TrendVolatility { trend, volatility, high })),
        family("trend_momentum", "趋势与动量确认", "把价格相对均线的位置与区间收益结合，两个方向都弱时暂停弹性投入。", "复合", "趋势和动量都来自价格，不能视为两个独立风险来源。", OfficialStrategyRisk::Stable, "Meb Faber trend research and TA-Lib ROC", "https://mebfaber.com/white-papers/", "research reference", &["趋势", "动量", "复合"], [(50, 21), (100, 63), (150, 126), (200, 189), (252, 252)].map(|(trend, momentum)| F::TrendMomentum { trend, momentum })),
        family("momentum_vol", "动量与波动确认", "负动量与高波动同时出现时暂停弹性投入，只有一个风险信号时减半。", "复合", "阈值不会区分个股事件、市场系统风险或正常波动。", OfficialStrategyRisk::Balanced, "TA-Lib ROC and QuantConnect volatility indicators", "https://ta-lib.org/", "BSD indicator reference", &["动量", "波动率", "复合"], [(21, 21, 30), (63, 42, 32), (126, 63, 35), (189, 126, 38), (252, 252, 40)].map(|(momentum, volatility, high)| F::MomentumVolatility { momentum, volatility, high })),
        family("trend_drawdown", "趋势与回撤确认", "长期趋势转弱且回撤加深时暂停弹性投入，单一信号只减半。", "复合", "该规则偏防守，在下跌后减少投入可能牺牲低位参与。", OfficialStrategyRisk::Stable, "IndexLink Formula V1 composition", "https://github.com/GuZZ1119/indexlinkV2", "MIT", &["趋势", "回撤", "复合"], [(50, 63, -12), (100, 90, -15), (150, 126, -18), (200, 189, -22), (252, 252, -25)].map(|(trend, drawdown, severe)| F::TrendDrawdown { trend, drawdown, severe })),
        family("growth_vol", "增长与波动平衡", "同时观察中期增长和近期波动，以透明阈值管理弹性投入。", "复合", "增长和波动阈值不预测未来，震荡期可能频繁切换。", OfficialStrategyRisk::Balanced, "IndexLink Formula V1 composition", "https://github.com/GuZZ1119/indexlinkV2", "MIT", &["增长", "波动率", "复合"], [F::GrowthVolatility { growth: 63, volatility: 21, high: 30, calm: 18 }, F::GrowthVolatility { growth: 90, volatility: 42, high: 28, calm: 19 }, F::LegacyGrowthVolatility, F::GrowthVolatility { growth: 189, volatility: 126, high: 32, calm: 22 }, F::GrowthVolatility { growth: 252, volatility: 252, high: 35, calm: 24 }]),
    ]
}

#[allow(clippy::too_many_arguments)]
fn family(
    id: &'static str,
    name: &'static str,
    description: &'static str,
    category: &'static str,
    limitation: &'static str,
    risk: OfficialStrategyRisk,
    source_name: &'static str,
    source_url: &'static str,
    source_license: &'static str,
    tags: &'static [&'static str],
    blueprints: [FormulaBlueprint; 5],
) -> FamilySeed {
    FamilySeed {
        id,
        name,
        description,
        category,
        limitation,
        risk,
        source_name,
        source_url,
        source_license,
        tags,
        blueprints,
    }
}

impl FormulaBlueprint {
    fn build(self, policy: PolicyRef, name: &str) -> Result<StrategySpec, ApiError> {
        use FormulaBlueprint as F;
        let rules = match self {
            F::PriceSma { window, severe } => band_rules(
                indicator(IndicatorSpec::MovingAverageDistance(window_of(window)?)),
                percent(severe),
                Decimal::ZERO,
                ComparisonOperator::LessThanOrEqual,
            )?,
            F::PriceEma { window, .. } => single_guard_rule(
                difference(
                    IndicatorSpec::ClosePrice,
                    IndicatorSpec::ExponentialMovingAverage(window_of(window)?),
                ),
                ComparisonOperator::LessThanOrEqual,
                Decimal::ZERO,
            ),
            F::SmaCross { fast, slow, .. } => single_guard_rule(
                difference(
                    IndicatorSpec::SimpleMovingAverage(window_of(fast)?),
                    IndicatorSpec::SimpleMovingAverage(window_of(slow)?),
                ),
                ComparisonOperator::LessThanOrEqual,
                Decimal::ZERO,
            ),
            F::EmaCross { fast, slow, .. } => single_guard_rule(
                difference(
                    IndicatorSpec::ExponentialMovingAverage(window_of(fast)?),
                    IndicatorSpec::ExponentialMovingAverage(window_of(slow)?),
                ),
                ComparisonOperator::LessThanOrEqual,
                Decimal::ZERO,
            ),
            F::SmaRibbon { fast, medium, slow } => ribbon_rules(fast, medium, slow)?,
            F::AbsoluteMomentum { window, severe } => band_rules(
                indicator(IndicatorSpec::PriceReturn(window_of(window)?)),
                percent(severe),
                Decimal::ZERO,
                ComparisonOperator::LessThanOrEqual,
            )?,
            F::ReturnBand {
                window,
                warn,
                severe,
            } => band_rules(
                indicator(IndicatorSpec::PriceReturn(window_of(window)?)),
                percent(severe),
                percent(warn),
                ComparisonOperator::LessThanOrEqual,
            )?,
            F::DualMomentum { short, long } => dual_risk_rules(
                IndicatorSpec::PriceReturn(window_of(short)?),
                IndicatorSpec::PriceReturn(window_of(long)?),
                Decimal::ZERO,
                ComparisonOperator::LessThanOrEqual,
            )?,
            F::MomentumAcceleration {
                short,
                long,
                severe,
            } => band_rules(
                difference(
                    IndicatorSpec::PriceReturn(window_of(short)?),
                    IndicatorSpec::PriceReturn(window_of(long)?),
                ),
                percent(severe),
                Decimal::ZERO,
                ComparisonOperator::LessThanOrEqual,
            )?,
            F::RsiHeat {
                window,
                warn,
                severe,
            } => band_rules(
                indicator(IndicatorSpec::RelativeStrengthIndex(window_of(window)?)),
                Decimal::new(severe, 0),
                Decimal::new(warn, 0),
                ComparisonOperator::GreaterThanOrEqual,
            )?,
            F::PricePercentile {
                window,
                warn,
                severe,
            } => band_rules(
                indicator(IndicatorSpec::PricePercentile(window_of(window)?)),
                percent(severe),
                percent(warn),
                ComparisonOperator::GreaterThanOrEqual,
            )?,
            F::VolatilityBrake {
                window,
                warn,
                severe,
            } => band_rules(
                indicator(IndicatorSpec::AnnualizedVolatility(window_of(window)?)),
                percent(severe),
                percent(warn),
                ComparisonOperator::GreaterThanOrEqual,
            )?,
            F::VolatilityExpansion {
                short,
                long,
                warn,
                severe,
            } => band_rules(
                difference(
                    IndicatorSpec::AnnualizedVolatility(window_of(short)?),
                    IndicatorSpec::AnnualizedVolatility(window_of(long)?),
                ),
                percent(severe),
                percent(warn),
                ComparisonOperator::GreaterThanOrEqual,
            )?,
            F::DrawdownGuard {
                window,
                warn,
                severe,
            } => band_rules(
                indicator(IndicatorSpec::Drawdown(window_of(window)?)),
                percent(severe),
                percent(warn),
                ComparisonOperator::LessThanOrEqual,
            )?,
            F::NearHighCooling {
                window,
                warn,
                severe,
            } => band_rules(
                indicator(IndicatorSpec::Drawdown(window_of(window)?)),
                percent(severe),
                percent(warn),
                ComparisonOperator::GreaterThanOrEqual,
            )?,
            F::TrendVolatility {
                trend,
                volatility,
                high,
            } => dual_risk_rules(
                IndicatorSpec::MovingAverageDistance(window_of(trend)?),
                IndicatorSpec::AnnualizedVolatility(window_of(volatility)?),
                percent(high),
                ComparisonOperator::GreaterThanOrEqual,
            )?,
            F::TrendMomentum { trend, momentum } => dual_risk_rules(
                IndicatorSpec::MovingAverageDistance(window_of(trend)?),
                IndicatorSpec::PriceReturn(window_of(momentum)?),
                Decimal::ZERO,
                ComparisonOperator::LessThanOrEqual,
            )?,
            F::MomentumVolatility {
                momentum,
                volatility,
                high,
            } => mixed_risk_rules(
                IndicatorSpec::PriceReturn(window_of(momentum)?),
                ComparisonOperator::LessThanOrEqual,
                Decimal::ZERO,
                IndicatorSpec::AnnualizedVolatility(window_of(volatility)?),
                ComparisonOperator::GreaterThanOrEqual,
                percent(high),
            )?,
            F::TrendDrawdown {
                trend,
                drawdown,
                severe,
            } => mixed_risk_rules(
                IndicatorSpec::MovingAverageDistance(window_of(trend)?),
                ComparisonOperator::LessThanOrEqual,
                Decimal::ZERO,
                IndicatorSpec::Drawdown(window_of(drawdown)?),
                ComparisonOperator::LessThanOrEqual,
                percent(severe),
            )?,
            F::GrowthVolatility {
                growth,
                volatility,
                high,
                calm,
            } => growth_volatility_rules(growth, volatility, high, calm)?,
            F::LegacyMa200 => vec![StrategyRule::new(
                compare(
                    IndicatorSpec::MovingAverageDistance(window_of(200)?),
                    ComparisonOperator::LessThan,
                    Decimal::ZERO,
                ),
                PolicyAction::skip_opportunity(),
            )],
            F::LegacyGrowthVolatility => legacy_growth_volatility_rules()?,
        };
        StrategySpec::new(policy, name, rules).map_err(|_| ApiError::ServiceUnavailable)
    }

    fn required_close_observations(self) -> usize {
        let policy = policy_version("dsl_requirement_probe", 1).expect("static probe policy");
        self.build(policy, "Requirement probe")
            .expect("static preset must be valid")
            .required_close_observations()
    }

    fn rule_text(self) -> String {
        use FormulaBlueprint as F;
        match self {
            F::PriceSma { window, .. } => format!("检查价格相对 {window} 日简单均线的位置；明显低于均线时暂停弹性桶，轻度低于时减半。"),
            F::PriceEma { window, .. } => format!("检查价格相对 {window} 日指数均线的位置；低于均线时暂停弹性桶。"),
            F::SmaCross { fast, slow, .. } => format!("比较 SMA{fast} 与 SMA{slow}；快线低于慢线时暂停弹性桶。"),
            F::EmaCross { fast, slow, .. } => format!("比较 EMA{fast} 与 EMA{slow}；快线低于慢线时暂停弹性桶。"),
            F::SmaRibbon { fast, medium, slow } => format!("比较 SMA{fast}、SMA{medium} 与 SMA{slow}；两段排列都转弱时暂停，一个转弱时减半。"),
            F::AbsoluteMomentum { window, .. } => format!("检查 {window} 日收益率；明显为负时暂停弹性桶，轻度为负时减半。"),
            F::ReturnBand { window, warn, severe } => format!("{window} 日收益低于 {severe}% 时暂停弹性桶，低于 {warn}% 时减半。"),
            F::DualMomentum { short, long } => format!("同时检查 {short} 日与 {long} 日收益；两者均为负时暂停，一个为负时减半。"),
            F::MomentumAcceleration { short, long, .. } => format!("比较 {short} 日与 {long} 日收益差；近期动量明显落后时暂停弹性桶，轻度落后时减半。"),
            F::RsiHeat { window, warn, severe } => format!("RSI({window}) 不低于 {severe} 时暂停弹性桶，不低于 {warn} 时减半。"),
            F::PricePercentile { window, warn, severe } => format!("{window} 日价格分位不低于 {severe}% 时暂停弹性桶，不低于 {warn}% 时减半。"),
            F::VolatilityBrake { window, warn, severe } => format!("{window} 日年化波动不低于 {severe}% 时暂停弹性桶，不低于 {warn}% 时减半。"),
            F::VolatilityExpansion { short, long, warn, severe } => format!("{short} 日波动率比 {long} 日高 {severe} 个百分点时暂停弹性桶，高 {warn} 个百分点时减半。"),
            F::DrawdownGuard { window, warn, severe } => format!("{window} 日回撤不高于 {severe}% 时暂停弹性桶，不高于 {warn}% 时减半。"),
            F::NearHighCooling { window, warn, severe } => format!("{window} 日回撤小于 {severe}%（接近高点）时暂停弹性桶，小于 {warn}% 时减半。"),
            F::TrendVolatility { trend, volatility, high } => format!("{trend} 日趋势为负且 {volatility} 日波动不低于 {high}% 时暂停弹性桶；仅出现一个风险信号时减半。"),
            F::TrendMomentum { trend, momentum } => format!("{trend} 日均线趋势与 {momentum} 日动量均为负时暂停弹性桶；仅一个为负时减半。"),
            F::MomentumVolatility { momentum, volatility, high } => format!("{momentum} 日动量为负且 {volatility} 日波动不低于 {high}% 时暂停弹性桶；仅一个风险信号时减半。"),
            F::TrendDrawdown { trend, drawdown, severe } => format!("{trend} 日趋势为负且 {drawdown} 日回撤低于 {severe}% 时暂停弹性桶；仅一个风险信号时减半。"),
            F::GrowthVolatility { growth, volatility, high, calm } => format!("{volatility} 日波动不低于 {high}% 时暂停弹性桶；{growth} 日增长非正或波动高于 {calm}% 时减半。"),
            F::LegacyMa200 => "每期检查价格相对 200 日均线的位置；低于均线时弹性桶为 0，否则按标准额度。".to_owned(),
            F::LegacyGrowthVolatility => "63 日年化波动不低于 25% 时弹性额度减半；126 日增长高于 5% 且波动低于 20% 时弹性额度为 1.2 倍（受计划单次上限约束）。".to_owned(),
        }
    }
}

fn band_rules(
    expression: ValueExpression,
    severe: Decimal,
    warn: Decimal,
    operator: ComparisonOperator,
) -> Result<Vec<StrategyRule>, ApiError> {
    Ok(vec![
        StrategyRule::new(
            Condition::compare(expression.clone(), operator, severe),
            PolicyAction::skip_opportunity(),
        ),
        StrategyRule::new(
            Condition::compare(expression, operator, warn),
            PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(0.5)),
        ),
    ])
}

fn single_guard_rule(
    expression: ValueExpression,
    operator: ComparisonOperator,
    threshold: Decimal,
) -> Vec<StrategyRule> {
    vec![StrategyRule::new(
        Condition::compare(expression, operator, threshold),
        PolicyAction::skip_opportunity(),
    )]
}

fn ribbon_rules(fast: u16, medium: u16, slow: u16) -> Result<Vec<StrategyRule>, ApiError> {
    let fast_below_medium = Condition::compare(
        difference(
            IndicatorSpec::SimpleMovingAverage(window_of(fast)?),
            IndicatorSpec::SimpleMovingAverage(window_of(medium)?),
        ),
        ComparisonOperator::LessThanOrEqual,
        Decimal::ZERO,
    );
    let medium_below_slow = Condition::compare(
        difference(
            IndicatorSpec::SimpleMovingAverage(window_of(medium)?),
            IndicatorSpec::SimpleMovingAverage(window_of(slow)?),
        ),
        ComparisonOperator::LessThanOrEqual,
        Decimal::ZERO,
    );
    Ok(vec![
        StrategyRule::new(
            Condition::all(vec![fast_below_medium.clone(), medium_below_slow.clone()])
                .map_err(|_| ApiError::ServiceUnavailable)?,
            PolicyAction::skip_opportunity(),
        ),
        StrategyRule::new(
            Condition::any(vec![fast_below_medium, medium_below_slow])
                .map_err(|_| ApiError::ServiceUnavailable)?,
            PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(0.5)),
        ),
    ])
}

fn dual_risk_rules(
    left: IndicatorSpec,
    right: IndicatorSpec,
    right_threshold: Decimal,
    right_operator: ComparisonOperator,
) -> Result<Vec<StrategyRule>, ApiError> {
    mixed_risk_rules(
        left,
        ComparisonOperator::LessThanOrEqual,
        Decimal::ZERO,
        right,
        right_operator,
        right_threshold,
    )
}

fn mixed_risk_rules(
    left: IndicatorSpec,
    left_operator: ComparisonOperator,
    left_threshold: Decimal,
    right: IndicatorSpec,
    right_operator: ComparisonOperator,
    right_threshold: Decimal,
) -> Result<Vec<StrategyRule>, ApiError> {
    let left_risk = compare(left, left_operator, left_threshold);
    let right_risk = compare(right, right_operator, right_threshold);
    Ok(vec![
        StrategyRule::new(
            Condition::all(vec![left_risk.clone(), right_risk.clone()])
                .map_err(|_| ApiError::ServiceUnavailable)?,
            PolicyAction::skip_opportunity(),
        ),
        StrategyRule::new(
            Condition::any(vec![left_risk, right_risk])
                .map_err(|_| ApiError::ServiceUnavailable)?,
            PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(0.5)),
        ),
    ])
}

fn growth_volatility_rules(
    growth: u16,
    volatility: u16,
    high: i64,
    calm: i64,
) -> Result<Vec<StrategyRule>, ApiError> {
    let high_volatility = compare(
        IndicatorSpec::AnnualizedVolatility(window_of(volatility)?),
        ComparisonOperator::GreaterThanOrEqual,
        percent(high),
    );
    let weak_or_not_calm = Condition::any(vec![
        compare(
            IndicatorSpec::PriceReturn(window_of(growth)?),
            ComparisonOperator::LessThanOrEqual,
            Decimal::ZERO,
        ),
        compare(
            IndicatorSpec::AnnualizedVolatility(window_of(volatility)?),
            ComparisonOperator::GreaterThanOrEqual,
            percent(calm),
        ),
    ])
    .map_err(|_| ApiError::ServiceUnavailable)?;
    Ok(vec![
        StrategyRule::new(high_volatility, PolicyAction::skip_opportunity()),
        StrategyRule::new(
            weak_or_not_calm,
            PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(0.5)),
        ),
    ])
}

fn legacy_growth_volatility_rules() -> Result<Vec<StrategyRule>, ApiError> {
    let growth = window_of(126)?;
    let volatility = window_of(63)?;
    Ok(vec![
        StrategyRule::new(
            compare(
                IndicatorSpec::AnnualizedVolatility(volatility),
                ComparisonOperator::GreaterThanOrEqual,
                Decimal::new(25, 2),
            ),
            PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(0.5)),
        ),
        StrategyRule::new(
            Condition::all(vec![
                compare(
                    IndicatorSpec::PriceReturn(growth),
                    ComparisonOperator::GreaterThan,
                    Decimal::new(5, 2),
                ),
                compare(
                    IndicatorSpec::AnnualizedVolatility(volatility),
                    ComparisonOperator::LessThan,
                    Decimal::new(20, 2),
                ),
            ])
            .map_err(|_| ApiError::ServiceUnavailable)?,
            PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(1.2)),
        ),
    ])
}

fn compare(
    indicator: IndicatorSpec,
    operator: ComparisonOperator,
    threshold: Decimal,
) -> Condition {
    Condition::compare(ValueExpression::indicator(indicator), operator, threshold)
}

fn indicator(indicator: IndicatorSpec) -> ValueExpression {
    ValueExpression::indicator(indicator)
}

fn difference(left: IndicatorSpec, right: IndicatorSpec) -> ValueExpression {
    ValueExpression::subtract(indicator(left), indicator(right))
}

fn percent(value: i64) -> Decimal {
    Decimal::new(value, 2)
}

fn window_of(days: u16) -> Result<LookbackWindow, ApiError> {
    LookbackWindow::new(days).map_err(|_| ApiError::ServiceUnavailable)
}

fn policy_version(id: &str, version: u32) -> Result<PolicyRef, ApiError> {
    Ok(PolicyRef::new(
        PolicyId::new(id).map_err(|_| ApiError::ServiceUnavailable)?,
        PolicyVersion::new(version).map_err(|_| ApiError::ServiceUnavailable)?,
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn registry_contains_one_benchmark_and_one_hundred_formula_presets() {
        assert_eq!(registry().len(), FORMULA_PRESET_COUNT + 1);
        assert_eq!(
            registry().iter().filter(|entry| entry.is_formula()).count(),
            FORMULA_PRESET_COUNT
        );
        assert_eq!(
            registry()
                .iter()
                .filter_map(|entry| entry.family.as_ref().map(|family| family.id.as_str()))
                .collect::<BTreeSet<_>>()
                .len(),
            20
        );
        assert_eq!(
            registry()
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            registry().len()
        );
    }

    #[test]
    fn every_formula_is_immutable_valid_and_reconstructable() {
        for descriptor in registry().iter().filter(|entry| entry.is_formula()) {
            let policy = descriptor.policy().unwrap();
            let first = descriptor.strategy().unwrap().unwrap();
            let rebuilt = StrategySpecDocument::from_strategy_spec(&first)
                .into_strategy_spec()
                .unwrap();
            assert_eq!(first, rebuilt, "{}", descriptor.id);
            assert!(is_reserved(&policy));
            assert!(is_catalog_policy(&policy));
            assert!(!first.has_fixed_opportunity_amount_action());
            assert!(first.required_close_observations() <= 253);
            assert!(descriptor.family.is_some());
            assert!(descriptor.preset.is_some());
            assert!(descriptor.source.is_some());
        }
    }

    #[test]
    fn existing_public_policy_ids_keep_their_original_formulas() {
        let ma = strategy(&policy_version(MA200_TREND_GUARD_ID, 1).unwrap())
            .unwrap()
            .unwrap();
        let growth = strategy(&policy_version(GROWTH_VOLATILITY_BALANCE_ID, 1).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(ma.required_close_observations(), 200);
        assert_eq!(growth.required_close_observations(), 127);
        assert_eq!(ma.rules().len(), 1);
        assert_eq!(growth.rules().len(), 2);
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
