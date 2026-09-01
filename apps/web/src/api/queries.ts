import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import type {
  CreateInvestmentPlanRequest,
  AutomaticDecisionPreviewRequest,
  DecisionPreviewRequest,
  DecisionPreviewResponse,
  DecisionRecord,
  FundamentalPreviewRequest,
  FundamentalSignal,
  InvestmentPlan,
  MarketSignalInput,
  PaperPortfolioSnapshot,
  PaperPerformance,
  ActualPerformance,
  HistoricalBacktest,
  HoldingPriceHistory,
  MarketSentimentEvidence,
  TrendPreviewRequest,
  TrendSignal,
  StoredStrategySpec,
  StrategySpecDocument,
  StrategyAdmissionReport,
  StrategyValidationResponse,
  AiProviderListResponse,
  CopilotDraftRequest,
  CopilotDraftResponse,
  UpdateInvestmentPlanRequest,
  HealthStatus,
  ReadyStatus,
  RuntimeStatus,
} from './types'

const apiBaseUrl = (import.meta.env.VITE_API_BASE_URL ?? '').replace(/\/$/, '')

/** Error returned to the UI without exposing transport or provider internals. */
export class ApiRequestError extends Error {
  /** HTTP status returned by the safe API envelope when one was available. */
  readonly status?: number

  /** Stable public error code returned by the API envelope when one was available. */
  readonly code?: string

  /** Build a client-safe request error without retaining provider or transport details. */
  constructor(message: string, options: { status?: number; code?: string } = {}) {
    super(message)
    this.name = 'ApiRequestError'
    this.status = options.status
    this.code = options.code
  }
}

interface ErrorEnvelope {
  error?: { code?: string; message?: string }
}

/** Call one same-origin or configured Rust HTTP endpoint. */
async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${apiBaseUrl}${path}`, {
    ...init,
    headers: {
      Accept: 'application/json',
      ...(init?.body ? { 'Content-Type': 'application/json' } : {}),
      ...init?.headers,
    },
  })
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as ErrorEnvelope | null
    throw new ApiRequestError(body?.error?.message ?? 'request failed', {
      status: response.status,
      code: body?.error?.code,
    })
  }
  if (response.status === 204) {
    return undefined as T
  }
  return (await response.json()) as T
}

/** List normalized investment plans from the Rust API. */
export function fetchPlans(): Promise<InvestmentPlan[]> {
  return request('/investment-plans')
}

/** Read process liveness without checking optional provider dependencies. */
export function fetchHealth(): Promise<HealthStatus> {
  return request('/health')
}

/** Check SQLite readiness through the backend's safe readiness contract. */
export function fetchReady(): Promise<ReadyStatus> {
  return request('/ready')
}

/** Read configured capabilities and latest scheduler counters without triggering work. */
export function fetchRuntimeStatus(): Promise<RuntimeStatus> {
  return request('/runtime-status')
}

/** Create one normalized investment plan through the Rust API. */
export function createPlan(input: CreateInvestmentPlanRequest): Promise<InvestmentPlan> {
  return request('/investment-plans', { method: 'POST', body: JSON.stringify(input) })
}

/** Update one existing plan without changing its symbol, currency, or schedule kind. */
export function updatePlan(planId: string, input: UpdateInvestmentPlanRequest): Promise<InvestmentPlan> {
  return request(`/investment-plans/${encodeURIComponent(planId)}`, { method: 'PATCH', body: JSON.stringify(input) })
}

/** List immutable restricted strategy versions for the Strategy Studio. */
export function fetchStrategies(): Promise<StoredStrategySpec[]> {
  return request('/strategies')
}

/** Validate a form-authored strategy without writing it to SQLite. */
export function validateStrategy(input: StrategySpecDocument): Promise<StrategyValidationResponse> {
  return request('/strategies/validate', { method: 'POST', body: JSON.stringify(input) })
}

/** Persist one validated immutable DSL strategy version. */
export function createStrategy(input: StrategySpecDocument): Promise<StoredStrategySpec> {
  return request('/strategies', { method: 'POST', body: JSON.stringify(input) })
}

/** List only credential-free AI profiles deployed by the current server. */
export function fetchAiProviders(): Promise<AiProviderListResponse> {
  return request('/ai/providers')
}

/** Ask a deployed AI profile for an unpersisted, restricted DSL candidate. */
export function generateCopilotDraft(input: CopilotDraftRequest): Promise<CopilotDraftResponse> {
  return request('/strategies/copilot-draft', { method: 'POST', body: JSON.stringify(input) })
}

/** Run the committed fixed-fixture safety and Fixed-DCA comparison for one stored version. */
export function evaluateStrategyAdmission(policy: import('./types').PolicyReference): Promise<StrategyAdmissionReport> {
  return request(`/strategies/${encodeURIComponent(policy.id)}/${policy.version}/admission`)
}

/** Bind a confirmed immutable strategy version to a recurring plan. */
export function activatePlanPolicy(planId: string, policy: import('./types').PolicyReference): Promise<InvestmentPlan> {
  return request(`/investment-plans/${encodeURIComponent(planId)}/activate-policy`, {
    method: 'POST',
    body: JSON.stringify({ policy }),
  })
}

/** Delete one recurring holding and its local-only dependent records. */
export async function deletePlan(planId: string): Promise<void> {
  await request(`/investment-plans/${encodeURIComponent(planId)}`, { method: 'DELETE' })
}

/** Calculate a 70% fundamental signal from caller-provided historical data. */
export function previewFundamental(input: FundamentalPreviewRequest): Promise<FundamentalSignal> {
  return request('/signals/fundamental/preview', {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

/** Calculate a 20% trend signal from caller-provided historical data. */
export function previewTrend(input: TrendPreviewRequest): Promise<TrendSignal> {
  return request('/signals/trend/preview', {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

/** Read one automatic, source-labelled signal snapshot from the local Rust API. */
export function fetchMarketSignalInput(symbol: string): Promise<MarketSignalInput> {
  return request(`/signals/market-input/${encodeURIComponent(symbol)}`)
}

/** Ask the configured Qwen pipeline for a source-backed current market-sentiment explanation. */
export async function fetchMarketSentiment(): Promise<MarketSentimentEvidence> {
  const response = await request<Partial<MarketSentimentEvidence>>('/market-sentiment/preview', { method: 'POST' })
  if (typeof response.rationale !== 'string' || !Array.isArray(response.warnings) || !Array.isArray(response.headlines)) {
    throw new ApiRequestError('本机 Rust 服务仍在运行旧版 Qwen 响应契约。请重启 indexlink-server 后重试。', { status: 426 })
  }
  return response as MarketSentimentEvidence
}

/** Read funds, positions, and recent orders from the configured local paper account. */
export function fetchPaperPortfolio(): Promise<PaperPortfolioSnapshot> {
  return request('/paper-portfolio')
}

/** Refresh one plan's local paper ledger from read-only OpenD account data. */
export function fetchPaperPerformance(planId: string): Promise<PaperPerformance> {
  return request(`/investment-plans/${encodeURIComponent(planId)}/paper-performance`)
}

/** Refresh and read every active holding's real local-paper trajectory. */
export function fetchActualPerformance(): Promise<ActualPerformance> {
  return request('/paper-performance/actual')
}

/** Read one transparent year of price-only historical plain-versus-adaptive replay. */
export function fetchHistoricalBacktest(): Promise<HistoricalBacktest> {
  return request('/paper-performance/historical-backtest')
}

/** Read actual OpenD price lines plus local paper buy/sell markers for every active holding. */
export function fetchHoldingPriceHistory(period: '3m' | '6m' | '1y' | '3y'): Promise<HoldingPriceHistory[]> {
  return request(`/market-data/holdings?period=${period}`)
}

/** Store a user-confirmed local opening balance used only for return calculations. */
export function setPaperOpeningBalance(
  planId: string,
  input: { amount: string; occurred_at: string },
): Promise<void> {
  return request(`/investment-plans/${encodeURIComponent(planId)}/paper-performance/opening-balance`, {
    method: 'PUT',
    body: JSON.stringify(input),
  })
}

/** Compose a decision, persist its audit record, and optionally submit a paper order. */
export function previewDecision(
  planId: string,
  input: DecisionPreviewRequest,
): Promise<DecisionPreviewResponse> {
  return request(`/investment-plans/${planId}/decision-preview`, {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

/** Compose a decision from server-sourced 70/20 inputs and persist its audit record. */
export function previewAutomaticDecision(
  planId: string,
  input: AutomaticDecisionPreviewRequest,
): Promise<DecisionPreviewResponse> {
  return request(`/investment-plans/${planId}/automatic-decision-preview`, {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

/** List persisted decision records for one selected plan. */
export function fetchDecisionRecords(planId: string): Promise<DecisionRecord[]> {
  return request(`/investment-plans/${planId}/decisions?limit=50`)
}

/** List newest cross-plan decision records for review filtering and pagination. */
export function fetchAllDecisionRecords(): Promise<DecisionRecord[]> {
  return request('/decisions?limit=200')
}

/** Fetch one decision record for a detail route. */
export function fetchDecisionRecord(id: string): Promise<DecisionRecord> {
  return request(`/decisions/${id}`)
}

/** Confirm the immutable approval record and submit its audited paper order once. */
export function approveDecisionPaperOrder(id: string, idempotencyKey: string) {
  return request(`/decisions/${encodeURIComponent(id)}/approve-paper-order`, {
    method: 'POST',
    body: JSON.stringify({ idempotency_key: idempotencyKey }),
  })
}

/** React Query hook for live plan data. */
export function usePlans() {
  return useQuery({ queryKey: ['plans'], queryFn: fetchPlans })
}

/** Cache the process liveness response and refresh it periodically for the status strip. */
export function useHealth() {
  return useQuery({ queryKey: ['health'], queryFn: fetchHealth, refetchInterval: 30_000, retry: false })
}

/** Cache SQLite readiness separately so the UI can distinguish database and process failures. */
export function useReady() {
  return useQuery({ queryKey: ['ready'], queryFn: fetchReady, refetchInterval: 30_000, retry: false })
}

/** Cache safe optional-dependency and scheduler state without issuing a trading or AI request. */
export function useRuntimeStatus() {
  return useQuery({ queryKey: ['runtime-status'], queryFn: fetchRuntimeStatus, refetchInterval: 15_000, retry: false })
}

/** Cache a selected plan's latest server-sourced market snapshot across navigation. */
export function useMarketSignalInput(symbol: string | null) {
  return useQuery({
    queryKey: ['market-signal-input', symbol],
    queryFn: () => fetchMarketSignalInput(symbol!),
    enabled: false,
  })
}

/** Cache the latest Qwen response across pages until the user explicitly refreshes it. */
export function useMarketSentiment() {
  return useQuery({ queryKey: ['market-sentiment'], queryFn: fetchMarketSentiment, enabled: false })
}

/** Cache the read-only paper-account snapshot. */
export function usePaperPortfolio() {
  return useQuery({ queryKey: ['paper-portfolio'], queryFn: fetchPaperPortfolio, enabled: false })
}

/** Cache a plan's local paper-performance ledger. */
export function usePaperPerformance(planId: string | null) {
  return useQuery({
    queryKey: ['paper-performance', planId],
    queryFn: () => fetchPaperPerformance(planId!),
    enabled: false,
  })
}

/** Cache the combined local paper trajectory. */
export function useActualPerformance() {
  return useQuery({ queryKey: ['actual-performance'], queryFn: fetchActualPerformance, enabled: false })
}

/** Cache the explicit one-year historical replay. */
export function useHistoricalBacktest() {
  return useQuery({ queryKey: ['historical-backtest'], queryFn: fetchHistoricalBacktest, enabled: false })
}

/** Cache price histories separately by requested visible range. */
export function useHoldingPriceHistory(period: '3m' | '6m' | '1y' | '3y') {
  return useQuery({
    queryKey: ['holding-price-history', period],
    queryFn: () => fetchHoldingPriceHistory(period),
    enabled: false,
  })
}

/** React Query hook for Strategy Studio discovery data. */
export function useStrategies() {
  return useQuery({ queryKey: ['strategies'], queryFn: fetchStrategies })
}

/** Cache the safe server-side provider registry for the Copilot selector. */
export function useAiProviders() {
  return useQuery({ queryKey: ['ai-providers'], queryFn: fetchAiProviders })
}

/** Generate a candidate only; this mutation deliberately does not invalidate persisted strategies. */
export function useCopilotDraft() {
  return useMutation({ mutationFn: generateCopilotDraft })
}

/** Validate without mutation so form failures remain readable and local. */
export function useValidateStrategy() {
  return useMutation({ mutationFn: validateStrategy })
}

/** Save an immutable strategy and refresh the Studio list. */
export function useCreateStrategy() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: createStrategy,
    onSuccess: async () => { await queryClient.invalidateQueries({ queryKey: ['strategies'] }) },
  })
}

/** Run admission only after the user explicitly requests a fixed-sample evaluation. */
export function useStrategyAdmission() {
  return useMutation({
    mutationFn: (policy: import('./types').PolicyReference) => evaluateStrategyAdmission(policy),
  })
}

/** Activate a policy then refresh all plan-backed screens. */
export function useActivatePlanPolicy() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ planId, policy }: { planId: string; policy: import('./types').PolicyReference }) => activatePlanPolicy(planId, policy),
    onSuccess: async () => { await queryClient.invalidateQueries({ queryKey: ['plans'] }) },
  })
}

/** React Query mutation that refreshes the plan list after creation. */
export function useCreatePlan() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: createPlan,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['plans'] })
    },
  })
}

/** Delete a recurring holding and invalidate every plan-backed view. */
export function useDeletePlan() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: deletePlan,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['plans'] })
    },
  })
}

/** Update plan configuration then refresh all plan-backed views. */
export function useUpdatePlan() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ planId, input }: { planId: string; input: UpdateInvestmentPlanRequest }) => updatePlan(planId, input),
    onSuccess: async () => { await queryClient.invalidateQueries({ queryKey: ['plans'] }) },
  })
}

/** React Query hook for the selected plan's decision history. */
export function useDecisionRecords(planId: string | null) {
  return useQuery({
    queryKey: ['decision-records', planId],
    queryFn: () => fetchDecisionRecords(planId!),
    enabled: planId !== null,
  })
}

/** Cache the bounded cross-plan decision history used by the review page. */
export function useAllDecisionRecords() {
  return useQuery({ queryKey: ['decision-records', 'all'], queryFn: fetchAllDecisionRecords })
}

/** React Query hook for a single decision-record detail. */
export function useDecisionRecord(id: string | null) {
  return useQuery({
    queryKey: ['decision-record', id],
    queryFn: () => fetchDecisionRecord(id!),
    enabled: id !== null,
  })
}

/** Submit a human-approved paper order and refresh its immutable audit record. */
export function useApproveDecisionPaperOrder() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ id, idempotencyKey }: { id: string; idempotencyKey: string }) => approveDecisionPaperOrder(id, idempotencyKey),
    onSuccess: async (_data, variables) => {
      await queryClient.invalidateQueries({ queryKey: ['decision-record', variables.id] })
      await queryClient.invalidateQueries({ queryKey: ['decision-records'] })
      await queryClient.invalidateQueries({ queryKey: ['paper-portfolio'] })
      await queryClient.invalidateQueries({ queryKey: ['actual-performance'] })
    },
  })
}
