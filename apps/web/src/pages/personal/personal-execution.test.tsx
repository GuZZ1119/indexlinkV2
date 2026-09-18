import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'

import PersonalPage from '@/pages/personal'
import { setSelectedPlanId } from '@/stores/ui'

const plan = {
  id: '10000000-0000-4000-8000-000000000001',
  name: '我的标普长期计划',
  symbol: 'VOO',
  base_contribution: '1000.00',
  currency: 'USD',
  schedule_kind: 'monthly',
  schedule_day: 15,
  schedule_days: [15],
  policy: { id: 'fixed_dca', version: 1 },
  execution_configuration: {
    bucket_allocation: { core_ratio: '1.00', opportunity_ratio: '0.00' },
    risk_mode: 'fixed',
    opportunity_cash_policy: 'expire_each_period',
  },
  max_single_execution: '1500.00',
  is_active: true,
  created_at: '2026-09-01T00:00:00Z',
  updated_at: '2026-09-01T00:00:00Z',
}

const decision = {
  id: '20000000-0000-4000-8000-000000000002',
  plan_id: plan.id,
  symbol: 'VOO',
  currency: 'USD',
  execution_status: 'due',
  planned_contribution: '1000.00',
  execution_snapshot: {},
  fundamental_snapshot: {},
  trend_snapshot: {},
  decision_snapshot: { action: 'standard', multiplier: 1 },
  summary: '按固定计划投入，不根据短期涨跌改变节奏。',
  created_at: '2026-09-15T00:00:00Z',
}

type TestEvent = {
  id: string
  decision_record_id: string
  plan_id: string
  outcome: 'executed' | 'partial' | 'skipped'
  actual_amount?: string
  currency: string
  note?: string
  occurred_at: string
  recorded_at: string
  source: 'user_reported'
}

const jsonResponse = (body: unknown, status = 200) => ({
  ok: status >= 200 && status < 300,
  status,
  json: async () => body,
})

function renderPage() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
  return render(<QueryClientProvider client={queryClient}><MemoryRouter><PersonalPage /></MemoryRouter></QueryClientProvider>)
}

function createApi(options: {
  plans?: unknown[]
  decisions?: unknown[]
  events?: TestEvent[]
  failHistory?: boolean
  postStatus?: number
} = {}) {
  const events = options.events ?? []
  const requests: Array<{ url: string; method: string; body?: Record<string, unknown> }> = []
  const fetchMock = vi.fn(async (input: string | URL | Request, init?: RequestInit) => {
    const url = String(input)
    const method = init?.method ?? 'GET'
    const body = init?.body ? JSON.parse(String(init.body)) as Record<string, unknown> : undefined
    requests.push({ url, method, body })
    if (url.endsWith('/investment-plans')) return jsonResponse(options.plans ?? [plan])
    if (url.includes('/investment-plans/') && url.includes('/decisions')) return jsonResponse(options.decisions ?? [decision])
    if (url.includes(`/decisions/${decision.id}/manual-executions`) && method === 'POST') {
      if (options.postStatus) return jsonResponse({ error: { code: 'bad_request', message: 'invalid' } }, options.postStatus)
      const created: TestEvent = {
        id: String(body?.event_id),
        decision_record_id: decision.id,
        plan_id: plan.id,
        outcome: body?.outcome as TestEvent['outcome'],
        actual_amount: body?.actual_amount as string | undefined,
        currency: 'USD',
        note: body?.note as string | undefined,
        occurred_at: String(body?.occurred_at),
        recorded_at: '2026-09-16T01:00:00Z',
        source: 'user_reported',
      }
      events.push(created)
      return jsonResponse(created, 201)
    }
    if (url.includes(`/decisions/${decision.id}/manual-executions`)) {
      if (options.failHistory) return jsonResponse({ error: { code: 'unavailable', message: 'offline' } }, 503)
      return jsonResponse(events)
    }
    throw new Error(`unexpected request: ${method} ${url}`)
  })
  return { fetchMock, requests, events }
}

describe('personal manual execution loop', () => {
  beforeEach(() => setSelectedPlanId(null))
  afterEach(() => { cleanup(); vi.unstubAllGlobals() })

  it('reads the real advice, confirms an executed amount, and refreshes history', async () => {
    const api = createApi()
    vi.stubGlobal('fetch', api.fetchMock)
    renderPage()

    expect(await screen.findByRole('heading', { name: /按建议投入/ })).toBeTruthy()
    expect(screen.getByText(/保持原来的金额和节奏/)).toBeTruthy()
    expect(screen.queryByText(decision.summary)).toBeNull()
    expect(await screen.findByText(/还没有执行记录/)).toBeTruthy()
    const pendingStatus = screen.getByLabelText('本期办理状态')
    expect(pendingStatus.textContent).toContain('本期待办 · 未完成')
    expect(pendingStatus.closest('section')?.getAttribute('data-advice-state')).toBe('pending')
    expect(pendingStatus.closest('section')?.className).toContain('duration-700')

    expect(screen.queryByRole('button', { name: '部分执行' })).toBeNull()
    fireEvent.click(await screen.findByRole('button', { name: '我已执行' }))
    const amount = screen.getByLabelText('实际金额') as HTMLInputElement
    expect(amount.value).toBe('1000.00')
    fireEvent.change(amount, { target: { value: '980.50' } })
    fireEvent.change(screen.getByLabelText('备注（可选）'), { target: { value: ' 已在券商完成 ' } })
    fireEvent.click(screen.getByRole('button', { name: '确认记录完成' }))

    expect(await screen.findByText('已完成，已加入执行历史。')).toBeTruthy()
    expect(await screen.findByText(/980\.50/)).toBeTruthy()
    expect(screen.getByText('已在券商完成')).toBeTruthy()
    const post = api.requests.find((request) => request.method === 'POST')
    expect(post?.body).toMatchObject({ outcome: 'executed', actual_amount: '980.50', note: '已在券商完成' })
    expect(post?.body?.event_id).toMatch(/^[0-9a-f-]{36}$/)
    expect(screen.queryByRole('button', { name: '我已执行' })).toBeNull()
    expect(screen.getAllByText(/每个计划日只能确认一次/).length).toBeGreaterThan(0)
    const completedStatus = screen.getByLabelText('本期办理状态')
    expect(completedStatus.textContent).toContain('本期待办 · 已完成')
    expect(completedStatus.closest('section')?.getAttribute('data-advice-state')).toBe('completed')
  })

  it('validates an executed amount locally and can record one skipped outcome without an amount', async () => {
    const invalid = createApi()
    vi.stubGlobal('fetch', invalid.fetchMock)
    const first = renderPage()
    fireEvent.click(await screen.findByRole('button', { name: '我已执行' }))
    fireEvent.change(screen.getByLabelText('实际金额'), { target: { value: '0' } })
    fireEvent.click(screen.getByRole('button', { name: '确认记录完成' }))
    expect(screen.getByRole('alert').textContent).toContain('请输入大于 0')
    expect(invalid.requests.filter((request) => request.method === 'POST')).toHaveLength(0)
    first.unmount()

    const skipped = createApi()
    vi.stubGlobal('fetch', skipped.fetchMock)
    renderPage()
    fireEvent.click(await screen.findByRole('button', { name: '这次跳过' }))
    expect(screen.queryByLabelText('实际金额')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: '确认记录跳过' }))
    expect(await screen.findByText('已跳过，已加入执行历史。')).toBeTruthy()

    const posts = skipped.requests.filter((request) => request.method === 'POST')
    expect(posts).toHaveLength(1)
    expect(posts[0].body).toMatchObject({ outcome: 'skipped' })
    expect(posts[0].body).not.toHaveProperty('actual_amount')
    expect(await screen.findByText('已跳过')).toBeTruthy()
    expect(screen.queryByRole('button', { name: '这次跳过' })).toBeNull()
    const skippedStatus = screen.getByLabelText('本期办理状态')
    expect(skippedStatus.textContent).toContain('本期待办 · 已跳过')
    expect(skippedStatus.closest('section')?.getAttribute('data-advice-state')).toBe('skipped')
  })

  it('keeps the real no-advice and journal failure states explicit', async () => {
    const waitingDecision = { ...decision, execution_status: 'waiting' }
    const api = createApi({ decisions: [waitingDecision] })
    vi.stubGlobal('fetch', api.fetchMock)
    const first = renderPage()
    expect(await screen.findByRole('heading', { name: '现在只需要继续等待' })).toBeTruthy()
    expect(screen.getByText(/下一次计划日/)).toBeTruthy()
    expect(screen.queryByRole('button', { name: '我已执行' })).toBeNull()
    expect(api.requests.some((request) => request.url.includes('manual-executions'))).toBe(false)
    first.unmount()

    const failed = createApi({ failHistory: true })
    vi.stubGlobal('fetch', failed.fetchMock)
    renderPage()
    expect(await screen.findByText('暂时读不到执行历史。')).toBeTruthy()
    expect(screen.getByLabelText('本期办理状态').textContent).toContain('执行状态待确认')
    fireEvent.click(screen.getByRole('button', { name: '重新读取' }))
    await waitFor(() => expect(failed.requests.filter((request) => request.url.includes('manual-executions'))).toHaveLength(2))
  })

  it('keeps a legacy partial result visibly resolved without presenting it as completed', async () => {
    const partial: TestEvent = {
      id: '30000000-0000-4000-8000-000000000003',
      decision_record_id: decision.id,
      plan_id: plan.id,
      outcome: 'partial',
      actual_amount: '500.00',
      currency: 'USD',
      occurred_at: '2026-09-15T01:00:00Z',
      recorded_at: '2026-09-15T01:01:00Z',
      source: 'user_reported',
    }
    const api = createApi({ events: [partial] })
    vi.stubGlobal('fetch', api.fetchMock)
    renderPage()

    const status = await screen.findByLabelText('本期办理状态')
    await waitFor(() => expect(status.textContent).toContain('本期待办 · 历史部分完成'))
    expect(status.closest('section')?.getAttribute('data-advice-state')).toBe('partial')
    expect(screen.queryByRole('button', { name: '我已执行' })).toBeNull()
  })

  it('supports switching between real plans and shows service failures without mock data', async () => {
    const weeklyPlan = {
      ...plan,
      id: '10000000-0000-4000-8000-000000000009',
      name: '每周储蓄计划',
      schedule_kind: 'weekly',
      schedule_days: [1, 4],
      is_active: false,
    }
    const api = createApi({ plans: [plan, weeklyPlan], decisions: [] })
    vi.stubGlobal('fetch', api.fetchMock)
    const first = renderPage()
    const selector = await screen.findByLabelText('正在查看的计划') as HTMLSelectElement
    fireEvent.change(selector, { target: { value: weeklyPlan.id } })
    await waitFor(() => expect(api.requests.some((request) => request.url.includes(weeklyPlan.id))).toBe(true))
    first.unmount()

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse({ error: { code: 'unavailable', message: 'offline' } }, 503)))
    renderPage()
    expect((await screen.findByRole('alert')).textContent).toContain('本机服务暂时没有响应')
  })

  it('keeps invalid time, cancellation, and server validation failures inside the confirmation card', async () => {
    const api = createApi({ postStatus: 400 })
    vi.stubGlobal('fetch', api.fetchMock)
    renderPage()
    fireEvent.click(await screen.findByRole('button', { name: '我已执行' }))
    fireEvent.change(screen.getByLabelText('实际时间'), { target: { value: '' } })
    fireEvent.click(screen.getByRole('button', { name: '确认记录完成' }))
    expect(screen.getByRole('alert').textContent).toContain('请选择有效的执行时间')
    fireEvent.click(screen.getByRole('button', { name: '取消' }))
    expect(screen.queryByLabelText('实际金额')).toBeNull()

    fireEvent.click(await screen.findByRole('button', { name: '我已执行' }))
    fireEvent.click(screen.getByRole('button', { name: '确认记录完成' }))
    expect((await screen.findByRole('alert')).textContent).toContain('没有通过校验')
  })

  it('renders server action wording and real weekly paused plan facts', async () => {
    const weeklyPlan = { ...plan, schedule_kind: 'weekly', schedule_days: [1, 4], is_active: false }
    const skippedDecision = { ...decision, planned_contribution: undefined, decision_snapshot: { ...decision.decision_snapshot, action: 'skip' } }
    const skipped = createApi({ plans: [weeklyPlan], decisions: [skippedDecision] })
    vi.stubGlobal('fetch', skipped.fetchMock)
    const first = renderPage()
    expect(await screen.findByRole('heading', { name: '本期不需要投入' })).toBeTruthy()
    expect(screen.getByText('每周 1、4')).toBeTruthy()
    expect(screen.getByText('已暂停')).toBeTruthy()
    first.unmount()

    const delayedDecision = { ...decision, decision_snapshot: { ...decision.decision_snapshot, action: 'tactical_delay' } }
    const delayed = createApi({ decisions: [delayedDecision] })
    vi.stubGlobal('fetch', delayed.fetchMock)
    const second = renderPage()
    expect(await screen.findByRole('heading', { name: '本期先等等' })).toBeTruthy()
    second.unmount()

    const amountless = createApi({ decisions: [{ ...decision, planned_contribution: undefined }] })
    vi.stubGlobal('fetch', amountless.fetchMock)
    renderPage()
    expect(await screen.findByRole('heading', { name: '查看本期建议' })).toBeTruthy()
    fireEvent.click(await screen.findByRole('button', { name: '我已执行' }))
    expect((screen.getByLabelText('实际金额') as HTMLInputElement).value).toBe('')
  })

  it('treats a retry conflict as an already-recorded event and refreshes the journal', async () => {
    const existing: TestEvent[] = []
    const api = createApi({ events: existing })
    let postSeen = false
    api.fetchMock.mockImplementation(async (input: string | URL | Request, init?: RequestInit) => {
      const url = String(input)
      const method = init?.method ?? 'GET'
      const body = init?.body ? JSON.parse(String(init.body)) as Record<string, unknown> : undefined
      api.requests.push({ url, method, body })
      if (url.endsWith('/investment-plans')) return jsonResponse([plan])
      if (url.includes(`/investment-plans/${plan.id}/decisions`)) return jsonResponse([decision])
      if (url.includes('manual-executions') && method === 'POST') {
        postSeen = true
        existing.push({
          id: String(body?.event_id), decision_record_id: decision.id, plan_id: plan.id,
          outcome: 'executed', actual_amount: '1000.00', currency: 'USD',
          occurred_at: String(body?.occurred_at), recorded_at: '2026-09-16T01:00:00Z', source: 'user_reported',
        })
        return jsonResponse({ error: { code: 'conflict', message: 'resource already exists' } }, 409)
      }
      if (url.includes('manual-executions')) return jsonResponse(existing)
      throw new Error(`unexpected request: ${method} ${url}`)
    })
    vi.stubGlobal('fetch', api.fetchMock)
    renderPage()
    fireEvent.click(await screen.findByRole('button', { name: '我已执行' }))
    fireEvent.click(screen.getByRole('button', { name: '确认记录完成' }))

    expect(await screen.findByText('这次记录已经存在，执行历史已为你刷新。')).toBeTruthy()
    expect(postSeen).toBe(true)
    await waitFor(() => expect(screen.getByLabelText('执行历史').textContent).toContain('1,000.00'))
  })
})
