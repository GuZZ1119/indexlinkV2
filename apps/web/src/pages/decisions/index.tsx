import { History } from 'lucide-react'
import { Link, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useState } from 'react'

import { useAllDecisionRecords, useApproveDecisionPaperOrder, useDecisionRecord, useManualExecutions, usePlans } from '@/api/queries'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ManualExecutionHistory } from '@/components/v2_1/manual-execution-history'
import { actionBadgeClass } from '@/lib/decision'
import { cn } from '@/lib/utils'
import type { DecisionAction, DecisionRecord, PersistedMarketSentimentSnapshot } from '@/api/types'
import { filterDecisionRecords } from './filters'

const PAGE_SIZE = 12
const actions: DecisionAction[] = ['overweight', 'standard', 'underweight', 'tactical_delay', 'skip']

/** Display cross-plan persisted decision history, with filters that do not discard cached server data. */
export default function DecisionsPage() {
  const { t } = useTranslation()
  const { id } = useParams()
  const { data: plans = [] } = usePlans()
  const history = useAllDecisionRecords()
  const record = useDecisionRecord(id ?? null)
  const approvePaperOrder = useApproveDecisionPaperOrder()
  const [planId, setPlanId] = useState('')
  const [action, setAction] = useState('')
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [page, setPage] = useState(1)

  if (id) return <DecisionDetail record={record.data} isPending={record.isPending} error={record.error} approvePaperOrder={approvePaperOrder} />
  if (history.isPending) return <PageMessage message={t('live.history.loading')} />
  if (history.error) return <PageMessage message={errorMessage(history.error)} />

  const filtered = filterDecisionRecords(history.data ?? [], { planId, action: action as '' | DecisionAction, from, to })
  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE))
  const currentPage = Math.min(page, totalPages)
  const visible = filtered.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE)
  const planName = new Map(plans.map((plan) => [plan.id, `${plan.name} · ${plan.symbol}`]))
  const reset = () => { setPlanId(''); setAction(''); setFrom(''); setTo(''); setPage(1) }
  const updatePage = (setter: (value: string) => void, value: string) => { setter(value); setPage(1) }

  return (
    <div className="mx-auto w-full max-w-5xl p-4 lg:p-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2"><History className="size-4 text-muted-foreground" />{t('live.history.title')}</CardTitle>
          <CardDescription>{t('live.history.description')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <section className="grid gap-2 rounded-lg border bg-muted/20 p-3 sm:grid-cols-2 lg:grid-cols-5">
            <label className="grid gap-1 text-xs"><span>{t('decisions.plan')}</span><select value={planId} onChange={(event) => updatePage(setPlanId, event.target.value)} className="h-9 rounded-md border bg-background px-2 text-sm"><option value="">{t('decisions.allPlans')}</option>{plans.map((plan) => <option key={plan.id} value={plan.id}>{plan.name} · {plan.symbol}</option>)}</select></label>
            <label className="grid gap-1 text-xs"><span>{t('decisions.action')}</span><select value={action} onChange={(event) => updatePage(setAction, event.target.value)} className="h-9 rounded-md border bg-background px-2 text-sm"><option value="">{t('decisions.filters')}</option>{actions.map((value) => <option key={value} value={value}>{t(`action.${value}`)}</option>)}</select></label>
            <label className="grid gap-1 text-xs"><span>{t('decisions.dateFrom')}</span><input value={from} onChange={(event) => updatePage(setFrom, event.target.value)} type="date" className="h-9 rounded-md border bg-background px-2 text-sm" /></label>
            <label className="grid gap-1 text-xs"><span>{t('decisions.dateTo')}</span><input value={to} onChange={(event) => updatePage(setTo, event.target.value)} type="date" className="h-9 rounded-md border bg-background px-2 text-sm" /></label>
            <div className="flex items-end"><Button variant="outline" className="w-full" onClick={reset}>{t('decisions.clear')}</Button></div>
          </section>
          {visible.length === 0 ? <PageMessage message={t('decisions.noMatches')} /> : visible.map((item) => {
            const decision = item.decision_snapshot
            return <Link key={item.id} to={`/decisions/${item.id}`} className="block rounded-lg border p-4 transition-colors hover:bg-muted/50"><div className="flex flex-wrap items-center justify-between gap-2"><span className="font-mono font-semibold">{item.symbol}</span><span className="text-xs text-muted-foreground">{planName.get(item.plan_id) ?? item.plan_id}</span><Badge className={cn(actionBadgeClass[decision.action])}>{t(`action.${decision.action}`)}</Badge><span className="text-xs text-muted-foreground">{new Date(item.created_at).toLocaleString()}</span></div><p className="mt-2 line-clamp-2 text-sm text-muted-foreground">{item.summary}</p>{item.broker_order_ack && <p className="mt-2 text-xs text-semantic-positive">{t('live.history.paperAck')}: {item.broker_order_ack.status} · {item.broker_order_ack.order_id}</p>}</Link>
          })}
          <div className="flex flex-wrap items-center justify-between gap-2 border-t pt-3 text-xs text-muted-foreground"><span>{t('decisions.page', { current: currentPage, total: totalPages, count: filtered.length })}</span><div className="flex gap-2"><Button variant="outline" size="sm" disabled={currentPage <= 1} onClick={() => setPage(currentPage - 1)}>{t('decisions.previous')}</Button><Button variant="outline" size="sm" disabled={currentPage >= totalPages} onClick={() => setPage(currentPage + 1)}>{t('decisions.next')}</Button></div></div>
        </CardContent>
      </Card>
    </div>
  )
}

/** Render one full immutable decision record and preserve its approval gate. */
function DecisionDetail({ record, isPending, error, approvePaperOrder }: { record?: DecisionRecord; isPending: boolean; error: unknown; approvePaperOrder: ReturnType<typeof useApproveDecisionPaperOrder> }) {
  const { t } = useTranslation()
  const journal = useManualExecutions(record?.id ?? null)
  if (isPending) return <PageMessage message={t('live.history.loadRecord')} />
  if (error || !record) return <PageMessage message={errorMessage(error)} />
  const decision = record.decision_snapshot
  return <div className="mx-auto grid w-full max-w-6xl gap-5 p-4 lg:grid-cols-[minmax(0,1fr)_21rem] lg:p-6"><Card><CardHeader><CardTitle className="flex items-center gap-2"><span>{record.symbol}</span><Badge className={cn(actionBadgeClass[decision.action])}>{t(`action.${decision.action}`)}</Badge></CardTitle><CardDescription>{new Date(record.created_at).toLocaleString()}</CardDescription></CardHeader><CardContent className="space-y-4"><p className="rounded-lg bg-muted/50 p-3 text-sm leading-relaxed">{record.summary}</p><AuditOverview record={record} /><div className="grid gap-4 md:grid-cols-2"><SignalEvidence title={t('live.history.fundamental')} snapshot={record.fundamental_snapshot} /><SignalEvidence title={t('live.history.trend')} snapshot={record.trend_snapshot} /></div>{record.sentiment_snapshot && <SentimentEvidence value={record.sentiment_snapshot} />}<OrderEvidence record={record} approvePaperOrder={approvePaperOrder} /></CardContent></Card><ManualExecutionHistory events={journal.data ?? []} pending={journal.isPending} error={journal.error} onRetry={() => void journal.refetch()} className="self-start" /></div>
}

/** Render saved Qwen reasoning as readable audit evidence instead of a raw JSON blob. */
function SentimentEvidence({ value }: { value: PersistedMarketSentimentSnapshot }) {
  const { t } = useTranslation()
  const evidence = typeof value.rationale === 'string' && Array.isArray(value.warnings) && Array.isArray(value.headlines) ? { rationale: value.rationale, warnings: value.warnings, headlines: value.headlines } : null
  const audit = value.audit
  return <section className="space-y-3 rounded-lg border bg-muted/20 p-4 text-sm"><h2 className="font-semibold">{t('live.history.sentiment')}</h2>{audit?.status === 'degraded' && <p className="rounded-md border border-amber-300 bg-amber-50 p-2 text-sm text-amber-900">{t('decisions.audit.aiFallback')} {audit.reason && `${t('decisions.audit.aiFallbackReason')}: ${audit.reason}`}</p>}{!evidence && !audit && <p className="text-muted-foreground">{t('dashboard.decisionExplanation.aiLegacySource')}</p>}{evidence && <p className="text-muted-foreground">{evidence.rationale}</p>}{audit?.status === 'available' && <div className="grid gap-2 rounded-md border bg-background p-3 text-xs sm:grid-cols-3"><AuditFact label={t('decisions.audit.aiGeneratedAt')} value={audit.generated_at ? new Date(audit.generated_at).toLocaleString() : t('decisions.audit.sourceUnavailable')} /><AuditFact label={t('decisions.audit.aiLatency')} value={typeof audit.latency_ms === 'number' ? `${audit.latency_ms} ms` : t('decisions.audit.sourceUnavailable')} /><AuditFact label={t('decisions.audit.aiPromptVersion')} value={audit.prompt_version ?? t('decisions.audit.sourceUnavailable')} /></div>}{evidence && evidence.warnings.length > 0 && <div><p className="font-medium">{t('dashboard.decisionExplanation.aiWarnings')}</p><ul className="mt-1 list-disc space-y-1 pl-5 text-muted-foreground">{evidence.warnings.map((warning) => <li key={warning}>{warning}</li>)}</ul></div>}{evidence && <div><p className="font-medium">{t('dashboard.decisionExplanation.aiHeadlines')}</p><ul className="mt-1 space-y-1 text-muted-foreground">{evidence.headlines.map((headline) => <li key={`${headline.published_at}-${headline.title}`}>{headline.url ? <a className="underline-offset-4 hover:underline" href={headline.url} rel="noreferrer" target="_blank">{headline.title}</a> : headline.title}<span className="ml-2 text-xs">{new Date(headline.published_at).toLocaleString()}</span></li>)}</ul></div>}</section>
}

/** Render a concise empty, loading, or error state without mock content. */
function PageMessage({ message }: { message: string }) { return <div className="rounded-lg border border-dashed p-6 text-sm text-muted-foreground">{message}</div> }

/** Render the decision time, planned amount, weights, and action without exposing raw JSON. */
function AuditOverview({ record }: { record: DecisionRecord }) {
  const { t } = useTranslation()
  const decision = record.decision_snapshot
  const trigger = readText(record.execution_snapshot, 'trigger')
  const policy = record.policy_evidence?.policy ?? decision.policy
  const change = decision.change_from_previous
  return <><section className="grid gap-3 rounded-lg border bg-muted/20 p-4 text-sm sm:grid-cols-2 lg:grid-cols-4"><AuditFact label={t('decisions.audit.executionTime')} value={new Date(record.created_at).toLocaleString()} /><AuditFact label={t('decisions.audit.plannedAmount')} value={record.planned_contribution ? `${record.planned_contribution} ${record.currency}` : t('decisions.audit.notDue')} /><AuditFact label={t('decisions.audit.trigger')} value={trigger ?? t('decisions.audit.historical')} /><AuditFact label={t('decisions.audit.policy')} value={policy ? `${policy.id}@${policy.version}` : t('decisions.audit.preMigration')} /><AuditFact label={t('decisions.audit.composite')} value={`${t(`action.${decision.action}`)} · ${(decision.multiplier * 100).toFixed(0)}%`} /><AuditFact label={t('decisions.audit.fundamental')} value={formatScore(decision.fundamental_score, t)} /><AuditFact label={t('decisions.audit.trend')} value={formatScore(decision.trend_score, t)} /><AuditFact label={t('decisions.audit.sentiment')} value={typeof decision.sentiment_score === 'number' ? decision.sentiment_score.toFixed(2) : t('decisions.audit.fallback')} /><AuditFact label={t('decisions.audit.weightMode')} value={decision.weight_mode ?? 'fixed_dca'} /></section>{change && <DecisionChangeEvidence change={change} />}</>
}

/** Explain the stored delta without recalculating policy inputs or outputs in the browser. */
function DecisionChangeEvidence({ change }: { change: NonNullable<DecisionRecord['decision_snapshot']['change_from_previous']> }) {
  const { t } = useTranslation()
  const message = change.status === 'initial' ? t('decisions.audit.changeInitial') : change.status === 'unchanged' ? t('decisions.audit.changeUnchanged') : t('decisions.audit.changeChanged')
  return <section className="mt-3 space-y-2 rounded-lg border bg-muted/20 p-4 text-sm"><h2 className="font-semibold">{t('decisions.audit.change')}</h2><p className="text-muted-foreground">{message}</p>{change.status === 'changed' && <div className="grid gap-2 sm:grid-cols-3">{typeof change.action_changed === 'boolean' && <AuditFact label={t('decisions.audit.actionChanged')} value={change.action_changed ? 'true' : 'false'} />}{typeof change.multiplier_delta === 'number' && <AuditFact label={t('decisions.audit.multiplierDelta')} value={change.multiplier_delta.toFixed(2)} />}{typeof change.final_score_delta === 'number' && <AuditFact label={t('decisions.audit.scoreDelta')} value={change.final_score_delta.toFixed(2)} />}</div>}{change.previous_record_id && <p className="font-mono text-xs text-muted-foreground">{t('decisions.audit.previousRecord')}: {change.previous_record_id}</p>}</section>
}

/** Show an omitted fixed-DCA score as unused rather than as a synthetic zero. */
function formatScore(value: number | null | undefined, t: (key: string) => string): string { return typeof value === 'number' ? value.toFixed(2) : t('decisions.audit.unused') }

/** Render one readable source and score layer from a structured audit snapshot. */
function SignalEvidence({ title, snapshot }: { title: string; snapshot: Record<string, unknown> }) {
  const { t } = useTranslation()
  const source = asRecord(snapshot.source)
  const signal = asRecord(snapshot.signal) ?? snapshot
  return <section className="space-y-2 rounded-lg border p-4 text-sm"><h2 className="font-semibold">{title}</h2><p className="text-muted-foreground">{readText(source, 'kind') === 'automatic_market_data' ? t('decisions.audit.automatic') : t('decisions.audit.manual')}</p><div className="grid grid-cols-2 gap-2 text-xs text-muted-foreground">{Object.entries(signal).filter(([, value]) => typeof value === 'number' || typeof value === 'string').map(([key, value]) => <span key={key}>{key}: {String(value)}</span>)}</div>{source && <p className="text-xs leading-relaxed text-muted-foreground">{readText(source, title.includes('基本') || title.includes('Fundamental') ? 'fundamental' : 'trend') ?? readText(source, 'description') ?? t('decisions.audit.sourceUnavailable')}</p>}</section>
}

/** Render the paper-order intent and acknowledgement as readable evidence. */
function OrderEvidence({ record, approvePaperOrder }: { record: DecisionRecord; approvePaperOrder: ReturnType<typeof useApproveDecisionPaperOrder> }) {
  const { t } = useTranslation()
  const approvalRequired = readBoolean(record.execution_snapshot, 'execution', 'bucket_split', 'requires_approval')
  const canApprove = approvalRequired && !record.broker_order_request && !record.broker_order_ack && record.execution_status === 'due'
  if (!record.broker_order_request && !record.broker_order_ack && !canApprove) return null
  return <section className="space-y-2 rounded-lg border p-4 text-sm"><h2 className="font-semibold">{t('decisions.audit.orders')}</h2>{record.broker_order_request && <p className="text-muted-foreground">{t('decisions.audit.request')}: {readText(record.broker_order_request, 'side')} · {readText(record.broker_order_request, 'quantity')} · {readText(record.broker_order_request, 'order_type')}</p>}{record.broker_order_ack ? <p className="text-semantic-positive">{t('decisions.audit.acknowledgement')}: {record.broker_order_ack.status} · {record.broker_order_ack.order_id} · {record.broker_order_ack.environment}</p> : <p className="text-muted-foreground">{t('decisions.audit.noOrder')}</p>}{canApprove && <div className="space-y-2 pt-2"><p className="text-xs text-muted-foreground">{t('decisions.audit.approval')}</p><Button disabled={approvePaperOrder.isPending} onClick={() => approvePaperOrder.mutate({ id: record.id, idempotencyKey: globalThis.crypto.randomUUID() })}>{approvePaperOrder.isPending ? t('decisions.audit.submitting') : t('decisions.audit.approve')}</Button>{approvePaperOrder.error && <p className="text-xs text-destructive">{errorMessage(approvePaperOrder.error)}</p>}</div>}</section>
}

/** Convert safe request errors to a concise display message. */
function errorMessage(error: unknown): string { return error instanceof Error ? error.message : 'Request failed' }
/** Read one display-safe text field from a JSON audit object. */
function readText(value: Record<string, unknown> | undefined, key: string): string | undefined { const field = value?.[key]; return typeof field === 'string' || typeof field === 'number' ? String(field) : undefined }
/** Read a nested boolean from a trusted audit snapshot for display-only gating. */
function readBoolean(value: Record<string, unknown>, ...keys: string[]): boolean { let current: unknown = value; for (const key of keys) { if (typeof current !== 'object' || current === null || Array.isArray(current)) return false; current = (current as Record<string, unknown>)[key] } return current === true }
/** Narrow one unknown JSON value to an object for display-only extraction. */
function asRecord(value: unknown): Record<string, unknown> | undefined { return typeof value === 'object' && value !== null && !Array.isArray(value) ? value as Record<string, unknown> : undefined }
/** Render one compact audit field. */
function AuditFact({ label, value }: { label: string; value: string }) { return <div><p className="text-xs text-muted-foreground">{label}</p><p className="mt-1 font-medium">{value}</p></div> }
