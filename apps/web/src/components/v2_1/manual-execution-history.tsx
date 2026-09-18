import { History, Loader2 } from 'lucide-react'

import type { ManualExecutionEvent, ManualExecutionOutcome } from '@/api/types'
import { cn } from '@/lib/utils'

const outcomeLabel: Record<ManualExecutionOutcome, string> = {
  executed: '已完成',
  partial: '部分完成',
  skipped: '已跳过',
}

/** Read-only timeline for user-reported facts linked to one immutable decision. */
export function ManualExecutionHistory({
  events,
  pending,
  error,
  onRetry,
  className,
}: {
  events: ManualExecutionEvent[]
  pending: boolean
  error: Error | null
  onRetry: () => void
  className?: string
}) {
  const newestFirst = [...events].reverse()
  return (
    <aside className={cn('rounded-[1.35rem] border border-slate-200 bg-white p-5', className)} aria-label="执行历史">
      <div className="flex items-center gap-2"><History className="size-4 text-[#2d6a57]" /><h2 className="text-lg font-semibold tracking-[-0.025em] text-[#102028]">执行历史</h2></div>
      {pending ? <p className="mt-5 flex items-center gap-2 text-sm text-slate-500"><Loader2 className="size-4 animate-spin" />正在读取本机记录…</p> : null}
      {!pending && error ? (
        <div className="mt-5 rounded-xl bg-red-50 p-4 text-sm leading-6 text-red-700"><p>暂时读不到执行历史。</p><button type="button" className="mt-2 font-medium underline underline-offset-4" onClick={onRetry}>重新读取</button></div>
      ) : null}
      {!pending && !error && newestFirst.length === 0 ? (
        <p className="mt-5 rounded-xl bg-[#f4f7f6] p-4 text-sm leading-6 text-slate-600">还没有执行记录。完成券商侧操作后，从个人中心选择实际结果。</p>
      ) : null}
      {!pending && !error && newestFirst.length > 0 ? (
        <ol className="mt-5 space-y-5">
          {newestFirst.map((event, index) => (
            <li key={event.id} className="relative pl-6">
              <span className="absolute left-0 top-1.5 size-2.5 rounded-full bg-[#2d6a57]" />
              {index < newestFirst.length - 1 ? <span className="absolute left-[4px] top-5 h-[calc(100%+8px)] w-px bg-slate-200" /> : null}
              <div className="flex flex-wrap items-baseline justify-between gap-2"><p className="text-sm font-medium text-[#102028]">{outcomeLabel[event.outcome]}</p>{event.actual_amount ? <p className="text-sm font-medium text-[#2d6a57]">{formatMoney(event.currency, event.actual_amount)}</p> : null}</div>
              <p className="mt-1 text-xs text-slate-400">{formatDateTime(event.occurred_at)} · 由你记录</p>
              {event.note ? <p className="mt-2 text-sm leading-6 text-slate-600">{event.note}</p> : null}
            </li>
          ))}
        </ol>
      ) : null}
    </aside>
  )
}

function formatMoney(currency: string, amount: string): string {
  const numeric = Number(amount)
  if (!Number.isFinite(numeric)) return `${currency} ${amount}`
  try {
    return new Intl.NumberFormat('zh-CN', { style: 'currency', currency, maximumFractionDigits: 2 }).format(numeric)
  } catch {
    return `${currency} ${numeric.toLocaleString('zh-CN', { maximumFractionDigits: 2 })}`
  }
}

function formatDateTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return new Intl.DateTimeFormat('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }).format(date)
}
