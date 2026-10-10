import type { IncomingMessage } from 'node:http'
import { describe, expect, it } from 'vitest'

import config, { apiProxy } from './vite.config'

describe('Vite API proxy', () => {
  it('forwards the consumer strategy catalog to the local Rust service', () => {
    expect(apiProxy['/strategy-catalog']).toBe('http://127.0.0.1:8080')
    expect(apiProxy['/strategy-backtests']).toBe('http://127.0.0.1:8080')
  })

  it('forwards manually triggered AI endpoints to the local Rust service', () => {
    expect(apiProxy['/ai']).toBe('http://127.0.0.1:8080')
    expect(apiProxy['/personal'].target).toBe('http://127.0.0.1:8080')
  })

  it.each(['/personal', '/personal/', '/personal?from=bookmark', '/decisions', '/decisions/', '/decisions/audit-id', '/decisions/audit-id/?tab=audit'])('serves the SPA for HTML document navigation to %s', (url) => {
    const proxy = url.startsWith('/personal') ? apiProxy['/personal'] : apiProxy['/decisions']
    const request = { method: 'GET', url, headers: { accept: 'text/html,application/xhtml+xml;q=0.9' } } as IncomingMessage
    expect(proxy.bypass(request)).toBe('/index.html')
  })

  it.each([
    ['GET', '/decisions/audit-id', 'application/json'],
    ['GET', '/decisions', undefined],
    ['GET', '/decisions/audit-id/manual-executions', 'text/html'],
    ['POST', '/personal/ai-summary', 'text/html'],
    ['POST', '/decisions/audit-id/approve-paper-order', 'application/json'],
    ['POST', '/decisions/audit-id/manual-executions', 'application/json'],
  ])('keeps %s %s on the Rust API', (method, url, accept) => {
    const request = { method, url, headers: { accept } } as IncomingMessage
    const proxy = url.startsWith('/personal') ? apiProxy['/personal'] : apiProxy['/decisions']
    expect(proxy.bypass(request)).toBeUndefined()
  })

  it('uses the same proxy boundary for development and production preview', () => {
    const resolved = config as { server: { proxy: unknown }; preview: { proxy: unknown } }
    expect(resolved.server.proxy).toBe(apiProxy)
    expect(resolved.preview.proxy).toBe(apiProxy)
  })
})
