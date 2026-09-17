import {
  ArrowRight,
  CalendarDays,
  CheckCircle2,
  CircleDollarSign,
  Loader2,
  Pause,
  Play,
  ShieldCheck,
  Trash2,
} from 'lucide-react'
import { useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router'
import { useSnapshot } from 'valtio'

import {
  useCreatePlan,
  useDeletePlan,
  usePlans,
  usePreviewAutomaticDecision,
  useUpdatePlan,
} from '@/api/queries'
import type { CreateInvestmentPlanRequest, InvestmentPlan } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { PageHeading } from '@/components/v2_1/page-heading'
import { setSelectedPlanId, uiStore } from '@/stores/ui'

type MinimalPlanDraft = {
  name: string
  symbol: string
  amount: string
  scheduleKind: 'monthly' | 'weekly'
  scheduleDay: number
}

const weekdays = [
  { value: 1, label: '星期一' },
  { value: 2, label: '星期二' },
  { value: 3, label: '星期三' },
  { value: 4, label: '星期四' },
  { value: 5, label: '星期五' },
  { value: 6, label: '星期六' },
  { value: 7, label: '星期日' },
] as const

/** Consumer plan setup: only the choices required for a zero-dependency Fixed DCA plan. */
export default function PlansPage() {
  const navigate = useNavigate()
  const { selectedPlanId } = useSnapshot(uiStore)
  const plans = usePlans()
  const create = useCreatePlan()
  const prepareDecision = usePreviewAutomaticDecision()
  const update = useUpdatePlan()
  const remove = useDeletePlan()
  const [draft, setDraft] = useState<MinimalPlanDraft>(initialDraft)
  const [savedWithoutAdvice, setSavedWithoutAdvice] = useState<InvestmentPlan | null>(null)

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    create.reset()
    prepareDecision.reset()
    setSavedWithoutAdvice(null)
    const symbol = draft.symbol.trim().toUpperCase()
    const payload = fixedDcaPayload(draft, symbol)
    let created: InvestmentPlan
    try {
      created = await create.mutateAsync(payload)
    } catch {
      return
    }
    setSelectedPlanId(created.id)
    try {
      await prepareDecision.mutateAsync({ planId: created.id })
    } catch {
      setSavedWithoutAdvice(created)
      return
    }
    setDraft(initialDraft())
    navigate('/personal')
  }

  const retryAdvice = async () => {
    if (!savedWithoutAdvice) return
    try {
      await prepareDecision.mutateAsync({ planId: savedWithoutAdvice.id })
    } catch {
      return
    }
    setSavedWithoutAdvice(null)
    navigate('/personal')
  }

  const requestError = create.error ?? plans.error
  const saving = create.isPending || prepareDecision.isPending

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <PageHeading
        eyebrow="建立计划"
        title="把决定做一次，之后按节奏继续"
        description="V2.1 先只建立固定定投：不看短期行情，不需要 AI、券商连接、Docker 或任何 API Key。"
      />

      <section className="grid gap-7 lg:grid-cols-[minmax(0,1fr)_22rem]">
        <form onSubmit={(event) => void submit(event)} className="rounded-[1.6rem] bg-[#102028] p-6 text-white sm:p-8 lg:p-10">
          <div className="max-w-2xl">
            <p className="text-sm text-[#b8d5c6]">你的固定定投</p>
            <h2 className="mt-3 text-2xl font-semibold tracking-[-0.035em] sm:text-3xl">先确定标的、金额和日期</h2>
            <p className="mt-3 text-sm leading-7 text-slate-300">系统会保存一条可审计的计划，并按你选择的日期生成安排。所有实际操作仍由你在自己的券商完成。</p>
          </div>

          <div className="mt-7 grid gap-5 sm:grid-cols-2">
            <PlanField label="投资标的" hint="当前最小版本以 USD 指数 ETF 为主，例如 VOO。">
              <Input required aria-label="投资标的" autoCapitalize="characters" value={draft.symbol} onChange={(event) => setDraft((current) => ({ ...current, symbol: event.target.value }))} placeholder="VOO" className="h-11 border-white/15 bg-white text-[#102028]" />
            </PlanField>
            <PlanField label="每次投入金额（USD）" hint="这也是本计划单次投入的安全上限。">
              <Input required aria-label="每次投入金额（USD）" inputMode="decimal" value={draft.amount} onChange={(event) => setDraft((current) => ({ ...current, amount: event.target.value }))} placeholder="1000.00" className="h-11 border-white/15 bg-white text-[#102028]" />
            </PlanField>
            <PlanField label="执行节奏" hint="固定日期只定义提醒和建议，不会自动下单。">
              <select
                aria-label="执行节奏"
                value={draft.scheduleKind}
                onChange={(event) => {
                  const scheduleKind = event.target.value as MinimalPlanDraft['scheduleKind']
                  setDraft((current) => ({ ...current, scheduleKind, scheduleDay: defaultScheduleDay(scheduleKind) }))
                }}
                className="h-11 rounded-xl border border-white/15 bg-white px-3 text-sm text-[#102028] outline-none focus-visible:ring-3 focus-visible:ring-[#b8d5c6]/50"
              >
                <option value="monthly">每月一次</option>
                <option value="weekly">每周一次</option>
              </select>
            </PlanField>
            <PlanField label={draft.scheduleKind === 'monthly' ? '每月几号' : '每周哪一天'} hint="新计划会立即准备一次当前状态，只有执行日才会显示待执行。">
              {draft.scheduleKind === 'monthly' ? (
                <Input required aria-label="每月几号" type="number" min={1} max={28} value={draft.scheduleDay} onChange={(event) => setDraft((current) => ({ ...current, scheduleDay: Number(event.target.value) }))} className="h-11 border-white/15 bg-white text-[#102028]" />
              ) : (
                <select aria-label="每周哪一天" value={draft.scheduleDay} onChange={(event) => setDraft((current) => ({ ...current, scheduleDay: Number(event.target.value) }))} className="h-11 rounded-xl border border-white/15 bg-white px-3 text-sm text-[#102028] outline-none focus-visible:ring-3 focus-visible:ring-[#b8d5c6]/50">
                  {weekdays.map((day) => <option key={day.value} value={day.value}>{day.label}</option>)}
                </select>
              )}
            </PlanField>
          </div>

          <PlanField label="计划名称（可选）" hint="留空时会使用“标的 + 长期计划”。" className="mt-5">
            <Input aria-label="计划名称（可选）" value={draft.name} onChange={(event) => setDraft((current) => ({ ...current, name: event.target.value }))} placeholder="例如：我的退休储蓄" className="h-11 border-white/15 bg-white text-[#102028]" />
          </PlanField>

          {requestError ? <p role="alert" className="mt-5 rounded-xl border border-red-300/30 bg-red-300/10 px-4 py-3 text-sm text-red-100">计划没有保存成功。请检查输入并确认本机服务正在运行。</p> : null}
          {savedWithoutAdvice ? (
            <div role="status" className="mt-5 rounded-xl border border-amber-200/30 bg-amber-200/10 px-4 py-3 text-sm leading-6 text-amber-50">
              <p>计划已经保存，但本期安排暂时没有准备完成。你可以安全重试，不会重复建立计划。</p>
              <button type="button" onClick={() => void retryAdvice()} disabled={prepareDecision.isPending} className="mt-2 font-medium underline underline-offset-4 disabled:opacity-60">重新准备本期安排</button>
            </div>
          ) : null}

          <Button type="submit" disabled={saving} className="mt-7 h-11 rounded-full bg-white px-5 text-[#102028] hover:bg-[#dcece4]">
            {saving ? <><Loader2 className="animate-spin" />正在建立…</> : <>建立并查看本期安排 <ArrowRight /></>}
          </Button>
        </form>

        <aside className="space-y-5 rounded-[1.35rem] border border-slate-200 bg-white p-6">
          <div><ShieldCheck className="size-5 text-[#2d6a57]" /><h2 className="mt-4 text-xl font-semibold tracking-[-0.03em] text-[#102028]">系统替你固定的边界</h2></div>
          <ul className="space-y-5 text-sm leading-6 text-slate-600">
            <Boundary icon={<CheckCircle2 />} title="固定定投" text="每次建议都使用你设定的金额，不读取市场或 AI 信号。" />
            <Boundary icon={<CircleDollarSign />} title="金额有上限" text="最小版本不会建议超过本次设定金额。" />
            <Boundary icon={<CalendarDays />} title="日期可解释" text="只有约定日期才显示为待执行，其他时间只告诉你继续等待。" />
          </ul>
          <p className="border-t border-slate-100 pt-5 text-xs leading-5 text-slate-400">高级策略、机会资金与券商实验仍保留在原 API 中，但不会进入这条普通用户路径。</p>
        </aside>
      </section>

      <PlanList
        plans={plans.data ?? []}
        pending={plans.isPending}
        selectedPlanId={selectedPlanId}
        busy={update.isPending || remove.isPending}
        onSelect={setSelectedPlanId}
        onToggle={(plan) => update.mutate({ planId: plan.id, input: { is_active: !plan.is_active } })}
        onRemove={(plan) => {
          if (!globalThis.confirm(`删除“${plan.name}”及其本地记录？`)) return
          if (selectedPlanId === plan.id) setSelectedPlanId(null)
          remove.mutate(plan.id)
        }}
      />
    </div>
  )
}

function PlanField({ label, hint, className, children }: { label: string; hint: string; className?: string; children: React.ReactNode }) {
  return <label className={`grid gap-1.5 text-sm font-medium text-slate-100 ${className ?? ''}`}>{label}{children}<span className="text-xs font-normal leading-5 text-slate-400">{hint}</span></label>
}

function Boundary({ icon, title, text }: { icon: React.ReactNode; title: string; text: string }) {
  return <li className="flex gap-3"><span className="mt-0.5 text-[#2d6a57]">{icon}</span><div><p className="font-medium text-[#102028]">{title}</p><p className="mt-1">{text}</p></div></li>
}

function PlanList({
  plans,
  pending,
  selectedPlanId,
  busy,
  onSelect,
  onToggle,
  onRemove,
}: {
  plans: InvestmentPlan[]
  pending: boolean
  selectedPlanId: string | null
  busy: boolean
  onSelect: (id: string) => void
  onToggle: (plan: InvestmentPlan) => void
  onRemove: (plan: InvestmentPlan) => void
}) {
  return (
    <section>
      <div><h2 className="text-xl font-semibold tracking-[-0.03em] text-[#102028]">已有计划</h2><p className="mt-1 text-sm text-slate-500">选择后，个人中心会显示这个计划的本期安排。</p></div>
      {pending ? <p className="mt-5 flex items-center gap-2 text-sm text-slate-500"><Loader2 className="size-4 animate-spin" />正在读取计划…</p> : null}
      {!pending && plans.length === 0 ? <p className="mt-5 rounded-xl border border-dashed border-slate-300 bg-white p-5 text-sm text-slate-600">还没有计划。上面的四个选择就足够开始。</p> : null}
      {!pending && plans.length > 0 ? (
        <div className="mt-5 grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {plans.map((plan) => (
            <article key={plan.id} className={`rounded-[1.2rem] border bg-white p-5 ${selectedPlanId === plan.id ? 'border-[#2d6a57]' : 'border-slate-200'}`}>
              <button type="button" onClick={() => onSelect(plan.id)} className="w-full text-left focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-[#b8d5c6]/50">
                <div className="flex items-start justify-between gap-3"><div><p className="font-semibold text-[#102028]">{plan.name}</p><p className="mt-1 text-sm text-slate-500">{plan.symbol} · 固定定投</p></div><span className={`rounded-full px-2.5 py-1 text-xs ${plan.is_active ? 'bg-[#e6f1eb] text-[#2d6a57]' : 'bg-slate-100 text-slate-500'}`}>{plan.is_active ? '进行中' : '已暂停'}</span></div>
                <p className="mt-5 text-xl font-semibold tracking-[-0.03em] text-[#102028]">{formatMoney(plan.currency, plan.base_contribution)}</p>
                <p className="mt-1 text-xs text-slate-500">{scheduleLabel(plan)}</p>
              </button>
              <div className="mt-5 flex gap-2 border-t border-slate-100 pt-4">
                <Button type="button" variant="outline" size="sm" disabled={busy} onClick={() => onToggle(plan)}>{plan.is_active ? <><Pause />暂停</> : <><Play />继续</>}</Button>
                <Button type="button" variant="ghost" size="sm" disabled={busy} onClick={() => onRemove(plan)} className="text-slate-500 hover:text-red-700"><Trash2 />删除</Button>
              </div>
            </article>
          ))}
        </div>
      ) : null}
    </section>
  )
}

function initialDraft(): MinimalPlanDraft {
  return { name: '', symbol: '', amount: '1000.00', scheduleKind: 'monthly', scheduleDay: defaultScheduleDay('monthly') }
}

function defaultScheduleDay(kind: MinimalPlanDraft['scheduleKind']): number {
  const now = new Date()
  if (kind === 'monthly') return Math.min(now.getUTCDate(), 28)
  return now.getUTCDay() === 0 ? 7 : now.getUTCDay()
}

function fixedDcaPayload(draft: MinimalPlanDraft, symbol: string): CreateInvestmentPlanRequest {
  return {
    name: draft.name.trim() || `${symbol} 长期计划`,
    symbol,
    base_contribution: draft.amount.trim(),
    currency: 'USD',
    schedule_kind: draft.scheduleKind,
    schedule_day: draft.scheduleDay,
    schedule_days: [draft.scheduleDay],
    policy: { id: 'fixed_dca', version: 1 },
    bucket_allocation: { core_ratio: '1.00', opportunity_ratio: '0.00' },
    risk_mode: 'fixed',
    opportunity_cash_policy: 'expire_each_period',
    max_single_execution: draft.amount.trim(),
  }
}

function scheduleLabel(plan: InvestmentPlan): string {
  if (plan.schedule_kind === 'weekly') return `每周 ${weekdays.find((day) => day.value === plan.schedule_day)?.label ?? plan.schedule_day}`
  return `每月 ${plan.schedule_day} 日`
}

function formatMoney(currency: string, amount: string): string {
  const numeric = Number(amount)
  return Number.isFinite(numeric)
    ? new Intl.NumberFormat('zh-CN', { style: 'currency', currency, maximumFractionDigits: 2 }).format(numeric)
    : `${currency} ${amount}`
}
