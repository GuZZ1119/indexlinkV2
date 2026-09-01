# Strategy Research Correction V2 / 策略研究口径修正 V2

> Status / 状态：研究评估完成；**没有**修改生产默认权重、`decision-engine`、API 或下单行为。
> Machine-readable result / 机器可读结果：[calibration-v2.report.json](crates/strategy-evaluation/data/generated/calibration-v2.report.json)

## Scope / 范围

V2 preserves the immutable V1 fixture and report. It creates a separately
versioned dataset and makes two predeclared research corrections:

V2 保留不可变的 V1 夹具和报告，新增独立版本的数据集，并预先登记两项研究修正：

1. A decision is calculated after the final available monthly observation at
   `t`; both strategies execute only at the first committed daily observation
   **strictly later than `t`**. No decision may buy at its own closing price.
2. The research-only trend candidate keeps the core bucket fixed and applies a
   continuous cap only to the opportunity multiplier. It never creates a
   whole-order `TacticalDelay` veto.

1. 在 `t` 日当月最后一个可得观察值收盘后计算；两条对照线只能在**严格晚于
   `t`** 的第一个已提交日度观察值成交，决策日收盘价不可同时作为成交价。
2. 研究专用趋势候选永久保留核心桶，仅对机会桶倍率施加连续上限，不再产生整笔
   `TacticalDelay` 否决。

The execution price remains a next-trading-day **close proxy**, not an
intraday/open fill model. Dividends, taxes, ETF spreads, and market-impact
variation remain out of scope.

成交价仍是下一交易日**收盘价代理**，并非盘中或开盘成交模型；股息、税、ETF
点差和随规模变化的冲击成本仍不在本轮范围内。

## Data integrity and timing / 数据完整性与时点

| Item / 项目 | Value / 数值 |
| --- | --- |
| Dataset / 数据集 | `calibration-v2`（parent / 父版本：`calibration-v1`） |
| Assets / 标的 | FRED S&P 500（SPY proxy）与 NASDAQ Composite（QQQ proxy） |
| Decision observation / 决策观察 | 每月最后一个可得交易观察值 |
| Execution observation / 成交观察 | 严格晚于决策日的第一个 FRED 日度价格 |
| Factors / 因子 | CAPE、ERP proxy、MA200 distance、RSI-14、VIX |
| Integrity / 完整性 | 原始快照与 SHA-256 位于 `calibration-v2.manifest.json` |

The fixture generator drops a month if a factor or a strictly later execution
price is missing; it never forward-fills or interpolates. A focused test also
asserts that every evaluated execution date is later than its decision date.

若任一因子或严格后续的成交价缺失，生成器会丢弃该月，不前填、不插值；聚焦测试还会
验证每条评估记录的成交日均晚于其决策日。

## Continuous trend-cap candidate / 连续趋势上限候选

```text
core contribution              = 70% of period budget (fixed)
opportunity base multiplier    = production-shaped multiplier(fundamental directional score)
tail risk                      = max(MA200-distance percentile, RSI percentile, VIX percentile)
trend cap                      = 1.00 until tail risk > 0.50, then linearly declines to 0.25 at 1.00
opportunity multiplier         = base multiplier × trend cap
whole-order TacticalDelay veto = never emitted by this candidate
```

The cap is deliberately bounded in `[0.25, 1.00]`. The regression suite proves
the cap reaches both bounds and, across every fixture observation, the core
contribution remains USD 700 for the fixed USD 1,000 budget.

该上限严格限制在 `[0.25, 1.00]`。回归测试验证其两个边界，并对所有夹具观察值验证：
在固定 USD 1,000 周期预算下，核心桶始终为 USD 700。

## Results / 结果

All lines use identical USD 1,000 monthly cash flows, 5 bps buy cost, no
cash interest, and include retained cash in terminal wealth. Historical AI is
still the reproducible 90/10/0 fallback; frozen Qwen data is not used for
return claims.

所有对照线使用相同的每月 USD 1,000 现金流、5 bps 买入成本、零现金利息，并将
留存现金计入期末净值。历史 AI 仍使用可复现的 90/10/0 降级线；冻结 Qwen 数据不参与
收益结论。

| Proxy / 代理 | Strategy / 策略 | XIRR | Terminal wealth / 期末净值 | Difference vs DCA / 相对 DCA | Max drawdown / 最大回撤 | Volatility / 波动率 | Cash utilisation / 现金使用率 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| S&P 500 (SPY proxy) | Fixed DCA | 19.43% | $71,454 | — | 7.83% | 12.68% | 100.00% |
| S&P 500 (SPY proxy) | Current Core/Opportunity | 17.37% | $68,727 | -3.82% | 7.63% | 11.68% | 82.65% |
| S&P 500 (SPY proxy) | C1 bounded continuous | 18.98% | $70,847 | -0.85% | 7.78% | 12.46% | 96.00% |
| S&P 500 (SPY proxy) | C2 trend continuous cap | 16.46% | $67,555 | -5.46% | 6.63% | 10.84% | 78.52% |
| NASDAQ Composite (QQQ proxy) | Fixed DCA | 16.80% | $809,186 | — | 34.32% | 17.99% | 100.00% |
| NASDAQ Composite (QQQ proxy) | Current Core/Opportunity | 15.75% | $735,283 | -9.13% | 32.53% | 16.78% | 83.31% |
| NASDAQ Composite (QQQ proxy) | C1 bounded continuous | 16.64% | $797,460 | -1.45% | 34.05% | 17.81% | 96.96% |
| NASDAQ Composite (QQQ proxy) | C2 trend continuous cap | 15.41% | $712,292 | -11.97% | 32.13% | 16.37% | 80.39% |

C2 reduces drawdown and volatility only by retaining more cash; it does not
produce a matched-DCA return advantage. It loses in all 3/3 S&P rolling
24-month windows and in 12/14 NASDAQ windows (2/14 higher terminal wealth).
It must **not** become a production default. This is a useful negative result:
moving trend from a veto into a continuous cap fixes the execution semantics,
but the stated cap is still too conservative for this dataset.

C2 的较低回撤与波动主要来自更高现金留存，并未形成相对匹配 DCA 的收益优势。它在
S&P 的 3/3 个滚动 24 个月窗口均落后，在 NASDAQ 的 14 个窗口中有 12 个落后
（仅 2/14 的期末净值更高）。因此**不得**升级为生产默认。这是一项有价值的负结果：
把趋势从 veto 改为连续上限修正了执行语义，但该预登记上限对本数据集仍过于保守。

## Reproduce / 复跑

```bash
python3 tools/generate_calibration_fixture.py
cargo test -p strategy-evaluation --locked
cargo run -p strategy-evaluation --locked -- \
  crates/strategy-evaluation/data/generated/calibration-v2.report.json
```

## Next research decision / 下一项研究决策

Keep V2 and C2 as an immutable negative-control result. Any later candidate
must be predeclared before inspecting its hold-out windows, keep the same
timing and cost assumptions, and be evaluated separately from production.

保留 V2 与 C2 作为不可变的负对照结果。后续任何候选必须在查看留出窗口前预登记，
保持相同时点和成本假设，并始终与生产策略分开评估。
