# IndexLink API 管理清单

本文档用于前后端对接和 MVP 范围管理，记录当前已经可用的 HTTP API、请求/响应约定，以及后续仍需补充的接口。

## 通用约定

- 默认请求和响应均为 JSON。
- 金额、比例、数量等 decimal 字段在 JSON 中使用字符串，例如 `"1000.00"`、`"0.80"`、`"1.00"`。
- UUID 路径参数非法时返回 `400 bad_request`。
- 资源不存在时返回 `404 not_found`。
- 已发送订单但未收到可信回执时返回 `409 order_outcome_unknown`；客户端不得自动重试。
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

响应：创建后的 investment plan。服务端会规范化 `symbol` 与 `currency` 为大写。

`bucket_allocation` 的比例使用 `0..=1` 的 decimal 字符串（例如 `0.80` 即 `80%`），两桶必须恰好合计 `1`。当核心桶为 `100%` 时，`risk_mode` 必须为 `fixed`，且 `opportunity_cash_policy` 只能为默认 `expire_each_period`；存在机会桶时必须显式选择 `autopilot` 或 `approval`。

`opportunity_cash_policy` 可为 `expire_each_period`、`carry_forward` 或 `carry_with_cap`。`carry_with_cap` 必须同时提交正的 `opportunity_cash_cap`（金额型上限）；期数型上限尚未实现。`period_execution_limit` 是同一周或月内自动 paper order 的累计金额上限；每笔自动订单先原子预留，broker 接受后确认，终态 `filled`/`closed` 的实际成交金额会回写修正占用。pending/partial 订单保守维持估计占用。legacy 手动 preview 保留显式数量兼容，尚不进入该自动金额账本。为兼容旧客户端，省略新增配置时默认 `100%` 核心桶、`fixed` 与 `expire_each_period`。

`schedule_kind` 接受 `monthly`（日期为 `1..=28`）或 `weekly`（ISO 星期为 `1..=7`）。`schedule_days` 可提供同一周期的多个固定日，必须有序、无重复，且其第一项必须等于兼容字段 `schedule_day`；省略时等价于仅 `[schedule_day]`。scheduler 使用此集合按 UTC 日期运行。

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

#### `GET /strategies`

列出本机 SQLite 中已保存的不可变 DSL 策略版本，按创建时间倒序排列。每个响应包含 `policy`、`name`、经过领域校验的 `document` 与 UTC `created_at`。服务端读取 `document` 后会重新通过 DSL 构造器校验；损坏或不一致的本地数据不会返回给客户端，而是统一返回 `503 service_unavailable`。

#### `POST /strategies/validate` 与 `POST /strategies`

Strategy Studio 先将表单文档发送到 `POST /strategies/validate`；响应会返回 `valid`、可读校验错误或规范化文档。校验通过后才可 `POST /strategies` 保存为不可变版本。线上 Runtime 支持收盘价、SMA、EMA、RSI、回撤与 VIX；每次运行保存 `as_of`、本机 OpenD 日线/Cboe VIX 来源及所用窗口。没有自由代码、任意脚本或核心桶否决。

#### `POST /strategies/copilot-draft`

读取一个用户目标并生成**候选**受限 DSL 文档；它不是策略保存、策略激活、回测、审计或下单入口。请求只接受已部署且在 `GET /ai/providers` 中声明 `restricted_policy_drafts: true` 的 profile。`qwen-default` 是示例 profile；任何 OpenAI-compatible provider 都必须由服务端通过无密钥 `AI_PROVIDER_PROFILES` 清单显式部署后才可被选择。

```json
{
  "profile_id": "qwen-default",
  "policy_id": "dsl_copilot_rsi_guard",
  "policy_version": 1,
  "objective": "Only increase the opportunity bucket when RSI is oversold."
}
```

服务端将模型原始 JSON 重新解析为 `StrategySpecDocument` 并通过既有领域构造器校验；模型输出的 policy ID/version 必须与请求完全一致，且只能使用 Close、SMA、EMA、RSI、Drawdown、VIX 与白名单机会桶动作。响应仅包含规范化 `document`、简短 `explanation`、最多五条 `warnings`、无密钥 `provider` 和从服务端封闭列表中选出的 `evidence` 引用。未知 profile、非 `dsl_` ID、无能力 profile 返回安全 `400`；模型返回非法 DSL、版本不一致或伪造证据引用时返回安全 `503`。

该接口绝不会写 SQLite、创建 decision record、执行固定样本准入、绑定计划、激活策略或提交 paper order。用户仍必须将返回文档提交给 `POST /strategies/validate`，通过 admission，并显式保存/激活。

#### `POST /strategies/:policy_id/:policy_version/simulate`

以一个计划标的的当前只读市场数据模拟已保存策略，返回截至日期、首条命中规则、机会桶倍率及指标证据；不写入审计、不提交订单。Studio 用它解释“当前为何命中/未命中”。

#### `GET /strategies/:policy_id/:policy_version/admission`

对已保存的 DSL 版本运行**激活前固定样本准入评估**；不会保存、绑定、审计或下单。响应包含：

- `core_bucket_safe`：规则是否只能影响机会桶；
- `budget_safe`：动作是否满足固定样本的周期预算上限；
- `eligible` 与安全的拒绝原因；
- 每个已覆盖标的在**相同外部现金流、成本、决策/成交时点**下的策略与 `Fixed DCA` 对照：期末净值、最大回撤、年化波动率与现金使用率。

当前不可变 `technical-v1` 已为 Close、SMA、EMA、RSI、Drawdown 与 VIX 提供因果历史证据；策略引用的每个指标都必须在各决策日具有足够预热，否则 admission 明确拒绝激活。服务端不会以合成输入伪造回测。

#### `POST /investment-plans/:id/activate-policy`

用户确认后将已保存、可由线上 evidence profile 支持、且已通过固定样本准入评估的策略版本绑定到计划。随后自动 Decision Preview、scheduler 与决策审计都解析同一 `policy_id@version`；审批模式仍只生成建议和审计，不会自动提交 paper order。

#### `GET /strategies/:policy_id/:policy_version`

读取一个精确不可变版本，例如：

```text
/strategies/dsl_rsi_opportunity_guard/1
```

非法策略标识/版本返回 `400 bad_request`，不存在的合法版本返回 `404 not_found`。内置 `fixed_dca` 与 `core_opportunity_v1` 不是 DSL 文档，因此本路由不会把它们伪装成可编辑策略。

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

Dashboard 与最小 Scheduler 使用的默认入口。请求体**不接受**人工填写的 fundamental 或 trend 字段；后端为计划标的读取 OpenD/CAPE/国债/VIX 输入并计算 70/20。仅当计划绑定旧 `CoreOpportunityV1` 时，服务器默认 AI Evidence profile 的有界情绪分数才会兼容映射为旧 10% 输入；Fixed DCA 与 DSL Runtime 不读取 AI Evidence。双桶比例始终读取已持久化计划配置；请求体只允许经操作者确认的 `paper_order`：

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

服务端使用当前 UTC 月内日期。70/20 市场源不可用时返回统一 `503 service_unavailable`，不创建伪造的决策或审计记录；Qwen 不可用时仍创建记录并明确标记 `sentiment_unavailable` / `90/10/0`。响应新增 `audit_record_id`，可用 `GET /decisions/:id` 读取可读证据。省略 `paper_order` 时绝不下单。

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
- `decision`：`final_score`、`multiplier`、`action`、`weight_mode` 和分层 score。
- `market_sentiment`：为 HTTP 兼容保留的 AI Evidence 字段，包含无密钥 `provider` profile、`score`、受长度限制的 `rationale`、最多五条 `warnings`，以及实际送入模型的 RSS `headlines`（标题、链接、UTC 发布时间）。Provider 不负责生成来源链接。
- `paper_order_ack`：只有 due 且 action 可执行时才出现。
- `summary`：演示级摘要。

`sentiment` 不是请求字段。后端会为旧 `CoreOpportunityV1` 自动拉取 CNBC RSS 并调用服务器默认的已部署 AI Evidence provider；成功时将分数、依据、风险提示、新闻来源和无密钥 Provider profile 写入本地 decision record。未配置 Key 或新闻/AI provider 暂时不可用时，旧引擎使用 `90/10/0` 降级权重且不提交伪造情绪快照。Fixed DCA 与 DSL 不会因 AI 不可用而改变推荐。手工 `sentiment` 字段会返回 `400 bad_request`，不能绕过该链路。

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

#### `POST /decisions/:id/approve-paper-order`

仅允许对已持久化且状态为 `due` 的 `approval` 模式 decision record 进行一次人工确认模拟下单。请求体只接受非空 `idempotency_key`；服务端从该记录的不可变双桶快照读取推荐金额，再以本机最新可信价格换算整股数量，**不会重新运行 70/20/10、Qwen 或接受调用方自填金额/数量**。

订单意图会先原子写入该 decision record，再调用 paper-only broker；重复确认会返回 `400 bad_request`，避免同一存证重复下单。网络超时返回 `409 order_outcome_unknown`，客户端不得自动重试。

## AI Evidence API

### 已部署 Provider Profile

#### `GET /ai/providers`

返回服务器实际部署、可由用户选择的 AI profile 列表。每项只包含 `id`、`provider`、显示名、模型名与无授权能力声明；不会返回 Key、base URL、账户、secret manager 引用或内部错误。生产 composition root 支持遗留 `DASHSCOPE_*` 单 Qwen 配置，也支持 `AI_PROVIDER_PROFILES` 多 Profile 清单；后者必须引用服务端已有的 Key 环境变量并恰有一个默认 profile。

`restricted_policy_drafts: true` 只表示该 profile 可以生成**只读**、受限的 DSL 候选；它不授予保存、激活、准入或下单权限。候选仍须由用户显式执行校验、固定样本准入、保存与激活流程。

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

已具备 broker port、MockBroker、OpenD raw TCP paper session 与下单 adapter。server 未设置 `OPEND_PROVIDER` 时保留 MockBroker；设置 `futu` 或 `moomoo` 后，server 在启动时连接本机 loopback OpenD 并注入真实 `OpenDPaperBroker`。启动失败会安全失败，绝不会静默降级到 mock broker。

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

#### `GET /paper-performance/historical-backtest`

返回最近一年的历史价格回放，两条曲线为固定月度普通定投与基于当月真实 MA200 距离、限制在 `0.5x–1.5x` 的自适应投入。它不会虚构不可审计的历史 Qwen 情绪或历史宏观快照，因此响应 `methodology` 会明确说明：该图是价格规则回放，不是已实现账户收益，也不是完整 70/20/10 决策重放。

本地启用配置：

```bash
OPEND_PROVIDER=futu
OPEND_HOST=127.0.0.1
OPEND_PORT=11111
OPEND_ACCOUNT_ID='<paper-account-id>'
```

- 配置仅接受 `futu` / `moomoo` 和 loopback host（`127.0.0.1`、`::1`、`localhost`）。
- server 配置层只构造 `Paper` adapter；没有 live environment 或 live gate 配置项。
- 未设置 `OPEND_PROVIDER` 时，演示继续使用 paper-only `MockBroker`。
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
12. `POST /decisions/:id/approve-paper-order`
13. `GET /paper-performance/actual`
14. `GET /market-data/holdings?period=1y`
14. `GET /paper-performance/historical-backtest`
15. `GET /strategies`
16. `GET /strategies/:policy_id/:policy_version`
17. `GET /strategies/:policy_id/:policy_version/admission`

## 当前 MVP 缺口优先级

1. 使用真实 DashScope Key 完成一次本机 Qwen network smoke。
2. 使用 Futu/Moomoo 虚拟账户完成一次真实 OpenD paper-order smoke。
3. 将按月归档的 CAPE、ERP、MA200 distance、RSI、VIX 与 Qwen 情绪输入纳入本地决策快照，才能把当前价格规则回放提升为可完整复现的历史 70/20/10 回测；当前不会伪造这些历史输入。
4. 扩展 DSL 的版本化历史夹具、跨版本比较和人工审核体验；首版受控创建、验证、固定样本准入、激活与 Strategy Studio 已完成。
