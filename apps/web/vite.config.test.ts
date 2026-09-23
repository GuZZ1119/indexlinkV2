import { describe, expect, it } from 'vitest'

import { apiProxy } from './vite.config'

describe('Vite API proxy', () => {
  it('forwards the consumer strategy catalog to the local Rust service', () => {
    expect(apiProxy['/strategy-catalog']).toBe('http://127.0.0.1:8080')
    expect(apiProxy['/strategy-backtests']).toBe('http://127.0.0.1:8080')
  })

  it('forwards manually triggered AI endpoints to the local Rust service', () => {
    expect(apiProxy['/ai']).toBe('http://127.0.0.1:8080')
    expect(apiProxy['/personal']).toBe('http://127.0.0.1:8080')
  })
})
