import {
  ArrowRight,
  CalendarDays,
  Check,
  CheckCircle2,
  CircleDollarSign,
  Clock3,
  Loader2,
  SkipForward,
  Sparkles,
} from 'lucide-react'
import { useState, type FormEvent } from 'react'
import { Link } from 'react-router'
import { useSnapshot } from 'valtio'

import {
  ApiRequestError,
  useAppendManualExecution,
  useDecisionRecords,
  useManualExecutions,
  usePlans,
} from '@/api/queries'
import type {
  DecisionRecord,
  InvestmentPlan,
  ManualExecutionOutcome,
} from '@/api/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ManualExecutionHistory } from '@/components/v2_1/manual-execution-history'
import { PageHeading } from '@/components/v2_1/page-heading'
import { setSelectedPlanId, uiStore } from '@/stores/ui'

type ExecutionDraft = {
  eventId: string
  outcome: ManualExecutionOutcome
  amount: string
  occurredAt: string
  note: string
}

const outcomeCopy: Record<ManualExecutionOutcome, { action: string; confirm: string; history: string }> = {
  executed: { action: '我已执行', confirm: '确认记录完成', history: '已完成' },
  partial: { action: '部分执行', confirm: '确认记录部分执行', history: '部分完成' },
  skipped: { action: '这次跳过', confirm: '确认记录跳过', history: '已跳过' },
}

const actionExplanation: Record<DecisionRecord['decision_snapshot']['action'], string> = {
  overweight: '本期建议在计划边界内多投入一些。执行前仍由你确认实际金额。',
  standard: '本期建议保持原来的金额和节奏，不根据短期涨跌临时改变计划。',
  underweight: '本期建议比常规金额少投入一些，未投入部分仍按你的计划规则处理。',
  tactical_delay: '本期建议暂缓执行。它只是在放慢节奏，不代表对未来涨跌作出预测。',
  skip: '本期规则没有给出投入动作。你可以记录跳过，保留完整执行历史。',
}

/** Real-data personal home for the smallest advice-to-audit loop. */
export default function PersonalPage() {
  const { selectedPlanId } = useSnapshot(uiStore)
  const plans = usePlans()
  const activePlan = selectActivePlan(plans.data ?? [], selectedPlanId)
  const decisions = useDecisionRecords(activePlan?.id ?? null)
  const actionableDecision = decisions.data?.find((decision) => decision.execution_status === 'due') ?? null
  const requestError = plans.error ?? decisions.error

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <div className="flex flex-col gap-5 md:flex-row md:items-end md:justify-between">
        <PageHeading
          eyebrow="个人中心"
          title="今天，只做计划要求的事"
          description="建议、你的确认和执行历史都来自本机记录。这里不替你下单，也不会改写原来的建议。"
        />
        {(plans.data?.length ?? 0) > 1 ? (
          <label className="grid min-w-56 gap-1.5 text-sm font-medium text-[#102028]">
            查看计划
            <select
              aria-label="查看计划"
              className="h-10 rounded-xl border border-slate-200 bg-white px-3 text-sm outline-none focus-visible:border-[#2d6a57] focus-visible:ring-3 focus-visible:ring-[#b8d5c6]/50"
              value={activePlan?.id ?? ''}
              onChange={(event) => setSelectedPlanId(event.target.value)}
            >
              {plans.data?.map((plan) => <option key={plan.id} value={plan.id}>{plan.name}</option>)}
            </select>
          </label>
        ) : null}
      </div>

      {plans.isPending || (activePlan && decisions.isPending) ? <LoadingState /> : null}
      {!plans.isPending && requestError ? <RequestErrorState /> : null}
      {!plans.isPending && !requestError && !activePlan ? <NoPlanState /> : null}
      {!plans.isPending && !requestError && activePlan && !decisions.isPending && !actionableDecision ? (
        <NoDecisionState plan={activePlan} />
      ) : null}
      {activePlan && actionableDecision ? (
        <DecisionExecution key={actionableDecision.id} plan={activePlan} decision={actionableDecision} />
      ) : null}
    </div>
  )
}

function DecisionExecution({ plan, decision }: { plan: InvestmentPlan; decision: DecisionRecord }) {
  const journal = useManualExecutions(decision.id)
  const append = useAppendManualExecution()
  const [draft, setDraft] = useState<ExecutionDraft | null>(null)
  const [formError, setFormError] = useState<string | null>(null)
  const [feedback, setFeedback] = useState<string | null>(null)

  const beginConfirmation = (outcome: ManualExecutionOutcome) => {
    append.reset()
    setFeedback(null)
    setFormError(null)
    setDraft({
      eventId: globalThis.crypto.randomUUID(),
      outcome,
      amount: outcome === 'skipped' ? '' : editableAmount(decision.planned_contribution),
      occurredAt: toLocalDatetimeValue(new Date()),
      note: '',
    })
  }

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!draft) return
    const amount = draft.amount.trim()
    if (draft.outcome !== 'skipped' && !isPositiveDecimal(amount)) {
      setFormError('请输入大于 0 的实际金额。')
      return
    }
    const occurredAt = new Date(draft.occurredAt)
    if (Number.isNaN(occurredAt.getTime())) {
      setFormError('请选择有效的执行时间。')
      return
    }
    setFormError(null)
    try {
      await append.mutateAsync({
        decisionId: decision.id,
        input: {
          event_id: draft.eventId,
          outcome: draft.outcome,
          actual_amount: draft.outcome === 'skipped' ? undefined : amount,
          occurred_at: occurredAt.toISOString(),
          note: draft.note.trim() || undefined,
        },
      })
      setFeedback(`${outcomeCopy[draft.outcome].history}，已加入执行历史。`)
      setDraft(null)
    } catch (error) {
      if (error instanceof ApiRequestError && error.status === 409) {
        await journal.refetch()
        append.reset()
        setFeedback('这次记录已经存在，执行历史已为你刷新。')
        setDraft(null)
      }
    }
  }

  return (
    <>
      <section className="overflow-hidden rounded-[1.6rem] bg-[#102028] text-white">
        <div className="grid gap-8 p-6 sm:p-8 lg:grid-cols-[1.35fr_.65fr] lg:p-10">
          <div>
            <p className="inline-flex items-center gap-2 text-sm text-[#b8d5c6]"><Sparkles className="size-4" />本期建议</p>
            <h2 className="mt-5 max-w-xl text-3xl font-semibold tracking-[-0.045em] sm:text-4xl">
              {decisionTitle(decision)}
            </h2>
            <p className="mt-4 max-w-2xl text-[0.95rem] leading-7 text-slate-300">{actionExplanation[decision.decision_snapshot.action]}</p>
            <p className="mt-3 text-xs leading-5 text-slate-400">这是已保存的建议，不是自动下单。请在券商侧操作后，再回来如实记录结果。</p>

            <div className="mt-7 flex flex-wrap gap-3" aria-label="记录执行结果">
              <button type="button" onClick={() => beginConfirmation('executed')} className="rounded-full bg-white px-4 py-2.5 text-sm font-medium text-[#102028] transition-colors hover:bg-[#dcece4] focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-white/50">我已执行</button>
              <button type="button" onClick={() => beginConfirmation('partial')} className="rounded-full border border-white/25 px-4 py-2.5 text-sm font-medium text-white transition-colors hover:bg-white/10 focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-white/40">部分执行</button>
              <button type="button" onClick={() => beginConfirmation('skipped')} className="rounded-full px-4 py-2.5 text-sm font-medium text-slate-300 transition-colors hover:bg-white/10 hover:text-white focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-white/40">这次跳过</button>
              <Link to={`/decisions/${decision.id}`} className="inline-flex items-center gap-1 rounded-full px-3 py-2.5 text-sm font-medium text-[#b8d5c6] hover:text-white">查看原建议 <ArrowRight className="size-3.5" /></Link>
            </div>

            {draft ? (
              <ConfirmationForm
                draft={draft}
                currency={decision.currency}
                pending={append.isPending}
                error={formError ?? mutationMessage(append.error)}
                onChange={setDraft}
                onCancel={() => { append.reset(); setDraft(null); setFormError(null) }}
                onSubmit={submit}
              />
            ) : null}
            {feedback ? <p role="status" className="mt-5 rounded-xl border border-[#8eb7a3]/40 bg-[#dcece4]/10 px-4 py-3 text-sm text-[#dcece4]">{feedback}</p> : null}
          </div>

          <div className="grid content-end gap-3 rounded-[1.2rem] border border-white/10 bg-white/5 p-5">
            <div><p className="text-sm text-slate-300">正在执行</p><p className="mt-1 text-xl font-medium">{plan.name}</p></div>
            <div className="h-px bg-white/10" />
            <div><p className="text-sm text-slate-300">投资标的</p><p className="mt-1 font-medium">{plan.symbol}</p></div>
            <div><p className="text-sm text-slate-300">建议生成于</p><p className="mt-1 font-medium">{formatDateTime(decision.created_at)}</p></div>
          </div>
        </div>
      </section>

      <section className="grid gap-7 lg:grid-cols-[minmax(0,1fr)_22rem]">
        <div className="rounded-[1.35rem] border border-slate-200 bg-white p-5 sm:p-6">
          <div className="flex items-start justify-between gap-4">
            <div>
              <h2 className="text-xl font-semibold tracking-[-0.03em] text-[#102028]">你的计划</h2>
              <p className="mt-1 text-sm text-slate-500">这里显示真实计划配置，不用盯住行情。</p>
            </div>
            <Link to="/plans" className="text-sm font-medium text-[#2d6a57]">管理计划</Link>
          </div>
          <div className="mt-6 grid gap-4 sm:grid-cols-3">
            <PlanFact icon={<CircleDollarSign />} label="常规金额" value={formatMoney(plan.currency, plan.base_contribution)} />
            <PlanFact icon={<CalendarDays />} label="执行节奏" value={scheduleLabel(plan)} />
            <PlanFact icon={<CheckCircle2 />} label="当前状态" value={plan.is_active ? '正在坚持' : '已暂停'} />
          </div>
          <div className="mt-6 rounded-xl bg-[#f4f7f6] px-4 py-3 text-sm leading-6 text-slate-600">
            原建议会一直保持不变。每次确认只会在它后面新增一条由你报告的事实记录。
          </div>
        </div>

        <ManualExecutionHistory events={journal.data ?? []} pending={journal.isPending} error={journal.error} onRetry={() => void journal.refetch()} />
      </section>
    </>
  )
}

function ConfirmationForm({
  draft,
  currency,
  pending,
  error,
  onChange,
  onCancel,
  onSubmit,
}: {
  draft: ExecutionDraft
  currency: string
  pending: boolean
  error: string | null
  onChange: (draft: ExecutionDraft) => void
  onCancel: () => void
  onSubmit: (event: FormEvent<HTMLFormElement>) => void
}) {
  return (
    <form onSubmit={onSubmit} className="mt-5 max-w-2xl rounded-2xl border border-white/15 bg-white/[0.07] p-4 sm:p-5">
      <div className="flex items-center justify-between gap-4">
        <div><p className="font-medium">{outcomeCopy[draft.outcome].action}</p><p className="mt-1 text-xs text-slate-400">确认后会新增一条记录，历史不能编辑。</p></div>
        {draft.outcome === 'executed' ? <Check className="size-5 text-[#b8d5c6]" /> : draft.outcome === 'partial' ? <Clock3 className="size-5 text-[#b8d5c6]" /> : <SkipForward className="size-5 text-[#b8d5c6]" />}
      </div>
      <div className="mt-4 grid gap-4 sm:grid-cols-2">
        {draft.outcome !== 'skipped' ? (
          <label className="grid gap-1.5 text-sm text-slate-200">
            实际金额（{currency}）
            <Input
              aria-label="实际金额"
              inputMode="decimal"
              value={draft.amount}
              onChange={(event) => onChange({ ...draft, amount: event.target.value })}
              className="h-10 border-white/15 bg-white text-[#102028]"
            />
          </label>
        ) : null}
        <label className="grid gap-1.5 text-sm text-slate-200">
          实际时间
          <Input
            aria-label="实际时间"
            type="datetime-local"
            value={draft.occurredAt}
            onChange={(event) => onChange({ ...draft, occurredAt: event.target.value })}
            className="h-10 border-white/15 bg-white text-[#102028]"
          />
        </label>
      </div>
      <label className="mt-4 grid gap-1.5 text-sm text-slate-200">
        备注（可选）
        <textarea
          aria-label="备注（可选）"
          maxLength={160}
          rows={2}
          value={draft.note}
          onChange={(event) => onChange({ ...draft, note: event.target.value })}
          placeholder={draft.outcome === 'skipped' ? '例如：本月现金安排有变化' : '例如：已在常用券商完成'}
          className="resize-none rounded-xl border border-white/15 bg-white px-3 py-2 text-sm text-[#102028] outline-none placeholder:text-slate-400 focus-visible:ring-3 focus-visible:ring-[#b8d5c6]/50"
        />
      </label>
      {error ? <p role="alert" className="mt-3 text-sm text-[#ffd3ce]">{error}</p> : null}
      <div className="mt-4 flex flex-wrap gap-2">
        <Button type="submit" disabled={pending} className="h-10 rounded-full bg-white px-4 text-[#102028] hover:bg-[#dcece4]">
          {pending ? <><Loader2 className="animate-spin" />正在记录…</> : outcomeCopy[draft.outcome].confirm}
        </Button>
        <Button type="button" variant="ghost" disabled={pending} onClick={onCancel} className="h-10 rounded-full px-4 text-slate-300 hover:bg-white/10 hover:text-white">取消</Button>
      </div>
    </form>
  )
}

function LoadingState() {
  return <section className="grid min-h-72 place-items-center rounded-[1.6rem] bg-[#102028] text-sm text-slate-300"><p className="flex items-center gap-2"><Loader2 className="size-4 animate-spin" />正在读取你的计划与本期建议…</p></section>
}

function RequestErrorState() {
  return <section role="alert" className="rounded-[1.6rem] border border-red-200 bg-red-50 p-7"><h2 className="text-xl font-semibold text-red-900">本机服务暂时没有响应</h2><p className="mt-2 text-sm leading-6 text-red-700">确认 Rust 服务已经启动后刷新页面。你的计划和历史记录不会因此被改动。</p></section>
}

function NoPlanState() {
  return <section className="rounded-[1.6rem] border border-dashed border-slate-300 bg-white p-8"><h2 className="text-2xl font-semibold tracking-[-0.035em] text-[#102028]">先建立第一个长期计划</h2><p className="mt-3 max-w-xl text-sm leading-7 text-slate-600">设置标的、常规金额和执行节奏后，本期建议才会出现在这里。</p><Link to="/plans" className="mt-5 inline-flex items-center gap-1 rounded-full bg-[#102028] px-4 py-2.5 text-sm font-medium text-white">建立计划 <ArrowRight className="size-3.5" /></Link></section>
}

function NoDecisionState({ plan }: { plan: InvestmentPlan }) {
  const nextDate = nextScheduledDate(plan)
  return <section className="rounded-[1.6rem] bg-[#102028] p-8 text-white"><p className="text-sm text-[#b8d5c6]">{plan.name}</p><h2 className="mt-4 text-3xl font-semibold tracking-[-0.04em]">{plan.is_active ? '现在只需要继续等待' : '这个计划已经暂停'}</h2><p className="mt-3 max-w-xl text-sm leading-7 text-slate-300">{plan.is_active ? `计划按${scheduleLabel(plan)}执行，下一次计划日是 ${nextDate}。只有到计划日，真实建议才会出现在这里。` : '暂停期间不会产生新的待执行建议；继续计划后，系统会恢复原来的固定节奏。'}</p><Link to="/decisions" className="mt-5 inline-flex items-center gap-1 text-sm font-medium text-[#b8d5c6]">查看全部建议 <ArrowRight className="size-3.5" /></Link></section>
}

function PlanFact({ icon, label, value }: { icon: React.ReactNode; label: string; value: string }) {
  return <div className="rounded-xl border border-slate-100 p-4"><div className="flex items-center gap-2 text-sm text-slate-500">{icon}{label}</div><p className="mt-3 font-medium text-[#102028]">{value}</p></div>
}

function selectActivePlan(plans: InvestmentPlan[], selectedPlanId: string | null): InvestmentPlan | null {
  return plans.find((plan) => plan.id === selectedPlanId) ?? plans.find((plan) => plan.is_active) ?? plans[0] ?? null
}

function decisionTitle(decision: DecisionRecord): string {
  if (decision.decision_snapshot.action === 'skip') return '本期不需要投入'
  if (decision.decision_snapshot.action === 'tactical_delay') return '本期先等等'
  return decision.planned_contribution
    ? `按建议投入 ${formatMoney(decision.currency, decision.planned_contribution)}`
    : '查看本期建议'
}

function scheduleLabel(plan: InvestmentPlan): string {
  if (plan.schedule_kind === 'weekly') return `每周 ${plan.schedule_days.join('、')}`
  return `每月 ${plan.schedule_days.join('、')} 日`
}

function nextScheduledDate(plan: InvestmentPlan, now = new Date()): string {
  const today = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate())
  const candidates: number[] = []
  if (plan.schedule_kind === 'weekly') {
    const todayWeekday = now.getUTCDay() === 0 ? 7 : now.getUTCDay()
    for (const weekday of plan.schedule_days) {
      candidates.push(today + ((weekday - todayWeekday + 7) % 7) * 86_400_000)
    }
  } else {
    for (const monthOffset of [0, 1]) {
      const month = now.getUTCMonth() + monthOffset
      for (const day of plan.schedule_days) {
        const candidate = Date.UTC(now.getUTCFullYear(), month, day)
        if (candidate >= today) candidates.push(candidate)
      }
    }
  }
  const next = new Date(Math.min(...candidates))
  if (Number.isNaN(next.getTime())) return '下一次约定日期'
  return new Intl.DateTimeFormat('zh-CN', { month: 'long', day: 'numeric', timeZone: 'UTC' }).format(next)
}

function formatMoney(currency: string, amount: string): string {
  const numeric = Number(amount)
  if (!Number.isFinite(numeric)) return `${currency} ${amount}`
  try {
    return new Intl.NumberFormat('zh-CN', { style: 'currency', currency, maximumFractionDigits: 2 }).format(numeric)
  } catch {
    return `${currency} ${numeric.toLocaleString('zh-CN', { maximumFractionDigits: 2 })}`
  }
}

function editableAmount(value?: string): string {
  if (!value) return ''
  const numeric = Number(value)
  return Number.isFinite(numeric) ? numeric.toFixed(2) : value
}

function formatDateTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return new Intl.DateTimeFormat('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }).format(date)
}

function toLocalDatetimeValue(date: Date): string {
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000)
  return local.toISOString().slice(0, 16)
}

function isPositiveDecimal(value: string): boolean {
  return /^\d+(?:\.\d+)?$/.test(value) && Number(value) > 0
}

function mutationMessage(error: Error | null): string | null {
  if (!error) return null
  if (error instanceof ApiRequestError && error.status === 400) return '这条记录没有通过校验，请检查金额和时间。'
  return '暂时没有记录成功。请保留当前内容并重试。'
}
