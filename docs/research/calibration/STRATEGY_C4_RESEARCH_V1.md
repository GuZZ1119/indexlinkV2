# C4 Bounded Budget With Deadline Research V1
# C4 有期限机会预算调度研究 V1

> Status / 状态：预登记离线研究已完成。C4 **未**成为生产默认策略，未修改
> `decision-engine`、API、scheduler 或 paper order 行为。
>
> Machine-readable result / 机器可读结果：
> [calibration-v2-c4-research.report.json](crates/strategy-evaluation/data/generated/calibration-v2-c4-research.report.json)

## Scope and fairness / 范围与公平口径

All lines use frozen `calibration-v2`, the same monthly USD 1,000 external cash
flow, the same first strictly later daily close for execution, 5 bps purchase
cost, and zero interest on uninvested cash. Historical performance uses the
causal 90/10/0 AI-unavailable fallback; frozen Qwen values are not used to
claim historical alpha.

所有策略使用冻结的 `calibration-v2`、相同的每月 USD 1,000 外部入金、严格晚于
决策日的首个日度收盘价、5 bps 买入成本，以及零现金利息。历史收益采用因果的
90/10/0 AI 不可用降级；冻结的 Qwen 值不用于宣称历史超额收益。

The primary allocation is 70% core / 30% opportunity. The fixed core always
executes in every non-DCA line. This is a research comparison, not investment
advice or a return guarantee.

主配置为 70% 核心 / 30% 机会。所有非 DCA 策略的核心桶均固定执行。本报告是研究
比较，不构成投资建议或收益承诺。

## Strategy differences / 策略差异

| ID | Strategy / 策略 | What changes / 变化点 | Cash policy / 现金口径 |
| --- | --- | --- | --- |
| DCA | Fixed DCA / 固定定投 | 每期完整投入预算；不读取基本面、趋势或 AI。 | 无机会现金。 |
| Current | Current Core/Opportunity / 当前双桶 | 使用当前综合分数和动作影响机会桶；核心桶保留。 | 未投入机会金额持续滚存。 |
| C1 | Bounded continuous / 连续有界 | 机会倍率 = `0.75 + 0.50 × final_score`，范围 `[0.75, 1.25]`；不使用趋势全局 veto。 | 可持续滚存。 |
| C2 | Trend continuous cap / 趋势连续压低 | 基本面倍率再乘趋势风险上限 `[0.25, 1.00]`。 | 可持续滚存；作为“现金拖累”负对照。 |
| C3 | Trend caps additions / 趋势只限加码 | 基本面倍率 `[0.75, 1.25]`；趋势仅将加码压到最高 `1.00`。 | 可持续滚存。 |
| C4 | Bounded budget with deadline / 有期限预算调度 | 核心固定；过去 12 个已完成基本面方向分数的历史排名映射机会倍率 `[0.85, 1.15]`；趋势仅限制 `1.00` 以上加码。 | 少投金额拆为带日期 lot；最多滚存 3 个周期，到期强制追赶；不能预支未来现金；AI 仅解释。 |

## C4 formula / C4 公式

```text
core_t = period_budget × core_ratio                # always execute
opportunity_budget_t = period_budget × opportunity_ratio

fundamental_rank_t
  = rank(current directional fundamental score among only prior 12 decisions)
m_f = 0.85 + 0.30 × fundamental_rank_t             # [0.85, 1.15]

tail_risk_t = max(MA200-distance percentile, RSI percentile, VIX percentile)
m_trend_cap = linearly declines from 1.15 to 1.00 when tail_risk goes 0.75 → 1.00

m_opportunity = min(m_f, m_trend_cap)
```

When `m_opportunity < 1.00`, the difference enters a dated deferred-cash lot.
When it is above `1.00`, the candidate can release existing non-expired lots,
but cannot borrow future contributions. Each lot is forcibly invested after at
most three scheduled periods. Therefore trend cannot stop the core or reduce a
normal opportunity contribution, and deferred cash cannot accumulate forever.

当 `m_opportunity < 1.00` 时，差额进入带到期日的延后现金 lot。高于 `1.00` 时，
只能释放已有、未到期的 lot，不能预支未来入金。每个 lot 最多三个调度周期后强制投入。
因此趋势不能阻断核心桶或压低正常机会投入，延期现金也不能无限堆积。

## Full-sample comparison / 全样本对比

### S&P 500 index (SPY proxy), 2022-06 to 2026-06

| Strategy / 策略 | XIRR | Terminal wealth / 期末净值 | vs DCA | Max drawdown / 最大回撤 | Volatility / 波动率 | Sortino | Recovery / 恢复月数 | Cash use / 现金使用率 | Terminal cash / 期末现金 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| DCA | 19.43% | $71,453.78 | — | 7.83% | 12.68% | 2.524 | 2 | 100.00% | $0.00 |
| Current | 17.37% | $68,726.66 | -3.82% | 7.63% | 11.68% | 2.461 | 2 | 82.65% | $8,501.91 |
| C1 | 18.98% | $70,847.28 | -0.85% | 7.78% | 12.46% | 2.511 | 2 | 96.00% | $1,961.79 |
| C2 | 16.46% | $67,555.21 | -5.46% | 6.63% | 10.84% | 2.527 | 2 | 78.52% | $10,526.97 |
| C3 | 18.60% | $70,351.88 | -1.54% | 7.70% | 12.25% | 2.504 | 2 | 93.87% | $3,005.86 |
| **C4** | **19.41%** | **$71,426.45** | **-0.04%** | **7.83%** | **12.66%** | **2.526** | **2** | **99.83%** | **$82.50** |

### NASDAQ Composite (QQQ proxy), 2010-10 to 2026-06

| Strategy / 策略 | XIRR | Terminal wealth / 期末净值 | vs DCA | Max drawdown / 最大回撤 | Volatility / 波动率 | Sortino | Recovery / 恢复月数 | Cash use / 现金使用率 | Terminal cash / 期末现金 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| DCA | 16.80% | $809,185.64 | — | 34.32% | 17.99% | 1.517 | 14 | 100.00% | $0.00 |
| Current | 15.75% | $735,283.11 | -9.13% | 32.53% | 16.78% | 1.516 | 14 | 83.31% | $31,537.30 |
| C1 | 16.64% | $797,459.71 | -1.45% | 34.05% | 17.81% | 1.519 | 14 | 96.96% | $5,749.10 |
| C2 | 15.41% | $712,292.49 | -11.97% | 32.13% | 16.37% | 1.514 | 14 | 80.39% | $37,071.28 |
| C3 | 16.49% | $786,667.12 | -2.78% | 33.82% | 17.65% | 1.520 | 14 | 94.71% | $9,990.72 |
| **C4** | **16.79%** | **$808,774.46** | **-0.05%** | **34.32%** | **17.99%** | **1.517** | **14** | **99.96%** | **$82.50** |

## C4 cash diagnostics / C4 现金诊断

| Proxy / 代理 | Forced catch-up / 到期追赶 | Extra release / 加码释放 | Matured lots / 到期 lot 数 | Max deferred cash / 最大延后现金 |
| --- | ---: | ---: | ---: | ---: |
| SPY proxy | $787.50 | $112.50 | 23 | $135.00 |
| QQQ proxy | $2,608.14 | $369.36 | 86 | $135.00 |

## Rolling out-of-sample result / 滚动样本外结果

| Proxy / 代理 | C4 windows >= DCA / 不差于 DCA 窗口 | Worst terminal difference / 最差终值差 |
| --- | ---: | ---: |
| SPY proxy | 0 / 3 | -0.036% |
| QQQ proxy | 6 / 14 | -0.083% |

## Decision / 结论

C4 succeeds as an **execution-boundary correction**, not as an alpha strategy:
it removes the persistent cash drag of Current/C2 and preserves the core bucket,
but its results are almost indistinguishable from DCA. This is expected when
the three-period deadline prevents cash from staying out of a rising market.

C4 作为**执行边界修正**是成功的：它消除了 Current/C2 的长期现金拖累，并保留核心桶；
但它与 DCA 的结果几乎没有差异。三期到期追赶避免现金长期错过上涨，因此出现这个结果是
合理的，而不是超额收益证据。

Do not promote C4. It fails the predeclared purpose of proving a material
return or risk-adjusted improvement: SPY has no C4 rolling-window win, QQQ
only wins 6 of 14 windows, and drawdown/recovery improvements are immaterial.
The next research task is factor predictive-validity testing, not another
parameter search on these same results.

不要升级 C4。它没有证明显著收益或风险调整后优势：SPY 样本外无胜出窗口，QQQ 仅在
14 个窗口中的 6 个胜出，回撤/恢复期改善也不具经济意义。下一项研究应是因子预测有效性
检验，而不是继续根据本结果搜索参数。

## Reproduce / 复跑

```bash
cargo test -p strategy-evaluation --locked
cargo run -p strategy-evaluation --locked -- \
  crates/strategy-evaluation/data/generated/calibration-v2-c4-research.report.json
```
