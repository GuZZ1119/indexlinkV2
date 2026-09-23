import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'

import { StrategyCard } from '@/components/v2_1/strategy-card'
import { AppHeader } from '@/components/layout/app-header'
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
    expect(fetchMock).toHaveBeenCalledTimes(3)
    expect(fetchMock.mock.calls.map((call) => String(call[0]))).toEqual(expect.arrayContaining([
      expect.stringContaining('/investment-plans'),
      expect.stringContaining('/strategy-catalog'),
      expect.stringContaining('/ai/providers'),
    ]))
  })

  it('exposes plan management as My plans directly below Personal', () => {
    renderPage(<AppSidebar />)
    const personal = screen.getByRole('link', { name: '个人中心' })
    const plans = screen.getByRole('link', { name: '我的计划' })

    expect(plans.getAttribute('href')).toBe('/plans')
    expect(personal.compareDocumentPosition(plans) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('uses the project logo as the persistent home brand without duplicate wordmarks', () => {
    renderPage(<AppHeader />)

    const home = screen.getByRole('link', { name: 'IndexLink' })
    expect(home.getAttribute('href')).toBe('/personal')
    expect(home.querySelector('img')?.getAttribute('src')).toBe('/logo.png')
    expect(home.textContent).toBe('')
    expect(screen.queryByText('个人资料')).toBeNull()
    expect(screen.queryByText('退出登录')).toBeNull()
  })

  it('opens a real mobile navigation drawer from the header menu button', async () => {
    renderPage(<AppHeader />)

    fireEvent.click(screen.getByRole('button', { name: '收放侧栏' }))

    expect(await screen.findByRole('navigation', { name: '主要导航' })).toBeTruthy()
    expect(screen.getByRole('link', { name: '策略工坊' }).getAttribute('href')).toBe('/strategy-builder')
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
    expect(fixedDcaAnalysis.getAttribute('href')).toContain('strategy=fixed_dca')
    expect(screen.queryByText('正在查看')).toBeNull()
    expect(screen.queryByText('自适应长期计划')).toBeNull()
    expect(screen.getAllByRole('link', { name: '建立固定投入计划' })).toHaveLength(1)
    expect(screen.getAllByRole('link', { name: '按此参数建立计划' })).toHaveLength(2)
    expect(screen.getByText('3 个公式档位 · 2 个家族')).toBeTruthy()
    expect(screen.getAllByRole('link', { name: '查看这组参数的分析' })[0].getAttribute('href')).toContain('strategy=dsl_ma200_trend_guard')
    fireEvent.change(screen.getByRole('combobox', { name: '价格与简单均线检查参数' }), { target: { value: 'dsl_price_sma_responsive' } })
    expect(screen.getAllByRole('link', { name: '按此参数建立计划' })[0].getAttribute('href')).toContain('policy_id=dsl_price_sma_responsive')
    expect(screen.getAllByRole('link', { name: '查看这组参数的分析' })[0].getAttribute('href')).toContain('strategy=dsl_price_sma_responsive')
  })

  it('blocks plan creation when the server catalog has no eligible research result', async () => {
    const blocked = { ...strategyCatalog()[1], adoptable: false, research_status: 'blocked', research: undefined }
    const fetchMock = vi.fn().mockImplementation((url: string) => Promise.resolve(response(url.includes('/strategy-catalog') ? [blocked] : [])))
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyCenterPage />)

    expect(await screen.findByText('尚未通过，暂不可创建')).toBeTruthy()
    expect(screen.getByText('尚未通过')).toBeTruthy()
    expect(screen.queryByRole('link', { name: '按此参数建立计划' })).toBeNull()
  })

  it('separates personal immutable versions and keeps exact analysis and plan links', async () => {
    const personal = {
      ...strategyCatalog()[1],
      origin: 'personal' as const,
      lifecycle: 'saved' as const,
      status: 'usable' as const,
      policy: { id: 'dsl_personal_drawdown', version: 3 },
      name: '我的回撤规则',
      family: undefined,
      preset: undefined,
      source: undefined,
    }
    vi.stubGlobal('fetch', vi.fn().mockImplementation((url: string) => Promise.resolve(response(url.includes('/strategy-catalog') ? [...strategyCatalog(), personal] : []))))
    renderPage(<StrategyCenterPage />)

    expect(await screen.findByRole('heading', { name: '我的个人策略' })).toBeTruthy()
    expect(await screen.findByRole('heading', { name: '我的回撤规则' })).toBeTruthy()
    expect(screen.getByText('v3')).toBeTruthy()
    expect(screen.getByRole('link', { name: '用真实标的回测' }).getAttribute('href')).toContain('strategy_version=3')
    const personalPlanLink = screen.getAllByRole('link', { name: '按此参数建立计划' }).find((link) => link.getAttribute('href')?.includes('dsl_personal_drawdown'))
    expect(personalPlanLink?.getAttribute('href')).toContain('policy_version=3')
    expect(screen.getByText('3 个公式档位 · 2 个家族')).toBeTruthy()
  })

  it('filters grouped strategy families by searchable tags and category', async () => {
    vi.stubGlobal('fetch', vi.fn().mockImplementation((url: string) => Promise.resolve(response(url.includes('/strategy-catalog') ? strategyCatalog() : []))))
    renderPage(<StrategyCenterPage />)

    const search = await screen.findByRole('searchbox', { name: '搜索策略' })
    fireEvent.change(search, { target: { value: '波动率' } })
    expect(screen.getByRole('heading', { name: '增长与波动平衡' })).toBeTruthy()
    expect(screen.queryByRole('heading', { name: '价格与简单均线' })).toBeNull()

    fireEvent.change(search, { target: { value: '' } })
    fireEvent.click(screen.getByRole('button', { name: '趋势' }))
    expect(screen.getByRole('heading', { name: '价格与简单均线' })).toBeTruthy()
    expect(screen.queryByRole('heading', { name: '增长与波动平衡' })).toBeNull()
  })

  it('shows explicit empty and failed catalog states without inventing strategies', async () => {
    vi.stubGlobal('fetch', vi.fn().mockImplementation((url: string) => Promise.resolve(response(url.includes('/strategy-catalog') ? [] : []))))
    const first = renderPage(<StrategyCenterPage />)
    expect(await screen.findByText('服务当前没有发布可供普通用户采用的策略。')).toBeTruthy()
    expect(screen.getByText('目前没有正在执行的计划。已暂停计划和新建入口都在“我的计划”中。')).toBeTruthy()
    first.unmount()

    vi.stubGlobal('fetch', vi.fn().mockImplementation((url: string) => Promise.resolve(url.includes('/strategy-catalog') ? response({ error: { code: 'offline' } }, false) : response([]))))
    renderPage(<StrategyCenterPage />)
    expect(await screen.findByText('暂时无法读取策略目录')).toBeTruthy()
  })

  it('renders safe fallbacks for an ungrouped preset and an unknown weekly plan', async () => {
    const orphan = {
      ...strategyCatalog()[1],
      policy: { id: 'dsl_orphan_rule', version: 1 },
      name: '未分组规则',
      summary: '仍可由服务端执行。',
      family: undefined,
      preset: undefined,
      source: undefined,
      tags: undefined,
      validation_mode: undefined,
      data_requirement: { required_close_observations: 0 },
    }
    vi.stubGlobal('fetch', vi.fn().mockImplementation((url: string) => Promise.resolve(response(url.includes('/strategy-catalog') ? [orphan] : [investmentPlan({ policy: { id: 'dsl_unknown_saved', version: 1 }, schedule_kind: 'weekly', schedule_day: 2, base_contribution: 'not-a-number' })]))))
    renderPage(<StrategyCenterPage />)

    expect(await screen.findByRole('heading', { name: '未分组规则' })).toBeTruthy()
    expect(screen.getByText('不需要行情')).toBeTruthy()
    expect(screen.getByText('可以建立计划')).toBeTruthy()
    expect(screen.getByText(/已保存的版本策略/)).toBeTruthy()
    expect(screen.getByText(/每周第 2 天/)).toBeTruthy()
    expect(screen.getByText(/USD not-a-number/)).toBeTruthy()
  })

  it('runs selected strategies on one real normalized analysis chart', async () => {
    const fetchMock = vi.fn().mockImplementation((url: string) => Promise.resolve(response(url.includes('/strategy-catalog') ? strategyCatalog() : strategyBacktest(['fixed_dca', 'dsl_ma200_trend_guard']))))
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyAnalysisPage />)
    expect(await screen.findByText('US.SPY · 真实日线回测')).toBeTruthy()
    expect(screen.queryByText('演示数据 · 非真实回测')).toBeNull()
    expect(screen.getByRole('heading', { name: '策略净值指数（起点 = 100）' })).toBeTruthy()
    expect(screen.getByLabelText('策略净值指数说明').textContent).toContain('它不是股价，也不是账户金额')
    expect(screen.getByLabelText('重合曲线说明').textContent).toContain('不是策略缺失')
    expect(screen.getByText(/本机 OpenD \/ history-kline-v10/)).toBeTruthy()
    expect(screen.getByRole('heading', { name: 'US.SPY 走势与规则触发点' })).toBeTruthy()
    expect(screen.getByText(/不是预测出的最佳买点/)).toBeTruthy()
    expect(screen.getByLabelText('US.SPY复权收盘价与各策略规则触发点')).toBeTruthy()
    expect(screen.getByText(/悬停可查看触发日的模拟投入金额/)).toBeTruthy()
    fireEvent.change(screen.getByRole('combobox', { name: '添加对比策略' }), { target: { value: 'dsl_ma200_trend_guard@1' } })
    await waitFor(() => expect(screen.getByRole('button', { name: '移除价格与简单均线（200日）' }).getAttribute('aria-pressed')).toBe('true'))
    expect(screen.getByRole('button', { name: '移除每月稳步投入' }).getAttribute('aria-pressed')).toBe('true')
    fireEvent.click(screen.getByRole('button', { name: '近 6 个月' }))
    await waitFor(() => expect(screen.getByRole('button', { name: '近 6 个月' }).getAttribute('aria-pressed')).toBe('true'))
    fireEvent.click(screen.getByRole('button', { name: '运行真实回测' }))
    await waitFor(() => expect(fetchMock.mock.calls.filter(([url]) => String(url).includes('/strategy-backtests')).length).toBe(2))
    fireEvent.click(screen.getByRole('button', { name: '全部样本' }))
    await waitFor(() => expect(screen.getByRole('button', { name: '全部样本' }).getAttribute('aria-pressed')).toBe('true'))
    const submitted = JSON.parse(String(fetchMock.mock.calls.filter(([url]) => String(url).includes('/strategy-backtests')).at(-1)?.[1]?.body))
    expect(submitted.strategy_refs).toEqual([{ policy_id: 'fixed_dca', policy_version: 1 }, { policy_id: 'dsl_ma200_trend_guard', policy_version: 1 }])
    expect(submitted.range).toBe('6m')
  })

  it('explains a real backtest only after the user requests it', async () => {
    const profile = { id: 'gpt-local', provider: 'openai', display_name: 'GPT', model: 'gpt-local', capabilities: { market_evidence: false, restricted_policy_drafts: true, read_only_explanations: true } }
    const fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes('/ai/providers')) return Promise.resolve(response({ providers: [profile] }))
      if (url.includes('/strategy-backtests/explain')) return Promise.resolve(response({ provider: profile, source_checksum: 'abcdef123456', explanation: { headline: '这次回测怎么读', summary: '收益为正，但只代表历史区间。', observations: ['现金使用率较高。'], risks: ['未来结果可能不同。'] } }))
      if (url.includes('/strategy-catalog')) return Promise.resolve(response(strategyCatalog()))
      return Promise.resolve(response(strategyBacktest(['fixed_dca'])))
    })
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyAnalysisPage />)

    expect(await screen.findByRole('button', { name: '解释这次结果' })).toBeTruthy()
    expect(fetchMock.mock.calls.some(([url]) => String(url).includes('/strategy-backtests/explain'))).toBe(false)
    fireEvent.click(screen.getByRole('button', { name: '解释这次结果' }))
    expect(await screen.findByRole('heading', { name: '这次回测怎么读' })).toBeTruthy()
    expect(screen.getByText('收益为正，但只代表历史区间。')).toBeTruthy()
    expect(fetchMock.mock.calls.filter(([url]) => String(url).includes('/strategy-backtests/explain'))).toHaveLength(1)
  })

  it('turns an unavailable AI explanation into an actionable credential message', async () => {
    const profile = { id: 'session-qwen', provider: 'qwen', display_name: 'Qwen（本次运行）', model: 'qwen-plus', capabilities: { market_evidence: true, restricted_policy_drafts: true, read_only_explanations: true } }
    const fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes('/ai/providers')) return Promise.resolve(response({ providers: [profile] }))
      if (url.includes('/strategy-backtests/explain')) return Promise.resolve(response({ error: { code: 'service_unavailable', message: 'service is unavailable' } }, false))
      if (url.includes('/strategy-catalog')) return Promise.resolve(response(strategyCatalog()))
      return Promise.resolve(response(strategyBacktest(['fixed_dca'])))
    })
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyAnalysisPage />)

    fireEvent.click(await screen.findByRole('button', { name: '解释这次结果' }))
    expect(await screen.findByText(/重新输入有效的 API Key/)).toBeTruthy()
    expect(screen.queryByText('service is unavailable')).toBeNull()
  })

  it('generates the personal summary only after a manual click', async () => {
    const profile = { id: 'deepseek-local', provider: 'deepseek', display_name: 'DeepSeek', model: 'deepseek-chat', capabilities: { market_evidence: false, restricted_policy_drafts: true, read_only_explanations: true } }
    const fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes('/ai/providers')) return Promise.resolve(response({ providers: [profile] }))
      if (url.includes('/personal/ai-summary')) return Promise.resolve(response({ provider: profile, plan_count: 0, decision_count: 0, explanation: { headline: '近期没有计划', summary: '目前没有本机计划可整理。', observations: ['没有计划记录。'], risks: ['摘要不代表券商账户状态。'] } }))
      if (url.includes('/strategy-catalog')) return Promise.resolve(response([]))
      return Promise.resolve(response([]))
    })
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<PersonalPage />)

    expect(await screen.findByRole('button', { name: '手动生成摘要' })).toBeTruthy()
    expect(fetchMock.mock.calls.some(([url]) => String(url).includes('/personal/ai-summary'))).toBe(false)
    fireEvent.click(screen.getByRole('button', { name: '手动生成摘要' }))
    expect(await screen.findByRole('heading', { name: '近期没有计划' })).toBeTruthy()
    expect(fetchMock.mock.calls.filter(([url]) => String(url).includes('/personal/ai-summary'))).toHaveLength(1)
  })

  it('does not report distinct normalized paths as overlapping', async () => {
    const backtest = strategyBacktest(['fixed_dca', 'dsl_ma200_trend_guard'])
    backtest.result.series[1].normalized_points[1].value = 104.6
    vi.stubGlobal('fetch', vi.fn().mockImplementation((url: string) => Promise.resolve(response(url.includes('/strategy-catalog') ? strategyCatalog() : backtest))))
    renderPage(<StrategyAnalysisPage />)

    expect(await screen.findByText('US.SPY · 真实日线回测')).toBeTruthy()
    expect(screen.queryByLabelText('重合曲线说明')).toBeNull()
  })

  it('opens the requested catalog strategy in the plain analysis view', async () => {
    vi.stubGlobal('fetch', vi.fn().mockImplementation((url: string) => Promise.resolve(response(url.includes('/strategy-catalog') ? strategyCatalog() : strategyBacktest(['dsl_ma200_trend_guard'])))))
    renderPage(<StrategyAnalysisPage />, '/strategy-analysis?strategy=dsl_ma200_trend_guard&view=plain')
    expect(await screen.findByText('US.SPY · 真实日线回测')).toBeTruthy()
    expect(screen.getByRole('button', { name: '价格与简单均线（200日）（至少保留一个）' }).getAttribute('aria-pressed')).toBe('true')
    expect(screen.queryByRole('button', { name: /每月稳步投入/ })).toBeNull()
    expect(screen.getByRole('button', { name: '直观视角' }).getAttribute('aria-pressed')).toBe('true')
  })

  it('uses a deep link only to initialize and then respects the user comparison', async () => {
    const fetchMock = vi.fn().mockImplementation((url: string, options?: RequestInit) => {
      if (url.includes('/strategy-catalog')) return Promise.resolve(response(strategyCatalog()))
      const ids = options?.body ? JSON.parse(String(options.body)).strategy_refs.map((item: { policy_id: string }) => item.policy_id) : ['dsl_ma200_trend_guard']
      return Promise.resolve(response(strategyBacktest(ids)))
    })
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyAnalysisPage />, '/strategy-analysis?strategy=dsl_ma200_trend_guard&view=plain')

    expect(await screen.findByRole('button', { name: '价格与简单均线（200日）（至少保留一个）' })).toBeTruthy()
    fireEvent.change(screen.getByLabelText('回测标的'), { target: { value: 'HK.00700' } })
    fireEvent.change(screen.getByLabelText('每月投入金额'), { target: { value: '2500' } })
    fireEvent.change(screen.getByRole('combobox', { name: '添加对比策略' }), { target: { value: 'fixed_dca@1' } })
    fireEvent.click(await screen.findByRole('button', { name: '移除价格与简单均线（200日）' }))
    fireEvent.click(screen.getByRole('button', { name: '运行真实回测' }))

    await waitFor(() => {
      const submitted = JSON.parse(String(fetchMock.mock.calls.at(-1)?.[1]?.body))
      expect(submitted.strategy_refs).toEqual([{ policy_id: 'fixed_dca', policy_version: 1 }])
      expect(submitted.symbol).toBe('HK.00700')
      expect(submitted.contribution).toBe('2500')
    })
  })

  it('waits for and then fails closed when a deep-linked catalog cannot load', async () => {
    let rejectCatalog: (() => void) | undefined
    vi.stubGlobal('fetch', vi.fn().mockImplementation((url: string) => {
      if (!url.includes('/strategy-catalog')) return Promise.resolve(response(strategyBacktest(['fixed_dca'])))
      return new Promise((resolve) => { rejectCatalog = () => resolve(response({ error: { code: 'offline' } }, false)) })
    }))
    renderPage(<StrategyAnalysisPage />, '/strategy-analysis?strategy=dsl_ma200_trend_guard&view=plain')
    expect(screen.getByText('正在确认策略目录…')).toBeTruthy()
    await act(async () => { rejectCatalog?.() })
    expect(await screen.findByText('暂时无法确认这个策略')).toBeTruthy()
  })

  it('does not silently replace an unknown policy id with Fixed DCA', async () => {
    const fetchMock = vi.fn().mockImplementation((url: string) => Promise.resolve(response(url.includes('/strategy-catalog') ? strategyCatalog() : strategyBacktest(['fixed_dca']))))
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyAnalysisPage />, '/strategy-analysis?strategy=dsl_removed_strategy&view=plain')

    expect(await screen.findByText('策略目录中没有这个策略')).toBeTruthy()
    expect(screen.getByText(/dsl_removed_strategy/)).toBeTruthy()
    expect(fetchMock.mock.calls.some(([url]) => String(url).includes('/strategy-backtests'))).toBe(false)
  })

  it('can select the last preset from a 100-policy catalog without a frontend allowlist', async () => {
    const template = strategyCatalog()[1]
    const largeCatalog = [strategyCatalog()[0], ...Array.from({ length: 99 }, (_, index) => ({
      ...template,
      policy: { id: `dsl_generated_${String(index + 1).padStart(3, '0')}`, version: 1 },
      name: `规则预设 ${index + 1}`,
    }))]
    const fetchMock = vi.fn().mockImplementation((url: string) => Promise.resolve(response(url.includes('/strategy-catalog') ? largeCatalog : strategyBacktest(['fixed_dca']))))
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyAnalysisPage />)

    expect(await screen.findByText('US.SPY · 真实日线回测')).toBeTruthy()
    fireEvent.change(screen.getByRole('combobox', { name: '添加对比策略' }), { target: { value: 'dsl_generated_099@1' } })
    expect(await screen.findByRole('button', { name: '移除规则预设 99' })).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: '运行真实回测' }))

    await waitFor(() => expect(fetchMock.mock.calls.filter(([url]) => String(url).includes('/strategy-backtests')).length).toBe(2))
    const submitted = JSON.parse(String(fetchMock.mock.calls.filter(([url]) => String(url).includes('/strategy-backtests')).at(-1)?.[1]?.body))
    expect(submitted.strategy_refs).toEqual([{ policy_id: 'fixed_dca', policy_version: 1 }, { policy_id: 'dsl_generated_099', policy_version: 1 }])
  })

  it('uses the same real response for professional metrics', async () => {
    const fetchMock = vi.fn().mockImplementation((url: string) => Promise.resolve(response(url.includes('/strategy-catalog') ? strategyCatalog() : strategyBacktest(['fixed_dca']))))
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<StrategyAnalysisPage />, '/strategy-analysis?view=research')
    fireEvent.click(screen.getByRole('button', { name: '专业研究' }))
    expect(await screen.findByLabelText('真实专业回测指标')).toBeTruthy()
    expect(screen.getAllByText('+8.2%').length).toBeGreaterThan(0)
    expect(screen.getByText('-18.4%')).toBeTruthy()
    expect(screen.getByText(/每条策略均评估 12 个计划日/)).toBeTruthy()
    expect(screen.getByLabelText('区间收益计算公式').textContent).toContain('NAV')
    fireEvent.click(screen.getByRole('button', { name: '年化收益' }))
    expect((await screen.findByLabelText('年化收益计算公式')).textContent).toContain('365.25')
    expect(screen.getByRole('heading', { name: '资金与执行事实' })).toBeTruthy()
    expect(screen.getByRole('heading', { name: '回撤发生在什么时候' })).toBeTruthy()
    expect(screen.getByLabelText('策略每日回撤曲线')).toBeTruthy()
    expect(screen.getByRole('heading', { name: '每期资金去了哪里' })).toBeTruthy()
    expect(screen.getByLabelText('每月稳步投入每期核心、机会和未投入资金')).toBeTruthy()
    expect(fetchMock.mock.calls.filter(([url]) => String(url).includes('/strategy-backtests'))).toHaveLength(1)
  })

  it('never replaces an unavailable provider with a demo chart', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response({ error: { code: 'service_unavailable', message: 'offline' } }, false)))
    renderPage(<StrategyAnalysisPage />)
    expect(await screen.findByText('真实行情源未连接')).toBeTruthy()
    expect(screen.getByText(/需要本机 OpenD/)).toBeTruthy()
    expect(screen.getByRole('link', { name: '前往高级实验室查看数据连接' }).getAttribute('href')).toBe('/lab')
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
    expect(screen.queryByText('真实行情源未连接')).toBeNull()
  })

  it('configures AI from the Lab only after an explicit submit and never re-renders the key', async () => {
    let configured = false
    const profile = { id: 'session-claude', provider: 'claude', display_name: 'Claude（本次运行）', model: 'claude-test', capabilities: { market_evidence: true, restricted_policy_drafts: true, read_only_explanations: true } }
    const fetchMock = vi.fn().mockImplementation(async (url: string, options?: RequestInit) => {
      if (url.endsWith('/ai/providers')) return response({ providers: configured ? [profile] : [] })
      if (url.endsWith('/ai/session-provider') && options?.method === 'POST') {
        configured = true
        return response({ provider: profile, storage: 'process_memory' })
      }
      if (url.endsWith('/ai/session-provider/test') && options?.method === 'POST') {
        return response({ provider: profile, status: 'available' })
      }
      if (url.endsWith('/ai/session-provider') && options?.method === 'DELETE') {
        configured = false
        return { ok: true, status: 204, json: async () => null }
      }
      throw new Error(`unexpected ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock)

    renderPage(<LabPage />)
    expect(screen.queryByText('MA200 一年历史回放')).toBeNull()
    expect(fetchMock).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: '输入 API 配置' }))
    await screen.findByLabelText('AI 服务商')
    expect((screen.getByLabelText('AI 服务商') as HTMLSelectElement).value).toBe('qwen_cloud')
    expect((screen.getByLabelText('AI 模型名称') as HTMLInputElement).value).toBe('qwen3.8-max')
    fireEvent.change(screen.getByLabelText('AI 服务商'), { target: { value: 'qwen' } })
    expect((screen.getByLabelText('AI 模型名称') as HTMLInputElement).value).toBe('qwen-plus')
    fireEvent.change(screen.getByLabelText('AI 服务商'), { target: { value: 'claude' } })
    fireEvent.change(screen.getByLabelText('AI 模型名称'), { target: { value: 'claude-test' } })
    fireEvent.change(screen.getByLabelText('AI API Key'), { target: { value: 'private-browser-key' } })
    fireEvent.click(screen.getByRole('button', { name: '保存本次连接' }))

    expect(await screen.findByText(/凭据已保存到当前后端进程，但尚未向供应商验证/)).toBeTruthy()
    expect(screen.getByText('凭据已保存 · 尚未验证')).toBeTruthy()
    expect((screen.getByLabelText('AI API Key') as HTMLInputElement).value).toBe('')
    const request = fetchMock.mock.calls.find(([url, options]) => String(url).endsWith('/ai/session-provider') && options?.method === 'POST')
    expect(request).toBeTruthy()
    expect(JSON.parse(String(request?.[1]?.body))).toEqual({ provider: 'claude', model: 'claude-test', api_key: 'private-browser-key' })
    expect(document.body.textContent).not.toContain('private-browser-key')
    fireEvent.click(screen.getByRole('button', { name: '验证 AI 可用性' }))
    expect(await screen.findByText('AI 连接可用')).toBeTruthy()
    expect(screen.getByText('连接已验证')).toBeTruthy()
    expect(fetchMock.mock.calls.filter(([url, options]) => String(url).endsWith('/ai/session-provider/test') && options?.method === 'POST')).toHaveLength(1)
    fireEvent.click(await screen.findByRole('button', { name: '清除连接' }))
    await waitFor(() => expect(screen.queryByRole('button', { name: '清除连接' })).toBeNull())
  })

  it('turns a provider probe failure into actionable account guidance', async () => {
    let configured = false
    const profile = { id: 'session-qwen-cloud', provider: 'qwen-cloud', display_name: 'QwenCloud（本次运行）', model: 'qwen3.8-max', capabilities: { market_evidence: true, restricted_policy_drafts: true, read_only_explanations: true } }
    const fetchMock = vi.fn().mockImplementation(async (url: string, options?: RequestInit) => {
      if (url.endsWith('/ai/providers')) return response({ providers: configured ? [profile] : [] })
      if (url.endsWith('/ai/session-provider') && options?.method === 'POST') {
        configured = true
        return response({ provider: profile, storage: 'process_memory' })
      }
      if (url.endsWith('/ai/session-provider/test') && options?.method === 'POST') {
        return response({ provider: profile, status: 'access_denied' })
      }
      throw new Error(`unexpected ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock)

    renderPage(<LabPage />)
    fireEvent.click(screen.getByRole('button', { name: '输入 API 配置' }))
    fireEvent.change(await screen.findByLabelText('AI API Key'), { target: { value: 'unit-test-private-key' } })
    fireEvent.click(screen.getByRole('button', { name: '保存本次连接' }))
    fireEvent.click(await screen.findByRole('button', { name: '验证 AI 可用性' }))

    expect(await screen.findByText('账户权限或计费未开通')).toBeTruthy()
    expect(screen.getByText(/Pay-As-You-Go 计费状态/)).toBeTruthy()
    expect(screen.getByText('验证未通过')).toBeTruthy()
  })

  it('configures a loopback read-only OpenD source without enabling broker access', async () => {
    let configured = false
    const fetchMock = vi.fn().mockImplementation(async (url: string, options?: RequestInit) => {
      if (url.endsWith('/runtime-status')) return response({ market_data: configured ? 'configured' : 'not_configured', historical_prices: configured ? 'configured' : 'not_configured' })
      if (url.endsWith('/market-data/session-opend') && options?.method === 'POST') {
        configured = true
        return response({ provider: 'opend', host: '127.0.0.1', port: 11111, storage: 'process_memory', access: 'read_only_market_data' })
      }
      if (url.endsWith('/market-data/session-opend') && options?.method === 'DELETE') {
        configured = false
        return { ok: true, status: 204, json: async () => null }
      }
      throw new Error(`unexpected ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<LabPage />)
    fireEvent.click(screen.getByRole('button', { name: '输入 OpenD 配置' }))
    expect((await screen.findByLabelText('OpenD 本机地址') as HTMLInputElement).value).toBe('127.0.0.1')
    fireEvent.click(screen.getByRole('button', { name: '保存只读连接' }))

    expect(await screen.findByText(/没有开启模拟券商或订单权限/)).toBeTruthy()
    const request = fetchMock.mock.calls.find(([url, options]) => String(url).endsWith('/market-data/session-opend') && options?.method === 'POST')
    expect(JSON.parse(String(request?.[1]?.body))).toEqual({ host: '127.0.0.1', port: 11111 })
    fireEvent.click(await screen.findByRole('button', { name: '清除本次连接' }))
    await waitFor(() => expect(screen.queryByRole('button', { name: '清除本次连接' })).toBeNull())
  })

  it('keeps the entered key editable when the local backend rejects an AI configuration', async () => {
    const fetchMock = vi.fn().mockImplementation(async (url: string, options?: RequestInit) => {
      if (url.endsWith('/ai/providers')) return response({ providers: [] })
      if (url.endsWith('/ai/session-provider') && options?.method === 'POST') {
        return { ok: false, status: 400, json: async () => ({ error: { code: 'bad_request', message: 'invalid request' } }) }
      }
      throw new Error(`unexpected ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<LabPage />)
    fireEvent.click(screen.getByRole('button', { name: '输入 API 配置' }))
    fireEvent.change(await screen.findByLabelText('AI API Key'), { target: { value: 'key-to-correct' } })
    fireEvent.click(screen.getByRole('button', { name: '保存本次连接' }))

    expect((await screen.findByRole('alert')).textContent).toContain('连接配置未保存')
    expect((screen.getByLabelText('AI API Key') as HTMLInputElement).value).toBe('key-to-correct')
  })

  it('explains an invalid OpenD address without claiming a connection', async () => {
    const fetchMock = vi.fn().mockImplementation(async (url: string, options?: RequestInit) => {
      if (url.endsWith('/runtime-status')) return response({ market_data: 'not_configured', historical_prices: 'not_configured' })
      if (url.endsWith('/market-data/session-opend') && options?.method === 'POST') {
        return { ok: false, status: 400, json: async () => ({ error: { code: 'bad_request', message: 'invalid request' } }) }
      }
      throw new Error(`unexpected ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock)
    renderPage(<LabPage />)
    fireEvent.click(screen.getByRole('button', { name: '输入 OpenD 配置' }))
    fireEvent.change(await screen.findByLabelText('OpenD 本机地址'), { target: { value: '192.0.2.10' } })
    fireEvent.click(screen.getByRole('button', { name: '保存只读连接' }))

    expect((await screen.findByRole('alert')).textContent).toContain('地址必须是本机回环 IP')
    expect(screen.queryByText('只读行情已配置')).toBeNull()
  })

  it('renders a compact strategy card without the rule panel', () => {
    render(<MemoryRouter><StrategyCard strategy={strategyCatalog()[0]} analysisHref="/strategy-analysis?strategy=steady-dca&view=plain" variant="compact" /></MemoryRouter>)
    expect(screen.getByText('每月稳步投入')).toBeTruthy()
    expect(screen.queryByText('它会怎么做：')).toBeNull()
  })

  it('labels formula-card validation honestly when no embedded research is present', () => {
    const generated = strategyCatalog()[1]
    const first = render(<MemoryRouter><StrategyCard strategy={generated} analysisHref="/strategy-analysis?strategy=dsl_price_sma_responsive"><span>建立入口</span></StrategyCard></MemoryRouter>)
    expect(screen.getByText('等待完整数据')).toBeTruthy()
    expect(screen.getByText('已通过准入')).toBeTruthy()
    expect(screen.getByText('它会怎么做：')).toBeTruthy()
    expect(screen.getByText('建立入口')).toBeTruthy()
    first.unmount()

    render(<MemoryRouter><StrategyCard strategy={{ ...generated, risk: 'balanced', adoptable: false, research_status: 'blocked' }} analysisHref="/strategy-analysis" /></MemoryRouter>)
    expect(screen.getByText('平衡')).toBeTruthy()
    expect(screen.getByText('暂不可采用')).toBeTruthy()
  })
})

function strategyCatalog() {
  const base = {
    origin: 'official' as const,
    lifecycle: 'published' as const,
    status: 'usable' as const,
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
    { ...base, policy: { id: 'fixed_dca', version: 1 }, name: '每月稳步投入', summary: '固定日期投入。', rule: '按计划金额投入。', limitation: '不会主动降低回撤。', tags: ['固定定投'], validation_mode: 'reference' as const },
    { ...base, policy: { id: 'dsl_price_sma_responsive', version: 1 }, name: '价格与简单均线（50日）', summary: '管理弹性投入。', rule: '低于短均线时暂停弹性桶。', limitation: '均线具有滞后性。', data_requirement: { required_close_observations: 51 }, family: { id: 'price_sma', name: '价格与简单均线', description: '用价格相对长期简单均线的位置控制弹性投入。', category: '趋势' }, preset: { id: 'responsive', name: '50日', order: 1 }, source: { name: 'Meb Faber', url: 'https://mebfaber.com/white-papers/', license: 'research reference', adaptation: 'independent' }, tags: ['均线', '趋势'], validation_mode: 'compiled_formula' as const, default_plan: { ...base.default_plan, core_ratio: '0.7', opportunity_ratio: '0.3', risk_mode: 'approval' as const }, research_status: 'available' as const },
    { ...base, policy: { id: 'dsl_ma200_trend_guard', version: 1 }, name: '价格与简单均线（200日）', summary: '管理弹性投入。', rule: '低于均线时暂停弹性桶。', limitation: '均线具有滞后性。', data_requirement: { required_close_observations: 201 }, family: { id: 'price_sma', name: '价格与简单均线', description: '用价格相对长期简单均线的位置控制弹性投入。', category: '趋势' }, preset: { id: 'balanced', name: '200日', order: 3 }, source: { name: 'Meb Faber', url: 'https://mebfaber.com/white-papers/', license: 'research reference', adaptation: 'independent' }, tags: ['均线', '趋势'], validation_mode: 'fixed_fixture' as const, default_plan: { ...base.default_plan, core_ratio: '0.7', opportunity_ratio: '0.3', risk_mode: 'approval' as const }, research_status: 'available' as const, research: admission },
    { ...base, policy: { id: 'dsl_growth_volatility_balance', version: 1 }, name: '增长与波动平衡（增长126日 / 波动63日）', summary: '检查增长与波动。', rule: '按阈值调整弹性桶。', limitation: '震荡期可能切换。', risk: 'balanced' as const, data_requirement: { required_close_observations: 127 }, family: { id: 'growth_vol', name: '增长与波动平衡', description: '同时观察中期增长和近期波动。', category: '复合' }, preset: { id: 'balanced', name: '增长126日 / 波动63日', order: 3 }, source: { name: 'IndexLink', url: 'https://github.com/GuZZ1119/indexlinkV2', license: 'MIT', adaptation: 'native' }, tags: ['增长', '波动率', '复合'], validation_mode: 'fixed_fixture' as const, default_plan: { ...base.default_plan, core_ratio: '0.7', opportunity_ratio: '0.3', risk_mode: 'approval' as const }, research_status: 'available' as const, research: admission },
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
      market_points: [{ date: '2023-09-18', adjusted_close: 100 }, { date: '2026-09-18', adjusted_close: 118.2 }],
      series: strategyIds.map((strategyId) => ({
        strategy_id: strategyId,
        strategy_version: 1,
        strategy_name: strategyId,
        normalized_points: [{ date: '2023-09-18', value: 100 }, { date: '2026-09-18', value: 108.2 }],
        execution_points: [{ date: '2023-09-18', adjusted_close: 100, invested_amount: strategyId === 'fixed_dca' ? 1000 : 700, budget_utilisation_percent: strategyId === 'fixed_dca' ? 100 : 70, scheduled_contribution_amount: 1000, core_invested_amount: strategyId === 'fixed_dca' ? 1000 : 700, opportunity_invested_amount: 0, unallocated_amount: strategyId === 'fixed_dca' ? 0 : 300, transaction_cost: strategyId === 'fixed_dca' ? 0.5 : 0.35, strategy_rule_matched: strategyId !== 'fixed_dca' }],
        drawdown_points: [{ date: '2023-09-18', value_percent: 0 }, { date: '2026-09-18', value_percent: -4.2 }],
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
        calculation_details: {
          elapsed_days: 1096,
          daily_return_count: 751,
          mean_daily_return_percent: 0.03,
          daily_standard_deviation_percent: 0.8,
          downside_deviation_percent: 0.5,
          drawdown_peak_date: '2025-02-19',
          drawdown_trough_date: '2025-04-08',
          drawdown_recovery_date: '2025-06-27',
          total_transaction_cost: strategyId === 'fixed_dca' ? 6 : 4.2,
          rule_matched_count: strategyId === 'fixed_dca' ? 0 : 1,
          standard_execution_count: strategyId === 'fixed_dca' ? 1 : 0,
          trading_periods_per_year: 252,
          calendar_days_per_year: 365.25,
          buy_cost_bps: 5,
        },
      })),
    },
  }
}
