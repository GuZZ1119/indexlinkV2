import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'

import { StrategyCard } from '@/components/v2_1/strategy-card'
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

  it('shows one clear monthly action and makes its local-only result visible', () => {
    const fetchMock = vi.fn()
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<PersonalPage />)
    expect(screen.getByRole('heading', { name: '按计划投入 ¥2,200' })).toBeTruthy()
    expect(screen.queryByText('MA200 一年历史回放')).toBeNull()
    expect(fetchMock).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: '我已完成这次投入' }))
    expect(screen.getByRole('status').textContent).toContain('当前浏览器会话')
    expect(screen.getByText('6 / 6 次')).toBeTruthy()
  })

  it('explains and compares strategies before a user selects one', () => {
    renderPage(<StrategyCenterPage />)
    fireEvent.click(screen.getAllByRole('button', { name: '选用这个策略' })[0])
    expect(screen.getByRole('status').textContent).toContain('个人中心已同步更新')
    fireEvent.click(screen.getByRole('button', { name: '和其他策略对比' }))
    expect(screen.getByText('最该知道的限制')).toBeTruthy()
    fireEvent.change(screen.getByLabelText('选择对比策略'), { target: { value: 'defensive-balance' } })
    expect(screen.getByText('股债平衡')).toBeTruthy()
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
