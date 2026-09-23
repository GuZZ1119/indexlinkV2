import { describe, expect, it } from 'vitest'

import type { DynamicBacktestSeries, StrategyCatalogEntry } from '@/api/types'
import { buildMarketExecutionChartOption, buildNormalizedChartOption } from '@/pages/strategy-analysis/chart-options'
import { buildMarketChartData } from '@/pages/strategy-analysis/market-execution-model'
import { buildAllocationChartOption, buildDrawdownChartOption } from '@/pages/strategy-analysis/research-chart-options'

describe('market execution chart helpers', () => {
  it('joins only executions inside the common market window and prefers catalog names', () => {
    const series = backtestSeries('formula', [
      executionPoint(700, 70, true),
      { ...executionPoint(700, 70, true), date: '2025-12-31', adjusted_close: 99 },
    ])
    const catalog = new Map([['formula', { name: '趋势保护' } as StrategyCatalogEntry]])
    const points = buildMarketChartData([
      { date: '2026-01-02', adjusted_close: 101 },
      { date: '2026-01-05', adjusted_close: 102 },
    ], [series], catalog)

    expect(points).toHaveLength(2)
    expect(points[0].executions).toHaveLength(1)
    expect(points[0].executions[0].strategyName).toBe('趋势保护')
    expect(points[1].executions).toEqual([])

    const fallback = buildMarketChartData([{ date: '2026-01-02', adjusted_close: 101 }], [series], new Map())
    expect(fallback[0].executions[0].strategyName).toBe('formula')
  })

  it('builds synchronized price and strategy lanes with fixed-size markers', () => {
    const first = backtestSeries('first', [executionPoint(700, 70, true), { ...executionPoint(1000, 100, false), date: '2026-01-05', adjusted_close: 102 }])
    const second = backtestSeries('second', [executionPoint(1000, 100, true)])
    const third = { ...backtestSeries('third', [executionPoint(400, 40, true)]), strategy_name: undefined } as unknown as DynamicBacktestSeries
    const catalog = new Map([
      ['first', { name: '趋势保护' } as StrategyCatalogEntry],
      ['second', { name: '固定投入' } as StrategyCatalogEntry],
    ])
    const points = buildMarketChartData([
      { date: '2026-01-02', adjusted_close: 101 },
      { date: '2026-01-05', adjusted_close: 102 },
    ], [first, second, third], catalog)
    const option = buildMarketExecutionChartOption(points, [first, second, third], catalog, 'USD') as unknown as ChartOption

    expect(option.xAxis).toHaveLength(2)
    expect(option.grid[0].left).toBe(option.grid[1].left)
    expect(option.yAxis[1].data).toEqual(['趋势保护', '固定投入', 'third'])
    expect(option.dataZoom[0]).toMatchObject({ type: 'inside', xAxisIndex: [0, 1], zoomOnMouseWheel: true, moveOnMouseMove: true })
    expect(option.dataZoom[1]).toMatchObject({ type: 'slider', xAxisIndex: [0, 1] })
    expect(option.series[1]).toMatchObject({ id: 'price-signal-first', type: 'scatter', symbol: 'diamond', symbolSize: 13 })
    expect(option.series[2]).toMatchObject({ id: 'price-signal-second', type: 'scatter', symbol: 'rect', symbolSize: 13 })
    expect(option.series[3]).toMatchObject({ id: 'price-signal-third', type: 'scatter', symbol: 'triangle', symbolSize: 13 })
    expect(option.series[1].data[0].value).toEqual(['2026-01-02', 101, 700, 70, 101])
    expect(option.series[1].data).toHaveLength(1)
    expect(option.series[4]).toMatchObject({ id: 'signal-lane-first', type: 'scatter', symbol: 'diamond', symbolSize: 15 })
    expect(option.series[4].data[0].value).toEqual(['2026-01-02', '趋势保护', 700, 70, 101])

    const tooltip = option.tooltip.formatter([
      { seriesType: 'line', seriesName: '复权收盘价', value: ['2026-01-02', 101] },
      { seriesType: 'scatter', seriesName: '趋势保护', value: ['2026-01-02', '趋势保护', 700, 70, 101] },
    ])
    expect(tooltip).toContain('复权收盘价')
    expect(tooltip).toContain('趋势保护')
    expect(tooltip).toContain('本期预算 70%')
    expect(option.tooltip.formatter([])).toBe('')
    expect(option.tooltip.formatter({ seriesType: 'scatter', value: 'invalid' })).toContain('策略')
    expect(option.xAxis[1].axisLabel.formatter('2026-01-02')).toBe('2026-01')
    expect(option.xAxis[1].axisLabel.formatter('not-a-date')).toBe('not-a-date')
    expect(option.yAxis[0].axisLabel?.formatter?.(999)).toBe('999')
    expect(option.yAxis[0].axisLabel?.formatter?.(12_000)).toContain('万')
    expect(option.series[1].emphasis).toMatchObject({ scale: 1.5, focus: 'self' })
    expect(option.series[4].emphasis).toMatchObject({ scale: 1.45, focus: 'self' })
  })

  it('enables the same wheel zoom on normalized curves and preserves overlap styling', () => {
    const first = backtestSeries('first', [])
    const second = backtestSeries('second', [])
    const catalog = new Map([
      ['first', { name: '策略一' } as StrategyCatalogEntry],
      ['second', { name: '策略二' } as StrategyCatalogEntry],
    ])
    const fallback = { ...backtestSeries('fallback', []), strategy_name: undefined } as unknown as DynamicBacktestSeries
    const option = buildNormalizedChartOption([first, second, fallback], catalog, new Set(['second'])) as unknown as NormalizedOption

    expect(option.dataZoom[0]).toMatchObject({ type: 'inside', zoomOnMouseWheel: true })
    expect(option.series[0].lineStyle.type).toBe('solid')
    expect(option.series[1].lineStyle.type).toBe('dashed')
    expect(option.series[0].emphasis?.lineStyle?.color).toBe(option.series[0].lineStyle.color)
    expect(option.tooltip.formatter([{ seriesName: '策略一', value: ['2026-01-02', 108.25] }])).toContain('108.25')
    expect(option.tooltip.formatter([])).toBe('')
    expect(option.tooltip.formatter({ value: undefined })).toContain('策略')
    expect(option.xAxis.axisLabel.formatter('2026-01-02')).toBe('2026-01')
    expect(option.yAxis.axisLabel.formatter(108.25)).toBe('108')
  })

  it('builds auditable drawdown and per-period allocation charts from server fields', () => {
    const first = backtestSeries('first', [executionPoint(700, 70, true)])
    const catalog = new Map([['first', { name: '趋势保护' } as StrategyCatalogEntry]])
    const drawdown = buildDrawdownChartOption([first], catalog) as unknown as ResearchOption
    const allocation = buildAllocationChartOption(first, 'USD') as unknown as ResearchOption

    expect(drawdown.series[0].data).toEqual([
      ['2026-01-02', 0],
      ['2026-01-05', -1.25],
    ])
    expect(drawdown.tooltip.formatter([{ seriesName: '趋势保护', value: ['2026-01-05', -1.25] }])).toContain('-1.25%')
    expect(allocation.series.map((item) => item.stack)).toEqual(['period-budget', 'period-budget', 'period-budget'])
    expect(allocation.series[0].data).toEqual([700])
    expect(allocation.series[1].data).toEqual([0])
    expect(allocation.series[2].data).toEqual([300])
    expect(allocation.tooltip.formatter([{ axisValue: '2026-01-02', seriesName: '核心投入', value: 700 }])).toContain('US$700.00')
  })
})

interface ChartOption {
  grid: Array<{ left: number }>
  xAxis: Array<{ axisLabel: { formatter: (value: number | string) => string } }>
  yAxis: Array<{ data?: string[]; axisLabel?: { formatter?: (value: number) => string } }>
  dataZoom: Array<Record<string, unknown>>
  series: Array<{
    type: string
    symbol?: string
    symbolSize?: number
    lineStyle: { type?: string }
    data: Array<{ value: unknown[] }>
    emphasis?: { scale?: number; focus?: string; lineStyle?: { color?: string } }
  }>
  tooltip: { formatter: (input: unknown) => string }
}

interface NormalizedOption {
  xAxis: { axisLabel: { formatter: (value: number | string) => string } }
  yAxis: { axisLabel: { formatter: (value: number) => string } }
  dataZoom: Array<Record<string, unknown>>
  series: Array<{ lineStyle: { type?: string; color?: string }; emphasis?: { lineStyle?: { color?: string } } }>
  tooltip: { formatter: (input: unknown) => string }
}

interface ResearchOption {
  series: Array<{ data: unknown[]; stack?: string }>
  tooltip: { formatter: (input: unknown) => string }
}

function backtestSeries(strategyId: string, executionPoints: DynamicBacktestSeries['execution_points']): DynamicBacktestSeries {
  return {
    strategy_id: strategyId,
    strategy_version: 1,
    strategy_name: strategyId,
    normalized_points: [
      { date: '2026-01-02', value: 100 },
      { date: '2026-01-05', value: 101 },
    ],
    execution_points: executionPoints,
    drawdown_points: [
      { date: '2026-01-02', value_percent: 0 },
      { date: '2026-01-05', value_percent: -1.25 },
    ],
    metrics: {
      total_return_percent: 0,
      maximum_drawdown_percent: 0,
      total_contributed: 0,
      total_invested: 0,
      cash_utilisation_percent: 0,
      terminal_wealth: 0,
      terminal_cash: 0,
    },
    calculation_details: {
      elapsed_days: 3,
      daily_return_count: 2,
      mean_daily_return_percent: 0.5,
      daily_standard_deviation_percent: 1.2,
      downside_deviation_percent: 0.7,
      drawdown_peak_date: '2026-01-02',
      drawdown_trough_date: '2026-01-05',
      total_transaction_cost: 0.35,
      rule_matched_count: executionPoints.filter((point) => point.strategy_rule_matched).length,
      standard_execution_count: executionPoints.filter((point) => !point.strategy_rule_matched).length,
      trading_periods_per_year: 252,
      calendar_days_per_year: 365.25,
      buy_cost_bps: 5,
    },
  }
}

function executionPoint(investedAmount: number, budgetPercent: number, strategyRuleMatched = true) {
  return {
    date: '2026-01-02',
    adjusted_close: 101,
    invested_amount: investedAmount,
    budget_utilisation_percent: budgetPercent,
    scheduled_contribution_amount: 1000,
    core_invested_amount: investedAmount >= 700 ? 700 : investedAmount,
    opportunity_invested_amount: Math.max(investedAmount - 700, 0),
    unallocated_amount: Math.max(1000 - investedAmount, 0),
    transaction_cost: investedAmount * 0.0005,
    strategy_rule_matched: strategyRuleMatched,
  }
}
