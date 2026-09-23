import type { EChartsCoreOption } from 'echarts/core'

import type { DynamicBacktestSeries, StrategyCatalogEntry } from '@/api/types'
import { strategyAnalysisColor } from '@/features/v2_1/model'
import type { MarketChartPoint } from '@/pages/strategy-analysis/market-execution-model'

interface TooltipParam {
  seriesName?: string
  seriesType?: string
  value?: unknown
}

const markerSymbols = ['diamond', 'rect', 'triangle'] as const

export function buildNormalizedChartOption(series: DynamicBacktestSeries[], catalogById: Map<string, StrategyCatalogEntry>, dashedSeries: ReadonlySet<string>): EChartsCoreOption {
  return {
    animationDuration: 260,
    aria: { enabled: true, decal: { show: false }, description: '策略净值指数折线图。所有策略从一百开始，可用鼠标滚轮缩放时间范围并拖动平移。' },
    grid: { left: 72, right: 28, top: 24, bottom: 68 },
    tooltip: {
      trigger: 'axis',
      confine: true,
      renderMode: 'richText',
      axisPointer: { type: 'line', lineStyle: { color: '#9dbbad', type: 'dashed' } },
      formatter: (input: unknown) => formatNormalizedTooltip(input),
    },
    xAxis: {
      type: 'time',
      boundaryGap: false,
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { color: '#718096', hideOverlap: true, formatter: formatAxisDate },
      splitLine: { show: false },
    },
    yAxis: {
      type: 'value',
      scale: true,
      name: '净值指数',
      nameTextStyle: { color: '#718096', align: 'left', padding: [0, 0, 8, -44] },
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { color: '#718096', formatter: (value: number) => value.toFixed(0) },
      splitLine: { lineStyle: { color: '#dce7e1', type: 'dashed' } },
    },
    dataZoom: sharedDataZoom([0]),
    series: series.map((item) => {
      const color = strategyAnalysisColor(item.strategy_id)
      return {
        id: item.strategy_id,
        name: strategyName(item, catalogById),
        type: 'line',
        showSymbol: false,
        symbol: 'circle',
        smooth: 0.18,
        sampling: 'lttb',
        lineStyle: {
          width: 3,
          color,
          type: dashedSeries.has(item.strategy_id) ? 'dashed' : 'solid',
        },
        itemStyle: { color },
        emphasis: { focus: 'series', lineStyle: { width: 4, color } },
        data: item.normalized_points.map((point) => [point.date, point.value]),
      }
    }),
  }
}

export function buildMarketExecutionChartOption(points: MarketChartPoint[], series: DynamicBacktestSeries[], catalogById: Map<string, StrategyCatalogEntry>, currency: string): EChartsCoreOption {
  const names = series.map((item) => strategyName(item, catalogById))
  return {
    animationDuration: 260,
    aria: { enabled: true, decal: { show: false }, description: '复权收盘价与策略规则触发点。触发点同时显示在价格曲线和策略轨道中，可用鼠标滚轮缩放时间范围并拖动平移。' },
    axisPointer: {
      show: true,
      triggerOn: 'mousemove|click',
      snap: false,
      link: [{ xAxisIndex: [0, 1] }],
      lineStyle: { color: '#9dbbad', type: 'dashed' },
      label: { show: false },
    },
    grid: [
      { left: 132, right: 30, top: 24, height: '53%' },
      { left: 132, right: 30, top: '67%', bottom: 72 },
    ],
    tooltip: {
      trigger: 'item',
      confine: true,
      renderMode: 'richText',
      formatter: (input: unknown) => formatMarketTooltip(input, currency),
    },
    xAxis: [
      {
        type: 'time',
        gridIndex: 0,
        boundaryGap: false,
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: { show: false },
        splitLine: { show: false },
      },
      {
        type: 'time',
        gridIndex: 1,
        boundaryGap: false,
        axisLine: { lineStyle: { color: '#d5e2dc' } },
        axisTick: { show: false },
        axisLabel: { color: '#718096', hideOverlap: true, formatter: formatAxisDate },
        splitLine: { show: false },
      },
    ],
    yAxis: [
      {
        type: 'value',
        gridIndex: 0,
        scale: true,
        name: `复权价（${currency}）`,
        nameTextStyle: { color: '#718096', align: 'left', padding: [0, 0, 8, -50] },
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: { color: '#718096', formatter: (value: number) => formatCompactNumber(value) },
        splitLine: { lineStyle: { color: '#dce7e1', type: 'dashed' } },
      },
      {
        type: 'category',
        gridIndex: 1,
        inverse: true,
        data: names,
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: { color: '#40545d', fontSize: 11, width: 108, overflow: 'truncate' },
        splitLine: { show: true, lineStyle: { color: '#e5ece8' } },
      },
    ],
    dataZoom: sharedDataZoom([0, 1]),
    series: [
      {
        id: 'market-price',
        name: '复权收盘价',
        type: 'line',
        xAxisIndex: 0,
        yAxisIndex: 0,
        showSymbol: false,
        smooth: 0.12,
        sampling: 'lttb',
        lineStyle: { width: 2.5, color: '#55768a' },
        areaStyle: { color: 'rgba(85, 118, 138, 0.08)' },
        itemStyle: { color: '#55768a' },
        emphasis: { focus: 'series', lineStyle: { width: 3.5 } },
        data: points.map((point) => [point.date, point.adjusted_close]),
      },
      ...series.map((item, index) => ({
        id: `price-signal-${item.strategy_id}`,
        name: strategyName(item, catalogById),
        type: 'scatter' as const,
        xAxisIndex: 0,
        yAxisIndex: 0,
        symbol: markerSymbols[index % markerSymbols.length],
        symbolSize: 13,
        z: 6,
        itemStyle: signalItemStyle(item.strategy_id),
        emphasis: { scale: 1.5, focus: 'self' as const },
        data: strategySignalPoints(item).map((execution) => ({
          value: [execution.date, execution.adjusted_close, execution.invested_amount, execution.budget_utilisation_percent, execution.adjusted_close],
        })),
      })),
      ...series.map((item, index) => ({
        id: `signal-lane-${item.strategy_id}`,
        name: strategyName(item, catalogById),
        type: 'scatter' as const,
        xAxisIndex: 1,
        yAxisIndex: 1,
        symbol: markerSymbols[index % markerSymbols.length],
        symbolSize: 15,
        itemStyle: signalItemStyle(item.strategy_id),
        emphasis: { scale: 1.45, focus: 'self' as const },
        data: strategySignalPoints(item).map((execution) => ({
          value: [execution.date, strategyName(item, catalogById), execution.invested_amount, execution.budget_utilisation_percent, execution.adjusted_close],
        })),
      })),
    ],
  }
}

function strategySignalPoints(item: DynamicBacktestSeries) {
  if (item.strategy_id === 'fixed_dca') return item.execution_points
  return item.execution_points.filter((execution) => execution.strategy_rule_matched)
}

function signalItemStyle(strategyId: string) {
  return {
    color: strategyAnalysisColor(strategyId),
    borderColor: '#ffffff',
    borderWidth: 2,
    shadowBlur: 5,
    shadowColor: 'rgba(15, 32, 40, 0.18)',
  }
}

function sharedDataZoom(xAxisIndex: number[]) {
  return [
    {
      type: 'inside' as const,
      xAxisIndex,
      filterMode: 'none' as const,
      zoomOnMouseWheel: true,
      moveOnMouseWheel: false,
      moveOnMouseMove: true,
      preventDefaultMouseMove: true,
      start: 0,
      end: 100,
    },
    {
      type: 'slider' as const,
      xAxisIndex,
      filterMode: 'none' as const,
      height: 20,
      bottom: 16,
      borderColor: '#d5e2dc',
      backgroundColor: '#f3f7f5',
      fillerColor: 'rgba(45, 106, 87, 0.13)',
      handleStyle: { color: '#2d6a57', borderColor: '#2d6a57' },
      moveHandleStyle: { color: '#8bb3a0' },
      dataBackground: { lineStyle: { color: '#9dbbad' }, areaStyle: { color: '#dce9e3' } },
      selectedDataBackground: { lineStyle: { color: '#2d6a57' }, areaStyle: { color: '#b9d4c7' } },
      showDetail: false,
      brushSelect: false,
      start: 0,
      end: 100,
    },
  ]
}

function formatNormalizedTooltip(input: unknown): string {
  const params = normalizeTooltipParams(input)
  if (params.length === 0) return ''
  const date = valueAt(params[0].value, 0)
  const rows = params.map((param) => `${param.seriesName ?? '策略'}  ${formatNumber(valueAt(param.value, 1), 2)}`)
  return [`${formatFullDate(date)}`, ...rows, '起点 = 100'].join('\n')
}

function formatMarketTooltip(input: unknown, currency: string): string {
  const params = normalizeTooltipParams(input)
  if (params.length === 0) return ''
  const date = valueAt(params[0].value, 0)
  const rows = params.flatMap((param) => {
    if (param.seriesType === 'line') return [`复权收盘价  ${formatCurrency(currency, valueAt(param.value, 1))}`]
    const value = Array.isArray(param.value) ? param.value : []
    return [
      `${param.seriesName ?? '策略'}`,
      `模拟投入  ${formatCurrency(currency, value[2])}  ·  本期预算 ${formatNumber(value[3], 0)}%`,
    ]
  })
  return [`${formatFullDate(date)}`, ...rows].join('\n')
}

function normalizeTooltipParams(input: unknown): TooltipParam[] {
  if (Array.isArray(input)) return input.filter((item): item is TooltipParam => Boolean(item && typeof item === 'object'))
  return input && typeof input === 'object' ? [input as TooltipParam] : []
}

function valueAt(value: unknown, index: number): unknown {
  return Array.isArray(value) ? value[index] : undefined
}

function strategyName(item: DynamicBacktestSeries, catalogById: Map<string, StrategyCatalogEntry>): string {
  return catalogById.get(item.strategy_id)?.name ?? item.strategy_name ?? item.strategy_id
}

function formatAxisDate(value: number | string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return String(value)
  return `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, '0')}`
}

function formatFullDate(value: unknown): string {
  if (typeof value !== 'string' && typeof value !== 'number') return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return String(value)
  return new Intl.DateTimeFormat('zh-CN', { timeZone: 'UTC', year: 'numeric', month: 'long', day: 'numeric' }).format(date)
}

function formatCurrency(currency: string, value: unknown): string {
  const amount = Number(value)
  if (!Number.isFinite(amount)) return '—'
  return new Intl.NumberFormat('zh-CN', { style: 'currency', currency, maximumFractionDigits: 2 }).format(amount)
}

function formatNumber(value: unknown, fractionDigits: number): string {
  const number = Number(value)
  return Number.isFinite(number) ? number.toFixed(fractionDigits) : '—'
}

function formatCompactNumber(value: number): string {
  return new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 2, notation: Math.abs(value) >= 10_000 ? 'compact' : 'standard' }).format(value)
}
