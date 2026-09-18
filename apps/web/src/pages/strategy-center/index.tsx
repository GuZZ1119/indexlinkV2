import { BarChart3, ChevronDown, GitCompareArrows, Plus, RotateCcw } from 'lucide-react'
import { useState } from 'react'
import { Link } from 'react-router'
import { useSnapshot } from 'valtio'

import { PageHeading } from '@/components/v2_1/page-heading'
import { StrategyCard } from '@/components/v2_1/strategy-card'
import { StrategyCenterNav } from '@/components/v2_1/strategy-center-nav'
import { compareStrategies, consumerStrategies, findConsumerStrategy, type StrategyId } from '@/features/v2_1/model'
import { setActiveStrategyId, uiStore } from '@/stores/ui'

export default function StrategyCenterPage() {
  const { activeStrategyId } = useSnapshot(uiStore)
  const [compareWith, setCompareWith] = useState<StrategyId>('steady-dca')
  const [showComparison, setShowComparison] = useState(false)
  const [selectionNotice, setSelectionNotice] = useState<string | null>(null)
  const current = findConsumerStrategy(activeStrategyId)
  const rows = compareStrategies(current.id, compareWith)
  const selectStrategy = (strategyId: StrategyId) => {
    setActiveStrategyId(strategyId)
    setSelectionNotice(`已选用“${findConsumerStrategy(strategyId).name}”。个人中心已同步更新。`)
  }

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <PageHeading eyebrow="策略中心" title="先看懂，再开始坚持" description="每一份策略都明确告诉你它想解决什么、历史上经历过什么，以及最不适合它的情况。" action={<span className="inline-flex items-center gap-2 rounded-full border border-slate-200 bg-white px-4 py-2.5 text-sm text-slate-500"><Plus className="size-4" />创建策略将在下一阶段开放</span>} />
      <StrategyCenterNav />

      <section className="rounded-[1.35rem] border border-[#cfded8] bg-[#f1f7f4] p-5 sm:p-6">
        <div className="flex flex-col justify-between gap-5 md:flex-row md:items-center"><div><p className="text-sm font-medium text-[#2d6a57]">你正在坚持</p><h2 className="mt-1 text-xl font-semibold tracking-[-0.03em] text-[#102028]">{current.name}</h2><p className="mt-2 text-sm leading-6 text-slate-600">{current.strength}</p></div><button type="button" onClick={() => setShowComparison((value) => !value)} className="inline-flex items-center justify-center gap-2 rounded-full bg-[#102028] px-4 py-2.5 text-sm font-medium text-white hover:bg-[#1c343f]"><GitCompareArrows className="size-4" />{showComparison ? '收起对比' : '和其他策略对比'}</button></div>
        {showComparison && <div className="mt-6 border-t border-[#cfded8] pt-5"><div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between"><label className="text-sm text-slate-600">和 <select aria-label="选择对比策略" value={compareWith} onChange={(event) => setCompareWith(event.target.value as StrategyId)} className="mx-1 rounded-lg border border-slate-300 bg-white px-2 py-1 text-[#102028] outline-none focus-visible:ring-2 focus-visible:ring-[#2d6a57]">{consumerStrategies.filter((strategy) => strategy.id !== current.id).map((strategy) => <option key={strategy.id} value={strategy.id}>{strategy.name}</option>)}</select> 对比</label><span className="text-xs text-slate-500">示例回测仅用于展示界面，不代表实时收益</span></div><div className="mt-4 overflow-hidden rounded-xl border border-[#d8e5df] bg-white"><div className="grid grid-cols-[7rem_minmax(8rem,1fr)_minmax(8rem,1fr)] border-b border-slate-100 bg-[#fafcfb] text-xs font-medium text-slate-500"><div className="p-3">比较项</div><div className="p-3">{current.shortName}</div><div className="p-3">{findConsumerStrategy(compareWith).shortName}</div></div>{rows.map((row) => <div key={row.label} className="grid grid-cols-[7rem_minmax(8rem,1fr)_minmax(8rem,1fr)] border-b border-slate-100 last:border-0 text-sm"><div className="p-3 text-slate-500">{row.label}</div><div className="p-3 leading-6 text-[#102028]">{row.left}</div><div className="p-3 leading-6 text-[#102028]">{row.right}</div></div>)}</div></div>}
      </section>

      {selectionNotice && <p role="status" className="rounded-xl border border-[#b8d5c6] bg-[#f1f7f4] px-4 py-3 text-sm text-[#245a49]">{selectionNotice}</p>}

      <section><div className="mb-5 flex items-end justify-between gap-4"><div><h2 className="text-xl font-semibold tracking-[-0.03em] text-[#102028]">从简单的方式开始</h2><p className="mt-1 text-sm text-slate-500">目前是本地精选策略库；公开分享与真实回测将在后续版本接入。</p></div><span className="hidden items-center gap-1 text-sm text-slate-400 sm:inline-flex">风险筛选将在策略库上线后开放 <ChevronDown className="size-4" /></span></div><div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">{consumerStrategies.map((strategy) => <StrategyCard key={strategy.id} strategy={strategy} selected={activeStrategyId === strategy.id} onSelect={selectStrategy}><Link to="/strategy-analysis" onClick={() => setActiveStrategyId(strategy.id)} className="inline-flex items-center gap-1 text-sm font-medium text-[#294f60] hover:text-[#102028]">分析走势 <BarChart3 className="size-3.5" /></Link></StrategyCard>)}</div></section>

      <section className="grid gap-4 md:grid-cols-2"><InfoBlock icon={<BarChart3 />} title="回测不是承诺" text="你会看到策略过去经历了什么，也会看到费用、样本范围和最难坚持的阶段。它不能预测下一次市场。" /><InfoBlock icon={<RotateCcw />} title="选用不是复制" text="先从完整理解开始。未来可以 fork 一份策略，改成符合自己投入金额、市场和风险承受能力的个人计划。" /></section>
    </div>
  )
}

function InfoBlock({ icon, title, text }: { icon: React.ReactNode; title: string; text: string }) {
  return <div className="flex gap-4 rounded-[1.2rem] border border-slate-200 bg-white p-5"><span className="grid size-9 shrink-0 place-items-center rounded-full bg-[#e5eff4] text-[#294f60]">{icon}</span><div><h3 className="font-semibold text-[#102028]">{title}</h3><p className="mt-1.5 text-sm leading-6 text-slate-600">{text}</p></div></div>
}
