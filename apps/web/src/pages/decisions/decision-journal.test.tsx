import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter, Route, Routes } from 'react-router'

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

describe('decision detail execution journal', () => {
  afterEach(() => { cleanup(); vi.unstubAllGlobals() })

  it('shows the immutable advice beside every user-reported event', async () => {
    vi.stubGlobal('fetch', vi.fn(async (input: string | URL | Request) => {
      const url = String(input)
      if (url.endsWith('/investment-plans')) return jsonResponse([])
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
    expect(await screen.findByText('部分完成')).toBeTruthy()
    expect(screen.getByText(/400\.00/)).toBeTruthy()
    expect(screen.getByText('保留本月现金')).toBeTruthy()
    expect(screen.getByText(/由你记录/)).toBeTruthy()
  })
})
