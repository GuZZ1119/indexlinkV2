import { BarChart3, ExternalLink, Layers3, Loader2, Plus, RotateCcw, Search, ShieldCheck, WandSparkles } from 'lucide-react'
import { useMemo, useState } from 'react'
import { Link } from 'react-router'

import { usePlans, useStrategyCatalog } from '@/api/queries'
import type { InvestmentPlan, StrategyCatalogEntry } from '@/api/types'
import { PageHeading } from '@/components/v2_1/page-heading'
import { StrategyCard } from '@/components/v2_1/strategy-card'
import { StrategyCenterNav } from '@/components/v2_1/strategy-center-nav'
import { setSelectedPlanId } from '@/stores/ui'

type StrategyFamilyGroup = {
  id: string
  name: string
  description: string
  category: string
  strategies: StrategyCatalogEntry[]
}

const ALL_CATEGORIES = '全部方法'

export default function StrategyCenterPage() {
  const plans = usePlans()
  const catalog = useStrategyCatalog()
  const activePlans = (plans.data ?? []).filter((plan) => plan.is_active)
  const [query, setQuery] = useState('')
  const [category, setCategory] = useState(ALL_CATEGORIES)
  const [selectedPolicies, setSelectedPolicies] = useState<Record<string, string>>({})
  const catalogView = useMemo(() => buildCatalogView(catalog.data ?? [], query, category), [catalog.data, query, category])

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <PageHeading
        eyebrow="策略中心"
        title="先选方法，再挑适合你的参数"
        description="固定定投是共同基准；其余策略按规则家族收拢。每组参数都直接写明观察天数，可以用同一只标的真实回测后再建立计划。"
        action={<Link to="/strategy-builder" className="inline-flex items-center gap-2 rounded-full bg-[#102830] px-4 py-2.5 text-sm font-semibold text-white shadow-[0_8px_20px_rgba(16,40,48,0.14)] hover:bg-[#1d3a43]"><WandSparkles className="size-4" />建立我的策略</Link>}
      />
      <StrategyCenterNav />

      <ActivePlansPanel plans={activePlans} strategies={catalog.data ?? []} pending={plans.isPending} failed={plans.isError} />

      <PersonalStrategies strategies={catalogView.personalStrategies} loading={catalog.isPending} />

      <section aria-labelledby="official-strategies-heading">
        <div className="mb-5 flex flex-wrap items-end justify-between gap-4">
          <div>
            <h2 id="official-strategies-heading" className="text-xl font-semibold tracking-[-0.03em] text-[#102028]">从能解释清楚的规则开始</h2>
            <p className="mt-1 max-w-3xl text-sm leading-6 text-slate-500">先理解一种方法，再选择明确的观察周期。浏览不会改变计划，只有点击“建立计划”才会进入个人配置。</p>
          </div>
          {catalog.data ? <p className="text-sm tabular-nums text-slate-500">{catalogView.totalFormulaCount} 个公式档位 · {catalogView.allFamilies.length} 个家族</p> : null}
        </div>

        {catalog.isPending ? <div className="flex min-h-48 items-center justify-center rounded-[1.35rem] border border-slate-200 bg-white text-sm text-slate-500"><Loader2 className="mr-2 size-4 animate-spin" />正在读取官方策略目录…</div> : null}
        {catalog.isError ? <div role="alert" className="rounded-[1.35rem] border border-[#d9c7a9] bg-[#fffaf1] px-5 py-6 text-sm leading-6 text-slate-700"><p className="font-medium text-[#6f511f]">暂时无法读取策略目录</p><p className="mt-1">请确认本机 Rust 服务已经更新并正在运行。页面不会用静态策略或演示收益代替真实响应。</p></div> : null}
        {catalog.data?.length === 0 ? <div className="rounded-[1.35rem] border border-dashed border-slate-300 bg-white px-5 py-8 text-sm text-slate-600">服务当前没有发布可供普通用户采用的策略。</div> : null}

        {catalogView.benchmark ? (
          <div className="mb-7">
            <div className="mb-3 flex items-center gap-2 text-sm font-medium text-[#2d6a57]"><RotateCcw className="size-4" />共同基准</div>
            <StrategyCard strategy={catalogView.benchmark} analysisHref={analysisHref(catalogView.benchmark)} variant="compact">
              <StrategyAction strategy={catalogView.benchmark} />
            </StrategyCard>
          </div>
        ) : null}

        {catalogView.allFamilies.length > 0 ? (
          <>
            <CatalogFilters query={query} category={category} categories={catalogView.categories} onQuery={setQuery} onCategory={setCategory} />
            {catalogView.families.length > 0 ? (
              <div className="mt-5 grid gap-4 lg:grid-cols-2">
                {catalogView.families.map((family) => {
                  const selected = selectedStrategy(family, selectedPolicies[family.id])
                  return (
                    <FamilyCard
                      key={family.id}
                      family={family}
                      strategy={selected}
                      onSelect={(policyId) => setSelectedPolicies((current) => ({ ...current, [family.id]: policyId }))}
                    />
                  )
                })}
              </div>
            ) : <div className="mt-5 rounded-[1.35rem] border border-dashed border-slate-300 bg-white px-5 py-8 text-sm text-slate-600">没有符合当前搜索和类别的策略。可以清空搜索或切换到“全部方法”。</div>}
          </>
        ) : null}
      </section>

      <section className="grid gap-4 md:grid-cols-2">
        <InfoBlock icon={<BarChart3 />} title="回测是一次统一体检" text="所有 Formula 策略和固定定投使用相同外部现金流、费用与成交时点。它能暴露规则的缺点，不能承诺未来收益。" />
        <InfoBlock icon={<Layers3 />} title="参数不是收益排名" text="20 日、50 日或多周期组合只代表观察窗口与阈值不同。先在自己的标的上回测，再选择你能理解和坚持的参数。" />
      </section>
    </div>
  )
}

function CatalogFilters({ query, category, categories, onQuery, onCategory }: { query: string; category: string; categories: string[]; onQuery: (value: string) => void; onCategory: (value: string) => void }) {
  return <div className="rounded-[1.25rem] border border-[#d5e2dc] bg-[#f7faf8] p-4 sm:p-5">
    <label className="relative block max-w-xl">
      <span className="sr-only">搜索策略</span>
      <Search className="pointer-events-none absolute left-4 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
      <input type="search" value={query} onChange={(event) => onQuery(event.target.value)} placeholder="搜索方法、规则或标签" className="h-11 w-full rounded-xl border border-[#c8d8d1] bg-white pl-11 pr-4 text-sm text-[#102028] outline-none transition focus:border-[#2d6a57] focus:ring-4 focus:ring-[#2d6a57]/10" />
    </label>
    <div className="mt-4 flex flex-wrap gap-2" aria-label="按策略类别筛选">
      {[ALL_CATEGORIES, ...categories].map((item) => <button key={item} type="button" aria-pressed={category === item} onClick={() => onCategory(item)} className={`rounded-full px-3.5 py-2 text-sm font-medium transition-colors ${category === item ? 'bg-[#102830] text-white' : 'border border-[#d5e2dc] bg-white text-slate-600 hover:border-[#9ebcad] hover:text-[#245a49]'}`}>{item}</button>)}
    </div>
  </div>
}

function PersonalStrategies({ strategies, loading }: { strategies: StrategyCatalogEntry[]; loading: boolean }) {
  return <section id="personal-strategies" aria-labelledby="personal-strategies-heading" className="scroll-mt-24 rounded-[1.45rem] border border-[#cddfd6] bg-[#f7fbf9] p-5 sm:p-6">
    <div className="flex flex-wrap items-end justify-between gap-4">
      <div><p className="text-sm font-medium text-[#2d6a57]">只保存在这台设备</p><h2 id="personal-strategies-heading" className="mt-1 text-xl font-semibold tracking-[-0.03em] text-[#102028]">我的个人策略</h2><p className="mt-1 max-w-2xl text-sm leading-6 text-slate-500">每张卡都是不可变版本。先用真实标的回测，再决定是否建立计划。</p></div>
      <Link to="/strategy-builder" className="inline-flex items-center gap-2 rounded-full border border-[#9fbfaf] bg-white px-4 py-2.5 text-sm font-semibold text-[#245a49] hover:border-[#78a991]"><Plus className="size-4" />新建个人策略</Link>
    </div>
    {loading ? <p className="mt-5 flex items-center gap-2 text-sm text-slate-500"><Loader2 className="size-4 animate-spin" />正在读取个人策略…</p> : strategies.length === 0 ? <div className="mt-5 rounded-xl border border-dashed border-[#b8d5c6] bg-white/70 px-4 py-5 text-sm leading-6 text-slate-600">还没有个人策略。你可以用指标、观察天数和机会额度搭出第一条规则；不需要写代码。</div> : <div className="mt-5 grid gap-4 lg:grid-cols-2">{strategies.map((strategy) => <PersonalStrategyCard key={policyKey(strategy)} strategy={strategy} />)}</div>}
  </section>
}

function PersonalStrategyCard({ strategy }: { strategy: StrategyCatalogEntry }) {
  return <article className="flex flex-col rounded-[1.2rem] border border-[#cfded8] bg-white p-5">
    <div className="flex items-start justify-between gap-3"><div><span className="inline-flex items-center gap-1.5 rounded-full bg-[#e8f2ed] px-2.5 py-1 text-xs font-medium text-[#2d6a57]"><ShieldCheck className="size-3.5" />个人策略</span><h3 className="mt-3 text-lg font-semibold tracking-[-0.025em] text-[#102028]">{strategy.name}</h3></div><span className="text-xs font-medium text-slate-400">v{strategy.policy.version}</span></div>
    <p className="mt-3 text-sm leading-6 text-slate-600">{strategy.rule}</p>
    <dl className="mt-4 grid grid-cols-2 gap-3 rounded-xl bg-[#f6f8f7] p-3 text-xs"><div><dt className="text-slate-500">所需历史</dt><dd className="mt-1 font-semibold text-[#102028]">{historyLabel(strategy)}</dd></div><div><dt className="text-slate-500">当前状态</dt><dd className="mt-1 font-semibold text-[#102028]">{strategy.adoptable ? '可回测并建计划' : '已保存，暂不可建计划'}</dd></div></dl>
    <p className="mt-3 text-xs leading-5 text-slate-500">{strategy.limitation}</p>
    <div className="mt-auto flex flex-wrap items-center gap-3 border-t border-slate-100 pt-4"><Link to={analysisHref(strategy)} className="inline-flex items-center gap-2 text-sm font-semibold text-[#245a49] underline decoration-[#aac7b9] underline-offset-4"><BarChart3 className="size-4" />用真实标的回测</Link><div className="ml-auto"><StrategyAction strategy={strategy} /></div></div>
  </article>
}

function FamilyCard({ family, strategy, onSelect }: { family: StrategyFamilyGroup; strategy: StrategyCatalogEntry; onSelect: (policyId: string) => void }) {
  return <article className="flex flex-col rounded-[1.35rem] border border-slate-200 bg-white p-5 [content-visibility:auto] [contain-intrinsic-size:420px] sm:p-6">
    <div className="flex items-start justify-between gap-3">
      <div><span className="rounded-full bg-[#e5eff4] px-2.5 py-1 text-xs font-medium text-[#294f60]">{family.category}</span><h3 className="mt-4 text-xl font-semibold tracking-[-0.03em] text-[#102028]">{family.name}</h3></div>
      <span className={`rounded-full px-2.5 py-1 text-xs font-medium ${strategy.risk === 'stable' ? 'bg-[#e8f2ed] text-[#2d6a57]' : 'bg-[#f5eee2] text-[#795a2c]'}`}>{riskLabel(strategy.risk)}</span>
    </div>
    <p className="mt-3 min-h-12 text-sm leading-6 text-slate-600">{family.description}</p>

    <label className="mt-5 block">
      <span className="text-xs font-medium text-slate-500">检查参数</span>
      <select aria-label={`${family.name}检查参数`} value={strategy.policy.id} onChange={(event) => onSelect(event.target.value)} className="mt-2 h-11 w-full rounded-xl border border-[#c8d8d1] bg-[#f8faf9] px-3 text-sm font-semibold text-[#102028] outline-none transition focus:border-[#2d6a57] focus:ring-4 focus:ring-[#2d6a57]/10">
        {family.strategies.map((item) => <option key={item.policy.id} value={item.policy.id}>{item.preset?.name ?? item.name}</option>)}
      </select>
    </label>

    <div className="mt-4 rounded-xl bg-[#f6f8f6] p-4">
      <p className="text-sm leading-6 text-slate-700"><span className="font-medium text-[#102028]">当前档位怎么做：</span>{strategy.rule}</p>
      <dl className="mt-4 grid grid-cols-2 gap-3 border-t border-slate-200/80 pt-4 text-xs">
        <div><dt className="text-slate-500">所需日线</dt><dd className="mt-1 font-semibold text-[#102028]">{historyLabel(strategy)}</dd></div>
        <div><dt className="text-slate-500">服务端校验</dt><dd className="mt-1 font-semibold text-[#102028]">{validationLabel(strategy)}</dd></div>
      </dl>
    </div>

    <p className="mt-4 text-xs leading-5 text-slate-500">{strategy.limitation}</p>
    {strategy.source ? <a href={strategy.source.url} target="_blank" rel="noreferrer" className="mt-3 inline-flex w-fit items-center gap-1.5 text-xs font-medium text-[#2d6a57] underline decoration-[#aac7b9] underline-offset-4 hover:text-[#1f5444]">参考：{strategy.source.name}<ExternalLink className="size-3" /><span className="sr-only">（在新窗口打开）</span></a> : null}

    <div className="mt-auto flex flex-col gap-4 border-t border-slate-100 pt-5 sm:flex-row sm:items-center sm:justify-between">
      <Link to={analysisHref(strategy)} className="inline-flex w-fit items-center gap-2 text-sm font-semibold text-[#245a49] underline decoration-[#aac7b9] underline-offset-4 hover:text-[#1f5444] focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-[#8eb7a3]"><BarChart3 className="size-4" />查看这组参数的分析</Link>
      <div className="sm:ml-auto"><StrategyAction strategy={strategy} /></div>
    </div>
  </article>
}

function StrategyAction({ strategy }: { strategy: StrategyCatalogEntry }) {
  if (!strategy.adoptable) return <span className="inline-flex rounded-full border border-slate-200 bg-slate-100 px-4 py-2.5 text-sm font-medium text-slate-500">尚未通过，暂不可创建</span>
  const search = new URLSearchParams({ policy_id: strategy.policy.id, policy_version: String(strategy.policy.version) })
  return <Link to={`/plans?${search.toString()}#new-plan`} className="inline-flex items-center gap-2 rounded-full bg-[#102830] px-4 py-2.5 text-sm font-semibold text-white shadow-[0_8px_20px_rgba(16,40,48,0.14)] transition-colors hover:bg-[#1d3a43] focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-[#8eb7a3]"><Plus className="size-4" />{strategy.policy.id === 'fixed_dca' ? '建立固定投入计划' : '按此参数建立计划'}</Link>
}

function buildCatalogView(entries: StrategyCatalogEntry[], query: string, selectedCategory: string) {
  const personalStrategies = entries.filter((strategy) => strategy.origin === 'personal')
  const officialEntries = entries.filter((strategy) => strategy.origin !== 'personal')
  const benchmark = officialEntries.find((strategy) => strategy.policy.id === 'fixed_dca')
  const groups = new Map<string, StrategyFamilyGroup>()
  for (const strategy of officialEntries) {
    if (strategy.policy.id === 'fixed_dca') continue
    const family = strategy.family ?? { id: strategy.policy.id, name: strategy.name, description: strategy.summary, category: '其他' }
    const group = groups.get(family.id) ?? { ...family, strategies: [] }
    group.strategies.push(strategy)
    groups.set(family.id, group)
  }
  const allFamilies = [...groups.values()].map((family) => ({ ...family, strategies: [...family.strategies].sort((left, right) => (left.preset?.order ?? 0) - (right.preset?.order ?? 0)) }))
  const normalizedQuery = query.trim().toLocaleLowerCase('zh-CN')
  const families = allFamilies.filter((family) => {
    if (selectedCategory !== ALL_CATEGORIES && family.category !== selectedCategory) return false
    if (!normalizedQuery) return true
    return family.strategies.some((strategy) => strategySearchText(strategy, family).includes(normalizedQuery))
  })
  return {
    benchmark,
    personalStrategies,
    allFamilies,
    families,
    categories: [...new Set(allFamilies.map((family) => family.category))].sort((left, right) => left.localeCompare(right, 'zh-CN')),
    totalFormulaCount: allFamilies.reduce((total, family) => total + family.strategies.length, 0),
  }
}

function policyKey(strategy: StrategyCatalogEntry): string { return `${strategy.policy.id}@${strategy.policy.version}` }

function strategySearchText(strategy: StrategyCatalogEntry, family: StrategyFamilyGroup): string {
  return [family.name, family.description, family.category, strategy.name, strategy.summary, strategy.rule, ...(strategy.tags ?? [])].join(' ').toLocaleLowerCase('zh-CN')
}

function selectedStrategy(family: StrategyFamilyGroup, selectedPolicyId?: string): StrategyCatalogEntry {
  return family.strategies.find((strategy) => strategy.policy.id === selectedPolicyId)
    ?? family.strategies.find((strategy) => strategy.preset?.id === 'balanced')
    ?? family.strategies[0]
}

function analysisHref(strategy: StrategyCatalogEntry): string {
  const search = new URLSearchParams({ strategy: strategy.policy.id, strategy_version: String(strategy.policy.version), view: 'plain' })
  return `/strategy-analysis?${search.toString()}`
}

function riskLabel(risk: StrategyCatalogEntry['risk']): string { return risk === 'stable' ? '稳健' : '平衡' }
function historyLabel(strategy: StrategyCatalogEntry): string { const count = strategy.data_requirement.required_close_observations; return count === 0 ? '不需要行情' : `至少 ${count} 个交易日` }
function validationLabel(strategy: StrategyCatalogEntry): string { if (!strategy.adoptable) return '尚未通过'; return ({ reference: '固定基准', fixed_fixture: '固定样本准入', compiled_formula: '公式与预算检查' } as Record<string, string>)[strategy.validation_mode ?? ''] ?? '可以建立计划' }

function ActivePlansPanel({ plans, strategies, pending, failed }: { plans: InvestmentPlan[]; strategies: StrategyCatalogEntry[]; pending: boolean; failed: boolean }) {
  const strategyNames = new Map(strategies.map((strategy) => [strategy.policy.id, strategy.name]))
  return <section className="rounded-[1.35rem] border border-[#cfded8] bg-[#f1f7f4] p-5 sm:p-6"><div className="flex flex-wrap items-end justify-between gap-3"><div><p className="text-sm font-medium text-[#2d6a57]">你正在坚持的计划</p><h2 className="mt-1 text-xl font-semibold tracking-[-0.03em] text-[#102028]">来自“我的计划”的真实记录</h2></div><Link to="/plans" className="text-sm font-medium text-[#2d6a57] hover:text-[#1f5444]">管理我的计划 →</Link></div>{pending ? <p className="mt-5 flex items-center gap-2 text-sm text-slate-500"><Loader2 className="size-4 animate-spin" />正在读取计划…</p> : null}{failed ? <p role="alert" className="mt-5 rounded-xl border border-[#d9c7a9] bg-white/70 px-4 py-3 text-sm text-slate-600">暂时无法读取真实计划。请确认本机服务正在运行。</p> : null}{!pending && !failed && plans.length === 0 ? <p className="mt-5 rounded-xl border border-dashed border-[#b8d5c6] bg-white/70 px-4 py-4 text-sm text-slate-600">目前没有正在执行的计划。已暂停计划和新建入口都在“我的计划”中。</p> : null}{!pending && !failed && plans.length > 0 ? <div className="mt-5 grid gap-3 md:grid-cols-2 xl:grid-cols-3">{plans.map((plan) => <Link key={plan.id} to="/plans" onClick={() => setSelectedPlanId(plan.id)} className="rounded-xl border border-[#d4e3dc] bg-white/80 p-4 transition-colors hover:border-[#8eb7a3] hover:bg-white focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-[#b8d5c6]"><div className="flex items-start justify-between gap-3"><h3 className="font-semibold text-[#102028]">{plan.name}</h3><span className="rounded-full bg-[#e6f1eb] px-2.5 py-1 text-xs font-medium text-[#2d6a57]">进行中</span></div><p className="mt-2 text-sm text-slate-600">{plan.symbol} · {planStrategyLabel(plan, strategyNames)}</p><p className="mt-3 text-xs text-slate-500">{planScheduleLabel(plan)} · 每次 {formatPlanMoney(plan)}</p></Link>)}</div> : null}</section>
}

function planStrategyLabel(plan: InvestmentPlan, strategyNames: Map<string, string>): string { if (plan.policy.id === 'core_opportunity_v1') return '旧自适应策略'; return strategyNames.get(plan.policy.id) ?? (plan.policy.id === 'fixed_dca' ? '固定定投' : '已保存的版本策略') }
function planScheduleLabel(plan: InvestmentPlan): string { return plan.schedule_kind === 'weekly' ? `每周第 ${plan.schedule_day} 天` : `每月 ${plan.schedule_day} 日` }
function formatPlanMoney(plan: InvestmentPlan): string { const amount = Number(plan.base_contribution); return Number.isFinite(amount) ? new Intl.NumberFormat('zh-CN', { style: 'currency', currency: plan.currency, maximumFractionDigits: 2 }).format(amount) : `${plan.currency} ${plan.base_contribution}` }
function InfoBlock({ icon, title, text }: { icon: React.ReactNode; title: string; text: string }) { return <div className="flex gap-4 rounded-[1.2rem] border border-slate-200 bg-white p-5"><span className="grid size-9 shrink-0 place-items-center rounded-full bg-[#e5eff4] text-[#294f60]">{icon}</span><div><h3 className="font-semibold text-[#102028]">{title}</h3><p className="mt-1.5 text-sm leading-6 text-slate-600">{text}</p></div></div> }
