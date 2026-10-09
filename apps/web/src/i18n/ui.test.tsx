import ts from 'typescript'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { AppHeader } from '@/components/layout/app-header'
import PersonalPage from '@/pages/personal'
import PlansPage from '@/pages/plans'
import StrategyCenterPage from '@/pages/strategy-center'
import StrategyBuilderPage from '@/pages/strategy-builder'
import StrategyAnalysisPage from '@/pages/strategy-analysis'
import LabPage from '@/pages/lab'
import type { StrategyCatalogEntry } from '@/api/types'
import i18n, { appLanguage } from './index'
import { uiText, uiLocale } from './ui'
import { uiEnglish } from './ui-messages'
import { catalogEnglish } from './catalog-messages'
import { catalogText, localizeCatalog } from './catalog'

afterEach(async () => { cleanup(); vi.unstubAllGlobals(); await i18n.changeLanguage('zh') })

function mount(page: React.ReactNode, route: string) {
  const fetchMock = vi.fn(async (url: string) => ({ ok: !url.includes('/strategy-backtests'), status: url.includes('/strategy-backtests') ? 503 : 200, json: async () => url.endsWith('/ai/providers') ? { providers: [] } : [] }))
  vi.stubGlobal('fetch', fetchMock)
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
  render(<QueryClientProvider client={client}><MemoryRouter initialEntries={[route]}><AppHeader />{page}</MemoryRouter></QueryClientProvider>)
  return fetchMock
}

describe('language switching on current routes', () => {
  it.each([
    [PersonalPage, '/personal', '今天，只做计划要求的事'],
    [PlansPage, '/plans', '所有长期计划，都在这里'],
    [StrategyCenterPage, '/strategy-center', '先选方法，再挑适合你的参数'],
    [StrategyBuilderPage, '/strategy-builder', '把想法写成一条能验证的规则'],
    [StrategyAnalysisPage, '/strategy-analysis', '同一只标的，同一段时间，再比较策略'],
    [LabPage, '/lab', '把复杂连接，留在一个清楚的地方'],
  ])('switches a current page in place and persists the preference', async (Page, route, title) => {
    mount(<Page />, route)
    expect(screen.getByRole('heading', { level: 1, name: title })).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: '切换语言' }))
    await waitFor(() => expect(screen.getByRole('heading', { level: 1, name: uiEnglish[title] })).toBeTruthy())
    expect(document.documentElement.lang).toBe('en')
    expect(localStorage.getItem('i18nextLng')).toBe('en')
    fireEvent.click(screen.getByRole('button', { name: 'Switch language' }))
    await waitFor(() => expect(screen.getByRole('heading', { level: 1, name: title })).toBeTruthy())
    expect(document.documentElement.lang).toBe('zh')
  })

  it('keeps user-authored drafts and changes the static editor options without calling AI', async () => {
    const fetchMock = mount(<StrategyBuilderPage />, '/strategy-builder')
    fireEvent.change(screen.getByLabelText('策略名称'), { target: { value: '我的退休计划' } })
    const lookback = screen.getByLabelText('观察天数')
    fireEvent.change(lookback, { target: { value: '126' } })
    fireEvent.click(screen.getByRole('button', { name: '切换语言' }))
    await screen.findByRole('heading', { name: 'Turn your idea into a testable rule' })
    expect((screen.getByLabelText('Strategy name') as HTMLInputElement).value).toBe('我的退休计划')
    expect((screen.getByLabelText('Lookback days') as HTMLInputElement).value).toBe('126')
    expect(screen.getByRole('option', { name: 'Price return' })).toBeTruthy()
    expect(fetchMock.mock.calls.every(([url]) => url.endsWith('/ai/providers'))).toBe(true)
  })

  it('normalizes regional browser locales and interpolates UI labels without interpreting punctuation as paths', async () => {
    expect(appLanguage('zh-Hant-HK')).toBe('zh')
    expect(appLanguage('en-AU')).toBe('en')
    await i18n.changeLanguage('en-AU')
    expect(uiLocale()).toBe('en-US')
    expect(uiText('个人中心')).toBe('My space')
    expect(uiText('删除优先规则 {{p0}}', { p0: 2 })).toBe('Remove priority rule 2')
    await i18n.changeLanguage('zh-CN')
    expect(uiLocale()).toBe('zh-CN')
    expect(uiText('删除优先规则 {{p0}}', { p0: 2 })).toBe('删除优先规则 2')
  })
})

describe('translation completeness and evidence boundary', () => {
  it('has English copy for all current source-keyed UI messages', () => {
    const files = import.meta.glob<string>(['../pages/{personal,plans,strategy-center,strategy-builder,strategy-analysis,lab}/*.{ts,tsx}', '../components/v2_1/*.tsx', '../features/v2_1/*.ts', '../api/queries.ts'], { eager: true, query: '?raw', import: 'default' })
    const messages = { ...catalogEnglish, ...uiEnglish }
    for (const [file, code] of Object.entries(files).filter(([file]) => !file.includes('.test.'))) {
      const source = ts.createSourceFile(file, code, ts.ScriptTarget.Latest, true)
      const visit = (node: ts.Node) => {
        if (ts.isJsxText(node)) expect(node.text, `${file}: untranslated JSX`).not.toMatch(/[\u4e00-\u9fff]/)
        if (ts.isJsxAttribute(node) && node.initializer && ts.isStringLiteral(node.initializer)) expect(node.initializer.text, `${file}: untranslated attribute`).not.toMatch(/[\u4e00-\u9fff]/)
        if (ts.isCallExpression(node) && node.expression.getText(source) === 'uiText' && node.arguments[0] && ts.isStringLiteral(node.arguments[0])) {
          expect(messages[node.arguments[0].text], `${file}: ${node.arguments[0].text}`).toBeTruthy()
        }
        ts.forEachChild(node, visit)
      }
      visit(source)
    }
    for (const [source, target] of Object.entries(messages)) {
      expect(target).not.toMatch(/[\u4e00-\u9fff]/)
      expect(target.match(/\{\{p\d+\}\}/g)?.sort() ?? []).toEqual(source.match(/\{\{p\d+\}\}/g)?.sort() ?? [])
    }
  })

  it('projects official catalog copy without modifying cached source data or personal strategies', async () => {
    const official = { origin: 'official', name: '价格与简单均线（50日）', summary: '用价格相对长期简单均线的位置控制弹性投入。', rule: '50 日收益低于 -10% 时暂停弹性桶，低于 -4% 时减半。', limitation: '均线确认较慢，快速反转时可能延后恢复弹性投入。', family: { id: 'price_sma', name: '价格与简单均线', description: '用价格相对长期简单均线的位置控制弹性投入。', category: '趋势' }, preset: { id: 'balanced', name: '趋势150日 / 波动63日', order: 3 }, policy: { id: 'dsl_price_sma_fast', version: 1 } } as StrategyCatalogEntry
    const personal = { ...official, origin: 'personal', name: '我的规则', rule: '我自己的文字' } as StrategyCatalogEntry
    const entries = [official, personal]
    const snapshot = JSON.stringify(entries)
    await i18n.changeLanguage('en')
    const translated = localizeCatalog(entries)
    expect(translated[0].name).toBe('Price vs simple moving average 50d')
    expect(translated[0].rule).toContain('50-day returns are below -10%')
    expect(translated[0].rule).toContain('-4%')
    expect(translated[0].preset?.name).toBe('Trend 150d / Volatility 63d')
    expect(translated[0].policy).toBe(official.policy)
    expect(translated[1]).toBe(personal)
    expect(JSON.stringify(entries)).toBe(snapshot)
    expect(catalogText('未知的供应商原文')).toBe('未知的供应商原文')
    expect(catalogText('未知说明（保留标点）')).toBe('未知说明（保留标点）')
    expect(catalogText('价格与简单均线 · 均衡')).toBe('Price vs simple moving average · Balanced')
    expect(catalogText('20/50日')).toBe('20/50d')
    await act(async () => { await i18n.changeLanguage('zh') })
    expect(localizeCatalog(entries)).toBe(entries)
  })
})
