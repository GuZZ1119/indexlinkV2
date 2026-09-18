export type StrategyId = 'steady-dca' | 'adaptive-70-20-10' | 'defensive-balance'

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
    id: 'adaptive-70-20-10',
    name: '自适应长期计划',
    shortName: '70 / 20 / 10',
    summary: '以历史估值位置为主，趋势和新闻情绪为辅，微调每月投入。',
    fit: '愿意多看一眼原因，但不想盯盘的人',
    cadence: '每月一次',
    rule: '相对低位多投一点，过热时放慢节奏。',
    strength: '在不改变长期方向的前提下，给执行节奏一点弹性。',
    limitation: '它不预测涨跌；极端行情中历史规律可能失效。',
    risk: '平衡',
    annualizedReturn: '8.6%',
    maxDrawdown: '-31.8%',
    sample: '2014–2024 · 美股宽基示例',
  },
  {
    id: 'defensive-balance',
    name: '稳中有进组合',
    shortName: '股债平衡',
    summary: '用股票指数与短债组合降低波动，并在固定日期恢复目标配比。',
    fit: '更在意波动感受与持续持有的人',
    cadence: '每季度检查',
    rule: '维持股票与短债的目标比例，偏离后再平衡。',
    strength: '回撤通常更温和，更容易长期坚持。',
    limitation: '上行很快的股票市场里，可能明显落后于全股票方案。',
    risk: '稳健',
    annualizedReturn: '6.7%',
    maxDrawdown: '-19.6%',
    sample: '2014–2024 · 美股 ETF 示例',
  },
] as const

export function findConsumerStrategy(id: StrategyId | null): ConsumerStrategy {
  return consumerStrategies.find((strategy) => strategy.id === id) ?? consumerStrategies[1]
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
  'adaptive-70-20-10': { drift: 0.62, marketSensitivity: 0.72, rhythm: 0.11, setback: 0.15 },
  'defensive-balance': { drift: 0.43, marketSensitivity: 0.44, rhythm: 0.08, setback: 0.08 },
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
  'adaptive-70-20-10': '#2d6a57',
  'defensive-balance': '#ad7d35',
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
