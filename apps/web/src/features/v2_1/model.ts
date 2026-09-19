export type StrategyId = 'steady-dca' | 'ma200-trend-guard' | 'growth-volatility-balance'

export interface ConsumerStrategy {
  id: StrategyId
  name: string
  shortName: string
  summary: string
  fit: string
  cadence: string
  rule: string
  strength: string
  limitation: string
  risk: '稳健' | '平衡' | '进取'
  annualizedReturn: string
  maxDrawdown: string
  sample: string
}

export const consumerStrategies: readonly ConsumerStrategy[] = [
  {
    id: 'steady-dca',
    name: '每月稳步投入',
    shortName: '固定定投',
    summary: '在固定日期，用固定金额持续买入宽基指数。',
    fit: '想先养成长期投资习惯的人',
    cadence: '每月一次',
    rule: '无论市场涨跌，按计划投入。',
    strength: '规则最简单，不需要判断市场。',
    limitation: '市场极端高估时，仍会按原金额买入。',
    risk: '稳健',
    annualizedReturn: '8.1%',
    maxDrawdown: '-33.4%',
    sample: '2014–2024 · 美股宽基示例',
  },
  {
    id: 'ma200-trend-guard',
    name: '200 日均线趋势保护',
    shortName: 'MA200 保护',
    summary: '固定核心投入不变，只在价格低于 200 日均线时暂停弹性投入。',
    fit: '希望长期投入，同时给弱趋势留出缓冲的人',
    cadence: '每月一次',
    rule: '低于 200 日均线时暂停 30% 弹性桶，70% 核心桶继续。',
    strength: '规则简单，可直接从每日收盘价复核。',
    limitation: '均线具有滞后性，快速反转时可能错过部分恢复。',
    risk: '稳健',
    annualizedReturn: '查看真实研究',
    maxDrawdown: '查看真实研究',
    sample: 'technical-v1 固定样本',
  },
  {
    id: 'growth-volatility-balance',
    name: '增长与波动平衡',
    shortName: '增长 × 波动',
    summary: '用 126 日增长和 63 日年化波动率调整弹性投入。',
    fit: '能接受固定阈值，并希望兼顾趋势与波动的人',
    cadence: '每月一次',
    rule: '高波动时弹性桶减半，增长为正且平稳时提高到 1.2 倍。',
    strength: '两个指标都来自同一收盘价模板，规则可复核。',
    limitation: '阈值不预测未来，震荡期可能在不同档位间切换。',
    risk: '平衡',
    annualizedReturn: '查看真实研究',
    maxDrawdown: '查看真实研究',
    sample: 'technical-v1 固定样本',
  },
] as const

export function findConsumerStrategy(id: StrategyId | null): ConsumerStrategy {
  return consumerStrategies.find((strategy) => strategy.id === id) ?? consumerStrategies[0]
}

export interface ComparisonRow {
  label: string
  left: string
  right: string
}

export const strategyAnalysisRanges = [
  { id: '1y', label: '近 1 年', months: 12 },
  { id: '3y', label: '近 3 年', months: 36 },
  { id: 'all', label: '全部样本', months: 60 },
] as const

export type StrategyAnalysisRange = (typeof strategyAnalysisRanges)[number]['id']

export type StrategyAnalysisPoint = { date: string } & Partial<Record<StrategyId, number>>

export interface StrategyAnalysisSummary {
  id: StrategyId
  endIndex: number
  change: number
}

const analysisProfiles: Record<StrategyId, { drift: number; marketSensitivity: number; rhythm: number; setback: number }> = {
  'steady-dca': { drift: 0.55, marketSensitivity: 0.78, rhythm: 0.14, setback: 0.23 },
  'ma200-trend-guard': { drift: 0.51, marketSensitivity: 0.67, rhythm: 0.1, setback: 0.13 },
  'growth-volatility-balance': { drift: 0.58, marketSensitivity: 0.72, rhythm: 0.12, setback: 0.17 },
}

/**
 * Build a deterministic local preview for the analysis shell. Each selected series
 * is rebased to 100 at the first point so the page compares the experience, never
 * account size. This is intentionally not a substitute for versioned backtest data.
 */
export function buildNormalizedStrategyAnalysis(ids: readonly StrategyId[], range: StrategyAnalysisRange): { points: StrategyAnalysisPoint[]; summaries: StrategyAnalysisSummary[] } {
  const months = strategyAnalysisRanges.find((candidate) => candidate.id === range)?.months ?? 60
  const selectedIds = [...new Set(ids)]
  const startMonth = 60 - months
  const values = new Map<StrategyId, number>(selectedIds.map((id) => [id, 100]))
  const points: StrategyAnalysisPoint[] = []

  for (let offset = 0; offset <= months; offset += 1) {
    const month = startMonth + offset
    const point: StrategyAnalysisPoint = { date: analysisDate(month) }
    for (const id of selectedIds) {
      const nextValue = offset === 0 ? 100 : Number(((values.get(id) ?? 100) * (1 + previewMonthlyReturn(id, month))).toFixed(2))
      values.set(id, nextValue)
      point[id] = nextValue
    }
    points.push(point)
  }

  return {
    points,
    summaries: selectedIds.map((id) => {
      const endIndex = values.get(id) ?? 100
      return { id, endIndex, change: Number((endIndex - 100).toFixed(1)) }
    }),
  }
}

function previewMonthlyReturn(id: StrategyId, month: number): number {
  const profile = analysisProfiles[id]
  const broadMarket = Math.sin(month * 0.72) * 0.018 + Math.cos(month * 0.23) * 0.011 - (month % 17 === 0 ? 0.035 : 0)
  const strategyRhythm = Math.sin(month * 0.39 + profile.marketSensitivity) * profile.rhythm / 100
  const drawdownBuffer = broadMarket < 0 ? profile.setback / 100 : 0
  return profile.drift / 100 + broadMarket * profile.marketSensitivity + strategyRhythm - drawdownBuffer
}

function analysisDate(month: number): string {
  const date = new Date(Date.UTC(2021, 5 + month, 1))
  return new Intl.DateTimeFormat('zh-CN', { year: 'numeric', month: 'short', timeZone: 'UTC' }).format(date)
}

export const strategyAnalysisColors: Record<StrategyId, string> = {
  'steady-dca': '#50738a',
  'ma200-trend-guard': '#2d6a57',
  'growth-volatility-balance': '#ad7d35',
}

/** Build a deliberately small, comparable view without pretending these demo figures are live data. */
export function compareStrategies(leftId: StrategyId, rightId: StrategyId): ComparisonRow[] {
  const left = findConsumerStrategy(leftId)
  const right = findConsumerStrategy(rightId)
  return [
    { label: '适合的人', left: left.fit, right: right.fit },
    { label: '执行节奏', left: left.cadence, right: right.cadence },
    { label: '历史年化', left: left.annualizedReturn, right: right.annualizedReturn },
    { label: '最大回撤', left: left.maxDrawdown, right: right.maxDrawdown },
    { label: '最该知道的限制', left: left.limitation, right: right.limitation },
  ]
}

export type LabConnectionState = 'not-configured' | 'local-only'

export function connectionLabel(state: LabConnectionState): string {
  return state === 'local-only' ? '仅本机可用' : '尚未配置'
}
