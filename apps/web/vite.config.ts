import path from 'node:path'
import type { IncomingMessage } from 'node:http'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

/** Only HTML navigation to a known page bypasses its overlapping JSON API proxy. */
function pageDocumentBypass(request: IncomingMessage): string | undefined {
  const pathname = request.url?.split('?')[0] ?? ''
  const isPage = /^\/(?:personal|decisions(?:\/[^/]+)?)\/?$/.test(pathname)
  const acceptsHtml = request.headers.accept?.split(',').some((type) => type.trim().startsWith('text/html'))
  if (request.method === 'GET' && isPage && acceptsHtml) return '/index.html'
}

export const apiProxy = {
  '/investment-plans': 'http://127.0.0.1:8080',
  '/signals': 'http://127.0.0.1:8080',
  '/decisions': { target: 'http://127.0.0.1:8080', bypass: pageDocumentBypass },
  '/market-sentiment': 'http://127.0.0.1:8080',
  '/market-data': 'http://127.0.0.1:8080',
  '/paper-portfolio': 'http://127.0.0.1:8080',
  '/paper-performance': 'http://127.0.0.1:8080',
  '/strategies': 'http://127.0.0.1:8080',
  '/strategy-catalog': 'http://127.0.0.1:8080',
  '/strategy-backtests': 'http://127.0.0.1:8080',
  '/ai': 'http://127.0.0.1:8080',
  '/personal': { target: 'http://127.0.0.1:8080', bypass: pageDocumentBypass },
  '/health': 'http://127.0.0.1:8080',
  '/ready': 'http://127.0.0.1:8080',
  '/runtime-status': 'http://127.0.0.1:8080',
} as const

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: apiProxy,
  },
  preview: {
    proxy: apiProxy,
  },
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, './src'),
    },
  },
})
