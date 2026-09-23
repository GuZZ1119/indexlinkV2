import { proxy } from 'valtio'

import type { StrategyId } from '@/features/v2_1/model'
import type { PolicyReference, StrategyBacktestRange, StrategyBacktestRequest } from '@/api/types'

export type ResearchMetricKey = 'total_return_percent' | 'annualized_return_percent' | 'xirr_percent' | 'maximum_drawdown_percent' | 'annualized_volatility_percent' | 'sortino_ratio' | 'cash_utilisation_percent'

/** Browser-only UI state; server data remains in React Query. */
export const uiStore = proxy<{ selectedPlanId: string | null; activeStrategyId: StrategyId }>({
  selectedPlanId: null,
  activeStrategyId: 'fixed_dca',
})

/** Select the plan used by the Dashboard and decision-history pages. */
export function setSelectedPlanId(planId: string | null) {
  uiStore.selectedPlanId = planId
}

/** Switch the local-first strategy a user is currently exploring or following. */
export function setActiveStrategyId(strategyId: StrategyId) {
  uiStore.activeStrategyId = strategyId
}

/** Draft and last-submitted controls for the real strategy analysis page. */
export const strategyAnalysisStore = proxy<{
  symbol: string
  range: StrategyBacktestRange
  strategyRefs: PolicyReference[]
  monthlyDay: number
  contribution: string
  view: 'plain' | 'research'
  researchMetric: ResearchMetricKey
  researchStrategyId: string | null
  appliedRouteStrategyId: string | null
  submitted: StrategyBacktestRequest
}>({
  symbol: 'US.SPY',
  range: '3y',
  strategyRefs: [{ id: 'fixed_dca', version: 1 }],
  monthlyDay: 18,
  contribution: '1000.00',
  view: 'plain',
  researchMetric: 'total_return_percent',
  researchStrategyId: null,
  appliedRouteStrategyId: null,
  submitted: {
    symbol: 'US.SPY',
    strategy_refs: [{ policy_id: 'fixed_dca', policy_version: 1 }],
    range: '3y',
    monthly_day: 18,
    contribution: '1000.00',
  },
})

/** Toggle a local comparison selection while preserving at least one and at most three. */
export function toggleAnalysisStrategy(policy: PolicyReference) {
  const current = strategyAnalysisStore.strategyRefs
  const key = policyKey(policy)
  if (current.some((item) => policyKey(item) === key)) {
    if (current.length > 1) strategyAnalysisStore.strategyRefs = current.filter((item) => policyKey(item) !== key)
  } else if (current.length < 3) {
    strategyAnalysisStore.strategyRefs = [...current, policy]
  }
}

/** Freeze the current draft into the React Query request key. */
export function submitStrategyAnalysis() {
  strategyAnalysisStore.submitted = {
    symbol: strategyAnalysisStore.symbol.trim().toUpperCase(),
    strategy_refs: strategyAnalysisStore.strategyRefs.map((policy) => ({ policy_id: policy.id, policy_version: policy.version })),
    range: strategyAnalysisStore.range,
    monthly_day: strategyAnalysisStore.monthlyDay,
    contribution: strategyAnalysisStore.contribution.trim(),
  }
}

/** Apply a deep-linked strategy and immediately run it on the current draft instrument. */
export function openStrategyAnalysis(policy: PolicyReference) {
  if (!policy.id.trim()) return
  strategyAnalysisStore.strategyRefs = [policy]
  submitStrategyAnalysis()
}

/** Restore deterministic defaults for isolated tests and explicit reset actions. */
export function resetStrategyAnalysis() {
  strategyAnalysisStore.symbol = 'US.SPY'
  strategyAnalysisStore.range = '3y'
  strategyAnalysisStore.strategyRefs = [{ id: 'fixed_dca', version: 1 }]
  strategyAnalysisStore.monthlyDay = 18
  strategyAnalysisStore.contribution = '1000.00'
  strategyAnalysisStore.view = 'plain'
  strategyAnalysisStore.researchMetric = 'total_return_percent'
  strategyAnalysisStore.researchStrategyId = null
  strategyAnalysisStore.appliedRouteStrategyId = null
  submitStrategyAnalysis()
}

/** Stable key for one immutable policy version in browser-only selection state. */
export function policyKey(policy: PolicyReference): string {
  return `${policy.id}@${policy.version}`
}
