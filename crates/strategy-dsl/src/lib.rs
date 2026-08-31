#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! 受限、无 IO 的策略 DSL 抽象语法树与校验器。
//!
//! 本 crate 只定义可保存、可审阅的策略规则，不读取市场数据、环境变量或数据库，
//! 也不执行订单。运行时只解释已校验的 AST，不读取市场数据、环境变量、数据库或
//! 网络；存储和 HTTP API 属于后续阶段。该边界只允许白名单指标与动作，因此不会执行
//! 用户代码或任意脚本。

use std::collections::BTreeMap;

use core_domain::{Action, Multiplier};
use rust_decimal::Decimal;
use strategy_policy::{DecisionContext, InvestmentRecommendation, PolicyRef};
use time::Date;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

const MAX_NAME_LEN: usize = 120;
const MAX_RULES: usize = 32;
const MAX_EXPRESSION_DEPTH: usize = 8;
const MAX_EXPRESSION_NODES: usize = 128;
const MAX_LOOKBACK_DAYS: u16 = 365;

/// 一个已校验的指标回看窗口（以交易日计）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LookbackWindow(u16);

impl LookbackWindow {
    /// 构造介于 2 至 365 个交易日的回看窗口。
    pub fn new(days: u16) -> Result<Self, StrategyDslValidationError> {
        if !(2..=MAX_LOOKBACK_DAYS).contains(&days) {
            return Err(StrategyDslValidationError::InvalidLookbackWindow);
        }
        Ok(Self(days))
    }

    /// 返回窗口包含的交易日数量。
    #[must_use]
    pub fn days(self) -> u16 {
        self.0
    }
}

/// 除法表达式可使用的已校验非零常数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NonZeroDecimal(Decimal);

impl NonZeroDecimal {
    /// 构造一个非零常数除数。
    pub fn new(value: Decimal) -> Result<Self, StrategyDslValidationError> {
        if value.is_zero() {
            return Err(StrategyDslValidationError::ZeroDivisor);
        }
        Ok(Self(value))
    }

    /// 返回已校验的除数。
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

/// DSL 首版允许读取的市场指标。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IndicatorSpec {
    /// 当期可得收盘价。
    ClosePrice,
    /// 简单移动平均线。
    SimpleMovingAverage(LookbackWindow),
    /// 指数移动平均线。
    ExponentialMovingAverage(LookbackWindow),
    /// 相对强弱指数。
    RelativeStrengthIndex(LookbackWindow),
    /// 相对过去峰值的回撤。
    Drawdown(LookbackWindow),
    /// Cboe VIX 水平。
    Vix,
}

/// 白名单表达式的不可变值节点。
#[derive(Debug, Clone, PartialEq)]
pub struct ValueExpression(ExpressionKind);

#[derive(Debug, Clone, PartialEq)]
enum ExpressionKind {
    Constant(Decimal),
    Indicator(IndicatorSpec),
    Add(Box<ValueExpression>, Box<ValueExpression>),
    Subtract(Box<ValueExpression>, Box<ValueExpression>),
    Multiply(Box<ValueExpression>, Decimal),
    Divide(Box<ValueExpression>, NonZeroDecimal),
}

impl ValueExpression {
    /// 构造一个固定数值表达式。
    #[must_use]
    pub fn constant(value: Decimal) -> Self {
        Self(ExpressionKind::Constant(value))
    }

    /// 构造一个白名单指标表达式。
    #[must_use]
    pub fn indicator(indicator: IndicatorSpec) -> Self {
        Self(ExpressionKind::Indicator(indicator))
    }

    /// 构造两个表达式的加法。
    #[must_use]
    pub fn sum(left: Self, right: Self) -> Self {
        Self(ExpressionKind::Add(Box::new(left), Box::new(right)))
    }

    /// 构造两个表达式的减法。
    #[must_use]
    pub fn subtract(left: Self, right: Self) -> Self {
        Self(ExpressionKind::Subtract(Box::new(left), Box::new(right)))
    }

    /// 构造一个固定常数乘法。
    #[must_use]
    pub fn multiply(expression: Self, factor: Decimal) -> Self {
        Self(ExpressionKind::Multiply(Box::new(expression), factor))
    }

    /// 构造一个除以非零常数的表达式。
    #[must_use]
    pub fn divide(expression: Self, divisor: NonZeroDecimal) -> Self {
        Self(ExpressionKind::Divide(Box::new(expression), divisor))
    }

    fn collect_indicators(&self, output: &mut std::collections::BTreeSet<IndicatorSpec>) {
        match &self.0 {
            ExpressionKind::Constant(_) => {}
            ExpressionKind::Indicator(indicator) => {
                output.insert(*indicator);
            }
            ExpressionKind::Add(left, right) | ExpressionKind::Subtract(left, right) => {
                left.collect_indicators(output);
                right.collect_indicators(output);
            }
            ExpressionKind::Multiply(expression, _) | ExpressionKind::Divide(expression, _) => {
                expression.collect_indicators(output);
            }
        }
    }

    fn complexity(&self) -> (usize, usize) {
        match &self.0 {
            ExpressionKind::Constant(_) | ExpressionKind::Indicator(_) => (1, 1),
            ExpressionKind::Add(left, right) | ExpressionKind::Subtract(left, right) => {
                let (left_depth, left_nodes) = left.complexity();
                let (right_depth, right_nodes) = right.complexity();
                (
                    left_depth.max(right_depth) + 1,
                    left_nodes + right_nodes + 1,
                )
            }
            ExpressionKind::Multiply(expression, _) | ExpressionKind::Divide(expression, _) => {
                let (depth, nodes) = expression.complexity();
                (depth + 1, nodes + 1)
            }
        }
    }
}

/// 受限 DSL 可用的比较运算符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// 大于阈值。
    GreaterThan,
    /// 大于或等于阈值。
    GreaterThanOrEqual,
    /// 小于阈值。
    LessThan,
    /// 小于或等于阈值。
    LessThanOrEqual,
}

/// 由白名单表达式构成的不可变条件树。
#[derive(Debug, Clone, PartialEq)]
pub struct Condition(ConditionKind);

#[derive(Debug, Clone, PartialEq)]
enum ConditionKind {
    Comparison {
        expression: ValueExpression,
        operator: ComparisonOperator,
        threshold: Decimal,
    },
    All(Vec<Condition>),
    Any(Vec<Condition>),
}

impl Condition {
    /// 构造一个表达式与固定阈值的比较条件。
    #[must_use]
    pub fn compare(
        expression: ValueExpression,
        operator: ComparisonOperator,
        threshold: Decimal,
    ) -> Self {
        Self(ConditionKind::Comparison {
            expression,
            operator,
            threshold,
        })
    }

    /// 构造必须全部满足的条件组合。
    pub fn all(conditions: Vec<Self>) -> Result<Self, StrategyDslValidationError> {
        if conditions.is_empty() {
            return Err(StrategyDslValidationError::EmptyConditionGroup);
        }
        Ok(Self(ConditionKind::All(conditions)))
    }

    /// 构造任一满足即可的条件组合。
    pub fn any(conditions: Vec<Self>) -> Result<Self, StrategyDslValidationError> {
        if conditions.is_empty() {
            return Err(StrategyDslValidationError::EmptyConditionGroup);
        }
        Ok(Self(ConditionKind::Any(conditions)))
    }

    fn collect_indicators(&self, output: &mut std::collections::BTreeSet<IndicatorSpec>) {
        match &self.0 {
            ConditionKind::Comparison { expression, .. } => expression.collect_indicators(output),
            ConditionKind::All(conditions) | ConditionKind::Any(conditions) => {
                for condition in conditions {
                    condition.collect_indicators(output);
                }
            }
        }
    }

    fn complexity(&self) -> (usize, usize) {
        match &self.0 {
            ConditionKind::Comparison { expression, .. } => expression.complexity(),
            ConditionKind::All(conditions) | ConditionKind::Any(conditions) => conditions
                .iter()
                .map(Self::complexity)
                .fold((1, 1), |(depth, nodes), (child_depth, child_nodes)| {
                    (depth.max(child_depth + 1), nodes + child_nodes)
                }),
        }
    }
}

/// DSL 首版允许生成的、尚未执行的动作。
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyAction(PolicyActionKind);

#[derive(Debug, Clone, PartialEq)]
enum PolicyActionKind {
    /// 设置机会桶的固定建议金额；核心桶不受该动作影响。
    SetOpportunityFixedAmount(Decimal),
    /// 调整机会桶的倍率；核心桶始终由计划配置决定。
    SetOpportunityMultiplier(Multiplier),
    /// 跳过当前周期的机会桶；不会删除或否决核心桶。
    SkipOpportunity,
}

impl PolicyAction {
    /// 构造一个金额大于零的机会桶固定金额动作。
    pub fn set_opportunity_fixed_amount(
        amount: Decimal,
    ) -> Result<Self, StrategyDslValidationError> {
        if amount <= Decimal::ZERO {
            return Err(StrategyDslValidationError::InvalidFixedAmount);
        }
        Ok(Self(PolicyActionKind::SetOpportunityFixedAmount(amount)))
    }

    /// 构造一个已由 [`Multiplier`] 限定的机会桶倍率动作。
    #[must_use]
    pub fn set_opportunity_multiplier(multiplier: Multiplier) -> Self {
        Self(PolicyActionKind::SetOpportunityMultiplier(multiplier))
    }

    /// 构造一个只跳过当前机会桶的动作。
    #[must_use]
    pub fn skip_opportunity() -> Self {
        Self(PolicyActionKind::SkipOpportunity)
    }

    /// 判断该动作是否只影响机会桶而不会否决核心桶。
    #[must_use]
    pub fn is_opportunity_only(&self) -> bool {
        matches!(
            self.0,
            PolicyActionKind::SetOpportunityFixedAmount(_)
                | PolicyActionKind::SetOpportunityMultiplier(_)
                | PolicyActionKind::SkipOpportunity
        )
    }

    fn validate_for_budget(&self, budget: Decimal) -> Result<(), StrategyDslValidationError> {
        if let PolicyActionKind::SetOpportunityFixedAmount(amount) = &self.0 {
            if *amount > budget {
                return Err(StrategyDslValidationError::ActionExceedsBudget);
            }
        }
        Ok(())
    }

    fn runtime_action(&self) -> DslRuntimeAction {
        match self.0 {
            PolicyActionKind::SetOpportunityFixedAmount(amount) => {
                DslRuntimeAction::OpportunityFixedAmount(amount)
            }
            PolicyActionKind::SetOpportunityMultiplier(multiplier) => {
                DslRuntimeAction::OpportunityMultiplier(multiplier)
            }
            PolicyActionKind::SkipOpportunity => DslRuntimeAction::SkipOpportunity,
        }
    }
}

/// 一个受限条件与单个白名单动作组成的策略规则。
#[derive(Debug, Clone, PartialEq)]
pub struct StrategyRule {
    condition: Condition,
    action: PolicyAction,
}

impl StrategyRule {
    /// 构造一条条件规则。
    #[must_use]
    pub fn new(condition: Condition, action: PolicyAction) -> Self {
        Self { condition, action }
    }

    /// 返回规则条件。
    #[must_use]
    pub fn condition(&self) -> &Condition {
        &self.condition
    }

    /// 返回规则动作。
    #[must_use]
    pub fn action(&self) -> &PolicyAction {
        &self.action
    }
}

/// 可保存、可审阅且可由确定性解释器执行的受限策略定义。
#[derive(Debug, Clone, PartialEq)]
pub struct StrategySpec {
    policy: PolicyRef,
    name: String,
    rules: Vec<StrategyRule>,
}

impl StrategySpec {
    /// 构造并校验一个版本化的自定义策略定义。
    pub fn new(
        policy: PolicyRef,
        name: impl Into<String>,
        rules: Vec<StrategyRule>,
    ) -> Result<Self, StrategyDslValidationError> {
        let name = normalize_name(name.into())?;
        if !policy.id().as_str().starts_with("dsl_") {
            return Err(StrategyDslValidationError::CustomPolicyIdRequired);
        }
        if rules.is_empty() || rules.len() > MAX_RULES {
            return Err(StrategyDslValidationError::InvalidRuleCount);
        }
        for rule in &rules {
            let (depth, nodes) = rule.condition.complexity();
            if depth > MAX_EXPRESSION_DEPTH || nodes > MAX_EXPRESSION_NODES {
                return Err(StrategyDslValidationError::ExpressionTooComplex);
            }
        }

        Ok(Self {
            policy,
            name,
            rules,
        })
    }

    /// 返回该策略不可变的标识与版本。
    #[must_use]
    pub fn policy(&self) -> &PolicyRef {
        &self.policy
    }

    /// 返回已规范化的策略名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回已校验的规则列表。
    #[must_use]
    pub fn rules(&self) -> &[StrategyRule] {
        &self.rules
    }

    /// 返回本策略在运行时需要的白名单指标集合。
    ///
    /// 应用层可用此集合在激活前确认当前数据适配器能够提供全部证据，避免把只能
    /// 离线研究的策略误用于线上计划。
    #[must_use]
    pub fn required_indicators(&self) -> std::collections::BTreeSet<IndicatorSpec> {
        let mut indicators = std::collections::BTreeSet::new();
        for rule in &self.rules {
            rule.condition.collect_indicators(&mut indicators);
        }
        indicators
    }

    /// 是否包含需要执行层按精确金额处理的机会桶固定金额动作。
    ///
    /// 当前线上 Runtime 仅把 DSL 的倍率或跳过动作映射到既有双桶执行接口；调用方
    /// 应在激活前拒绝这种动作，而离线研究仍可保留它。
    #[must_use]
    pub fn has_fixed_opportunity_amount_action(&self) -> bool {
        self.rules.iter().any(|rule| {
            matches!(
                rule.action.0,
                PolicyActionKind::SetOpportunityFixedAmount(_)
            )
        })
    }

    /// 使用某个计划周期预算再次校验固定金额动作。
    ///
    /// 该方法只校验 DSL 意图，不替代实际执行时的周期累计、机会现金、可用现金或
    /// paper-only 约束。
    pub fn validate_for_budget(&self, budget: Decimal) -> Result<(), StrategyDslValidationError> {
        if budget <= Decimal::ZERO {
            return Err(StrategyDslValidationError::InvalidBudget);
        }
        self.rules
            .iter()
            .try_for_each(|rule| rule.action.validate_for_budget(budget))
    }

    /// 在已解析的证据上以固定规则顺序执行本策略。
    ///
    /// 第一个满足条件的规则生效；没有规则满足时，运行时返回机会桶的标准倍率。该
    /// 解释器不会读取或写入外部状态，也不会生成 broker 订单。调用方仍必须将返回的
    /// [`InvestmentRecommendation`] 与核心桶、周期上限、可用现金及审批约束合并。
    pub fn evaluate(
        &self,
        context: &DecisionContext<DslEvidence>,
    ) -> Result<DslEvaluation, StrategyDslRuntimeError> {
        self.validate_for_budget(context.scheduled_contribution())?;

        for (index, rule) in self.rules.iter().enumerate() {
            if rule.condition.matches(context.evidence())? {
                return Ok(DslEvaluation::from_action(
                    self.policy.clone(),
                    context,
                    Some(index),
                    rule.action.runtime_action(),
                ));
            }
        }

        Ok(DslEvaluation::from_action(
            self.policy.clone(),
            context,
            None,
            DslRuntimeAction::StandardOpportunity,
        ))
    }
}

impl ValueExpression {
    fn evaluate(&self, evidence: &DslEvidence) -> Result<Decimal, StrategyDslRuntimeError> {
        match &self.0 {
            ExpressionKind::Constant(value) => Ok(*value),
            ExpressionKind::Indicator(indicator) => evidence.value(*indicator),
            ExpressionKind::Add(left, right) => left
                .evaluate(evidence)?
                .checked_add(right.evaluate(evidence)?)
                .ok_or(StrategyDslRuntimeError::ArithmeticOverflow),
            ExpressionKind::Subtract(left, right) => left
                .evaluate(evidence)?
                .checked_sub(right.evaluate(evidence)?)
                .ok_or(StrategyDslRuntimeError::ArithmeticOverflow),
            ExpressionKind::Multiply(expression, factor) => expression
                .evaluate(evidence)?
                .checked_mul(*factor)
                .ok_or(StrategyDslRuntimeError::ArithmeticOverflow),
            ExpressionKind::Divide(expression, divisor) => expression
                .evaluate(evidence)?
                .checked_div(divisor.value())
                .ok_or(StrategyDslRuntimeError::ArithmeticOverflow),
        }
    }
}

impl Condition {
    fn matches(&self, evidence: &DslEvidence) -> Result<bool, StrategyDslRuntimeError> {
        match &self.0 {
            ConditionKind::Comparison {
                expression,
                operator,
                threshold,
            } => {
                let value = expression.evaluate(evidence)?;
                Ok(match operator {
                    ComparisonOperator::GreaterThan => value > *threshold,
                    ComparisonOperator::GreaterThanOrEqual => value >= *threshold,
                    ComparisonOperator::LessThan => value < *threshold,
                    ComparisonOperator::LessThanOrEqual => value <= *threshold,
                })
            }
            ConditionKind::All(conditions) => {
                for condition in conditions {
                    if !condition.matches(evidence)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ConditionKind::Any(conditions) => {
                for condition in conditions {
                    if condition.matches(evidence)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }
}

/// 一根带市场可得日期的已验证收盘价。
///
/// 该类型只表达已收盘、可在决策时读取的价格；它不代表成交价格。历史评估器必须
/// 另行选择严格晚于决策日的交易日完成模拟成交。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechnicalClose {
    as_of: Date,
    close: Decimal,
}

impl TechnicalClose {
    /// 构造一个正数收盘价观测。
    pub fn new(as_of: Date, close: Decimal) -> Result<Self, StrategyDslRuntimeError> {
        if close <= Decimal::ZERO {
            return Err(StrategyDslRuntimeError::InvalidMarketObservation);
        }
        Ok(Self { as_of, close })
    }

    /// 返回该价格可被策略读取的日期。
    #[must_use]
    pub fn as_of(self) -> Date {
        self.as_of
    }

    /// 返回已验证的收盘价。
    #[must_use]
    pub fn close(self) -> Decimal {
        self.close
    }
}

/// 一条带市场可得日期的已验证 VIX 观测。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechnicalVix {
    as_of: Date,
    value: Decimal,
}

impl TechnicalVix {
    /// 构造一个非负 VIX 观测。
    pub fn new(as_of: Date, value: Decimal) -> Result<Self, StrategyDslRuntimeError> {
        if value < Decimal::ZERO {
            return Err(StrategyDslRuntimeError::InvalidMarketObservation);
        }
        Ok(Self { as_of, value })
    }

    /// 返回该 VIX 观测可被策略读取的日期。
    #[must_use]
    pub fn as_of(self) -> Date {
        self.as_of
    }

    /// 返回已验证的 VIX 水平。
    #[must_use]
    pub fn value(self) -> Decimal {
        self.value
    }
}

/// 同一决策截止日可读取的技术市场快照。
///
/// 构造时会拒绝未来观测、非严格递增价格日期与非法价格。交易日之间可以有自然的
/// 周末或节假日间隔；该类型不填补任何缺失价格。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechnicalMarketSnapshot {
    as_of: Date,
    closes: Vec<TechnicalClose>,
    vix: TechnicalVix,
}

impl TechnicalMarketSnapshot {
    /// 用截至 `as_of` 的日线与 VIX 构造因果技术快照。
    pub fn new(
        as_of: Date,
        closes: Vec<TechnicalClose>,
        vix: TechnicalVix,
    ) -> Result<Self, StrategyDslRuntimeError> {
        if vix.as_of() > as_of
            || closes.iter().any(|close| close.as_of() > as_of)
            || closes
                .windows(2)
                .any(|pair| pair[0].as_of() >= pair[1].as_of())
        {
            return Err(StrategyDslRuntimeError::NonCausalMarketSnapshot);
        }
        if closes.is_empty() {
            return Err(StrategyDslRuntimeError::MissingIndicator);
        }
        Ok(Self { as_of, closes, vix })
    }

    /// 返回策略决策截止日。
    #[must_use]
    pub fn as_of(&self) -> Date {
        self.as_of
    }

    /// 返回日线观测；顺序严格递增且不包含未来值。
    #[must_use]
    pub fn closes(&self) -> &[TechnicalClose] {
        &self.closes
    }

    /// 返回最后可得、且不晚于截止日的 VIX 观测。
    #[must_use]
    pub fn vix(&self) -> TechnicalVix {
        self.vix
    }

    /// 使用 DSL 唯一的纯函数实现计算本快照所需证据。
    pub fn evidence_for(
        &self,
        strategy: &StrategySpec,
    ) -> Result<DslEvidence, StrategyDslRuntimeError> {
        let closes = self
            .closes
            .iter()
            .map(|observation| observation.close())
            .collect::<Vec<_>>();
        DslEvidence::from_market_snapshot(strategy, &closes, self.vix.value())
    }
}

/// 一组与策略声明完全匹配的已解析指标数值。
///
/// 证据必须由应用层或研究器在 `as_of` 时点之前准备。运行时只读取该快照，因此不会
/// 隐式引入网络、数据库或未来数据。
#[derive(Debug, Clone, PartialEq)]
pub struct DslEvidence {
    values: BTreeMap<IndicatorSpec, Decimal>,
}

impl DslEvidence {
    /// 从白名单指标及其同一时点数值构造证据快照。
    ///
    /// 同一指标重复出现会被拒绝，避免调用方依赖插入顺序覆盖证据。
    pub fn new(
        values: impl IntoIterator<Item = (IndicatorSpec, Decimal)>,
    ) -> Result<Self, StrategyDslRuntimeError> {
        let mut normalized = BTreeMap::new();
        for (indicator, value) in values {
            if normalized.insert(indicator, value).is_some() {
                return Err(StrategyDslRuntimeError::DuplicateIndicator);
            }
        }
        Ok(Self { values: normalized })
    }

    /// 返回某个已提供指标的快照值。
    pub fn value(&self, indicator: IndicatorSpec) -> Result<Decimal, StrategyDslRuntimeError> {
        self.values
            .get(&indicator)
            .copied()
            .ok_or(StrategyDslRuntimeError::MissingIndicator)
    }

    /// 按稳定指标顺序遍历用于本次运行的证据快照。
    pub fn values(&self) -> impl Iterator<Item = (IndicatorSpec, Decimal)> + '_ {
        self.values
            .iter()
            .map(|(indicator, value)| (*indicator, *value))
    }

    /// 从同一 `as_of` 的价格序列和 VIX 快照构造策略所需的技术证据。
    ///
    /// 输入价格必须按时间升序排列且仅包含决策日及以前的可得收盘价。线上适配器和
    /// 离线回测都应调用此函数，避免 SMA、EMA、RSI 或回撤使用不同公式。
    pub fn from_market_snapshot(
        strategy: &StrategySpec,
        closes: &[Decimal],
        vix: Decimal,
    ) -> Result<Self, StrategyDslRuntimeError> {
        let mut values = Vec::new();
        for indicator in strategy.required_indicators() {
            let value = match indicator {
                IndicatorSpec::ClosePrice => *closes
                    .last()
                    .ok_or(StrategyDslRuntimeError::MissingIndicator)?,
                IndicatorSpec::SimpleMovingAverage(window) => {
                    simple_moving_average(closes, window.days())?
                }
                IndicatorSpec::ExponentialMovingAverage(window) => {
                    exponential_moving_average(closes, window.days())?
                }
                IndicatorSpec::RelativeStrengthIndex(window) => {
                    relative_strength_index(closes, window.days())?
                }
                IndicatorSpec::Drawdown(window) => drawdown(closes, window.days())?,
                IndicatorSpec::Vix => vix,
            };
            values.push((indicator, value));
        }
        Self::new(values)
    }

    /// 从带日期边界的因果技术快照构造策略证据。
    ///
    /// 线上实时读取和离线历史评估都应优先使用此入口。它会先验证所有观测都不晚于
    /// 决策截止日，再复用与 [`Self::from_market_snapshot`] 相同的指标公式。
    pub fn from_as_of_market_snapshot(
        strategy: &StrategySpec,
        snapshot: &TechnicalMarketSnapshot,
    ) -> Result<Self, StrategyDslRuntimeError> {
        snapshot.evidence_for(strategy)
    }
}

fn trailing(closes: &[Decimal], count: u16) -> Result<&[Decimal], StrategyDslRuntimeError> {
    let count = usize::from(count);
    closes
        .get(closes.len().saturating_sub(count)..)
        .filter(|values| values.len() == count)
        .ok_or(StrategyDslRuntimeError::MissingIndicator)
}

fn simple_moving_average(
    closes: &[Decimal],
    window: u16,
) -> Result<Decimal, StrategyDslRuntimeError> {
    let values = trailing(closes, window)?;
    values
        .iter()
        .try_fold(Decimal::ZERO, |total, value| {
            total
                .checked_add(*value)
                .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)
        })
        .and_then(|total| {
            total
                .checked_div(Decimal::from(window))
                .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)
        })
}

fn exponential_moving_average(
    closes: &[Decimal],
    window: u16,
) -> Result<Decimal, StrategyDslRuntimeError> {
    let values = trailing(closes, window)?;
    let denominator = Decimal::from(u32::from(window) + 1);
    let alpha = Decimal::from(2_u32)
        .checked_div(denominator)
        .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)?;
    let one_minus = Decimal::ONE
        .checked_sub(alpha)
        .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)?;
    values.iter().skip(1).try_fold(values[0], |ema, close| {
        ema.checked_mul(one_minus)
            .and_then(|value| {
                close
                    .checked_mul(alpha)
                    .and_then(|weighted| value.checked_add(weighted))
            })
            .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)
    })
}

fn relative_strength_index(
    closes: &[Decimal],
    window: u16,
) -> Result<Decimal, StrategyDslRuntimeError> {
    let values = trailing(closes, window.saturating_add(1))?;
    let (gains, losses) = values
        .windows(2)
        .try_fold::<_, _, Result<_, StrategyDslRuntimeError>>(
            (Decimal::ZERO, Decimal::ZERO),
            |(gains, losses), pair| {
                let change = pair[1]
                    .checked_sub(pair[0])
                    .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)?;
                if change >= Decimal::ZERO {
                    Ok((
                        gains
                            .checked_add(change)
                            .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)?,
                        losses,
                    ))
                } else {
                    Ok((
                        gains,
                        losses
                            .checked_add(-change)
                            .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)?,
                    ))
                }
            },
        )?;
    if losses.is_zero() {
        return Ok(Decimal::from(100_u32));
    }
    let relative_strength = gains
        .checked_div(losses)
        .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)?;
    Decimal::from(100_u32)
        .checked_sub(
            Decimal::from(100_u32)
                .checked_div(
                    Decimal::ONE
                        .checked_add(relative_strength)
                        .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)?,
                )
                .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)?,
        )
        .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)
}

fn drawdown(closes: &[Decimal], window: u16) -> Result<Decimal, StrategyDslRuntimeError> {
    let values = trailing(closes, window)?;
    let peak = values
        .iter()
        .copied()
        .max()
        .ok_or(StrategyDslRuntimeError::MissingIndicator)?;
    let close = *values
        .last()
        .ok_or(StrategyDslRuntimeError::MissingIndicator)?;
    close
        .checked_div(peak)
        .and_then(|ratio| ratio.checked_sub(Decimal::ONE))
        .ok_or(StrategyDslRuntimeError::ArithmeticOverflow)
}

/// 解释器实际命中的、只影响机会桶的动作。
#[derive(Debug, Clone, PartialEq)]
pub enum DslRuntimeAction {
    /// 用固定金额建议机会桶投入；实际金额仍受计划执行约束。
    OpportunityFixedAmount(Decimal),
    /// 用有界倍率建议机会桶投入。
    OpportunityMultiplier(Multiplier),
    /// 跳过当前机会桶；核心桶不受影响。
    SkipOpportunity,
    /// 没有匹配规则时的标准机会桶投入。
    StandardOpportunity,
}

impl DslRuntimeAction {
    fn action_and_multiplier(&self) -> (Action, Multiplier) {
        match self {
            Self::OpportunityFixedAmount(_) | Self::StandardOpportunity => {
                (Action::Standard, Multiplier::new_clamped(1.0))
            }
            Self::OpportunityMultiplier(multiplier) => (multiplier.to_action(), *multiplier),
            Self::SkipOpportunity => (Action::Skip, Multiplier::MIN),
        }
    }
}

/// 一次 DSL 解释的确定性结果。
#[derive(Debug, Clone, PartialEq)]
pub struct DslEvaluation {
    recommendation: InvestmentRecommendation,
    matched_rule_index: Option<usize>,
    action: DslRuntimeAction,
}

impl DslEvaluation {
    fn from_action(
        policy: PolicyRef,
        context: &DecisionContext<DslEvidence>,
        matched_rule_index: Option<usize>,
        action: DslRuntimeAction,
    ) -> Self {
        let (recommendation_action, multiplier) = action.action_and_multiplier();
        Self {
            recommendation: InvestmentRecommendation::from_context(
                policy,
                context,
                recommendation_action,
                multiplier,
            ),
            matched_rule_index,
            action,
        }
    }

    /// 返回策略契约可消费的通用推荐。
    #[must_use]
    pub fn recommendation(&self) -> &InvestmentRecommendation {
        &self.recommendation
    }

    /// 返回首条命中的规则位置；没有命中时为 `None`。
    #[must_use]
    pub fn matched_rule_index(&self) -> Option<usize> {
        self.matched_rule_index
    }

    /// 返回不会影响核心桶的具体 DSL 动作。
    #[must_use]
    pub fn action(&self) -> &DslRuntimeAction {
        &self.action
    }
}

fn normalize_name(value: String) -> Result<String, StrategyDslValidationError> {
    let normalized = value.trim();
    if normalized.is_empty() || normalized.len() > MAX_NAME_LEN {
        Err(StrategyDslValidationError::InvalidName)
    } else {
        Ok(normalized.to_owned())
    }
}

/// 受限策略定义未通过安全或可执行性校验。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StrategyDslValidationError {
    /// 策略名称为空白或超过长度上限。
    #[error("strategy name must be non-blank and at most 120 characters")]
    InvalidName,
    /// 自定义策略必须使用 `dsl_` 前缀的策略标识。
    #[error("custom strategy policy id must start with dsl_")]
    CustomPolicyIdRequired,
    /// 规则数量为空或超过安全上限。
    #[error("strategy must contain between 1 and 32 rules")]
    InvalidRuleCount,
    /// 回看窗口不在支持范围内。
    #[error("lookback window must be between 2 and 365 trading days")]
    InvalidLookbackWindow,
    /// 除数不能为零。
    #[error("strategy expression divisor must not be zero")]
    ZeroDivisor,
    /// 条件组不能为空。
    #[error("condition group must not be empty")]
    EmptyConditionGroup,
    /// 条件树超过固定的深度或节点安全上限。
    #[error("strategy expression exceeds the supported complexity limit")]
    ExpressionTooComplex,
    /// 固定金额动作必须大于零。
    #[error("fixed contribution action must be greater than zero")]
    InvalidFixedAmount,
    /// DSL 固定金额超过调用方提供的周期预算。
    #[error("strategy action exceeds the plan period budget")]
    ActionExceedsBudget,
    /// 调用方提供的计划周期预算无效。
    #[error("plan period budget must be greater than zero")]
    InvalidBudget,
}

/// 已校验 DSL 在已解析证据上执行失败。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StrategyDslRuntimeError {
    /// 当前证据快照没有策略表达式所需的白名单指标。
    #[error("strategy evidence is missing a required indicator")]
    MissingIndicator,
    /// 同一指标在一份证据快照中被提供多次。
    #[error("strategy evidence contains a duplicate indicator")]
    DuplicateIndicator,
    /// 有界 AST 的 Decimal 运算超出可表示范围。
    #[error("strategy expression arithmetic overflowed")]
    ArithmeticOverflow,
    /// 一条技术市场观测的价格或 VIX 值违反非负/正数不变量。
    #[error("market evidence contains an invalid observation")]
    InvalidMarketObservation,
    /// 市场快照包含未来日期或未按严格日期顺序排列的价格观测。
    #[error("market evidence is not causal at the requested as-of date")]
    NonCausalMarketSnapshot,
    /// 调用时的周期预算无法满足已保存 DSL 的固定金额约束。
    #[error(transparent)]
    Validation(#[from] StrategyDslValidationError),
}

/// 版本化策略定义的可持久化、可传输 JSON 文档。
///
/// 此类型仅在 `serde` feature 启用时提供。反序列化后必须调用
/// [`StrategySpecDocument::into_strategy_spec`]，由它重新构造并校验领域类型；不得将
/// JSON 直接当作已验证的运行时策略使用。
#[cfg(feature = "serde")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategySpecDocument {
    /// 稳定自定义策略标识，例如 `dsl_rsi_opportunity_guard`。
    pub policy_id: String,
    /// 不可变策略版本，必须大于零。
    pub policy_version: u32,
    /// 用户可读策略名称。
    pub name: String,
    /// 固定顺序的条件和动作规则。
    pub rules: Vec<StrategyRuleDocument>,
}

/// 一条可持久化的受限策略规则。
#[cfg(feature = "serde")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategyRuleDocument {
    /// 条件树。
    pub condition: ConditionDocument,
    /// 命中条件时执行的机会桶动作。
    pub action: PolicyActionDocument,
}

/// 可持久化的白名单指标定义。
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum IndicatorDocument {
    /// 当期可得收盘价。
    ClosePrice,
    /// 简单移动平均线及其交易日窗口。
    SimpleMovingAverage {
        /// 交易日回看窗口。
        lookback_days: u16,
    },
    /// 指数移动平均线及其交易日窗口。
    ExponentialMovingAverage {
        /// 交易日回看窗口。
        lookback_days: u16,
    },
    /// RSI 及其交易日窗口。
    RelativeStrengthIndex {
        /// 交易日回看窗口。
        lookback_days: u16,
    },
    /// 相对峰值回撤及其交易日窗口。
    Drawdown {
        /// 交易日回看窗口。
        lookback_days: u16,
    },
    /// Cboe VIX 水平。
    Vix,
}

/// 可持久化的白名单数值表达式。
#[cfg(feature = "serde")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ValueExpressionDocument {
    /// 固定 Decimal 字符串常数。
    Constant {
        /// Decimal 字符串常数。
        value: String,
    },
    /// 一个白名单市场指标。
    Indicator {
        /// 白名单指标。
        indicator: IndicatorDocument,
    },
    /// 两个表达式之和。
    Add {
        /// 左表达式。
        left: Box<ValueExpressionDocument>,
        /// 右表达式。
        right: Box<ValueExpressionDocument>,
    },
    /// 两个表达式之差。
    Subtract {
        /// 左表达式。
        left: Box<ValueExpressionDocument>,
        /// 右表达式。
        right: Box<ValueExpressionDocument>,
    },
    /// 表达式乘以 Decimal 字符串常数。
    Multiply {
        /// 原表达式。
        expression: Box<ValueExpressionDocument>,
        /// 乘数。
        factor: String,
    },
    /// 表达式除以非零 Decimal 字符串常数。
    Divide {
        /// 原表达式。
        expression: Box<ValueExpressionDocument>,
        /// 非零除数。
        divisor: String,
    },
}

/// 可持久化的条件树。
#[cfg(feature = "serde")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConditionDocument {
    /// 表达式与固定阈值的比较。
    Comparison {
        /// 被比较的表达式。
        expression: ValueExpressionDocument,
        /// 支持的比较符。
        operator: ComparisonOperatorDocument,
        /// Decimal 字符串阈值。
        threshold: String,
    },
    /// 所有子条件必须成立。
    All {
        /// 子条件。
        conditions: Vec<ConditionDocument>,
    },
    /// 任一子条件成立即可。
    Any {
        /// 子条件。
        conditions: Vec<ConditionDocument>,
    },
}

/// JSON 文档允许使用的比较符。
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperatorDocument {
    /// 大于。
    GreaterThan,
    /// 大于或等于。
    GreaterThanOrEqual,
    /// 小于。
    LessThan,
    /// 小于或等于。
    LessThanOrEqual,
}

/// JSON 文档允许使用的机会桶动作。
#[cfg(feature = "serde")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PolicyActionDocument {
    /// 为机会桶设置固定 Decimal 字符串金额。
    SetOpportunityFixedAmount {
        /// Decimal 字符串金额。
        amount: String,
    },
    /// 为机会桶设置有界倍率。
    SetOpportunityMultiplier {
        /// `[0.0, 1.5]` 内的有限倍率。
        multiplier: f64,
    },
    /// 跳过当期机会桶。
    SkipOpportunity,
}

#[cfg(feature = "serde")]
impl StrategySpecDocument {
    /// 从已校验领域策略生成规范化的持久化文档。
    #[must_use]
    pub fn from_strategy_spec(spec: &StrategySpec) -> Self {
        Self {
            policy_id: spec.policy.id().as_str().to_owned(),
            policy_version: spec.policy.version().value(),
            name: spec.name.clone(),
            rules: spec
                .rules
                .iter()
                .map(StrategyRuleDocument::from_rule)
                .collect(),
        }
    }

    /// 将反序列化文档转换回完整校验的领域策略。
    ///
    /// # 错误
    ///
    /// 文档包含不合法 Decimal、倍率、窗口或策略结构时返回
    /// [`StrategyDslDocumentError`]，不会产生部分有效策略。
    pub fn into_strategy_spec(self) -> Result<StrategySpec, StrategyDslDocumentError> {
        let policy = PolicyRef::new(
            strategy_policy::PolicyId::new(self.policy_id)?,
            strategy_policy::PolicyVersion::new(self.policy_version)?,
        );
        let rules = self
            .rules
            .into_iter()
            .map(StrategyRuleDocument::into_rule)
            .collect::<Result<Vec<_>, _>>()?;
        StrategySpec::new(policy, self.name, rules).map_err(Into::into)
    }
}

#[cfg(feature = "serde")]
impl StrategyRuleDocument {
    fn from_rule(rule: &StrategyRule) -> Self {
        Self {
            condition: ConditionDocument::from_condition(&rule.condition),
            action: PolicyActionDocument::from_action(&rule.action),
        }
    }

    fn into_rule(self) -> Result<StrategyRule, StrategyDslDocumentError> {
        Ok(StrategyRule::new(
            self.condition.into_condition()?,
            self.action.into_action()?,
        ))
    }
}

#[cfg(feature = "serde")]
impl IndicatorDocument {
    fn from_indicator(indicator: IndicatorSpec) -> Self {
        match indicator {
            IndicatorSpec::ClosePrice => Self::ClosePrice,
            IndicatorSpec::SimpleMovingAverage(window) => Self::SimpleMovingAverage {
                lookback_days: window.days(),
            },
            IndicatorSpec::ExponentialMovingAverage(window) => Self::ExponentialMovingAverage {
                lookback_days: window.days(),
            },
            IndicatorSpec::RelativeStrengthIndex(window) => Self::RelativeStrengthIndex {
                lookback_days: window.days(),
            },
            IndicatorSpec::Drawdown(window) => Self::Drawdown {
                lookback_days: window.days(),
            },
            IndicatorSpec::Vix => Self::Vix,
        }
    }

    fn into_indicator(self) -> Result<IndicatorSpec, StrategyDslDocumentError> {
        let window = |days| LookbackWindow::new(days).map_err(StrategyDslDocumentError::from);
        match self {
            Self::ClosePrice => Ok(IndicatorSpec::ClosePrice),
            Self::SimpleMovingAverage { lookback_days } => {
                Ok(IndicatorSpec::SimpleMovingAverage(window(lookback_days)?))
            }
            Self::ExponentialMovingAverage { lookback_days } => Ok(
                IndicatorSpec::ExponentialMovingAverage(window(lookback_days)?),
            ),
            Self::RelativeStrengthIndex { lookback_days } => {
                Ok(IndicatorSpec::RelativeStrengthIndex(window(lookback_days)?))
            }
            Self::Drawdown { lookback_days } => Ok(IndicatorSpec::Drawdown(window(lookback_days)?)),
            Self::Vix => Ok(IndicatorSpec::Vix),
        }
    }
}

#[cfg(feature = "serde")]
impl ValueExpressionDocument {
    fn from_expression(expression: &ValueExpression) -> Self {
        match &expression.0 {
            ExpressionKind::Constant(value) => Self::Constant {
                value: value.to_string(),
            },
            ExpressionKind::Indicator(indicator) => Self::Indicator {
                indicator: IndicatorDocument::from_indicator(*indicator),
            },
            ExpressionKind::Add(left, right) => Self::Add {
                left: Box::new(Self::from_expression(left)),
                right: Box::new(Self::from_expression(right)),
            },
            ExpressionKind::Subtract(left, right) => Self::Subtract {
                left: Box::new(Self::from_expression(left)),
                right: Box::new(Self::from_expression(right)),
            },
            ExpressionKind::Multiply(expression, factor) => Self::Multiply {
                expression: Box::new(Self::from_expression(expression)),
                factor: factor.to_string(),
            },
            ExpressionKind::Divide(expression, divisor) => Self::Divide {
                expression: Box::new(Self::from_expression(expression)),
                divisor: divisor.value().to_string(),
            },
        }
    }

    fn into_expression(self) -> Result<ValueExpression, StrategyDslDocumentError> {
        match self {
            Self::Constant { value } => Ok(ValueExpression::constant(parse_decimal(value)?)),
            Self::Indicator { indicator } => {
                Ok(ValueExpression::indicator(indicator.into_indicator()?))
            }
            Self::Add { left, right } => Ok(ValueExpression::sum(
                left.into_expression()?,
                right.into_expression()?,
            )),
            Self::Subtract { left, right } => Ok(ValueExpression::subtract(
                left.into_expression()?,
                right.into_expression()?,
            )),
            Self::Multiply { expression, factor } => Ok(ValueExpression::multiply(
                expression.into_expression()?,
                parse_decimal(factor)?,
            )),
            Self::Divide {
                expression,
                divisor,
            } => Ok(ValueExpression::divide(
                expression.into_expression()?,
                NonZeroDecimal::new(parse_decimal(divisor)?)?,
            )),
        }
    }
}

#[cfg(feature = "serde")]
impl ConditionDocument {
    fn from_condition(condition: &Condition) -> Self {
        match &condition.0 {
            ConditionKind::Comparison {
                expression,
                operator,
                threshold,
            } => Self::Comparison {
                expression: ValueExpressionDocument::from_expression(expression),
                operator: ComparisonOperatorDocument::from_operator(*operator),
                threshold: threshold.to_string(),
            },
            ConditionKind::All(conditions) => Self::All {
                conditions: conditions.iter().map(Self::from_condition).collect(),
            },
            ConditionKind::Any(conditions) => Self::Any {
                conditions: conditions.iter().map(Self::from_condition).collect(),
            },
        }
    }

    fn into_condition(self) -> Result<Condition, StrategyDslDocumentError> {
        match self {
            Self::Comparison {
                expression,
                operator,
                threshold,
            } => Ok(Condition::compare(
                expression.into_expression()?,
                operator.into_operator(),
                parse_decimal(threshold)?,
            )),
            Self::All { conditions } => Ok(Condition::all(
                conditions
                    .into_iter()
                    .map(Self::into_condition)
                    .collect::<Result<Vec<_>, _>>()?,
            )?),
            Self::Any { conditions } => Ok(Condition::any(
                conditions
                    .into_iter()
                    .map(Self::into_condition)
                    .collect::<Result<Vec<_>, _>>()?,
            )?),
        }
    }
}

#[cfg(feature = "serde")]
impl ComparisonOperatorDocument {
    fn from_operator(value: ComparisonOperator) -> Self {
        match value {
            ComparisonOperator::GreaterThan => Self::GreaterThan,
            ComparisonOperator::GreaterThanOrEqual => Self::GreaterThanOrEqual,
            ComparisonOperator::LessThan => Self::LessThan,
            ComparisonOperator::LessThanOrEqual => Self::LessThanOrEqual,
        }
    }

    fn into_operator(self) -> ComparisonOperator {
        match self {
            Self::GreaterThan => ComparisonOperator::GreaterThan,
            Self::GreaterThanOrEqual => ComparisonOperator::GreaterThanOrEqual,
            Self::LessThan => ComparisonOperator::LessThan,
            Self::LessThanOrEqual => ComparisonOperator::LessThanOrEqual,
        }
    }
}

#[cfg(feature = "serde")]
impl PolicyActionDocument {
    fn from_action(action: &PolicyAction) -> Self {
        match &action.0 {
            PolicyActionKind::SetOpportunityFixedAmount(amount) => {
                Self::SetOpportunityFixedAmount {
                    amount: amount.to_string(),
                }
            }
            PolicyActionKind::SetOpportunityMultiplier(multiplier) => {
                Self::SetOpportunityMultiplier {
                    multiplier: multiplier.value(),
                }
            }
            PolicyActionKind::SkipOpportunity => Self::SkipOpportunity,
        }
    }

    fn into_action(self) -> Result<PolicyAction, StrategyDslDocumentError> {
        match self {
            Self::SetOpportunityFixedAmount { amount } => Ok(
                PolicyAction::set_opportunity_fixed_amount(parse_decimal(amount)?)?,
            ),
            Self::SetOpportunityMultiplier { multiplier } => {
                if !multiplier.is_finite()
                    || !(Multiplier::MIN.value()..=Multiplier::MAX.value()).contains(&multiplier)
                {
                    return Err(StrategyDslDocumentError::InvalidMultiplier);
                }
                Ok(PolicyAction::set_opportunity_multiplier(
                    Multiplier::new_clamped(multiplier),
                ))
            }
            Self::SkipOpportunity => Ok(PolicyAction::skip_opportunity()),
        }
    }
}

#[cfg(feature = "serde")]
fn parse_decimal(value: String) -> Result<Decimal, StrategyDslDocumentError> {
    value
        .parse::<Decimal>()
        .map_err(|_| StrategyDslDocumentError::InvalidDecimal)
}

/// 持久化 DSL 文档无法重新通过领域校验。
#[cfg(feature = "serde")]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StrategyDslDocumentError {
    /// 一个 Decimal 字符串不是合法有限十进制数。
    #[error("strategy document contains an invalid decimal")]
    InvalidDecimal,
    /// 一个倍率不是 `[0.0, 1.5]` 内的有限数。
    #[error("strategy document contains an invalid multiplier")]
    InvalidMultiplier,
    /// 策略标识或版本不满足策略领域不变量。
    #[error(transparent)]
    Policy(#[from] strategy_policy::PolicyValidationError),
    /// DSL AST 未通过白名单或复杂度校验。
    #[error(transparent)]
    Validation(#[from] StrategyDslValidationError),
}

#[cfg(test)]
mod tests {
    use core_domain::Multiplier;
    use strategy_policy::{PolicyId, PolicyVersion};
    use time::{Date, Month};

    use super::*;

    fn policy() -> PolicyRef {
        PolicyRef::new(
            PolicyId::new("dsl_value_guard").unwrap(),
            PolicyVersion::new(1).unwrap(),
        )
    }

    fn condition() -> Condition {
        Condition::compare(
            ValueExpression::indicator(IndicatorSpec::RelativeStrengthIndex(
                LookbackWindow::new(14).unwrap(),
            )),
            ComparisonOperator::LessThan,
            Decimal::new(30, 0),
        )
    }

    fn context(evidence: DslEvidence) -> DecisionContext<DslEvidence> {
        DecisionContext::new(
            Date::from_calendar_date(2026, Month::January, 15).unwrap(),
            Decimal::new(100, 0),
            evidence,
        )
        .unwrap()
    }

    fn date(day: u8) -> Date {
        Date::from_calendar_date(2026, Month::January, day).unwrap()
    }

    /// Verify a bounded, white-listed rule can be saved and checked against a plan budget.
    #[test]
    fn accepts_a_safe_custom_strategy_and_budget() {
        let strategy = StrategySpec::new(
            policy(),
            "  RSI opportunity guard  ",
            vec![StrategyRule::new(
                condition(),
                PolicyAction::set_opportunity_fixed_amount(Decimal::new(100, 0)).unwrap(),
            )],
        )
        .unwrap();

        assert_eq!(strategy.name(), "RSI opportunity guard");
        assert_eq!(strategy.policy().to_string(), "dsl_value_guard@1");
        assert_eq!(strategy.rules().len(), 1);
        assert_eq!(strategy.validate_for_budget(Decimal::new(100, 0)), Ok(()));
    }

    /// Verify public constructors reject invalid invariant values before a strategy can contain them.
    #[test]
    fn rejects_invalid_windows_divisors_condition_groups_and_fixed_amounts() {
        assert_eq!(
            LookbackWindow::new(1),
            Err(StrategyDslValidationError::InvalidLookbackWindow)
        );
        assert_eq!(
            NonZeroDecimal::new(Decimal::ZERO),
            Err(StrategyDslValidationError::ZeroDivisor)
        );
        assert_eq!(
            Condition::all(vec![]),
            Err(StrategyDslValidationError::EmptyConditionGroup)
        );
        assert_eq!(
            PolicyAction::set_opportunity_fixed_amount(Decimal::ZERO),
            Err(StrategyDslValidationError::InvalidFixedAmount)
        );
    }

    /// Verify only versioned custom policy identifiers can define DSL rules.
    #[test]
    fn rejects_builtin_policy_ids_and_empty_rule_sets() {
        let builtin = PolicyRef::new(
            PolicyId::new("fixed_dca").unwrap(),
            PolicyVersion::new(1).unwrap(),
        );
        assert_eq!(
            StrategySpec::new(
                builtin,
                "Fixed DCA copy",
                vec![StrategyRule::new(
                    condition(),
                    PolicyAction::skip_opportunity()
                )]
            ),
            Err(StrategyDslValidationError::CustomPolicyIdRequired)
        );
        assert_eq!(
            StrategySpec::new(policy(), "Empty", vec![]),
            Err(StrategyDslValidationError::InvalidRuleCount)
        );
    }

    /// Verify expression-tree bounds prevent unbounded user-authored nesting.
    #[test]
    fn rejects_an_expression_that_exceeds_the_depth_limit() {
        let mut expression = ValueExpression::indicator(IndicatorSpec::ClosePrice);
        for _ in 0..MAX_EXPRESSION_DEPTH {
            expression = ValueExpression::multiply(expression, Decimal::ONE);
        }
        let deep_condition =
            Condition::compare(expression, ComparisonOperator::GreaterThan, Decimal::ZERO);

        assert_eq!(
            StrategySpec::new(
                policy(),
                "Too deep",
                vec![StrategyRule::new(
                    deep_condition,
                    PolicyAction::skip_opportunity()
                )],
            ),
            Err(StrategyDslValidationError::ExpressionTooComplex)
        );
    }

    /// Verify a policy definition cannot claim a fixed amount above its plan period budget.
    #[test]
    fn rejects_fixed_actions_above_the_plan_budget() {
        let strategy = StrategySpec::new(
            policy(),
            "Budget guard",
            vec![StrategyRule::new(
                condition(),
                PolicyAction::set_opportunity_fixed_amount(Decimal::new(101, 0)).unwrap(),
            )],
        )
        .unwrap();

        assert_eq!(
            strategy.validate_for_budget(Decimal::new(100, 0)),
            Err(StrategyDslValidationError::ActionExceedsBudget)
        );
        assert_eq!(
            strategy.validate_for_budget(Decimal::ZERO),
            Err(StrategyDslValidationError::InvalidBudget)
        );
    }

    /// Verify only core-safe opportunity actions are representable by the first DSL revision.
    #[test]
    fn models_opportunity_actions_without_a_core_bucket_veto() {
        assert!(PolicyAction::set_opportunity_fixed_amount(Decimal::ONE)
            .unwrap()
            .is_opportunity_only());
        let action = PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(1.2));
        assert!(action.is_opportunity_only());
        assert!(PolicyAction::skip_opportunity().is_opportunity_only());
    }

    /// Verify the interpreter uses first-match order and returns a policy recommendation.
    #[test]
    fn evaluates_the_first_matching_rule_deterministically() {
        let rsi = IndicatorSpec::RelativeStrengthIndex(LookbackWindow::new(14).unwrap());
        let strategy = StrategySpec::new(
            policy(),
            "Ordered RSI rules",
            vec![
                StrategyRule::new(
                    Condition::compare(
                        ValueExpression::indicator(rsi),
                        ComparisonOperator::LessThan,
                        Decimal::new(40, 0),
                    ),
                    PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(1.2)),
                ),
                StrategyRule::new(condition(), PolicyAction::skip_opportunity()),
            ],
        )
        .unwrap();
        let evaluation = strategy
            .evaluate(&context(
                DslEvidence::new([(rsi, Decimal::new(25, 0))]).unwrap(),
            ))
            .unwrap();

        assert_eq!(evaluation.matched_rule_index(), Some(0));
        assert_eq!(evaluation.recommendation().multiplier().value(), 1.2);
        assert_eq!(evaluation.recommendation().action(), Action::Overweight);
        assert_eq!(
            evaluation.action(),
            &DslRuntimeAction::OpportunityMultiplier(Multiplier::new_clamped(1.2))
        );
    }

    /// Verify absent evidence fails closed instead of silently substituting an indicator value.
    #[test]
    fn rejects_execution_when_required_evidence_is_missing() {
        let strategy = StrategySpec::new(
            policy(),
            "Required RSI",
            vec![StrategyRule::new(
                condition(),
                PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(1.1)),
            )],
        )
        .unwrap();

        assert_eq!(
            strategy.evaluate(&context(DslEvidence::new([]).unwrap())),
            Err(StrategyDslRuntimeError::MissingIndicator)
        );
    }

    /// Verify a no-match result leaves the opportunity bucket at its standard multiplier.
    #[test]
    fn defaults_to_standard_opportunity_when_no_rule_matches() {
        let rsi = IndicatorSpec::RelativeStrengthIndex(LookbackWindow::new(14).unwrap());
        let strategy = StrategySpec::new(
            policy(),
            "No match default",
            vec![StrategyRule::new(
                Condition::compare(
                    ValueExpression::indicator(rsi),
                    ComparisonOperator::LessThan,
                    Decimal::new(30, 0),
                ),
                PolicyAction::skip_opportunity(),
            )],
        )
        .unwrap();
        let evaluation = strategy
            .evaluate(&context(
                DslEvidence::new([(rsi, Decimal::new(55, 0))]).unwrap(),
            ))
            .unwrap();

        assert_eq!(evaluation.matched_rule_index(), None);
        assert_eq!(evaluation.recommendation().action(), Action::Standard);
        assert_eq!(evaluation.recommendation().multiplier().value(), 1.0);
        assert_eq!(evaluation.action(), &DslRuntimeAction::StandardOpportunity);
    }

    /// Verify every online technical indicator is calculated by the shared deterministic builder.
    #[test]
    fn builds_price_technical_evidence_for_the_runtime_and_backtests() {
        let policy = PolicyRef::new(
            PolicyId::new("dsl_technical_snapshot").unwrap(),
            PolicyVersion::new(1).unwrap(),
        );
        let strategy = StrategySpec::new(
            policy,
            "technical snapshot",
            vec![
                StrategyRule::new(
                    Condition::compare(
                        ValueExpression::indicator(IndicatorSpec::ClosePrice),
                        ComparisonOperator::GreaterThan,
                        Decimal::ZERO,
                    ),
                    PolicyAction::skip_opportunity(),
                ),
                StrategyRule::new(
                    Condition::compare(
                        ValueExpression::indicator(IndicatorSpec::SimpleMovingAverage(
                            LookbackWindow::new(2).unwrap(),
                        )),
                        ComparisonOperator::GreaterThan,
                        Decimal::ZERO,
                    ),
                    PolicyAction::skip_opportunity(),
                ),
                StrategyRule::new(
                    Condition::compare(
                        ValueExpression::indicator(IndicatorSpec::ExponentialMovingAverage(
                            LookbackWindow::new(2).unwrap(),
                        )),
                        ComparisonOperator::GreaterThan,
                        Decimal::ZERO,
                    ),
                    PolicyAction::skip_opportunity(),
                ),
                StrategyRule::new(
                    Condition::compare(
                        ValueExpression::indicator(IndicatorSpec::RelativeStrengthIndex(
                            LookbackWindow::new(2).unwrap(),
                        )),
                        ComparisonOperator::GreaterThan,
                        Decimal::ZERO,
                    ),
                    PolicyAction::skip_opportunity(),
                ),
                StrategyRule::new(
                    Condition::compare(
                        ValueExpression::indicator(IndicatorSpec::Drawdown(
                            LookbackWindow::new(2).unwrap(),
                        )),
                        ComparisonOperator::LessThanOrEqual,
                        Decimal::ZERO,
                    ),
                    PolicyAction::skip_opportunity(),
                ),
                StrategyRule::new(
                    Condition::compare(
                        ValueExpression::indicator(IndicatorSpec::Vix),
                        ComparisonOperator::GreaterThan,
                        Decimal::ZERO,
                    ),
                    PolicyAction::skip_opportunity(),
                ),
            ],
        )
        .unwrap();
        let evidence = DslEvidence::from_market_snapshot(
            &strategy,
            &[
                Decimal::new(100, 0),
                Decimal::new(110, 0),
                Decimal::new(105, 0),
            ],
            Decimal::new(20, 0),
        )
        .unwrap();

        assert_eq!(
            evidence.value(IndicatorSpec::ClosePrice).unwrap(),
            Decimal::new(105, 0)
        );
        assert_eq!(
            evidence
                .value(IndicatorSpec::SimpleMovingAverage(
                    LookbackWindow::new(2).unwrap()
                ))
                .unwrap(),
            Decimal::new(1075, 1)
        );
        assert_eq!(
            evidence.value(IndicatorSpec::Vix).unwrap(),
            Decimal::new(20, 0)
        );
    }

    /// Verify a dated snapshot rejects any observation that would read beyond its decision cutoff.
    #[test]
    fn dated_market_snapshot_rejects_future_or_duplicate_observations() {
        let close = TechnicalClose::new(date(2), Decimal::new(100, 0)).unwrap();
        let vix = TechnicalVix::new(date(2), Decimal::new(20, 0)).unwrap();
        assert_eq!(
            TechnicalMarketSnapshot::new(date(1), vec![close], vix),
            Err(StrategyDslRuntimeError::NonCausalMarketSnapshot)
        );

        let duplicate = vec![
            TechnicalClose::new(date(1), Decimal::new(99, 0)).unwrap(),
            TechnicalClose::new(date(1), Decimal::new(100, 0)).unwrap(),
        ];
        assert_eq!(
            TechnicalMarketSnapshot::new(
                date(1),
                duplicate,
                TechnicalVix::new(date(1), Decimal::new(20, 0)).unwrap(),
            ),
            Err(StrategyDslRuntimeError::NonCausalMarketSnapshot)
        );
    }

    /// Verify dated evidence uses only closes at or before the requested cutoff.
    #[test]
    fn dated_market_snapshot_has_no_lookahead_and_requires_warmup() {
        let sma = IndicatorSpec::SimpleMovingAverage(LookbackWindow::new(3).unwrap());
        let strategy = StrategySpec::new(
            policy(),
            "Three day causal SMA",
            vec![StrategyRule::new(
                Condition::compare(
                    ValueExpression::indicator(sma),
                    ComparisonOperator::GreaterThan,
                    Decimal::ZERO,
                ),
                PolicyAction::skip_opportunity(),
            )],
        )
        .unwrap();
        let closes = vec![
            TechnicalClose::new(date(1), Decimal::new(100, 0)).unwrap(),
            TechnicalClose::new(date(2), Decimal::new(110, 0)).unwrap(),
            TechnicalClose::new(date(3), Decimal::new(120, 0)).unwrap(),
        ];
        let snapshot = TechnicalMarketSnapshot::new(
            date(3),
            closes.clone(),
            TechnicalVix::new(date(2), Decimal::new(20, 0)).unwrap(),
        )
        .unwrap();
        let evidence = DslEvidence::from_as_of_market_snapshot(&strategy, &snapshot).unwrap();
        assert_eq!(evidence.value(sma).unwrap(), Decimal::new(110, 0));

        let insufficient = TechnicalMarketSnapshot::new(
            date(2),
            closes[..2].to_vec(),
            TechnicalVix::new(date(2), Decimal::new(20, 0)).unwrap(),
        )
        .unwrap();
        assert_eq!(
            DslEvidence::from_as_of_market_snapshot(&strategy, &insufficient),
            Err(StrategyDslRuntimeError::MissingIndicator)
        );
    }
}
