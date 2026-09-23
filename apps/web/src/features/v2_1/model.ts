/** Immutable backend policy id. The catalog, rather than a frontend union, owns its values. */
export type StrategyId = string

/** Date windows accepted by the real strategy-backtest API. */
export const strategyAnalysisRanges = [
  { id: '1m', label: '近 1 个月', months: 1 },
  { id: '3m', label: '近 3 个月', months: 3 },
  { id: '6m', label: '近 6 个月', months: 6 },
  { id: '1y', label: '近 1 年', months: 12 },
  { id: '3y', label: '近 3 年', months: 36 },
  { id: '5y', label: '近 5 年', months: 60 },
  { id: 'all', label: '全部样本', months: 60 },
] as const

export type StrategyAnalysisRange = (typeof strategyAnalysisRanges)[number]['id']

/** Stable chart color for any server-owned policy id, including future catalog presets. */
export function strategyAnalysisColor(policyId: string): string {
  let hash = 2_166_136_261
  for (let index = 0; index < policyId.length; index += 1) {
    hash ^= policyId.charCodeAt(index)
    hash = Math.imul(hash, 16_777_619)
  }
  return `hsl(${Math.abs(hash) % 360} 42% 38%)`
}

export type LabConnectionState = 'not-configured' | 'local-only'

export function connectionLabel(state: LabConnectionState): string {
  return state === 'local-only' ? '仅本机可用' : '尚未配置'
}
