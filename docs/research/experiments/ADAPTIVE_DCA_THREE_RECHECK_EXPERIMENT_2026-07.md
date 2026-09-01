# 三次复判自适应定投历史实验

# Three-Recheck Adaptive DCA Historical Experiment

**实验日期 / Date:** 2026-07-20 (AEST)<br>
**独立性 / Independence:** 本报告是对 [一次复判实验](ADAPTIVE_DCA_EXPERIMENT_2026-07.md) 的新增对照；不会修改、替代或重写前一份报告。<br>
**样本 / Samples:** 完全复用前一份报告的 30 个开始月份、资金画像、外部现金流、SPY 数据和历史 `90/10/0` 降级信号。

## 结论摘要 / Executive summary

- 本实验只改变未执行周期的复判状态机：首次执行日未执行后，分别在 **+5、+10、+15 个交易日**最多复判三次；三次后仍为 `TacticalDelay` 或 `Skip`，本月不执行，现金滚入下月。
- This experiment changes only the retry state machine: after a non-executable scheduled decision, it re-checks at **+5, +10, and +15 trading days**, at most three times. If the third re-check remains `TacticalDelay` or `Skip`, the cycle is not executed and cash carries into the next month.
- 自适应终值较高为 **8/30**，平均终值相对普通定投差异为 **-1.65%**。相较一次复判的 `-1.83%`，改善 **0.18 个百分点**；但不能据此宣称策略跑赢普通定投。
- Adaptive DCA has a higher terminal value in **8/30** pairs, with a mean terminal-value difference of **-1.65%** versus plain DCA. This improves by **0.18 percentage points** from the one-recheck result of `-1.83%`, but does not justify any claim of outperformance.

> **完整性声明 / Integrity statement:** 样本、资金流、价格源、信号公式与普通定投基准均从前一份实验直接复用；没有按结果重新选样本。历史 Qwen 输入不可验证，故全部历史决策仍是 `90/10/0` 降级，不得称为完整历史 `70/20/10` 回测。
>
> **Integrity statement:** Samples, cash flows, price source, signal formula, and plain-DCA benchmark are reused directly from the earlier experiment; no samples were reselected after observing results. Historical Qwen inputs cannot be verified, so every historical decision remains a `90/10/0` fallback and must not be described as a complete historical `70/20/10` backtest.

## 1. 执行规则 / Execution rules

每个普通定投 / 自适应定投配对使用相同的开始月份、初始金额、月度现金流和每月最后一个可用交易日这一唯一执行日。

Each plain/adaptive pair uses the same start month, initial amount, monthly cash flow, and one shared execution day: the final available trading day in each month.

| 阶段 / Stage | 普通定投 / Plain DCA | 自适应定投 / Adaptive DCA |
| --- | --- | --- |
| 月度执行日 D | 固定投入当月现金流 / invest fixed monthly flow | 运行冻结的 `90/10/0` 决策 / evaluate frozen `90/10/0` |
| 可执行动作 | 不适用 / n.a. | `Underweight`、`Standard`、`Overweight`：投入滚存现金 + 当月现金流 × 倍率 |
| 第 1 次复判 | 不适用 / n.a. | D + 5 个交易日 / D + 5 trading days |
| 第 2 次复判 | 不适用 / n.a. | D + 10 个交易日 / D + 10 trading days |
| 第 3 次复判 | 不适用 / n.a. | D + 15 个交易日 / D + 15 trading days |
| 三次仍不可执行 | 不适用 / n.a. | 本月不执行；全部未投资现金滚入以后月份 / skip cycle; carry all uninvested cash forward |

每次复判均重新计算当日的历史基本面/趋势输入；历史情绪固定为不可用。滚存资金在未来首次可执行时可全额投入，**不占用当月新现金流的月度上限**。不借贷、不加杠杆，现金按 0% 处理；未纳入分红、税费、滑点、交易费或真实成交约束。

Each re-check recomputes same-day historical fundamental/trend inputs; historical sentiment remains unavailable. On the first later executable date, all carried cash can be invested and **does not consume the new-flow monthly cap**. There is no borrowing or leverage, cash earns 0%, and dividends, tax, slippage, fees, and real-fill constraints are excluded.

## 2. 复判运行统计 / Re-check execution statistics

| 指标 / Metric | 结果 / Result |
| --- | ---: |
| 月度决策周期 / Monthly decision cycles | 360 |
| 无需复判、执行日直接完成 / Completed on scheduled day | 258 |
| 第 1 次复判后完成 / Completed after first re-check | 42 |
| 第 2 次复判后完成 / Completed after second re-check | 8 |
| 进入第 3 次复判 / Reached third re-check | 52 |
| 三次后仍未执行 / Still non-executable after third re-check | 48 |
| 复判调用总数 / Total re-check evaluations | 214 |

48 个“本月不执行”周期不是资金丢失：它们进入滚存余额，在后续可执行月与当月按倍率的新增现金流一起投入；若实验窗口结束前一直没有执行机会，则作为现金计入期末自适应终值。

The 48 skipped cycles do not lose funds: they enter the carry balance and are invested with a later executable month's multiplier-adjusted new flow. If no later execution opportunity occurs before the experiment ends, they remain cash in the adaptive terminal value.

## 3. 历史控制变量结果 / Historical controlled results

**结果列说明 / Result columns:** 两种策略使用相同现金流与终止估值日。`差异 / Delta = (adaptive / plain - 1) × 100%`，是终值差异，不是年化收益率、XIRR 或相对大盘超额收益。

| ID | 开始 / Start | 情境与预算 / Scenario and budget | 普通终值 / Plain end | 自适应终值 / Adaptive end | 差异 / Delta | 较高者 / Higher |
| --- | --- | --- | ---: | ---: | ---: | --- |
| R01 | 2020-01 | 学生 / Student, $500 + $200/月 | $3,295.32 | $3,102.37 | -5.86% | 普通 / Plain |
| R02 | 2020-03 | 上班族 / Worker, $5,000 + $600/月 | $14,983.47 | $13,235.82 | -11.66% | 普通 / Plain |
| R03 | 2020-04 | 家庭 / Family, $30,000 + $2,000/月 | $69,683.83 | $64,864.19 | -6.92% | 普通 / Plain |
| R04 | 2020-06 | 学生 / Student, $500 + $200/月 | $3,226.30 | $3,109.24 | -3.63% | 普通 / Plain |
| R05 | 2020-09 | 上班族 / Worker, $5,000 + $600/月 | $14,428.68 | $13,312.07 | -7.74% | 普通 / Plain |
| R06 | 2021-01 | 家庭 / Family, $30,000 + $2,000/月 | $63,473.67 | $62,021.60 | -2.29% | 普通 / Plain |
| R07 | 2021-03 | 学生 / Student, $500 + $200/月 | $2,743.61 | $2,707.15 | -1.33% | 普通 / Plain |
| R08 | 2021-06 | 上班族 / Worker, $5,000 + $600/月 | $11,061.98 | $11,000.97 | -0.55% | 普通 / Plain |
| R09 | 2021-09 | 家庭 / Family, $30,000 + $2,000/月 | $48,313.06 | $47,341.03 | -2.01% | 普通 / Plain |
| R10 | 2021-11 | 学生 / Student, $500 + $200/月 | $2,503.98 | $2,504.78 | +0.03% | 自适应 / Adaptive |
| R11 | 2022-01 | 上班族 / Worker, $5,000 + $600/月 | $10,663.65 | $10,777.62 | +1.07% | 自适应 / Adaptive |
| R12 | 2022-03 | 家庭 / Family, $30,000 + $2,000/月 | $48,906.64 | $50,068.16 | +2.38% | 自适应 / Adaptive |
| R13 | 2022-06 | 学生 / Student, $500 + $200/月 | $2,883.66 | $2,871.86 | -0.41% | 普通 / Plain |
| R14 | 2022-09 | 上班族 / Worker, $5,000 + $600/月 | $13,592.18 | $13,437.17 | -1.14% | 普通 / Plain |
| R15 | 2022-10 | 家庭 / Family, $30,000 + $2,000/月 | $56,377.11 | $56,468.65 | +0.16% | 自适应 / Adaptive |
| R16 | 2023-01 | 学生 / Student, $500 + $200/月 | $3,031.81 | $3,028.84 | -0.10% | 普通 / Plain |
| R17 | 2023-03 | 上班族 / Worker, $5,000 + $600/月 | $14,219.33 | $14,133.62 | -0.60% | 普通 / Plain |
| R18 | 2023-06 | 家庭 / Family, $30,000 + $2,000/月 | $60,739.51 | $60,469.91 | -0.44% | 普通 / Plain |
| R19 | 2023-08 | 学生 / Student, $500 + $200/月 | $3,114.98 | $3,098.25 | -0.54% | 普通 / Plain |
| R20 | 2023-10 | 上班族 / Worker, $5,000 + $600/月 | $14,186.24 | $13,872.55 | -2.21% | 普通 / Plain |
| R21 | 2024-01 | 家庭 / Family, $30,000 + $2,000/月 | $60,552.00 | $59,037.83 | -2.50% | 普通 / Plain |
| R22 | 2024-04 | 学生 / Student, $500 + $200/月 | $2,739.91 | $2,725.13 | -0.54% | 普通 / Plain |
| R23 | 2024-07 | 上班族 / Worker, $5,000 + $600/月 | $12,724.98 | $12,766.86 | +0.33% | 自适应 / Adaptive |
| R24 | 2024-09 | 家庭 / Family, $30,000 + $2,000/月 | $58,020.08 | $58,301.07 | +0.48% | 自适应 / Adaptive |
| R25 | 2024-11 | 学生 / Student, $500 + $200/月 | $3,047.97 | $3,047.27 | -0.02% | 普通 / Plain |
| R26 | 2025-01 | 上班族 / Worker, $5,000 + $600/月 | $12,984.08 | $13,018.34 | +0.26% | 自适应 / Adaptive |
| R27 | 2025-03 | 家庭 / Family, $30,000 + $2,000/月 | $60,600.37 | $60,618.26 | +0.03% | 自适应 / Adaptive |
| R28 | 2025-04 | 学生 / Student, $500 + $200/月 | $2,898.99 | $2,862.93 | -1.24% | 普通 / Plain |
| R29 | 2025-06 | 上班族 / Worker, $5,000 + $600/月 | $13,548.13 | $13,355.25 | -1.42% | 普通 / Plain |
| R30 | 2025-07 | 家庭 / Family, $30,000 + $2,000/月 | $59,775.16 | $59,077.27 | -1.17% | 普通 / Plain |

| 汇总 / Aggregate | 结果 / Result |
| --- | ---: |
| 配对数量 / Pairs | 30 |
| 自适应终值较高 / Adaptive higher terminal value | 8 / 30 (26.67%) |
| 普通定投终值较高 / Plain higher terminal value | 22 / 30 (73.33%) |
| 平均终值相对差 / Mean terminal relative difference | **-1.65%** |
| 一次复判平均差 / One-recheck mean difference | -1.83% |
| 三次复判相对改善 / Improvement versus one re-check | +0.18 percentage points |

## 4. 解读与限制 / Interpretation and limitations

三次复判使部分短期趋势风险在月内/跨月早期得到重新评估，因而比一次复判略少闲置现金；但改善很小，且胜出组数没有增加。这说明问题不能只靠增加复判次数解决：历史 AI 缺失、`TacticalDelay` 的语义、现金收益、补投期限与月度上限仍会实质影响结果。

Three re-checks allow some short-term trend risks to be reconsidered within the month or early in the next month, leaving slightly less idle cash than one re-check. The improvement is small and the number of winning pairs does not increase. Retry count alone cannot solve the problem: missing historical AI, the semantics of `TacticalDelay`, cash yield, catch-up deadline, and monthly-cap policy all materially affect results.

本实验仍不包含逐月历史 Qwen、新闻可得性、分红、交易成本、滑点、税费、现金利息、多标的组合或真实成交，因此不能用于投资承诺或收益宣传。前一份实验和本报告中的 30 个窗口彼此重叠，不能视为 30 个统计独立的市场时期。

This experiment still excludes monthly historical Qwen, news availability, dividends, trading costs, slippage, tax, cash interest, multi-asset portfolios, and real fills; it must not support investment promises or return marketing. The 30 windows in this and the earlier report overlap, so they are not 30 statistically independent market periods.

## 5. 与当前产品的关系 / Relationship to the current product

本报告是只读离线实验，不提交、撤销或修改 OpenD 订单。当前生产 Scheduler 仍只有固定月度审计，不具备三次复判、滚存状态或自动下单。将此规则接入产品需要单独实现 SQLite 执行状态机、UTC/交易日历、存证、幂等与人工 paper-order 安全门。

This report is a read-only offline experiment and does not submit, cancel, or modify OpenD orders. The current production Scheduler still performs only fixed-monthly auditing; it has no three-recheck state, carry ledger, or automatic order placement. Integrating this rule requires a separate SQLite execution state machine, UTC/trading calendar, audit records, idempotency, and the existing human paper-order safety gate.
