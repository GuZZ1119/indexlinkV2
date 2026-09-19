import path from 'node:path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export const apiProxy = {
  '/investment-plans': 'http://127.0.0.1:8080',
  '/signals': 'http://127.0.0.1:8080',
  '/decisions': 'http://127.0.0.1:8080',
  '/market-sentiment': 'http://127.0.0.1:8080',
  '/market-data': 'http://127.0.0.1:8080',
  '/paper-portfolio': 'http://127.0.0.1:8080',
  '/paper-performance': 'http://127.0.0.1:8080',
  '/strategies': 'http://127.0.0.1:8080',
  '/strategy-catalog': 'http://127.0.0.1:8080',
  '/strategy-backtests': 'http://127.0.0.1:8080',
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
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
})
