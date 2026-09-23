import { describe, expect, it } from 'vitest'

import {
  buildPersonalStrategyDocument,
  createPersonalStrategyDraft,
  personalDraftFromDocument,
  personalPolicyId,
} from './model'

describe('personal strategy builder model', () => {
  it('maps a friendly draft to the restricted formula contract', () => {
    const draft = createPersonalStrategyDraft()
    draft.name = '下跌时增加机会投入'

    expect(buildPersonalStrategyDocument(draft, 'dsl_personal_demo')).toEqual({
      policy_id: 'dsl_personal_demo',
      policy_version: 1,
      name: '下跌时增加机会投入',
      rules: [{
        condition: {
          kind: 'comparison',
          expression: { kind: 'indicator', indicator: { kind: 'price_return', lookback_days: 63 } },
          operator: 'less_than',
          threshold: '0',
        },
        action: { kind: 'set_opportunity_multiplier', multiplier: 1.2 },
      }],
    })
  })

  it('uses an explicit skip action for zero opportunity allocation', () => {
    const draft = createPersonalStrategyDraft()
    draft.name = '等待'
    draft.rules[0].multiplier = 0

    expect(buildPersonalStrategyDocument(draft, 'dsl_personal_wait').rules[0].action).toEqual({ kind: 'skip_opportunity' })
  })

  it('converts user-facing percentages to formula decimals', () => {
    const draft = createPersonalStrategyDraft()
    draft.name = '十个百分点'
    draft.rules[0].conditions[0].threshold = '-10'

    const condition = buildPersonalStrategyDocument(draft, 'dsl_personal_percent').rules[0].condition
    expect(condition).toMatchObject({ threshold: '-0.1' })
  })

  it('rejects invalid ranges before calling the server', () => {
    const draft = createPersonalStrategyDraft()
    draft.name = '超出范围'
    draft.rules[0].conditions[0].lookbackDays = 0

    expect(() => buildPersonalStrategyDocument(draft, 'dsl_personal_invalid')).toThrow('观察窗口')

    draft.rules[0].conditions[0].lookbackDays = 366
    expect(() => buildPersonalStrategyDocument(draft, 'dsl_personal_invalid')).toThrow('2–365')
  })

  it('creates a transport-safe opaque personal policy id', () => {
    expect(personalPolicyId('37A8-9B_C')).toBe('dsl_personal_37a89bc')
  })

  it('maps a bounded AI document back into the consumer editor', () => {
    expect(personalDraftFromDocument({
      policy_id: 'dsl_personal_ai',
      policy_version: 1,
      name: 'AI 草案',
      rules: [{
        condition: {
          kind: 'comparison',
          expression: { kind: 'indicator', indicator: { kind: 'drawdown', lookback_days: 63 } },
          operator: 'less_than_or_equal',
          threshold: '-0.1',
        },
        action: { kind: 'set_opportunity_multiplier', multiplier: 1.2 },
      }],
    })).toMatchObject({
      name: 'AI 草案',
      rules: [{ conditions: [{ indicator: 'drawdown', lookbackDays: 63, threshold: '-10' }], multiplier: 1.2 }],
    })
  })

  it('rejects an AI action outside the consumer editor boundary', () => {
    expect(() => personalDraftFromDocument({
      policy_id: 'dsl_personal_ai',
      policy_version: 1,
      name: '越界草案',
      rules: [{
        condition: {
          kind: 'comparison',
          expression: { kind: 'indicator', indicator: { kind: 'close_price' } },
          operator: 'less_than',
          threshold: '10',
        },
        action: { kind: 'set_opportunity_fixed_amount', amount: '100' },
      }],
    })).toThrow('不支持的额度动作')
  })
})
