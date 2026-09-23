import type {
  StrategyComparisonDocument,
  StrategyConditionDocument,
  StrategyIndicatorDocument,
  StrategyRuleDocument,
  StrategySpecDocument,
} from '@/api/types'

export const MAX_PERSONAL_RULES = 3
export const MAX_CONDITIONS_PER_RULE = 3
export const PERSONAL_MULTIPLIERS = [0, 0.5, 1, 1.2] as const

export type PersonalMultiplier = (typeof PERSONAL_MULTIPLIERS)[number]

export interface PersonalConditionDraft {
  indicator: StrategyIndicatorDocument['kind']
  lookbackDays: number
  operator: StrategyComparisonDocument['operator']
  threshold: string
}

export interface PersonalRuleDraft {
  match: 'all' | 'any'
  conditions: PersonalConditionDraft[]
  multiplier: PersonalMultiplier
}

export interface PersonalStrategyDraft {
  name: string
  rules: PersonalRuleDraft[]
}

const WINDOWLESS_INDICATORS = new Set<StrategyIndicatorDocument['kind']>(['close_price', 'vix'])
const PERCENT_INDICATORS = new Set<StrategyIndicatorDocument['kind']>([
  'price_return',
  'annualized_volatility',
  'price_percentile',
  'moving_average_distance',
  'drawdown',
])
const PERSONAL_INDICATORS = new Set<StrategyIndicatorDocument['kind']>([
  'price_return',
  'annualized_volatility',
  'price_percentile',
  'moving_average_distance',
  'relative_strength_index',
  'drawdown',
  'close_price',
])

/** Start from a useful, legible rule rather than exposing the transport DSL. */
export function createPersonalStrategyDraft(): PersonalStrategyDraft {
  return {
    name: '',
    rules: [{
      match: 'all',
      conditions: [{ indicator: 'price_return', lookbackDays: 63, operator: 'less_than', threshold: '0' }],
      multiplier: 1.2,
    }],
  }
}

/** Build the only document shape the consumer editor is permitted to submit. */
export function buildPersonalStrategyDocument(
  draft: PersonalStrategyDraft,
  policyId: string,
  policyVersion = 1,
): StrategySpecDocument {
  assertPersonalDraft(draft)
  const rules = draft.rules.map<StrategyRuleDocument>((rule) => ({
    condition: conditionDocument(rule),
    action: rule.multiplier === 0
      ? { kind: 'skip_opportunity' }
      : { kind: 'set_opportunity_multiplier', multiplier: rule.multiplier },
  }))
  return { policy_id: policyId, policy_version: policyVersion, name: draft.name.trim(), rules }
}

/** Convert a validated AI candidate into the smaller consumer-editor contract. */
export function personalDraftFromDocument(document: StrategySpecDocument): PersonalStrategyDraft {
  if (document.rules.length < 1 || document.rules.length > MAX_PERSONAL_RULES) {
    throw new Error('AI 草案超出个人工坊最多三条规则的范围')
  }
  const rules = document.rules.map<PersonalRuleDraft>((rule) => {
    const comparisons = rule.condition.kind === 'comparison'
      ? [rule.condition]
      : rule.condition.conditions
    if (comparisons.length < 1 || comparisons.length > MAX_CONDITIONS_PER_RULE) {
      throw new Error('AI 草案中的条件数量超出个人工坊范围')
    }
    const multiplier: PersonalMultiplier = rule.action.kind === 'skip_opportunity'
      ? 0
      : rule.action.kind === 'set_opportunity_multiplier' && PERSONAL_MULTIPLIERS.includes(rule.action.multiplier as PersonalMultiplier)
        ? rule.action.multiplier as PersonalMultiplier
        : (() => { throw new Error('AI 草案包含个人工坊不支持的额度动作') })()
    return {
      match: rule.condition.kind === 'any' ? 'any' : 'all',
      multiplier,
      conditions: comparisons.map((comparison) => {
        const indicator = comparison.expression.indicator
        if (!PERSONAL_INDICATORS.has(indicator.kind)) {
          throw new Error(`AI 草案使用了个人工坊暂不支持的指标：${indicator.kind}`)
        }
        const rawThreshold = Number(comparison.threshold)
        if (!Number.isFinite(rawThreshold)) throw new Error('AI 草案包含无效阈值')
        return {
          indicator: indicator.kind,
          lookbackDays: 'lookback_days' in indicator ? indicator.lookback_days : 20,
          operator: comparison.operator,
          threshold: String(PERCENT_INDICATORS.has(indicator.kind) ? rawThreshold * 100 : rawThreshold),
        }
      }),
    }
  })
  const draft = { name: document.name, rules }
  assertPersonalDraft(draft)
  return draft
}

/** Generate a valid opaque local identity without asking ordinary users to manage IDs. */
export function personalPolicyId(randomPart: string): string {
  const normalized = randomPart.toLowerCase().replace(/[^a-z0-9]/g, '').slice(0, 32)
  if (!normalized) throw new Error('无法生成个人策略标识')
  return `dsl_personal_${normalized}`
}

/** Explain one saved action in the same vocabulary used by the editor. */
export function multiplierLabel(multiplier: PersonalMultiplier): string {
  if (multiplier === 0) return '本期不使用机会额度'
  if (multiplier === 0.5) return '使用 50% 机会额度'
  if (multiplier === 1) return '使用 100% 机会额度'
  return '使用 120% 机会额度'
}

function conditionDocument(rule: PersonalRuleDraft): StrategyConditionDocument {
  const conditions = rule.conditions.map<StrategyComparisonDocument>((condition) => ({
    kind: 'comparison',
    expression: { kind: 'indicator', indicator: indicatorDocument(condition) },
    operator: condition.operator,
    threshold: thresholdDocument(condition),
  }))
  if (conditions.length === 1) return conditions[0]
  return { kind: rule.match, conditions }
}

function thresholdDocument(condition: PersonalConditionDraft): string {
  const value = Number(condition.threshold)
  return PERCENT_INDICATORS.has(condition.indicator) ? String(value / 100) : String(value)
}

function indicatorDocument(condition: PersonalConditionDraft): StrategyIndicatorDocument {
  if (condition.indicator === 'close_price' || condition.indicator === 'vix') {
    return { kind: condition.indicator }
  }
  return { kind: condition.indicator, lookback_days: condition.lookbackDays }
}

function assertPersonalDraft(draft: PersonalStrategyDraft): void {
  if (!draft.name.trim()) throw new Error('请为策略命名')
  if (draft.rules.length < 1 || draft.rules.length > MAX_PERSONAL_RULES) {
    throw new Error(`个人策略需要 1–${MAX_PERSONAL_RULES} 条优先规则`)
  }
  for (const rule of draft.rules) {
    if (rule.conditions.length < 1 || rule.conditions.length > MAX_CONDITIONS_PER_RULE) {
      throw new Error(`每条规则需要 1–${MAX_CONDITIONS_PER_RULE} 个条件`)
    }
    if (!PERSONAL_MULTIPLIERS.includes(rule.multiplier)) throw new Error('机会额度不在允许范围内')
    for (const condition of rule.conditions) {
      if (!condition.threshold.trim() || !Number.isFinite(Number(condition.threshold))) {
        throw new Error('阈值必须是有效数字')
      }
      if (!WINDOWLESS_INDICATORS.has(condition.indicator)
        && (!Number.isInteger(condition.lookbackDays) || condition.lookbackDays < 2 || condition.lookbackDays > 365)) {
        throw new Error('观察窗口必须是 2–365 个交易日')
      }
    }
  }
}
