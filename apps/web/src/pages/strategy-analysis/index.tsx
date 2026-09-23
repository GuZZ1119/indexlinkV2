import { BarChart3, Bot, Check, CircleAlert, Database, Loader2, RefreshCw, Sparkles } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { useSnapshot } from 'valtio'
import { Link, useSearchParams } from 'react-router'

import { ApiRequestError, describeAiActionError, useAiProviders, useExplainStrategyBacktest, useStrategyBacktest, useStrategyCatalog } from '@/api/queries'
import type { AiProviderProfile, AiReadOnlyExplanation, BacktestMarketPoint, DynamicBacktestSeries, PolicyReference, StrategyBacktestRequest, StrategyCatalogEntry } from '@/api/types'
import { Button } from '@/components/ui/button'
import { PageHeading } from '@/components/v2_1/page-heading'
import { StrategyCenterNav } from '@/components/v2_1/strategy-center-nav'
import { strategyAnalysisColor, strategyAnalysisRanges, type StrategyId } from '@/features/v2_1/model'
import { buildMarketExecutionChartOption, buildNormalizedChartOption } from '@/pages/strategy-analysis/chart-options'
import { InteractiveChart } from '@/pages/strategy-analysis/interactive-chart'
import { buildMarketChartData } from '@/pages/strategy-analysis/market-execution-model'
import { ResearchView } from '@/pages/strategy-analysis/research-view'
import { openStrategyAnalysis, policyKey, strategyAnalysisStore, submitStrategyAnalysis, toggleAnalysisStrategy } from '@/stores/ui'

export default function StrategyAnalysisPage() {
  const state = useSnapshot(strategyAnalysisStore)
  const [searchParams] = useSearchParams()
  const catalog = useStrategyCatalog()
  const requestedStrategy = parsePolicyReference(searchParams.get('strategy'), searchParams.get('strategy_version'))
  const requestedView = searchParams.get('view') === 'research' ? 'research' : 'plain'
  const catalogEntries = useMemo(() => Array.isArray(catalog.data) ? catalog.data : [], [catalog.data])
  const catalogById = useMemo(() => newestCatalogEntryByPolicyId(catalogEntries), [catalogEntries])
  const catalogByKey = useMemo(() => new Map(catalogEntries.map((strategy) => [policyKey(strategy.policy), strategy])), [catalogEntries])
  const requestedCatalogStrategy = requestedStrategy ? catalogByKey.get(policyKey(requestedStrategy)) : undefined
  const invalidRequestedStrategy = Boolean(requestedStrategy && catalog.isSuccess && Array.isArray(catalog.data) && !requestedCatalogStrategy)

  useEffect(() => {
    strategyAnalysisStore.view = requestedView
    if (requestedCatalogStrategy && state.appliedRouteStrategyId !== policyKey(requestedCatalogStrategy.policy)) {
      strategyAnalysisStore.appliedRouteStrategyId = policyKey(requestedCatalogStrategy.policy)
      openStrategyAnalysis(requestedCatalogStrategy.policy)
    }
  }, [requestedCatalogStrategy, requestedView, state.appliedRouteStrategyId])

  const request = useMemo<StrategyBacktestRequest>(() => ({
    symbol: state.submitted.symbol,
    strategy_refs: state.submitted.strategy_refs.map((policy) => ({ ...policy })),
    range: state.submitted.range,
    monthly_day: state.submitted.monthly_day,
    contribution: state.submitted.contribution,
  }), [state.submitted])
  const effectiveRequest = useMemo<StrategyBacktestRequest | null>(() => {
    if (!requestedStrategy) return request
    if (!requestedCatalogStrategy) return null
    return state.appliedRouteStrategyId === policyKey(requestedCatalogStrategy.policy) ? request : null
  }, [request, requestedCatalogStrategy, requestedStrategy, state.appliedRouteStrategyId])
  const backtest = useStrategyBacktest(effectiveRequest)
  const result = backtest.data
  const providers = useAiProviders()
  const explain = useExplainStrategyBacktest()
  const [profileId, setProfileId] = useState('')
  const [explanation, setExplanation] = useState<{ value: AiReadOnlyExplanation; checksum: string; provider: string } | null>(null)
  const explanationProviders = (providers.data?.providers ?? []).filter((provider) => provider.capabilities.read_only_explanations)
  const effectiveProfileId = profileId || explanationProviders[0]?.id || ''

  const requestExplanation = async () => {
    if (!effectiveRequest || !effectiveProfileId) return
    setExplanation(null)
    try {
      const response = await explain.mutateAsync({ profile_id: effectiveProfileId, backtest: effectiveRequest })
      setExplanation({ value: response.explanation, checksum: response.source_checksum, provider: response.provider.display_name })
    } catch {
      // React Query retains the safe error for the actionable inline message below.
    }
  }

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <PageHeading eyebrow="策略中心 / 策略分析" title="同一只标的，同一段时间，再比较策略" description="输入任意支持的美股、A 股或港股代码。所有策略使用同一份真实日线、同样的投入节奏和共同起点，结果不会混入账户余额或演示数据。" />
      <StrategyCenterNav />

      <form className="rounded-[1.45rem] border border-[#cddfd6] bg-[#f7fbf9] p-5 shadow-[0_18px_50px_rgb(15_32_40_/_0.06)] sm:p-6" onSubmit={(event) => { event.preventDefault(); setExplanation(null); explain.reset(); submitStrategyAnalysis() }}>
        <div className="grid gap-5 lg:grid-cols-[minmax(0,1.4fr)_minmax(12rem,.55fr)_auto] lg:items-end">
          <label className="block"><span className="text-sm font-semibold text-[#102028]">回测哪只标的</span><span className="mt-1 block text-xs leading-5 text-slate-500">美股如 US.SPY，港股如 HK.00700，A 股如 SH.510300 / SZ.159915</span><input aria-label="回测标的" value={state.symbol} onChange={(event) => { strategyAnalysisStore.symbol = event.target.value }} className="mt-2 h-12 w-full rounded-xl border border-[#bfd3c9] bg-white px-4 text-base font-semibold uppercase text-[#102028] outline-none transition focus:border-[#2d6a57] focus:ring-4 focus:ring-[#2d6a57]/10" /></label>
          <label className="block"><span className="text-sm font-semibold text-[#102028]">每月投入</span><span className="mt-1 block text-xs leading-5 text-slate-500">按标的交易币种计算</span><input aria-label="每月投入金额" inputMode="decimal" value={state.contribution} onChange={(event) => { strategyAnalysisStore.contribution = event.target.value }} className="mt-2 h-12 w-full rounded-xl border border-[#bfd3c9] bg-white px-4 font-semibold text-[#102028] outline-none transition focus:border-[#2d6a57] focus:ring-4 focus:ring-[#2d6a57]/10" /></label>
          <button type="submit" disabled={!state.symbol.trim() || backtest.isFetching} className="inline-flex h-12 items-center justify-center gap-2 rounded-xl bg-[#102028] px-5 text-sm font-semibold text-white transition hover:bg-[#1a343f] disabled:cursor-not-allowed disabled:opacity-55"><RefreshCw className={`size-4 ${backtest.isFetching ? 'animate-spin' : ''}`} />{backtest.isFetching ? '正在计算' : '运行真实回测'}</button>
        </div>
        <div className="mt-5 flex flex-wrap gap-2" aria-label="选择时间范围">{strategyAnalysisRanges.map((item) => <button key={item.id} type="button" aria-pressed={state.range === item.id} onClick={() => { strategyAnalysisStore.range = item.id }} className={`rounded-full px-3.5 py-2 text-sm font-medium transition-colors ${state.range === item.id ? 'bg-[#102028] text-white' : 'border border-[#d5e2dc] bg-white text-slate-600 hover:border-[#a9c6b8]'}`}>{item.label}</button>)}</div>
      </form>

      <div className="flex w-fit rounded-full border border-slate-200 bg-white p-1 shadow-sm" aria-label="选择分析视角"><button type="button" aria-pressed={state.view === 'plain'} onClick={() => { strategyAnalysisStore.view = 'plain' }} className={`rounded-full px-3.5 py-2 text-sm font-medium transition-colors ${state.view === 'plain' ? 'bg-[#102028] text-white' : 'text-slate-500 hover:bg-slate-100 hover:text-[#102028]'}`}>直观视角</button><button type="button" aria-pressed={state.view === 'research'} onClick={() => { strategyAnalysisStore.view = 'research' }} className={`rounded-full px-3.5 py-2 text-sm font-medium transition-colors ${state.view === 'research' ? 'bg-[#102028] text-white' : 'text-slate-500 hover:bg-slate-100 hover:text-[#102028]'}`}>专业研究</button></div>

      {invalidRequestedStrategy ? <UnknownStrategyState policyId={requestedStrategy!.id} /> : catalog.isLoading && requestedStrategy ? <LoadingState label="正在确认策略目录…" /> : catalog.isError && requestedStrategy ? <CatalogErrorState /> : backtest.isLoading ? <LoadingState /> : backtest.isError ? <ErrorState error={backtest.error} /> : result ? <>
        <ProvenanceStrip response={result} />
        <AiBacktestExplanationPanel providers={explanationProviders} profileId={effectiveProfileId} onProfileChange={setProfileId} pending={explain.isPending} error={explain.error} explanation={explanation} onExplain={() => void requestExplanation()} />
        {state.view === 'plain'
          ? <PlainView series={result.result.series} marketPoints={result.result.market_points} currency={result.data.currency} symbol={result.result.symbol} selected={state.strategyRefs as readonly PolicyReference[]} catalog={catalogEntries} />
          : <ResearchView series={result.result.series} currency={result.data.currency} effectiveStart={result.result.effective_start} effectiveEnd={result.result.effective_end} contributionCount={result.result.contribution_count} catalogById={catalogById} />}
      </> : null}
    </div>
  )
}

function AiBacktestExplanationPanel({ providers, profileId, onProfileChange, pending, error, explanation, onExplain }: { providers: AiProviderProfile[]; profileId: string; onProfileChange: (value: string) => void; pending: boolean; error: Error | null; explanation: { value: AiReadOnlyExplanation; checksum: string; provider: string } | null; onExplain: () => void }) {
  return <section className="rounded-[1.35rem] border border-[#d8e5df] bg-white p-5 sm:p-6" aria-labelledby="ai-backtest-title"><div className="flex flex-col gap-5 md:flex-row md:items-end md:justify-between"><div className="max-w-2xl"><p className="inline-flex items-center gap-2 text-sm font-medium text-[#2d6a57]"><Bot className="size-4" />可选 AI 解读</p><h2 id="ai-backtest-title" className="mt-2 text-xl font-semibold tracking-[-0.03em] text-[#102028]">把真实回测结果讲成人话</h2><p className="mt-2 text-sm leading-6 text-slate-600">只有点击后，服务端才会重新计算同一份回测，并把指标与数据来源发送给你选择的 API。AI 不参与回测计算，也不会修改策略或计划。</p></div>{providers.length > 0 ? <div className="flex flex-col gap-2 sm:flex-row sm:items-end"><label className="grid gap-1 text-xs font-medium text-slate-500">解释模型<select aria-label="回测解释模型" value={profileId} onChange={(event) => onProfileChange(event.target.value)} className="h-10 min-w-52 rounded-xl border border-slate-200 bg-[#f7f9f8] px-3 text-sm text-[#102028]">{providers.map((provider) => <option key={provider.id} value={provider.id}>{provider.display_name} · {provider.model}</option>)}</select></label><Button type="button" disabled={pending} onClick={onExplain} className="h-10 rounded-full bg-[#102830] px-4">{pending ? <><Loader2 className="animate-spin" />正在解释…</> : <><Sparkles />解释这次结果</>}</Button></div> : null}</div>{providers.length === 0 ? <p className="mt-4 rounded-xl bg-[#f6f8f7] px-4 py-3 text-sm text-slate-600">本机尚未配置可解释结果的 AI profile；回测与专业指标仍可正常使用。</p> : null}{error ? <p role="alert" className="mt-4 rounded-xl border border-[#dec9a6] bg-[#fff8eb] px-4 py-3 text-sm text-[#73572f]">{describeAiActionError(error, 'AI 暂时无法解释这次结果。原始回测不受影响，请稍后重试。')}</p> : null}{explanation ? <ExplanationCard explanation={explanation.value} footer={`${explanation.provider} · 数据校验 ${explanation.checksum.slice(0, 10)}…`} /> : null}</section>
}

function ExplanationCard({ explanation, footer }: { explanation: AiReadOnlyExplanation; footer: string }) {
  return <article className="mt-5 rounded-2xl bg-[#f1f7f4] p-4 sm:p-5"><h3 className="font-semibold text-[#102028]">{explanation.headline}</h3><p className="mt-2 text-sm leading-7 text-slate-700">{explanation.summary}</p>{explanation.observations.length > 0 ? <div className="mt-4"><p className="text-xs font-semibold uppercase tracking-[0.1em] text-[#2d6a57]">从数据里能看到</p><ul className="mt-2 list-disc space-y-1.5 pl-5 text-sm leading-6 text-slate-600">{explanation.observations.map((item) => <li key={item}>{item}</li>)}</ul></div> : null}{explanation.risks.length > 0 ? <div className="mt-4"><p className="text-xs font-semibold uppercase tracking-[0.1em] text-[#8a5f26]">别忽略这些限制</p><ul className="mt-2 list-disc space-y-1.5 pl-5 text-sm leading-6 text-slate-600">{explanation.risks.map((item) => <li key={item}>{item}</li>)}</ul></div> : null}<p className="mt-4 border-t border-[#d8e5df] pt-3 text-xs text-slate-500">{footer} · 解释未保存，也不是投资建议</p></article>
}

function PlainView({ series, marketPoints, currency, symbol, selected, catalog }: { series: DynamicBacktestSeries[]; marketPoints: BacktestMarketPoint[]; currency: string; symbol: string; selected: readonly PolicyReference[]; catalog: StrategyCatalogEntry[] }) {
  const catalogById = useMemo(() => newestCatalogEntryByPolicyId(catalog), [catalog])
  const catalogByKey = useMemo(() => new Map(catalog.map((strategy) => [policyKey(strategy.policy), strategy])), [catalog])
  const selectedPolicyIds = useMemo(() => new Set(selected.map((policy) => policy.id)), [selected])
  const available = catalog.filter((strategy) => !selectedPolicyIds.has(strategy.policy.id))
  const selectionFull = selected.length >= 3
  const overlaps = useMemo(() => findOverlappingSeries(series), [series])
  const dashedSeries = useMemo(() => new Set(overlaps.map((overlap) => overlap.coveringId)), [overlaps])
  const normalizedOption = useMemo(() => buildNormalizedChartOption(series, catalogById, dashedSeries), [catalogById, dashedSeries, series])
  return <><section className="rounded-[1.45rem] border border-slate-200 bg-white p-5 shadow-sm sm:p-6">
    <div className="grid gap-6 lg:grid-cols-[15rem_minmax(0,1fr)]">
      <aside><p className="text-sm font-semibold text-[#102028]">选择要对比的策略</p><p className="mt-1 text-xs leading-5 text-slate-500">最多选择三条。改变选择后，再运行真实回测。</p><label className="mt-4 block"><span className="sr-only">添加对比策略</span><select aria-label="添加对比策略" value="" disabled={selectionFull || available.length === 0} onChange={(event) => { const strategy = catalogByKey.get(event.target.value); if (strategy) toggleAnalysisStrategy(strategy.policy) }} className="h-11 w-full rounded-xl border border-[#bfd3c9] bg-white px-3 text-sm font-medium text-[#102028] outline-none transition focus:border-[#2d6a57] focus:ring-4 focus:ring-[#2d6a57]/10 disabled:cursor-not-allowed disabled:bg-slate-50 disabled:text-slate-400"><option value="">{selectionFull ? '已选满 3 条策略' : '添加一个策略…'}</option>{available.map((strategy) => <option key={policyKey(strategy.policy)} value={policyKey(strategy.policy)}>{strategy.name}{strategy.origin === 'personal' ? ` · 个人 v${strategy.policy.version}` : ''}</option>)}</select></label><div className="mt-3 space-y-2">{selected.map((policy) => { const key = policyKey(policy); const strategy = catalogByKey.get(key); const name = strategy?.name ?? policy.id; const canRemove = selected.length > 1; return <button key={key} type="button" aria-label={canRemove ? `移除${name}` : `${name}（至少保留一个）`} aria-pressed="true" disabled={!canRemove} onClick={() => toggleAnalysisStrategy(policy)} className="flex w-full items-center gap-3 rounded-xl border border-[#a9cbbb] bg-[#f1f7f4] p-3 text-left shadow-[0_8px_24px_rgb(45_106_87_/_0.08)] transition hover:border-[#78a991] disabled:cursor-default"><span className="grid size-5 shrink-0 place-items-center rounded-full" style={{ backgroundColor: strategyAnalysisColor(policy.id) }}><Check className="size-3.5 text-white" /></span><span className="min-w-0"><span className="block truncate text-sm font-medium text-[#102028]">{name}</span><span className="mt-0.5 block text-xs text-slate-500">{scheduleLabel(strategy)}{strategy?.origin === 'personal' ? ` · 个人 v${policy.version}` : ''}</span></span></button> })}</div></aside>
      <div className="min-w-0">
        <div className="flex flex-wrap items-end justify-between gap-3">
          <div>
            <p className="inline-flex items-center gap-2 text-sm font-medium text-[#2d6a57]"><BarChart3 className="size-4" />真实策略净值走势</p>
            <h2 className="mt-2 text-2xl font-semibold tracking-[-0.035em] text-[#102028]">策略净值指数（起点 = 100）</h2>
          </div>
          <p className="max-w-md text-xs leading-5 text-slate-500">同一标的、日期和外部投入下，只比较策略如何使用资金。</p>
        </div>
        <div className="mt-4 rounded-xl border border-[#d8e5df] bg-[#f3f7f5] px-4 py-3 text-sm leading-6 text-[#40545d]" aria-label="策略净值指数说明">
          <strong className="font-semibold text-[#102028]">Y 轴怎么读：</strong>100 是共同起点；110 表示策略净值较起点上涨 10%，95 表示下跌 5%。它不是股价，也不是账户金额。系统会剔除每次新增投入对收益的影响，再把“持仓市值 + 未投入现金”的每日变化连接起来。
        </div>
        <div className="mt-5 h-[25rem] rounded-[1.2rem] bg-[#f8faf9] p-2 sm:p-4">
          <InteractiveChart option={normalizedOption} ariaLabel="策略净值指数折线图，所有策略从100开始；滚动鼠标滚轮可缩放时间范围，拖动图表可平移" />
        </div>
        <ChartInteractionHint />
        {overlaps.length > 0 && <OverlapNotice overlaps={overlaps} catalogById={catalogById} />}
      </div>
    </div>
    <div className="mt-6 grid gap-3 border-t border-slate-100 pt-6 sm:grid-cols-2 xl:grid-cols-3">{series.map((item) => <article key={item.strategy_id} className="rounded-xl border border-slate-200 bg-[#fbfcfb] p-4"><div className="flex items-center justify-between gap-3"><p className="font-medium text-[#102028]">{catalogById.get(item.strategy_id)?.name ?? item.strategy_name ?? item.strategy_id}</p><span className="text-xs text-slate-500">{currency}</span></div><p className={`mt-4 text-2xl font-semibold tracking-[-0.035em] ${item.metrics.total_return_percent >= 0 ? 'text-[#2d6a57]' : 'text-[#8a5f26]'}`}>{formatPercent(item.metrics.total_return_percent)}</p><p className="mt-1 text-xs text-slate-500">区间变化 · 最大回撤 {formatPercent(-Math.abs(item.metrics.maximum_drawdown_percent))}</p></article>)}</div>
  </section><MarketExecutionChart marketPoints={marketPoints} series={series} catalogById={catalogById} currency={currency} symbol={symbol} /></>
}

function MarketExecutionChart({ marketPoints, series, catalogById, currency, symbol }: { marketPoints: BacktestMarketPoint[]; series: DynamicBacktestSeries[]; catalogById: Map<string, StrategyCatalogEntry>; currency: string; symbol: string }) {
  const chartData = useMemo(() => buildMarketChartData(marketPoints, series, catalogById), [catalogById, marketPoints, series])
  const chartOption = useMemo(() => buildMarketExecutionChartOption(chartData, series, catalogById, currency), [catalogById, chartData, currency, series])
  return <section className="rounded-[1.45rem] border border-slate-200 bg-white p-5 shadow-sm sm:p-6" aria-labelledby="market-execution-title">
    <div className="flex flex-col gap-4 border-b border-slate-100 pb-5 md:flex-row md:items-end md:justify-between">
      <div className="max-w-3xl">
        <p className="text-sm font-medium text-[#2d6a57]">真实股价与策略动作</p>
        <h2 id="market-execution-title" className="mt-2 text-2xl font-semibold tracking-[-0.035em] text-[#102028]">{symbol} 走势与规则触发点</h2>
        <p className="mt-2 text-sm leading-6 text-slate-600">上方彩色点直接落在本次回测使用的复权价格上，下方轨道显示同一批日期。Formula 只标出规则实际命中的周期；固定投入则保留全部计划日，避免把共同评估日误当成每个策略独有的信号。</p>
      </div>
      <p className="max-w-xs text-xs leading-5 text-slate-500">标记大小固定；悬停后显示模拟投入金额和本期预算比例</p>
    </div>
    <div className="mt-4 flex flex-wrap gap-x-5 gap-y-2" aria-label="模拟投入点图例">{series.map((item, index) => <span key={item.strategy_id} className="inline-flex items-center gap-2 text-xs font-medium text-slate-600"><span aria-hidden="true" className={`size-2.5 ring-2 ring-white ${index % 3 === 0 ? 'rotate-45 rounded-[2px]' : index % 3 === 1 ? 'rounded-[2px]' : 'rounded-full'}`} style={{ backgroundColor: strategyAnalysisColor(item.strategy_id), boxShadow: '0 0 0 1px rgb(203 213 225)' }} />{catalogById.get(item.strategy_id)?.name ?? item.strategy_name ?? item.strategy_id}</span>)}</div>
    <div className="mt-4 h-[34rem] rounded-[1.2rem] bg-[#f8faf9] p-2 sm:p-4">
      <InteractiveChart option={chartOption} ariaLabel={`${symbol}复权收盘价与各策略规则触发点`} />
    </div>
    <ChartInteractionHint />
    <p className="mt-2 text-xs leading-5 text-slate-500">标记是历史规则执行结果，不是预测出的最佳买点。Formula 没有标记的计划日表示规则未命中、继续使用标准额度；悬停可查看触发日的模拟投入金额和本期预算比例。</p>
  </section>
}

function ChartInteractionHint() { return <p className="mt-3 text-xs leading-5 text-slate-500">滚动鼠标滚轮可围绕当前位置缩放时间范围；按住图表左右拖动可平移；底部时间条可以快速调整或恢复区间。</p> }

function ProvenanceStrip({ response }: { response: NonNullable<ReturnType<typeof useStrategyBacktest>['data']> }) {
  const data = response.data
  const provider = data.provider === 'opend' ? '本机 OpenD' : data.provider
  return <section className="grid gap-3 rounded-[1.2rem] border border-[#c8ddd3] bg-[#f1f7f4] p-4 md:grid-cols-[1fr_auto_auto] md:items-center"><div className="flex items-start gap-3"><span className="grid size-9 shrink-0 place-items-center rounded-full bg-white text-[#2d6a57]"><Database className="size-4" /></span><div><p className="font-semibold text-[#102028]">{response.result.symbol} · 真实日线回测</p><p className="mt-1 text-xs leading-5 text-slate-600">{provider} / {data.dataset_version} · {adjustmentLabel(data.adjustment)} · 数据校验 {data.checksum.slice(0, 10)}…</p><p className="mt-1 text-xs leading-5 text-slate-500">这不是内置演示曲线；新的任意标的回测需要可用行情源，精确缓存命中时可复用本地数据。</p></div></div><div className="text-xs text-slate-500"><span className="block">共同有效范围</span><strong className="mt-1 block text-sm text-[#102028]">{response.result.effective_start} → {response.result.effective_end}</strong></div><div className="text-xs text-slate-500"><span className="block">交易币种 / 时区</span><strong className="mt-1 block text-sm text-[#102028]">{data.currency} · {data.timezone}</strong></div></section>
}

function LoadingState({ label = '正在读取真实日线并统一计算策略…' }: { label?: string }) { return <section className="rounded-[1.45rem] border border-slate-200 bg-white p-8" role="status"><div className="h-4 w-40 animate-pulse rounded bg-slate-200" /><div className="mt-5 h-72 animate-pulse rounded-2xl bg-slate-100" /><p className="mt-4 text-sm text-slate-500">{label}</p></section> }

function UnknownStrategyState({ policyId }: { policyId: string }) { return <section className="rounded-[1.35rem] border border-[#dec9a6] bg-[#fff8eb] p-5" role="alert"><div className="flex items-start gap-3"><CircleAlert className="mt-0.5 size-5 shrink-0 text-[#8a5f26]" /><div><h2 className="font-semibold text-[#4d391d]">策略目录中没有这个策略</h2><p className="mt-2 text-sm leading-6 text-[#73572f]">“{policyId}”可能已下线或链接有误。页面不会把它替换成固定定投，请返回策略中心重新选择。</p></div></div></section> }

function CatalogErrorState() { return <section className="rounded-[1.35rem] border border-[#dec9a6] bg-[#fff8eb] p-5" role="alert"><div className="flex items-start gap-3"><CircleAlert className="mt-0.5 size-5 shrink-0 text-[#8a5f26]" /><div><h2 className="font-semibold text-[#4d391d]">暂时无法确认这个策略</h2><p className="mt-2 text-sm leading-6 text-[#73572f]">策略目录读取失败，因此不会用其他策略代替运行。请确认本机服务后重试。</p></div></div></section> }

function ErrorState({ error }: { error: Error }) {
  const unavailable = error instanceof ApiRequestError && error.status === 503
  return <section className="rounded-[1.35rem] border border-[#dec9a6] bg-[#fff8eb] p-5" role="alert"><div className="flex items-start gap-3"><CircleAlert className="mt-0.5 size-5 shrink-0 text-[#8a5f26]" /><div><h2 className="font-semibold text-[#4d391d]">{unavailable ? '真实行情源未连接' : '这次回测无法完成'}</h2><p className="mt-2 text-sm leading-6 text-[#73572f]">{unavailable ? '当前真实回测需要本机 OpenD，或命中已经保存的同范围日线。没有可用数据时不会用演示曲线替代结果。' : '请检查市场前缀、标的代码、金额和时间范围；新上市标的也可能没有足够的指标预热历史。'}</p>{unavailable && <Link to="/lab" className="mt-3 inline-flex text-sm font-semibold text-[#6c4b1d] underline decoration-[#b98b4c]/50 underline-offset-4 hover:text-[#4d391d]">前往高级实验室查看数据连接</Link>}</div></div></section>
}

interface SeriesOverlap {
  visibleId: string
  coveringId: string
}

function findOverlappingSeries(series: DynamicBacktestSeries[]): SeriesOverlap[] {
  const overlaps: SeriesOverlap[] = []
  for (let leftIndex = 0; leftIndex < series.length; leftIndex += 1) {
    for (let rightIndex = leftIndex + 1; rightIndex < series.length; rightIndex += 1) {
      if (seriesVisuallyOverlap(series[leftIndex], series[rightIndex])) {
        overlaps.push({ visibleId: series[leftIndex].strategy_id, coveringId: series[rightIndex].strategy_id })
      }
    }
  }
  return overlaps
}

function seriesVisuallyOverlap(left: DynamicBacktestSeries, right: DynamicBacktestSeries): boolean {
  if (left.normalized_points.length === 0 || left.normalized_points.length !== right.normalized_points.length) return false
  return left.normalized_points.every((point, index) => {
    const comparison = right.normalized_points[index]
    return point.date === comparison.date && Math.abs(point.value - comparison.value) <= 0.01
  })
}

function OverlapNotice({ overlaps, catalogById }: { overlaps: SeriesOverlap[]; catalogById: Map<string, StrategyCatalogEntry> }) {
  const name = (policyId: string) => catalogById.get(policyId)?.name ?? policyId
  return <div className="mt-3 rounded-xl border border-[#d7c7a7] bg-[#fbf7ee] px-4 py-3 text-sm leading-6 text-[#6d5733]" role="note" aria-label="重合曲线说明"><strong className="font-semibold text-[#4d391d]">这些策略都已完成计算：</strong>{overlaps.map((overlap) => `${name(overlap.visibleId)}与${name(overlap.coveringId)}`).join('；')}在当前区间的净值路径重合。图中用虚线露出下方曲线；重合表示策略在这些评估日产生了相同或近乎相同的资金结果，不是策略缺失。</div>
}

function parsePolicyReference(id: string | null, version: string | null): PolicyReference | null {
  const normalizedId = id?.trim()
  if (!normalizedId) return null
  const normalizedVersion = Number(version ?? '1')
  return Number.isInteger(normalizedVersion) && normalizedVersion > 0 ? { id: normalizedId as StrategyId, version: normalizedVersion } : null
}
function scheduleLabel(strategy?: StrategyCatalogEntry): string { return strategy?.default_plan.schedule_kind === 'weekly' ? '每周检查' : '每月检查' }
function formatPercent(value: number): string { return `${value >= 0 ? '+' : ''}${value.toFixed(1)}%` }
function adjustmentLabel(value: string): string { return ({ all: '全量复权', forward: '前复权', raw: '不复权' } as Record<string, string>)[value] ?? value }

function newestCatalogEntryByPolicyId(catalog: StrategyCatalogEntry[]): Map<string, StrategyCatalogEntry> {
  const entries = new Map<string, StrategyCatalogEntry>()
  for (const strategy of catalog) {
    const current = entries.get(strategy.policy.id)
    if (!current || strategy.policy.version > current.policy.version) entries.set(strategy.policy.id, strategy)
  }
  return entries
}
