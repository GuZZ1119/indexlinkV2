import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { BarChart3, Bot, ClipboardCheck, RefreshCw, Send } from 'lucide-react'
import { CartesianGrid, Line, LineChart, ReferenceDot, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import { useSnapshot } from 'valtio'

import {
  ApiRequestError,
  previewAutomaticDecision,
  setPaperOpeningBalance,
  useActualPerformance,
  useDecisionRecords,
  useHoldingPriceHistory,
  useMarketSentiment,
  useMarketSignalInput,
  usePaperPerformance,
  usePaperPortfolio,
  usePlans,
} from '@/api/queries'
import type {
  DecisionRecord,
  DecisionResult,
  DecisionPreviewResponse,
  InvestmentPlan,
  MarketSignalInput,
  ActualPerformance,
  HoldingPriceHistory,
  MarketSentimentEvidence,
  PersistedMarketSentimentSnapshot,
  PaperPortfolioSnapshot,
  PaperPerformance,
} from '@/api/types'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { actionBadgeClass, formatCurrency, formatMultiplier } from '@/lib/decision'
import { cn } from '@/lib/utils'
import { setSelectedPlanId, uiStore } from '@/stores/ui'

type OverviewDecision = {
  createdAt: string
  decision: DecisionResult
  executionStatus: 'due' | 'waiting' | 'inactive'
  plannedContribution?: string
  summary: string
  marketSentiment?: MarketSentimentEvidence
}

/** Return a unique, non-secret idempotency key for one user-confirmed paper order. */
function paperOrderKey(): string {
  const suffix = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`
  return `web-paper-${suffix}`
}

/** Render the live Decision Preview workflow backed by Rust HTTP APIs. */
export default function DashboardPage() {
  const { t } = useTranslation()
  const { data: plans = [], isPending: plansPending, error: plansError } = usePlans()
  const { selectedPlanId } = useSnapshot(uiStore)
  const queryClient = useQueryClient()
  const [submitPaperOrder, setSubmitPaperOrder] = useState(false)
  const [quantity, setQuantity] = useState('1.00')
  const [result, setResult] = useState<DecisionPreviewResponse | null>(null)
  const [pricePeriod, setPricePeriod] = useState<'3m' | '6m' | '1y' | '3y'>('1y')

  useEffect(() => {
    if (selectedPlanId === null && plans[0]) {
      setSelectedPlanId(plans[0].id)
    }
  }, [plans, selectedPlanId])

  const selectedPlan = useMemo(
    () => plans.find((plan) => plan.id === selectedPlanId) ?? null,
    [plans, selectedPlanId],
  )
  const isApprovalPlan = selectedPlan?.execution_configuration.risk_mode === 'approval'
  const { data: decisionRecords = [], error: decisionRecordsError } = useDecisionRecords(selectedPlan?.id ?? null)
  const marketSignalQuery = useMarketSignalInput(selectedPlan?.symbol ?? null)
  const marketSentimentQuery = useMarketSentiment()
  const paperPortfolioQuery = usePaperPortfolio()
  const paperPerformanceQuery = usePaperPerformance(selectedPlan?.id ?? null)
  const actualPerformanceQuery = useActualPerformance()
  const priceHistoryQuery = useHoldingPriceHistory(pricePeriod)

  const decisionMutation = useMutation({
    mutationFn: async () => {
      if (!selectedPlan) {
        throw new ApiRequestError('select a plan before running a decision')
      }
      const preview = await previewAutomaticDecision(selectedPlan.id, {
        ...(submitPaperOrder && !isApprovalPlan
          ? {
              paper_order: {
                idempotency_key: paperOrderKey(),
                side: 'buy' as const,
                order_type: 'market' as const,
                quantity,
              },
            }
          : {}),
      })
      return preview
    },
    onSuccess: async (preview) => {
      setResult(preview)
      await queryClient.invalidateQueries({ queryKey: ['decision-records', selectedPlanId] })
      await queryClient.invalidateQueries({ queryKey: ['decision-records', 'all'] })
    },
  })
  const openingBalanceMutation = useMutation({
    mutationFn: async (input: { amount: string; occurred_at: string }) => {
      if (!selectedPlan) {
        throw new ApiRequestError('select a plan before setting an opening balance')
      }
      await setPaperOpeningBalance(selectedPlan.id, input)
    },
    onSuccess: async () => { await paperPerformanceQuery.refetch() },
  })

  const coreError = decisionMutation.error
    ?? plansError
    ?? decisionRecordsError
  const hasSignalInput = marketSignalQuery.data !== undefined || result !== null
  const overviewDecision = useMemo(
    () => latestOverviewDecision(result, decisionRecords[0]),
    [decisionRecords, result],
  )

  return (
    <div className="mx-auto flex w-full max-w-[1600px] flex-col gap-4 p-4 lg:p-6">
      <DashboardOverview
        plan={selectedPlan}
        decision={overviewDecision}
        marketRefresh={marketSignalQuery.data ?? null}
        portfolio={paperPortfolioQuery.data ?? null}
        portfolioRefreshing={paperPortfolioQuery.isFetching}
        portfolioError={paperPortfolioQuery.error}
        onRefreshPortfolio={() => void paperPortfolioQuery.refetch()}
        performance={paperPerformanceQuery.data ?? null}
        performanceRefreshing={paperPerformanceQuery.isFetching || openingBalanceMutation.isPending}
        performanceError={paperPerformanceQuery.error ?? openingBalanceMutation.error}
        onRefreshPerformance={() => void paperPerformanceQuery.refetch()}
        onSetOpeningBalance={(input) => openingBalanceMutation.mutate(input)}
        actualPerformance={actualPerformanceQuery.data ?? null}
        actualRefreshing={actualPerformanceQuery.isFetching}
        actualError={actualPerformanceQuery.error}
        onRefreshActual={() => void actualPerformanceQuery.refetch()}
        priceHistory={priceHistoryQuery.data ?? null}
        priceRefreshing={priceHistoryQuery.isFetching}
        priceHistoryError={priceHistoryQuery.error}
        pricePeriod={pricePeriod}
        onPricePeriodChange={setPricePeriod}
        onRefreshPrices={() => void priceHistoryQuery.refetch()}
      />

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ClipboardCheck className="size-4 text-muted-foreground" />
            {t('live.decision.title')}
          </CardTitle>
          <CardDescription>
            {t('live.decision.description')}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-5">
          <DemoSteps
            planReady={selectedPlan !== null}
            signalReady={hasSignalInput}
            result={result}
            paperOrderRequested={submitPaperOrder && !isApprovalPlan}
          />
          <label className="grid gap-1.5 text-sm font-medium">
            {t('live.decision.plan')}
            <select
              className="h-8 rounded-lg border border-input bg-transparent px-2.5 text-sm"
              value={selectedPlanId ?? ''}
              disabled={plansPending || plans.length === 0}
              onChange={(event) => setSelectedPlanId(event.target.value || null)}
            >
              {plans.length === 0 ? (
                <option value="">{t('live.decision.createPlanFirst')}</option>
              ) : (
                plans.map((plan) => (
                  <option key={plan.id} value={plan.id}>
                    {plan.name} · {plan.symbol} · {plan.currency} {plan.base_contribution}
                  </option>
                ))
              )}
            </select>
          </label>

          {plans.length === 0 && (
            <p className="rounded-lg border border-dashed p-3 text-sm text-muted-foreground">
              {t('dashboard.extra.createPlanPrompt', { plans: t('dashboard.extra.plans') })}
            </p>
          )}

          {selectedPlan && <PlanExecutionConfig plan={selectedPlan} />}
        </CardContent>
      </Card>

      <Card className="border-primary/60 bg-primary/5 shadow-sm">
        <CardHeader>
          <div>
            <CardTitle className="flex items-center gap-2 text-primary"><RefreshCw className="size-5" />{t('live.decision.marketRefreshTitle')}</CardTitle>
            <CardDescription>{t('live.decision.marketRefreshDescription')}</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="flex flex-col items-center gap-3">
          <Button className="w-full max-w-xl" size="lg" disabled={!selectedPlan || marketSignalQuery.isFetching} onClick={() => { setResult(null); void marketSignalQuery.refetch() }}>
            <RefreshCw className={cn('size-4', marketSignalQuery.isFetching && 'animate-spin')} />
            {marketSignalQuery.isFetching ? t('live.decision.marketRefreshing') : t('live.decision.marketRefresh')}
          </Button>
          {marketSignalQuery.data && (
            <p className="text-center text-sm text-muted-foreground">
            {t('live.decision.marketRefreshed', { symbol: marketSignalQuery.data.symbol, date: marketSignalQuery.data.as_of })}
            </p>
          )}
          {marketSignalQuery.error ? <OptionalServiceError message={t('dashboard.extra.marketUnavailable')} /> : null}
        </CardContent>
      </Card>

      <Card className="border-violet-500/35 bg-violet-500/5">
        <CardHeader>
          <div>
            <CardTitle className="flex items-center gap-2"><Bot className="size-5 text-violet-700" />{t('dashboard.extra.aiTitle')}</CardTitle>
            <CardDescription>{t('dashboard.extra.aiDescription')}</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex justify-center">
            <Button className="w-full max-w-xl" variant="outline" disabled={marketSentimentQuery.isFetching} onClick={() => void marketSentimentQuery.refetch()}>
              <Bot className={cn('size-4', marketSentimentQuery.isFetching && 'animate-pulse')} />
              {marketSentimentQuery.isFetching ? t('dashboard.extra.aiLoading') : t('dashboard.extra.aiFetch')}
            </Button>
          </div>
          {marketSentimentQuery.data && <MarketSentimentCard sentiment={marketSentimentQuery.data} />}
          {marketSentimentQuery.error ? <OptionalServiceError message={t('dashboard.extra.aiUnavailable')} /> : null}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Send className="size-4 text-muted-foreground" />
            {t('live.decision.paperTitle')}
          </CardTitle>
          <CardDescription>
            {t('live.decision.paperDescription')}
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap items-end gap-4">
          {isApprovalPlan ? (
            <p className="max-w-xl text-sm text-muted-foreground">{t('dashboard.extra.approvalOnly')}</p>
          ) : (
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={submitPaperOrder}
                onChange={(event) => setSubmitPaperOrder(event.target.checked)}
              />
              {t('live.decision.submitPaper')}
            </label>
          )}
          {submitPaperOrder && !isApprovalPlan && (
            <label className="grid gap-1.5 text-sm font-medium">
              {t('live.decision.quantity')}
              <Input value={quantity} onChange={(event) => setQuantity(event.target.value)} />
            </label>
          )}
          <Button
            className="ml-auto"
            disabled={!selectedPlan || decisionMutation.isPending}
            onClick={() => decisionMutation.mutate()}
          >
            {decisionMutation.isPending ? t('live.decision.running') : t('live.decision.runAutomatic')}
          </Button>
        </CardContent>
      </Card>

      {coreError && (
        <p className="rounded-lg border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
          {requestErrorMessage(coreError, decisionMutation.error !== null)}
        </p>
      )}

      {result && <DecisionResultCard result={result} paperOrderRequested={submitPaperOrder && !isApprovalPlan} />}
    </div>
  )
}

/** Prefer the just-returned preview, then fall back to the latest persisted audit record. */
function latestOverviewDecision(
  preview: DecisionPreviewResponse | null,
  record: DecisionRecord | undefined,
): OverviewDecision | null {
  if (preview) {
    return {
      createdAt: new Date().toISOString(),
      decision: preview.decision,
      executionStatus: preview.execution.status,
      plannedContribution: preview.execution.planned_contribution,
      summary: preview.summary,
      marketSentiment: preview.market_sentiment,
    }
  }
  if (!record) {
    return null
  }
  return {
    createdAt: record.created_at,
    decision: record.decision_snapshot,
    executionStatus: record.execution_status,
    plannedContribution: record.planned_contribution,
    summary: record.summary,
    marketSentiment: evidenceFromSnapshot(record.sentiment_snapshot),
  }
}

/** Display the persisted plan configuration that the server will actually execute. */
function PlanExecutionConfig({ plan }: { plan: InvestmentPlan }) {
  const { t } = useTranslation()
  const config = plan.execution_configuration
  return (
    <div className="grid gap-3 rounded-lg border bg-muted/20 p-3 text-sm sm:grid-cols-2 lg:grid-cols-4">
      <OverviewFact label={t('dashboard.extra.coreBucket')} value={`${config.bucket_allocation.core_ratio} (${t('dashboard.extra.fixed')})`} />
      <OverviewFact label={t('dashboard.extra.opportunityBucket')} value={`${config.bucket_allocation.opportunity_ratio} (${t('dashboard.extra.strategyAdjusted')})`} />
      <OverviewFact label={t('dashboard.extra.executionMode')} value={config.risk_mode === 'approval' ? t('dashboard.extra.approval') : config.risk_mode === 'autopilot' ? t('dashboard.extra.autopilot') : t('dashboard.extra.fixedDca')} />
      <OverviewFact label={t('dashboard.extra.opportunityCash')} value={config.opportunity_cash_policy} />
    </div>
  )
}

/** Return structured evidence only when a persisted snapshot has every required field. */
function evidenceFromSnapshot(
  snapshot: PersistedMarketSentimentSnapshot | undefined,
): MarketSentimentEvidence | undefined {
  if (!snapshot || typeof snapshot.score !== 'number' || typeof snapshot.rationale !== 'string' || !Array.isArray(snapshot.warnings) || !Array.isArray(snapshot.headlines)) {
    return undefined
  }
  return {
    score: snapshot.score,
    label: snapshot.score > 0 ? 'positive' : snapshot.score < 0 ? 'negative' : 'neutral',
    rationale: snapshot.rationale,
    warnings: snapshot.warnings,
    headlines: snapshot.headlines,
    generated_at: snapshot.audit?.generated_at,
    latency_ms: snapshot.audit?.latency_ms,
    prompt_version: snapshot.audit?.prompt_version,
  }
}

/** Render a standalone Qwen result before a decision record has been generated. */
function MarketSentimentCard({ sentiment }: { sentiment: MarketSentimentEvidence }) {
  const { t } = useTranslation()
  const label = sentiment.label === 'positive' ? t('dashboard.extra.sentimentPositive') : sentiment.label === 'negative' ? t('dashboard.extra.sentimentNegative') : t('dashboard.extra.sentimentNeutral')
  return (
    <div className="space-y-3 rounded-lg border bg-background/70 p-4 text-sm">
      <div className="flex items-center justify-between gap-3">
        <span className="font-semibold">{t('dashboard.extra.sentiment')}: {label}</span>
        <Badge variant="outline">{sentiment.score.toFixed(2)}</Badge>
      </div>
      <div><p className="font-medium">{t('dashboard.extra.rationale')}</p><p className="mt-1 leading-relaxed text-muted-foreground">{sentiment.rationale}</p></div>
      {sentiment.warnings.length > 0 && (
        <div><p className="font-medium">{t('dashboard.extra.warnings')}</p><ul className="mt-1 list-disc space-y-1 pl-5 text-muted-foreground">{sentiment.warnings.map((warning) => <li key={warning}>{warning}</li>)}</ul></div>
      )}
      <div>
        <p className="font-medium">{t('dashboard.extra.headlines')}</p>
        <ul className="mt-1 space-y-1 text-muted-foreground">
          {sentiment.headlines.map((headline) => (
            <li key={`${headline.published_at}-${headline.title}`}>
              {headline.url ? <a className="underline-offset-4 hover:underline" href={headline.url} rel="noreferrer" target="_blank">{headline.title}</a> : headline.title}
              <span className="ml-2 text-xs">{formatLocalDate(headline.published_at)}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  )
}

/** Explain failures without leaking provider configuration, transport details, or credentials. */
function requestErrorMessage(error: unknown, automaticDecision: boolean): string {
  if (error instanceof ApiRequestError) {
    if (error.status === 404 && automaticDecision) {
      return '当前运行中的 Rust 服务尚未重启到自动决策版本。请停止旧的 indexlink-server 后运行 `cargo run -p indexlink-server`，再重试；本次没有写入决策存证。'
    }
    if (error.status === 404) {
      return '本机 Rust API 未提供此功能版本。请重启 indexlink-server 后重试。'
    }
    if (error.status === 426) {
      return '本机 Rust 服务仍在运行旧版响应契约。请重启 indexlink-server 后重试。'
    }
    if (error.status === 503) {
      return '所需的本机或公开数据源暂不可用。模拟账户已连接只代表资金/订单通道可读；自动决策还需要行情、估值、波动率与 Qwen（如已配置）来源。请稍后重试。'
    }
  }
  return error instanceof Error ? error.message : 'request failed'
}

/** Render the restored dashboard layout with only source-backed decision data. */
function DashboardOverview({
  plan,
  decision,
  marketRefresh,
  portfolio,
  portfolioRefreshing,
  portfolioError,
  onRefreshPortfolio,
  performance,
  performanceRefreshing,
  performanceError,
  onRefreshPerformance,
  onSetOpeningBalance,
  actualPerformance,
  actualRefreshing,
  actualError,
  onRefreshActual,
  priceHistory,
  priceRefreshing,
  priceHistoryError,
  pricePeriod,
  onPricePeriodChange,
  onRefreshPrices,
}: {
  plan: InvestmentPlan | null
  decision: OverviewDecision | null
  marketRefresh: MarketSignalInput | null
  portfolio: PaperPortfolioSnapshot | null
  portfolioRefreshing: boolean
  portfolioError: Error | null
  onRefreshPortfolio: () => void
  performance: PaperPerformance | null
  performanceRefreshing: boolean
  performanceError: Error | null
  onRefreshPerformance: () => void
  onSetOpeningBalance: (input: { amount: string; occurred_at: string }) => void
  actualPerformance: ActualPerformance | null
  actualRefreshing: boolean
  actualError: Error | null
  onRefreshActual: () => void
  priceHistory: HoldingPriceHistory[] | null
  priceRefreshing: boolean
  priceHistoryError: Error | null
  pricePeriod: '3m' | '6m' | '1y' | '3y'
  onPricePeriodChange: (period: '3m' | '6m' | '1y' | '3y') => void
  onRefreshPrices: () => void
}) {
  const { t } = useTranslation()
  const currency = plan?.currency ?? 'USD'
  const signalValues: Array<[string, number]> = marketRefresh
    ? [
        [t('dashboard.valuation.metrics.cape'), marketRefresh.fundamental.cape_current],
        [t('dashboard.valuation.metrics.erp'), marketRefresh.fundamental.erp_current],
        [t('dashboard.valuation.metrics.ma200'), marketRefresh.trend.ma_distance_current],
        [t('dashboard.valuation.metrics.rsi'), marketRefresh.trend.rsi_current],
        [t('dashboard.valuation.metrics.vix'), marketRefresh.trend.vix_current],
      ]
    : []
  const decisionScore = decision ? toScore(decision.decision.final_score) : null
  const action = decision?.decision.action

  return (
    <section className="space-y-4" aria-label={t('dashboard.overview.title')}>
      <Card className="border-primary/30 bg-linear-to-br from-primary/8 via-card to-card shadow-sm">
        <CardHeader className="gap-4 lg:grid-cols-[minmax(0,1fr)_auto]">
          <div>
            <CardTitle className="flex items-center gap-2 text-lg">
              <BarChart3 className="size-5 text-primary" />
              {t('dashboard.overview.title')}
            </CardTitle>
            <CardDescription>{t('dashboard.overview.description')}</CardDescription>
          </div>
          {plan ? (
            <div className="rounded-lg border bg-background/80 px-3 py-2 text-sm">
              <span className="font-semibold">{plan.symbol}</span>
              <span className="ml-2 text-muted-foreground">{plan.name}</span>
            </div>
          ) : (
            <div className="rounded-lg border border-dashed px-3 py-2 text-sm text-muted-foreground">
              {t('dashboard.overview.selectPlan')}
            </div>
          )}
        </CardHeader>
        <CardContent className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
          <div>
            <div className="text-4xl font-semibold tracking-tight">
              {decisionScore === null ? '—' : `${decisionScore} / 100`}
            </div>
            <p className="mt-1 text-sm text-muted-foreground">
              {decision
                ? t('dashboard.overview.latestScore', { date: formatLocalDate(decision.createdAt) })
                : t('dashboard.overview.awaitingDecision')}
            </p>
          </div>
          <div className="grid grid-cols-2 gap-x-8 gap-y-3 text-sm sm:grid-cols-4">
            <OverviewFact
              label={t('dashboard.valuation.suggestedAction')}
              value={action ? <Badge className={actionBadgeClass[action]}>{t(`action.${action}`)}</Badge> : '—'}
            />
            <OverviewFact
              label={t('dashboard.valuation.multiplier')}
              value={decision ? formatMultiplier(decision.decision.multiplier) : '—'}
            />
            <OverviewFact
              label={t('dashboard.valuation.expectedAmount')}
              value={decision?.plannedContribution
                ? formatCurrency(Number(decision.plannedContribution), currency)
                : '—'}
            />
            <OverviewFact
              label={t('dashboard.overview.schedule')}
              value={plan ? t('dashboard.overview.monthlyDay', { day: plan.schedule_day }) : '—'}
            />
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,2fr)_minmax(320px,1fr)]">
        <div className="space-y-4">
          <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            <ScoreCard
              title={t('dashboard.scores.fundamental')}
              weight={70}
              value={decision ? toScore(decision.decision.fundamental_score) : null}
            />
            <ScoreCard
              title={t('dashboard.scores.trend')}
              weight={20}
              value={decision ? toScore(decision.decision.trend_score) : null}
            />
            <ScoreCard
              title={t('dashboard.scores.sentiment')}
              weight={10}
              value={typeof decision?.decision.sentiment_score === 'number'
                ? toScore(decision.decision.sentiment_score)
                : null}
            />
            <ScoreCard
              title={t('dashboard.scores.composite')}
              weight={null}
              value={decision ? toScore(decision.decision.final_score) : null}
              emphasize
            />
          </div>

          <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            <PortfolioMetric
              label={t('dashboard.portfolio.totalAssets')}
              value={portfolio ? formatCurrency(Number(portfolio.total_assets), portfolio.currency) : null}
            />
            <PortfolioMetric
              label={t('dashboard.portfolio.cash')}
              value={portfolio ? formatCurrency(Number(portfolio.cash), portfolio.currency) : null}
            />
            <PortfolioMetric
              label={t('dashboard.portfolio.positionPnl')}
              value={portfolio
                ? formatCurrency(sumPositionPnl(portfolio), portfolio.currency)
                : null}
            />
            <PortfolioMetric
              label={t('dashboard.portfolio.marketValue')}
              value={portfolio ? formatCurrency(Number(portfolio.market_value), portfolio.currency) : null}
            />
          </div>

          <Card className="min-h-80">
            <CardHeader>
              <CardTitle>{t('dashboard.chart.title')}</CardTitle>
              <CardDescription>{t('dashboard.chart.subtitle')}</CardDescription>
            </CardHeader>
            <CardContent>
              {performance?.points.length ? <PerformanceChart performance={performance} /> : (
                <div className="flex min-h-56 items-center justify-center rounded-lg border border-dashed bg-muted/20">
                  <div className="max-w-md space-y-2 px-6 text-center">
                    <p className="font-medium">{t('dashboard.emptyPerformance.title')}</p>
                    <p className="text-sm leading-relaxed text-muted-foreground">{t('dashboard.emptyPerformance.description')}</p>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </div>

        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>{t('dashboard.latest.title')}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {decision ? (
                <>
                  <div className="flex items-center justify-between gap-3">
                    <span className="font-semibold">{plan?.symbol ?? '—'}</span>
                    {action && <Badge className={actionBadgeClass[action]}>{t(`action.${action}`)}</Badge>}
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-sm">
                    <OverviewFact label={t('dashboard.latest.amount')} value={decision.plannedContribution
                      ? formatCurrency(Number(decision.plannedContribution), currency)
                      : '—'} />
                    <OverviewFact label={t('dashboard.latest.multiplier')} value={formatMultiplier(decision.decision.multiplier)} />
                  </div>
                  <DecisionExplanation decision={decision.decision} marketSentiment={decision.marketSentiment} />
                </>
              ) : (
                <EmptyState text={t('dashboard.latest.empty')} />
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>{t('dashboard.risk.title')}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {decision ? <RiskNotices decision={decision} /> : <EmptyState text={t('dashboard.risk.empty')} />}
            </CardContent>
          </Card>
        </div>
      </div>

      <Card className="border-dashed bg-muted/20">
        <CardHeader>
          <CardTitle>{t('dashboard.marketSnapshot.title')}</CardTitle>
          <CardDescription>{t('dashboard.marketSnapshot.description')}</CardDescription>
        </CardHeader>
        <CardContent>
          {marketRefresh ? (
            <div className="space-y-4">
              <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
                {signalValues.map(([label, value]) => (
                  <OverviewFact key={label} label={label} value={Number(value).toFixed(2)} />
                ))}
              </div>
              <div className="grid gap-2 rounded-lg border bg-muted/20 p-3 text-sm text-muted-foreground sm:grid-cols-3">
                <span>{t('dashboard.marketSnapshot.priceSource')}: {marketRefresh.sources.price}</span>
                <span>{t('dashboard.marketSnapshot.fundamentalSource')}: {marketRefresh.sources.fundamental}</span>
                <span>{t('dashboard.marketSnapshot.volatilitySource')}: {marketRefresh.sources.volatility}</span>
              </div>
            </div>
          ) : (
            <EmptyState text={t('dashboard.marketSnapshot.empty')} />
          )}
        </CardContent>
      </Card>

      <Card className="border-slate-200 bg-white/95">
        <CardHeader>
          <div>
            <CardTitle>{t('dashboard.portfolio.title')}</CardTitle>
            <CardDescription>{t('dashboard.portfolio.description')}</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex justify-center">
          <Button className="w-full max-w-md" size="lg" disabled={portfolioRefreshing} onClick={onRefreshPortfolio}>
            <RefreshCw className={cn('size-4', portfolioRefreshing && 'animate-spin')} />
            {portfolioRefreshing ? t('dashboard.portfolio.refreshing') : t('dashboard.portfolio.refresh')}
          </Button>
          </div>
          {portfolioError ? <OptionalServiceError message={t('dashboard.extra.portfolioUnavailable')} /> : null}
          {portfolio ? <PaperPortfolioDetails portfolio={portfolio} /> : <EmptyState text={t('dashboard.portfolio.empty')} />}
        </CardContent>
      </Card>

      <PaperPerformanceDetails
        performance={performance}
        refreshing={performanceRefreshing}
        error={performanceError}
        currency={currency}
        onRefresh={onRefreshPerformance}
        onSetOpeningBalance={onSetOpeningBalance}
      />

      <Card className="border-sky-200/80 bg-sky-50/45">
        <CardHeader>
          <div><CardTitle>{t('dashboard.extra.actualTitle')}</CardTitle><CardDescription>{t('dashboard.extra.actualDescription')}</CardDescription></div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex justify-center"><Button className="w-full max-w-md" variant="outline" disabled={actualRefreshing} onClick={onRefreshActual}><RefreshCw className={cn('size-4', actualRefreshing && 'animate-spin')} />{t('dashboard.extra.actualRefresh')}</Button></div>
          {actualError ? <OptionalServiceError message={t('dashboard.extra.actualUnavailable')} /> : null}
          {actualPerformance?.total_points.length ? <ActualPerformanceChart performance={actualPerformance} /> : <EmptyState text={t('dashboard.extra.actualEmpty')} />}
        </CardContent>
      </Card>

      <Card className="border-blue-200/80 bg-blue-50/45">
        <CardHeader>
          <div><CardTitle>{t('dashboard.extra.pricesTitle')}</CardTitle><CardDescription>{t('dashboard.extra.pricesDescription')}</CardDescription></div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap justify-center gap-2"><select className="h-9 rounded-md border bg-background px-2 text-sm" value={pricePeriod} onChange={(event) => onPricePeriodChange(event.target.value as '3m' | '6m' | '1y' | '3y')}><option value="3m">{t('dashboard.extra.period3m')}</option><option value="6m">{t('dashboard.extra.period6m')}</option><option value="1y">{t('dashboard.extra.period1y')}</option><option value="3y">{t('dashboard.extra.period3y')}</option></select><Button className="min-w-32" variant="outline" disabled={priceRefreshing} onClick={onRefreshPrices}><RefreshCw className={cn('size-4', priceRefreshing && 'animate-spin')} />{t('dashboard.extra.pricesRefresh')}</Button></div>
          {priceHistory?.some((item) => item.prices.length) ? <HoldingPriceChart holdings={priceHistory} /> : <EmptyState text={priceHistoryError ? requestErrorMessage(priceHistoryError, false) : t('dashboard.extra.pricesEmpty')} />}
        </CardContent>
      </Card>

    </section>
  )
}

/** Render locally persisted adaptive-versus-plain-DCA values without synthesizing points. */
function PerformanceChart({ performance }: { performance: PaperPerformance }) {
  const { t } = useTranslation()
  const data = performance.points.map((point) => ({
    date: formatLocalDate(point.observed_at),
    adaptive: Number(point.adaptive_value),
    plain: Number(point.plain_dca_value),
  }))
  return (
    <div className="h-64">
      <ResponsiveContainer width="100%" height="100%">
      <LineChart data={data} margin={{ top: 12, right: 16, left: 0, bottom: 0 }}>
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis dataKey="date" tickLine={false} axisLine={false} minTickGap={32} />
        <YAxis tickLine={false} axisLine={false} width={64} />
        <Tooltip formatter={(value) => (typeof value === 'number' ? formatCurrency(value, performance.currency) : '—')} />
        <Line type="monotone" dataKey="adaptive" name={t('dashboard.performance.adaptive')} stroke="#16a34a" strokeWidth={2} dot={false} />
        <Line type="monotone" dataKey="plain" name={t('dashboard.performance.plain')} stroke="#64748b" strokeWidth={2} dot={false} />
      </LineChart>
      </ResponsiveContainer>
    </div>
  )
}

/** Render every locally tracked holding and the explicit total on one real paper-performance chart. */
function ActualPerformanceChart({ performance }: { performance: ActualPerformance }) {
  const { t } = useTranslation()
  const data = useMemo(() => {
    const rows = new Map<string, Record<string, string | number>>()
    for (const point of performance.total_points) {
      rows.set(point.observed_at.slice(0, 10), { date: formatLocalDate(point.observed_at), total: Number(point.adaptive_value) })
    }
    for (const series of performance.series) {
      for (const point of series.points) {
        const key = point.observed_at.slice(0, 10)
        const row = rows.get(key) ?? { date: formatLocalDate(point.observed_at) }
        row[series.plan_id] = Number(point.adaptive_value)
        rows.set(key, row)
      }
    }
    return [...rows.values()]
  }, [performance])
  return <div className="h-80"><ResponsiveContainer width="100%" height="100%"><LineChart data={data} margin={{ top: 12, right: 16, left: 0, bottom: 0 }}><CartesianGrid strokeDasharray="3 3" /><XAxis dataKey="date" tickLine={false} axisLine={false} minTickGap={32} /><YAxis tickLine={false} axisLine={false} width={72} /><Tooltip formatter={(value) => typeof value === 'number' ? formatCurrency(value, performance.currency) : '—'} /><Line type="monotone" dataKey="total" name={t('dashboard.extra.totalHoldings')} stroke="#111827" strokeWidth={3} dot={false} />{performance.series.map((series, index) => <Line key={series.plan_id} type="monotone" dataKey={series.plan_id} name={`${series.name} · ${series.symbol}`} stroke={chartColor(index)} strokeWidth={2} dot={false} connectNulls />)}</LineChart></ResponsiveContainer><p className="mt-2 text-xs text-muted-foreground">{t('dashboard.extra.totalLine')}</p></div>
}

/** Render one normalized multi-symbol OpenD price chart and place only locally verified fills. */
function HoldingPriceChart({ holdings }: { holdings: HoldingPriceHistory[] }) {
  const { t } = useTranslation()
  const { data, keys, markers } = useMemo(() => {
    const rows = new Map<string, Record<string, string | number>>()
    const markers: Array<{ key: string; date: string; value: number; side: 'buy' | 'sell' }> = []
    const keys = holdings.filter((holding) => holding.prices.length > 0).map((holding) => holding.plan_id)
    for (const holding of holdings) {
      const base = holding.prices[0]?.close
      if (!base) continue
      for (const point of holding.prices) {
        const row = rows.get(point.date) ?? { date: point.date }
        row[holding.plan_id] = point.close / base * 100
        rows.set(point.date, row)
      }
      for (const trade of holding.trades) {
        const date = trade.observed_at.slice(0, 10)
        markers.push({ key: holding.plan_id, date, value: Number(trade.price) / base * 100, side: trade.side })
      }
    }
    return { data: [...rows.values()], keys, markers }
  }, [holdings])
  return <div className="h-80"><ResponsiveContainer width="100%" height="100%"><LineChart data={data} margin={{ top: 12, right: 16, left: 0, bottom: 0 }}><CartesianGrid strokeDasharray="3 3" /><XAxis dataKey="date" tickLine={false} axisLine={false} minTickGap={42} /><YAxis domain={['auto', 'auto']} tickLine={false} axisLine={false} width={56} tickFormatter={(value) => `${Number(value).toFixed(0)}`} /><Tooltip formatter={(value) => typeof value === 'number' ? `${value.toFixed(2)} (${t('dashboard.extra.startAt100')})` : '—'} />{keys.map((key, index) => { const holding = holdings.find((item) => item.plan_id === key); return <Line key={key} type="monotone" dataKey={key} name={`${holding?.symbol ?? key} ${t('dashboard.extra.normalized')}`} stroke={chartColor(index)} strokeWidth={2} dot={false} connectNulls /> })}{markers.map((marker, index) => <ReferenceDot key={`${marker.key}-${marker.date}-${index}`} x={marker.date} y={marker.value} r={5} fill={marker.side === 'buy' ? '#16a34a' : '#dc2626'} stroke="white" />)}</LineChart></ResponsiveContainer><p className="mt-2 text-xs text-muted-foreground">{t('dashboard.extra.normalizedHint')}</p></div>
}

/** Return a stable contrast-friendly series colour without persisting UI state. */
function chartColor(index: number): string {
  return ['#2563eb', '#16a34a', '#d97706', '#9333ea', '#dc2626', '#0891b2'][index % 6]
}

/** Render local ledger status, return summary, and the explicit opening-balance setup. */
function PaperPerformanceDetails({ performance, refreshing, error, currency, onRefresh, onSetOpeningBalance }: {
  performance: PaperPerformance | null
  refreshing: boolean
  error: Error | null
  currency: string
  onRefresh: () => void
  onSetOpeningBalance: (input: { amount: string; occurred_at: string }) => void
}) {
  const { t } = useTranslation()
  const [amount, setAmount] = useState('')
  return (
    <Card className="border-slate-200 bg-slate-50/55">
      <CardHeader className="gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div><CardTitle>{t('dashboard.performance.title')}</CardTitle><CardDescription>{t('dashboard.performance.description')}</CardDescription></div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex justify-center"><Button className="w-full max-w-md" disabled={refreshing} onClick={onRefresh}><RefreshCw className={cn('size-4', refreshing && 'animate-spin')} />{t('dashboard.performance.refresh')}</Button></div>
        {error ? <OptionalServiceError message={t('dashboard.extra.performanceUnavailable')} /> : null}
        {!performance?.has_opening_balance && (
          <form className="flex flex-wrap items-end gap-3 rounded-lg border border-dashed p-3" onSubmit={(event) => { event.preventDefault(); onSetOpeningBalance({ amount, occurred_at: new Date().toISOString() }) }}>
            <label className="grid gap-1 text-sm font-medium"><span>{t('dashboard.performance.openingBalance')}</span><Input required inputMode="decimal" value={amount} onChange={(event) => setAmount(event.target.value)} placeholder="10000.00" /></label>
            <Button type="submit" disabled={!amount || refreshing}>{t('dashboard.performance.saveBaseline')}</Button>
          </form>
        )}
        {performance ? <div className="space-y-3">
          {!performance.data_complete && <p className="rounded-lg border border-amber-400/50 bg-amber-100/50 p-3 text-sm text-amber-900 dark:bg-amber-950/20 dark:text-amber-200">{t('dashboard.performance.incomplete')}</p>}
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <OverviewFact label={t('dashboard.performance.netContributions')} value={formatCurrency(Number(performance.net_contributions), currency)} />
            <OverviewFact label={t('dashboard.performance.totalReturn')} value={performance.total_return === undefined ? '—' : formatCurrency(Number(performance.total_return), currency)} />
            <OverviewFact label={t('dashboard.performance.realized')} value={formatCurrency(Number(performance.realized_pnl), currency)} />
            <OverviewFact label={t('dashboard.performance.unrealized')} value={formatCurrency(Number(performance.unrealized_pnl), currency)} />
          </div>
        </div> : <EmptyState text={t('dashboard.performance.empty')} />}
      </CardContent>
    </Card>
  )
}

/** Keep one optional provider failure inside the task that requested it. */
function OptionalServiceError({ message }: { message: string }) {
  return <p role="status" className="rounded-lg border border-amber-300/70 bg-amber-50 p-3 text-sm leading-6 text-amber-900">{message}</p>
}

/** Render a small label-value fact without claiming unavailable data is zero. */
function OverviewFact({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground">{label}</p>
      <div className="mt-1 min-h-5 text-sm font-semibold">{value}</div>
    </div>
  )
}

/** Explain the score-derived decision and show the structured Qwen evidence when available. */
function DecisionExplanation({
  decision,
  marketSentiment,
}: {
  decision: DecisionResult
  marketSentiment?: MarketSentimentEvidence
}) {
  const { t } = useTranslation()
  const sentiment = decision.sentiment_score
  return (
    <div className="space-y-3 border-t pt-3 text-sm">
      <p className="font-semibold">{t('dashboard.decisionExplanation.title')}</p>
      <div className="grid gap-2 rounded-lg bg-muted/30 p-3 sm:grid-cols-3">
        <ExplanationItem
          label={t('dashboard.decisionExplanation.fundamental')}
          value={typeof decision.fundamental_score === 'number'
            ? t('dashboard.decisionExplanation.scoreBand', { score: decision.fundamental_score.toFixed(2), band: scoreBand(t, decision.fundamental_score) })
            : t('dashboard.extra.fixedSignalUnused')}
        />
        <ExplanationItem
          label={t('dashboard.decisionExplanation.trend')}
          value={typeof decision.trend_score === 'number'
            ? t('dashboard.decisionExplanation.scoreBand', { score: decision.trend_score.toFixed(2), band: scoreBand(t, decision.trend_score) })
            : t('dashboard.extra.fixedSignalUnused')}
        />
        <ExplanationItem
          label={t('dashboard.decisionExplanation.ai')}
          value={typeof sentiment === 'number'
            ? t('dashboard.decisionExplanation.aiAvailable', { score: sentiment.toFixed(2) })
            : t('dashboard.decisionExplanation.aiUnavailable')}
        />
      </div>
      <p className="leading-relaxed text-muted-foreground">
        {t('dashboard.decisionExplanation.result', {
          action: t(`action.${decision.action}`),
          multiplier: formatMultiplier(decision.multiplier),
        })}
      </p>
      {marketSentiment && (
        <div className="space-y-3 rounded-lg border bg-muted/20 p-3">
          <div>
            <p className="font-medium">{t('dashboard.decisionExplanation.aiRationale')}</p>
            <p className="mt-1 leading-relaxed text-muted-foreground">{marketSentiment.rationale}</p>
          </div>
          {marketSentiment.warnings.length > 0 && (
            <div>
              <p className="font-medium">{t('dashboard.decisionExplanation.aiWarnings')}</p>
              <ul className="mt-1 list-disc space-y-1 pl-5 text-muted-foreground">
                {marketSentiment.warnings.map((warning) => <li key={warning}>{warning}</li>)}
              </ul>
            </div>
          )}
          <div>
            <p className="font-medium">{t('dashboard.decisionExplanation.aiHeadlines')}</p>
            <ul className="mt-1 space-y-1 text-muted-foreground">
              {marketSentiment.headlines.map((headline) => (
                <li key={`${headline.published_at}-${headline.title}`}>
                  {headline.url ? (
                    <a className="underline-offset-4 hover:underline" href={headline.url} rel="noreferrer" target="_blank">
                      {headline.title}
                    </a>
                  ) : headline.title}
                  <span className="ml-2 text-xs">{formatLocalDate(headline.published_at)}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}
      {typeof sentiment === 'number' && !marketSentiment && (
        <p className="text-xs text-muted-foreground">{t('dashboard.decisionExplanation.aiLegacySource')}</p>
      )}
    </div>
  )
}

/** Render one concise, source-backed explanation field. */
function ExplanationItem({ label, value }: { label: string; value: string }) {
  return <div><p className="text-xs text-muted-foreground">{label}</p><p className="mt-1 font-medium">{value}</p></div>
}

/** Convert a bounded decision score into a deliberately coarse presentation band. */
function scoreBand(t: ReturnType<typeof useTranslation>['t'], score: number): string {
  if (score <= 0.33) return t('dashboard.decisionExplanation.cautious')
  if (score >= 0.67) return t('dashboard.decisionExplanation.supportive')
  return t('dashboard.decisionExplanation.neutral')
}

/** Render a 70/20/10 decision score sourced from the latest decision record. */
function ScoreCard({
  title,
  weight,
  value,
  emphasize = false,
}: {
  title: string
  weight: number | null
  value: number | null
  emphasize?: boolean
}) {
  const { t } = useTranslation()
  const progress = value ?? 0
  return (
    <Card className={emphasize ? 'border-primary/40 bg-primary/5' : undefined} size="sm">
      <CardHeader className="flex-row items-center justify-between gap-2">
        <CardTitle>{title}</CardTitle>
        {weight !== null && <CardDescription>{t('dashboard.scores.weight', { value: weight })}</CardDescription>}
      </CardHeader>
      <CardContent>
        <div className="text-3xl font-semibold">{value === null ? '—' : value}<span className="ml-1 text-base font-normal text-muted-foreground">/ 100</span></div>
        <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-muted">
          <div className="h-full rounded-full bg-foreground transition-all" style={{ width: `${progress}%` }} />
        </div>
      </CardContent>
    </Card>
  )
}

/** Render a financial metric that awaits real fills and portfolio accounting. */
function PortfolioMetric({ label, value }: { label: string; value: string | null }) {
  const { t } = useTranslation()
  return (
    <Card size="sm">
      <CardHeader><CardTitle>{label}</CardTitle></CardHeader>
      <CardContent>
        <div className="text-3xl font-semibold">{value ?? '—'}</div>
        {value === null && <p className="mt-2 text-sm text-muted-foreground">{t('dashboard.unavailable.awaitingFill')}</p>}
      </CardContent>
    </Card>
  )
}

/** Render the provider-backed paper positions and recent order states. */
function PaperPortfolioDetails({ portfolio }: { portfolio: PaperPortfolioSnapshot }) {
  const { t } = useTranslation()
  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <div>
        <h3 className="mb-2 text-sm font-semibold">{t('dashboard.portfolio.positions')}</h3>
        {portfolio.positions.length === 0 ? <EmptyState text={t('dashboard.portfolio.noPositions')} /> : (
          <div className="space-y-2">
            {portfolio.positions.map((position) => (
              <div key={position.symbol} className="grid grid-cols-2 gap-2 rounded-lg border p-3 text-sm sm:grid-cols-4">
                <span className="font-semibold">{position.symbol}</span>
                <span>{position.quantity} {t('dashboard.portfolio.shares')}</span>
                <span>{formatCurrency(Number(position.market_value), portfolio.currency)}</span>
                <span className={Number(position.unrealized_pnl) < 0 ? 'text-destructive' : 'text-semantic-positive'}>
                  {formatCurrency(Number(position.unrealized_pnl), portfolio.currency)}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
      <div>
        <h3 className="mb-2 text-sm font-semibold">{t('dashboard.portfolio.orders')}</h3>
        {portfolio.orders.length === 0 ? <EmptyState text={t('dashboard.portfolio.noOrders')} /> : (
          <div className="space-y-2">
            {portfolio.orders.slice(0, 8).map((order) => (
              <div key={order.order_id} className="grid grid-cols-2 gap-2 rounded-lg border p-3 text-sm sm:grid-cols-4">
                <span className="font-semibold">{order.symbol}</span>
                <span>{t(`dashboard.portfolio.side.${order.side}`)}</span>
                <span>{order.filled_quantity} / {order.quantity}</span>
                <span className="font-mono text-xs text-muted-foreground">{t(`dashboard.portfolio.state.${order.state}`)}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

/** Sum OpenD's position-level unrealized P&L without inventing realized P&L. */
function sumPositionPnl(portfolio: PaperPortfolioSnapshot): number {
  return portfolio.positions.reduce((total, position) => total + Number(position.unrealized_pnl), 0)
}

/** Render a clear empty-state instead of silently substituting fictional data. */
function EmptyState({ text }: { text: string }) {
  return <p className="rounded-lg border border-dashed bg-muted/20 p-3 text-sm text-muted-foreground">{text}</p>
}

/** Derive safe, user-facing notices only from the latest server decision. */
function RiskNotices({ decision }: { decision: OverviewDecision }) {
  const { t } = useTranslation()
  const notices = [
    ...(decision.decision.action === 'underweight' || decision.decision.action === 'skip' || decision.decision.action === 'tactical_delay'
      ? [t('dashboard.risk.reduced')]
      : []),
    t('dashboard.risk.percentile'),
    ...(typeof decision.decision.sentiment_score !== 'number' ? [t('dashboard.risk.sentimentUnavailable')] : []),
  ]
  return (
    <ul className="space-y-2">
      {notices.map((notice) => (
        <li key={notice} className="rounded-lg border bg-muted/20 p-3 text-sm leading-relaxed text-muted-foreground">
          {notice}
        </li>
      ))}
    </ul>
  )
}

/** Convert a bounded engine score into the dashboard's 0–100 presentation. */
function toScore(value: number | null | undefined): number | null {
  return typeof value === 'number' ? Math.round(value * 100) : null
}

/** Render a nullable policy-layer score without implying that a fixed policy scored zero. */
function formatOptionalScore(value: number | null | undefined): string {
  return typeof value === 'number' ? value.toFixed(2) : '—'
}

/** Format a persisted UTC timestamp for a local overview label. */
function formatLocalDate(value: string): string {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleDateString()
}

/** Render the five observable stages of the local demonstration workflow. */
function DemoSteps({
  planReady,
  signalReady,
  result,
  paperOrderRequested,
}: {
  planReady: boolean
  signalReady: boolean
  result: DecisionPreviewResponse | null
  paperOrderRequested: boolean
}) {
  const { t } = useTranslation()
  const steps = [
    [t('live.demoSteps.plan'), planReady],
    [t('live.demoSteps.signals'), signalReady],
    [t('live.demoSteps.decision'), result !== null],
    [t('live.demoSteps.buckets'), result?.execution.bucket_split !== undefined],
    [t('live.demoSteps.paper'), paperOrderRequested ? result?.paper_order_ack !== undefined : false],
  ] as const
  return (
    <ol className="grid gap-2 sm:grid-cols-2 lg:grid-cols-5">
      {steps.map(([label, complete], index) => (
        <li
          key={label}
          className={cn(
            'rounded-lg border px-3 py-2 text-xs font-medium',
            complete ? 'border-semantic-positive/40 bg-semantic-positive/10' : 'bg-muted/40 text-muted-foreground',
          )}
        >
          {index + 1}. {label}
        </li>
      ))}
    </ol>
  )
}

/** Render a non-fabricated Decision Preview response and paper-order outcome. */
function DecisionResultCard({
  result,
  paperOrderRequested,
}: {
  result: DecisionPreviewResponse
  paperOrderRequested: boolean
}) {
  const { t } = useTranslation()
  const planned = result.execution.planned_contribution
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Bot className="size-4 text-muted-foreground" />
          {t('live.decision.result')}
          <Badge className={cn(actionBadgeClass[result.decision.action])}>
            {result.decision.action}
          </Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <Metric label={t('live.decision.execution')} value={result.execution.status} />
          <Metric label={t('live.decision.finalScore')} value={formatOptionalScore(result.decision.final_score)} />
          <Metric label={t('live.decision.multiplier')} value={formatMultiplier(result.decision.multiplier)} />
          <Metric label={t('live.decision.weightMode')} value={result.decision.weight_mode ?? 'fixed_dca'} />
          <Metric
            label={t('live.decision.plannedContribution')}
            value={planned ? formatCurrency(Number(planned), result.execution.currency) : '—'}
          />
          <Metric label={t('live.decision.fundamentalScore')} value={formatOptionalScore(result.decision.fundamental_score)} />
          <Metric label={t('live.decision.trendScore')} value={formatOptionalScore(result.decision.trend_score)} />
          <Metric
            label={t('live.decision.qwenSentiment')}
            value={
              typeof result.decision.sentiment_score === 'number'
                ? `${result.decision.sentiment_score.toFixed(2)} · ${t('live.decision.qwenAvailable')}`
                : t('live.decision.fallback')
            }
          />
        </div>
        {result.execution.bucket_split && (
          <div className="grid gap-3 rounded-lg border bg-muted/30 p-3 text-sm sm:grid-cols-2 lg:grid-cols-4">
            <Metric label={t('dashboard.extra.coreFixed')} value={formatCurrency(Number(result.execution.bucket_split.core_contribution), result.execution.currency)} />
            <Metric label={t('dashboard.extra.opportunitySuggested')} value={formatCurrency(Number(result.execution.bucket_split.opportunity_contribution), result.execution.currency)} />
            <Metric label={t('dashboard.extra.opportunityMultiplier')} value={`${result.execution.bucket_split.opportunity_multiplier}x`} />
            <Metric label={t('dashboard.extra.recommendedTotal')} value={formatCurrency(Number(result.execution.bucket_split.recommended_contribution), result.execution.currency)} />
            <Metric label={t('dashboard.extra.carriedCash')} value={formatCurrency(Number(result.execution.bucket_split.carried_opportunity_cash), result.execution.currency)} />
            <Metric label={t('dashboard.extra.unallocatedCash')} value={formatCurrency(Number(result.execution.bucket_split.unallocated_opportunity_contribution), result.execution.currency)} />
            <Metric label={t('dashboard.extra.cashPolicy')} value={result.execution.bucket_split.opportunity_cash_policy} />
            <Metric label={t('dashboard.extra.authorization')} value={result.execution.bucket_split.requires_approval ? t('dashboard.extra.pendingApproval') : t('dashboard.extra.automaticSubmit')} />
          </div>
        )}
        <DecisionExplanation decision={result.decision} marketSentiment={result.market_sentiment} />
        <a
          className="inline-flex text-sm font-medium text-primary underline-offset-4 hover:underline"
          href={`/decisions/${result.audit_record_id}`}
        >
          {t('live.decision.viewAudit')}
        </a>
        {result.paper_order_ack && (
          <div className="rounded-lg border border-semantic-positive/40 bg-semantic-positive/10 p-3 text-sm">
            {t('live.decision.paperAck')}: {result.paper_order_ack.status} · order{' '}
            {result.paper_order_ack.order_id} · {result.paper_order_ack.environment}
          </div>
        )}
        {paperOrderRequested && !result.paper_order_ack && (
          <div className="rounded-lg border border-muted-foreground/30 bg-muted/50 p-3 text-sm text-muted-foreground">
            {t('live.decision.paperNotSubmitted')}
          </div>
        )}
      </CardContent>
    </Card>
  )
}

/** Render one compact response metric. */
function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg bg-muted/60 p-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 break-words font-mono text-sm font-semibold">{value}</div>
    </div>
  )
}
