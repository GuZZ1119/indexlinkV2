import { afterEach, describe, expect, it } from 'vitest'
import i18n from '@/i18n'
import { savedRecommendationNotice } from './recommendation-notice'

afterEach(async () => { await i18n.changeLanguage('zh') })

describe('immutable recommendation date notice', () => {
  it('labels an old recommendation as a historical pending item without changing it', () => {
    const decision = Object.freeze({ created_at: '2026-09-19T12:19:33Z' })
    const notice = savedRecommendationNotice({ timezone: 'UTC' }, decision, new Date('2026-10-10T02:00:00Z'))
    expect(notice).toContain('历史待办')
    expect(notice).toContain('2026/09/19（UTC）')
    expect(notice).toContain('不是今天重新计算的金额')
    expect(decision.created_at).toBe('2026-09-19T12:19:33Z')
  })

  it('uses the plan calendar, not the UTC or browser day', () => {
    const saved = { created_at: '2026-09-30T13:30:00Z' }
    const now = new Date('2026-09-30T15:30:00Z')
    expect(savedRecommendationNotice({ timezone: 'UTC' }, saved, now)).toContain('今天（UTC）保存')
    expect(savedRecommendationNotice({ timezone: 'Australia/Sydney' }, saved, now)).toContain('历史待办')
    expect(savedRecommendationNotice({ timezone: 'Asia/Hong_Kong' }, saved, now)).toContain('今天（Asia/Hong_Kong）保存')
  })

  it('recognises the same plan day across different UTC dates without asserting execution is due today', () => {
    const notice = savedRecommendationNotice({ timezone: 'Australia/Sydney' }, { created_at: '2026-10-09T23:30:00Z' }, new Date('2026-10-10T01:30:00Z'))
    expect(notice).toContain('今天（Australia/Sydney）保存')
    expect(notice).toContain('请核对原计划日')
    expect(notice).not.toContain('当前是这份计划的执行日')
  })

  it.each([
    ['bad timestamp', 'UTC', new Date('2026-10-10T02:00:00Z')],
    ['2026-10-11T00:00:00Z', 'UTC', new Date('2026-10-10T02:00:00Z')],
    ['2026-10-09T00:00:00Z', 'invalid/timezone', new Date('2026-10-10T02:00:00Z')],
    ['2026-10-09T00:00:00Z', 'UTC', new Date('invalid')],
  ])('fails safe for unverifiable date/timezone %s / %s', (created_at, timezone, now) => {
    expect(savedRecommendationNotice({ timezone }, { created_at }, now)).toContain('无法核验')
  })

  it('translates the notice and its saved date without changing the plan timezone', async () => {
    await i18n.changeLanguage('en')
    const now = new Date('2026-10-10T02:00:00Z')
    expect(savedRecommendationNotice({ timezone: 'Australia/Sydney' }, { created_at: '2026-09-19T12:19:33Z' }, now)).toContain('saved on 09/19/2026 (Australia/Sydney)')
    expect(savedRecommendationNotice({ timezone: 'UTC' }, { created_at: now.toISOString() }, now)).toContain('saved today (UTC)')
    expect(savedRecommendationNotice({ timezone: 'UTC' }, { created_at: 'invalid' }, now)).toContain('could not be verified')
  })
})
