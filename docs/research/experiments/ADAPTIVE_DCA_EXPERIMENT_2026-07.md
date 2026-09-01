# 70/20/10 功能测试与历史降级对照实验

# 70/20/10 Functional Tests and Historical-Fallback Control Experiment

**实验日期 / Date:** 2026-07-20 (AEST)<br>
**代码版本 / Code version:** `main`（本报告对应的提交）<br>
**测试入口 / Test entry:** `crates/decision-engine/tests/scenario_matrix.rs`

## 结论摘要 / Executive summary

- 新增 **40 个可重复运行的功能/正确性测试**：30 个冻结历史场景覆盖 AI 不可用时的 `90/10/0` 降级路径；10 个“当前”冻结 Qwen 情绪场景覆盖正常 `70/20/10` 路径。
- Added **40 deterministic functional/correctness tests**: 30 historical fixtures cover the `90/10/0` AI-unavailable fallback, and 10 frozen current-Qwen fixtures cover the normal `70/20/10` path.
- 30 组历史控制变量对照中，自适应版本在 **8/30** 组终值高于普通定投；两者终值的平均相对差为 **-1.83%**（自适应低于普通定投）。这不是“自适应一定跑赢”的证据，但相较于“延迟即整月持有现金”的旧回放，固定 5 个交易日复判与现金滚存显著缩小了差距。
- Across the 30 controlled historical pairs, the adaptive version finished above plain DCA in **8/30** cases. Its mean terminal-value difference was **-1.83%** versus plain DCA. This is not evidence that adaptive DCA always wins, but the fixed five-trading-day re-check and cash carry-forward materially reduce the gap versus the previous “delay means cash for the whole month” replay.

> **完整性声明 / Integrity statement:** 样本固定覆盖疫情冲击、利率冲击、回撤、复苏与牛市，不按结果筛选；不会为了让自适应策略胜出而更改样本、权重或交易规则。历史期间没有可验证的逐月 Qwen 输入，因此历史部分必须明确按 AI 不可用降级，不能冒充完整的历史 `70/20/10` 回测。
>
> **Integrity statement:** The fixtures span COVID stress, rate shocks, drawdowns, recoveries, and bull markets. They are not altered based on outcomes. Because verified monthly historical Qwen inputs are unavailable, the historical section explicitly uses the AI-unavailable fallback and must not be presented as a full historical `70/20/10` backtest.

## 1. 测试目标与边界 / Goals and boundaries

本次工作验证的是决策引擎对冻结、已校验输入的计算正确性，不直接验证新闻抓取、Qwen 网络调用、OpenD 下单或投资结果。每个 fixture 都断言：权重模式、三层权重、最终分数、倍率与动作标签。

This work verifies decision-engine calculations on frozen, validated inputs. It does not directly test news fetching, the Qwen network call, OpenD order placement, or investment performance. Every fixture asserts the weight mode, all three weights, final score, multiplier, and action.

| 集合 / Set | 数量 / Count | 权重 / Weights | 情绪输入 / Sentiment input | 目的 / Purpose |
| --- | ---: | --- | --- | --- |
| 历史场景 / Historical | 30 | `90/10/0` | 不可用 / unavailable | 验证真实历史无法安全重建 Qwen 时的降级行为 / verify safe fallback |
| 当前 Qwen 场景 / Current Qwen | 10 | `70/20/10` | 冻结的有界分值 / frozen bounded score | 验证正常三层合成，不让 CI 依赖密钥或网络 / verify normal composition without secrets or network |

### 冻结夹具 / Frozen fixtures

30 个历史 fixture 以“使用者类型 + 起始金额 + 月度金额 + 时间/市场事件”命名，例如 `historical_worker_usd5000_usd600_covid_crash_2020_03`。10 个当前 fixture 同样以预算与 Qwen 语境命名，例如 `current_qwen_student_usd200_cautious_news`。这使失败信息可直接定位到可读的业务情境，而不是仅有数组下标。

The 30 historical fixtures are named with “investor profile + initial amount + monthly amount + date/market event”, e.g. `historical_worker_usd5000_usd600_covid_crash_2020_03`. The 10 current fixtures follow the same convention and include a Qwen context, e.g. `current_qwen_student_usd200_cautious_news`. Failures therefore identify a readable business context rather than only an array index.

## 2. 信号与数据方法 / Signal and data methodology

### 决策公式 / Decision formula

- 基本面（70%）：以估值位置的反向分数表示，估值位置越低，投入倾向越高。
- 趋势（20%）：偏好中性节奏；`FallingKnife` 与 `Overheated` 会覆盖为 `TacticalDelay`。
- AI 情绪（10%）：仅在 Qwen 输入可用时参与；`[-1, 1]` 映射到 `[0, 1]`。
- AI 缺失：使用项目既有 `90/10/0` 降级配置，**不伪造历史 Qwen 分数**。

- Fundamental (70%): an inverted valuation-position score; lower valuation position increases willingness to contribute.
- Trend (20%): favors neutral timing; `FallingKnife` and `Overheated` override the label to `TacticalDelay`.
- AI sentiment (10%): contributes only when Qwen input is available; `[-1, 1]` maps to `[0, 1]`.
- AI unavailable: uses the existing `90/10/0` fallback; **no historical Qwen score is fabricated**.

历史数据准备仅用于生成冻结夹具与对照表，读取过程不提交订单：本机 OpenD 的 `US.SPY` 日线收盘价（2010-01-04 至 2026-07-17）、公开 Shiller CAPE 月度序列、FRED `DGS10` 十年期国债收益率和 Cboe VIX 历史数据。信号使用项目默认的长期窗口思想（60 个历史月度观测、36 个月半衰期的 EW 分位）；对照回放只使用 SPY，因此它是单一指数样本，不代表多资产组合。

Historical preparation was only used to produce frozen fixtures and the control table; it placed no orders. Inputs were local OpenD `US.SPY` daily closes (2010-01-04 through 2026-07-17), public Shiller CAPE monthly data, FRED `DGS10` ten-year Treasury yields, and Cboe VIX history. Signals follow the project's default long-window intent (60 historical monthly observations and a 36-month EW-percentile half-life). The replay uses SPY only, so it is a single-index sample, not a multi-asset portfolio result.

## 3. 历史控制变量对照 / Historical controlled comparison

### 规则 / Rules

每个普通定投 / 自适应定投配对共享完全相同的：开始月份、初始现金、月度现金流、每月最后一个可用交易日这一**唯一执行日**及终止估值日。普通定投在该日固定把当月现金流按收盘价买入 SPY；自适应在同一日计算冻结的 `90/10/0` 决策倍率。若动作为 `TacticalDelay` 或 `Skip`，仅在 **5 个交易日后**复判一次；复判仍不可执行时，该月现金滚存。后续某月可执行时，滚存现金可完整投入且**不占用该月新增月度额度**，同时再按当月倍率投入当月新增现金流；不借贷、不加杠杆。未计入分红、税费、滑点、交易费或现金利息。

Each plain/adaptive pair shares exactly the same start month, initial cash, monthly cash flow, one **single execution day** (the final available trading day) per month, and terminal valuation date. Plain DCA buys SPY with that month's fixed flow on the same day. The adaptive replay evaluates the frozen `90/10/0` multiplier on that day. A `TacticalDelay` or `Skip` receives exactly one re-check **five trading days later**; if still not executable, that month's cash rolls forward. When a later month is executable, all carried cash may be invested without consuming that month's new-flow cap, plus the current month's multiplier-adjusted new flow. There is no borrowing or leverage. Dividends, tax, slippage, trading fees, cash interest, and real fill constraints are excluded.

**结果列说明 / Result columns:** `普通终值 / Plain end`、`自适应终值 / Adaptive end` 为美元；`差异 / Delta` = `(adaptive / plain - 1) × 100%`。它是终值差异而非年化收益率、XIRR 或对大盘的超额收益。

| ID | 开始 / Start | 情境与预算 / Scenario and budget | 普通终值 / Plain end | 自适应终值 / Adaptive end | 差异 / Delta | 较高者 / Higher |
| --- | --- | --- | ---: | ---: | ---: | --- |
| H01 | 2020-01 | 学生 / Student, $500 + $200/月 | $3,268.49 | $3,019.95 | -7.60% | 普通 / Plain |
| H02 | 2020-03 | 上班族 / Worker, $5,000 + $600/月 | $14,983.47 | $13,048.06 | -12.92% | 普通 / Plain |
| H03 | 2020-04 | 家庭 / Family, $30,000 + $2,000/月 | $69,049.99 | $63,624.42 | -7.86% | 普通 / Plain |
| H04 | 2020-06 | 学生 / Student, $500 + $200/月 | $3,226.30 | $3,036.98 | -5.87% | 普通 / Plain |
| H05 | 2020-09 | 上班族 / Worker, $5,000 + $600/月 | $14,428.68 | $13,302.64 | -7.80% | 普通 / Plain |
| H06 | 2021-01 | 家庭 / Family, $30,000 + $2,000/月 | $63,473.67 | $62,038.69 | -2.26% | 普通 / Plain |
| H07 | 2021-03 | 学生 / Student, $500 + $200/月 | $2,743.61 | $2,706.72 | -1.34% | 普通 / Plain |
| H08 | 2021-06 | 上班族 / Worker, $5,000 + $600/月 | $11,061.98 | $11,014.16 | -0.43% | 普通 / Plain |
| H09 | 2021-09 | 家庭 / Family, $30,000 + $2,000/月 | $48,313.06 | $47,679.16 | -1.31% | 普通 / Plain |
| H10 | 2021-11 | 学生 / Student, $500 + $200/月 | $2,503.98 | $2,504.78 | +0.03% | 自适应 / Adaptive |
| H11 | 2022-01 | 上班族 / Worker, $5,000 + $600/月 | $10,663.65 | $10,777.62 | +1.07% | 自适应 / Adaptive |
| H12 | 2022-03 | 家庭 / Family, $30,000 + $2,000/月 | $48,906.64 | $50,068.16 | +2.38% | 自适应 / Adaptive |
| H13 | 2022-06 | 学生 / Student, $500 + $200/月 | $2,883.66 | $2,871.86 | -0.41% | 普通 / Plain |
| H14 | 2022-09 | 上班族 / Worker, $5,000 + $600/月 | $13,592.18 | $13,437.17 | -1.14% | 普通 / Plain |
| H15 | 2022-10 | 家庭 / Family, $30,000 + $2,000/月 | $56,377.11 | $56,468.65 | +0.16% | 自适应 / Adaptive |
| H16 | 2023-01 | 学生 / Student, $500 + $200/月 | $3,031.81 | $3,028.84 | -0.10% | 普通 / Plain |
| H17 | 2023-03 | 上班族 / Worker, $5,000 + $600/月 | $13,974.68 | $13,906.30 | -0.49% | 普通 / Plain |
| H18 | 2023-06 | 家庭 / Family, $30,000 + $2,000/月 | $60,739.51 | $60,469.91 | -0.44% | 普通 / Plain |
| H19 | 2023-08 | 学生 / Student, $500 + $200/月 | $3,114.98 | $3,098.25 | -0.54% | 普通 / Plain |
| H20 | 2023-10 | 上班族 / Worker, $5,000 + $600/月 | $14,186.24 | $13,872.55 | -2.21% | 普通 / Plain |
| H21 | 2024-01 | 家庭 / Family, $30,000 + $2,000/月 | $60,552.00 | $59,037.83 | -2.50% | 普通 / Plain |
| H22 | 2024-04 | 学生 / Student, $500 + $200/月 | $2,739.91 | $2,725.13 | -0.54% | 普通 / Plain |
| H23 | 2024-07 | 上班族 / Worker, $5,000 + $600/月 | $12,724.98 | $12,766.86 | +0.33% | 自适应 / Adaptive |
| H24 | 2024-09 | 家庭 / Family, $30,000 + $2,000/月 | $58,020.08 | $58,301.07 | +0.48% | 自适应 / Adaptive |
| H25 | 2024-11 | 学生 / Student, $500 + $200/月 | $3,047.97 | $3,047.27 | -0.02% | 普通 / Plain |
| H26 | 2025-01 | 上班族 / Worker, $5,000 + $600/月 | $12,984.08 | $13,018.34 | +0.26% | 自适应 / Adaptive |
| H27 | 2025-03 | 家庭 / Family, $30,000 + $2,000/月 | $60,600.37 | $60,618.26 | +0.03% | 自适应 / Adaptive |
| H28 | 2025-04 | 学生 / Student, $500 + $200/月 | $2,898.99 | $2,862.93 | -1.24% | 普通 / Plain |
| H29 | 2025-06 | 上班族 / Worker, $5,000 + $600/月 | $13,548.13 | $13,355.25 | -1.42% | 普通 / Plain |
| H30 | 2025-07 | 家庭 / Family, $30,000 + $2,000/月 | $59,775.16 | $59,077.27 | -1.17% | 普通 / Plain |

### 汇总 / Aggregate result

| 指标 / Metric | 结果 / Result |
| --- | ---: |
| 配对数量 / Pairs | 30 |
| 自适应终值较高 / Adaptive higher terminal value | 8 / 30 (26.67%) |
| 普通定投终值较高 / Plain higher terminal value | 22 / 30 (73.33%) |
| 平均终值相对差 / Mean terminal relative difference | **-1.83%** |
| 固定 5 日复判次数 / Fixed five-day re-checks | 102 |
| 复判后仍未执行次数 / Still unexecuted after re-check | 60 |

**解读 / Interpretation:** 固定 5 日复判与滚存后，平均差异由旧规则的 `-6.11%` 缩小为 `-1.83%`，说明“延迟等于整月放弃”的假设过于保守。但仍有 22/30 组普通定投终值更高：趋势风险标签出现后，短期复判仍可能错过上涨或现金持续积累。完整研究仍须定义“复判后何时补投”“滚存现金是否有最大期限”“现金是否计息”，再做时间切分的样本外验证。

**Interpretation:** With a fixed five-day re-check and carry-forward, the mean difference improves from `-6.11%` to `-1.83%`, showing that “delay means no investment for the whole month” was overly conservative. Yet plain DCA still has the higher terminal value in 22/30 cases: a short re-check can still miss a rally or leave cash accumulating after trend-risk labels. A complete study must still define when to catch up after re-checking, whether carried cash has a maximum age, and whether cash earns interest, then validate on a time-split out-of-sample period.

## 4. 当前 Qwen 正常路径功能矩阵 / Current-Qwen normal-path matrix

这些不是历史新闻重放，而是无密钥、无网络的冻结 Qwen 分数，以验证 Qwen 可用时确实使用 `70/20/10`，并覆盖谨慎、积极、均衡、下跌风险与过热风险等动作。

These are not historical-news replays. They are frozen Qwen scores with no secret or network dependency, verifying that available Qwen sentiment actually selects `70/20/10` and covers cautious, positive, balanced, falling-knife, and overheated cases.

| ID | 使用者与预算 / Profile and budget | 基本面 / Fundamental | 趋势 / Trend | Qwen | 动作 / Action |
| --- | --- | ---: | ---: | ---: | --- |
| Q01 | 学生 $200/月，谨慎新闻 / student, cautious news | 0.85 | 0.45 | -0.60 | 减量 / Underweight |
| Q02 | 学生 $200/月，低估值 / student, low valuation | 0.25 | 0.50 | +0.40 | 加码 / Overweight |
| Q03 | 上班族 $600/月，均衡新闻 / worker, balanced news | 0.50 | 0.50 | 0.00 | 标准 / Standard |
| Q04 | 上班族 $600/月，积极新闻 / worker, positive news | 0.35 | 0.50 | +0.80 | 加码 / Overweight |
| Q05 | 家庭 $2,000/月，高估值 / family, high valuation | 0.80 | 0.50 | +0.60 | 减量 / Underweight |
| Q06 | 家庭 $2,000/月，下跌风险 / family, falling knife | 0.30 | 0.85 | +0.20 | 延迟 / Tactical delay |
| Q07 | 毕业生 $400/月，复苏新闻 / graduate, recovery news | 0.20 | 0.40 | -0.20 | 加码 / Overweight |
| Q08 | 投资者 $5,000/月，过热新闻 / investor, overheated news | 0.90 | 0.10 | -0.80 | 延迟 / Tactical delay |
| Q09 | ETF $1,000/月，混合新闻 / ETF, mixed news | 0.65 | 0.60 | +0.10 | 标准 / Standard |
| Q10 | 退休账户 $3,000/月，防御新闻 / retirement, defensive news | 0.45 | 0.50 | -0.40 | 标准 / Standard |

## 5. 可复现性与限制 / Reproducibility and limitations

运行以下命令可以重跑引擎的 50 个测试（10 个既有单元测试 + 40 个场景测试）：

Run the following command to execute all 50 engine tests (10 existing unit tests plus 40 scenario tests):

```bash
cargo test -p decision-engine --locked
```

本报告的历史表格来自固定的离线数据准备结果；原始下载文件、Qwen 密钥、账户信息及任何交易凭据均不会提交到仓库。若需要严格可复现的原始数据管线，应在后续工作中加入具有许可的数据归档、哈希清单和可审计的数据下载脚本。

The historical table comes from fixed offline preparation output. Raw downloads, Qwen keys, account details, and any trading credentials are not committed. Strict reproduction of the raw-data pipeline requires a licensed data archive, hash manifest, and auditable downloader in later work.

### 不能得出的结论 / Claims this report does not support

- 不能得出“70/20/10 长期必然优于普通定投”。
- 不能把 `90/10/0` 历史降级结果称为完整 Qwen 历史回测。
- 不能以单一 SPY、重叠 12 个月窗口或未含分红/费用的终值比较代表真实账户收益。
- 不构成投资建议；本系统仅测量历史位置与规则化风险偏好，不判断证券内在价值。

- It does not prove that 70/20/10 always outperforms plain DCA over the long term.
- It does not describe the `90/10/0` historical fallback as a complete historical Qwen backtest.
- A single SPY sample, overlapping 12-month windows, and terminal values excluding dividends/fees do not represent real-account returns.
- This is not investment advice; the system measures historical position and rule-based risk preference, not intrinsic value.

## 6. 下一步 / Next steps

1. 为历史新闻与模型输出建立经许可、带时间戳和哈希的归档，才可运行真正的逐月 `70/20/10` 回测。
2. 明确定义 `TacticalDelay` 的补投期限、月度额度结转与现金收益，然后将这些规则纳入回放与生产调度。
3. 增加多标的、非重叠的时间切分样本及分红/费用/滑点，报告终值、XIRR、最大回撤和相对大盘指标。

1. Build a licensed, timestamped, hashed archive of historical news and model outputs before attempting a true monthly `70/20/10` backtest.
2. Define the reinvestment deadline for `TacticalDelay`, monthly-cap rollover, and cash yield, then use the same rules in replay and production scheduling.
3. Add multi-asset, non-overlapping time-split samples plus dividends, fees, and slippage; report terminal value, XIRR, maximum drawdown, and benchmark-relative metrics.
