import path from 'node:path'
import { configDefaults, defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

/** Keep browser-like frontend tests independent from production Vite configuration. */
export default defineConfig({
  plugins: [react()],
  resolve: { alias: { '@': path.resolve(import.meta.dirname, './src') } },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/i18n/test-setup.ts'],
    exclude: [...configDefaults.exclude, 'src/pages/strategies/**'],
    coverage: {
      provider: 'v8',
      include: [
        'src/i18n/index.ts',
        'src/i18n/ui.ts',
        'src/i18n/catalog.ts',
        'src/i18n/ui-messages.ts',
        'src/i18n/catalog-messages.ts',
        'src/i18n/locales/en.ts',
        'src/i18n/locales/zh.ts',
        'src/pages/decisions/filters.ts',
        'src/features/v2_1/model.ts',
        'src/components/v2_1/page-heading.tsx',
        'src/components/v2_1/strategy-card.tsx',
        'src/components/v2_1/strategy-center-nav.tsx',
        'src/components/v2_1/manual-execution-history.tsx',
        'src/pages/personal/index.tsx',
        'src/pages/personal/recommendation-notice.ts',
        'src/pages/plans/index.tsx',
        'src/pages/strategy-center/index.tsx',
        'src/pages/strategy-analysis/index.tsx',
        'src/pages/strategy-analysis/chart-options.ts',
        'src/pages/strategy-analysis/market-execution-model.ts',
        'src/pages/lab/index.tsx',
      ],
      thresholds: { lines: 90, functions: 90, statements: 90, branches: 90 },
    },
  },
})
