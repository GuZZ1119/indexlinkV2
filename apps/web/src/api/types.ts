/** 决策最终动作，与后端 `decision-engine` 的 JSON 契约对齐。 */
export type DecisionAction =
  | 'overweight'
  | 'standard'
  | 'tactical_delay'
  | 'underweight'
  | 'skip'

/** Immutable strategy identity selected by a plan and persisted in each new audit. */
export interface PolicyReference {
  id: string
  version: number
}

/** Read-only document of one immutable restricted DSL strategy version. */
export interface StrategySpecDocument {
  policy_id: string
  policy_version: number
  name: string
  rules: StrategyRuleDocument[]
}

/** One form-authored condition/action rule; arbitrary source code is never accepted. */
export interface StrategyRuleDocument {
  condition: StrategyConditionDocument
  action:
    | { kind: 'set_opportunity_multiplier'; multiplier: number }
    | { kind: 'skip_opportunity' }
}

export type StrategyIndicatorDocument =
  | { kind: 'close_price' }
  | { kind: 'simple_moving_average'; lookback_days: number }
  | { kind: 'exponential_moving_average'; lookback_days: number }
  | { kind: 'relative_strength_index'; lookback_days: number }
  | { kind: 'drawdown'; lookback_days: number }
  | { kind: 'vix' }

export type StrategyComparisonDocument = {
    kind: 'comparison'
    expression: { kind: 'indicator'; indicator: StrategyIndicatorDocument }
    operator: 'greater_than' | 'greater_than_or_equal' | 'less_than' | 'less_than_or_equal'
    threshold: string
  }

export type StrategyConditionDocument = StrategyComparisonDocument | {
  kind: 'all' | 'any'
  conditions: StrategyComparisonDocument[]
}

/** Immutable stored DSL strategy displayed by the Studio. */
export interface StoredStrategySpec {
  policy: PolicyReference
  name: string
  document: StrategySpecDocument
  created_at: string
}

/** Safe validation result shown beside the Studio form. */
export interface StrategyValidationResponse {
  valid: boolean
  error?: string
  document?: StrategySpecDocument
}

/** Fixed-fixture safety and comparison report required before activating a DSL strategy. */
export interface StrategyAdmissionReport {
  eligible: boolean
  reason?: string
  core_bucket_safe: boolean
  budget_safe: boolean
  assets: StrategyAdmissionAsset[]
}

/** Matched historical result for one fixed-fixture symbol. */
export interface StrategyAdmissionAsset {
  symbol: string
  observations: number
  evidence_start_as_of: string
  evidence_end_as_of: string
  strategy: StrategyAdmissionMetrics
  fixed_dca: StrategyAdmissionMetrics
  rolling_out_of_sample: StrategyAdmissionRollingWindow[]
}

/** Comparable non-promotional metrics under one identical contribution schedule. */
export interface StrategyAdmissionMetrics {
  xirr_percent?: number
  terminal_wealth_usd: number
  maximum_drawdown_percent: number
  annualized_volatility_percent?: number
  sortino_ratio?: number
  cash_utilisation_percent: number
}

/** One fixed-size causal rolling comparison included in strategy admission. */
export interface StrategyAdmissionRollingWindow {
  start_as_of: string
  end_as_of: string
  observations: number
  strategy: StrategyAdmissionMetrics
  fixed_dca: StrategyAdmissionMetrics
}

/** A server-side investment plan. Decimal values remain JSON strings. */
export interface InvestmentPlan {
  id: string
  name: string
  symbol: string
  base_contribution: string
  currency: string
  schedule_kind: 'monthly' | 'weekly'
  schedule_day: number
  schedule_days: number[]
  policy: PolicyReference
  execution_configuration: PlanExecutionConfiguration
  max_single_execution: string
  is_active: boolean
  created_at: string
  updated_at: string
}

/** Persisted core/opportunity allocation and execution guardrails for one plan. */
export interface PlanExecutionConfiguration {
  bucket_allocation: { core_ratio: string; opportunity_ratio: string }
  risk_mode: 'fixed' | 'autopilot' | 'approval'
  opportunity_cash_policy: 'expire_each_period' | 'carry_forward' | 'carry_with_cap'
  opportunity_cash_cap?: string
  period_execution_limit?: string
}

/** Payload accepted when creating an investment plan. */
export interface CreateInvestmentPlanRequest {
  name: string
  symbol: string
  base_contribution: string
  currency: string
  schedule_kind: 'monthly' | 'weekly'
  schedule_day: number
  schedule_days?: number[]
  policy?: PolicyReference
  bucket_allocation?: { core_ratio: string; opportunity_ratio: string }
  risk_mode?: PlanExecutionConfiguration['risk_mode']
  opportunity_cash_policy?: PlanExecutionConfiguration['opportunity_cash_policy']
  opportunity_cash_cap?: string
  period_execution_limit?: string
  max_single_execution: string
}

/** Mutable subset accepted by PATCH /investment-plans/:id. */
export type UpdateInvestmentPlanRequest = Partial<Omit<CreateInvestmentPlanRequest, 'symbol' | 'currency' | 'schedule_kind'>> & { is_active?: boolean }

/** Caller-supplied monthly input for the 70% fundamental calculation. */
export interface FundamentalPreviewRequest {
  cape_history: number[]
  cape_current: number
  erp_history: number[]
  erp_current: number
}

/** Auditable 70% fundamental signal returned by the server. */
export interface FundamentalSignal {
  score: number
  cape_percentile: number
  erp_percentile: number
}

/** Caller-supplied monthly input for the 20% trend calculation. */
export interface TrendPreviewRequest {
  ma_distance_history: number[]
  ma_distance_current: number
  rsi_history: number[]
  rsi_current: number
  vix_history: number[]
  vix_current: number
}

/** Auditable 20% trend signal returned by the server. */
export interface TrendSignal {
  score: number
  ma_distance_percentile: number
  rsi_percentile: number
  vix_percentile: number
  regime: 'neutral' | 'overheated' | 'falling_knife'
}

/** Automatically refreshed, source-labelled inputs for the existing signal APIs. */
export interface MarketSignalInput {
  symbol: string
  as_of: string
  fundamental: FundamentalPreviewRequest
  trend: TrendPreviewRequest
  sources: {
    price: string
    fundamental: string
    volatility: string
  }
}

/** Optional paper-only order submitted from a decision preview. */
export interface PaperOrderRequest {
  idempotency_key: string
  side: 'buy' | 'sell'
  order_type: 'market' | 'limit'
  quantity: string
  limit_price?: string
}

/** Locally persisted performance point reconstructed from paper-account observations. */
export interface PaperPerformancePoint {
  observed_at: string
  adaptive_value: string
  plain_dca_value: string
  net_contributions: string
}

/** Local paper-account return summary and chart series for one investment plan. */
export interface PaperPerformance {
  currency: string
  has_opening_balance: boolean
  data_complete: boolean
  net_contributions: string
  adaptive_value: string
  plain_dca_value: string
  realized_pnl: string
  unrealized_pnl: string
  total_return?: string
  points: PaperPerformancePoint[]
}

/** One real local-paper series for a recurring holding, or the explicit total line. */
export interface ActualPerformanceSeries {
  plan_id: string
  name: string
  symbol: string
  points: PaperPerformancePoint[]
}

/** Combined local-paper trajectory across every active recurring holding. */
export interface ActualPerformance {
  currency: string
  series: ActualPerformanceSeries[]
  total_points: PaperPerformancePoint[]
}

/** One local paper fill placed on a historical price chart. */
export interface PaperTradeMarker {
  plan_id: string
  side: 'buy' | 'sell'
  quantity: string
  price: string
  observed_at: string
}

/** OpenD prices and locally confirmed paper fills for one active recurring holding. */
export interface HoldingPriceHistory {
  plan_id: string
  name: string
  symbol: string
  prices: Array<{ date: string; close: number }>
  trades: PaperTradeMarker[]
}

/** One value point in the transparent one-year historical replay. */
export interface HistoricalBacktestPoint {
  date: string
  plain_dca_value: number
  adaptive_value: number
}

/** Explicitly scoped historical comparison, not an account return claim. */
export interface HistoricalBacktest {
  currency: string
  methodology: string
  points: HistoricalBacktestPoint[]
}

/** Read-only service liveness response. */
export interface HealthStatus {
  status: 'ok'
  service: string
  version: string
}

/** Readiness response from the SQLite-backed backend. */
export interface ReadyStatus {
  status: 'ready'
  database: 'ok'
}

/** Last safe counters emitted by the server-owned scheduler. */
export interface SchedulerStatus {
  enabled: boolean
  tick_interval_seconds: number
  last_tick_at?: string
  last_summary?: {
    created: number
    catch_up_created: number
    already_claimed: number
    unavailable: number
  }
  last_error_at?: string
}

/** Safe capability and scheduler snapshot; no account or provider credentials are present. */
export interface RuntimeStatus {
  service: 'running'
  database: 'ready' | 'unavailable'
  market_data: 'configured' | 'not_configured'
  qwen: 'configured' | 'not_configured'
  paper_broker: 'configured' | 'not_configured'
  scheduler: SchedulerStatus
}

/** Request accepted by the composed Decision Preview endpoint. */
export interface DecisionPreviewRequest {
  day_of_month: number
  bucket_allocation: {
    core_ratio: string
    opportunity_ratio: string
  }
  fundamental: FundamentalSignal
  trend: TrendSignal
  paper_order?: PaperOrderRequest
}

/** Request accepted by the server-sourced Decision Preview endpoint. */
export interface AutomaticDecisionPreviewRequest {
  paper_order?: PaperOrderRequest
}

/** Execution preview returned as part of a decision. */
export interface ExecutionPreview {
  plan_id: string
  symbol: string
  currency: string
  schedule_kind: 'monthly' | 'weekly'
  schedule_day: number
  schedule_days: number[]
  status: 'due' | 'waiting' | 'inactive'
  planned_contribution?: string
  bucket_split?: {
    planned_contribution: string
    core_contribution: string
    opportunity_budget: string
    opportunity_multiplier: string
    carried_opportunity_cash: string
    opportunity_contribution: string
    unallocated_opportunity_contribution: string
    recommended_contribution: string
    opportunity_cash_policy: PlanExecutionConfiguration['opportunity_cash_policy']
    requires_approval: boolean
  }
}

/** Final weighted decision returned by the server. */
export interface DecisionResult {
  /** Bound immutable policy that produced this decision, when present in new records. */
  policy?: { id: string; version: number }
  /** Fixed DCA deliberately does not consume market or Qwen signal layers. */
  market_signals_used?: boolean
  final_score?: number
  multiplier: number
  action: DecisionAction
  weight_mode?: 'normal' | 'sentiment_unavailable'
  fundamental_score?: number
  trend_score?: number
  /** Absent from legacy records and `null` when Qwen is temporarily unavailable. */
  sentiment_score?: number | null
}

/** One RSS source headline retained with a Qwen market-sentiment result. */
export interface MarketSentimentHeadline {
  title: string
  url: string
  published_at: string
}

/** Structured Qwen explanation attached to a live decision and its audit snapshot. */
export interface MarketSentimentEvidence {
  score: number
  label: 'positive' | 'neutral' | 'negative'
  rationale: string
  warnings: string[]
  headlines: MarketSentimentHeadline[]
}

/** Stored sentiment snapshots remain backward compatible with score-only historical records. */
export interface PersistedMarketSentimentSnapshot {
  source: string
  score: number
  rationale?: string
  warnings?: string[]
  headlines?: MarketSentimentHeadline[]
}

/** Paper-order acknowledgement returned only after a broker accepts a request. */
export interface BrokerOrderAck {
  order_id: string
  environment: 'paper' | 'live'
  status: 'accepted' | 'duplicate'
}

/** Read-only snapshot of the configured OpenD paper account. */
export interface PaperPortfolioSnapshot {
  currency: string
  cash: string
  buying_power: string
  total_assets: string
  market_value: string
  positions: PaperPosition[]
  orders: PaperOrder[]
}

/** One current paper-account position returned by OpenD. */
export interface PaperPosition {
  symbol: string
  name?: string
  quantity: string
  price: string
  cost_price: string
  market_value: string
  unrealized_pnl: string
}

/** One recent normalized paper-order state returned by OpenD. */
export interface PaperOrder {
  order_id: string
  symbol: string
  side: 'buy' | 'sell'
  state: 'pending' | 'partially_filled' | 'filled' | 'closed' | 'unknown'
  quantity: string
  filled_quantity: string
  average_fill_price: string
}

/** Composed Decision Preview response. */
export interface DecisionPreviewResponse {
  audit_record_id: string
  execution: ExecutionPreview
  decision: DecisionResult
  market_sentiment?: MarketSentimentEvidence
  paper_order_ack?: BrokerOrderAck
  summary: string
}

/** Immutable policy identity and provider-neutral recommendation saved with new audits. */
export interface DecisionPolicyEvidence {
  policy: { id: string; version: number }
  recommendation_snapshot: {
    action: DecisionAction
    multiplier: number
    scheduled_contribution: string
    market_signals_used: boolean
  }
}

/** Persisted decision-record history item. */
export interface DecisionRecord {
  id: string
  plan_id: string
  symbol: string
  currency: string
  execution_status: 'due' | 'waiting' | 'inactive'
  planned_contribution?: string
  execution_snapshot: Record<string, unknown>
  fundamental_snapshot: Record<string, unknown>
  trend_snapshot: Record<string, unknown>
  sentiment_snapshot?: PersistedMarketSentimentSnapshot
  decision_snapshot: DecisionResult
  policy_evidence?: DecisionPolicyEvidence
  broker_order_request?: Record<string, unknown>
  broker_order_ack?: BrokerOrderAck
  summary: string
  created_at: string
}
