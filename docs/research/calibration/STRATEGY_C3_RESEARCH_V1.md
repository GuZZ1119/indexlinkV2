# C3 Fundamental / Trend Decoupling Research V1
# C3 基本面 / 趋势解耦研究 V1

> Status / 状态：预登记研究已完成；C3 **未**升级为生产默认，未修改 `decision-engine`、API、scheduler 或下单行为。
> Machine-readable result / 机器可读结果：[calibration-v2-c3-research.report.json](crates/strategy-evaluation/data/generated/calibration-v2-c3-research.report.json)

## Pre-registered rule / 预登记规则

The frozen input remains `calibration-v2`: monthly decision after the final
available observation, execution at the first strictly later daily close, the
same USD 1,000 monthly cash flow, 5 bps buy cost, and no cash interest. C3 was
specified before the evaluation result was read and uses no historical Qwen
score in its performance line.

输入仍为冻结的 `calibration-v2`：月末可得观察后决策、严格后一交易日收盘价成交、
相同的每月 USD 1,000 现金流、5 bps 买入成本和零现金利息。C3 在查看结果前已确定，
历史收益线不使用 Qwen 分数。

```text
core bucket = user-selected share of the period budget; always executed

fundamental opportunity multiplier m_f
  = production-shaped multiplier(directional fundamental score), bounded [0.75, 1.25]

tail risk r
  = max(MA200-distance percentile, RSI percentile, VIX percentile)

trend addition cap m_max
  = 1.25 when r <= 0.75; declines linearly to 1.00 when r = 1.00

final opportunity multiplier
  = min(m_f, m_max)
```

```text
核心桶 = 用户选择的周期预算比例；始终执行

基本面机会桶倍率 m_f
  = 方向化基本面分数的生产形状倍率，并限制在 [0.75, 1.25]

尾部风险 r
  = max(MA200 距离分位、RSI 分位、VIX 分位)

趋势加码上限 m_max
  = r <= 0.75 时为 1.25；r = 1.00 时线性下降至 1.00

最终机会桶倍率
  = min(m_f, m_max)
```

Therefore trend can remove an **overweight**, but cannot reduce a normal
opportunity contribution below `1.00`; only fundamental valuation can reduce
it to the bounded `0.75` floor. C3 never emits `TacticalDelay`. Regression
tests lock both the core contribution and this boundary.

因此趋势只能取消**加码**，不会把正常机会桶投入压到 `1.00` 以下；只有基本面估值可
将其降低到受限的 `0.75` 下限。C3 不产生 `TacticalDelay`。回归测试锁定核心桶及此边界。

## Acceptance protocol / 验收口径

The primary configuration is 70% core / 30% opportunity. Fixed DCA, current
Core/Opportunity, C1, C2, and C3 receive identical cash flows and execution
prices. Reported metrics are XIRR, terminal wealth, maximum drawdown,
annualized volatility, Sortino ratio, drawdown recovery months, cash
utilisation, and every 24-month rolling window. A lower drawdown alone is not
success if it is explained by materially lower cash utilisation.

主配置为 70% 核心 / 30% 机会。固定 DCA、当前核心/机会、C1、C2、C3 使用完全相同
的现金流和成交价。报告 XIRR、期末净值、最大回撤、年化波动、Sortino、回撤恢复月数、
现金使用率及全部 24 个月滚动窗口。若较低回撤只是显著降低现金使用率造成，则不算成功。

## Primary 70/30 result / 主配置 70/30 结果

| Proxy / 代理 | Strategy / 策略 | XIRR | Difference vs DCA / 相对 DCA | Max drawdown / 最大回撤 | Volatility / 波动率 | Sortino | Recovery / 恢复月数 | Cash utilisation / 现金使用率 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| S&P 500 (SPY proxy) | Fixed DCA | 19.43% | — | 7.83% | 12.68% | 2.524 | 2 | 100.00% |
| S&P 500 (SPY proxy) | Current | 17.37% | -3.82% | 7.63% | 11.68% | 2.461 | 2 | 82.65% |
| S&P 500 (SPY proxy) | C1 | 18.98% | -0.85% | 7.78% | 12.46% | 2.511 | 2 | 96.00% |
| S&P 500 (SPY proxy) | C2 | 16.46% | -5.46% | 6.63% | 10.84% | 2.527 | 2 | 78.52% |
| S&P 500 (SPY proxy) | C3 | 18.60% | -1.54% | 7.70% | 12.25% | 2.504 | 2 | 93.87% |
| NASDAQ Composite (QQQ proxy) | Fixed DCA | 16.80% | — | 34.32% | 17.99% | 1.517 | 14 | 100.00% |
| NASDAQ Composite (QQQ proxy) | Current | 15.75% | -9.13% | 32.53% | 16.78% | 1.516 | 14 | 83.31% |
| NASDAQ Composite (QQQ proxy) | C1 | 16.64% | -1.45% | 34.05% | 17.81% | 1.519 | 14 | 96.96% |
| NASDAQ Composite (QQQ proxy) | C2 | 15.41% | -11.97% | 32.13% | 16.37% | 1.514 | 14 | 80.39% |
| NASDAQ Composite (QQQ proxy) | C3 | 16.49% | -2.78% | 33.82% | 17.65% | 1.520 | 14 | 94.71% |

C3 is a structural improvement over the current strategy: it raises cash
utilisation from 82.65% to 93.87% for the S&P proxy and from 83.31% to 94.71%
for NASDAQ, substantially shrinking the terminal-wealth gap. It is still worse
than C1 and fixed DCA in both full samples. It wins 0/3 S&P and 2/14 NASDAQ
rolling windows; its worst window is -1.38% and -1.73% versus DCA respectively.

C3 相对当前策略是结构性改进：S&P 代理的现金使用率由 82.65% 提高到 93.87%，
NASDAQ 由 83.31% 提高到 94.71%，显著缩小终值差距；但它仍落后于 C1 和固定 DCA。
滚动窗口中，C3 在 S&P 为 0/3、NASDAQ 为 2/14 胜过 DCA；最差窗口分别为 -1.38%
和 -1.73%。

## User allocation sensitivity / 用户比例敏感性

The table shows C3 only. At 100/0 every strategy equals fixed DCA, which is a
useful invariant rather than a strategy win.

下表仅列 C3。100/0 时所有策略都等于固定 DCA，这是应成立的不变量，而非策略胜利。

| Proxy / 代理 | Core / Opportunity | C3 XIRR | C3 terminal difference vs DCA | Cash utilisation / 现金使用率 |
| --- | --- | ---: | ---: | ---: |
| S&P 500 proxy | 100 / 0 | 19.43% | 0.00% | 100.00% |
| S&P 500 proxy | 80 / 20 | 18.88% | -1.03% | 95.91% |
| S&P 500 proxy | 70 / 30 | 18.60% | -1.54% | 93.87% |
| S&P 500 proxy | 50 / 50 | 18.05% | -2.57% | 89.78% |
| NASDAQ proxy | 100 / 0 | 16.80% | 0.00% | 100.00% |
| NASDAQ proxy | 80 / 20 | 16.59% | -1.86% | 96.48% |
| NASDAQ proxy | 70 / 30 | 16.49% | -2.78% | 94.71% |
| NASDAQ proxy | 50 / 50 | 16.28% | -4.64% | 91.19% |

Higher core allocation naturally moves C3 closer to fixed DCA. This is not a
license to select 80/20 from these results: the ratio remains a user risk
choice, and future production selection needs a separate predeclared hold-out
protocol.

较高的核心比例自然使 C3 更接近固定 DCA。这不能据此选择 80/20：比例仍是用户的风险
选择，未来生产策略选择必须使用独立、预登记的留出集口径。

## Decision / 结论

Do not promote C3. The study supports the design principle that trend should
cap only opportunity overweight, but it does not yet demonstrate a return or
risk-adjusted advantage over fixed DCA or C1. The next legitimate experiment
is not to optimise weights on this dataset. It should separately evaluate a
small number of predeclared fundamental calibration mappings on an untouched
hold-out, while AI remains explanatory-only.

不要升级 C3。本研究支持“趋势只限制机会桶加码”的设计原则，但尚未证明其相对固定 DCA
或 C1 具有收益或风险调整后优势。下一项实验不应在本数据集上优化权重，而应在未见留出集
上评估少数预登记的基本面校准映射；AI 继续仅承担解释职责。

## Reproduce / 复跑

```bash
cargo test -p strategy-evaluation --locked
cargo run -p strategy-evaluation --locked -- \
  crates/strategy-evaluation/data/generated/calibration-v2-c3-research.report.json
```
