import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'

import { StrategyCard } from '@/components/v2_1/strategy-card'
import { AppSidebar } from '@/components/layout/app-sidebar'
import i18n from '@/i18n'
import LabPage from '@/pages/lab'
import PersonalPage from '@/pages/personal'
import StrategyAnalysisPage from '@/pages/strategy-analysis'
import StrategyCenterPage from '@/pages/strategy-center'
import { resetStrategyAnalysis, setActiveStrategyId } from '@/stores/ui'

const renderPage = (page: React.ReactNode, initialEntry = '/') => {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
  return render(<QueryClientProvider client={queryClient}><MemoryRouter initialEntries={[initialEntry]}>{page}</MemoryRouter></QueryClientProvider>)
}

const response = (body: unknown, ok = true) => ({ ok, status: ok ? 200 : 503, json: async () => body })

describe('V2.1 consumer shell', () => {
  beforeAll(async () => { await i18n.changeLanguage('zh') })
  beforeEach(() => { setActiveStrategyId('steady-dca'); resetStrategyAnalysis() })
  afterEach(() => { cleanup(); vi.unstubAllGlobals() })

  it('does not invent a monthly action when the real API has no plan', async () => {
    const fetchMock = vi.fn().mockResolvedValue(response([]))
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<PersonalPage />)
    expect(await screen.findByRole('heading', { name: '先建立第一个长期计划' })).toBeTruthy()
    expect(screen.queryByText('MA200 一年历史回放')).toBeNull()
    expect(fetchMock).toHaveBeenCalledTimes(2)
    expect(fetchMock.mock.calls.map((call) => String(call[0]))).toEqual(expect.arrayContaining([
      expect.stringContaining('/investment-plans'),
      expect.stringContaining('/strategy-catalog'),
    ]))
  })

  it('exposes plan management as My plans directly below Personal', () => {
    renderPage(<AppSidebar />)
    const personal = screen.getByRole('link', { name: '个人中心' })
    const plans = screen.getByRole('link', { name: '我的计划' })

    expect(plans.getAttribute('href')).toBe('/plans')
    expect(personal.compareDocumentPosition(plans) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('shows real active plans and keeps strategy-card exploration separate from adoption', async () => {
    const fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes('/strategy-catalog')) return Promise.resolve(response(strategyCatalog()))
      return Promise.resolve(response([
        investmentPlan({ id: 'plan-voo', name: 'VOO 长期计划', symbol: 'VOO', policy: { id: 'fixed_dca', version: 1 } }),
        investmentPlan({ id: 'plan-qqq', name: 'QQQ 旧计划', symbol: 'QQQ', policy: { id: 'core_opportunity_v1', version: 1 } }),
        investmentPlan({ id: 'plan-paused', name: '已暂停计划', is_active: false }),
      ]))
    })
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyCenterPage />)

    expect(await screen.findByText('VOO 长期计划')).toBeTruthy()
    expect(screen.getByText('QQQ 旧计划')).toBeTruthy()
    expect(screen.queryByText('已暂停计划')).toBeNull()
    expect(screen.queryByRole('button', { name: '和其他策略对比' })).toBeNull()

    const fixedDcaAnalysis = screen.getByRole('link', { name: '查看每月稳步投入的直观分析' })
    expect(fixedDcaAnalysis.getAttribute('href')).toBe('/strategy-analysis?strategy=steady-dca&view=plain')
    expect(screen.queryByText('正在查看')).toBeNull()
    expect(screen.queryByText('自适应长期计划')).toBeNull()
    expect(screen.getAllByRole('link', { name: '用这个策略建立计划' })).toHaveLength(3)
    expect(screen.getAllByRole('link', { name: '用这个策略建立计划' })[1].getAttribute('href')).toContain('policy_id=dsl_ma200_trend_guard')
    expect(screen.getAllByText('已通过准入').length).toBeGreaterThan(0)
  })

  it('blocks plan creation when the server catalog has no eligible research result', async () => {
    const blocked = { ...strategyCatalog()[1], adoptable: false, research_status: 'blocked', research: undefined }
    const fetchMock = vi.fn().mockImplementation((url: string) => Promise.resolve(response(url.includes('/strategy-catalog') ? [blocked] : [])))
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyCenterPage />)

    expect(await screen.findByText('研究未通过，暂不可创建')).toBeTruthy()
    expect(screen.getByText('等待完整数据')).toBeTruthy()
    expect(screen.getByText('暂不可采用')).toBeTruthy()
    expect(screen.queryByRole('link', { name: '用这个策略建立计划' })).toBeNull()
  })

  it('runs selected strategies on one real normalized analysis chart', async () => {
    const fetchMock = vi.fn().mockResolvedValue(response(strategyBacktest(['fixed_dca', 'dsl_ma200_trend_guard'])))
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyAnalysisPage />)
    expect(await screen.findByText('US.SPY · 真实日线回测')).toBeTruthy()
    expect(screen.queryByText('演示数据 · 非真实回测')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: /MA200 保护/ }))
    await waitFor(() => expect(screen.getByRole('button', { name: /MA200 保护/ }).getAttribute('aria-pressed')).toBe('true'))
    expect(screen.getByRole('button', { name: /固定定投/ }).getAttribute('aria-pressed')).toBe('true')
    fireEvent.click(screen.getByRole('button', { name: '近 6 个月' }))
    await waitFor(() => expect(screen.getByRole('button', { name: '近 6 个月' }).getAttribute('aria-pressed')).toBe('true'))
    fireEvent.click(screen.getByRole('button', { name: '运行真实回测' }))
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2))
    fireEvent.click(screen.getByRole('button', { name: '全部样本' }))
    await waitFor(() => expect(screen.getByRole('button', { name: '全部样本' }).getAttribute('aria-pressed')).toBe('true'))
    const submitted = JSON.parse(String(fetchMock.mock.calls.at(-1)?.[1]?.body))
    expect(submitted.strategy_ids).toEqual(['fixed_dca', 'dsl_ma200_trend_guard'])
    expect(submitted.range).toBe('6m')
  })

  it('opens the requested catalog strategy in the plain analysis view', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response(strategyBacktest(['dsl_ma200_trend_guard']))))
    renderPage(<StrategyAnalysisPage />, '/strategy-analysis?strategy=ma200-trend-guard&view=plain')
    expect(await screen.findByText('US.SPY · 真实日线回测')).toBeTruthy()
    expect(screen.getByRole('button', { name: /MA200 保护/ }).getAttribute('aria-pressed')).toBe('true')
    expect(screen.getByRole('button', { name: /固定定投/ }).getAttribute('aria-pressed')).toBe('false')
    expect(screen.getByRole('button', { name: '直观视角' }).getAttribute('aria-pressed')).toBe('true')
  })

  it('uses the same real response for professional metrics', async () => {
    const fetchMock = vi.fn().mockResolvedValue(response(strategyBacktest(['fixed_dca'])))
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyAnalysisPage />)
    fireEvent.click(screen.getByRole('button', { name: '专业研究' }))
    expect(await screen.findByLabelText('真实专业回测指标')).toBeTruthy()
    expect(screen.getByText('+8.2%')).toBeTruthy()
    expect(screen.getByText('-18.4%')).toBeTruthy()
    expect(screen.getByText(/每条策略均投入 12 次/)).toBeTruthy()
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it('never replaces an unavailable provider with a demo chart', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response({ error: { code: 'service_unavailable', message: 'offline' } }, false)))
    renderPage(<StrategyAnalysisPage />)
    expect(await screen.findByText('真实行情暂不可用')).toBeTruthy()
    expect(screen.queryByText('真实归一化走势')).toBeNull()
  })

  it('explains an invalid symbol without misreporting the provider as offline', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: false,
      status: 400,
      json: async () => ({ error: { code: 'bad_request', message: 'invalid request' } }),
    }))
    renderPage(<StrategyAnalysisPage />)
    expect(await screen.findByText('这次回测无法完成')).toBeTruthy()
    expect(screen.getByText(/请检查市场前缀、标的代码/)).toBeTruthy()
    expect(screen.queryByText('真实行情暂不可用')).toBeNull()
  })

  it('opens and closes a local configuration preview without claiming to connect anything', () => {
    renderPage(<LabPage />)
    fireEvent.click(screen.getAllByRole('button', { name: '打开配置预览' })[1])
    expect(screen.getByText('配置预览')).toBeTruthy()
    expect(screen.getByText(/不写入任何配置/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: '收起预览' }))
    expect(screen.queryByText('配置预览')).toBeNull()
  })

  it('runs the legacy MA200 replay only after an explicit Lab action', async () => {
    let finishReplay: (() => void) | undefined
    const fetchMock = vi.fn().mockImplementation(() => new Promise((resolve) => {
      finishReplay = () => resolve(response({
        currency: 'USD',
        methodology: 'Legacy hard-coded MA200 replay for compatibility only.',
        points: [{ date: '2026-01-02', plain_dca_value: 1000, adaptive_value: 1012 }],
      }))
    }))
    vi.stubGlobal('fetch', fetchMock)

    renderPage(<LabPage />)
    expect(fetchMock).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: '运行旧实验' }))
    expect(await screen.findByRole('button', { name: '正在运行…' })).toHaveProperty('disabled', true)
    await act(async () => { finishReplay?.() })

    expect(await screen.findByText('Legacy hard-coded MA200 replay for compatibility only.')).toBeTruthy()
    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(String(fetchMock.mock.calls[0][0])).toContain('/paper-performance/historical-backtest')
  })

  it('contains a legacy replay failure inside the Lab panel', async () => {
    const fetchMock = vi.fn().mockResolvedValue(response({ error: { code: 'optional_unavailable', message: 'offline' } }, false))
    vi.stubGlobal('fetch', fetchMock)

    renderPage(<LabPage />)
    fireEvent.click(screen.getByRole('button', { name: '运行旧实验' }))

    expect((await screen.findByRole('status')).textContent).toContain('旧回放暂时不可用')
    expect(screen.getByRole('heading', { name: '把复杂配置，留给想深入的人' })).toBeTruthy()
  })

  it('shows an explicit empty state when the legacy replay returns no points', async () => {
    const fetchMock = vi.fn().mockResolvedValue(response({ currency: 'USD', methodology: 'legacy', points: [] }))
    vi.stubGlobal('fetch', fetchMock)

    renderPage(<LabPage />)
    fireEvent.click(screen.getByRole('button', { name: '运行旧实验' }))

    expect(await screen.findByText('没有足够的历史数据生成这份旧回放。')).toBeTruthy()
  })

  it('renders a compact strategy card without the rule panel', () => {
    render(<MemoryRouter><StrategyCard strategy={strategyCatalog()[0]} analysisHref="/strategy-analysis?strategy=steady-dca&view=plain" variant="compact" /></MemoryRouter>)
    expect(screen.getByText('每月稳步投入')).toBeTruthy()
    expect(screen.queryByText('它会怎么做：')).toBeNull()
  })
})

function strategyCatalog() {
  const base = {
    risk: 'stable' as const,
    supported_symbols: [],
    supported_markets: ['us', 'hong_kong', 'china_shanghai', 'china_shenzhen'] as Array<'us' | 'hong_kong' | 'china_shanghai' | 'china_shenzhen'>,
    default_plan: { schedule_kind: 'monthly' as const, schedule_day: 18, core_ratio: '1.0', opportunity_ratio: '0.0', risk_mode: 'fixed' as const },
    data_requirements: [],
    data_requirement: { required_close_observations: 0 },
    adoptable: true,
    research_status: 'reference' as const,
  }
  const admission = {
    eligible: true,
    core_bucket_safe: true,
    budget_safe: true,
    assets: [{
      symbol: 'SP500', observations: 100, evidence_start_as_of: '2016-01-01', evidence_end_as_of: '2025-12-31',
      strategy: { terminal_wealth_usd: 10000, maximum_drawdown_percent: -20, cash_utilisation_percent: 90 },
      fixed_dca: { xirr_percent: 8.2, terminal_wealth_usd: 10500, maximum_drawdown_percent: -25, annualized_volatility_percent: 18.25, sortino_ratio: 0.61, cash_utilisation_percent: 100 },
      rolling_out_of_sample: [{ start_as_of: '2016-01-01', end_as_of: '2018-01-01', observations: 24, strategy: { terminal_wealth_usd: 2100, maximum_drawdown_percent: -12, cash_utilisation_percent: 99 }, fixed_dca: { terminal_wealth_usd: 2050, maximum_drawdown_percent: -14, cash_utilisation_percent: 100 } }],
    }],
  }
  return [
    { ...base, policy: { id: 'fixed_dca', version: 1 }, name: '每月稳步投入', summary: '固定日期投入。', rule: '按计划金额投入。', limitation: '不会主动降低回撤。' },
    { ...base, policy: { id: 'dsl_ma200_trend_guard', version: 1 }, name: '200 日均线趋势保护', summary: '管理弹性投入。', rule: '低于均线时暂停弹性桶。', limitation: '均线具有滞后性。', default_plan: { ...base.default_plan, core_ratio: '0.7', opportunity_ratio: '0.3', risk_mode: 'approval' as const }, research_status: 'available' as const, research: admission },
    { ...base, policy: { id: 'dsl_growth_volatility_balance', version: 1 }, name: '增长与波动平衡', summary: '检查增长与波动。', rule: '按阈值调整弹性桶。', limitation: '震荡期可能切换。', risk: 'balanced' as const, default_plan: { ...base.default_plan, core_ratio: '0.7', opportunity_ratio: '0.3', risk_mode: 'approval' as const }, research_status: 'available' as const, research: admission },
  ]
}

function investmentPlan(overrides: Record<string, unknown> = {}) {
  return {
    id: 'plan-1',
    name: 'VOO 长期计划',
    symbol: 'VOO',
    base_contribution: '1000.00',
    currency: 'USD',
    schedule_kind: 'monthly',
    schedule_day: 18,
    schedule_days: [18],
    policy: { id: 'fixed_dca', version: 1 },
    execution_configuration: {
      bucket_allocation: { core_ratio: '1.00', opportunity_ratio: '0.00' },
      risk_mode: 'fixed',
      opportunity_cash_policy: 'expire_each_period',
    },
    max_single_execution: '1000.00',
    is_active: true,
    created_at: '2026-09-18T00:00:00Z',
    updated_at: '2026-09-18T00:00:00Z',
    ...overrides,
  }
}

function strategyBacktest(strategyIds: string[]) {
  return {
    requested_range: '3y',
    data: {
      provider: 'opend', market: 'us', instrument_type: 'equity_or_etf', currency: 'USD',
      timezone: 'America/New_York', adjustment: 'all', fetched_at: '2026-09-19T00:00:00Z',
      requested_start: '2022-08-15', requested_end: '2026-09-19', dataset_version: 'history-kline-v10',
      checksum: 'a'.repeat(64),
    },
    result: {
      symbol: 'US.SPY', effective_start: '2023-09-18', effective_end: '2026-09-18', contribution_count: 12,
      series: strategyIds.map((strategyId) => ({
        strategy_id: strategyId,
        strategy_version: 1,
        strategy_name: strategyId,
        normalized_points: [{ date: '2023-09-18', value: 100 }, { date: '2026-09-18', value: 108.2 }],
        metrics: {
          total_return_percent: 8.2,
          annualized_return_percent: 2.66,
          xirr_percent: 7.4,
          maximum_drawdown_percent: 18.4,
          annualized_volatility_percent: 12.8,
          sortino_ratio: 0.74,
          total_contributed: 12000,
          total_invested: 12000,
          cash_utilisation_percent: 100,
          terminal_wealth: 12984,
          terminal_cash: 0,
        },
      })),
    },
  }
}
