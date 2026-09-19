import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter, Route, Routes } from 'react-router'

import PlansPage from '@/pages/plans'
import { setSelectedPlanId } from '@/stores/ui'

const createdPlan = {
  id: '10000000-0000-4000-8000-000000000001',
  name: 'VOO 长期计划',
  symbol: 'VOO',
  base_contribution: '800.00',
  currency: 'USD',
  schedule_kind: 'monthly',
  schedule_day: new Date().getUTCDate() > 28 ? 28 : new Date().getUTCDate(),
  schedule_days: [new Date().getUTCDate() > 28 ? 28 : new Date().getUTCDate()],
  policy: { id: 'fixed_dca', version: 1 },
  execution_configuration: {
    bucket_allocation: { core_ratio: '1.00', opportunity_ratio: '0.00' },
    risk_mode: 'fixed',
    opportunity_cash_policy: 'expire_each_period',
  },
  max_single_execution: '800.00',
  is_active: true,
  created_at: '2026-09-17T00:00:00Z',
  updated_at: '2026-09-17T00:00:00Z',
}

const jsonResponse = (body: unknown, status = 200) => ({
  ok: status >= 200 && status < 300,
  status,
  json: async () => body,
})

function renderPage(initialEntry = '/plans') {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={[initialEntry]}>
        <Routes>
          <Route path="/plans" element={<PlansPage />} />
          <Route path="/personal" element={<p>个人中心已打开</p>} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('minimal fixed DCA plan setup', () => {
  beforeEach(() => setSelectedPlanId(null))
  afterEach(() => { cleanup(); vi.unstubAllGlobals() })

  it('creates a zero-dependency fixed DCA plan and prepares its real advice', async () => {
    const requests: Array<{ method: string; url: string; body?: Record<string, unknown> }> = []
    vi.stubGlobal('fetch', vi.fn(async (input: string | URL | Request, init?: RequestInit) => {
      const url = String(input)
      const method = init?.method ?? 'GET'
      const body = init?.body ? JSON.parse(String(init.body)) as Record<string, unknown> : undefined
      requests.push({ method, url, body })
      if (method === 'GET' && url.endsWith('/investment-plans')) return jsonResponse([])
      if (method === 'GET' && url.endsWith('/strategy-catalog')) return jsonResponse([])
      if (method === 'POST' && url.endsWith('/investment-plans')) return jsonResponse(createdPlan, 201)
      if (method === 'POST' && url.includes('/automatic-decision-preview')) return jsonResponse({ record: { id: 'decision-1' } }, 201)
      throw new Error(`unexpected request: ${method} ${url}`)
    }))
    renderPage()

    fireEvent.change(await screen.findByLabelText('投资标的'), { target: { value: ' voo ' } })
    fireEvent.change(screen.getByLabelText('每次投入金额（USD）'), { target: { value: '800.00' } })
    fireEvent.click(screen.getByRole('button', { name: /建立并查看本期安排/ }))

    expect(await screen.findByText('个人中心已打开')).toBeTruthy()
    const create = requests.find((request) => request.method === 'POST' && request.url.endsWith('/investment-plans'))
    expect(create?.body).toMatchObject({
      name: 'VOO 长期计划',
      symbol: 'VOO',
      base_contribution: '800.00',
      currency: 'USD',
      policy: { id: 'fixed_dca', version: 1 },
      bucket_allocation: { core_ratio: '1.00', opportunity_ratio: '0.00' },
      risk_mode: 'fixed',
      opportunity_cash_policy: 'expire_each_period',
      max_single_execution: '800.00',
    })
    expect(requests.find((request) => request.url.includes('/automatic-decision-preview'))?.body).toEqual({})
  })

  it('creates the selected official Formula plan with its server-owned bucket defaults', async () => {
    const requests: Array<{ method: string; url: string; body?: Record<string, unknown> }> = []
    const formulaPlan = {
      ...createdPlan,
      name: 'VOO 200 日均线趋势保护',
      policy: { id: 'dsl_ma200_trend_guard', version: 1 },
      execution_configuration: {
        bucket_allocation: { core_ratio: '0.70', opportunity_ratio: '0.30' },
        risk_mode: 'approval',
        opportunity_cash_policy: 'expire_each_period',
      },
    }
    vi.stubGlobal('fetch', vi.fn(async (input: string | URL | Request, init?: RequestInit) => {
      const url = String(input)
      const method = init?.method ?? 'GET'
      const body = init?.body ? JSON.parse(String(init.body)) as Record<string, unknown> : undefined
      requests.push({ method, url, body })
      if (method === 'GET' && url.endsWith('/investment-plans')) return jsonResponse([])
      if (method === 'GET' && url.endsWith('/strategy-catalog')) return jsonResponse([formulaCatalogEntry])
      if (method === 'POST' && url.endsWith('/investment-plans')) return jsonResponse(formulaPlan, 201)
      if (method === 'POST' && url.includes('/automatic-decision-preview')) return jsonResponse({ record: { id: 'decision-formula' } }, 201)
      throw new Error(`unexpected request: ${method} ${url}`)
    }))
    renderPage('/plans?policy_id=dsl_ma200_trend_guard&policy_version=1#new-plan')

    expect(await screen.findByRole('heading', { name: '建立“200 日均线趋势保护”计划' })).toBeTruthy()
    expect(screen.getByText('当前策略支持：SPY、VOO。')).toBeTruthy()
    fireEvent.change(screen.getByLabelText('投资标的'), { target: { value: 'VOO' } })
    fireEvent.change(screen.getByLabelText('每次投入金额（USD）'), { target: { value: '800.00' } })
    fireEvent.click(screen.getByRole('button', { name: /建立并查看本期安排/ }))

    expect(await screen.findByText('个人中心已打开')).toBeTruthy()
    const create = requests.find((request) => request.method === 'POST' && request.url.endsWith('/investment-plans'))
    expect(create?.body).toMatchObject({
      name: 'VOO 200 日均线趋势保护',
      policy: { id: 'dsl_ma200_trend_guard', version: 1 },
      bucket_allocation: { core_ratio: '0.7', opportunity_ratio: '0.3' },
      risk_mode: 'approval',
    })
  })

  it('does not create a Formula plan for a symbol outside the official allowlist', async () => {
    const fetchMock = vi.fn(async (input: string | URL | Request, init?: RequestInit) => {
      void init
      const url = String(input)
      if (url.endsWith('/strategy-catalog')) return jsonResponse([formulaCatalogEntry])
      if (url.endsWith('/investment-plans')) return jsonResponse([])
      throw new Error(`unexpected request: ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock)
    renderPage('/plans?policy_id=dsl_ma200_trend_guard&policy_version=1#new-plan')

    await screen.findByRole('heading', { name: '建立“200 日均线趋势保护”计划' })
    fireEvent.change(screen.getByLabelText('投资标的'), { target: { value: 'QQQ' } })
    fireEvent.click(screen.getByRole('button', { name: /建立并查看本期安排/ }))

    expect((await screen.findByRole('alert')).textContent).toContain('当前策略只支持 SPY、VOO')
    expect(fetchMock.mock.calls.some(([, init]) => (init as RequestInit | undefined)?.method === 'POST')).toBe(false)
  })

  it('retries advice preparation without creating the saved plan twice', async () => {
    let createCount = 0
    let previewCount = 0
    vi.stubGlobal('fetch', vi.fn(async (input: string | URL | Request, init?: RequestInit) => {
      const url = String(input)
      const method = init?.method ?? 'GET'
      if (method === 'GET') return jsonResponse([])
      if (url.endsWith('/investment-plans')) {
        createCount += 1
        return jsonResponse(createdPlan, 201)
      }
      if (url.includes('/automatic-decision-preview')) {
        previewCount += 1
        return previewCount === 1
          ? jsonResponse({ error: { code: 'unavailable', message: 'retry' } }, 503)
          : jsonResponse({ record: { id: 'decision-1' } }, 201)
      }
      throw new Error(`unexpected request: ${method} ${url}`)
    }))
    renderPage()

    fireEvent.change(await screen.findByLabelText('投资标的'), { target: { value: 'VOO' } })
    fireEvent.click(screen.getByRole('button', { name: /建立并查看本期安排/ }))
    expect((await screen.findByRole('status')).textContent).toContain('计划已经保存')
    fireEvent.click(screen.getByRole('button', { name: '重新准备本期安排' }))

    expect(await screen.findByText('个人中心已打开')).toBeTruthy()
    expect(createCount).toBe(1)
    expect(previewCount).toBe(2)
  })

  it('keeps weekly cadence understandable and manages existing plans', async () => {
    const requests: Array<{ method: string; url: string; body?: Record<string, unknown> }> = []
    vi.stubGlobal('fetch', vi.fn(async (input: string | URL | Request, init?: RequestInit) => {
      const url = String(input)
      const method = init?.method ?? 'GET'
      const body = init?.body ? JSON.parse(String(init.body)) as Record<string, unknown> : undefined
      requests.push({ method, url, body })
      if (method === 'GET') return jsonResponse([{ ...createdPlan, schedule_kind: 'weekly', schedule_day: 3, schedule_days: [3] }])
      if (method === 'PATCH') return jsonResponse({ ...createdPlan, is_active: false })
      if (method === 'DELETE') return { ...jsonResponse(undefined, 204), json: async () => undefined }
      throw new Error(`unexpected request: ${method} ${url}`)
    }))
    renderPage()

    expect(screen.getByRole('heading', { name: '所有长期计划，都在这里' })).toBeTruthy()
    expect(await screen.findByText('每周 星期三')).toBeTruthy()
    expect(screen.getByText(/策略：固定定投 · 单次上限/)).toBeTruthy()
    const planHeading = screen.getByRole('heading', { name: '你的长期计划' })
    const createHeading = screen.getByRole('heading', { name: '从一份简单的固定定投开始' })
    expect(planHeading.compareDocumentPosition(createHeading) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    fireEvent.change(screen.getByLabelText('执行节奏'), { target: { value: 'weekly' } })
    expect(screen.getByLabelText('每周哪一天')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: '暂停' }))
    await waitFor(() => expect(requests.some((request) => request.method === 'PATCH' && request.body?.is_active === false)).toBe(true))
    fireEvent.click(screen.getByRole('button', { name: '删除' }))
    expect(screen.getByRole('dialog')).toBeTruthy()
    expect(screen.getByRole('heading', { name: `删除“${createdPlan.name}”？` })).toBeTruthy()
    expect(requests.some((request) => request.method === 'DELETE')).toBe(false)
    fireEvent.click(screen.getByRole('button', { name: '确认删除' }))
    await waitFor(() => expect(requests.some((request) => request.method === 'DELETE')).toBe(true))
  })

  it('shows a paused monthly plan and keeps a rejected create or delete safe', async () => {
    const requests: Array<{ method: string; url: string }> = []
    vi.stubGlobal('fetch', vi.fn(async (input: string | URL | Request, init?: RequestInit) => {
      const url = String(input)
      const method = init?.method ?? 'GET'
      requests.push({ method, url })
      if (method === 'GET') return jsonResponse([{ ...createdPlan, base_contribution: 'unknown', policy: { id: 'core_opportunity_v1', version: 1 }, is_active: false }])
      if (method === 'POST' && url.endsWith('/investment-plans')) return jsonResponse({ error: { code: 'invalid', message: 'invalid' } }, 400)
      throw new Error(`unexpected request: ${method} ${url}`)
    }))
    renderPage()

    expect(await screen.findByText(`每月 ${createdPlan.schedule_day} 日`)).toBeTruthy()
    expect(screen.getByText('USD unknown')).toBeTruthy()
    expect(screen.getAllByText(/旧自适应策略/).length).toBeGreaterThan(0)
    expect(screen.getByRole('button', { name: '继续' })).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: '删除' }))
    expect(screen.getByRole('dialog')).toBeTruthy()
    expect(requests.some((request) => request.method === 'DELETE')).toBe(false)
    fireEvent.click(screen.getByRole('button', { name: '保留计划' }))
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(requests.some((request) => request.method === 'DELETE')).toBe(false)

    fireEvent.change(screen.getByLabelText('投资标的'), { target: { value: 'VOO' } })
    fireEvent.change(screen.getByLabelText('计划名称（可选）'), { target: { value: ' 安稳计划 ' } })
    fireEvent.click(screen.getByRole('button', { name: /建立并查看本期安排/ }))
    expect((await screen.findByRole('alert')).textContent).toContain('计划没有保存成功')
  })
})

const formulaCatalogEntry = {
  policy: { id: 'dsl_ma200_trend_guard', version: 1 },
  name: '200 日均线趋势保护',
  summary: '保留固定核心投入，在价格低于 200 日均线时暂停当期弹性投入。',
  rule: '低于均线时弹性桶为 0。',
  limitation: '均线具有滞后性。',
  risk: 'stable',
  supported_symbols: ['SPY', 'VOO'],
  default_plan: { schedule_kind: 'monthly', schedule_day: 18, core_ratio: '0.7', opportunity_ratio: '0.3', risk_mode: 'approval' },
  data_requirements: ['daily_close_200'],
  adoptable: true,
  research_status: 'available',
}
