import {
  AlertTriangle,
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
import { Dialog as DialogPrimitive } from 'radix-ui'
import { useEffect, useRef, useState, type FormEvent } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { useSnapshot } from 'valtio'

import {
  ApiRequestError,
  useCreatePlan,
  useDeletePlan,
  usePlans,
  usePreviewAutomaticDecision,
  useStrategyCatalog,
  useUpdatePlan,
} from '@/api/queries'
import type { CreateInvestmentPlanRequest, InvestmentPlan, StrategyCatalogEntry } from '@/api/types'
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

const formulaWeekdays = weekdays.filter((day) => day.value <= 5)

/** Consumer plan setup driven by the server-owned, adoptable strategy catalog. */
export default function PlansPage() {
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()
  const { selectedPlanId } = useSnapshot(uiStore)
  const plans = usePlans()
  const catalog = useStrategyCatalog()
  const create = useCreatePlan()
  const prepareDecision = usePreviewAutomaticDecision()
  const update = useUpdatePlan()
  const remove = useDeletePlan()
  const [draft, setDraft] = useState<MinimalPlanDraft>(initialDraft)
  const [savedWithoutAdvice, setSavedWithoutAdvice] = useState<InvestmentPlan | null>(null)
  const [selectionError, setSelectionError] = useState<string | null>(null)
  const [planPendingDelete, setPlanPendingDelete] = useState<InvestmentPlan | null>(null)
  const defaultsAppliedFor = useRef<string | null>(null)
  const requestedPolicyId = searchParams.get('policy_id')
  const requestedVersion = Number(searchParams.get('policy_version'))
  const hasRequestedPolicy = requestedPolicyId !== null
  const selectedPolicyKey = (
    hasRequestedPolicy && Number.isInteger(requestedVersion)
      ? policyKey({ id: requestedPolicyId, version: requestedVersion })
      : null
  )
  const selectedStrategy = catalog.data?.find((strategy) => policyKey(strategy.policy) === selectedPolicyKey)
  const requestedStrategyUnavailable = hasRequestedPolicy && !catalog.isPending && !catalog.isError && (!selectedStrategy || !selectedStrategy.adoptable)
  const isFormula = selectedStrategy?.policy.id !== 'fixed_dca'
  const instrument = parseInstrument(draft.symbol)
  const currency = instrument?.currency ?? 'USD'
  const cadenceLabel = isFormula ? '评估节奏' : '投入节奏'

  useEffect(() => {
    if (!selectedStrategy) return
    const key = policyKey(selectedStrategy.policy)
    if (defaultsAppliedFor.current === key) return
    defaultsAppliedFor.current = key
    setDraft((current) => ({
      ...current,
      scheduleKind: selectedStrategy.default_plan.schedule_kind,
      scheduleDay: defaultScheduleDay(selectedStrategy.default_plan.schedule_kind, selectedStrategy),
    }))
  }, [selectedStrategy])

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    create.reset()
    prepareDecision.reset()
    setSavedWithoutAdvice(null)
    setSelectionError(null)
    const symbol = draft.symbol.trim().toUpperCase()
    if (catalog.isPending || catalog.isError || requestedStrategyUnavailable || !selectedStrategy || !selectedStrategy.adoptable) {
      setSelectionError('请先选择一份已通过准入的官方策略。')
      return
    }
    const parsedInstrument = parseInstrument(symbol)
    if (!parsedInstrument) {
      setSelectionError('无法识别这个标的。请使用 US.AAPL、HK.00700、SH.600519、SZ.000001，或输入无前缀美股代码。')
      return
    }
    if (selectedStrategy.supported_markets?.length > 0 && !selectedStrategy.supported_markets.includes(parsedInstrument.market)) {
      setSelectionError(`当前策略支持 ${marketAvailabilityText(selectedStrategy)}，暂不支持${parsedInstrument.marketLabel}。`)
      return
    }
    const payload = planPayload(draft, symbol, parsedInstrument.currency, selectedStrategy)
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
        eyebrow="我的计划"
        title="所有长期计划，都在这里"
        description="查看正在执行的标的、金额和节奏，或在需要时暂停、继续和删除。新计划统一从策略中心选择。"
      />

      <PlanList
        plans={plans.data ?? []}
        strategies={catalog.data ?? []}
        pending={plans.isPending}
        selectedPlanId={selectedPlanId}
        busy={update.isPending || remove.isPending}
        onSelect={setSelectedPlanId}
        onToggle={(plan) => update.mutate({ planId: plan.id, input: { is_active: !plan.is_active } })}
        onRemove={setPlanPendingDelete}
      />

      <DeletePlanDialog
        plan={planPendingDelete}
        pending={remove.isPending}
        onCancel={() => setPlanPendingDelete(null)}
        onConfirm={(plan) => remove.mutate(plan.id, {
          onSuccess: () => {
            if (selectedPlanId === plan.id) setSelectedPlanId(null)
            setPlanPendingDelete(null)
          },
        })}
      />

      {hasRequestedPolicy ? <section id="new-plan" className="scroll-mt-24">
        <div><p className="text-sm font-medium text-[#2d6a57]">建立新计划</p><h2 className="mt-2 text-2xl font-semibold tracking-[-0.035em] text-[#102028]">{selectedStrategy ? `建立“${selectedStrategy.name}”计划` : hasRequestedPolicy ? '正在确认策略版本' : '先选择一份策略'}</h2><p className="mt-2 text-sm text-slate-500">先选方法，再设定标的、基础预算和评估日。策略版本与安全边界来自服务端目录。</p></div>
        {hasRequestedPolicy && catalog.isPending ? <p className="mt-4 inline-flex items-center gap-2 rounded-full bg-[#f1f7f4] px-4 py-2 text-sm text-[#2d6a57]"><Loader2 className="size-4 animate-spin" />正在核对官方策略版本…</p> : null}
        {catalog.isError ? <p role="alert" className="mt-4 rounded-xl border border-[#d9c7a9] bg-[#fffaf1] px-4 py-3 text-sm text-[#6f511f]">暂时无法读取策略目录。恢复本机服务后刷新页面，已有计划不受影响。</p> : null}
        {requestedStrategyUnavailable ? <p role="alert" className="mt-4 rounded-xl border border-[#d9c7a9] bg-[#fffaf1] px-4 py-3 text-sm text-[#6f511f]">这份策略当前不存在或没有通过研究准入。请返回策略中心重新选择。</p> : null}
        {selectedStrategy?.adoptable ? (
        <div className="mt-5 grid gap-7 lg:grid-cols-[minmax(0,1fr)_22rem]">
        <form onSubmit={(event) => void submit(event)} className="rounded-[1.6rem] bg-[#102028] p-6 text-white sm:p-8 lg:p-10">
          <div className="max-w-2xl">
            <p className="text-sm text-[#b8d5c6]">{selectedStrategy.name}</p>
            <h2 className="mt-3 text-2xl font-semibold tracking-[-0.035em] sm:text-3xl">先确定标的、金额和日期</h2>
            <p className="mt-3 text-sm leading-7 text-slate-300">{selectedStrategy.summary} 所有实际操作仍由你在自己的券商完成。</p>
          </div>

          <div className="mt-7 grid gap-5 sm:grid-cols-2">
            <PlanField label="投资标的" hint={instrumentHint(draft.symbol, selectedStrategy)}>
              <Input required aria-label="投资标的" autoCapitalize="characters" value={draft.symbol} onChange={(event) => setDraft((current) => ({ ...current, symbol: event.target.value }))} placeholder="US.AAPL / HK.00700" className="h-11 border-white/15 bg-white uppercase text-[#102028]" />
            </PlanField>
            <PlanField label={selectedStrategy.policy.id === 'fixed_dca' ? `每期投入金额（${currency}）` : `每期基础预算（${currency}）`} hint={selectedStrategy.policy.id === 'fixed_dca' ? '固定定投每期使用这个金额。' : '评估日会在这个预算内计算当期建议，不会超过它。'}>
              <Input required aria-label={selectedStrategy.policy.id === 'fixed_dca' ? `每期投入金额（${currency}）` : `每期基础预算（${currency}）`} inputMode="decimal" value={draft.amount} onChange={(event) => setDraft((current) => ({ ...current, amount: event.target.value }))} placeholder="1000.00" className="h-11 border-white/15 bg-white text-[#102028]" />
            </PlanField>
            <PlanField label={cadenceLabel} hint={isFormula ? '到评估日读取最近可用行情并计算本期金额，不会自动下单。' : '固定日期定义本期投入提醒，不会自动下单。'}>
              <select
                aria-label={cadenceLabel}
                value={draft.scheduleKind}
                onChange={(event) => {
                  const scheduleKind = event.target.value as MinimalPlanDraft['scheduleKind']
                  setDraft((current) => ({ ...current, scheduleKind, scheduleDay: defaultScheduleDay(scheduleKind, selectedStrategy) }))
                }}
                className="h-11 rounded-xl border border-white/15 bg-white px-3 text-sm text-[#102028] outline-none focus-visible:ring-3 focus-visible:ring-[#b8d5c6]/50"
              >
                <option value="monthly">{isFormula ? '每月评估一次' : '每月投入一次'}</option>
                <option value="weekly">{isFormula ? '每周评估一次' : '每周投入一次'}</option>
              </select>
            </PlanField>
            <PlanField label={scheduleDayLabel(draft.scheduleKind, isFormula)} hint={isFormula ? '只有约定评估日才读取规则需要的最近有效日线，并生成本期建议。' : '新计划会立即准备一次当前状态，只有投入日才会显示待执行。'}>
              {draft.scheduleKind === 'monthly' ? (
                <Input required aria-label={scheduleDayLabel(draft.scheduleKind, isFormula)} type="number" min={1} max={28} value={draft.scheduleDay} onChange={(event) => setDraft((current) => ({ ...current, scheduleDay: Number(event.target.value) }))} className="h-11 border-white/15 bg-white text-[#102028]" />
              ) : (
                <select aria-label={scheduleDayLabel(draft.scheduleKind, isFormula)} value={draft.scheduleDay} onChange={(event) => setDraft((current) => ({ ...current, scheduleDay: Number(event.target.value) }))} className="h-11 rounded-xl border border-white/15 bg-white px-3 text-sm text-[#102028] outline-none focus-visible:ring-3 focus-visible:ring-[#b8d5c6]/50">
                  {(isFormula ? formulaWeekdays : weekdays).map((day) => <option key={day.value} value={day.value}>{day.label}</option>)}
                </select>
              )}
            </PlanField>
          </div>

          <PlanField label="计划名称（可选）" hint="留空时会使用“标的 + 长期计划”。" className="mt-5">
            <Input aria-label="计划名称（可选）" value={draft.name} onChange={(event) => setDraft((current) => ({ ...current, name: event.target.value }))} placeholder="例如：我的退休储蓄" className="h-11 border-white/15 bg-white text-[#102028]" />
          </PlanField>

          {selectionError ? <p role="alert" className="mt-5 rounded-xl border border-amber-200/30 bg-amber-200/10 px-4 py-3 text-sm text-amber-50">{selectionError}</p> : null}
          {requestError ? <p role="alert" className="mt-5 rounded-xl border border-red-300/30 bg-red-300/10 px-4 py-3 text-sm text-red-100">{planRequestErrorMessage(requestError, isFormula)}</p> : null}
          {savedWithoutAdvice ? (
            <div role="status" className="mt-5 rounded-xl border border-amber-200/30 bg-amber-200/10 px-4 py-3 text-sm leading-6 text-amber-50">
              <p>计划已经保存，但本期安排暂时没有准备完成。你可以安全重试，不会重复建立计划。</p>
              <button type="button" onClick={() => void retryAdvice()} disabled={prepareDecision.isPending} className="mt-2 font-medium underline underline-offset-4 disabled:opacity-60">重新准备本期安排</button>
            </div>
          ) : null}

          <Button type="submit" disabled={saving || requestedStrategyUnavailable || catalog.isPending} className="mt-7 h-11 rounded-full bg-white px-5 text-[#102028] hover:bg-[#dcece4]">
            {saving ? <><Loader2 className="animate-spin" />正在建立…</> : <>建立并查看本期安排 <ArrowRight /></>}
          </Button>
        </form>

        <aside className="space-y-5 rounded-[1.35rem] border border-slate-200 bg-white p-6">
          <div><ShieldCheck className="size-5 text-[#2d6a57]" /><h2 className="mt-4 text-xl font-semibold tracking-[-0.03em] text-[#102028]">系统替你固定的边界</h2></div>
          <ul className="space-y-5 text-sm leading-6 text-slate-600">
            <Boundary icon={<CheckCircle2 />} title={selectedStrategy.name} text={formulaBoundary(selectedStrategy)} />
            <Boundary icon={<CircleDollarSign />} title="金额有上限" text="最小版本不会建议超过本次设定金额。" />
            <Boundary icon={<CalendarDays />} title="日期可解释" text={isFormula ? '只有约定评估日才读取最近可用行情并计算建议，其他时间继续等待。' : '只有约定投入日才显示为待执行，其他时间继续等待。'} />
          </ul>
          <p className="border-t border-slate-100 pt-5 text-xs leading-5 text-slate-400">{selectedStrategy && selectedStrategy.policy.id !== 'fixed_dca' ? '公式只生成建议；当前版本不会自动连接券商，也不会绕过你的手工确认。' : '高级券商实验仍保留在高级实验室，不会进入这条普通用户路径。'}</p>
        </aside>
        </div>
        ) : null}
      </section> : null}
    </div>
  )
}

function DeletePlanDialog({ plan, pending, onCancel, onConfirm }: { plan: InvestmentPlan | null; pending: boolean; onCancel: () => void; onConfirm: (plan: InvestmentPlan) => void }) {
  return (
    <DialogPrimitive.Root open={plan !== null} onOpenChange={(open) => { if (!open && !pending) onCancel() }}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-[#102028]/25 backdrop-blur-[2px] data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 motion-reduce:animate-none" />
        <DialogPrimitive.Content className="fixed left-1/2 top-1/2 z-50 w-[calc(100%-2rem)] max-w-md -translate-x-1/2 -translate-y-1/2 rounded-[1.4rem] border border-[#ead8bd] bg-[#fffdf8] p-6 shadow-[0_24px_70px_rgba(16,32,40,0.24)] outline-none sm:p-7 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 motion-reduce:animate-none">
          <span className="grid size-10 place-items-center rounded-full bg-[#f6e9d5] text-[#8a5a20]"><AlertTriangle className="size-5" /></span>
          <DialogPrimitive.Title className="mt-5 text-xl font-semibold tracking-[-0.03em] text-[#102028]">删除“{plan?.name}”？</DialogPrimitive.Title>
          <DialogPrimitive.Description className="mt-2 text-sm leading-6 text-slate-600">这会删除该计划及其本地建议、执行记录和纸面数据，无法恢复；不会影响券商账户中的持仓或订单。</DialogPrimitive.Description>
          <div className="mt-6 flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
            <Button type="button" variant="outline" disabled={pending} onClick={onCancel} className="rounded-full">保留计划</Button>
            <Button type="button" disabled={pending || !plan} onClick={() => { if (plan) onConfirm(plan) }} className="rounded-full bg-[#8c3f35] text-white hover:bg-[#743229]">{pending ? <><Loader2 className="animate-spin" />正在删除…</> : <><Trash2 />确认删除</>}</Button>
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
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
  strategies,
  pending,
  selectedPlanId,
  busy,
  onSelect,
  onToggle,
  onRemove,
}: {
  plans: InvestmentPlan[]
  strategies: StrategyCatalogEntry[]
  pending: boolean
  selectedPlanId: string | null
  busy: boolean
  onSelect: (id: string) => void
  onToggle: (plan: InvestmentPlan) => void
  onRemove: (plan: InvestmentPlan) => void
}) {
  const strategyNames = new Map(strategies.map((strategy) => [strategy.policy.id, strategy.name]))
  return (
    <section>
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div><h2 className="text-xl font-semibold tracking-[-0.03em] text-[#102028]">你的长期计划</h2><p className="mt-1 text-sm text-slate-500">点击一张计划卡，它就会成为个人中心正在查看的计划。</p></div>
        <Link to="/strategy-center" className="inline-flex h-10 items-center justify-center gap-2 rounded-full bg-[#102028] px-4 text-sm font-medium text-white hover:bg-[#18313c] focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-[#b8d5c6]"><span aria-hidden="true">＋</span>建立新计划</Link>
      </div>
      {pending ? <p className="mt-5 flex items-center gap-2 text-sm text-slate-500"><Loader2 className="size-4 animate-spin" />正在读取计划…</p> : null}
      {!pending && plans.length === 0 ? <p className="mt-5 rounded-xl border border-dashed border-slate-300 bg-white p-5 text-sm text-slate-600">还没有计划。前往策略中心选择一种方法，再建立你的第一份长期计划。</p> : null}
      {!pending && plans.length > 0 ? (
        <div className="mt-5 grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {plans.map((plan) => (
            <article key={plan.id} className={`rounded-[1.2rem] border bg-white p-5 ${selectedPlanId === plan.id ? 'border-[#2d6a57]' : 'border-slate-200'}`}>
              <button type="button" onClick={() => onSelect(plan.id)} className="w-full text-left focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-[#b8d5c6]/50">
                <div className="flex items-start justify-between gap-3"><div><p className="font-semibold text-[#102028]">{plan.name}</p><p className="mt-1 text-sm text-slate-500">{plan.symbol} · {strategyLabel(plan, strategyNames)}</p></div><span className={`rounded-full px-2.5 py-1 text-xs ${plan.is_active ? 'bg-[#e6f1eb] text-[#2d6a57]' : 'bg-slate-100 text-slate-500'}`}>{plan.is_active ? '进行中' : '已暂停'}</span></div>
                <div className="mt-5 grid grid-cols-2 gap-3 rounded-xl bg-[#f4f7f6] p-3">
                  <div><p className="text-xs text-slate-400">{plan.policy.id === 'fixed_dca' ? '每期投入' : '基础预算'}</p><p className="mt-1 font-semibold text-[#102028]">{formatMoney(plan.currency, plan.base_contribution)}</p></div>
                  <div><p className="text-xs text-slate-400">{plan.policy.id === 'fixed_dca' ? '投入节奏' : '评估节奏'}</p><p className="mt-1 font-semibold text-[#102028]">{scheduleLabel(plan)}</p></div>
                </div>
                <p className="mt-3 text-xs leading-5 text-slate-500">策略：{strategyLabel(plan, strategyNames)} · 单次上限 {formatMoney(plan.currency, plan.max_single_execution)}</p>
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

function defaultScheduleDay(kind: MinimalPlanDraft['scheduleKind'], strategy?: StrategyCatalogEntry): number {
  if (strategy?.default_plan.schedule_kind === kind) {
    if (kind === 'monthly') return clamp(strategy.default_plan.schedule_day, 1, 28)
    return strategy.policy.id === 'fixed_dca'
      ? clamp(strategy.default_plan.schedule_day, 1, 7)
      : nearestWeekday(strategy.default_plan.schedule_day)
  }
  const now = new Date()
  if (kind === 'monthly') return Math.min(now.getUTCDate(), 28)
  const day = now.getUTCDay() === 0 ? 7 : now.getUTCDay()
  return strategy?.policy.id === 'fixed_dca' ? day : nearestWeekday(day)
}

function planPayload(draft: MinimalPlanDraft, symbol: string, currency: string, strategy: StrategyCatalogEntry): CreateInvestmentPlanRequest {
  const defaults = strategy.default_plan
  return {
    name: draft.name.trim() || `${symbol} ${strategy.name}`,
    symbol,
    base_contribution: draft.amount.trim(),
    currency,
    schedule_kind: draft.scheduleKind,
    schedule_day: draft.scheduleDay,
    schedule_days: [draft.scheduleDay],
    policy: strategy.policy,
    bucket_allocation: {
      core_ratio: defaults.core_ratio,
      opportunity_ratio: defaults.opportunity_ratio,
    },
    risk_mode: defaults.risk_mode,
    opportunity_cash_policy: 'expire_each_period',
    max_single_execution: draft.amount.trim(),
  }
}

function policyKey(policy: { id: string; version: number }): string {
  return `${policy.id}@${policy.version}`
}

function scheduleLabel(plan: InvestmentPlan): string {
  if (plan.schedule_kind === 'weekly') return `每周 ${weekdays.find((day) => day.value === plan.schedule_day)?.label ?? plan.schedule_day}`
  return `每月 ${plan.schedule_day} 日`
}

function strategyLabel(plan: InvestmentPlan, strategyNames?: Map<string, string>): string {
  if (plan.policy.id === 'core_opportunity_v1') return '旧自适应策略'
  const catalogName = strategyNames?.get(plan.policy.id)
  if (catalogName) return catalogName
  if (plan.policy.id === 'fixed_dca') return '固定定投'
  return '自定义策略'
}

function formulaBoundary(strategy: StrategyCatalogEntry | undefined): string {
  if (!strategy || strategy.policy.id === 'fixed_dca') return '每次建议都使用你设定的金额，不读取市场或 AI 信号。'
  return `${Number(strategy.default_plan.core_ratio) * 100}% 核心投入保持固定；规则只调整 ${Number(strategy.default_plan.opportunity_ratio) * 100}% 弹性额度。`
}

function formatMoney(currency: string, amount: string): string {
  const numeric = Number(amount)
  return Number.isFinite(numeric)
    ? new Intl.NumberFormat('zh-CN', { style: 'currency', currency, maximumFractionDigits: 2 }).format(numeric)
    : `${currency} ${amount}`
}

type SupportedMarket = StrategyCatalogEntry['supported_markets'][number]

type ParsedInstrument = {
  market: SupportedMarket
  marketLabel: string
  currency: 'USD' | 'HKD' | 'CNY'
}

function parseInstrument(value: string): ParsedInstrument | null {
  const normalized = value.trim().toUpperCase()
  if (!normalized) return null
  const qualified = normalized.match(/^([A-Z]{2})\.(.+)$/)
  if (qualified) {
    const [, prefix, symbol] = qualified
    if (prefix === 'US' && isUsSymbol(symbol)) return { market: 'us', marketLabel: '美股', currency: 'USD' }
    if (prefix === 'HK' && /^\d{1,6}$/.test(symbol)) return { market: 'hong_kong', marketLabel: '港股', currency: 'HKD' }
    if (prefix === 'SH' && /^\d{6}$/.test(symbol)) return { market: 'china_shanghai', marketLabel: '沪市', currency: 'CNY' }
    if (prefix === 'SZ' && /^\d{6}$/.test(symbol)) return { market: 'china_shenzhen', marketLabel: '深市', currency: 'CNY' }
    return null
  }
  return isUsSymbol(normalized) ? { market: 'us', marketLabel: '美股', currency: 'USD' } : null
}

function isUsSymbol(value: string): boolean {
  return value.length > 0 && value.length <= 15 && /^[A-Z0-9.-]+$/.test(value)
}

function marketAvailabilityText(strategy: StrategyCatalogEntry): string {
  const labels: Record<SupportedMarket, string> = {
    us: '美股',
    hong_kong: '港股',
    china_shanghai: '沪市',
    china_shenzhen: '深市',
  }
  return strategy.supported_markets.map((market) => labels[market]).join('、')
}

function instrumentHint(value: string, strategy: StrategyCatalogEntry): string {
  const parsed = parseInstrument(value)
  const markets = marketAvailabilityText(strategy)
  const requirement = strategy.data_requirement.required_close_observations
  if (!parsed) {
    return `支持 ${markets}；例如 US.AAPL、HK.00700、SH.600519、SZ.000001。`
  }
  if (requirement === 0) {
    return `${parsed.marketLabel} · ${parsed.currency}；这份策略不读取行情。`
  }
  return `${parsed.marketLabel} · ${parsed.currency}；建立前会检查至少 ${requirement} 条有效日线。`
}

function scheduleDayLabel(kind: MinimalPlanDraft['scheduleKind'], formula: boolean): string {
  if (kind === 'monthly') return formula ? '每月哪天评估' : '每月几号投入'
  return formula ? '每周哪天评估' : '每周哪天投入'
}

function nearestWeekday(day: number): number {
  const normalized = clamp(day, 1, 7)
  return normalized > 5 ? 1 : normalized
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(Math.max(value, minimum), maximum)
}

function planRequestErrorMessage(error: Error, formula: boolean): string {
  if (error instanceof ApiRequestError && error.status === 503 && formula) {
    return '这份规则需要的历史行情暂时不可用。请确认本机 OpenD、行情权限和市场数据服务后重试。'
  }
  if (error instanceof ApiRequestError && error.status === 400 && formula) {
    return '这个标的没有通过规则的数据检查。请核对代码，或换用有足够有效日线历史的标的。'
  }
  if (error instanceof ApiRequestError && error.status === 400) {
    return '计划参数没有通过检查。请核对标的代码、金额和日期。'
  }
  return '计划没有保存成功。请检查输入并确认本机服务正在运行。'
}
