# Strategy Calibration Candidates V1 / 策略校准候选 V1

> Status / 状态：实验评估完成；没有候选被设为生产默认，也没有改动任何 API、scheduler 或下单行为。

## Candidate C1 — bounded continuous opportunity / 有界连续机会桶

**Predeclared rule / 预先声明的规则**

```text
core bucket             = 70% of the monthly budget
opportunity multiplier  = 0.75 + 0.50 × current_final_score
opportunity range       = [0.75, 1.25]
global TacticalDelay    = not an order veto in this evaluation-only candidate
costs/cash/data         = identical to calibration-v1 fixed DCA comparison
```

The candidate was chosen from the baseline finding, not from a selected return
window: the historical fallback score is low-centred and the current API gate
can block a domain-preserved core amount. C1 tests a deliberately modest
alternative: preserve the core bucket and make only the opportunity amount
continuous, with a floor that reduces cash drag. It is **not** a signal that
the project can predict price or that this rule should ship.

该候选来自基线发现，而非从某个收益窗口倒推：历史降级分数中心偏低，且当前 API
gate 可能挡住领域层保留的核心金额。C1 只测试一个克制的替代方案：保留核心桶，
仅将机会金额改为连续、带下限的倍率以减少现金拖累。它**不**代表项目可以预测价格，
也不代表该规则应直接上线。

## Matched results / 匹配口径结果

| Proxy / 代理 | C1 XIRR | C1 terminal wealth / 期末净值 | Difference vs DCA / 相对固定定投 | C1 max drawdown / 最大回撤 | C1 volatility / 波动率 | Cash utilisation / 现金使用率 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| S&P 500 (SPY proxy) | 19.16% | $71,059 | -0.85% | 9.64% | 13.08% | 96.00% |
| NASDAQ Composite (QQQ proxy) | 16.72% | $803,443 | -1.46% | 32.76% | 16.67% | 96.96% |

The result is deliberately not promoted as success: C1 remains behind matched
fixed DCA in both whole-sample comparisons. It reduces the baseline strategy's
cash drag but does not demonstrate a robust excess-return advantage.

结果不被包装为成功：C1 在两个全样本对照中仍落后于匹配的固定定投。它减少了基线
策略的现金拖累，但没有证明稳定的超额收益优势。

## Rolling out-of-sample / 滚动样本外

Using the same 24-month windows advancing by 12 months:

| Proxy / 代理 | Windows / 窗口数 | C1 terminal wealth beats DCA / C1 期末净值跑赢固定定投 |
| --- | ---: | ---: |
| S&P 500 proxy | 3 | 0 / 3 |
| NASDAQ proxy | 14 | 2 / 14 |

Individual window results are in the versioned
`crates/strategy-evaluation/data/generated/calibration-v1.report.json` file.
No window has been removed or used to select a different multiplier.

每个窗口的结果位于版本化的 `calibration-v1.report.json`。没有删除窗口，也没有
根据窗口结果再选择不同倍率。

## Decision / 结论

Do **not** promote C1 into production. It is useful as a control showing that
the low baseline score and global veto explain much of the cash drag, but the
same-data out-of-sample result remains insufficient. The next legitimate step
is to write down no more than one or two additional, economically motivated
candidates before looking at their hold-out results—for example, calibrating
the fundamental percentile centre or changing the trend layer from a veto to a
bounded opportunity cap—then repeat the identical protocol.

**不要**将 C1 提升为生产默认。它作为对照说明了低基线分数和全局 veto 是现金拖累的
重要来源，但同数据的样本外结果仍不足。下一步应在看留出集结果前最多再预先写下
一到两个具备经济动机的候选，例如校准基本面分位中心，或把趋势层从 veto 改为有界的
机会桶上限，然后重复完全相同的评估协议。
