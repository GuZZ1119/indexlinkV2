import type { DecisionRecord, InvestmentPlan } from '@/api/types'
import { uiLocale, uiText } from '@/i18n/ui'

/** Describe when an immutable recommendation was saved, never infer its scheduled execution day. */
export function savedRecommendationNotice(
  plan: Pick<InvestmentPlan, 'timezone'>,
  decision: Pick<DecisionRecord, 'created_at'>,
  now = new Date(),
): string {
  const saved = new Date(decision.created_at)
  const unknownDate = () => uiText("无法核验这份原建议的保存日期。请查看原建议并核对本机时间后记录结果。")
  if (!Number.isFinite(saved.getTime()) || !Number.isFinite(now.getTime()) || saved > now) return unknownDate()
  try {
    const day = new Intl.DateTimeFormat('en-CA', { timeZone: plan.timezone, year: 'numeric', month: '2-digit', day: '2-digit' })
    if (day.format(saved) === day.format(now)) {
      return uiText("这份建议在今天（{{p0}}）保存，不是实时重新计算。请核对原计划日，本次建议只能确认一次。", { p0: plan.timezone })
    }
    const savedDate = new Intl.DateTimeFormat(uiLocale(), { timeZone: plan.timezone, year: 'numeric', month: '2-digit', day: '2-digit' }).format(saved)
    return uiText("历史待办：这是 {{p0}}（{{p1}}）保存的原建议，不是今天重新计算的金额。请核对原计划日后记录结果。", { p0: savedDate, p1: plan.timezone })
  } catch {
    return unknownDate()
  }
}
