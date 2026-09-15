import { AlertTriangle, LoaderCircle, RotateCcw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'

import { useHistoricalBacktest } from '@/api/queries'
import type { HistoricalBacktest } from '@/api/types'

/** Keep the hard-coded MA200 replay behind an explicit Lab-only action. */
export function LegacyReplayPanel() {
  const { t } = useTranslation()
  const replay = useHistoricalBacktest()

  return (
    <section className="rounded-[1.35rem] border border-[#e1d7bd] bg-[#fffdf7] p-5 sm:p-6" aria-labelledby="legacy-replay-title">
      <div className="flex flex-col gap-5 lg:flex-row lg:items-start lg:justify-between">
        <div className="max-w-3xl">
          <p className="inline-flex items-center gap-2 text-sm font-medium text-[#8a641f]"><AlertTriangle className="size-4" />{t('legacyReplay.badge')}</p>
          <h2 id="legacy-replay-title" className="mt-2 text-xl font-semibold tracking-[-0.03em] text-[#102028]">{t('legacyReplay.title')}</h2>
          <p className="mt-2 text-sm leading-6 text-slate-600">{t('legacyReplay.description')}</p>
          <p className="mt-3 text-sm font-medium leading-6 text-[#6f5019]">{t('legacyReplay.warning')}</p>
        </div>
        <button
          type="button"
          onClick={() => void replay.refetch()}
          disabled={replay.isFetching}
          className="inline-flex shrink-0 items-center justify-center gap-2 rounded-full border border-[#c9b57f] bg-white px-4 py-2.5 text-sm font-medium text-[#6f5019] transition-colors hover:bg-[#faf5e8] disabled:cursor-wait disabled:text-slate-400"
        >
          {replay.isFetching ? <LoaderCircle className="size-4 animate-spin" /> : <RotateCcw className="size-4" />}
          {replay.isFetching ? t('legacyReplay.running') : t('legacyReplay.run')}
        </button>
      </div>

      {replay.error ? <p role="status" className="mt-5 rounded-xl border border-[#eadfc4] bg-white p-4 text-sm leading-6 text-slate-600">{t('legacyReplay.unavailable')}</p> : null}
      {replay.data?.points.length ? <LegacyReplayChart backtest={replay.data} /> : null}
      {replay.data && replay.data.points.length === 0 ? <p className="mt-5 text-sm text-slate-500">{t('legacyReplay.empty')}</p> : null}
    </section>
  )
}

function LegacyReplayChart({ backtest }: { backtest: HistoricalBacktest }) {
  const { t } = useTranslation()
  const data = backtest.points.map((point) => ({ date: point.date, plain: point.plain_dca_value, adaptive: point.adaptive_value }))

  return (
    <div className="mt-6 space-y-3 border-t border-[#eee5cf] pt-5">
      <div className="h-72">
        <ResponsiveContainer width="100%" height="100%" minWidth={0} minHeight={1}>
          <LineChart data={data} margin={{ top: 12, right: 16, left: 0, bottom: 0 }}>
            <CartesianGrid vertical={false} stroke="#e8e0cc" strokeDasharray="3 3" />
            <XAxis dataKey="date" tickLine={false} axisLine={false} minTickGap={32} />
            <YAxis tickLine={false} axisLine={false} width={72} />
            <Tooltip formatter={(value) => typeof value === 'number' ? `${backtest.currency} ${value.toLocaleString()}` : '—'} />
            <Line type="monotone" dataKey="plain" name={t('dashboard.performance.plain')} stroke="#64748b" strokeWidth={2} dot={false} />
            <Line type="monotone" dataKey="adaptive" name={t('dashboard.performance.adaptive')} stroke="#9a6d20" strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
      <p className="rounded-xl bg-white p-4 text-xs leading-6 text-slate-500"><span className="font-medium text-slate-700">{t('legacyReplay.methodology')}</span>{backtest.methodology}</p>
    </div>
  )
}
