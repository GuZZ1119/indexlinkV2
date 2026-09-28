import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'
import { MemoryRouter, Route, Routes } from 'react-router'

import i18n from '@/i18n'
import DecisionsPage from '@/pages/decisions'

const decision = {
  id: '20000000-0000-4000-8000-000000000002',
  plan_id: '10000000-0000-4000-8000-000000000001',
  symbol: 'VOO',
  currency: 'USD',
  execution_status: 'due',
  planned_contribution: '1000.00',
  execution_snapshot: { trigger: 'scheduled' },
  fundamental_snapshot: {},
  trend_snapshot: {},
  decision_snapshot: { action: 'standard', multiplier: 1, policy: { id: 'fixed_dca', version: 1 } },
  policy_evidence: { policy: { id: 'fixed_dca', version: 1 } },
  summary: '按固定计划投入，不根据短期涨跌改变节奏。',
  created_at: '2026-09-17T00:00:00Z',
}

const jsonResponse = (body: unknown) => ({ ok: true, status: 200, json: async () => body })
const runtimeStatus = {
  service: 'running', database: 'ready', market_data: 'not_configured', historical_prices: 'not_configured',
  qwen: 'not_configured', paper_broker: 'not_configured',
  scheduler: { enabled: true, tick_seconds: 60, summary: { created: 0, catch_up_created: 0, already_claimed: 0, unavailable: 0 } },
}

describe('decision detail execution journal', () => {
  beforeAll(async () => { await i18n.changeLanguage('zh') })
  afterEach(() => { cleanup(); vi.unstubAllGlobals() })

  it('shows the immutable advice beside every user-reported event', async () => {
    vi.stubGlobal('fetch', vi.fn(async (input: string | URL | Request) => {
      const url = String(input)
      if (url.endsWith('/investment-plans')) return jsonResponse([])
      if (url.endsWith('/runtime-status')) return jsonResponse(runtimeStatus)
      if (url.endsWith(`/decisions/${decision.id}`)) return jsonResponse(decision)
      if (url.endsWith(`/decisions/${decision.id}/manual-executions`)) return jsonResponse([{
        id: '30000000-0000-4000-8000-000000000003',
        decision_record_id: decision.id,
        plan_id: decision.plan_id,
        outcome: 'partial',
        actual_amount: '400.00',
        currency: 'USD',
        note: '保留本月现金',
        occurred_at: '2026-09-17T01:00:00Z',
        recorded_at: '2026-09-17T01:01:00Z',
        source: 'user_reported',
      }])
      if (url.includes('/decisions?limit=')) return jsonResponse([])
      throw new Error(`unexpected request: ${url}`)
    }))
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={[`/decisions/${decision.id}`]}>
          <Routes><Route path="/decisions/:id" element={<DecisionsPage />} /></Routes>
        </MemoryRouter>
      </QueryClientProvider>,
    )

    expect(await screen.findByText(decision.summary)).toBeTruthy()
    expect(screen.getByText('这次计划建议')).toBeTruthy()
    expect(screen.getByText(/US\$1,000/)).toBeTruthy()
    expect(screen.getByText('固定金额与固定节奏')).toBeTruthy()
    expect(screen.getByText('建议已保存；实际操作与结果仍由你确认')).toBeTruthy()
    expect(screen.getByText('技术审计信息').closest('details')?.hasAttribute('open')).toBe(false)
    expect(await screen.findByText('部分完成')).toBeTruthy()
    expect(screen.getByText(/400\.00/)).toBeTruthy()
    expect(screen.getByText('保留本月现金')).toBeTruthy()
    expect(screen.getByText(/由你记录/)).toBeTruthy()
  })

  it('does not offer a paper order when the runtime capability is not configured', async () => {
    const approvalDecision = {
      ...decision,
      execution_snapshot: { execution: { bucket_split: { requires_approval: true } } },
    }
    const requests: string[] = []
    vi.stubGlobal('fetch', vi.fn(async (input: string | URL | Request) => {
      const url = String(input)
      requests.push(url)
      if (url.endsWith('/investment-plans')) return jsonResponse([])
      if (url.endsWith('/runtime-status')) return jsonResponse(runtimeStatus)
      if (url.endsWith(`/decisions/${decision.id}`)) return jsonResponse(approvalDecision)
      if (url.endsWith(`/decisions/${decision.id}/manual-executions`)) return jsonResponse([])
      if (url.includes('/decisions?limit=')) return jsonResponse([])
      throw new Error(`unexpected request: ${url}`)
    }))
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={[`/decisions/${decision.id}`]}>
          <Routes><Route path="/decisions/:id" element={<DecisionsPage />} /></Routes>
        </MemoryRouter>
      </QueryClientProvider>,
    )

    expect(await screen.findByText(/OpenD 模拟账户未启用/)).toBeTruthy()
    expect(screen.queryByRole('button', { name: '确认模拟下单' })).toBeNull()
    expect(requests.some((url) => url.includes('approve-paper-order'))).toBe(false)
  })

  it('explains an unavailable paper capability without exposing an order action', async () => {
    const approvalDecision = {
      ...decision,
      execution_snapshot: { execution: { bucket_split: { requires_approval: true } } },
    }
    const requests: string[] = []
    vi.stubGlobal('fetch', vi.fn(async (input: string | URL | Request) => {
      const url = String(input)
      requests.push(url)
      if (url.endsWith('/investment-plans')) return jsonResponse([])
      if (url.endsWith('/runtime-status')) return jsonResponse({ ...runtimeStatus, paper_broker: 'unavailable' })
      if (url.endsWith(`/decisions/${decision.id}`)) return jsonResponse(approvalDecision)
      if (url.endsWith(`/decisions/${decision.id}/manual-executions`)) return jsonResponse([])
      if (url.includes('/decisions?limit=')) return jsonResponse([])
      throw new Error(`unexpected request: ${url}`)
    }))
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={[`/decisions/${decision.id}`]}>
          <Routes><Route path="/decisions/:id" element={<DecisionsPage />} /></Routes>
        </MemoryRouter>
      </QueryClientProvider>,
    )

    expect(await screen.findByText(/OpenD 模拟账户当前不可用/)).toBeTruthy()
    expect(screen.queryByRole('button', { name: '确认模拟下单' })).toBeNull()
    expect(requests.some((url) => url.includes('approve-paper-order'))).toBe(false)
  })

  it('keeps the audited approval action when the paper capability is configured', async () => {
    const approvalDecision = {
      ...decision,
      execution_snapshot: { execution: { bucket_split: { requires_approval: true } } },
    }
    const requests: Array<{ url: string; method: string }> = []
    vi.stubGlobal('fetch', vi.fn(async (input: string | URL | Request, init?: RequestInit) => {
      const url = String(input)
      const method = init?.method ?? 'GET'
      requests.push({ url, method })
      if (url.endsWith('/investment-plans')) return jsonResponse([])
      if (url.endsWith('/runtime-status')) return jsonResponse({ ...runtimeStatus, paper_broker: 'configured' })
      if (url.endsWith(`/decisions/${decision.id}/approve-paper-order`) && method === 'POST') return jsonResponse({ accepted: true })
      if (url.endsWith(`/decisions/${decision.id}`)) return jsonResponse(approvalDecision)
      if (url.endsWith(`/decisions/${decision.id}/manual-executions`)) return jsonResponse([])
      if (url.includes('/decisions?limit=')) return jsonResponse([])
      throw new Error(`unexpected request: ${url}`)
    }))
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={[`/decisions/${decision.id}`]}>
          <Routes><Route path="/decisions/:id" element={<DecisionsPage />} /></Routes>
        </MemoryRouter>
      </QueryClientProvider>,
    )

    fireEvent.click(await screen.findByRole('button', { name: '确认模拟下单' }))
    await waitFor(() => expect(requests).toContainEqual({
      url: expect.stringContaining(`/decisions/${decision.id}/approve-paper-order`),
      method: 'POST',
    }))
  })
})
