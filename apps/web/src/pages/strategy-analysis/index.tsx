import { BarChart3, Check, CircleAlert, Database, RefreshCw, ShieldCheck } from 'lucide-react'
import { useEffect, useMemo } from 'react'
import { CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import { useSnapshot } from 'valtio'
import { useSearchParams } from 'react-router'

import { ApiRequestError, useStrategyBacktest, useStrategyCatalog } from '@/api/queries'
import type { DynamicBacktestSeries, StrategyBacktestRequest, StrategyCatalogEntry } from '@/api/types'
import { PageHeading } from '@/components/v2_1/page-heading'
import { StrategyCenterNav } from '@/components/v2_1/strategy-center-nav'
import { strategyAnalysisColor, strategyAnalysisRanges, type StrategyId } from '@/features/v2_1/model'
import { openStrategyAnalysis, strategyAnalysisStore, submitStrategyAnalysis, toggleAnalysisStrategy } from '@/stores/ui'

export default function StrategyAnalysisPage() {
  const state = useSnapshot(strategyAnalysisStore)
  const [searchParams] = useSearchParams()
  const catalog = useStrategyCatalog()
  const requestedStrategy = parsePolicyId(searchParams.get('strategy'))
  const requestedView = searchParams.get('view') === 'research' ? 'research' : 'plain'
  const catalogEntries = useMemo(() => Array.isArray(catalog.data) ? catalog.data : [], [catalog.data])
  const catalogById = useMemo(() => new Map(catalogEntries.map((strategy) => [strategy.policy.id, strategy])), [catalogEntries])
  const requestedCatalogStrategy = requestedStrategy ? catalogById.get(requestedStrategy) : undefined
  const invalidRequestedStrategy = Boolean(requestedStrategy && catalog.isSuccess && Array.isArray(catalog.data) && !requestedCatalogStrategy)

  useEffect(() => {
    strategyAnalysisStore.view = requestedView
    if (requestedCatalogStrategy && state.appliedRouteStrategyId !== requestedCatalogStrategy.policy.id) {
      strategyAnalysisStore.appliedRouteStrategyId = requestedCatalogStrategy.policy.id
      openStrategyAnalysis(requestedCatalogStrategy.policy.id)
    }
  }, [requestedCatalogStrategy, requestedView, state.appliedRouteStrategyId])

  const request = useMemo<StrategyBacktestRequest>(() => ({
    symbol: state.submitted.symbol,
    strategy_ids: [...state.submitted.strategy_ids],
    range: state.submitted.range,
    monthly_day: state.submitted.monthly_day,
    contribution: state.submitted.contribution,
  }), [state.submitted])
  const effectiveRequest = useMemo<StrategyBacktestRequest | null>(() => {
    if (!requestedStrategy) return request
    if (!requestedCatalogStrategy) return null
    return state.appliedRouteStrategyId === requestedCatalogStrategy.policy.id ? request : null
  }, [request, requestedCatalogStrategy, requestedStrategy, state.appliedRouteStrategyId])
  const backtest = useStrategyBacktest(effectiveRequest)
  const result = backtest.data
  const chartData = useMemo(() => mergeSeries(result?.result.series ?? []), [result])

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <PageHeading eyebrow="策略中心 / 策略分析" title="同一只标的，同一段时间，再比较策略" description="输入任意支持的美股、A 股或港股代码。所有策略使用同一份真实日线、同样的投入节奏和共同起点，结果不会混入账户余额或演示数据。" />
      <StrategyCenterNav />

      <form className="rounded-[1.45rem] border border-[#cddfd6] bg-[#f7fbf9] p-5 shadow-[0_18px_50px_rgb(15_32_40_/_0.06)] sm:p-6" onSubmit={(event) => { event.preventDefault(); submitStrategyAnalysis() }}>
        <div className="grid gap-5 lg:grid-cols-[minmax(0,1.4fr)_minmax(12rem,.55fr)_auto] lg:items-end">
          <label className="block"><span className="text-sm font-semibold text-[#102028]">回测哪只标的</span><span className="mt-1 block text-xs leading-5 text-slate-500">美股如 US.SPY，港股如 HK.00700，A 股如 SH.510300 / SZ.159915</span><input aria-label="回测标的" value={state.symbol} onChange={(event) => { strategyAnalysisStore.symbol = event.target.value }} className="mt-2 h-12 w-full rounded-xl border border-[#bfd3c9] bg-white px-4 text-base font-semibold uppercase text-[#102028] outline-none transition focus:border-[#2d6a57] focus:ring-4 focus:ring-[#2d6a57]/10" /></label>
          <label className="block"><span className="text-sm font-semibold text-[#102028]">每月投入</span><span className="mt-1 block text-xs leading-5 text-slate-500">按标的交易币种计算</span><input aria-label="每月投入金额" inputMode="decimal" value={state.contribution} onChange={(event) => { strategyAnalysisStore.contribution = event.target.value }} className="mt-2 h-12 w-full rounded-xl border border-[#bfd3c9] bg-white px-4 font-semibold text-[#102028] outline-none transition focus:border-[#2d6a57] focus:ring-4 focus:ring-[#2d6a57]/10" /></label>
          <button type="submit" disabled={!state.symbol.trim() || backtest.isFetching} className="inline-flex h-12 items-center justify-center gap-2 rounded-xl bg-[#102028] px-5 text-sm font-semibold text-white transition hover:bg-[#1a343f] disabled:cursor-not-allowed disabled:opacity-55"><RefreshCw className={`size-4 ${backtest.isFetching ? 'animate-spin' : ''}`} />{backtest.isFetching ? '正在计算' : '运行真实回测'}</button>
        </div>
        <div className="mt-5 flex flex-wrap gap-2" aria-label="选择时间范围">{strategyAnalysisRanges.map((item) => <button key={item.id} type="button" aria-pressed={state.range === item.id} onClick={() => { strategyAnalysisStore.range = item.id }} className={`rounded-full px-3.5 py-2 text-sm font-medium transition-colors ${state.range === item.id ? 'bg-[#102028] text-white' : 'border border-[#d5e2dc] bg-white text-slate-600 hover:border-[#a9c6b8]'}`}>{item.label}</button>)}</div>
      </form>

      <div className="flex w-fit rounded-full border border-slate-200 bg-white p-1 shadow-sm" aria-label="选择分析视角"><button type="button" aria-pressed={state.view === 'plain'} onClick={() => { strategyAnalysisStore.view = 'plain' }} className={`rounded-full px-3.5 py-2 text-sm font-medium transition-colors ${state.view === 'plain' ? 'bg-[#102028] text-white' : 'text-slate-500 hover:bg-slate-100 hover:text-[#102028]'}`}>直观视角</button><button type="button" aria-pressed={state.view === 'research'} onClick={() => { strategyAnalysisStore.view = 'research' }} className={`rounded-full px-3.5 py-2 text-sm font-medium transition-colors ${state.view === 'research' ? 'bg-[#102028] text-white' : 'text-slate-500 hover:bg-slate-100 hover:text-[#102028]'}`}>专业研究</button></div>

      {invalidRequestedStrategy ? <UnknownStrategyState policyId={requestedStrategy!} /> : catalog.isLoading && requestedStrategy ? <LoadingState label="正在确认策略目录…" /> : catalog.isError && requestedStrategy ? <CatalogErrorState /> : backtest.isLoading ? <LoadingState /> : backtest.isError ? <ErrorState error={backtest.error} /> : result ? <>
        <ProvenanceStrip response={result} />
        {state.view === 'plain'
          ? <PlainView series={result.result.series} chartData={chartData} currency={result.data.currency} selected={state.strategyIds as readonly StrategyId[]} catalog={catalogEntries} />
          : <ResearchView series={result.result.series} currency={result.data.currency} effectiveStart={result.result.effective_start} effectiveEnd={result.result.effective_end} contributionCount={result.result.contribution_count} catalogById={catalogById} />}
      </> : null}
    </div>
  )
}

function PlainView({ series, chartData, currency, selected, catalog }: { series: DynamicBacktestSeries[]; chartData: Array<Record<string, string | number>>; currency: string; selected: readonly StrategyId[]; catalog: StrategyCatalogEntry[] }) {
  const catalogById = new Map(catalog.map((strategy) => [strategy.policy.id, strategy]))
  const available = catalog.filter((strategy) => !selected.includes(strategy.policy.id))
  const selectionFull = selected.length >= 3
  return <section className="rounded-[1.45rem] border border-slate-200 bg-white p-5 shadow-sm sm:p-6">
    <div className="grid gap-6 lg:grid-cols-[15rem_minmax(0,1fr)]">
      <aside><p className="text-sm font-semibold text-[#102028]">选择要对比的策略</p><p className="mt-1 text-xs leading-5 text-slate-500">最多选择三条。改变选择后，再运行真实回测。</p><label className="mt-4 block"><span className="sr-only">添加对比策略</span><select aria-label="添加对比策略" value="" disabled={selectionFull || available.length === 0} onChange={(event) => { if (event.target.value) toggleAnalysisStrategy(event.target.value) }} className="h-11 w-full rounded-xl border border-[#bfd3c9] bg-white px-3 text-sm font-medium text-[#102028] outline-none transition focus:border-[#2d6a57] focus:ring-4 focus:ring-[#2d6a57]/10 disabled:cursor-not-allowed disabled:bg-slate-50 disabled:text-slate-400"><option value="">{selectionFull ? '已选满 3 条策略' : '添加一个策略…'}</option>{available.map((strategy) => <option key={strategy.policy.id} value={strategy.policy.id}>{strategy.name}</option>)}</select></label><div className="mt-3 space-y-2">{selected.map((policyId) => { const strategy = catalogById.get(policyId); const name = strategy?.name ?? policyId; const canRemove = selected.length > 1; return <button key={policyId} type="button" aria-label={canRemove ? `移除${name}` : `${name}（至少保留一个）`} aria-pressed="true" disabled={!canRemove} onClick={() => toggleAnalysisStrategy(policyId)} className="flex w-full items-center gap-3 rounded-xl border border-[#a9cbbb] bg-[#f1f7f4] p-3 text-left shadow-[0_8px_24px_rgb(45_106_87_/_0.08)] transition hover:border-[#78a991] disabled:cursor-default"><span className="grid size-5 shrink-0 place-items-center rounded-full" style={{ backgroundColor: strategyAnalysisColor(policyId) }}><Check className="size-3.5 text-white" /></span><span className="min-w-0"><span className="block truncate text-sm font-medium text-[#102028]">{name}</span><span className="mt-0.5 block text-xs text-slate-500">{scheduleLabel(strategy)}</span></span></button> })}</div></aside>
      <div className="min-w-0"><div className="flex flex-wrap items-end justify-between gap-3"><div><p className="inline-flex items-center gap-2 text-sm font-medium text-[#2d6a57]"><BarChart3 className="size-4" />真实归一化走势</p><h2 className="mt-2 text-2xl font-semibold tracking-[-0.035em] text-[#102028]">共同起点 = 100</h2></div><p className="max-w-md text-xs leading-5 text-slate-500">这里只比较策略造成的路径差异；金额、币种和投入次数不会让某条线凭空领先。</p></div><div className="mt-5 h-[23rem] rounded-[1.2rem] bg-[#f8faf9] p-3 sm:p-5"><ResponsiveContainer width="100%" height="100%" minWidth={0} minHeight={1}><LineChart data={chartData} margin={{ top: 8, right: 12, left: -12, bottom: 0 }}><CartesianGrid vertical={false} stroke="#dce7e1" strokeDasharray="3 3" /><XAxis dataKey="date" tickLine={false} axisLine={false} minTickGap={38} tick={{ fill: '#718096', fontSize: 12 }} /><YAxis domain={['auto', 'auto']} tickLine={false} axisLine={false} width={46} tickFormatter={(value) => Number(value).toFixed(0)} tick={{ fill: '#718096', fontSize: 12 }} /><Tooltip cursor={{ stroke: '#b8d5c6', strokeDasharray: '3 3' }} contentStyle={{ borderRadius: 14, borderColor: '#d8e5df', boxShadow: '0 10px 30px rgb(15 32 40 / 0.08)' }} formatter={(value, name) => [`${Number(value).toFixed(2)} 指数`, `${name}`]} />{series.map((item) => <Line key={item.strategy_id} type="monotone" dataKey={item.strategy_id} name={catalogById.get(item.strategy_id)?.name ?? item.strategy_name ?? item.strategy_id} stroke={strategyAnalysisColor(item.strategy_id)} strokeWidth={3} dot={false} activeDot={{ r: 4 }} />)}</LineChart></ResponsiveContainer></div></div>
    </div>
    <div className="mt-6 grid gap-3 border-t border-slate-100 pt-6 sm:grid-cols-2 xl:grid-cols-3">{series.map((item) => <article key={item.strategy_id} className="rounded-xl border border-slate-200 bg-[#fbfcfb] p-4"><div className="flex items-center justify-between gap-3"><p className="font-medium text-[#102028]">{catalogById.get(item.strategy_id)?.name ?? item.strategy_name ?? item.strategy_id}</p><span className="text-xs text-slate-500">{currency}</span></div><p className={`mt-4 text-2xl font-semibold tracking-[-0.035em] ${item.metrics.total_return_percent >= 0 ? 'text-[#2d6a57]' : 'text-[#8a5f26]'}`}>{formatPercent(item.metrics.total_return_percent)}</p><p className="mt-1 text-xs text-slate-500">区间变化 · 最大回撤 {formatPercent(-Math.abs(item.metrics.maximum_drawdown_percent))}</p></article>)}</div>
  </section>
}

function ResearchView({ series, currency, effectiveStart, effectiveEnd, contributionCount, catalogById }: { series: DynamicBacktestSeries[]; currency: string; effectiveStart: string; effectiveEnd: string; contributionCount: number; catalogById: Map<string, StrategyCatalogEntry> }) {
  return <section className="rounded-[1.45rem] border border-slate-200 bg-white p-5 shadow-sm sm:p-6" aria-label="真实专业回测指标"><div className="flex flex-col gap-3 border-b border-slate-100 pb-5 md:flex-row md:items-end md:justify-between"><div><p className="inline-flex items-center gap-2 text-sm font-medium text-[#2d6a57]"><ShieldCheck className="size-4" />专业研究</p><h2 className="mt-2 text-2xl font-semibold tracking-[-0.035em] text-[#102028]">同一条曲线的风险与资金指标</h2><p className="mt-2 text-sm text-slate-600">{effectiveStart} 至 {effectiveEnd} · 每条策略均投入 {contributionCount} 次</p></div><span className="rounded-full bg-[#e9f2ee] px-3 py-1.5 text-xs font-semibold text-[#2d6a57]">真实 API · {currency}</span></div><div className="mt-5 overflow-x-auto"><table className="w-full min-w-[760px] border-separate border-spacing-0 text-left text-sm"><thead><tr>{['策略', '区间收益', '年化收益', 'XIRR', '最大回撤', '年化波动', 'Sortino', '现金使用率'].map((label) => <th key={label} className="border-b border-slate-200 px-3 py-3 font-medium text-slate-500">{label}</th>)}</tr></thead><tbody>{series.map((item) => <tr key={item.strategy_id} className="text-[#102028]"><td className="border-b border-slate-100 px-3 py-4 font-semibold">{catalogById.get(item.strategy_id)?.name ?? item.strategy_name ?? item.strategy_id}<span className="mt-1 block text-xs font-normal text-slate-400">v{item.strategy_version}</span></td><Metric value={formatPercent(item.metrics.total_return_percent)} /><Metric value={formatOptionalPercent(item.metrics.annualized_return_percent)} /><Metric value={formatOptionalPercent(item.metrics.xirr_percent)} /><Metric value={formatPercent(-Math.abs(item.metrics.maximum_drawdown_percent))} /><Metric value={formatOptionalPercent(item.metrics.annualized_volatility_percent)} /><Metric value={item.metrics.sortino_ratio?.toFixed(2) ?? '样本不足'} /><Metric value={formatPercent(item.metrics.cash_utilisation_percent)} /></tr>)}</tbody></table></div><div className="mt-5 grid gap-3 md:grid-cols-3"><Fact title="成本口径" text="每次模拟买入计 5 bps 成本；暂不包含税、滑点和最小交易单位。" /><Fact title="因果边界" text="Formula 只读取模拟成交日前已经收盘的数据，不读取当天成交价作决定。" /><Fact title="不是预测" text="这些数字只描述所选历史区间；更换标的、范围或数据修订都会改变结果。" /></div></section>
}

function ProvenanceStrip({ response }: { response: NonNullable<ReturnType<typeof useStrategyBacktest>['data']> }) {
  const data = response.data
  return <section className="grid gap-3 rounded-[1.2rem] border border-[#c8ddd3] bg-[#f1f7f4] p-4 md:grid-cols-[1fr_auto_auto] md:items-center"><div className="flex items-start gap-3"><span className="grid size-9 shrink-0 place-items-center rounded-full bg-white text-[#2d6a57]"><Database className="size-4" /></span><div><p className="font-semibold text-[#102028]">{response.result.symbol} · 真实日线回测</p><p className="mt-1 text-xs leading-5 text-slate-600">{data.provider} / {data.dataset_version} · {adjustmentLabel(data.adjustment)} · 数据校验 {data.checksum.slice(0, 10)}…</p></div></div><div className="text-xs text-slate-500"><span className="block">共同有效范围</span><strong className="mt-1 block text-sm text-[#102028]">{response.result.effective_start} → {response.result.effective_end}</strong></div><div className="text-xs text-slate-500"><span className="block">交易币种 / 时区</span><strong className="mt-1 block text-sm text-[#102028]">{data.currency} · {data.timezone}</strong></div></section>
}

function LoadingState({ label = '正在读取真实日线并统一计算策略…' }: { label?: string }) { return <section className="rounded-[1.45rem] border border-slate-200 bg-white p-8" role="status"><div className="h-4 w-40 animate-pulse rounded bg-slate-200" /><div className="mt-5 h-72 animate-pulse rounded-2xl bg-slate-100" /><p className="mt-4 text-sm text-slate-500">{label}</p></section> }

function UnknownStrategyState({ policyId }: { policyId: string }) { return <section className="rounded-[1.35rem] border border-[#dec9a6] bg-[#fff8eb] p-5" role="alert"><div className="flex items-start gap-3"><CircleAlert className="mt-0.5 size-5 shrink-0 text-[#8a5f26]" /><div><h2 className="font-semibold text-[#4d391d]">策略目录中没有这个策略</h2><p className="mt-2 text-sm leading-6 text-[#73572f]">“{policyId}”可能已下线或链接有误。页面不会把它替换成固定定投，请返回策略中心重新选择。</p></div></div></section> }

function CatalogErrorState() { return <section className="rounded-[1.35rem] border border-[#dec9a6] bg-[#fff8eb] p-5" role="alert"><div className="flex items-start gap-3"><CircleAlert className="mt-0.5 size-5 shrink-0 text-[#8a5f26]" /><div><h2 className="font-semibold text-[#4d391d]">暂时无法确认这个策略</h2><p className="mt-2 text-sm leading-6 text-[#73572f]">策略目录读取失败，因此不会用其他策略代替运行。请确认本机服务后重试。</p></div></div></section> }

function ErrorState({ error }: { error: Error }) {
  const unavailable = error instanceof ApiRequestError && error.status === 503
  return <section className="rounded-[1.35rem] border border-[#dec9a6] bg-[#fff8eb] p-5" role="alert"><div className="flex items-start gap-3"><CircleAlert className="mt-0.5 size-5 shrink-0 text-[#8a5f26]" /><div><h2 className="font-semibold text-[#4d391d]">{unavailable ? '真实行情暂不可用' : '这次回测无法完成'}</h2><p className="mt-2 text-sm leading-6 text-[#73572f]">{unavailable ? '请确认本机 OpenD 已启动，并在后端启用只读市场数据。页面不会用静态曲线替代真实响应。' : '请检查市场前缀、标的代码、金额和时间范围；新上市标的也可能没有足够的指标预热历史。'}</p></div></div></section>
}

function Metric({ value }: { value: string }) { return <td className="border-b border-slate-100 px-3 py-4 tabular-nums">{value}</td> }
function Fact({ title, text }: { title: string; text: string }) { return <div className="rounded-xl bg-[#f6f8f7] p-4"><p className="text-sm font-semibold text-[#102028]">{title}</p><p className="mt-2 text-xs leading-5 text-slate-600">{text}</p></div> }

function mergeSeries(series: DynamicBacktestSeries[]): Array<Record<string, string | number>> {
  const dates = new Map<string, Record<string, string | number>>()
  for (const item of series) for (const point of item.normalized_points) {
    const row = dates.get(point.date) ?? { date: point.date }
    row[item.strategy_id] = point.value
    dates.set(point.date, row)
  }
  return [...dates.values()].sort((left, right) => String(left.date).localeCompare(String(right.date)))
}

function parsePolicyId(value: string | null): StrategyId | null { return value?.trim() || null }
function scheduleLabel(strategy?: StrategyCatalogEntry): string { return strategy?.default_plan.schedule_kind === 'weekly' ? '每周检查' : '每月检查' }
function formatPercent(value: number): string { return `${value >= 0 ? '+' : ''}${value.toFixed(1)}%` }
function formatOptionalPercent(value?: number): string { return value === undefined || value === null ? '样本不足' : formatPercent(value) }
function adjustmentLabel(value: string): string { return ({ all: '全量复权', forward: '前复权', raw: '不复权' } as Record<string, string>)[value] ?? value }
