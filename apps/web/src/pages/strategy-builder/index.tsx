import { useState, type Dispatch, type FormEvent, type SetStateAction } from 'react'
import { ArrowLeft, Check, CircleAlert, Plus, ShieldCheck, Trash2 } from 'lucide-react'
import { Link, useNavigate } from 'react-router'

import { useCreateStrategy, useValidateStrategy } from '@/api/queries'
import type { StrategyIndicatorDocument } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { PageHeading } from '@/components/v2_1/page-heading'
import {
  buildPersonalStrategyDocument,
  createPersonalStrategyDraft,
  MAX_CONDITIONS_PER_RULE,
  MAX_PERSONAL_RULES,
  multiplierLabel,
  PERSONAL_MULTIPLIERS,
  personalPolicyId,
  type PersonalConditionDraft,
  type PersonalRuleDraft,
  type PersonalStrategyDraft,
} from './model'

const INDICATORS: Array<{ value: StrategyIndicatorDocument['kind']; label: string; unit: string }> = [
  { value: 'price_return', label: '区间涨跌幅', unit: '%，例如 -10 表示下跌 10%' },
  { value: 'annualized_volatility', label: '年化波动率', unit: '%，例如 25 表示 25%' },
  { value: 'price_percentile', label: '历史价格位置', unit: '%，例如 20 表示处于较低 20%' },
  { value: 'moving_average_distance', label: '相对均线距离', unit: '%，负数表示低于均线' },
  { value: 'relative_strength_index', label: 'RSI 强弱指标', unit: '0–100' },
  { value: 'drawdown', label: '距阶段高点回撤', unit: '%，例如 -15 表示回撤 15%' },
  { value: 'close_price', label: '收盘价', unit: '标的交易币种' },
]

const OPERATORS = [
  { value: 'less_than', label: '低于' },
  { value: 'less_than_or_equal', label: '不高于' },
  { value: 'greater_than', label: '高于' },
  { value: 'greater_than_or_equal', label: '不低于' },
] as const

/** Consumer-facing visual builder for the restricted Formula V1 contract. */
export default function StrategyBuilderPage() {
  const navigate = useNavigate()
  const validate = useValidateStrategy()
  const create = useCreateStrategy()
  const [draft, setDraft] = useState<PersonalStrategyDraft>(createPersonalStrategyDraft)
  const [policyId] = useState(() => personalPolicyId(globalThis.crypto?.randomUUID?.() ?? String(Date.now())))
  const [error, setError] = useState<string | null>(null)

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    setError(null)
    try {
      const document = buildPersonalStrategyDocument(draft, policyId)
      const result = await validate.mutateAsync(document)
      if (!result.valid || !result.document) throw new Error(result.error ?? '这组规则未通过服务端校验')
      const saved = await create.mutateAsync(result.document)
      const search = new URLSearchParams({ strategy: saved.policy.id, strategy_version: String(saved.policy.version), view: 'plain' })
      navigate(`/strategy-center?created=${encodeURIComponent(saved.policy.id)}#personal-strategies`, { state: { analysisHref: `/strategy-analysis?${search.toString()}` } })
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '个人策略保存失败')
    }
  }

  return <main className="min-w-0 flex-1 px-5 py-8 sm:px-8 lg:px-10">
    <div className="mx-auto max-w-[74rem]">
      <Link to="/strategy-center" className="inline-flex items-center gap-2 text-sm font-medium text-[#2d6a57] hover:text-[#1f5444]"><ArrowLeft className="size-4" />返回策略中心</Link>
      <PageHeading eyebrow="我的策略工坊" title="把想法写成一条能验证的规则" description="选择指标、观察时间和触发动作。系统只接受白名单公式；不会运行代码，也不会碰你的核心投入。" />

      <form className="mt-8 grid gap-6 xl:grid-cols-[minmax(0,1fr)_20rem]" onSubmit={(event) => void submit(event)}>
        <div className="min-w-0 space-y-5">
          <section className="rounded-[1.5rem] border border-[#cfded8] bg-[#f1f7f4] p-5 sm:p-7">
            <p className="text-sm font-medium text-[#2d6a57]">先说清楚它是什么</p>
            <label className="mt-4 block text-sm font-semibold text-[#102028]">策略名称
              <Input autoFocus className="mt-2 h-12 rounded-xl border-[#bfd2c9] bg-white text-base" value={draft.name} maxLength={60} onChange={(event) => setDraft((current) => ({ ...current, name: event.target.value }))} placeholder="例如：大跌时增加机会投入" />
            </label>
            <p className="mt-2 text-xs leading-5 text-slate-500">名称只负责帮助你辨认；策略判断完全由下面的规则决定。</p>
          </section>

          {draft.rules.map((rule, ruleIndex) => <RuleSection key={ruleIndex} index={ruleIndex} rule={rule} onChange={(next) => updateRule(setDraft, ruleIndex, next)} onRemove={() => setDraft((current) => ({ ...current, rules: current.rules.filter((_, index) => index !== ruleIndex) }))} removable={draft.rules.length > 1} />)}

          {draft.rules.length < MAX_PERSONAL_RULES ? <Button type="button" variant="outline" className="h-11 rounded-full border-[#9fbfaf] bg-white px-5 text-[#245a49]" onClick={() => setDraft((current) => ({ ...current, rules: [...current.rules, createPersonalStrategyDraft().rules[0]] }))}><Plus className="mr-2 size-4" />增加下一条优先规则</Button> : null}
        </div>

        <aside className="space-y-4 xl:sticky xl:top-24 xl:self-start">
          <section className="rounded-[1.5rem] bg-[#102830] p-5 text-white shadow-[0_18px_45px_rgba(16,40,48,0.14)]">
            <ShieldCheck className="size-6 text-[#b8d5c6]" />
            <h2 className="mt-5 text-xl font-semibold tracking-[-0.03em]">系统替你守住边界</h2>
            <ul className="mt-4 space-y-3 text-sm leading-6 text-slate-200">
              <li className="flex gap-2"><Check className="mt-1 size-4 shrink-0 text-[#9bc8b2]" />最多三条优先规则，每条最多三个条件。</li>
              <li className="flex gap-2"><Check className="mt-1 size-4 shrink-0 text-[#9bc8b2]" />只能调整机会额度；计划里的核心投入保持不变。</li>
              <li className="flex gap-2"><Check className="mt-1 size-4 shrink-0 text-[#9bc8b2]" />保存后生成不可变 v1，回测、计划和历史记录引用同一版本。</li>
            </ul>
            {error ? <p role="alert" className="mt-5 rounded-xl border border-[#806f4c] bg-[#4b3820] p-3 text-sm leading-6 text-[#f0d7a4]"><CircleAlert className="mr-2 inline size-4" />{error}</p> : null}
            <Button type="submit" disabled={validate.isPending || create.isPending} className="mt-6 h-12 w-full rounded-full bg-white text-[#102830] hover:bg-[#eaf2ee]">{validate.isPending ? '正在检查规则…' : create.isPending ? '正在保存…' : '验证并放入策略中心'}</Button>
          </section>
          <p className="px-2 text-xs leading-5 text-slate-500">保存不是采用。进入策略中心后，你仍需选择真实标的回测，再建立自己的执行计划。</p>
        </aside>
      </form>
    </div>
  </main>
}

function RuleSection({ index, rule, onChange, onRemove, removable }: { index: number; rule: PersonalRuleDraft; onChange: (rule: PersonalRuleDraft) => void; onRemove: () => void; removable: boolean }) {
  return <section className="rounded-[1.5rem] border border-slate-200 bg-white p-5 sm:p-7">
    <div className="flex items-start justify-between gap-4">
      <div><p className="text-sm font-medium text-[#2d6a57]">优先规则 {index + 1}</p><h2 className="mt-1 text-xl font-semibold tracking-[-0.025em] text-[#102028]">什么时候改变本期机会额度？</h2></div>
      {removable ? <Button type="button" variant="ghost" size="icon" aria-label={`删除优先规则 ${index + 1}`} onClick={onRemove}><Trash2 className="size-4" /></Button> : null}
    </div>
    {rule.conditions.length > 1 ? <label className="mt-5 inline-flex items-center gap-2 text-sm text-slate-600">当下面条件
      <select className="h-9 rounded-lg border border-slate-200 bg-[#f7f9f8] px-2 font-medium text-[#102028]" value={rule.match} onChange={(event) => onChange({ ...rule, match: event.target.value as PersonalRuleDraft['match'] })}><option value="all">全部满足</option><option value="any">任一满足</option></select>
    </label> : null}
    <div className="mt-4 space-y-3">
      {rule.conditions.map((condition, conditionIndex) => <ConditionRow key={conditionIndex} condition={condition} onChange={(next) => onChange({ ...rule, conditions: rule.conditions.map((item, itemIndex) => itemIndex === conditionIndex ? next : item) })} onRemove={() => onChange({ ...rule, conditions: rule.conditions.filter((_, itemIndex) => itemIndex !== conditionIndex) })} removable={rule.conditions.length > 1} />)}
    </div>
    {rule.conditions.length < MAX_CONDITIONS_PER_RULE ? <Button type="button" size="sm" variant="ghost" className="mt-3 text-[#2d6a57]" onClick={() => onChange({ ...rule, conditions: [...rule.conditions, createPersonalStrategyDraft().rules[0].conditions[0]] })}><Plus className="mr-1 size-3.5" />再加一个条件</Button> : null}
    <div className="mt-6 border-t border-slate-100 pt-5">
      <label className="text-sm font-semibold text-[#102028]">条件命中时
        <select className="mt-2 h-12 w-full rounded-xl border border-[#c8d8d1] bg-[#f6f9f7] px-3 text-sm font-semibold text-[#102028]" value={rule.multiplier} onChange={(event) => onChange({ ...rule, multiplier: Number(event.target.value) as PersonalRuleDraft['multiplier'] })}>{PERSONAL_MULTIPLIERS.map((value) => <option key={value} value={value}>{multiplierLabel(value)}</option>)}</select>
      </label>
      <p className="mt-2 text-xs leading-5 text-slate-500">没有任何规则命中时，机会额度保持 100%。规则按从上到下的顺序判断，第一条命中后停止。</p>
    </div>
  </section>
}

function ConditionRow({ condition, onChange, onRemove, removable }: { condition: PersonalConditionDraft; onChange: (condition: PersonalConditionDraft) => void; onRemove: () => void; removable: boolean }) {
  const definition = INDICATORS.find((item) => item.value === condition.indicator) ?? INDICATORS[0]
  const hasWindow = condition.indicator !== 'close_price' && condition.indicator !== 'vix'
  return <div className="grid gap-3 rounded-xl bg-[#f6f8f7] p-4 md:grid-cols-[minmax(11rem,1.3fr)_7rem_8rem_8rem_auto] md:items-end">
    <label className="text-xs font-medium text-slate-500">观察什么
      <select className="mt-1 h-10 w-full rounded-lg border border-slate-200 bg-white px-2 text-sm font-medium text-[#102028]" value={condition.indicator} onChange={(event) => onChange({ ...condition, indicator: event.target.value as PersonalConditionDraft['indicator'] })}>{INDICATORS.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}</select>
    </label>
    <label className="text-xs font-medium text-slate-500">观察天数
      <Input className="mt-1 h-10 bg-white" type="number" min={2} max={2520} disabled={!hasWindow} value={hasWindow ? condition.lookbackDays : ''} onChange={(event) => onChange({ ...condition, lookbackDays: Number(event.target.value) })} />
    </label>
    <label className="text-xs font-medium text-slate-500">如何比较
      <select className="mt-1 h-10 w-full rounded-lg border border-slate-200 bg-white px-2 text-sm" value={condition.operator} onChange={(event) => onChange({ ...condition, operator: event.target.value as PersonalConditionDraft['operator'] })}>{OPERATORS.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}</select>
    </label>
    <label className="text-xs font-medium text-slate-500">触发值
      <Input className="mt-1 h-10 bg-white" inputMode="decimal" value={condition.threshold} onChange={(event) => onChange({ ...condition, threshold: event.target.value })} />
    </label>
    {removable ? <Button type="button" variant="ghost" size="icon" aria-label="删除条件" onClick={onRemove}><Trash2 className="size-4" /></Button> : <span className="size-9" />}
    <p className="text-xs text-slate-500 md:col-span-full">单位：{definition.unit}</p>
  </div>
}

function updateRule(setDraft: Dispatch<SetStateAction<PersonalStrategyDraft>>, index: number, next: PersonalRuleDraft) {
  setDraft((current) => ({ ...current, rules: current.rules.map((rule, ruleIndex) => ruleIndex === index ? next : rule) }))
}
