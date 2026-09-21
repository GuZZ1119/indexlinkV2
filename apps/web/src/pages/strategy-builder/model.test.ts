import { describe, expect, it } from 'vitest'

import {
  buildPersonalStrategyDocument,
  createPersonalStrategyDraft,
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
  })

  it('creates a transport-safe opaque personal policy id', () => {
    expect(personalPolicyId('37A8-9B_C')).toBe('dsl_personal_37a89bc')
  })
})
