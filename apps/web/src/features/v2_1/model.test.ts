import { describe, expect, it } from 'vitest'

import { connectionLabel, strategyAnalysisColor, strategyAnalysisRanges } from './model'

describe('consumer strategy model', () => {
  it('keeps local-only and unconfigured integrations visually distinct', () => {
    expect(connectionLabel('local-only')).toBe('仅本机可用')
    expect(connectionLabel('not-configured')).toBe('尚未配置')
  })

  it('keeps all seven real backtest ranges and assigns stable dynamic colors', () => {
    expect(strategyAnalysisRanges.map((range) => range.id)).toEqual(['1m', '3m', '6m', '1y', '3y', '5y', 'all'])
    expect(strategyAnalysisColor('dsl_price_sma_responsive')).toBe(strategyAnalysisColor('dsl_price_sma_responsive'))
    expect(strategyAnalysisColor('dsl_price_sma_responsive')).not.toBe(strategyAnalysisColor('dsl_growth_vol_responsive'))
  })
})
