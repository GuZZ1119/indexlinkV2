import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import StrategyBuilderPage from './index'

describe('personal strategy builder page', () => {
  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
  })

  it('validates and saves only a restricted personal formula before returning to the catalog', async () => {
    const fetchMock = vi.fn().mockImplementation(async (url: string, options?: RequestInit) => {
      if (url.endsWith('/ai/providers')) return response({ providers: [] })
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
    const strategyCalls = fetchMock.mock.calls.filter(([url]) => String(url).endsWith('/strategies/validate') || String(url).endsWith('/strategies'))
    expect(strategyCalls).toHaveLength(2)
    const validated = JSON.parse(String(strategyCalls[0][1]?.body))
    expect(validated.policy_id).toMatch(/^dsl_personal_[a-z0-9]+$/)
    expect(validated.policy_version).toBe(1)
    expect(validated.rules[0].action).toEqual({ kind: 'set_opportunity_multiplier', multiplier: 1.2 })
    expect(JSON.stringify(validated)).not.toContain('javascript')
    await waitFor(() => expect(queryClient.getQueryState(['strategy-catalog'])?.isInvalidated ?? true).toBe(true))
  })

  it('calls AI only after a click and applies a bounded draft without saving it', async () => {
    const sessionProfile = { id: 'session-qwen-cloud', provider: 'qwen-cloud', display_name: 'QwenCloud（本次运行）', model: 'qwen3.8-max', capabilities: { market_evidence: true, restricted_policy_drafts: true, read_only_explanations: true } }
    const fetchMock = vi.fn().mockImplementation(async (url: string) => {
      if (url.endsWith('/ai/providers')) return response({ providers: [
        { id: 'qwen-default', provider: 'qwen', display_name: 'Qwen', model: 'qwen-plus', capabilities: { market_evidence: true, restricted_policy_drafts: true, read_only_explanations: true } },
        sessionProfile,
      ] })
      if (url.endsWith('/strategies/copilot-draft')) return response({
        provider: sessionProfile,
        document: {
          policy_id: 'dsl_personal_test', policy_version: 1, name: '回撤时增加机会额度', rules: [{
            condition: { kind: 'comparison', expression: { kind: 'indicator', indicator: { kind: 'drawdown', lookback_days: 63 } }, operator: 'less_than', threshold: '-0.1' },
            action: { kind: 'set_opportunity_multiplier', multiplier: 1.2 },
          }],
        },
        explanation: '只调整机会额度。', warnings: ['先回测。'], evidence: [],
      })
      throw new Error(`unexpected ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock)
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
    render(<QueryClientProvider client={queryClient}><MemoryRouter initialEntries={['/strategy-builder']}><StrategyBuilderPage /></MemoryRouter></QueryClientProvider>)

    expect(screen.getByRole('link', { name: '策略工坊' }).getAttribute('aria-current')).toBe('page')
    const modelSelect = await screen.findByRole('combobox', { name: '草案 AI 模型' })
    expect(modelSelect.className).toContain('min-w-0')
    expect(modelSelect.closest('section')?.className).toContain('overflow-hidden')
    expect((modelSelect as HTMLSelectElement).value).toBe('session-qwen-cloud')
    expect(fetchMock.mock.calls.some(([url]) => String(url).includes('/copilot-draft'))).toBe(false)
    fireEvent.change(screen.getByLabelText('策略自然语言描述'), { target: { value: '跌得多时增加机会投入' } })
    fireEvent.click(screen.getByRole('button', { name: '生成草案' }))
    expect(await screen.findByText('只调整机会额度。')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: '放入表单继续检查' }))
    expect((screen.getByLabelText('策略名称') as HTMLInputElement).value).toBe('回撤时增加机会额度')
    expect(fetchMock.mock.calls.filter(([url]) => String(url).includes('/copilot-draft'))).toHaveLength(1)
    const copilotRequest = fetchMock.mock.calls.find(([url]) => String(url).includes('/copilot-draft'))
    expect(JSON.parse(String(copilotRequest?.[1]?.body)).profile_id).toBe('session-qwen-cloud')
    expect(fetchMock.mock.calls.some(([url]) => String(url).endsWith('/strategies'))).toBe(false)
  })

  it('explains an unavailable AI provider without exposing the raw API envelope', async () => {
    const fetchMock = vi.fn().mockImplementation(async (url: string) => {
      if (url.endsWith('/ai/providers')) return response({ providers: [{ id: 'session-qwen', provider: 'qwen', display_name: 'Qwen（本次运行）', model: 'qwen-plus', capabilities: { market_evidence: true, restricted_policy_drafts: true, read_only_explanations: true } }] })
      if (url.endsWith('/strategies/copilot-draft')) return { ok: false, status: 503, json: async () => ({ error: { code: 'service_unavailable', message: 'service is unavailable' } }) }
      throw new Error(`unexpected ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock)
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
    render(<QueryClientProvider client={queryClient}><MemoryRouter initialEntries={['/strategy-builder']}><StrategyBuilderPage /></MemoryRouter></QueryClientProvider>)

    fireEvent.change(await screen.findByLabelText('策略自然语言描述'), { target: { value: '三个月跌幅超过 20% 时增加投入' } })
    fireEvent.click(screen.getByRole('button', { name: '生成草案' }))
    expect(await screen.findByText(/重新输入有效的 API Key/)).toBeTruthy()
    expect(screen.queryByText('service is unavailable')).toBeNull()
  })

  it('distinguishes a provider reply that misses the workshop form contract', async () => {
    const fetchMock = vi.fn().mockImplementation(async (url: string) => {
      if (url.endsWith('/ai/providers')) return response({ providers: [{ id: 'session-qwen-cloud', provider: 'qwen-cloud', display_name: 'QwenCloud（本次运行）', model: 'qwen3.8-max', capabilities: { market_evidence: true, restricted_policy_drafts: true, read_only_explanations: true } }] })
      if (url.endsWith('/strategies/copilot-draft')) return { ok: false, status: 502, json: async () => ({ error: { code: 'ai_draft_invalid', message: 'AI draft did not match the strategy form contract' } }) }
      throw new Error(`unexpected ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock)
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
    render(<QueryClientProvider client={queryClient}><MemoryRouter initialEntries={['/strategy-builder']}><StrategyBuilderPage /></MemoryRouter></QueryClientProvider>)

    fireEvent.change(await screen.findByLabelText('策略自然语言描述'), { target: { value: '三个月跌幅超过 20% 时增加投入' } })
    fireEvent.click(screen.getByRole('button', { name: '生成草案' }))

    expect(await screen.findByText(/指标、周期、阈值或额度超出策略工坊支持范围/)).toBeTruthy()
    expect(screen.queryByText(/API Key/)).toBeNull()
  })
})

function response(body: unknown) {
  return { ok: true, status: 200, json: async () => body }
}
