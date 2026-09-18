import { BarChart3, Check, CircleAlert, Info } from 'lucide-react'
import { useMemo, useState } from 'react'
import { CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import { useSnapshot } from 'valtio'

import { PageHeading } from '@/components/v2_1/page-heading'
import { ProfessionalResearchPanel } from '@/components/v2_1/professional-research-panel'
import { StrategyCenterNav } from '@/components/v2_1/strategy-center-nav'
import { buildNormalizedStrategyAnalysis, consumerStrategies, findConsumerStrategy, strategyAnalysisColors, strategyAnalysisRanges, type StrategyAnalysisRange, type StrategyId } from '@/features/v2_1/model'
import { uiStore } from '@/stores/ui'

function formatChange(value: number): string {
  return `${value >= 0 ? '+' : ''}${value.toFixed(1)}%`
}

type AnalysisView = 'plain' | 'research'

export default function StrategyAnalysisPage() {
  const { activeStrategyId } = useSnapshot(uiStore)
  const [range, setRange] = useState<StrategyAnalysisRange>('3y')
  const [view, setView] = useState<AnalysisView>('plain')
  const [strategyIds, setStrategyIds] = useState<StrategyId[]>([activeStrategyId])
  const analysis = useMemo(() => buildNormalizedStrategyAnalysis(strategyIds, range), [range, strategyIds])

  const toggleStrategy = (id: StrategyId) => {
    setStrategyIds((current) => current.includes(id)
      ? (current.length === 1 ? current : current.filter((currentId) => currentId !== id))
      : [...current, id])
  }

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <PageHeading eyebrow="策略中心 / 策略分析" title="把同一段路，放在一起看" description="选定时间范围后，每条曲线都从 100 开始。你看到的是策略在同一段时间里的变化体验，而不是谁投入的钱更多。" />
      <StrategyCenterNav />

      <div className="flex w-fit rounded-full border border-slate-200 bg-white p-1 shadow-sm" aria-label="选择分析视角"><button type="button" aria-pressed={view === 'plain'} onClick={() => setView('plain')} className={`rounded-full px-3.5 py-2 text-sm font-medium transition-colors ${view === 'plain' ? 'bg-[#102028] text-white' : 'text-slate-500 hover:bg-slate-100 hover:text-[#102028]'}`}>直观视角</button><button type="button" aria-pressed={view === 'research'} onClick={() => setView('research')} className={`rounded-full px-3.5 py-2 text-sm font-medium transition-colors ${view === 'research' ? 'bg-[#102028] text-white' : 'text-slate-500 hover:bg-slate-100 hover:text-[#102028]'}`}>专业研究</button></div>

      {view === 'plain' ? <>
      <section className="rounded-[1.45rem] border border-slate-200 bg-white p-5 shadow-sm sm:p-6">
        <div className="flex flex-col gap-6 border-b border-slate-100 pb-6 lg:flex-row lg:items-end lg:justify-between">
          <div><p className="inline-flex items-center gap-2 text-sm font-medium text-[#2d6a57]"><BarChart3 className="size-4" />走势对比</p><h2 className="mt-2 text-2xl font-semibold tracking-[-0.035em] text-[#102028]">归一化指数（起点 = 100）</h2><p className="mt-2 max-w-xl text-sm leading-6 text-slate-600">只比较变化幅度。换一个时间范围，所有已选策略都会在该范围的起点重新归一化。</p></div>
          <div className="flex flex-wrap gap-2" aria-label="选择时间范围">{strategyAnalysisRanges.map((item) => <button key={item.id} type="button" aria-pressed={range === item.id} onClick={() => setRange(item.id)} className={`rounded-full px-3.5 py-2 text-sm font-medium transition-colors ${range === item.id ? 'bg-[#102028] text-white' : 'border border-slate-200 text-slate-600 hover:border-slate-300 hover:bg-slate-50'}`}>{item.label}</button>)}</div>
        </div>

        <div className="mt-6 grid gap-6 lg:grid-cols-[15rem_minmax(0,1fr)]">
          <aside><p className="text-sm font-medium text-[#102028]">选择要对比的策略</p><p className="mt-1 text-xs leading-5 text-slate-500">至少保留一个；最多可同时比较三条。</p><div className="mt-4 space-y-2">{consumerStrategies.map((strategy) => { const selected = strategyIds.includes(strategy.id); return <button key={strategy.id} type="button" aria-pressed={selected} onClick={() => toggleStrategy(strategy.id)} className={`flex w-full items-center gap-3 rounded-xl border p-3 text-left transition-colors ${selected ? 'border-[#b8d5c6] bg-[#f1f7f4]' : 'border-slate-200 hover:border-slate-300'}`}><span className="grid size-5 shrink-0 place-items-center rounded-full border" style={{ borderColor: strategyAnalysisColors[strategy.id], backgroundColor: selected ? strategyAnalysisColors[strategy.id] : 'transparent' }}>{selected && <Check className="size-3.5 text-white" />}</span><span><span className="block text-sm font-medium text-[#102028]">{strategy.shortName}</span><span className="mt-0.5 block text-xs text-slate-500">{strategy.cadence}</span></span></button> })}</div></aside>

          <div className="min-w-0"><div className="h-[22rem] rounded-[1.2rem] bg-[#f8faf9] p-3 sm:p-5"><ResponsiveContainer width="100%" height="100%" minWidth={0} minHeight={1}><LineChart data={analysis.points} margin={{ top: 8, right: 12, left: -12, bottom: 0 }}><CartesianGrid vertical={false} stroke="#dce7e1" strokeDasharray="3 3" /><XAxis dataKey="date" tickLine={false} axisLine={false} minTickGap={38} tick={{ fill: '#718096', fontSize: 12 }} /><YAxis domain={['auto', 'auto']} tickLine={false} axisLine={false} width={46} tickFormatter={(value) => Number(value).toFixed(0)} tick={{ fill: '#718096', fontSize: 12 }} /><Tooltip cursor={{ stroke: '#b8d5c6', strokeDasharray: '3 3' }} contentStyle={{ borderRadius: 14, borderColor: '#d8e5df', boxShadow: '0 10px 30px rgb(15 32 40 / 0.08)' }} formatter={(value, name) => [`${Number(value).toFixed(1)} 指数`, `${name}`]} />{strategyIds.map((id) => <Line key={id} type="monotone" dataKey={id} name={findConsumerStrategy(id).shortName} stroke={strategyAnalysisColors[id]} strokeWidth={3} dot={false} activeDot={{ r: 4 }} />)}</LineChart></ResponsiveContainer></div><div className="mt-4 flex flex-wrap gap-x-5 gap-y-2">{strategyIds.map((id) => <span key={id} className="inline-flex items-center gap-2 text-sm text-slate-600"><span className="size-2.5 rounded-full" style={{ backgroundColor: strategyAnalysisColors[id] }} />{findConsumerStrategy(id).shortName}</span>)}</div></div>
        </div>

        <div className="mt-6 grid gap-3 border-t border-slate-100 pt-6 sm:grid-cols-2 xl:grid-cols-3">{analysis.summaries.map((summary) => { const strategy = findConsumerStrategy(summary.id); return <article key={summary.id} className="rounded-xl border border-slate-200 bg-[#fbfcfb] p-4"><div className="flex items-center justify-between gap-3"><p className="font-medium text-[#102028]">{strategy.shortName}</p><span className="text-xs text-slate-500">{strategy.risk}</span></div><p className={`mt-4 text-2xl font-semibold tracking-[-0.035em] ${summary.change >= 0 ? 'text-[#2d6a57]' : 'text-[#294f60]'}`}>{formatChange(summary.change)}</p><p className="mt-1 text-xs text-slate-500">区间变化 · 期末指数 {summary.endIndex.toFixed(1)}</p></article> })}</div>
      </section>

      <section className="grid gap-4 md:grid-cols-3"><InfoCard icon={<Info />} title="本页的数据是什么" text="当前使用本地确定性示例序列来完成交互与视觉验证，不是策略的真实回测结论。" /><InfoCard icon={<CircleAlert />} title="比较时要记住什么" text="归一化能公平比较路径，却不能说明未来收益；请同时看规则、适用人群和限制。" /><InfoCard icon={<BarChart3 />} title="真实回测接入后" text="将替换为带策略版本、数据集、费用与样本范围的可复核结果，图表交互保持不变。" /></section>
      </> : <ProfessionalResearchPanel />}
    </div>
  )
}

function InfoCard({ icon, title, text }: { icon: React.ReactNode; title: string; text: string }) {
  return <div className="rounded-[1.2rem] border border-slate-200 bg-white p-5"><span className="grid size-9 place-items-center rounded-full bg-[#e5eff4] text-[#294f60]">{icon}</span><h2 className="mt-4 font-semibold text-[#102028]">{title}</h2><p className="mt-2 text-sm leading-6 text-slate-600">{text}</p></div>
}
