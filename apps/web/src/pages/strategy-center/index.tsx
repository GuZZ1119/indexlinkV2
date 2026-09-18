import { BarChart3, ChevronDown, Loader2, Plus, RotateCcw } from 'lucide-react'
import { Link } from 'react-router'
import { useSnapshot } from 'valtio'

import { usePlans } from '@/api/queries'
import type { InvestmentPlan } from '@/api/types'
import { PageHeading } from '@/components/v2_1/page-heading'
import { StrategyCard } from '@/components/v2_1/strategy-card'
import { StrategyCenterNav } from '@/components/v2_1/strategy-center-nav'
import { consumerStrategies, type StrategyId } from '@/features/v2_1/model'
import { setActiveStrategyId, setSelectedPlanId, uiStore } from '@/stores/ui'

export default function StrategyCenterPage() {
  const { activeStrategyId } = useSnapshot(uiStore)
  const plans = usePlans()
  const activePlans = (plans.data ?? []).filter((plan) => plan.is_active)
  const selectStrategy = (strategyId: StrategyId) => setActiveStrategyId(strategyId)

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <PageHeading eyebrow="策略中心" title="先看懂，再开始坚持" description="每一份策略都明确告诉你它想解决什么、历史上经历过什么，以及最不适合它的情况。" action={<span className="inline-flex items-center gap-2 rounded-full border border-slate-200 bg-white px-4 py-2.5 text-sm text-slate-500"><Plus className="size-4" />创建策略将在下一阶段开放</span>} />
      <StrategyCenterNav />

      <ActivePlansPanel plans={activePlans} pending={plans.isPending} failed={plans.isError} />

      <section><div className="mb-5 flex items-end justify-between gap-4"><div><h2 className="text-xl font-semibold tracking-[-0.03em] text-[#102028]">从简单的方式开始</h2><p className="mt-1 text-sm text-slate-500">点击整张卡片查看策略；这不会把它伪装成你的真实计划。</p></div><span className="hidden items-center gap-1 text-sm text-slate-400 sm:inline-flex">风险筛选将在策略库上线后开放 <ChevronDown className="size-4" /></span></div><div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">{consumerStrategies.map((strategy) => <StrategyCard key={strategy.id} strategy={strategy} selected={activeStrategyId === strategy.id} onSelect={selectStrategy}><Link to="/strategy-analysis" onClick={() => setActiveStrategyId(strategy.id)} className="mx-5 mb-5 inline-flex items-center gap-1 text-sm font-medium text-[#294f60] hover:text-[#102028]">分析走势 <BarChart3 className="size-3.5" /></Link></StrategyCard>)}</div></section>

      <section className="grid gap-4 md:grid-cols-2"><InfoBlock icon={<BarChart3 />} title="回测不是承诺" text="你会看到策略过去经历了什么，也会看到费用、样本范围和最难坚持的阶段。它不能预测下一次市场。" /><InfoBlock icon={<RotateCcw />} title="选用不是复制" text="先从完整理解开始。未来可以 fork 一份策略，改成符合自己投入金额、市场和风险承受能力的个人计划。" /></section>
    </div>
  )
}

function ActivePlansPanel({ plans, pending, failed }: { plans: InvestmentPlan[]; pending: boolean; failed: boolean }) {
  return (
    <section className="rounded-[1.35rem] border border-[#cfded8] bg-[#f1f7f4] p-5 sm:p-6">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div><p className="text-sm font-medium text-[#2d6a57]">你正在坚持的计划</p><h2 className="mt-1 text-xl font-semibold tracking-[-0.03em] text-[#102028]">来自“我的计划”的真实记录</h2></div>
        <Link to="/plans" className="text-sm font-medium text-[#2d6a57] hover:text-[#1f5444]">管理我的计划 →</Link>
      </div>
      {pending ? <p className="mt-5 flex items-center gap-2 text-sm text-slate-500"><Loader2 className="size-4 animate-spin" />正在读取计划…</p> : null}
      {failed ? <p role="alert" className="mt-5 rounded-xl border border-[#d9c7a9] bg-white/70 px-4 py-3 text-sm text-slate-600">暂时无法读取真实计划。请确认本机服务正在运行。</p> : null}
      {!pending && !failed && plans.length === 0 ? <p className="mt-5 rounded-xl border border-dashed border-[#b8d5c6] bg-white/70 px-4 py-4 text-sm text-slate-600">目前没有正在执行的计划。已暂停计划和新建入口都在“我的计划”中。</p> : null}
      {!pending && !failed && plans.length > 0 ? (
        <div className="mt-5 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {plans.map((plan) => (
            <Link key={plan.id} to="/plans" onClick={() => setSelectedPlanId(plan.id)} className="rounded-xl border border-[#d4e3dc] bg-white/80 p-4 transition-colors hover:border-[#8eb7a3] hover:bg-white focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-[#b8d5c6]">
              <div className="flex items-start justify-between gap-3"><h3 className="font-semibold text-[#102028]">{plan.name}</h3><span className="rounded-full bg-[#e6f1eb] px-2.5 py-1 text-xs font-medium text-[#2d6a57]">进行中</span></div>
              <p className="mt-2 text-sm text-slate-600">{plan.symbol} · {planStrategyLabel(plan)}</p>
              <p className="mt-3 text-xs text-slate-500">{planScheduleLabel(plan)} · 每次 {formatPlanMoney(plan)}</p>
            </Link>
          ))}
        </div>
      ) : null}
    </section>
  )
}

function planStrategyLabel(plan: InvestmentPlan): string {
  if (plan.policy.id === 'fixed_dca') return '固定定投'
  if (plan.policy.id === 'core_opportunity_v1') return '自适应定投'
  return '已保存策略'
}

function planScheduleLabel(plan: InvestmentPlan): string {
  if (plan.schedule_kind === 'weekly') return `每周第 ${plan.schedule_day} 天`
  return `每月 ${plan.schedule_day} 日`
}

function formatPlanMoney(plan: InvestmentPlan): string {
  const amount = Number(plan.base_contribution)
  return Number.isFinite(amount)
    ? new Intl.NumberFormat('zh-CN', { style: 'currency', currency: plan.currency, maximumFractionDigits: 2 }).format(amount)
    : `${plan.currency} ${plan.base_contribution}`
}

function InfoBlock({ icon, title, text }: { icon: React.ReactNode; title: string; text: string }) {
  return <div className="flex gap-4 rounded-[1.2rem] border border-slate-200 bg-white p-5"><span className="grid size-9 shrink-0 place-items-center rounded-full bg-[#e5eff4] text-[#294f60]">{icon}</span><div><h3 className="font-semibold text-[#102028]">{title}</h3><p className="mt-1.5 text-sm leading-6 text-slate-600">{text}</p></div></div>
}
