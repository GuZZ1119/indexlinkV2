import { describe, expect, it } from 'vitest'

import { buildNormalizedStrategyAnalysis, compareStrategies, connectionLabel, consumerStrategies, findConsumerStrategy } from './model'

describe('consumer strategy model', () => {
  it('keeps the curated library readable and finds a selected strategy', () => {
    expect(consumerStrategies).toHaveLength(3)
    expect(findConsumerStrategy('steady-dca').shortName).toBe('固定定投')
  })

  it('uses the 70/20/10 strategy as a safe default when no selection exists', () => {
    expect(findConsumerStrategy(null).id).toBe('adaptive-70-20-10')
  })

  it('builds a small comparison with the strategy limitations included', () => {
    const rows = compareStrategies('adaptive-70-20-10', 'defensive-balance')
    expect(rows.map((row) => row.label)).toEqual(['适合的人', '执行节奏', '历史年化', '最大回撤', '最该知道的限制'])
    expect(rows.at(-1)?.left).toContain('预测')
  })

  it('keeps local-only and unconfigured integrations visually distinct', () => {
    expect(connectionLabel('local-only')).toBe('仅本机可用')
    expect(connectionLabel('not-configured')).toBe('尚未配置')
  })

  it('rebases every selected strategy to 100 before comparing a shared range', () => {
    const analysis = buildNormalizedStrategyAnalysis(['steady-dca', 'adaptive-70-20-10'], '3y')
    expect(analysis.points).toHaveLength(37)
    expect(analysis.points[0]['steady-dca']).toBe(100)
    expect(analysis.points[0]['adaptive-70-20-10']).toBe(100)
    expect(analysis.summaries.map((summary) => summary.id)).toEqual(['steady-dca', 'adaptive-70-20-10'])
    expect(analysis.summaries[0].endIndex).not.toBe(100)
  })
})
