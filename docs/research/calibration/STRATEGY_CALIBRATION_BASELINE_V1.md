# Strategy Calibration Baseline V1 / 策略校准基线 V1

> Status / 状态：已完成当前生产规则的离线、可复现基线测量；**未**据此修改任何生产权重、阈值或下单逻辑。
> Generated / 生成日期：2026-08-21
> Machine-readable result / 机器可读结果：[calibration-v1.report.json](crates/strategy-evaluation/data/generated/calibration-v1.report.json)

## 1. Purpose / 目的

This baseline measures the current IndexLink rules before any strategy tuning.
It is designed to falsify attractive narratives, not to manufacture an
outperformance claim.

本基线在调整策略以前测量现有 IndexLink 规则。目的不是制造“跑赢”叙事，
而是让分数分布、动作、现金拖累和风险口径可被复跑与证伪。

The evaluator calls the current production-domain functions directly:

- `evaluate_fundamental`
- `evaluate_trend`
- `evaluate_decision`
- `TwoBucketContributionSplit::from_decision_with_carry`

It performs no HTTP, broker, Qwen-key, or order IO.

## 2. Versioned data / 版本化数据

| Item / 项目 | Value / 数值 |
| --- | --- |
| Dataset / 数据集 | `calibration-v1` |
| Capture date / 采集日期 | 2026-08-21 |
| Frequency / 频率 | 每月最后一个可得交易观察值 |
| Price proxies / 价格代理 | FRED S&P 500 (`SP500`, SPY proxy)；FRED NASDAQ Composite (`NASDAQCOM`, QQQ proxy) |
| Factors / 因子 | Shiller CAPE、ERP proxy `100/CAPE - DGS10`、MA200 distance、RSI-14、VIX |
| Missing values / 缺失值 | 任一必需因子缺失即丢弃当月；不前填、不插值 |
| Integrity / 完整性 | 原始快照、生成器和 SHA-256 位于 `crates/strategy-evaluation/data/` 与 `calibration-v1.manifest.json` |

These are US-equity **index proxies**, not historical SPY/QQQ execution
prices. FRED's S&P 500 public series begins in 2016, so its first scored month
after the five-year monthly warm-up is 2022-06. NASDAQ Composite preserves a
longer scored range from 2010-10 and therefore includes the 2020 shock.

这些是美国权益**指数代理**，不是 SPY/QQQ 的历史成交价。FRED S&P 500
公开序列始于 2016 年，因此在五年按月预热后从 2022-06 才产生首个评分；
NASDAQ Composite 自 2010-10 起保留更长评分区间，涵盖 2020 年冲击。

### No look-ahead / 无未来函数

For a decision at date `t`, the evaluator passes exactly the preceding 60
monthly observations to each production factor function and passes the
observation at `t` as current input. Future rows are not available to a
decision. The focused test asserts this count for every asset.

对日期 `t` 的决策，评估器只把此前 60 个按月样本交给生产因子函数，并将
`t` 当日作为当前输入；未来行不会进入计算。聚焦测试逐个标的验证此边界。

## 3. Fair comparison protocol / 公平对照口径

Each evaluated strategy receives the same USD 1,000 external cash flow each
month, buys at close with the same 5 bps price impact, receives no interest on
cash, and includes all uninvested cash in terminal wealth. No dividends,
taxes, borrow, or leverage are assumed.

每条策略每月获得同样的 USD 1,000 外部现金流，以相同收盘价和 5 bps 买入
冲击成交；现金不计利息，且未投入现金计入期末净值。不假设股息、税、借贷或杠杆。

- **Fixed DCA / 固定定投**: spends the full period budget every scored month.
- **Core/Opportunity intent / 核心+机会意图**: 70% core plus 30% opportunity;
  the opportunity side uses the current multiplier and carry-forward policy.
- **Current API-effective / 当前 API 实效口径**: reflects the corrected API order
  gate: `Skip`/`TacticalDelay` reduce the opportunity bucket to zero, while a
  due validated order still carries the preserved core bucket. It is reported
  separately to prevent future API regressions from silently diverging from the
  domain split.

## 4. Historical causal line / 历史因果线

Historical news cannot be reconstructed faithfully from a live Qwen call.
Historical performance therefore uses `DecisionSentiment::Unavailable` and the
current **90/10/0** fallback. A separate frozen ten-score Qwen JSON cycle is
used only to compare 70/20/10 versus fallback score/action distributions; it is
not included in return, XIRR, or risk claims.

历史新闻不能由今天的 Qwen 调用可信地复原。因此所有历史收益使用
`DecisionSentiment::Unavailable` 和当前 **90/10/0** 降级。另有冻结的十个
Qwen 分数 JSON 循环，仅用于比较 70/20/10 与降级线的分数/动作分布；绝不计入
收益、XIRR 或风险结论。

## 5. Actual baseline results / 实测基线结果

### Score and action distributions / 分数与动作分布

| Proxy / 代理 | Scored months / 评分月数 | Mean | Median | P10 | P25 | P75 | P90 | Actions / 动作分布 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| S&P 500 (SPY proxy) | 49 | 0.234 | 0.191 | 0.075 | 0.106 | 0.362 | 0.468 | Skip 1, Underweight 30, Standard 11, Overweight 1, TacticalDelay 6 |
| NASDAQ Composite (QQQ proxy) | 189 | 0.309 | 0.280 | 0.079 | 0.155 | 0.428 | 0.591 | Skip 0, Underweight 99, Standard 26, Overweight 18, TacticalDelay 46 |

The low centre of the distribution is evidence for calibration review; it is
not evidence that the market was objectively unattractive. In the current
functions, the trend timing transform is bounded at 0.5 and the unavailable-AI
fallback removes the 10% sentiment contribution, which materially constrains
the final score.

分数中心偏低说明应做校准审查，**不**说明市场客观上“不值得投资”。当前趋势节奏
变换上限为 0.5，且 AI 不可用时 10% 情绪贡献消失，这两点都会显著压低最终分数。

### Layer means (historical fallback) / 各层均值（历史降级线）

| Proxy / 代理 | Fundamental raw / 基本面原始 | Fundamental directional / 方向变换后 | Fundamental contribution / 加权贡献 | Trend raw / 趋势原始 | Trend timing / 节奏变换后 | Trend contribution / 加权贡献 | AI contribution / AI 贡献 | Final / 最终 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| S&P 500 proxy | 0.774 | 0.226 | 0.204 | 0.457 | 0.302 | 0.030 | 0.000 | 0.234 |
| NASDAQ proxy | 0.690 | 0.310 | 0.279 | 0.460 | 0.304 | 0.030 | 0.000 | 0.309 |

### Matched return and risk comparison / 匹配资金流的收益与风险对照

| Proxy / 代理 | Strategy / 策略 | XIRR | Terminal wealth / 期末净值 | Max drawdown / 最大回撤 | Annualised volatility / 年化波动 | Cash utilisation / 现金使用率 | Terminal cash / 期末现金 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| S&P 500 proxy | Fixed DCA | 19.61% | $71,669 | 9.70% | 13.32% | 100.00% | $0 |
| S&P 500 proxy | Core/Opportunity intent | 17.54% | $68,926 | 9.44% | 12.28% | 82.65% | $8,502 |
| S&P 500 proxy | Current API-effective | 17.54% | $68,926 | 9.44% | 12.28% | 82.65% | $8,502 |
| NASDAQ proxy | Fixed DCA | 16.88% | $815,385 | 33.03% | 16.83% | 100.00% | $0 |
| NASDAQ proxy | Core/Opportunity intent | 15.84% | $740,761 | 31.29% | 15.71% | 83.31% | $31,537 |
| NASDAQ proxy | Current API-effective | 15.84% | $740,761 | 31.29% | 15.71% | 83.31% | $31,537 |

Core/Opportunity intent terminal wealth is **-3.83%** versus DCA for the S&P
500 proxy and **-9.15%** for the NASDAQ proxy. Lower drawdown and volatility
occur here alongside substantially lower cash utilisation; they cannot be
claimed as an unqualified improvement. The corrected API implementation now
matches the domain split for this boundary; current API-effective results equal
Core/Opportunity intent in this fixture. The separate line remains as a
regression check.

核心+机会意图相对固定定投的期末净值：S&P 500 代理 **-3.83%**，NASDAQ 代理
**-9.15%**。回撤、波动较低同时伴随明显较低的现金使用率，不能包装为无条件改善。
已修正的 API 实现在此边界与领域层拆分一致；本夹具中当前 API 实效结果与核心/机会
意图相同。保留单独行用于防止未来回归。

### Rolling out-of-sample windows / 滚动样本外窗口

After the five-year warm-up, 24-month windows advance by 12 months. The S&P
500 proxy has 3 complete windows: intent beats DCA in 0/3. The NASDAQ proxy has
14 complete windows: intent beats DCA in 1/14. Every individual window and all
metrics are retained in the machine-readable report; no windows were selected
or removed based on outcome.

五年预热后，使用 24 个月窗口、每 12 个月前移。S&P 500 代理有 3 个完整窗口，
意图策略跑赢固定定投 0/3；NASDAQ 代理有 14 个完整窗口，跑赢 1/14。每个窗口和
全部指标均保留在机器可读报告中，未按结果挑选或删除窗口。

### Frozen Qwen sensitivity / 冻结 Qwen 敏感性

Across 238 scored observations, frozen 70/20/10 sensitivity raises mean score
from **0.294** (90/10/0 fallback) to **0.316** (+0.022). It changes action
counts from Skip/Underweight/Standard/Overweight/TacticalDelay =
1/129/37/19/52 to 0/121/51/14/52. This is a distribution check only, not a
statement about Qwen's historical accuracy or investment value.

在 238 条评分观察中，冻结的 70/20/10 敏感性将平均分从 **0.294** 提升至
**0.316**（+0.022）；动作从 Skip/Underweight/Standard/Overweight/TacticalDelay =
1/129/37/19/52 变为 0/121/51/14/52。这只是分布检查，不代表 Qwen 的历史准确性或投资价值。

## 6. Reproduce / 复跑

```bash
python3 tools/generate_calibration_fixture.py
cargo test -p strategy-evaluation --locked
cargo run -p strategy-evaluation --locked -- \
  crates/strategy-evaluation/data/generated/calibration-v1.report.json
```

## 7. Resume-safe summary / 可用于简历的诚实表述

> Built a reproducible historical evaluation pipeline across **2 US-equity
> index proxies** and **2010–2026 / 16 scored years for the longer proxy**
> (with a common SPY-proxy scoring range from 2022), under matched monthly cash
> flows and 5 bps buy-cost assumptions; compared Core/Opportunity with fixed
> DCA using XIRR, terminal wealth, maximum drawdown, volatility, and cash
> utilisation, reporting actual rather than selected results.

中文：构建覆盖 **2 个美国权益指数代理**、最长 **2010–2026 / 16 个评分年份**
（SPY 代理的共同评分范围自 2022 起）的可复现历史评估管线；在一致月度现金流和
5 bps 买入成本假设下，以 XIRR、期末净值、最大回撤、波动率和现金使用率对比
核心/机会策略与固定定投，并如实报告结果而非挑选结果。

## 8. Limits and next decision / 局限与下一步决策

1. Index levels omit dividends, tax, and actual ETF spreads; this is a rule
   calibration baseline, not a tradable performance claim.
2. CAPE is a US-equity valuation input; this dataset must not be reused to
   score IEF, GLD, or other asset classes without an asset-appropriate model.
3. Frozen Qwen samples are not historical text reconstruction, so no 70/20/10
   historical return attribution is claimed.
4. The next change must be a separately versioned candidate rule, selected
   before looking at its hold-out results. It must preserve the same costs,
   cash accounting, and out-of-sample protocol.
5. The API-level order gate now submits the preserved core amount for
   `Skip`/`TacticalDelay`; route tests protect this boundary. Future changes must
   retain this split rather than reintroducing a global veto.
