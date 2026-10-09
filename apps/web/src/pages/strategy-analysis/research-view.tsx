import { uiLocale, uiText, useUiLocale } from '@/i18n/ui'
import { Calculator, CircleDollarSign, LineChart as LineChartIcon, WalletCards } from 'lucide-react'
import { useMemo } from 'react'
import { useSnapshot } from 'valtio'

import type { DynamicBacktestSeries, StrategyCatalogEntry } from '@/api/types'
import { strategyAnalysisColor } from '@/features/v2_1/model'
import { InteractiveChart } from '@/pages/strategy-analysis/interactive-chart'
import { buildAllocationChartOption, buildDrawdownChartOption } from '@/pages/strategy-analysis/research-chart-options'
import { strategyAnalysisStore, type ResearchMetricKey } from '@/stores/ui'

interface ResearchViewProps {
  series: DynamicBacktestSeries[]
  currency: string
  effectiveStart: string
  effectiveEnd: string
  contributionCount: number
  catalogById: Map<string, StrategyCatalogEntry>
}

interface MetricDefinition {
  key: ResearchMetricKey
  label: string
  formula: string
  explanation: string
}

const metricDefinitions: MetricDefinition[] = [
  { key: 'total_return_percent', get label() { return uiText("区间收益") }, formula: 'R = NAVₑₙ𝒹 ÷ NAVₛₜₐᵣₜ − 1', get explanation() { return uiText("使用剔除外部现金流影响后的时间加权净值，衡量整个所选区间的策略变化。") } },
  { key: 'annualized_return_percent', get label() { return uiText("年化收益") }, get formula() { return uiText("Rannual = (NAVₑₙ𝒹 ÷ NAVₛₜₐᵣₜ)^(365.25 ÷ 实际天数) − 1") }, get explanation() { return uiText("把区间时间加权收益换算为一年口径；短样本的年化结果可能被放大。") } },
  { key: 'xirr_percent', label: 'XIRR', formula: 'Σ CFᵢ ÷ (1 + r)^(Δdaysᵢ ÷ 365.25) = 0', get explanation() { return uiText("把每次外部投入视为负现金流、期末资产视为正现金流，求解投资者实际资金经历的年化收益率。") } },
  { key: 'maximum_drawdown_percent', get label() { return uiText("最大回撤") }, get formula() { return uiText("MDD = max((历史峰值 − 当日净值) ÷ 历史峰值)") }, get explanation() { return uiText("衡量从历史高点跌到随后低点的最大幅度；下方回撤图展示完整发生过程。") } },
  { key: 'annualized_volatility_percent', get label() { return uiText("年化波动") }, get formula() { return uiText("σannual = sample_std(日收益率) × √252") }, get explanation() { return uiText("以日时间加权收益的样本标准差衡量路径波动，并按 252 个交易日年化。") } },
  { key: 'sortino_ratio', label: 'Sortino', get formula() { return uiText("Sortino = 平均日收益 ÷ 下行偏差 × √252") }, get explanation() { return uiText("只把低于 0 的日收益计入下行偏差；当前最低可接受收益设为 0，未引入无风险利率。") } },
  { key: 'cash_utilisation_percent', get label() { return uiText("现金使用率") }, get formula() { return uiText("现金使用率 = 累计模拟买入金额 ÷ 累计外部投入") }, get explanation() { return uiText("说明计划资金有多少转成了资产；留在现金中的部分仍计入期末资产与净值。") } },
]

export function ResearchView({ series, currency, effectiveStart, effectiveEnd, contributionCount, catalogById }: ResearchViewProps) {
  const locale = useUiLocale()
  const state = useSnapshot(strategyAnalysisStore)
  const selectedMetric = metricDefinitions.find((metric) => metric.key === state.researchMetric) ?? metricDefinitions[0]
  const selectedSeries = series.find((item) => item.strategy_id === state.researchStrategyId) ?? series[0]
  const drawdownOption = useMemo(() => { void locale; return buildDrawdownChartOption(series, catalogById) }, [catalogById, series, locale])
  const allocationOption = useMemo(() => { void locale; return selectedSeries ? buildAllocationChartOption(selectedSeries, currency) : {} }, [currency, selectedSeries, locale])

  if (!selectedSeries) return null

  return <div className="space-y-6">
    <section className="rounded-[1.45rem] border border-slate-200 bg-white p-5 shadow-sm sm:p-6" aria-label={uiText("真实专业回测指标")}>
      <div className="flex flex-col gap-3 border-b border-slate-100 pb-5 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="inline-flex items-center gap-2 text-sm font-medium text-[#2d6a57]"><Calculator className="size-4" />{uiText("计算口径")}</p>
          <h2 className="mt-2 text-2xl font-semibold tracking-[-0.035em] text-[#102028]">{uiText("结果、公式和本次代入值")}</h2>
          <p className="mt-2 text-sm text-slate-600">{effectiveStart} {uiText("至")}{' '}{effectiveEnd} {uiText("· 每条策略均评估")}{' '}{contributionCount} {uiText("个计划日")}</p>
        </div>
        <span className="rounded-full bg-[#e9f2ee] px-3 py-1.5 text-xs font-semibold text-[#2d6a57]">{uiText("真实 API ·")}{' '}{currency}</span>
      </div>

      <div className="mt-5 overflow-x-auto">
        <table className="w-full min-w-[820px] border-separate border-spacing-0 text-left text-sm">
          <thead><tr><th className="border-b border-slate-200 px-3 py-3 font-medium text-slate-500">{uiText("策略")}</th>{metricDefinitions.map((metric) => <th key={metric.key} className="border-b border-slate-200 px-3 py-3 font-medium text-slate-500"><button type="button" aria-pressed={selectedMetric.key === metric.key} onClick={() => { strategyAnalysisStore.researchMetric = metric.key }} className={`rounded-md px-1 py-0.5 text-left underline decoration-dotted underline-offset-4 transition-colors ${selectedMetric.key === metric.key ? 'bg-[#e9f2ee] text-[#2d6a57] decoration-transparent' : 'hover:text-[#102028]'}`}>{metric.label}</button></th>)}</tr></thead>
          <tbody>{series.map((item) => <tr key={item.strategy_id} className="text-[#102028]"><td className="border-b border-slate-100 px-3 py-4 font-semibold">{strategyName(item, catalogById)}<span className="mt-1 block text-xs font-normal text-slate-400">v{item.strategy_version}</span></td>{metricDefinitions.map((metric) => <MetricCell key={metric.key} item={item} metric={metric} selected={selectedMetric.key === metric.key} />)}</tr>)}</tbody>
        </table>
      </div>

      <div className="mt-5 rounded-[1.1rem] border border-[#cfe0d8] bg-[#f3f7f5] p-5" aria-live="polite" aria-label={uiText("{{p0}}计算公式", { p0: selectedMetric.label })}>
        <p className="text-sm font-semibold text-[#2d6a57]">{selectedMetric.label}{uiText("怎么计算")}</p>
        <p className="mt-3 overflow-x-auto whitespace-nowrap font-mono text-[0.92rem] font-semibold text-[#102028]">{selectedMetric.formula}</p>
        <p className="mt-3 max-w-4xl text-sm leading-6 text-slate-600">{selectedMetric.explanation}</p>
        <div className="mt-4 grid gap-3 lg:grid-cols-3">{series.map((item) => <div key={item.strategy_id} className="rounded-xl border border-white/80 bg-white/80 p-4"><p className="text-sm font-semibold text-[#102028]">{strategyName(item, catalogById)}</p><p className="mt-2 text-xs leading-5 text-slate-600">{metricEvidence(selectedMetric.key, item, currency)}</p></div>)}</div>
      </div>

      <div className="mt-5 grid gap-3 md:grid-cols-3"><Fact title={uiText("成本口径")} text={uiText("每次模拟买入计 5 bps；累计成本按实际模拟成交金额计算，暂不包含税、滑点和最小交易单位。")} /><Fact title={uiText("因果边界")} text={uiText("Formula 只读取模拟成交日前已经收盘的数据，不读取当天成交价作决定。")} /><Fact title={uiText("不是预测")} text={uiText("这些数字只描述所选历史区间；更换标的、范围或数据修订都会改变结果。")} /></div>
    </section>

    <section className="rounded-[1.45rem] border border-slate-200 bg-white p-5 shadow-sm sm:p-6" aria-labelledby="capital-facts-title">
      <div className="flex items-start gap-3"><span className="grid size-10 shrink-0 place-items-center rounded-full bg-[#eef4f1] text-[#2d6a57]"><CircleDollarSign className="size-5" /></span><div><h2 id="capital-facts-title" className="text-xl font-semibold tracking-[-0.025em] text-[#102028]">{uiText("资金与执行事实")}</h2><p className="mt-1 text-sm leading-6 text-slate-600">{uiText("所有金额都来自同一条回测账本，不用收益率反推。")}</p></div></div>
      <div className="mt-5 overflow-x-auto"><table className="w-full min-w-[820px] text-left text-sm"><thead><tr>{[uiText("策略"), uiText("外部投入"), uiText("模拟买入"), uiText("期末资产"), uiText("期末现金"), uiText("累计成本"), uiText("规则命中")].map((label) => <th key={label} className="border-b border-slate-200 px-3 py-3 font-medium text-slate-500">{label}</th>)}</tr></thead><tbody>{series.map((item) => <tr key={item.strategy_id}><td className="border-b border-slate-100 px-3 py-4 font-semibold text-[#102028]">{strategyName(item, catalogById)}</td><MoneyCell value={item.metrics.total_contributed} currency={currency} /><MoneyCell value={item.metrics.total_invested} currency={currency} /><MoneyCell value={item.metrics.terminal_wealth} currency={currency} /><MoneyCell value={item.metrics.terminal_cash} currency={currency} /><MoneyCell value={item.calculation_details.total_transaction_cost} currency={currency} /><td className="border-b border-slate-100 px-3 py-4 tabular-nums text-[#102028]">{item.calculation_details.rule_matched_count} / {item.execution_points.length}<span className="mt-1 block text-xs text-slate-400">{uiText("命中 / 计划日")}</span></td></tr>)}</tbody></table></div>
    </section>

    <section className="rounded-[1.45rem] border border-slate-200 bg-white p-5 shadow-sm sm:p-6" aria-labelledby="drawdown-title">
      <div className="flex flex-col gap-4 border-b border-slate-100 pb-5 md:flex-row md:items-end md:justify-between"><div><p className="inline-flex items-center gap-2 text-sm font-medium text-[#2d6a57]"><LineChartIcon className="size-4" />{uiText("风险路径")}</p><h2 id="drawdown-title" className="mt-2 text-xl font-semibold tracking-[-0.025em] text-[#102028]">{uiText("回撤发生在什么时候")}</h2><p className="mt-2 text-sm leading-6 text-slate-600">{uiText("0% 表示处于历史新高；越向下表示距离此前高点越远。")}</p></div><div className="flex flex-wrap gap-x-4 gap-y-2">{series.map((item) => <span key={item.strategy_id} className="inline-flex items-center gap-2 text-xs text-slate-600"><span className="size-2.5 rounded-full" style={{ backgroundColor: strategyAnalysisColor(item.strategy_id) }} />{strategyName(item, catalogById)}</span>)}</div></div>
      <div className="mt-4 h-[22rem] rounded-[1.2rem] bg-[#f8faf9] p-2 sm:p-4"><InteractiveChart option={drawdownOption} ariaLabel={uiText("策略每日回撤曲线")} /></div>
      <div className="mt-4 grid gap-3 md:grid-cols-3">{series.map((item) => <div key={item.strategy_id} className="rounded-xl bg-[#f6f8f7] p-4"><p className="text-sm font-semibold text-[#102028]">{strategyName(item, catalogById)}</p><p className="mt-2 text-xs leading-5 text-slate-600">{uiText("峰值")}{' '}{formatDate(item.calculation_details.drawdown_peak_date)} {uiText("→ 低点")}{' '}{formatDate(item.calculation_details.drawdown_trough_date)}<br />{uiText("恢复")}{' '}{formatDate(item.calculation_details.drawdown_recovery_date, uiText("尚未恢复"))}</p></div>)}</div>
    </section>

    <section className="rounded-[1.45rem] border border-slate-200 bg-white p-5 shadow-sm sm:p-6" aria-labelledby="allocation-title">
      <div className="flex flex-col gap-4 border-b border-slate-100 pb-5 md:flex-row md:items-end md:justify-between"><div><p className="inline-flex items-center gap-2 text-sm font-medium text-[#2d6a57]"><WalletCards className="size-4" />{uiText("执行拆分")}</p><h2 id="allocation-title" className="mt-2 text-xl font-semibold tracking-[-0.025em] text-[#102028]">{uiText("每期资金去了哪里")}</h2><p className="mt-2 text-sm leading-6 text-slate-600">{uiText("核心投入始终受计划保护；机会投入由 Formula 调整，未使用部分留在现金中。")}</p></div><label className="text-sm font-medium text-[#102028]">{uiText("查看策略")}<select aria-label={uiText("选择资金拆分策略")} value={selectedSeries.strategy_id} onChange={(event) => { strategyAnalysisStore.researchStrategyId = event.target.value }} className="mt-2 block h-10 min-w-64 rounded-lg border border-[#bfd3c9] bg-white px-3 text-sm outline-none focus:border-[#2d6a57] focus:ring-4 focus:ring-[#2d6a57]/10">{series.map((item) => <option key={item.strategy_id} value={item.strategy_id}>{strategyName(item, catalogById)}</option>)}</select></label></div>
      <div className="mt-4 flex flex-wrap gap-4 text-xs text-slate-600"><Legend color="#55768a" label={uiText("核心投入")} /><Legend color="#2d6a57" label={uiText("机会投入")} /><Legend color="#b58a4a" label={uiText("本期未投入")} /></div>
      <div className="mt-3 h-[22rem] rounded-[1.2rem] bg-[#f8faf9] p-2 sm:p-4"><InteractiveChart option={allocationOption} ariaLabel={uiText("{{p0}}每期核心、机会和未投入资金", { p0: strategyName(selectedSeries, catalogById) })} /></div>
    </section>
  </div>
}

function MetricCell({ item, metric, selected }: { item: DynamicBacktestSeries; metric: MetricDefinition; selected: boolean }) {
  useUiLocale()
  return <td className="border-b border-slate-100 px-3 py-4"><button type="button" aria-label={uiText("查看{{p0}}的{{p1}}公式", { p0: item.strategy_name, p1: metric.label })} aria-pressed={selected} onClick={() => { strategyAnalysisStore.researchMetric = metric.key }} className={`rounded-md px-1 py-0.5 tabular-nums transition-colors ${selected ? 'bg-[#eef4f1] font-semibold text-[#2d6a57]' : 'text-[#102028] hover:bg-slate-50'}`}>{metricValue(metric.key, item)}</button></td>
}

function metricValue(key: ResearchMetricKey, item: DynamicBacktestSeries): string {
  const metrics = item.metrics
  if (key === 'sortino_ratio') return metrics.sortino_ratio?.toFixed(2) ?? uiText("样本不足")
  const value = metrics[key]
  if (value === undefined || value === null) return uiText("样本不足")
  if (key === 'maximum_drawdown_percent') return `-${Math.abs(value).toFixed(1)}%`
  const signed = key !== 'cash_utilisation_percent'
  return `${signed && value >= 0 ? '+' : ''}${value.toFixed(1)}%`
}

function metricEvidence(key: ResearchMetricKey, item: DynamicBacktestSeries, currency: string): string {
  const details = item.calculation_details
  const first = item.normalized_points[0]?.value
  const last = item.normalized_points.at(-1)?.value
  if (key === 'total_return_percent') return `${formatNumber(last)} ÷ ${formatNumber(first)} − 1 = ${metricValue(key, item)}`
  if (key === 'annualized_return_percent') return uiText("区间 {{p0}} 天；({{p1}} ÷ {{p2}})^(365.25 ÷ {{p3}}) − 1 = {{p4}}", { p0: details.elapsed_days, p1: formatNumber(last), p2: formatNumber(first), p3: details.elapsed_days, p4: metricValue(key, item) })
  if (key === 'xirr_percent') return uiText("{{p0}} 笔外部投入，期末现金流 {{p1}}；求根结果 {{p2}}", { p0: item.execution_points.length, p1: formatMoney(item.metrics.terminal_wealth, currency), p2: metricValue(key, item) })
  if (key === 'maximum_drawdown_percent') return uiText("{{p0}} 至 {{p1}}：{{p2}}", { p0: formatDate(details.drawdown_peak_date), p1: formatDate(details.drawdown_trough_date), p2: metricValue(key, item) })
  if (key === 'annualized_volatility_percent') return uiText("{{p0}}% × √{{p1}} = {{p2}}；样本 {{p3}} 日", { p0: formatOptionalNumber(details.daily_standard_deviation_percent, 3), p1: details.trading_periods_per_year, p2: metricValue(key, item), p3: details.daily_return_count })
  if (key === 'sortino_ratio') return `${formatOptionalNumber(details.mean_daily_return_percent, 4)}% ÷ ${formatOptionalNumber(details.downside_deviation_percent, 4)}% × √${details.trading_periods_per_year} = ${metricValue(key, item)}`
  return `${formatMoney(item.metrics.total_invested, currency)} ÷ ${formatMoney(item.metrics.total_contributed, currency)} = ${metricValue(key, item)}`
}

function MoneyCell({ value, currency }: { value: number; currency: string }) {
  useUiLocale()
  return <td className="border-b border-slate-100 px-3 py-4 tabular-nums text-[#102028]">{formatMoney(value, currency)}</td> }
function Fact({ title, text }: { title: string; text: string }) {
  useUiLocale()
  return <div className="rounded-xl bg-[#f6f8f7] p-4"><p className="text-sm font-semibold text-[#102028]">{title}</p><p className="mt-2 text-xs leading-5 text-slate-600">{text}</p></div> }
function Legend({ color, label }: { color: string; label: string }) {
  useUiLocale()
  return <span className="inline-flex items-center gap-2"><span className="size-2.5 rounded-[3px]" style={{ backgroundColor: color }} />{label}</span> }
function strategyName(item: DynamicBacktestSeries, catalogById: Map<string, StrategyCatalogEntry>): string { return catalogById.get(item.strategy_id)?.name ?? item.strategy_name ?? item.strategy_id }
function formatMoney(value: number, currency: string): string { return new Intl.NumberFormat(uiLocale(), { style: 'currency', currency, maximumFractionDigits: 2 }).format(value) }
function formatDate(value?: string, fallback = '—'): string { return value ? new Intl.DateTimeFormat(uiLocale(), { timeZone: 'UTC', year: 'numeric', month: 'short', day: 'numeric' }).format(new Date(value)) : fallback }
function formatNumber(value?: number): string { return value === undefined ? '—' : value.toFixed(2) }
function formatOptionalNumber(value: number | undefined, digits: number): string { return value === undefined || value === null ? '—' : value.toFixed(digits) }
