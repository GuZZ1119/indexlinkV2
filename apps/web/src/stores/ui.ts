import { proxy } from 'valtio'

import type { StrategyId } from '@/features/v2_1/model'

/** Browser-only UI state; server data remains in React Query. */
export const uiStore = proxy<{ selectedPlanId: string | null; activeStrategyId: StrategyId }>({
  selectedPlanId: null,
  activeStrategyId: 'steady-dca',
})

/** Select the plan used by the Dashboard and decision-history pages. */
export function setSelectedPlanId(planId: string | null) {
  uiStore.selectedPlanId = planId
}

/** Switch the local-first strategy a user is currently exploring or following. */
export function setActiveStrategyId(strategyId: StrategyId) {
  uiStore.activeStrategyId = strategyId
}
