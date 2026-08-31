import { useMemo, useState, type FormEvent } from 'react'
import { CheckCircle2, Copy, Play, Plus, Save, ShieldCheck, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { useActivatePlanPolicy, useCreateStrategy, usePlans, useStrategies, useStrategyAdmission, useValidateStrategy } from '@/api/queries'
import type { StrategyAdmissionReport, StrategyComparisonDocument, StrategyConditionDocument, StrategyIndicatorDocument, StrategyRuleDocument, StrategySpecDocument } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'

const indicator = (): StrategyIndicatorDocument => ({ kind: 'relative_strength_index', lookback_days: 14 })
const comparison = (): StrategyComparisonDocument => ({ kind: 'comparison', expression: { kind: 'indicator', indicator: indicator() }, operator: 'less_than', threshold: '35' })
const rule = (): StrategyRuleDocument => ({ condition: comparison(), action: { kind: 'set_opportunity_multiplier', multiplier: 1 } })
const newDocument = (): StrategySpecDocument => ({ policy_id: 'dsl_risk_guard', policy_version: 1, name: 'Opportunity risk guard', rules: [rule()] })

/** Form-only Strategy Studio; it never accepts arbitrary source code. */
export default function StrategiesPage() {
  const { t } = useTranslation()
  const strategies = useStrategies()
  const { data: plans = [] } = usePlans()
  const validate = useValidateStrategy()
  const create = useCreateStrategy()
  const activate = useActivatePlanPolicy()
  const admission = useStrategyAdmission()
  const [document, setDocument] = useState(newDocument)
  const [selected, setSelected] = useState<string | null>(null)
  const [simulation, setSimulation] = useState<{ as_of: string; matched_rule_index: number | null; action: string; multiplier: number; evidence: Array<{ indicator: string; value: string }> } | null>(null)
  const [simulationError, setSimulationError] = useState<string | null>(null)
  const [admissionPolicy, setAdmissionPolicy] = useState<string | null>(null)
  const selectedStrategy = useMemo(() => strategies.data?.find((item) => `${item.policy.id}@${item.policy.version}` === selected) ?? strategies.data?.[0], [selected, strategies.data])
  const selectedKey = selectedStrategy ? `${selectedStrategy.policy.id}@${selectedStrategy.policy.version}` : null
  const activeAdmission = admissionPolicy === selectedKey ? admission.data : undefined
  const error = validate.data?.valid === false ? validate.data.error : create.error ?? activate.error

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    const result = await validate.mutateAsync(document)
    if (!result.valid || !result.document) return
    const saved = await create.mutateAsync(result.document)
    setSelected(`${saved.policy.id}@${saved.policy.version}`)
  }
  const simulate = async (symbol: string) => {
    if (!selectedStrategy) return
    setSimulationError(null)
    try {
      const response = await fetch(`${import.meta.env.VITE_API_BASE_URL ?? ''}/strategies/${selectedStrategy.policy.id}/${selectedStrategy.policy.version}/simulate`, { method: 'POST', headers: { 'Content-Type': 'application/json', Accept: 'application/json' }, body: JSON.stringify({ symbol }) })
      const body = await response.json()
      if (!response.ok) throw new Error(body?.error?.message ?? 'simulation failed')
      setSimulation(body)
    } catch (reason) { setSimulationError(reason instanceof Error ? reason.message : 'simulation failed') }
  }
  const updateRule = (index: number, next: StrategyRuleDocument) => setDocument((current) => ({ ...current, rules: current.rules.map((item, itemIndex) => itemIndex === index ? next : item) }))

  return <div className="mx-auto grid w-full max-w-7xl gap-4 p-4 lg:grid-cols-[18rem_minmax(0,1fr)_22rem] lg:p-6">
    <Card><CardHeader><CardTitle>{t('studio.versions')}</CardTitle><CardDescription>{t('studio.versionsDescription')}</CardDescription></CardHeader><CardContent className="space-y-2">{strategies.data?.map((item) => <button key={`${item.policy.id}@${item.policy.version}`} type="button" onClick={() => { setSelected(`${item.policy.id}@${item.policy.version}`); setAdmissionPolicy(null) }} className="w-full rounded-lg border p-3 text-left hover:bg-muted/50"><strong>{item.name}</strong><span className="mt-1 block font-mono text-xs text-muted-foreground">{item.policy.id}@{item.policy.version}</span></button>)}<Button className="w-full" variant="outline" onClick={() => selectedStrategy && setDocument({ ...selectedStrategy.document, policy_id: `${selectedStrategy.policy.id}_v${selectedStrategy.policy.version + 1}`, policy_version: selectedStrategy.policy.version + 1, name: `${selectedStrategy.name} copy` })} disabled={!selectedStrategy}><Copy className="mr-2 size-4" />{t('studio.duplicate')}</Button></CardContent></Card>
    <div className="space-y-4"><Card className="border-sky-200 bg-sky-50/40"><CardHeader><CardTitle className="flex gap-2"><ShieldCheck className="size-5 text-sky-700" />{t('studio.title')}</CardTitle><CardDescription>{t('studio.description')}</CardDescription></CardHeader><CardContent><form className="space-y-4" onSubmit={(event) => void submit(event)}><div className="grid gap-3 sm:grid-cols-3"><Field label={t('studio.policyId')} value={document.policy_id} onChange={(value) => setDocument((current) => ({ ...current, policy_id: value }))} /><Field label={t('studio.version')} type="number" value={String(document.policy_version)} onChange={(value) => setDocument((current) => ({ ...current, policy_version: Number(value) }))} /><Field label={t('studio.name')} value={document.name} onChange={(value) => setDocument((current) => ({ ...current, name: value }))} /></div>{document.rules.map((item, index) => <RuleEditor key={index} index={index} rule={item} onChange={(next) => updateRule(index, next)} onRemove={() => setDocument((current) => ({ ...current, rules: current.rules.filter((_, itemIndex) => itemIndex !== index) }))} removable={document.rules.length > 1} />)}<Button type="button" variant="outline" onClick={() => setDocument((current) => ({ ...current, rules: [...current.rules, rule()] }))}><Plus className="mr-2 size-4" />{t('studio.addRule')}</Button>{error && <p className="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">{error instanceof Error ? error.message : error}</p>}{validate.data?.valid && <p className="flex gap-2 text-sm text-emerald-700"><CheckCircle2 className="size-4" />{t('studio.valid')}</p>}<Button className="w-full" type="submit" disabled={validate.isPending || create.isPending}><Save className="mr-2 size-4" />{t('studio.save')}</Button></form></CardContent></Card>{selectedStrategy && <Card><CardHeader><CardTitle>{t('studio.simulation')}</CardTitle><CardDescription>{t('studio.simulationDescription')}</CardDescription></CardHeader><CardContent className="space-y-3">{plans.map((plan) => <Button key={plan.id} variant="outline" className="mr-2" onClick={() => void simulate(plan.symbol)}><Play className="mr-2 size-4" />{t('studio.simulate', { symbol: plan.symbol })}</Button>)}{simulationError && <p className="text-sm text-destructive">{simulationError}</p>}{simulation && <div className="rounded-lg border bg-muted/40 p-3 text-sm"><p>{t('studio.asOf', { date: simulation.as_of })} {simulation.matched_rule_index === null ? t('studio.noRule') : t('studio.matchedRule', { index: simulation.matched_rule_index + 1 })}</p><p className="mt-1">{t('studio.action', { action: simulation.action, multiplier: simulation.multiplier.toFixed(2) })}</p><p className="mt-2 text-xs text-muted-foreground">{t('studio.evidence', { evidence: simulation.evidence.map((value) => `${value.indicator}=${value.value}`).join('; ') })}</p></div>}</CardContent></Card>}</div>
    <div className="space-y-4"><Card className="border-amber-200 bg-amber-50/40"><CardHeader><CardTitle>{t('studio.admission')}</CardTitle><CardDescription>{t('studio.admissionDescription')}</CardDescription></CardHeader><CardContent className="space-y-3"><Button className="w-full" variant="outline" disabled={!selectedStrategy || admission.isPending} onClick={() => { if (selectedStrategy) { setAdmissionPolicy(selectedKey); admission.mutate(selectedStrategy.policy) } }}><Play className="mr-2 size-4" />{t('studio.runAdmission')}</Button>{admission.error && <p className="text-sm text-destructive">{admission.error instanceof Error ? admission.error.message : t('studio.admissionFailed')}</p>}{activeAdmission && <AdmissionSummary report={activeAdmission} />}</CardContent></Card><Card><CardHeader><CardTitle>{t('studio.activate')}</CardTitle><CardDescription>{t('studio.activateDescription')}</CardDescription></CardHeader><CardContent className="space-y-3">{selectedStrategy && plans.map((plan) => <div key={plan.id} className="rounded-lg border p-3"><p className="font-medium">{plan.name} · {plan.symbol}</p><p className="text-xs text-muted-foreground">{t('studio.current', { policy: `${plan.policy.id}@${plan.policy.version}` })}</p><Button className="mt-3 w-full" size="sm" disabled={!activeAdmission?.eligible || activate.isPending} onClick={() => { if (globalThis.confirm(t('studio.activateConfirm', { strategy: selectedStrategy.name, plan: plan.name }))) activate.mutate({ planId: plan.id, policy: selectedStrategy.policy }) }}>{t('studio.activateButton')}</Button>{!activeAdmission?.eligible && <p className="mt-2 text-xs text-muted-foreground">{t('studio.admissionRequired')}</p>}</div>)}</CardContent></Card></div>
  </div>
}

/** Display truthful fixed-sample admission facts without promising performance. */
function AdmissionSummary({ report }: { report: StrategyAdmissionReport }) {
  const { t } = useTranslation()
  if (!report.eligible) return <p className="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">{t('studio.rejected', { reason: report.reason ?? t('studio.rejectedFallback') })}</p>
  return <div className="space-y-3 rounded-lg border bg-background p-3 text-sm"><p className="text-emerald-700">{t('studio.eligible')}</p>{report.assets.map((asset) => <div key={asset.symbol} className="rounded border p-2"><p className="font-medium">{asset.symbol} · {t('studio.observations', { count: asset.observations })}</p><p className="mt-1 text-xs text-muted-foreground">{t('studio.evidenceCoverage', { start: asset.evidence_start_as_of, end: asset.evidence_end_as_of })}</p><p className="mt-1 text-xs text-muted-foreground">{t('studio.rollingWindows', { count: asset.rolling_out_of_sample.length })}</p><div className="mt-2 grid grid-cols-2 gap-2 text-xs"><Metric label={t('studio.strategyTerminal')} value={`$${asset.strategy.terminal_wealth_usd.toFixed(0)}`} /><Metric label={t('studio.dcaTerminal')} value={`$${asset.fixed_dca.terminal_wealth_usd.toFixed(0)}`} /><Metric label={t('studio.strategyXirr')} value={percent(asset.strategy.xirr_percent, t('studio.insufficient'))} /><Metric label={t('studio.dcaXirr')} value={percent(asset.fixed_dca.xirr_percent, t('studio.insufficient'))} /><Metric label={t('studio.strategyDrawdown')} value={`${asset.strategy.maximum_drawdown_percent.toFixed(2)}%`} /><Metric label={t('studio.dcaDrawdown')} value={`${asset.fixed_dca.maximum_drawdown_percent.toFixed(2)}%`} /><Metric label={t('studio.strategyVolatility')} value={percent(asset.strategy.annualized_volatility_percent, t('studio.insufficient'))} /><Metric label={t('studio.dcaVolatility')} value={percent(asset.fixed_dca.annualized_volatility_percent, t('studio.insufficient'))} /><Metric label={t('studio.strategySortino')} value={number(asset.strategy.sortino_ratio, t('studio.insufficient'))} /><Metric label={t('studio.dcaSortino')} value={number(asset.fixed_dca.sortino_ratio, t('studio.insufficient'))} /><Metric label={t('studio.strategyCash')} value={`${asset.strategy.cash_utilisation_percent.toFixed(1)}%`} /><Metric label={t('studio.dcaCash')} value={`${asset.fixed_dca.cash_utilisation_percent.toFixed(1)}%`} /></div></div>)}</div>
}

/** Render one compact report metric. */
function Metric({ label, value }: { label: string; value: string }) { return <p className="rounded bg-muted/50 p-2"><span className="block text-muted-foreground">{label}</span>{value}</p> }
/** Render unavailable statistics without inventing a zero. */
function percent(value: number | undefined, unavailable: string) { return value === undefined ? unavailable : `${value.toFixed(2)}%` }
/** Render a nullable dimensionless statistic without inventing a zero. */
function number(value: number | undefined, unavailable: string) { return value === undefined ? unavailable : value.toFixed(2) }
/** Render one accessible text input. */
function Field({ label, value, onChange, type }: { label: string; value: string; onChange: (value: string) => void; type?: string }) { return <label className="grid gap-1 text-sm font-medium">{label}<Input type={type} value={value} onChange={(event) => onChange(event.target.value)} /></label> }

/** Edit one ordered whitelist rule without allowing arbitrary executable code. */
function RuleEditor({ index, rule, onChange, onRemove, removable }: { index: number; rule: StrategyRuleDocument; onChange: (value: StrategyRuleDocument) => void; onRemove: () => void; removable: boolean }) {
  const { t } = useTranslation()
  const comparisons = rule.condition.kind === 'comparison' ? [rule.condition] : rule.condition.conditions
  const group = rule.condition.kind === 'comparison' ? 'single' : rule.condition.kind
  const setCondition = (conditions: StrategyComparisonDocument[], nextGroup = group) => onChange({ ...rule, condition: nextGroup === 'single' ? conditions[0] : { kind: nextGroup, conditions } as StrategyConditionDocument })
  return <section className="space-y-3 rounded-lg border bg-background p-3"><div className="flex items-center justify-between"><strong className="text-sm">{t('studio.priorityRule', { index: index + 1 })}</strong>{removable && <Button type="button" variant="ghost" size="icon" onClick={onRemove}><Trash2 className="size-4" /></Button>}</div><label className="grid max-w-48 gap-1 text-sm">{t('studio.conditionGroup')}<select className="h-9 rounded-md border px-2" value={group} onChange={(event) => setCondition(comparisons, event.target.value)}><option value="single">{t('studio.single')}</option><option value="all">{t('studio.all')}</option><option value="any">{t('studio.any')}</option></select></label>{comparisons.map((condition, conditionIndex) => <ComparisonEditor key={conditionIndex} condition={condition} onChange={(next) => setCondition(comparisons.map((item, itemIndex) => itemIndex === conditionIndex ? next : item))} />)}{group !== 'single' && <Button type="button" size="sm" variant="outline" onClick={() => setCondition([...comparisons, comparison()])}><Plus className="mr-1 size-3" />{t('studio.addCondition')}</Button>}<label className="grid gap-1 text-sm">{t('studio.opportunityAction')}<select className="h-9 rounded-md border px-2" value={rule.action.kind} onChange={(event) => onChange({ ...rule, action: event.target.value === 'skip_opportunity' ? { kind: 'skip_opportunity' } : { kind: 'set_opportunity_multiplier', multiplier: 1 } })}><option value="set_opportunity_multiplier">{t('studio.setMultiplier')}</option><option value="skip_opportunity">{t('studio.skipOpportunity')}</option></select></label>{rule.action.kind === 'set_opportunity_multiplier' && <Field label={t('studio.multiplier')} type="number" value={String(rule.action.multiplier)} onChange={(value) => onChange({ ...rule, action: { kind: 'set_opportunity_multiplier', multiplier: Number(value) } })} />}</section>
}

/** Edit one safe comparison against a whitelisted evidence indicator. */
function ComparisonEditor({ condition, onChange }: { condition: StrategyComparisonDocument; onChange: (value: StrategyComparisonDocument) => void }) {
  const { t } = useTranslation()
  const indicator = condition.expression.indicator
  return <div className="grid gap-2 sm:grid-cols-3"><label className="grid gap-1 text-sm">{t('studio.indicator')}<select className="h-9 rounded-md border px-2" value={indicator.kind} onChange={(event) => { const kind = event.target.value as StrategyIndicatorDocument['kind']; const next: StrategyIndicatorDocument = kind === 'vix' || kind === 'close_price' ? { kind } : { kind, lookback_days: kind === 'relative_strength_index' ? 14 : kind === 'drawdown' ? 90 : 200 }; onChange({ ...condition, expression: { kind: 'indicator', indicator: next } }) }}><option value="close_price">{t('studio.close')}</option><option value="simple_moving_average">SMA</option><option value="exponential_moving_average">EMA</option><option value="relative_strength_index">RSI</option><option value="drawdown">{t('studio.strategyDrawdown')}</option><option value="vix">VIX</option></select></label>{'lookback_days' in indicator && <Field label={t('studio.window')} type="number" value={String(indicator.lookback_days)} onChange={(value) => onChange({ ...condition, expression: { kind: 'indicator', indicator: { ...indicator, lookback_days: Number(value) } } })} />}<label className="grid gap-1 text-sm">{t('studio.comparison')}<select className="h-9 rounded-md border px-2" value={condition.operator} onChange={(event) => onChange({ ...condition, operator: event.target.value as StrategyComparisonDocument['operator'] })}><option value="less_than">{t('studio.less')}</option><option value="less_than_or_equal">{t('studio.lessEqual')}</option><option value="greater_than">{t('studio.greater')}</option><option value="greater_than_or_equal">{t('studio.greaterEqual')}</option></select></label><Field label={t('studio.threshold')} value={condition.threshold} onChange={(value) => onChange({ ...condition, threshold: value })} /></div>
}
