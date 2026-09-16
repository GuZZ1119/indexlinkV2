import { CircleCheck, CircleX, Database, Radio, TimerReset } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'

import { useHealth, useReady, useRuntimeStatus } from '@/api/queries'
import { cn } from '@/lib/utils'

/** Display safe backend capability and scheduler state without triggering external providers. */
export function RuntimeStatus() {
  const { t } = useTranslation()
  const health = useHealth()
  const ready = useReady()
  const runtime = useRuntimeStatus()

  if (health.isError) {
    return <StatusBar tone="danger" icon={<CircleX />} label={t('runtime.apiOffline')} />
  }
  if (ready.isError || runtime.data?.database === 'unavailable') {
    return <StatusBar tone="danger" icon={<Database />} label={t('runtime.databaseUnavailable')} />
  }
  if (health.isPending || ready.isPending || runtime.isPending) {
    return <StatusBar tone="muted" icon={<Radio />} label={t('runtime.checking')} />
  }

  const status = runtime.data
  if (!status) {
    return <StatusBar tone="muted" icon={<Radio />} label={t('runtime.compatibilityUnavailable')} />
  }
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-b bg-muted/25 px-4 py-1.5 text-xs text-muted-foreground">
      <StatusItem tone="ok" icon={<CircleCheck />} label={t('runtime.apiReady')} />
      <StatusItem tone={status.paper_broker === 'configured' ? 'ok' : status.paper_broker === 'unavailable' ? 'danger' : 'muted'} label={status.paper_broker === 'configured' ? t('runtime.opendConfigured') : status.paper_broker === 'unavailable' ? t('runtime.opendUnavailable') : t('runtime.opendNotConfigured')} />
      <StatusItem tone={status.qwen === 'configured' ? 'ok' : 'muted'} label={status.qwen === 'configured' ? t('runtime.qwenConfigured') : t('runtime.qwenNotConfigured')} />
      <StatusItem tone={status.market_data === 'configured' ? 'ok' : status.market_data === 'unavailable' ? 'danger' : 'muted'} label={status.market_data === 'configured' ? t('runtime.marketConfigured') : status.market_data === 'unavailable' ? t('runtime.marketUnavailable') : t('runtime.marketNotConfigured')} />
      <StatusItem
        tone={status.scheduler.enabled ? 'ok' : 'muted'}
        icon={<TimerReset />}
        label={status.scheduler.enabled
          ? t('runtime.schedulerEnabled', { seconds: status.scheduler.tick_interval_seconds })
          : t('runtime.schedulerDisabled')}
      />
      {status.scheduler.last_summary && (
        <span>{t('runtime.schedulerLatest', { created: status.scheduler.last_summary.created, catchUp: status.scheduler.last_summary.catch_up_created, unavailable: status.scheduler.last_summary.unavailable })}</span>
      )}
      {status.scheduler.last_error_at && <span className="text-destructive">{t('runtime.schedulerFailed', { time: new Date(status.scheduler.last_error_at).toLocaleString() })}</span>}
    </div>
  )
}

/** Render an individual small runtime-state indicator. */
function StatusItem({ label, tone, icon }: { label: string; tone: 'ok' | 'muted' | 'danger'; icon?: ReactNode }) {
  return <span className={cn('inline-flex items-center gap-1', tone === 'ok' ? 'text-semantic-positive' : tone === 'danger' ? 'text-destructive' : 'text-muted-foreground')}>{icon}{label}</span>
}

/** Render an unambiguous full-width availability state. */
function StatusBar({ label, tone, icon }: { label: string; tone: 'danger' | 'muted'; icon: ReactNode }) {
  return <div className={cn('flex items-center gap-1.5 border-b px-4 py-1.5 text-xs', tone === 'danger' ? 'border-destructive/30 bg-destructive/10 text-destructive' : 'bg-muted/25 text-muted-foreground')}><span className="size-3.5">{icon}</span>{label}</div>
}
