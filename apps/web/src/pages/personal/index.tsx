import {
  ArrowRight,
  Bot,
  CalendarDays,
  Check,
  CheckCircle2,
  CircleAlert,
  CircleDollarSign,
  Loader2,
  SkipForward,
  Sparkles,
} from 'lucide-react'
import { useState, type FormEvent } from 'react'
import { Link } from 'react-router'
import { useSnapshot } from 'valtio'

import {
  ApiRequestError,
  describeAiActionError,
  useAppendManualExecution,
  useAiProviders,
  useDecisionRecords,
  useManualExecutions,
  usePlans,
  usePersonalAiSummary,
  useStrategyCatalog,
} from '@/api/queries'
import type {
  DecisionRecord,
  InvestmentPlan,
  ManualExecutionOutcome,
  StrategyCatalogEntry,
} from '@/api/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ManualExecutionHistory } from '@/components/v2_1/manual-execution-history'
import { PageHeading } from '@/components/v2_1/page-heading'
import { setSelectedPlanId, uiStore } from '@/stores/ui'

type ExecutionDraft = {
  eventId: string
  outcome: ConfirmableOutcome
  amount: string
  occurredAt: string
  note: string
}

type ConfirmableOutcome = Extract<ManualExecutionOutcome, 'executed' | 'skipped'>
type AdviceCardState = 'checking' | 'pending' | 'completed' | 'skipped' | 'partial' | 'unavailable'

const outcomeCopy: Record<ConfirmableOutcome, { action: string; confirm: string; history: string }> = {
  executed: { action: '我已执行', confirm: '确认记录完成', history: '已完成' },
  skipped: { action: '这次跳过', confirm: '确认记录跳过', history: '已跳过' },
}

const adviceCardVisuals: Record<AdviceCardState, { card: string; badge: string; label: string }> = {
  checking: {
    card: 'border-white/10 bg-[#102028]',
    badge: 'border-white/15 bg-white/[0.06] text-slate-300',
    label: '正在确认状态',
  },
  pending: {
    card: 'border-[#9a7847] bg-[#352b1d] shadow-[0_18px_45px_rgba(112,78,28,0.16)]',
    badge: 'border-[#c09a5b]/50 bg-[#e6bd75]/12 text-[#f1d7a6]',
    label: '本期待办 · 未完成',
  },
  completed: {
    card: 'border-[#527765] bg-[#173027] shadow-[0_18px_45px_rgba(31,86,65,0.14)]',
    badge: 'border-[#8eb7a3]/40 bg-[#b8d5c6]/10 text-[#dcece4]',
    label: '本期待办 · 已完成',
  },
  skipped: {
    card: 'border-slate-500/35 bg-[#17272d]',
    badge: 'border-slate-300/20 bg-white/[0.06] text-slate-300',
    label: '本期待办 · 已跳过',
  },
  partial: {
    card: 'border-slate-500/35 bg-[#17272d]',
    badge: 'border-slate-300/20 bg-white/[0.06] text-slate-300',
    label: '本期待办 · 历史部分完成',
  },
  unavailable: {
    card: 'border-[#766448] bg-[#20262a]',
    badge: 'border-[#a68b5d]/35 bg-[#d9bd88]/10 text-[#ead8b7]',
    label: '执行状态待确认',
  },
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
  const catalog = useStrategyCatalog()
  const activePlan = selectActivePlan(plans.data ?? [], selectedPlanId)
  const activeStrategy = activePlan
    ? catalog.data?.find((strategy) => strategy.policy.id === activePlan.policy.id && strategy.policy.version === activePlan.policy.version)
    : undefined
  const decisions = useDecisionRecords(activePlan?.id ?? null)
  const actionableDecision = decisions.data?.find((decision) => decision.execution_status === 'due') ?? null
  const requestError = plans.error ?? decisions.error

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <PageHeading
        eyebrow="个人中心"
        title="今天，只做计划要求的事"
        description="建议、你的确认和执行历史都来自本机记录。这里不替你下单，也不会改写原来的建议。"
      />

      {activePlan ? (
        <section aria-label="当前查看计划" className="rounded-[1.2rem] border border-[#b8d5c6] bg-white px-5 py-4 shadow-[0_10px_30px_rgba(16,32,40,0.05)]">
          <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-4">
            <p className="shrink-0 text-lg font-semibold tracking-[-0.025em] text-[#2d6a57]">正在查看</p>
            {(plans.data?.length ?? 0) > 1 ? (
              <label className="grid min-w-64 flex-1 gap-1.5 text-sm font-medium text-[#102028]">
                <span className="sr-only">正在查看的计划</span>
                <select
                  aria-label="正在查看的计划"
                  className="h-12 rounded-xl border border-slate-200 bg-[#f4f7f6] px-4 text-lg font-semibold tracking-[-0.02em] text-[#102028] outline-none focus-visible:border-[#2d6a57] focus-visible:ring-3 focus-visible:ring-[#b8d5c6]/50"
                  value={activePlan?.id ?? ''}
                  onChange={(event) => setSelectedPlanId(event.target.value)}
                >
                  {plans.data?.map((plan) => <option key={plan.id} value={plan.id}>{plan.name}</option>)}
                </select>
              </label>
            ) : <p className="text-lg font-semibold tracking-[-0.02em] text-[#102028]">{activePlan.name}</p>}
          </div>
          <p className="mt-2 text-sm text-slate-500 sm:ml-[6.8rem]">个人中心只显示这份计划的当前建议和执行记录。</p>
        </section>
      ) : null}

      <PersonalAiSummary />

      {plans.isPending || (activePlan && decisions.isPending) ? <LoadingState /> : null}
      {!plans.isPending && requestError ? <RequestErrorState /> : null}
      {!plans.isPending && !requestError && !activePlan ? <NoPlanState /> : null}
      {!plans.isPending && !requestError && activePlan && !decisions.isPending && !actionableDecision ? (
        <div className="space-y-7">
          <NoDecisionState plan={activePlan} />
          <PlanStrategySummary plan={activePlan} strategy={activeStrategy} catalogPending={catalog.isPending} />
        </div>
      ) : null}
      {activePlan && actionableDecision ? (
        <DecisionExecution key={actionableDecision.id} plan={activePlan} decision={actionableDecision} strategy={activeStrategy} catalogPending={catalog.isPending} />
      ) : null}
    </div>
  )
}

function PersonalAiSummary() {
  const providers = useAiProviders()
  const summary = usePersonalAiSummary()
  const [profileId, setProfileId] = useState('')
  const available = (providers.data?.providers ?? []).filter((provider) => provider.capabilities.read_only_explanations)
  const effectiveProfileId = profileId || available[0]?.id || ''
  return <section className="rounded-[1.2rem] border border-slate-200 bg-white px-5 py-4" aria-labelledby="personal-ai-title"><div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between"><div className="flex min-w-0 items-start gap-3"><span className="grid size-9 shrink-0 place-items-center rounded-xl bg-[#e8f1ed] text-[#2d6a57]"><Bot className="size-4" /></span><div><h2 id="personal-ai-title" className="font-semibold text-[#102028]">帮我整理近期计划</h2><p className="mt-1 text-sm leading-6 text-slate-500">可选的小摘要。只有点击后才会把本机计划和最近 20 条建议状态发送给你选的 API；不会读取新闻、生成交易或自动运行。</p></div></div>{available.length > 0 ? <div className="flex shrink-0 flex-col gap-2 sm:flex-row sm:items-end"><label className="grid gap-1 text-xs font-medium text-slate-500">摘要模型<select aria-label="个人摘要模型" value={effectiveProfileId} onChange={(event) => setProfileId(event.target.value)} className="h-10 min-w-48 rounded-xl border border-slate-200 bg-[#f7f9f8] px-3 text-sm text-[#102028]">{available.map((provider) => <option key={provider.id} value={provider.id}>{provider.display_name}</option>)}</select></label><Button type="button" disabled={summary.isPending} onClick={() => { summary.reset(); summary.mutate(effectiveProfileId) }} className="h-10 rounded-full bg-[#102830] px-4">{summary.isPending ? <><Loader2 className="animate-spin" />正在整理…</> : '手动生成摘要'}</Button></div> : <p className="shrink-0 text-xs text-slate-400">未配置解释型 AI</p>}</div>{summary.isError ? <p role="alert" className="mt-4 rounded-xl border border-[#dec9a6] bg-[#fff8eb] px-4 py-3 text-sm text-[#73572f]">{describeAiActionError(summary.error, '这次摘要没有生成。个人中心原有数据不受影响，请稍后重试。')}</p> : null}{summary.data ? <article className="mt-4 rounded-xl bg-[#f1f7f4] p-4"><div className="flex flex-wrap items-center justify-between gap-2"><h3 className="font-semibold text-[#102028]">{summary.data.explanation.headline}</h3><span className="text-xs text-slate-500">{summary.data.provider.display_name} · {summary.data.plan_count} 个计划 / {summary.data.decision_count} 条记录</span></div><p className="mt-2 text-sm leading-7 text-slate-700">{summary.data.explanation.summary}</p>{summary.data.explanation.observations.length > 0 ? <ul className="mt-3 list-disc space-y-1 pl-5 text-sm leading-6 text-slate-600">{summary.data.explanation.observations.map((item) => <li key={item}>{item}</li>)}</ul> : null}{summary.data.explanation.risks.length > 0 ? <p className="mt-3 text-xs leading-5 text-[#7a5a2b]">注意：{summary.data.explanation.risks.join('；')}</p> : null}<p className="mt-3 text-xs text-slate-400">摘要不落库，不会改变任何计划、建议或执行记录。</p></article> : null}</section>
}

function DecisionExecution({ plan, decision, strategy, catalogPending }: { plan: InvestmentPlan; decision: DecisionRecord; strategy?: StrategyCatalogEntry; catalogPending: boolean }) {
  const journal = useManualExecutions(decision.id)
  const append = useAppendManualExecution()
  const [draft, setDraft] = useState<ExecutionDraft | null>(null)
  const [formError, setFormError] = useState<string | null>(null)
  const [feedback, setFeedback] = useState<string | null>(null)
  const recordedOutcome = journal.data?.[0] ?? null
  const cardState = adviceCardState(journal.isPending, Boolean(journal.error), recordedOutcome?.outcome)
  const cardVisual = adviceCardVisuals[cardState]

  const beginConfirmation = (outcome: ConfirmableOutcome) => {
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
      <section
        data-advice-state={cardState}
        className={`overflow-hidden rounded-[1.6rem] border text-white transition-[background-color,border-color,box-shadow] duration-700 ease-out motion-reduce:transition-none ${cardVisual.card}`}
      >
        <div className="grid gap-8 p-6 sm:p-8 lg:grid-cols-[1.35fr_.65fr] lg:p-10">
          <div>
            <div className="flex flex-wrap items-center justify-between gap-3">
              <p className="inline-flex items-center gap-2 text-sm text-[#b8d5c6]"><Sparkles className="size-4" />本期建议</p>
              <span aria-label="本期办理状态" className={`inline-flex items-center gap-2 rounded-full border px-3 py-1.5 text-xs font-medium ${cardVisual.badge}`}>
                <AdviceStatusIcon state={cardState} />
                {cardVisual.label}
              </span>
            </div>
            <h2 className="mt-5 max-w-xl text-3xl font-semibold tracking-[-0.045em] sm:text-4xl">
              {decisionTitle(decision)}
            </h2>
            <p className="mt-4 max-w-2xl text-[0.95rem] leading-7 text-slate-300">{actionExplanation[decision.decision_snapshot.action]}</p>
            <p className="mt-3 text-xs leading-5 text-slate-400">这是已保存的建议，不是自动下单。请在券商侧操作后，再回来如实记录结果。</p>

            {journal.isPending ? <p className="mt-7 text-sm text-slate-400">正在确认本次建议是否已经记录…</p> : null}
            {!journal.isPending && journal.error ? <p className="mt-7 rounded-xl border border-amber-200/20 bg-amber-100/10 px-4 py-3 text-sm text-amber-50">暂时无法确认执行状态，因此不会开放重复记录。请在执行历史中重新读取。</p> : null}
            {!journal.isPending && !journal.error && recordedOutcome ? (
              <div className="mt-7 rounded-xl border border-[#b8d5c6]/35 bg-[#dcece4]/10 px-4 py-4" role="status">
                <p className="font-medium text-white">本次建议已记录为“{outcomeLabel(recordedOutcome.outcome)}”</p>
                <p className="mt-1 text-sm leading-6 text-slate-300">每个计划日只能确认一次，原记录不会被覆盖，也不能再次追加。</p>
              </div>
            ) : null}
            {!journal.isPending && !journal.error && !recordedOutcome ? (
              <div className="mt-7">
                <p className="mb-3 text-xs leading-5 text-slate-400">当前是这份计划的执行日。本次建议只能确认一次。</p>
                <div className="flex flex-wrap gap-3" aria-label="记录执行结果">
                  <button type="button" onClick={() => beginConfirmation('executed')} className="rounded-full bg-white px-4 py-2.5 text-sm font-medium text-[#102028] transition-colors hover:bg-[#dcece4] focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-white/50">我已执行</button>
                  <button type="button" onClick={() => beginConfirmation('skipped')} className="rounded-full px-4 py-2.5 text-sm font-medium text-slate-300 transition-colors hover:bg-white/10 hover:text-white focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-white/40">这次跳过</button>
                </div>
              </div>
            ) : null}
            <Link to={`/decisions/${decision.id}`} className="mt-4 inline-flex items-center gap-1 rounded-full text-sm font-medium text-[#b8d5c6] hover:text-white">查看原建议 <ArrowRight className="size-3.5" /></Link>

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
        <PlanStrategySummary plan={plan} strategy={strategy} catalogPending={catalogPending} />

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
        {draft.outcome === 'executed' ? <Check className="size-5 text-[#b8d5c6]" /> : <SkipForward className="size-5 text-[#b8d5c6]" />}
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

function PlanStrategySummary({ plan, strategy, catalogPending }: { plan: InvestmentPlan; strategy?: StrategyCatalogEntry; catalogPending: boolean }) {
  const fixedAmount = plan.policy.id === 'fixed_dca'
  const nextDate = nextScheduledDate(plan)
  const baseAmount = formatMoney(plan.currency, plan.base_contribution)
  const coreAmount = multiplyMoney(plan.currency, plan.base_contribution, plan.execution_configuration.bucket_allocation.core_ratio)
  const flexibleAmount = multiplyMoney(plan.currency, plan.base_contribution, plan.execution_configuration.bucket_allocation.opportunity_ratio)
  const maximumAmount = formatMoney(plan.currency, plan.max_single_execution)
  const name = strategy?.name ?? strategyLabel(plan)
  const summary = strategy?.summary ?? fallbackStrategySummary(plan)
  const rule = strategy?.rule ?? fallbackStrategyRule(plan)

  return (
    <section className="overflow-hidden rounded-[1.35rem] border border-slate-200 bg-white">
      <div className="p-5 sm:p-6">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <p className="text-sm font-medium text-[#2d6a57]">计划方法</p>
            <h2 className="mt-2 text-xl font-semibold tracking-[-0.03em] text-[#102028]">{catalogPending && !strategy ? '正在读取策略方法…' : name}</h2>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-slate-600">{summary}</p>
          </div>
          <div className="flex shrink-0 flex-wrap gap-2">
            <span className={`w-fit rounded-full px-3 py-1.5 text-xs font-medium ${plan.is_active ? 'bg-[#e6f1eb] text-[#2d6a57]' : 'bg-slate-100 text-slate-500'}`}>{plan.is_active ? '正在坚持' : '已暂停'}</span>
            <span className={`w-fit rounded-full px-3 py-1.5 text-xs font-medium ${fixedAmount ? 'bg-[#e8f1f4] text-[#355468]' : 'bg-[#e6f1eb] text-[#2d6a57]'}`}>
              {fixedAmount ? '固定金额' : '评估日按规则调整'}
            </span>
          </div>
        </div>

        <div className="mt-6 grid gap-4 sm:grid-cols-3">
          <PlanFact icon={<CalendarDays />} label="本期 / 下一评估" value={nextDate} />
          <PlanFact icon={<CircleDollarSign />} label={fixedAmount ? '每期投入' : '每期基础预算'} value={baseAmount} />
          <PlanFact icon={<CheckCircle2 />} label="评估节奏" value={scheduleLabel(plan)} />
        </div>

        <div className="mt-6 grid gap-4 rounded-[1.05rem] bg-[#f4f7f6] p-4 sm:grid-cols-[minmax(0,1.2fr)_minmax(16rem,.8fr)] sm:p-5">
          <div>
            <p className="text-xs font-medium uppercase tracking-[0.12em] text-[#2d6a57]">方法如何做决定</p>
            <p className="mt-2 text-sm font-medium leading-6 text-[#102028]">{rule}</p>
            {strategy?.limitation ? <p className="mt-2 text-xs leading-5 text-slate-500">需要知道：{strategy.limitation}</p> : null}
          </div>
          <div className="border-t border-slate-200 pt-4 sm:border-l sm:border-t-0 sm:pl-5 sm:pt-0">
            <p className="text-xs font-medium uppercase tracking-[0.12em] text-slate-400">当期金额怎么确定</p>
            {fixedAmount ? (
              <p className="mt-2 text-sm leading-6 text-slate-600">到 {nextDate} 按 <strong className="font-semibold text-[#102028]">{baseAmount}</strong> 生成建议，不读取行情指标。</p>
            ) : (
              <p className="mt-2 text-sm leading-6 text-slate-600">到 {nextDate} 先保留 <strong className="font-semibold text-[#102028]">{coreAmount}</strong> 核心投入，再用当日规则判断最多 <strong className="font-semibold text-[#102028]">{flexibleAmount}</strong> 弹性额度。当期总建议不会超过 {maximumAmount}。</p>
            )}
          </div>
        </div>

        <p className="mt-4 text-xs leading-5 text-slate-400">计划日只生成建议，不会自动下单；每期结果只能由你确认一次。
        </p>
      </div>
    </section>
  )
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

function outcomeLabel(outcome: ManualExecutionOutcome): string {
  if (outcome === 'executed') return '已完成'
  if (outcome === 'skipped') return '已跳过'
  return '部分完成（历史记录）'
}

function adviceCardState(
  pending: boolean,
  unavailable: boolean,
  outcome?: ManualExecutionOutcome,
): AdviceCardState {
  if (pending) return 'checking'
  if (unavailable) return 'unavailable'
  if (outcome === 'executed') return 'completed'
  if (outcome === 'skipped') return 'skipped'
  if (outcome === 'partial') return 'partial'
  return 'pending'
}

function AdviceStatusIcon({ state }: { state: AdviceCardState }) {
  if (state === 'checking') return <Loader2 className="size-3.5 animate-spin motion-reduce:animate-none" />
  if (state === 'completed') return <CheckCircle2 className="size-3.5" />
  if (state === 'skipped' || state === 'partial') return <SkipForward className="size-3.5" />
  return <CircleAlert className="size-3.5" />
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

function strategyLabel(plan: InvestmentPlan): string {
  if (plan.policy.id === 'fixed_dca') return '每月稳步投入'
  if (plan.policy.id === 'dsl_ma200_trend_guard') return '200 日均线趋势保护'
  if (plan.policy.id === 'dsl_growth_volatility_balance') return '增长与波动平衡'
  if (plan.policy.id === 'core_opportunity_v1') return '旧自适应策略'
  return '自定义策略'
}

function fallbackStrategySummary(plan: InvestmentPlan): string {
  if (plan.policy.id === 'fixed_dca') return '在约定日期按固定金额生成投入建议，不根据短期涨跌改变计划。'
  return '这份计划保留固定核心投入，并在计划日使用已冻结的策略版本计算弹性额度。'
}

function fallbackStrategyRule(plan: InvestmentPlan): string {
  if (plan.policy.id === 'fixed_dca') return '每个计划日使用同一份金额，不读取市场或 AI 信号。'
  return '只调整机会桶，不改写核心投入和单次金额上限。'
}

function multiplyMoney(currency: string, amount: string, ratio: string): string {
  const numericAmount = Number(amount)
  const numericRatio = Number(ratio)
  if (!Number.isFinite(numericAmount) || !Number.isFinite(numericRatio)) return '按计划比例计算'
  return formatMoney(currency, String(numericAmount * numericRatio))
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
