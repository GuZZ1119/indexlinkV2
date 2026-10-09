import { uiLocale, uiText } from '@/i18n/ui'
import type { InvestmentPlan } from '@/api/types'

/** Return the next plan-local calendar date without converting it back through the browser zone. */
export function nextScheduledDate(
  plan: InvestmentPlan,
  now = new Date(),
  options: { strictlyAfterToday?: boolean } = {},
): string {
  const today = datePartsInTimeZone(now, plan.timezone)
  const todayOrdinal = Date.UTC(today.year, today.month - 1, today.day)
  const candidates: Array<{ year: number; month: number; day: number }> = []
  if (plan.schedule_kind === 'weekly') {
    const todayWeekday = isoWeekday(today.year, today.month, today.day)
    for (const weekday of plan.schedule_days) {
      let daysUntil = (weekday - todayWeekday + 7) % 7
      if (options.strictlyAfterToday && daysUntil === 0) daysUntil = 7
      const date = new Date(todayOrdinal + daysUntil * 86_400_000)
      candidates.push({ year: date.getUTCFullYear(), month: date.getUTCMonth() + 1, day: date.getUTCDate() })
    }
  } else {
    for (const monthOffset of [0, 1]) {
      const monthAnchor = new Date(Date.UTC(today.year, today.month - 1 + monthOffset, 1))
      for (const day of plan.schedule_days) {
        const candidateDate = new Date(Date.UTC(monthAnchor.getUTCFullYear(), monthAnchor.getUTCMonth(), day))
        if (candidateDate.getUTCMonth() !== monthAnchor.getUTCMonth() || candidateDate.getUTCDate() !== day) continue
        const isFutureCandidate = options.strictlyAfterToday
          ? candidateDate.getTime() > todayOrdinal
          : candidateDate.getTime() >= todayOrdinal
        if (isFutureCandidate) candidates.push({ year: monthAnchor.getUTCFullYear(), month: monthAnchor.getUTCMonth() + 1, day })
      }
    }
  }
  const next = candidates.sort((left, right) => Date.UTC(left.year, left.month - 1, left.day) - Date.UTC(right.year, right.month - 1, right.day))[0]
  if (!next) return uiText("下一次约定日期")
  if (uiLocale() !== 'zh-CN') {
    const date = new Date(Date.UTC(next.year, next.month - 1, next.day))
    return `${new Intl.DateTimeFormat(uiLocale(), { timeZone: 'UTC', month: 'short', day: 'numeric' }).format(date)} (${plan.timezone})`
  }
  return uiText("{{p0}}月{{p1}}日（{{p2}}）", { p0: next.month, p1: next.day, p2: plan.timezone })
}

function datePartsInTimeZone(value: Date, timezone: string): { year: number; month: number; day: number } {
  const parts = new Intl.DateTimeFormat('en-CA', {
    timeZone: timezone,
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
  }).formatToParts(value)
  const read = (type: Intl.DateTimeFormatPartTypes) => Number(parts.find((part) => part.type === type)?.value)
  return { year: read('year'), month: read('month'), day: read('day') }
}

function isoWeekday(year: number, month: number, day: number): number {
  const weekday = new Date(Date.UTC(year, month - 1, day)).getUTCDay()
  return weekday === 0 ? 7 : weekday
}
