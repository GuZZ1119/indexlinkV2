import { BookOpenCheck, CircleAlert, LoaderCircle, ShieldCheck } from 'lucide-react'
import { useMemo, useState } from 'react'

import { useStrategies, useStrategyAdmission } from '@/api/queries'
import type { StrategyAdmissionAsset, StrategyAdmissionMetrics, StrategyAdmissionReport } from '@/api/types'

type MetricRow = { label: string; value: (metrics: StrategyAdmissionMetrics) => string }

const metricRows: readonly MetricRow[] = [
  { label: 'XIRR', value: (metrics) => percent(metrics.xirr_percent) },
  { label: '期末净值', value: (metrics) => `$${metrics.terminal_wealth_usd.toLocaleString(undefined, { maximumFractionDigits: 0 })}` },
  { label: '最大回撤', value: (metrics) => percent(metrics.maximum_drawdown_percent) },
  { label: '年化波动率', value: (metrics) => percent(metrics.annualized_volatility_percent) },
  { label: 'Sortino 比率', value: (metrics) => decimal(metrics.sortino_ratio) },
  { label: '现金使用率', value: (metrics) => percent(metrics.cash_utilisation_percent, 1) },
]

/** Read and display only the fixed-fixture admission report already produced by the Rust backend. */
export function ProfessionalResearchPanel() {
  const strategies = useStrategies()
  const admission = useStrategyAdmission()
  const [selectedKey, setSelectedKey] = useState<string | null>(null)
  const [requestedKey, setRequestedKey] = useState<string | null>(null)
  const selectedStrategy = useMemo(() => strategies.data?.find((strategy) => policyKey(strategy.policy) === selectedKey) ?? strategies.data?.[0], [selectedKey, strategies.data])
  const currentKey = selectedStrategy ? policyKey(selectedStrategy.policy) : null
  const report = requestedKey === currentKey ? admission.data : undefined

  return (
    <section aria-label="专业研究摘要" className="rounded-[1.45rem] border border-[#cfded8] bg-white p-5 shadow-sm sm:p-6">
      <header className="flex flex-col gap-4 border-b border-slate-100 pb-6 lg:flex-row lg:items-end lg:justify-between">
        <div><p className="inline-flex items-center gap-2 text-sm font-medium text-[#294f60]"><BookOpenCheck className="size-4" />后端固定样本研究</p><h2 className="mt-2 text-2xl font-semibold tracking-[-0.035em] text-[#102028]">专业视角：策略与 Fixed DCA 同口径对照</h2><p className="mt-2 max-w-2xl text-sm leading-6 text-slate-600">只读取 Rust 后端已保存 DSL 策略的准入报告；策略、样本、现金流、成本与成交时点由后端固定。它不使用上方的演示曲线。</p></div>
        <span className="inline-flex w-fit items-center gap-2 rounded-full bg-[#e5eff4] px-3 py-2 text-xs font-medium text-[#294f60]"><ShieldCheck className="size-3.5" />研究信息，不构成收益承诺</span>
      </header>

      <div className="mt-6 grid gap-5 lg:grid-cols-[minmax(15rem,.7fr)_minmax(0,1.3fr)]">
        <div className="rounded-[1.2rem] bg-[#f6f8f6] p-4"><p className="text-sm font-medium text-[#102028]">选择已保存的策略版本</p><p className="mt-1 text-xs leading-5 text-slate-500">内置 Fixed DCA 与 70 / 20 / 10 尚未公开统一准入报告；请在高级实验室保存 DSL 后读取其真实研究结果。</p>{strategies.isLoading ? <p className="mt-4 inline-flex items-center gap-2 text-sm text-slate-500"><LoaderCircle className="size-4 animate-spin" />正在读取已保存策略…</p> : strategies.error ? <Unavailable text="后端尚未连接，暂时无法读取专业研究结果。" /> : !selectedStrategy ? <Unavailable text="还没有已保存的 DSL 策略。可先在高级实验室创建并保存一份受限策略。" /> : <div className="mt-4 space-y-3"><label className="grid gap-1.5 text-sm font-medium text-[#102028]">策略版本<select aria-label="选择专业研究策略" value={currentKey ?? ''} onChange={(event) => { setSelectedKey(event.target.value); setRequestedKey(null) }} className="h-10 rounded-xl border border-slate-300 bg-white px-3 text-sm font-normal outline-none focus-visible:ring-2 focus-visible:ring-[#2d6a57]">{strategies.data?.map((strategy) => <option key={policyKey(strategy.policy)} value={policyKey(strategy.policy)}>{strategy.name} · {policyKey(strategy.policy)}</option>)}</select></label><button type="button" onClick={() => { if (selectedStrategy) { setRequestedKey(currentKey); admission.mutate(selectedStrategy.policy) } }} disabled={admission.isPending} className="inline-flex w-full items-center justify-center gap-2 rounded-full bg-[#102028] px-4 py-2.5 text-sm font-medium text-white transition-colors hover:bg-[#1c343f] disabled:cursor-wait disabled:bg-slate-500">{admission.isPending ? <><LoaderCircle className="size-4 animate-spin" />读取中…</> : '读取固定样本研究'}</button>{admission.error && <Unavailable text="后端未能返回这份策略的研究报告。请确认策略已保存且服务正在运行。" />}</div>}</div>

        <div>{report ? <ResearchReport report={report} strategyName={selectedStrategy?.name ?? '策略'} /> : <EmptyResearch />}</div>
      </div>
    </section>
  )
}

function ResearchReport({ report, strategyName }: { report: StrategyAdmissionReport; strategyName: string }) {
  if (!report.eligible) return <Unavailable text={report.reason ?? '该策略未通过核心桶或预算安全检查，因此不展示为可采用的研究结果。'} />
  return <div className="space-y-4">{report.assets.map((asset) => <AssetResearch key={asset.symbol} asset={asset} strategyName={strategyName} />)}</div>
}

function AssetResearch({ asset, strategyName }: { asset: StrategyAdmissionAsset; strategyName: string }) {
  return <article className="overflow-hidden rounded-[1.2rem] border border-slate-200"><div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-100 bg-[#fbfcfb] px-4 py-3"><div><h3 className="font-semibold text-[#102028]">{asset.symbol}</h3><p className="mt-0.5 text-xs text-slate-500">{asset.observations} 次观察 · 证据覆盖 {asset.evidence_start_as_of} 至 {asset.evidence_end_as_of}</p></div><span className="rounded-full bg-[#f1f7f4] px-2.5 py-1 text-xs font-medium text-[#2d6a57]">{asset.rolling_out_of_sample.length} 个滚动窗口</span></div><div className="overflow-x-auto"><table className="min-w-[34rem] w-full text-sm"><thead className="text-left text-xs text-slate-500"><tr><th className="px-4 py-3 font-medium">指标</th><th className="px-4 py-3 font-medium">{strategyName}</th><th className="px-4 py-3 font-medium">Fixed DCA</th></tr></thead><tbody>{metricRows.map((row) => <tr key={row.label} className="border-t border-slate-100"><th className="px-4 py-3 text-left font-medium text-[#102028]">{row.label}</th><td className="px-4 py-3 text-slate-700">{row.value(asset.strategy)}</td><td className="px-4 py-3 text-slate-700">{row.value(asset.fixed_dca)}</td></tr>)}</tbody></table></div><details className="border-t border-slate-100 px-4 py-3"><summary className="cursor-pointer text-sm font-medium text-[#294f60]">查看滚动样本外窗口</summary><div className="mt-3 grid gap-2 sm:grid-cols-2">{asset.rolling_out_of_sample.map((window) => <div key={`${window.start_as_of}-${window.end_as_of}`} className="rounded-lg bg-[#f6f8f6] p-3 text-xs text-slate-600"><p className="font-medium text-[#102028]">{window.start_as_of} 至 {window.end_as_of}</p><p className="mt-1">{window.observations} 次观察 · 策略回撤 {percent(window.strategy.maximum_drawdown_percent)} · DCA 回撤 {percent(window.fixed_dca.maximum_drawdown_percent)}</p></div>)}</div></details></article>
}

function EmptyResearch() { return <div className="flex h-full min-h-64 flex-col justify-center rounded-[1.2rem] border border-dashed border-slate-300 bg-[#fbfcfb] p-5"><p className="font-medium text-[#102028]">专业结果会在这里出现</p><p className="mt-2 max-w-md text-sm leading-6 text-slate-600">选择一份已保存 DSL 策略后，主动读取后端的固定样本报告。这里会并列显示 XIRR、期末净值、回撤、波动率、Sortino、现金使用率和滚动样本外窗口。</p></div> }

function Unavailable({ text }: { text: string }) { return <p role="status" className="mt-4 flex gap-2 rounded-xl border border-[#d8e5df] bg-white p-3 text-sm leading-6 text-slate-600"><CircleAlert className="mt-0.5 size-4 shrink-0 text-[#294f60]" />{text}</p> }
function policyKey(policy: { id: string; version: number }) { return `${policy.id}@${policy.version}` }
function percent(value: number | undefined, digits = 2) { return value === undefined ? '样本不足' : `${value.toFixed(digits)}%` }
function decimal(value: number | undefined) { return value === undefined ? '样本不足' : value.toFixed(2) }
