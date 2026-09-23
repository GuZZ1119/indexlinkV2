# IndexLink API 管理清单

本文档用于前后端对接和 MVP 范围管理，记录当前已经可用的 HTTP API、请求/响应约定，以及后续仍需补充的接口。

## 通用约定

- 默认请求和响应均为 JSON。
- 金额、比例、数量等 decimal 字段在 JSON 中使用字符串，例如 `"1000.00"`、`"0.80"`、`"1.00"`。
- UUID 路径参数非法时返回 `400 bad_request`。
- 资源不存在时返回 `404 not_found`。
- 已发送订单但未收到可信回执时返回 `409 order_outcome_unknown`；客户端不得自动重试。
- 重复提交已存在的不可变资源标识时返回 `409 conflict`。
- 服务依赖不可用时返回 `503 service_unavailable`。

统一错误响应：

```json
{
  "error": {
    "code": "bad_request",
    "message": "invalid request"
  }
}
```

## 已有 API

### 健康检查

#### `GET /health`

用于服务存活检查。

响应：

```json
{
  "status": "ok",
  "service": "indexlink-server",
  "version": "0.1.0"
}
```

#### `GET /ready`

用于依赖就绪检查，当前主要检查数据库。

成功响应：

```json
{
  "status": "ready",
  "database": "ok"
}
```

#### `GET /runtime-status`

返回 SQLite 核心、可选 adapter 与调度器的展示安全状态，不会主动调用外部行情、AI 或 broker。`market_data`、`historical_prices` 与 `paper_broker` 均为三态：`not_configured` 表示运营方未启用，`configured` 表示 adapter 已装配，`unavailable` 表示已启用但初始化失败。`market_data` 是旧 Core/Opportunity 兼容决策使用的 CAPE、国债、VIX 与技术信号整包；`historical_prices` 是 Formula 实时决策和产品回测使用的规范化历史日线。可选能力不可用不会改变核心服务的存活状态；依赖这些能力的具体路由会返回 `503 service_unavailable`。

响应示例：

```json
{
  "service": "running",
  "database": "ready",
  "market_data": "not_configured",
  "historical_prices": "not_configured",
  "qwen": "not_configured",
  "ai_provider_profiles": [],
  "paper_broker": "unavailable",
  "scheduler": {
    "enabled": true,
    "tick_interval_seconds": 60,
    "last_tick_at": null,
    "last_summary": null,
    "last_error_at": null
  }
}
```

### Investment Plans

#### `POST /investment-plans`

创建投资计划。

请求：

```json
{
  "name": "Core ETF",
  "symbol": "voo",
  "base_contribution": "1000.00",
  "currency": "usd",
  "schedule_kind": "monthly",
  "schedule_day": 15,
  "bucket_allocation": {
    "core_ratio": "0.80",
    "opportunity_ratio": "0.20"
  },
  "risk_mode": "approval",
  "opportunity_cash_policy": "carry_forward",
  "max_single_execution": "1500.00"
}
```

成功状态码：`201 Created`

响应：创建后的 investment plan。服务端会规范化 `symbol` 与 `currency` 为大写。官方目录计划的 `symbol` 接受 `US.AAPL`、`HK.00700`、`SH.600519`、`SZ.000001`；无前缀代码兼容解释为美股。港股短代码会补足为五位。客户端提交的币种必须与解析出的市场一致（美股 `USD`、港股 `HKD`、沪深 `CNY`），服务端会再次推导并保存权威币种，不信任浏览器用 `USD` 代替全市场事实。

`bucket_allocation` 的比例使用 `0..=1` 的 decimal 字符串（例如 `0.80` 即 `80%`），两桶必须恰好合计 `1`。当核心桶为 `100%` 时，`risk_mode` 必须为 `fixed`，且 `opportunity_cash_policy` 只能为默认 `expire_each_period`；存在机会桶时必须显式选择 `autopilot` 或 `approval`。

`opportunity_cash_policy` 可为 `expire_each_period`、`carry_forward` 或 `carry_with_cap`。`carry_with_cap` 必须同时提交正的 `opportunity_cash_cap`（金额型上限）；期数型上限尚未实现。`period_execution_limit` 是同一周或月内自动 paper order 的累计金额上限；每笔自动订单先原子预留，broker 接受后确认，终态 `filled`/`closed` 的实际成交金额会回写修正占用。pending/partial 订单保守维持估计占用。legacy 手动 preview 保留显式数量兼容，尚不进入该自动金额账本。为兼容旧客户端，省略新增配置时默认 `100%` 核心桶、`fixed` 与 `expire_each_period`。

`schedule_kind` 接受 `monthly`（日期为 `1..=28`）或 `weekly`（ISO 星期为 `1..=7`）。`schedule_days` 可提供同一周期的多个固定日，必须有序、无重复，且其第一项必须等于兼容字段 `schedule_day`；省略时等价于仅 `[schedule_day]`。scheduler 使用此集合按 UTC 日期运行。

普通用户从 `GET /strategy-catalog` 采用策略时，必须原样提交目录中的 `policy.id` / `policy.version` 与 `default_plan` 的核心桶、弹性桶和风险模式。`fixed_dca@1` 使用 100% 核心桶；官方与个人 Formula V1 使用 70% 核心桶、30% 弹性桶和 `approval`。服务端只按这份精确引用读取官方注册表或 SQLite 中已经保存并重新校验的公式，创建与更新 DTO 会拒绝客户端额外提交的 `formula` 字段；计划一旦创建便冻结该版本，不会自动跟随同 ID 的后续版本。

Formula 计划在写入前统一执行标的/市场/币种校验、真实周期预算检查、所需指标与最长 lookback 推导、行情完整性和最近数据时效检查；缺少行情能力、历史长度不足、数据过期、固定金额动作或当前没有 canonical provider 的 VIX 依赖都会故障关闭，且不会留下半条计划记录。自动建议、scheduler 与 decision preview 随后从服务端再次解析同一个已保存版本并复用 DSL 解释器。目录使用 `supported_markets` 和 `data_requirement.required_close_observations` 声明能力；已弃用的 `supported_symbols` 始终为空，不能再作为准入白名单。

Fixed DCA 只做代码格式、市场与币种一致性检查，不读取行情，因此未配置 OpenD/历史数据 provider 时仍可创建。官方 Formula 在创建、更新策略版本或激活既有计划之前，会通过当前 `HistoricalPriceProvider` 为该标的读取策略声明的最小有效日线窗口；历史不足或不支持的请求返回 `400 bad_request`，provider 未配置、认证/网络故障或数据集不一致返回 `503 service_unavailable`，且不会保存半成品计划。通过这项检查只代表数据足够运行确定性公式，不代表策略适合该证券，也不构成投资建议。

#### `GET /investment-plans`

列出所有投资计划。

响应：investment plan 数组。

#### `GET /investment-plans/:id`

按 ID 获取单个投资计划。

#### `PATCH /investment-plans/:id`

更新投资计划。字段均为可选，但不能提交空对象 `{}`。

请求示例：

```json
{
  "name": "Core ETF Plus",
  "base_contribution": "1200.00",
  "schedule_day": 20,
  "bucket_allocation": {
    "core_ratio": "0.70",
    "opportunity_ratio": "0.30"
  },
  "risk_mode": "autopilot",
  "opportunity_cash_policy": "carry_forward",
  "max_single_execution": "1800.00",
  "is_active": false
}
```

响应：更新后的 investment plan。

#### `DELETE /investment-plans/:id`

删除一个定投标的，成功返回 `204 No Content`。本机 SQLite 会通过外键级联删除该标的关联的 decision record、纸面订单、已观察成交、现金流与收益快照；该操作不可恢复，不会影响 OpenD 账户中的任何订单或持仓。

### Restricted DSL Strategy Versions

#### `GET /strategy-catalog`

返回统一的本机策略目录。目录首先返回一个 `fixed_dca@1` 基准，以及由 20 个透明规则家族各生成 5 档参数预设的 100 个不可变官方 Formula V1 版本；随后追加通过 `POST /strategies` 保存到本机的个人受限 Formula 版本。原有 `dsl_ma200_trend_guard@1` 与 `dsl_growth_volatility_balance@1` 的 ID、版本和公式保持不变。旧 `core_opportunity_v1@1` 依赖历史 70/20/10 与 AI 降级口径，不进入普通目录，也不能被目录前端误显示为可采用策略。

每项都以完整 `policy.id` / `policy.version` 作为唯一身份，并增加三个不能由名称推断的状态字段：

- `origin`：`official` 表示随当前二进制发布的官方版本，`personal` 表示保存在本机 SQLite 的个人版本；
- `lifecycle`：官方版本为 `published`，个人版本为 `saved`。个人版本的 `(policy_id, policy_version)` 是数据库主键，保存后不可原地覆盖；修改规则必须使用新版本；
- `status`：`usable` 表示当前版本已满足现有计划准入边界，`validated` 只表示受限 DSL 结构已通过领域校验，不代表可以建立计划。客户端仍必须以 `adoptable` 作为是否开放计划创建的直接布尔值。

个人条目会内嵌保存时的规范化 `formula`，不提供伪造的官方 family、preset 或公开研究来源；`source` 因此省略，`tags` 标记为个人受限规则。可由当前倍率/跳过运行时执行的个人版本会运行既有固定样本 admission：通过时 `validation_mode=fixed_fixture`、`research_status=available`、`status=usable`；未通过，或含线上计划运行时尚不支持的固定金额动作时，保持 `status=validated`、`adoptable=false` 与 `research_status=blocked`。保存状态和可用状态是两件事，目录不会把“已保存”包装成“已发布”或“可执行”。目录合并只是现有官方注册表与不可变策略存储的只读投影，不创建新版本、不修改公式，也不需要新增数据库迁移。

Formula 条目通过 `family`、`preset`、`tags` 与 `source` 提供分组、参数、检索标签和公开思想来源。`name` 是面向目录的参数化展示名（例如 `价格与指数（20日）`），`preset.name` 直接写出窗口组合（例如 `20日` 或 `趋势150日 / 波动63日`），不再使用“灵敏 / 稳健 / 长期”等无法直接核验的定性名称；策略的 `policy.id` / `policy.version` 与内部不可变名称不受展示文案调整影响。`source.adaptation` 会明确说明 IndexLink 只参考指标或研究思想，Formula 规则为独立实现，不复制第三方交易代码。`validation_mode` 有三种取值：`reference` 表示固定投入基准，`fixed_fixture` 表示兼容的两条原有策略附带完整固定样本研究，`compiled_formula` 表示其余生成预设已完成结构、预算和公式编译校验。为避免目录响应重复携带 98 份大型公式与研究结果，`compiled_formula` 条目不内嵌 `formula` / `research`；客户端可按精确 ID 读取 `/strategies/:id/:version`，实际采用仍必须通过所选标的的历史数据预检，回测则使用 `/strategy-backtests`。

每项包含 `origin`、`lifecycle`、`status`、`policy`、普通话名称/摘要、确定性规则、局限、风险标签、`supported_markets`、默认计划配置、可读 `data_requirements`、机器可读 `data_requirement.required_close_observations`、`adoptable` 与 `research_status`。`supported_symbols` 是只为旧客户端保留的弃用字段，固定返回空数组。官方 DSL 项还返回规范化 `formula` 和真实固定样本 `research`；只有 admission 的 `eligible` 为真时 `adoptable` 才为真。Fixed DCA 是对照基准，因此 `research_status` 为 `reference` 且不伪造 DSL 公式或差异化回测。当前官方 Formula V1 的准入研究仍只覆盖固定样本中的 S&P 500 / Nasdaq Composite 指数代理；其他 US/HK/SH/SZ 标的可以在满足数据窗口时运行，但界面必须明确“可计算”不等于“已证明适合”。

两个 Formula V1 策略默认使用 70% 核心桶与 30% 弹性桶：规则只能调整弹性桶，不能取消核心投入。目录读取不会保存计划、创建 decision、读取 AI 或提交订单。

#### `POST /strategy-backtests`

在一个用户选择的标的上，以相同日线快照、显示区间、月度投入日、外部现金流、5 bps 买入成本和成交时点比较 1–3 个官方或本机已保存的个人策略。该接口读取配置到 `ApiState` 的通用 `HistoricalPriceProvider`，再调用无 IO 的生产 Formula V1 回测 runtime；旧 MA200 专用回放端点已经删除，本接口也不调用 broker 或自动下单。

请求中的 `symbol` 接受 `US.SPY`、`HK.00700`、`SH.600519`、`SZ.000001`；无前缀符号兼容解释为美股。`range` 只接受 `1m`、`3m`、`6m`、`1y`、`3y`、`5y`、`all`。`monthly_day` 限制为 1–28，避免不同月份没有该日。新调用方使用 `strategy_refs` 提交 1–3 个精确的 `policy_id` / `policy_version`，从而让个人策略和官方策略采用同一不可变身份；引用不存在、版本错误或本地文档损坏时明确失败，不会改用最新版本、Fixed DCA 或演示策略。兼容字段 `strategy_ids` 只接受当前官方目录 ID，并由服务端解析到其目录版本；`strategy_refs` 与 `strategy_ids` 不能同时出现。金额使用十进制字符串，禁止 0 或负数，并以响应 `data.currency` 所示的标的交易币种解释。

```json
{
  "symbol": "HK.00700",
  "strategy_refs": [
    {"policy_id": "fixed_dca", "policy_version": 1},
    {"policy_id": "dsl_ma200_trend_guard", "policy_version": 1}
  ],
  "range": "3y",
  "monthly_day": 18,
  "contribution": "1000.00"
}
```

成功响应将所选 `requested_range`、`data` 来源元数据和 `result` 分开：

- `data`：provider、market、instrument type、currency、timezone、adjustment、导入时间、请求起止日、dataset version 与 SHA-256 checksum；美股请求 `all` 复权，港股/A 股请求前复权，实际能力仍由所配置 provider 明确决定；
- `result.effective_start/effective_end`：所有策略均有完整因果预热后的共同有效窗口；
- `result.contribution_count`：每个策略完全相同的外部投入次数；
- `result.market_points[]`：共同有效窗口内、来自同一数据快照的每日 `date` 与 `adjusted_close`；它用于把策略结果放回真实标的走势中解释，不是第二次行情请求；
- `result.series[]`：不可变策略 ID/version/name、服务端生成的每日归一化轨迹、专业指标、逐日回撤和模拟资金账本；
- `result.series[].execution_points[]`：每个共同计划日的 `date`、模拟成交使用的 `adjusted_close`、`scheduled_contribution_amount`、`core_invested_amount`、`opportunity_invested_amount`、`unallocated_amount`、`transaction_cost`、包含成本在内的模拟买入现金支出 `invested_amount`、`budget_utilisation_percent` 与 `strategy_rule_matched`。最后一个字段只在 Formula 的某条规则实际命中并改变机会桶决策时为 `true`；Fixed DCA 与沿用标准机会额度的 Formula 周期均为 `false`。这些点是历史规则执行结果，不声称是最佳买点，也不会提交订单；核心、机会和未投入金额之和等于本期外部投入，模拟买入现金支出中的交易成本不转换为资产单位；
- `result.series[].drawdown_points[]`：从同一条时间加权净值轨迹逐日计算的峰值相对回撤。历史新高为 `0`，回撤期为负百分比；
- `result.series[].calculation_details`：专业指标的可审计中间量与假设，包括区间日数、日收益样本数、平均日收益、样本标准差、下行偏差、最大回撤峰值/低点/恢复日期、累计模拟成本、规则命中/标准执行次数，以及 `252` 个交易日、`365.25` 个日历日和 `5 bps` 成本常量。

专业指标统一从本响应内的同一条轨迹与资金账本计算：

- 区间收益：`NAV_end / NAV_start - 1`；`normalized_points` 是剔除外部现金流影响后、共同起点归一为 `100` 的时间加权净值，不是股价或账户金额；
- 年化收益：`(NAV_end / NAV_start)^(365.25 / elapsed_days) - 1`；
- XIRR：将各次外部投入作为负现金流、期末总资产作为正现金流，求解 `Σ CF_i / (1+r)^(days_i/365.25) = 0`；
- 最大回撤：逐日计算 `(当日净值 / 历史峰值) - 1` 后取绝对幅度最大的下降；接口指标返回正的损失幅度，`drawdown_points` 用负值展示路径；
- 年化波动：日时间加权收益的样本标准差乘 `sqrt(252)`；
- Sortino：平均日收益除以零目标收益下的日度下行偏差，再乘 `sqrt(252)`；
- 现金使用率：累计模拟买入金额除以累计外部投入。

前端“专业研究”只呈现这些 API 值及其代入过程，不在浏览器重新推导一套结论。更换标的、区间、策略版本、数据修订或回测假设都会改变结果；短区间年化值尤其可能被放大。

Fixed DCA 与 Formula 使用同一份数据和现金流。Formula 在模拟成交日只能读取此前已完成的收盘价；接口为滚动指标额外请求预热数据，但归一化图表、`market_points` 和所有 `execution_points` 仍从用户请求的显示区间和所有策略共同有效日开始。`all` 表示当前 provider 在安全请求边界内返回的全部可用历史，不承诺供应商上市前数据或已退市证券连续性。

非法 JSON、范围、symbol、金额、日期、重复/过多/未知策略以及历史不足返回既有 `400 bad_request`；未配置历史行情 provider、供应商认证/限流/网络故障、本地快照冲突或损坏、内部确定性计算失败返回既有 `503 service_unavailable`。响应不包含 provider 凭据、账户信息或底层错误正文。

#### `GET /strategies`

列出本机 SQLite 中已保存的不可变 DSL 策略版本，按创建时间倒序排列。每个响应包含 `policy`、`name`、经过领域校验的 `document` 与 UTC `created_at`。服务端读取 `document` 后会重新通过 DSL 构造器校验；损坏或不一致的本地数据不会返回给客户端，而是统一返回 `503 service_unavailable`。

#### `POST /strategies/validate` 与 `POST /strategies`

策略工坊先将表单文档发送到 `POST /strategies/validate`；响应会返回 `valid`、可读校验错误或规范化文档。校验通过后才可 `POST /strategies` 保存为不可变版本。线上 Runtime 支持收盘价、周期收益率、年化历史波动率、价格分位、均线距离、SMA、EMA、RSI、回撤与 VIX；它按策略最长窗口请求行情并与固定样本研究复用同一因果证据构造器。每次运行保存 `as_of`、本机 OpenD 日线/Cboe VIX 来源及所用窗口。没有自由代码、任意脚本或核心桶否决。官方目录占用的 policy ID/version 不允许由本机自定义策略覆盖，冲突返回 `409 conflict`。

#### `POST /strategies/copilot-draft`

读取一个用户目标并生成**候选**受限策略文档；它不是策略保存、策略激活、回测、审计或下单入口。请求只接受已部署且在 `GET /ai/providers` 中声明 `restricted_policy_drafts: true` 的 profile。`qwen-default` 是示例 profile；任何 OpenAI-compatible provider 都必须由服务端通过无密钥 `AI_PROVIDER_PROFILES` 清单显式部署后才可被选择。

```json
{
  "profile_id": "qwen-default",
  "policy_id": "dsl_copilot_rsi_guard",
  "policy_version": 1,
  "objective": "Only increase the opportunity bucket when RSI is oversold."
}
```

服务端把“策略工坊 V1”表单契约和用户目标一同发送给模型。模型只返回 `form_config`、简短 `explanation` 与最多五条 `warnings`，不能编写 policy ID/version、证据 ID 或 DSL 表达式树。`form_config` 最多三条规则、每条最多三个条件，只允许价格涨跌幅、年化波动率、价格分位、均线距离、RSI、回撤和收盘价，以及跳过、50%、100%、120% 四档机会额度。服务端随后插入请求中的 policy ID/version，把用户可读的百分数转换为领域小数，确定性编译为 `StrategySpecDocument`，并通过既有领域构造器重新校验。响应仍只公开规范化 `document`、解释、警告、无密钥 `provider` 和服务端可信 `evidence`。

未知 profile、非 `dsl_` ID 或无能力 profile 返回安全 `400`；供应商超时、网络或 HTTP 故障返回 `503 service_unavailable`；模型已回复但外层 JSON 不符合约定返回 `502 ai_response_invalid`；JSON 可读但表单中的指标、窗口、阈值或额度不符合策略工坊 V1 返回 `502 ai_draft_invalid`。错误响应不包含模型原文、凭据、endpoint 或供应商内部细节。

该接口绝不会写 SQLite、创建 decision record、执行固定样本准入、绑定计划、激活策略或提交 paper order。用户仍必须将返回文档提交给 `POST /strategies/validate`，通过 admission，并显式保存/激活。

#### `POST /strategy-backtests/explain`

仅在用户点击“解释这次结果”后调用。请求包含一个已部署 `profile_id` 和完整 `backtest` 请求；服务端会重新运行 `POST /strategy-backtests` 的同一真实行情、策略版本与成本计算，再把服务端生成的结果事实交给所选 AI。浏览器不能提交自定义事实或自由 prompt。

响应包含无密钥 `provider`、真实行情 `source_checksum` 与有界 `explanation`（标题、普通语言摘要、最多五条观察和最多五条限制）。该接口不缓存或保存解释，不改动策略/计划，不创建建议，也不提交订单。回测或 AI 任一失败均故障关闭，绝不以演示数据或模型自行计算的收益替代。

#### `POST /personal/ai-summary`

仅在个人中心点击“手动生成摘要”后调用，请求只接受 `profile_id`。服务端从本机读取计划和最近最多 20 条决策记录，并附带是否已有用户手工确认；不会发送密钥、账户凭据、新闻正文或券商数据。响应为一次性普通语言摘要，不落库、不创建待办、不改变执行状态，也不会由 scheduler 调用。

以上两个解释接口只接受声明 `read_only_explanations: true` 的 profile。`AI_PROVIDER_PROFILES` 支持 `openai_chat_completions`（Qwen、DeepSeek 和兼容服务）、`openai_responses`（GPT）及 `anthropic_messages`（Claude）。密钥可由各 profile 的 `api_key_env` 指向本地服务端环境变量，也可经高级实验室注册为仅存当前 Rust 进程的会话 profile；任何 API 响应都不包含密钥或 endpoint。

#### `POST /strategies/:policy_id/:policy_version/simulate`

以一个计划标的的当前只读市场数据模拟已保存策略，返回截至日期、首条命中规则、机会桶倍率及指标证据；不写入审计、不提交订单。Studio 用它解释“当前为何命中/未命中”。

#### `GET /strategies/:policy_id/:policy_version/admission`

对已保存的 DSL 版本运行**激活前固定样本准入评估**；不会保存、绑定、审计或下单。响应包含：

- `core_bucket_safe`：规则是否只能影响机会桶；
- `budget_safe`：动作是否满足固定样本的周期预算上限；
- `eligible` 与安全的拒绝原因；
- 每个已覆盖标的在**相同外部现金流、成本、决策/成交时点**下的策略与 `Fixed DCA` 对照：期末净值、最大回撤、年化波动率与现金使用率。

当前不可变 `technical-v1` 已为 Close、周期收益率、年化历史波动率、价格分位、均线距离、SMA、EMA、RSI、Drawdown 与 VIX 提供因果历史证据；策略引用的每个指标都必须在各决策日具有足够预热，否则 admission 明确拒绝激活。服务端不会以合成输入伪造回测。

#### `POST /investment-plans/:id/activate-policy`

用户确认后将已保存、可由线上 evidence profile 支持、且已通过固定样本准入评估的策略版本绑定到计划。随后自动 Decision Preview、scheduler 与决策审计都解析同一 `policy_id@version`；审批模式仍只生成建议和审计，不会自动提交 paper order。

#### `GET /strategies/:policy_id/:policy_version`

读取一个精确不可变版本，例如：

```text
/strategies/dsl_rsi_opportunity_guard/1
```

非法策略标识/版本返回 `400 bad_request`，不存在的合法版本返回 `404 not_found`。两条官方 Formula V1 DSL 可通过本路由只读检查，不能覆盖；内置 `fixed_dca` 与旧 `core_opportunity_v1` 不是 DSL 文档，因此本路由不会把它们伪装成可编辑策略。

### Execution Preview + 双桶

#### `POST /investment-plans/:id/execution-preview`

预览计划在指定月内日期是否执行，并在 due 时根据**计划已持久化的配置**返回基准双桶拆分。

请求：

```json
{
  "day_of_month": 15
}
```

响应示例：

```json
{
  "plan_id": "00000000-0000-0000-0000-000000000001",
  "symbol": "VOO",
  "currency": "USD",
  "schedule_kind": "monthly",
  "schedule_day": 15,
  "day_of_month": 15,
  "status": "due",
  "planned_contribution": "1000.00",
  "bucket_split": {
    "planned_contribution": "1000.00",
    "core_contribution": "800.00",
    "opportunity_budget": "200.00",
    "opportunity_multiplier": "1",
    "opportunity_contribution": "200.00",
    "unallocated_opportunity_contribution": "0.00",
    "recommended_contribution": "1000.00",
    "opportunity_cash_policy": "carry_forward",
    "requires_approval": true
  }
}
```

`status` 可选值：

- `due`
- `waiting`
- `inactive`

校验规则：

- `day_of_month` 范围为 `1..=31`。
- 双桶比例与风险模式只可通过计划创建/更新接口修改，不能由预览请求覆盖。
- `unallocated_opportunity_contribution` 只是本次未建议投入的机会预算，不是现金池余额。

### Decision Preview + Paper Broker

#### `POST /investment-plans/:id/automatic-decision-preview`

Dashboard 与最小 Scheduler 使用的默认入口。请求体**不接受**人工填写的 fundamental 或 trend 字段。Fixed DCA 不读取任何行情；Formula 只通过 `HistoricalPriceProvider` 读取公式声明所需的规范化历史日线，并把 provider、数据版本、checksum、复权方式和证据截止日写入审计快照。只有旧 `CoreOpportunityV1` 兼容策略读取 OpenD/CAPE/国债/VIX 整包并计算 70/20；服务器默认 AI Evidence profile 的有界情绪分数也只会为该旧策略兼容映射为 10% 输入。双桶比例始终读取已持久化计划配置；请求体只允许经操作者确认的 `paper_order`：

```json
{
  "paper_order": {
    "idempotency_key": "operator-confirmed-key",
    "side": "buy",
    "order_type": "market",
    "quantity": "1.00"
  }
}
```

服务端使用当前 UTC 月内日期。策略实际需要的数据源不可用时返回统一 `503 service_unavailable`，不创建 `waiting`、伪造决策或审计记录；当前需要独立 VIX 的自定义 Formula 也会明确失败，不会用零值冒充证据。旧策略的 Qwen 不可用时仍创建记录并明确标记 `sentiment_unavailable` / `90/10/0`。响应新增 `audit_record_id`，可用 `GET /decisions/:id` 读取可读证据。省略 `paper_order` 时绝不下单。若本次自动预览在计划日成功保存为 `due`，它会同时占用相同的 `(plan_id, scheduled_for)` 调度标记，防止后台 Scheduler 在服务重启或下一 tick 为同一计划日重复生成建议。

server 默认启用周期 Scheduler：每 `SCHEDULER_TICK_SECONDS`（默认 60）秒检查一次，按每个 active plan 的 `monthly`/`weekly` `schedule_days` 与 UTC 日历创建自动决策存证。SQLite 的 `(plan_id, scheduled_for)` claim 阻止重启或下一 tick 重复存证；重启时仅补跑当前月或当前周尚未 claim 的日期。补跑不自动下单，且使用恢复时可用的数据生成存证；`approval` 计划仍须用户确认。

#### `POST /investment-plans/:id/decision-preview`

当前最适合前端演示主链路的接口。它会串联：

```text
investment plan
-> execution preview
-> bucket split
-> generic AI Evidence (legacy policy compatibility only)
-> selected policy runtime
-> optional configured paper order
-> local decision record
-> summary
```

请求：

```json
{
  "day_of_month": 15,
  "fundamental": {
    "score": 0.10,
    "cape_percentile": 0.10,
    "erp_percentile": 0.90
  },
  "trend": {
    "score": 0.50,
    "ma_distance_percentile": 0.50,
    "rsi_percentile": 0.50,
    "vix_percentile": 0.50,
    "regime": "neutral"
  },
  "paper_order": {
    "idempotency_key": "decision-preview-demo-1",
    "side": "buy",
    "order_type": "market",
    "quantity": "1.00"
  }
}
```

响应包含：

- `execution`：执行预览与持久化双桶配置产生的核心金额、机会预算、实际建议金额、未分配机会预算、滚存意图和审批要求。
- `decision`：`final_score`、`multiplier`、`action`、`weight_mode`、分层 score，以及相对同一计划上一份存证的只读 `change_from_previous`（首份、未变化或变化；包含可比较的动作/倍率/分数差异）。
- `market_sentiment`：为 HTTP 兼容保留的 AI Evidence 字段，包含无密钥 `provider` profile、`score`、受长度限制的 `rationale`、最多五条 `warnings`，以及实际送入模型的 RSS `headlines`（标题、链接、UTC 发布时间）。Provider 不负责生成来源链接。
- `ai_audit`：本次 AI 处理的安全调用追踪。成功时包含 `generated_at`、端到端 `latency_ms` 与服务端 `prompt_version`；旧策略安全降级时只返回分类后的 `reason`（`not_configured`、`news_unavailable`、`provider_timeout`、`provider_rejected`、`provider_response_invalid` 或 `provider_unavailable`）和 `observed_at`，不返回 endpoint、凭据或底层报错。
- `paper_order_ack`：只有 due 且 action 可执行时才出现。
- `summary`：演示级摘要。

`sentiment` 不是请求字段。后端会为旧 `CoreOpportunityV1` 自动拉取 CNBC RSS 并调用服务器默认的已部署 AI Evidence provider；成功时将分数、依据、风险提示、新闻来源、无密钥 Provider profile 与调用追踪写入本地 decision record。未配置 Key 或新闻/AI provider 暂时不可用时，旧引擎使用 `90/10/0` 降级权重，并保存不含伪造分数的安全降级原因。Fixed DCA 与 DSL 不会因 AI 不可用而改变推荐。手工 `sentiment` 字段会返回 `400 bad_request`，不能绕过该链路。

`decision.action` 可选值：

- `overweight`
- `standard`
- `tactical_delay`
- `underweight`
- `skip`

`decision.weight_mode` 可选值：

- `normal`
- `sentiment_unavailable`

`trend.regime` 请求值：

- `neutral`
- `overheated`
- `falling_knife`

`paper_order` 规则：

- `paper_order` 可省略；省略时只做 preview，不提交订单。
- 只有 `execution.status == "due"` 且 action 不是 `skip` / `tactical_delay` 时才提交配置的 paper order。
- 即使不会提交订单，只要请求中带了非法 `paper_order`，也会返回 `400 bad_request`。
- broker port 调用有 5 秒超时保护。

### Decision Record / History

#### `GET /investment-plans/:id/decisions`

列出一个已存在投资计划的历史 decision record，按 `created_at DESC, id DESC` 返回。

- `limit` 可选，默认 `50`，有效范围为 `1..=200`。
- 非法 plan UUID、非法 query 参数或越界 `limit` 返回 `400 bad_request`。
- 不存在的 investment plan 返回 `404 not_found`。
- Decision Preview 会自动创建本地审计记录；只读 history API 返回这些已持久化快照。可提交的 paper order 会先存订单意图，收到 broker ack 后再补写回执，避免存储故障把已提交订单伪装成可安全重试。

请求示例：

```text
GET /investment-plans/00000000-0000-0000-0000-000000000001/decisions?limit=20
```

响应是 decision record 数组。每条记录包含 execution、fundamental、trend、可选 sentiment（含 AI 依据、风险提示和来源）、decision 与可选 broker 的快照，以及最终 summary 和创建时间。

#### `GET /decisions/:id`

按 ID 查询单条 decision record。不存在时返回 `404 not_found`。

#### `POST /decisions/:id/manual-executions`

向指定 decision record 追加一条用户自行报告的最终执行结果。该接口不会修改 decision record，也不会调用 broker、重新计算策略或把用户报告冒充为已验证成交。事件只允许追加，不提供更新或删除 API；SQLite 同时拒绝对仍有关联计划的事件做直接 `UPDATE` / `DELETE`。

只有 `execution_status == "due"` 的决策可以记录执行结果；`waiting` 或 `inactive` 决策返回 `400 bad_request`。同一 decision 最多只能有一个最终结果：已有任意事件后再次提交，即使使用新的 `event_id`，也返回 `409 conflict`。这使计划日确认保持一次性，同时保留原始 append-only 审计语义。

请求示例：

```json
{
  "event_id": "00000000-0000-0000-0000-000000000301",
  "outcome": "partial",
  "actual_amount": "750.00",
  "occurred_at": "2026-09-16T08:30:00+10:00",
  "note": "本次只完成了部分投入"
}
```

- `event_id` 由客户端生成且必须为非 nil UUID；网络重试必须复用同一个值。重复 ID 或同一 decision 的第二个结果均返回 `409 conflict`，不会追加第二条事件。
- `outcome` 只接受 `executed`、`skipped`、`partial`。
- `executed` 与 `partial` 必须提交正数 decimal 字符串 `actual_amount`；`skipped` 必须省略该字段。
- `partial` 为 API 与既有审计记录的兼容值；V2.1 普通用户界面不再提供新增“部分执行”的入口，只提供 `executed` 与 `skipped`。
- `occurred_at` 必须为带时区的 RFC 3339 时间，保存时规范化为 UTC。
- `note` 可省略；提供时去除首尾空白，长度为 `1..=500`。
- 返回事件中的 `plan_id` 与 `currency` 从不可变 decision record 继承，调用方不能覆盖；`source` 固定为 `user_reported`。

成功返回 `201 Created`：

```json
{
  "id": "00000000-0000-0000-0000-000000000301",
  "decision_record_id": "00000000-0000-0000-0000-000000000001",
  "plan_id": "00000000-0000-0000-0000-000000000002",
  "outcome": "partial",
  "actual_amount": "750.00",
  "currency": "USD",
  "note": "本次只完成了部分投入",
  "occurred_at": "2026-09-15T22:30:00Z",
  "recorded_at": "2026-09-16T00:00:00Z",
  "source": "user_reported"
}
```

#### `GET /decisions/:id/manual-executions`

按 `recorded_at ASC, id ASC` 返回指定 decision record 的手工执行事件。不存在的 decision record 返回 `404 not_found`。返回数组为空表示该决策尚未收到用户执行反馈，不能据此推断已经执行或跳过。新数据库约束下数组最多包含一条；迁移前已经存在的多条历史仍按原样只读返回，不会被删除或改写。

#### `POST /decisions/:id/approve-paper-order`

仅允许对已持久化且状态为 `due` 的 `approval` 模式 decision record 进行一次人工确认模拟下单。请求体只接受非空 `idempotency_key`；服务端从该记录的不可变双桶快照读取推荐金额，再以本机最新可信价格换算整股数量，**不会重新运行 70/20/10、Qwen 或接受调用方自填金额/数量**。

订单意图会先原子写入该 decision record，再调用 paper-only broker；重复确认会返回 `400 bad_request`，避免同一存证重复下单。网络超时返回 `409 order_outcome_unknown`，客户端不得自动重试。

## AI Evidence API

### 已部署 Provider Profile

#### `GET /ai/providers`

返回环境部署及当前进程内存中可由用户选择的 AI profile 列表。每项只包含 `id`、`provider`、显示名、模型名与无授权能力声明；不会返回 Key、base URL、账户、secret manager 引用或内部错误。生产 composition root 支持遗留 `DASHSCOPE_*` 单 Qwen 配置，也支持 `AI_PROVIDER_PROFILES` 多 Profile 清单；高级实验室还可注册一个 `session-*` 会话 profile。

`restricted_policy_drafts: true` 只表示该 profile 可以生成**只读**、受限的 DSL 候选；它不授予保存、激活、准入或下单权限。候选仍须由用户显式执行校验、固定样本准入、保存与激活流程。

#### `POST /ai/session-provider`

从本机高级实验室注册一个进程内存 AI profile。请求只接受 `qwen_cloud`、`qwen`、`gpt`、`claude`、`deepseek`，以及非空 `model` 和 `api_key`；每个服务商的 HTTPS endpoint 与协议由服务端固定映射，浏览器不能提交任意 URL。保存配置本身不会探测或调用模型。

- `qwen_cloud`：QwenCloud Pay-As-You-Go，固定 `https://maas.qwencloudapi.com/compatible-mode/v1`，页面默认模型 `qwen3.8-max`，用于 `sk-ws-` Key。
- `qwen`：阿里云百炼 / DashScope，固定 `https://dashscope.aliyuncs.com/compatible-mode`，页面默认模型 `qwen-plus`。两类 Qwen Key 不互换。

```json
{
  "provider": "qwen_cloud",
  "model": "qwen3.8-max",
  "api_key": "<user-provided-key>"
}
```

响应只返回无凭据 `provider` 元数据和 `storage: "process_memory"`。API Key 仅保存在当前 Rust 进程的客户端实例中，不写浏览器存储、SQLite、`.env`、日志或响应；重新配置会覆盖前一条会话 profile，后端重启会自动清除。QwenCloud 会话使用 90 秒请求超时，其余会话 provider 使用 30 秒；这是为了覆盖 QwenCloud 首次响应可能接近 30 秒的情况，不改变模型权限或重试策略。真正的草案、解释、摘要或新闻情绪调用仍须用户在对应页面明确点击。

#### `POST /ai/session-provider/test`

对当前进程内存中的会话 profile 发起一次由用户明确点击的最小文本请求，用于同时验证 API Key、模型访问权和网络连通性。请求不接受 body，不保存探测提示词或模型响应，也不会创建策略、计划、行情记录或订单；供应商可能按其计费规则计算极少量 token。未配置会话 profile 时返回 `400 bad_request`。

成功到达供应商后，接口始终只返回无凭据 provider 元数据与一个安全分类，不回传上游正文或错误详情：

```json
{
  "provider": {
    "id": "session-qwen-cloud",
    "provider": "qwen-cloud",
    "display_name": "QwenCloud（本次运行）",
    "model": "qwen3.8-max",
    "capabilities": {
      "market_evidence": true,
      "restricted_policy_drafts": true,
      "read_only_explanations": true
    }
  },
  "status": "available"
}
```

`status` 为 `available`、`authentication_failed`、`access_denied`、`model_unavailable`、`rate_limited`、`request_rejected`、`network_unavailable`、`provider_unavailable` 或 `response_invalid`。前端据此给出 Key、计费、模型、限流或网络的下一步检查，不向浏览器泄露供应商原始错误正文。

#### `DELETE /ai/session-provider`

立即清除当前进程内存中的前端会话 profile，成功返回 `204 No Content`。环境变量部署的 profile 不受影响。

### 阿里云 Qwen Evidence Profile

#### `POST /market-sentiment/preview`

后端拉取 CNBC RSS 新闻并调用所选、已部署的 DashScope/OpenAI-compatible Evidence provider，返回有界情绪值及受控解释。设置 `DASHSCOPE_API_KEY` 后 server 在启动时构造并注册 Qwen profile；未设置 Key 时 server 仍可启动，但本路由返回统一的 `503 service_unavailable`，不暴露 provider URL 或凭据细节。可选查询参数 `profile_id` 必须匹配 `GET /ai/providers` 返回的 profile；缺省时使用服务器默认 profile，未知或格式不安全的 ID 返回统一 `400 bad_request`。

AI Evidence 独立于策略 Registry：`CoreOpportunityV1` 才会将其 `score` 映射为旧 10% 情绪输入；Fixed DCA 与 DSL Runtime 只可展示或审计该证据，不会让 AI 改写策略推荐。

响应字段：

- `provider`：无密钥的已部署 profile（`id`、provider、显示名、模型和能力声明）。
- `score`：`[-1.0, 1.0]` 内的情绪分数。
- `label`：`positive`、`neutral` 或 `negative`，由分数正负确定。
- `rationale`：Qwen 基于本次输入 headlines 给出的短依据；空白、过长或非结构化输出会被拒绝并触发安全降级。
- `warnings`：最多五条短风险提示。
- `headlines`：实际送入模型的 RSS 条目，含 `title`、HTTP(S) `url` 和 UTC `published_at`；不保存新闻正文。
- `generated_at`、`latency_ms`、`prompt_version`：服务端接受模型输出的 UTC 时间、新闻加模型的端到端耗时与版本化提示词契约；这些字段支持审计与排障，不是交易信号。

不会返回新闻正文、Key、provider URL 或模型内部错误。模型不能自行提供 URL；来源只能由 RSS 原始条目生成，避免将幻觉来源写入 API 或审计记录。

本地真实 Key smoke（不要把 Key 写入仓库或终端输出）：

```bash
read -r -s DASHSCOPE_API_KEY
export DASHSCOPE_API_KEY
cargo test -p ai-client --test news real_cnbc_with_qwen -- --ignored --nocapture
```

HTTP smoke：在同一终端环境启动 `cargo run -p indexlink-server` 后，执行：

```bash
curl -X POST 'http://127.0.0.1:8080/market-sentiment/preview?profile_id=qwen-default'
```

## Quant Signal APIs

### Automatic Market Signal Input API

#### `GET /signals/market-input/:symbol`

读取并组装当前计划标的的自动信号输入，供 Dashboard 填充既有 Fundamental/Trend Preview 表单；该端点只读，不会创建订单、不会访问交易账户，也不会保存密钥。server 必须已配置本机 loopback OpenD；未配置或任一数据源不可用时，统一返回安全的 `503 service_unavailable`。

- 价格与技术层：本机 OpenD 的美股日线，后端本地计算 MA200 distance 与 14 日 RSI，并按月保留最近 60 个快照。
- 基本面层：公开 Shiller CAPE 月度表；ERP 明确使用代理口径 `100 / CAPE - 美国财政部 10 年期国债收益率`，不是前瞻盈利预测。
- 波动层：Cboe 公开 VIX 历史 CSV，按每月最后一个可用观测值保留最近 60 个快照。

响应含 `fundamental`、`trend`、`as_of` 与来源说明。页面只在用户点击醒目的“自动拉取市场信号”按钮后请求；返回值仍展示在可编辑字段中，用户可在运行 Decision Preview 前审查。实际执行时，同一输入最终会随 Decision Record 保存到本地 SQLite 审计快照。

### Fundamental Signal API

#### `POST /signals/fundamental/preview`

用调用方提供的月度 CAPE/ERP 历史快照计算 70% 基本面层信号。历史数组须按旧到新排列，默认至少 `60` 个有效月度样本；不满足领域校验或出现未知字段时返回统一 `400 bad_request`。

请求字段：`cape_history`、`cape_current`、`erp_history`、`erp_current`。

响应字段：

- `score`：基本面综合分数，`0` 表示历史相对便宜、`1` 表示历史相对昂贵。
- `cape_percentile`、`erp_percentile`：用于 decision record 和演示解释的原始审计分位。

### Trend Signal API

#### `POST /signals/trend/preview`

用调用方提供的月度 MA200 distance、RSI、VIX 历史快照计算 20% 趋势层信号。历史数组须按旧到新排列，默认至少 `60` 个有效月度样本；不满足领域校验或出现未知字段时返回统一 `400 bad_request`。

请求字段：`ma_distance_history`、`ma_distance_current`、`rsi_history`、`rsi_current`、`vix_history`、`vix_current`。

响应字段：

- `score`：趋势综合分数。
- `ma_distance_percentile`、`rsi_percentile`、`vix_percentile`：原始审计分位。
- `regime`：`neutral`、`overheated` 或 `falling_knife`。

这两个端点不保存调用方请求本身；无论来自手工输入、JSON 导入还是自动市场快照，只有最终提交的 Decision Preview 输入会作为审计记录保存到本地 SQLite。

### Futu/Moomoo OpenD Paper Trading API

已具备 broker port、测试专用 MockBroker、OpenD raw TCP paper session 与下单 adapter。生产 server 未配置 broker 时不会安装 Mock；设置 `OPEND_PROVIDER` 后，可通过 `OPEND_MARKET_DATA_ENABLED` 与 `OPEND_PAPER_BROKER_ENABLED` 独立装配只读行情和模拟 broker。任一 adapter 初始化失败只会将对应 capability 标记为 `unavailable`，不会阻止 SQLite、Plan、Decision、Audit 或 HTTP server 启动，也绝不会静默降级到 Mock。

真实 OpenD 下单暂不需要单独 HTTP endpoint；它复用 `POST /investment-plans/:id/decision-preview` 的 `paper_order`，以确保订单必须经过计划、执行日和决策保护。

#### `GET /paper-portfolio`

读取当前已配置 OpenD 模拟账户的 USD 资金、当前美股持仓和近期美股订单状态，供 Dashboard 展示账户净资产、现金、证券市值、持仓盈亏、持仓与订单。它只调用 OpenD 的资金、持仓和订单读取协议；不会下单、撤单、改价、解锁交易，也不会返回 account id、登录凭据或 provider 原始错误。

请求需要已配置并成功初始化 `OPEND_PROVIDER`。未配置、OpenD 不可用、返回账户/环境不匹配或响应不完整时统一返回 `503 service_unavailable`。路由使用 OpenD 的强制缓存刷新读取最新状态，应只由用户点击“刷新模拟账户”触发，而不是高频轮询。

OpenD 模拟账户不支持独立成交列表与现金流记录；IndexLink 因此不把 `accepted` 伪装为成交，而是在本地账本中根据后续订单的累计成交数量、累计均价和订单状态增量生成可审计的本地 fill。该来源会在 Dashboard 明确标记，且只覆盖账本启用后由 IndexLink 接受并持续观察的订单。

#### `PUT /investment-plans/:id/paper-performance/opening-balance`

保存用户确认的本地模拟账户起始资金基准。请求体：

```json
{
  "amount": "10000.00",
  "occurred_at": "2026-07-19T10:00:00.000Z"
}
```

金额必须为非负 decimal 字符串；时间必须为 UTC RFC3339 毫秒格式。它只写入本机 SQLite 的 `cash_flows`，不会访问 OpenD、下单或修改模拟账户资金。每个 plan 仅保留一条可覆盖的 `opening_balance`；没有该基准时服务不会声称计算了总收益。

#### `GET /investment-plans/:id/paper-performance`

只读刷新 OpenD 资金、持仓和近期订单状态，然后将已知订单的成交增量、FIFO 持仓成本和本次估值写入本机 SQLite。响应包含：

- `net_contributions`、`realized_pnl`、`unrealized_pnl`、`total_return`；
- `adaptive_value` 与同一执行价下的 `plain_dca_value`；
- 用于 Dashboard 曲线的本地 `points`；
- `data_complete`，只有本地观察到的 FIFO 数量与当前 provider 持仓数量一致且已配置起始基准时才为 `true`。

普通定投基准为每个已观察到的买入订单，在该订单首次成交价按计划 `base_contribution` 买入的假想仓位；它不使用未来价格，也不伪造未观察到的历史成交。当前限制仍然是：历史数据从本地账本启用后开始积累，不能反推出启用前的完整模拟账户交易/入金历史；多计划共享同一模拟账户时，需分别持续追踪各计划发出的订单，不能把整个账户余额随意归因给某一计划。

#### `GET /paper-performance/actual`

一次只读 OpenD 模拟账户并刷新所有启用定投标的的本机快照。响应返回每个标的的 `series` 与 `total_points` 总和线；同日多次刷新会按每个标的保留最新快照，避免把重复状态相加。没有计划或没有本地成交时返回空序列，不会伪造收益。

#### `GET /market-data/holdings?period=3m|6m|1y|3y`

读取所有启用定投标的的 OpenD 日线收盘价，以及在所选窗口内本机账本已确认的 `trades`。多标的图应在前端按区间首日归一化，才可在同一坐标系比较；绿色/红色买卖标记只可使用本地确认 fill，不能用 accepted order 代替。

#### `POST /market-data/session-opend`

从高级实验室注册当前 Rust 进程使用的只读 OpenD 行情与历史日线适配器。请求只接受字面 loopback IP 和有效端口，例如：

```json
{"host":"127.0.0.1","port":11111}
```

保存配置不会立即探测 OpenD；第一次行情、策略预检或真实回测请求才会连接。响应标明 `storage: "process_memory"` 与 `access: "read_only_market_data"`。该端点不接受 account ID，不创建 paper broker，也不授予模拟或真实下单权限；后端重启即清除。

#### `DELETE /market-data/session-opend`

清除进程内存中的 OpenD 行情覆盖，成功返回 `204 No Content`。若启动环境已配置 OpenD，清除后仍回到该启动配置，不影响 paper broker。

本地启用配置：

```bash
OPEND_PROVIDER=futu
OPEND_HOST=127.0.0.1
OPEND_PORT=11111
OPEND_ACCOUNT_ID='<paper-account-id>'
OPEND_MARKET_DATA_ENABLED=true
OPEND_PAPER_BROKER_ENABLED=true
```

- 配置仅接受 `futu` / `moomoo` 和 loopback host（`127.0.0.1`、`::1`、`localhost`）。
- server 配置层只构造 `Paper` adapter；没有 live environment 或 live gate 配置项。
- 两个 capability 开关未显式设置时保持旧配置兼容：存在 `OPEND_PROVIDER` 即默认同时启用；也可分别设为 `false`。未配置或初始化失败的 broker 路由统一返回 `503 service_unavailable`，不会生成 `MOCK-*` 回执。
- 真实 smoke 是忽略式测试，必须显式确认且提供唯一 idempotency key、symbol 与 quantity；它不读取、不传输 OpenD 登录密码或 token。

真实 smoke 前先在 OpenD GUI 中登录并确认选择的是虚拟账户；以下命令会提交一笔 paper market order，不应在 CI 中执行：

```bash
export OPEND_PROVIDER=futu
export OPEND_HOST=127.0.0.1
export OPEND_PORT=11111
read -r -p 'Paper account ID: ' OPEND_ACCOUNT_ID
export OPEND_ACCOUNT_ID
read -r -p 'Unique idempotency key: ' OPEND_SMOKE_IDEMPOTENCY_KEY
export OPEND_SMOKE_IDEMPOTENCY_KEY
read -r -p 'US symbol: ' OPEND_SMOKE_SYMBOL
export OPEND_SMOKE_SYMBOL
read -r -p 'Quantity: ' OPEND_SMOKE_QUANTITY
export OPEND_SMOKE_QUANTITY
OPEND_SMOKE_CONFIRM=submit-paper-order \
  cargo test -p indexlink-server real_opend_paper_order_smoke -- --ignored --nocapture
```

### Decision Preview 输入与摘要

当前 `POST /investment-plans/:id/decision-preview` 仍保留为兼容/测试入口，并由后端自动获取 Qwen market sentiment；fundamental 与 trend 仍由调用方传入。前端不再使用它填写 70/20，而应使用 `automatic-decision-preview`。两条入口都会将 execution、70/20 输入来源、可选 sentiment、decision 和 broker 快照写入本地 SQLite decision record。

`summary` 已按 execution、计划金额、基本面、趋势和 regime、Qwen 情绪/降级权重、最终分数、倍率/action、双桶建议金额和 paper-order 状态给出稳定分层解释。输入快照不得包含 Qwen API key、OpenD 密码、account id、token 或其他 secret。历史客户端携带的 decision-preview `bucket_allocation` 会被校验但不再覆盖计划配置；应迁移为通过计划创建/更新接口配置双桶。

## 前端当前建议对接顺序

1. `GET /health`、`GET /ready`
2. `POST /investment-plans`
3. `GET /investment-plans`
4. `GET /investment-plans/:id`
5. `PATCH /investment-plans/:id`
6. `POST /investment-plans/:id/execution-preview`
7. `POST /investment-plans/:id/decision-preview`
8. `POST /signals/fundamental/preview`
9. `POST /signals/trend/preview`
10. `GET /investment-plans/:id/decisions`
11. `GET /decisions/:id`
12. `POST /decisions/:id/manual-executions`
13. `GET /decisions/:id/manual-executions`
14. `POST /decisions/:id/approve-paper-order`
15. `GET /paper-performance/actual`
16. `GET /market-data/holdings?period=1y`
17. `GET /ai/providers`
18. `POST/DELETE /ai/session-provider`
19. `POST /ai/session-provider/test`
20. `POST/DELETE /market-data/session-opend`
21. `GET /strategies`
22. `GET /strategies/:policy_id/:policy_version`
23. `GET /strategies/:policy_id/:policy_version/admission`

## 当前 V2.1 边界与后续优先级

1. API 默认只服务本机 loopback，当前没有账户认证；不得直接暴露到局域网或公网。
2. 使用真实 Futu/Moomoo 虚拟账户完成一次人工确认的 OpenD paper-order smoke；这不会改变 scheduler 禁止自动下单的边界。
3. 将资金周期与 Formula 观察频率拆分，并引入交易所日历、时区、节假日规则和周期级幂等。
4. 继续扩展 Formula 的跨版本研究与人工审核体验，但不引入任意用户代码。
5. 历史 70/20/10 需要完整版本化 CAPE、ERP、趋势、VIX 与新闻证据才能重新研究；当前不会伪造缺失输入，也不会把它放回普通策略目录。
