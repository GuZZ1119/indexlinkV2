import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import StrategyBuilderPage from './index'

describe('personal strategy builder page', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('validates and saves only a restricted personal formula before returning to the catalog', async () => {
    const fetchMock = vi.fn().mockImplementation(async (url: string, options?: RequestInit) => {
      const document = JSON.parse(String(options?.body))
      if (url.endsWith('/strategies/validate')) return response({ valid: true, document })
      if (url.endsWith('/strategies')) return response({ policy: { id: document.policy_id, version: 1 }, name: document.name, document, created_at: '2026-09-22T00:00:00Z' })
      throw new Error(`unexpected ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock)
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
    render(<QueryClientProvider client={queryClient}><MemoryRouter initialEntries={['/strategy-builder']}><Routes><Route path="/strategy-builder" element={<StrategyBuilderPage />} /><Route path="/strategy-center" element={<p>策略中心已打开</p>} /></Routes></MemoryRouter></QueryClientProvider>)

    fireEvent.change(screen.getByLabelText('策略名称'), { target: { value: '下跌时增加机会投入' } })
    fireEvent.click(screen.getByRole('button', { name: '验证并放入策略中心' }))

    expect(await screen.findByText('策略中心已打开')).toBeTruthy()
    expect(fetchMock).toHaveBeenCalledTimes(2)
    const validated = JSON.parse(String(fetchMock.mock.calls[0][1]?.body))
    expect(validated.policy_id).toMatch(/^dsl_personal_[a-z0-9]+$/)
    expect(validated.policy_version).toBe(1)
    expect(validated.rules[0].action).toEqual({ kind: 'set_opportunity_multiplier', multiplier: 1.2 })
    expect(JSON.stringify(validated)).not.toContain('javascript')
    await waitFor(() => expect(queryClient.getQueryState(['strategy-catalog'])?.isInvalidated ?? true).toBe(true))
  })
})

function response(body: unknown) {
  return { ok: true, status: 200, json: async () => body }
}
