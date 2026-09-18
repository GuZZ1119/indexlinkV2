import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'

import { StrategyCard } from '@/components/v2_1/strategy-card'
import { AppSidebar } from '@/components/layout/app-sidebar'
import { findConsumerStrategy } from '@/features/v2_1/model'
import i18n from '@/i18n'
import LabPage from '@/pages/lab'
import PersonalPage from '@/pages/personal'
import StrategyAnalysisPage from '@/pages/strategy-analysis'
import StrategyCenterPage from '@/pages/strategy-center'
import { setActiveStrategyId } from '@/stores/ui'

const renderPage = (page: React.ReactNode) => {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
  return render(<QueryClientProvider client={queryClient}><MemoryRouter>{page}</MemoryRouter></QueryClientProvider>)
}

const response = (body: unknown, ok = true) => ({ ok, status: ok ? 200 : 503, json: async () => body })

describe('V2.1 consumer shell', () => {
  beforeAll(async () => { await i18n.changeLanguage('zh') })
  beforeEach(() => setActiveStrategyId('adaptive-70-20-10'))
  afterEach(() => { cleanup(); vi.unstubAllGlobals() })

  it('does not invent a monthly action when the real API has no plan', async () => {
    const fetchMock = vi.fn().mockResolvedValue(response([]))
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<PersonalPage />)
    expect(await screen.findByRole('heading', { name: '先建立第一个长期计划' })).toBeTruthy()
    expect(screen.queryByText('MA200 一年历史回放')).toBeNull()
    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(String(fetchMock.mock.calls[0][0])).toContain('/investment-plans')
  })

  it('exposes plan management as My plans directly below Personal', () => {
    renderPage(<AppSidebar />)
    const personal = screen.getByRole('link', { name: '个人中心' })
    const plans = screen.getByRole('link', { name: '我的计划' })

    expect(plans.getAttribute('href')).toBe('/plans')
    expect(personal.compareDocumentPosition(plans) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('shows real active plans and keeps strategy-card exploration separate from adoption', async () => {
    const fetchMock = vi.fn().mockResolvedValue(response([
      investmentPlan({ id: 'plan-voo', name: 'VOO 长期计划', symbol: 'VOO', policy: { id: 'fixed_dca', version: 1 } }),
      investmentPlan({ id: 'plan-qqq', name: 'QQQ 自适应计划', symbol: 'QQQ', policy: { id: 'core_opportunity_v1', version: 1 } }),
      investmentPlan({ id: 'plan-paused', name: '已暂停计划', is_active: false }),
    ]))
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyCenterPage />)

    expect(await screen.findByText('VOO 长期计划')).toBeTruthy()
    expect(screen.getByText('QQQ 自适应计划')).toBeTruthy()
    expect(screen.queryByText('已暂停计划')).toBeNull()
    expect(screen.queryByRole('button', { name: '和其他策略对比' })).toBeNull()

    const fixedDcaCard = screen.getByRole('button', { name: '查看每月稳步投入' })
    expect(fixedDcaCard.getAttribute('aria-pressed')).toBe('false')
    fireEvent.click(fixedDcaCard)
    await waitFor(() => expect(screen.getByRole('button', { name: '查看每月稳步投入' }).getAttribute('aria-pressed')).toBe('true'))
    expect(screen.getAllByText('正在查看').length).toBeGreaterThan(0)
    expect(screen.queryByText('当前正在使用')).toBeNull()
    expect(screen.getAllByRole('link', { name: '分析走势' })[0].getAttribute('href')).toBe('/strategy-analysis')
  })

  it('compares selected strategies on one normalized analysis chart', () => {
    renderPage(<StrategyAnalysisPage />)
    expect(screen.getByText(/当前使用本地确定性示例序列来完成交互与视觉验证/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: /70 \/ 20 \/ 10/ }))
    expect(screen.getByRole('button', { name: /70 \/ 20 \/ 10/ }).getAttribute('aria-pressed')).toBe('true')
    fireEvent.click(screen.getByRole('button', { name: /固定定投/ }))
    expect(screen.getAllByText('固定定投', { selector: 'span' })).toHaveLength(2)
    fireEvent.click(screen.getByRole('button', { name: /70 \/ 20 \/ 10/ }))
    expect(screen.getByRole('button', { name: /70 \/ 20 \/ 10/ }).getAttribute('aria-pressed')).toBe('false')
    fireEvent.click(screen.getByRole('button', { name: '近 1 年' }))
    expect(screen.getByRole('button', { name: '近 1 年' }).getAttribute('aria-pressed')).toBe('true')
    fireEvent.click(screen.getByRole('button', { name: '全部样本' }))
    expect(screen.getByRole('button', { name: '全部样本' }).getAttribute('aria-pressed')).toBe('true')
  })

  it('keeps backend fixed-sample metrics separate from the demo curve in professional research', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(response([{ policy: { id: 'dsl_rsi_guard', version: 1 }, name: 'RSI 风险保护', document: {}, created_at: '2026-09-10T00:00:00Z' }]))
      .mockResolvedValueOnce(response({ eligible: true, core_bucket_safe: true, budget_safe: true, assets: [{ symbol: 'SPY', observations: 120, evidence_start_as_of: '2016-01-01', evidence_end_as_of: '2025-12-31', strategy: { terminal_wealth_usd: 12450, maximum_drawdown_percent: -22.5, cash_utilisation_percent: 98.4 }, fixed_dca: { xirr_percent: 8.2, terminal_wealth_usd: 12110, maximum_drawdown_percent: -24.1, annualized_volatility_percent: 18.25, sortino_ratio: 0.61, cash_utilisation_percent: 100 }, rolling_out_of_sample: [{ start_as_of: '2016-01-01', end_as_of: '2018-01-01', observations: 24, strategy: { terminal_wealth_usd: 2100, maximum_drawdown_percent: -12, cash_utilisation_percent: 99 }, fixed_dca: { terminal_wealth_usd: 2050, maximum_drawdown_percent: -14, cash_utilisation_percent: 100 } }] }] }))
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyAnalysisPage />)
    fireEvent.click(screen.getByRole('button', { name: '专业研究' }))
    expect(await screen.findByLabelText('选择专业研究策略')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: '读取固定样本研究' }))
    expect(await screen.findByText('18.25%')).toBeTruthy()
    expect(screen.getAllByText('样本不足')).toHaveLength(3)
    expect(screen.getByText('查看滚动样本外窗口')).toBeTruthy()
    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  it('explains when professional research has no saved strategy or a rejected report', async () => {
    const emptyFetch = vi.fn().mockResolvedValue(response([]))
    vi.stubGlobal('fetch', emptyFetch)
    const firstRender = renderPage(<StrategyAnalysisPage />)
    fireEvent.click(screen.getByRole('button', { name: '专业研究' }))
    expect(await screen.findByText(/还没有已保存的 DSL 策略/)).toBeTruthy()
    firstRender.unmount()

    const rejectedFetch = vi.fn()
      .mockResolvedValueOnce(response([{ policy: { id: 'dsl_guard', version: 1 }, name: '预算保护', document: {}, created_at: '2026-09-10T00:00:00Z' }]))
      .mockResolvedValueOnce(response({ eligible: false, reason: '预算约束未通过', core_bucket_safe: true, budget_safe: false, assets: [] }))
    vi.stubGlobal('fetch', rejectedFetch)
    renderPage(<StrategyAnalysisPage />)
    fireEvent.click(screen.getByRole('button', { name: '专业研究' }))
    await screen.findByLabelText('选择专业研究策略')
    fireEvent.click(screen.getByRole('button', { name: '读取固定样本研究' }))
    expect(await screen.findByText('预算约束未通过')).toBeTruthy()
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
    render(<StrategyCard strategy={findConsumerStrategy('steady-dca')} selected={false} variant="compact" onSelect={() => undefined} />)
    expect(screen.getByText('每月稳步投入')).toBeTruthy()
    expect(screen.queryByText('它会怎么做：')).toBeNull()
  })
})

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
