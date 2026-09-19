import { BarChart3, Database, Loader2, Plus, RotateCcw } from 'lucide-react'
import { Link } from 'react-router'

import { usePlans, useStrategyCatalog } from '@/api/queries'
import type { InvestmentPlan, StrategyCatalogEntry } from '@/api/types'
import { PageHeading } from '@/components/v2_1/page-heading'
import { StrategyCard } from '@/components/v2_1/strategy-card'
import { StrategyCenterNav } from '@/components/v2_1/strategy-center-nav'
import type { StrategyId } from '@/features/v2_1/model'
import { setActiveStrategyId, setSelectedPlanId } from '@/stores/ui'

export default function StrategyCenterPage() {
  const plans = usePlans()
  const catalog = useStrategyCatalog()
  const activePlans = (plans.data ?? []).filter((plan) => plan.is_active)

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <PageHeading
        eyebrow="策略中心"
        title="先看规则，再决定要不要坚持"
        description="这里的每一项都来自本机服务：规则有固定版本，历史研究有相同的时间和成本口径，不能运行的策略不会开放创建。"
        action={<span className="inline-flex items-center gap-2 rounded-full border border-[#cfded8] bg-[#f1f7f4] px-4 py-2.5 text-sm text-[#2d6a57]"><Database className="size-4" />官方 Formula V1 目录</span>}
      />
      <StrategyCenterNav />

      <ActivePlansPanel plans={activePlans} pending={plans.isPending} failed={plans.isError} />

      <section aria-labelledby="official-strategies-heading">
        <div className="mb-5 flex items-end justify-between gap-4">
          <div>
            <h2 id="official-strategies-heading" className="text-xl font-semibold tracking-[-0.03em] text-[#102028]">从能解释清楚的规则开始</h2>
            <p className="mt-1 max-w-3xl text-sm leading-6 text-slate-500">点击卡片会进入该策略的直观分析；“建立计划”才会进入你的个人计划，浏览本身不会改变任何数据。</p>
          </div>
        </div>

        {catalog.isPending ? <div className="flex min-h-48 items-center justify-center rounded-[1.35rem] border border-slate-200 bg-white text-sm text-slate-500"><Loader2 className="mr-2 size-4 animate-spin" />正在读取官方策略目录…</div> : null}
        {catalog.isError ? <div role="alert" className="rounded-[1.35rem] border border-[#d9c7a9] bg-[#fffaf1] px-5 py-6 text-sm leading-6 text-slate-700"><p className="font-medium text-[#6f511f]">暂时无法读取策略目录</p><p className="mt-1">请确认本机 Rust 服务已经更新并正在运行。页面不会用静态策略或演示收益代替真实响应。</p></div> : null}
        {catalog.data?.length === 0 ? <div className="rounded-[1.35rem] border border-dashed border-slate-300 bg-white px-5 py-8 text-sm text-slate-600">服务当前没有发布可供普通用户采用的策略。</div> : null}
        {catalog.data && catalog.data.length > 0 ? (
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
            {catalog.data.map((strategy) => (
              <StrategyCard key={`${strategy.policy.id}@${strategy.policy.version}`} strategy={strategy} analysisHref={`/strategy-analysis?strategy=${catalogStrategyId(strategy.policy.id)}&view=plain`} onOpenAnalysis={() => setActiveStrategyId(catalogStrategyId(strategy.policy.id))}>
                <StrategyAction strategy={strategy} />
              </StrategyCard>
            ))}
          </div>
        ) : null}
      </section>

      <section className="grid gap-4 md:grid-cols-2">
        <InfoBlock icon={<BarChart3 />} title="回测是一次统一体检" text="Formula 策略和固定定投使用相同外部现金流、费用与成交时点。它能暴露规则的缺点，不能承诺未来收益。" />
        <InfoBlock icon={<RotateCcw />} title="公式只管理执行节奏" text="当前两条 Formula 策略保留 70% 固定核心投入，只调整 30% 弹性额度；确认结果仍由你手工记录。" />
      </section>
    </div>
  )
}

function catalogStrategyId(policyId: string): StrategyId {
  if (policyId === 'dsl_ma200_trend_guard') return 'ma200-trend-guard'
  if (policyId === 'dsl_growth_volatility_balance') return 'growth-volatility-balance'
  return 'steady-dca'
}

function StrategyAction({ strategy }: { strategy: StrategyCatalogEntry }) {
  if (!strategy.adoptable) {
    return <span className="inline-flex rounded-full border border-slate-200 bg-slate-100 px-4 py-2.5 text-sm font-medium text-slate-500">研究未通过，暂不可创建</span>
  }
  const search = new URLSearchParams({
    policy_id: strategy.policy.id,
    policy_version: String(strategy.policy.version),
  })
  return (
    <Link to={`/plans?${search.toString()}#new-plan`} className="inline-flex items-center gap-2 rounded-full bg-[#102830] px-4 py-2.5 text-sm font-semibold text-white shadow-[0_8px_20px_rgba(16,40,48,0.16)] transition-colors hover:bg-[#1d3a43] focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-[#8eb7a3]">
      <Plus className="size-4" />用这个策略建立计划
    </Link>
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
  if (plan.policy.id === 'dsl_ma200_trend_guard') return '200 日均线趋势保护'
  if (plan.policy.id === 'dsl_growth_volatility_balance') return '增长与波动平衡'
  if (plan.policy.id === 'core_opportunity_v1') return '旧自适应策略'
  return '自定义策略'
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
