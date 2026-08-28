# IndexLink 策略工作台迁移计划 / Strategy Studio Migration Plan

> 状态：PR 1–6 与 PR 7a–7d 均已完成：不可变版本存储、策略校验/保存、Strategy Studio、当前数据模拟、固定样本准入与计划激活已接入统一 Runtime。下一项候选工作是受限 Qwen Strategy Copilot，或扩展版本化历史夹具以放开更多 DSL 指标的准入。
> Status: PRs 1–6 and PRs 7a–7d are complete: immutable version storage, policy validation/saving, Strategy Studio, current-data simulation, fixed-fixture admission, and plan activation now use the unified runtime. The next candidate is a bounded Qwen Strategy Copilot or expanded versioned historical fixtures for additional DSL indicators.

## 1. 新定位 / New Positioning

IndexLink 将从单一的“70/20/10 自适应定投模型”演进为一个**透明、可审计、可扩展的个人量化定投策略工作台与 paper-trading 执行平台**。

IndexLink will evolve from a single “70/20/10 adaptive DCA model” into a **transparent, auditable, extensible personal quantitative strategy studio and paper-trading execution platform**.

产品不承诺策略必然跑赢固定定投或市场。它的核心价值是让使用者能够以相同的市场数据、资金流、成本假设和执行时点，创建、验证、比较、激活、执行和回放策略。

The product does not promise that a policy will outperform fixed DCA or the market. Its value is enabling users to create, validate, compare, activate, execute, and replay policies under matched market data, cash flows, cost assumptions, and execution timing.

目标生命周期：

```text
创建策略 → 验证 → 回测 → 审阅 → 保存版本 → 激活 → 调度
→ 评估 → Paper 执行 → 监控 → 审计
```

Target lifecycle:

```text
Create Strategy → Validate → Backtest → Review → Save Version → Activate
→ Schedule → Evaluate → Paper Execute → Monitor → Audit
```

## 2. 迁移原则与非目标 / Migration Principles and Non-goals

1. **不删除历史研究。** 现有 70/20/10、C1–C4、校准数据和报告保留为可复现实验资产；它们不再构成收益承诺。
2. **确定性运行时优先。** 相同的策略版本、完整上下文和执行时点必须产生相同推荐；策略运行时不得发起网络请求、读取环境变量或直接下单。
3. **策略与基础设施解耦。** API、scheduler、broker、SQLite 和 OpenD 不应理解 CAPE、ERP、RSI、VIX、70/20/10 或 `TacticalDelay` 的内部含义。
4. **固定 DCA 是公平基准。** `FixedDcaPolicy` 是后续新计划的默认候选，并是所有策略研究的匹配对照；已有计划不会被静默改变。
5. **AI 不是交易授权者。** Qwen 只能生成策略候选、解释、风险提示和变化摘要；它不能改变已经验证的策略逻辑，不能绕过金额/环境/人工确认边界，也不能直接发单。
6. **限定表达能力。** 自定义策略仅允许受限 DSL/AST、白名单指标和白名单动作；不执行用户代码，不支持任意脚本、实盘自动交易或云端多用户同步。

## 3. 目标领域边界 / Target Domain Boundaries

### 3.1 新增稳定契约 / New Stable Contract

已新增无 IO 的策略领域 crate：`strategy-policy`。它定义：

| 类型 / Type | 用途 / Purpose |
| :--- | :--- |
| `PolicyId` | 已校验、稳定的策略标识。 |
| `PolicyVersion` | 不可变策略版本。 |
| `PolicyRef` | `id + version` 的激活绑定。 |
| `DecisionContext` | 已解析的执行日期、预算、计划约束、市场证据和 `as_of`；不得包含 IO。 |
| `InvestmentRecommendation` | 已实现策略引用、动作、倍率与周期预算；桶拆分、原因和风险提示继续由计划/应用层保留。 |
| `InvestmentPolicy` | `DecisionContext -> InvestmentRecommendation` 的确定性评估契约。 |

The implemented `strategy-policy` crate is pure and I/O-free. Its runtime contract is:

```text
PolicyRef + complete DecisionContext → InvestmentRecommendation
```

`DecisionContext` contains resolved evidence rather than a database connection, HTTP client, Qwen client, or broker. The same runtime must therefore power both historical evaluation and live preview.

### 3.2 内置策略 / Built-in Policies

| 策略 / Policy | 状态 / Status | 行为 / Behaviour |
| :--- | :--- | :--- |
| `CoreOpportunityV1` | 现有逻辑的兼容包装器 | 包装当前 70/20/10、双桶和动作语义，输出保持逐项回归兼容。 |
| `FixedDcaPolicy` | 首个新增策略 | 固定按周期预算推荐金额，用作新计划默认候选和公平基准。 |
| DSL 策略 | 已实现受控闭环 | 用户保存的受限规则策略；仅白名单指标/表达式/机会桶动作，必须通过固定样本准入后才能激活。 |

`CoreOpportunityV1` will call the existing legacy decision implementation unchanged during the first migration stage. No existing public `decision-engine` type is renamed or removed in that stage.

### 3.3 策略内部证据 / Internal Evidence

CAPE、ERP、MA、RSI、VIX、Qwen 情绪和 `TacticalDelay` 是 `CoreOpportunityV1` 的内部证据或标签，不是平台级 API/Broker 语义。DSL 初始仅支持价格、SMA、EMA、RSI、回撤和 VIX；动作仅可影响机会桶（固定金额、倍率或跳过），不能否决核心桶。

## 4. 当前耦合审计 / Current Coupling Audit

当前硬耦合集中在应用编排层，而非全部仓库：

| 位置 / Location | 当前职责 / Current coupling | 迁移方式 / Migration treatment |
| :--- | :--- | :--- |
| `crates/decision-engine` | 70/20/10 合成、倍率与 `TacticalDelay` | 首期冻结并由 `CoreOpportunityV1` 包装；后续不再作为唯一策略入口。 |
| `crates/api/src/routes/decision_preview.rs` | 直接构造旧输入、调用旧引擎、创建记录和可选订单 | 改为调用 policy resolver；保留旧 HTTP 字段直到版本化 API 完成。 |
| `crates/investment-plans` | 到期预览、预算与双桶金额语义 | 接收通用推荐金额/拆分；旧计划保留旧行为。 |
| `crates/decision-records` 与 SQLite adapter | 保存 70/20/10 输入快照与订单回执 | 追加策略 ID、版本、快照、哈希和通用证据；旧列与旧记录继续可读。 |
| `apps/server` scheduler | 调用固定决策入口 | 迁移为调用 policy resolver，同时保留幂等 claim 与“不自动下单”边界。 |
| `broker` / OpenD | paper-only 订单提交 | 保持不变；只接收已验证的订单请求。 |

## 5. 向后兼容与数据迁移 / Backward Compatibility and Data Migration

1. 已有计划在数据库迁移中显式回填为 `CoreOpportunityV1@1`，不会在升级后自动变为固定 DCA。
2. 新建计划在产品确认后才将 `FixedDcaPolicy@1` 作为默认候选；这一默认值变化必须有 API、UI 和迁移测试。
3. 旧 Decision Preview 响应暂时保留；新响应以附加 `policy` 与 `recommendation` 字段的方式演进，避免破坏前端。
4. `decision_records` 采用追加字段/迁移：`policy_id`、`policy_version`、`policy_snapshot`、`policy_hash`、`evidence_snapshot`、`recommendation_snapshot`。既有 70/20/10 快照不删除。
5. scheduler 幂等键、机会现金池、周期预算预留、paper-only 环境限制和人工确认下单边界必须逐项回归验证。
6. C1–C4 和既有回测报告只作为研究工件保存，不被改写为新策略的业绩宣传。

## 6. 小步实施计划 / Small-PR Delivery Plan

### PR 1 — 策略契约与 Legacy 包装 / Policy Contract and Legacy Wrapper（已完成 / Complete）

- 新增 `strategy-policy` 的标识、版本、上下文、推荐与 trait。
- 新增 `CoreOpportunityV1` 适配器，调用现有函数并做逐项输出回归测试。
- 不改数据库、HTTP、scheduler、OpenD 或默认行为。

### PR 2 — 固定 DCA 与统一解析入口 / Fixed DCA and Unified Resolver（已完成 / Complete）

- 增加 `FixedDcaPolicy` 与内置策略 registry。
- 为计划增加最小的内置策略绑定，并通过 SQLite migration 将旧计划回填到 Legacy。
- 将手动预览和 scheduler 的应用入口改为 resolver；验证固定 DCA 与 Legacy 都能走同一 paper-only 闭环。

实施说明：SQLite migration 将已存在计划绑定为 `core_opportunity_v1@1`，新计划默认
绑定 `fixed_dca@1`。固定 DCA 不读取市场、Qwen 或调用方伪造的 70/20 信号；其审计快照
会明确记录信号未使用。当前 resolver 仅接受两个内置策略，未知引用在 HTTP 边界安全拒绝。

### PR 3 — 策略版本领域与审计升级 / Strategy Version Domain and Audit Upgrade（已完成 / Complete）

- 计划通过不可变 `PolicyRef` 绑定支持的内置策略版本；未知策略在 HTTP 边界安全拒绝。
- SQLite 决策记录追加保存 `policy_id`、`policy_version` 和通用 `recommendation_snapshot`；旧记录继续读取为无策略证据。
- `StrategySpec`、策略状态、哈希与用户策略 CRUD 仍属于后续受限 DSL/Studio 工作。

### PR 4 — 受限 DSL/AST 与校验 / Restricted DSL/AST and Validation（已完成 / Complete）

- 新增独立的无 IO `strategy-dsl` crate，提供 `StrategySpec`、`StrategyRule`、白名单 `IndicatorSpec`、有限 `ValueExpression`、条件树和动作定义。
- 只允许价格、SMA、EMA、RSI、回撤、VIX；动作仅为固定金额、机会桶倍率与跳过机会桶，不能表达核心桶否决、网络调用或任意代码执行。
- 构造器和校验器拒绝非法窗口、空条件组、零除、非自定义策略 ID、空/过多规则、过深/过大条件树及超过调用方周期预算的固定金额。
- 本阶段不含 serde、SQLite、HTTP、策略运行时、回测接入、Qwen 或前端编辑器。

### PR 5 — 确定性 DSL Runtime / Deterministic DSL Runtime（已完成 / Complete）

- `StrategySpec::evaluate` 将完整 `DecisionContext<DslEvidence>` 解释为 `DslEvaluation` 与通用 `InvestmentRecommendation`。
- 规则按固定首条命中顺序求值；缺失指标、重复指标、算术溢出和无效周期预算安全失败。
- 运行时不得发起 IO、写审计或生成订单；其动作只能作用于机会桶，核心桶不会被 DSL 否决。

### PR 6 — 统一历史评估 / Unified Historical Evaluation（首个候选已完成 / First Candidate Complete）

- `strategy-evaluation` 直接调用相同 `StrategySpec::evaluate` runtime，新增 `dsl_rsi_opportunity_guard_v1` 研究候选。
- 候选在决策日只读取 RSI-14：低于 35 时机会桶为 `1.10x`，高于 65 时为 `0.85x`，否则为 `1.00x`；核心桶固定，成交仍在第一条严格更晚的观察价格。
- 使用匹配的现金流、成本、成交时点、XIRR、期末净值、最大回撤、波动率、Sortino 与现金使用率比较策略和固定 DCA。该候选仅为研究，未变更生产默认策略。

### PR 7a — 策略版本存储与只读 API / Strategy Version Storage and Read-only API（已完成 / Complete）

- SQLite 新增不可变 `strategy_specs` 表，以 `(policy_id, policy_version)` 作为主键保存规范化 DSL JSON；读取路径先反序列化 DTO，再调用领域构造器重建并校验策略，损坏或不一致的数据安全拒绝。
- 新增只读 `GET /strategies` 与 `GET /strategies/:policy_id/:policy_version`；没有创建、更新、删除、激活、执行或下单 HTTP 入口。
- 该阶段不改变现有计划绑定、内置 resolver、scheduler 或 paper-only 执行边界。

### PR 7b — 策略验证、激活与 Web Studio / Strategy Validation, Activation and Web Studio（已完成 / Complete）

- `POST /strategies/validate` 返回可读、无内部细节的 DSL 校验结果；`POST /strategies` 只保存已验证的不可变版本，策略不支持更新或任意脚本。
- Strategy Studio 提供版本列表、只读规则详情、RSI(14)/VIX 白名单表单、验证错误与明确的计划激活确认。
- 已激活 DSL 会由自动 Decision Preview、scheduler 与审计共用同一 Runtime；仅允许机会桶倍率或跳过机会桶，核心桶不可否决，审批模式不会自动下单，仍为 paper-only。
- 当前线上 evidence profile 仅支持 RSI(14) 与 VIX；其余 DSL AST 指标保留给离线研究，不能被激活。

### PR 7c — DSL Runtime V2 / Shared Technical Evidence（已完成 / Complete）

- 线上与离线研究共用 `DslEvidence::from_market_snapshot`：由同一段纯 Decimal 逻辑计算收盘价、SMA、EMA、RSI、窗口回撤和 VIX，输入仅允许决策 `as_of` 当日及以前的日线。
- 自动 preview/scheduler 在审计来源快照中记录 `as_of`、OpenD/Cboe 数据来源和实际所需指标窗口；保存策略后可用只读 `simulate` 查看首条命中规则和输入证据。
- Studio 支持多条有序优先规则、单条件/全部满足/任一满足组、窗口与阈值、版本复制以及当前数据模拟；仍不支持自由代码或固定金额动作线上激活。

### PR 7d — 固定样本策略准入 / Fixed-fixture Strategy Admission（已完成 / Complete）

- 保存前重走 DSL 文档到领域构造器的完整校验；保存本身不等同于可执行或可激活。
- `GET /strategies/:policy_id/:policy_version/admission` 对每个已保存版本执行固定、版本化 `calibration-v2` 回测，同时锁定核心桶只能由计划配置生成、DSL 动作只能影响机会桶，并以固定周期预算检验动作边界。
- 策略与 `Fixed DCA` 必须在相同外部现金流、成本与“决策日后下一观察日成交”口径下展示期末净值、最大回撤、年化波动率和现金使用率；结果是准入信息，不构成收益承诺，也不以跑赢为激活条件。
- 当前夹具仅含 RSI(14)/VIX 的版本化因果输入。依赖价格、SMA、EMA、回撤的策略可继续保存/当前数据模拟，但不能激活，直到其历史证据被纳入不可变夹具。

### PR 8 — Qwen Strategy Copilot / Qwen Strategy Copilot

- Qwen 根据受限 schema 生成“候选策略草案”、解释与警告。
- 候选必须经确定性 validator、回测和用户审阅后才能保存；AI 输出永不直接激活或下单。

## 7. 验收门槛 / Acceptance Gates

- 每个新公开 Rust API 有 rustdoc，带不变量的类型仅通过构造函数或 `TryFrom` 建立。
- 每个行为变化都有聚焦测试，至少运行 `cargo test -p core-domain`；策略 PR 额外运行相关 crate 测试、fmt、Clippy 与 `git diff --check`。
- 历史评估不得使用未来数据；决策日与成交日至少相隔一个可用交易日。
- 任意策略推荐均受计划预算、周期上限、机会现金、可用现金和 paper-only 安全边界限制。
- 对外文档只报告实际、可复现结果；策略未证明稳定优势时不得宣称提高收益。

## 8. 当前结论 / Current Decision

策略契约、Legacy 包装、Fixed DCA、统一 resolver、最小策略版本审计、受限 DSL 定义/校验、首个 runtime-backed 历史候选、不可变版本存储、Studio 与固定样本准入已建立；不继续以 C5/C6/C7 方式搜索 70/20/10 权重，也不把 C1–C4 或 DSL RSI 候选升级为默认生产策略。下一项可执行工作是 **PR 8：Qwen Strategy Copilot**，或先扩展版本化历史夹具以放开更多 DSL 指标的准入。

With this foundation in place, no further C5/C6/C7 weight search will be promoted to production. The next executable work item is **PR 8: Qwen Strategy Copilot**, or versioned historical-fixture expansion for additional DSL indicators.
