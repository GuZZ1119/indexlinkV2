import type { EChartsCoreOption } from 'echarts/core'

import type { DynamicBacktestSeries, StrategyCatalogEntry } from '@/api/types'
import { strategyAnalysisColor } from '@/features/v2_1/model'

interface TooltipParam {
  axisValue?: string
  seriesName?: string
  value?: unknown
}

export function buildDrawdownChartOption(series: DynamicBacktestSeries[], catalogById: Map<string, StrategyCatalogEntry>): EChartsCoreOption {
  return {
    animationDuration: 260,
    aria: { enabled: true, decal: { show: false }, description: '各策略相对自身历史高点的每日回撤曲线，零表示处于新高。' },
    grid: { left: 72, right: 28, top: 24, bottom: 68 },
    tooltip: {
      trigger: 'axis',
      confine: true,
      renderMode: 'richText',
      axisPointer: { type: 'line', lineStyle: { color: '#9dbbad', type: 'dashed' } },
      formatter: (input: unknown) => formatDrawdownTooltip(input),
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
      max: 0,
      name: '回撤',
      nameTextStyle: { color: '#718096', align: 'left', padding: [0, 0, 8, -42] },
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { color: '#718096', formatter: (value: number) => `${value.toFixed(0)}%` },
      splitLine: { lineStyle: { color: '#e2e9e5', type: 'dashed' } },
    },
    dataZoom: researchDataZoom(),
    series: series.map((item) => {
      const color = strategyAnalysisColor(item.strategy_id)
      return {
        id: `drawdown-${item.strategy_id}`,
        name: strategyName(item, catalogById),
        type: 'line',
        showSymbol: false,
        smooth: 0.12,
        sampling: 'lttb',
        lineStyle: { width: 2.5, color },
        areaStyle: { color, opacity: 0.06 },
        itemStyle: { color },
        emphasis: { focus: 'series', lineStyle: { width: 3.5, color } },
        data: item.drawdown_points.map((point) => [point.date, point.value_percent]),
      }
    }),
  }
}

export function buildAllocationChartOption(series: DynamicBacktestSeries, currency: string): EChartsCoreOption {
  const dates = series.execution_points.map((point) => point.date)
  return {
    animationDuration: 260,
    aria: { enabled: true, decal: { show: false }, description: '每个计划日的核心投入、机会投入和未投入现金堆叠柱状图。' },
    grid: { left: 74, right: 28, top: 24, bottom: 82 },
    tooltip: {
      trigger: 'axis',
      confine: true,
      renderMode: 'richText',
      axisPointer: { type: 'shadow', shadowStyle: { color: 'rgba(45, 106, 87, 0.06)' } },
      formatter: (input: unknown) => formatAllocationTooltip(input, currency),
    },
    xAxis: {
      type: 'category',
      data: dates,
      axisLine: { lineStyle: { color: '#d5e2dc' } },
      axisTick: { show: false },
      axisLabel: { color: '#718096', hideOverlap: true, formatter: formatAxisDate },
    },
    yAxis: {
      type: 'value',
      min: 0,
      name: `本期资金（${currency}）`,
      nameTextStyle: { color: '#718096', align: 'left', padding: [0, 0, 8, -54] },
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { color: '#718096', formatter: formatCompactNumber },
      splitLine: { lineStyle: { color: '#e2e9e5', type: 'dashed' } },
    },
    dataZoom: researchDataZoom(),
    series: [
      allocationBar('核心投入', '#55768a', series.execution_points.map((point) => point.core_invested_amount)),
      allocationBar('机会投入', '#2d6a57', series.execution_points.map((point) => point.opportunity_invested_amount)),
      allocationBar('本期未投入', '#b58a4a', series.execution_points.map((point) => point.unallocated_amount)),
    ],
  }
}

function allocationBar(name: string, color: string, data: number[]) {
  return {
    name,
    type: 'bar' as const,
    stack: 'period-budget',
    barMaxWidth: 24,
    itemStyle: { color, borderRadius: name === '本期未投入' ? [4, 4, 0, 0] : 0 },
    emphasis: { focus: 'series' as const },
    data,
  }
}

function researchDataZoom() {
  return [
    { type: 'inside' as const, filterMode: 'none' as const, zoomOnMouseWheel: true, moveOnMouseWheel: false, moveOnMouseMove: true, preventDefaultMouseMove: true, start: 0, end: 100 },
    { type: 'slider' as const, filterMode: 'none' as const, height: 20, bottom: 16, borderColor: '#d5e2dc', backgroundColor: '#f3f7f5', fillerColor: 'rgba(45, 106, 87, 0.13)', handleStyle: { color: '#2d6a57', borderColor: '#2d6a57' }, showDetail: false, brushSelect: false, start: 0, end: 100 },
  ]
}

function formatDrawdownTooltip(input: unknown): string {
  const params = normalizeTooltipParams(input)
  if (params.length === 0) return ''
  const date = Array.isArray(params[0].value) ? params[0].value[0] : params[0].axisValue
  const rows = params.map((param) => `${param.seriesName ?? '策略'}  ${formatPercent(Array.isArray(param.value) ? param.value[1] : param.value)}`)
  return [formatFullDate(date), ...rows].join('\n')
}

function formatAllocationTooltip(input: unknown, currency: string): string {
  const params = normalizeTooltipParams(input)
  if (params.length === 0) return ''
  const rows = params.map((param) => `${param.seriesName ?? '资金'}  ${formatCurrency(currency, param.value)}`)
  return [formatFullDate(params[0].axisValue), ...rows].join('\n')
}

function normalizeTooltipParams(input: unknown): TooltipParam[] {
  if (Array.isArray(input)) return input.filter((item): item is TooltipParam => Boolean(item && typeof item === 'object'))
  return input && typeof input === 'object' ? [input as TooltipParam] : []
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

function formatPercent(value: unknown): string {
  const number = Number(value)
  return Number.isFinite(number) ? `${number.toFixed(2)}%` : '—'
}

function formatCurrency(currency: string, value: unknown): string {
  const amount = Number(value)
  if (!Number.isFinite(amount)) return '—'
  return new Intl.NumberFormat('zh-CN', { style: 'currency', currency, maximumFractionDigits: 2 }).format(amount)
}

function formatCompactNumber(value: number): string {
  return new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 2, notation: Math.abs(value) >= 10_000 ? 'compact' : 'standard' }).format(value)
}
