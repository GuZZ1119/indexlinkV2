# IndexLink V2.1 本地优先发布过渡计划

> 状态：**工程与本地安全收口完成，进入发布验证**（2026-09-23 审计）。M0 工程收敛和 M1 的 `Plan → readable Decision → user-reported execution → Audit` 已完成；官方目录现包含一个 Fixed DCA 基准与 20 个规则家族的 100 个不可变 Formula 预设，真实多市场回测、动态标的建计划与 Formula 创建前数据预检均已落地。下一阶段回到 3–5 位目标用户验证，不把完整调度重构重新变成上线前置条件。

> 执行硬约束、停止条件与 Gate 顺序以 [`v2_1_closeout_hardness.md`](./v2_1_closeout_hardness.md) 为准。本文件记录产品能力、当前事实和后续边界，不授权并行启动全部路线。

## 1. 发布定位

IndexLink V2.1 是面向长期投资者的**本地优先策略与手动执行工作台**。

用户选择受审查的官方策略，把策略版本和自己的标的、预算、节奏冻结为本地计划，获得可理解、可审计的周期建议，在外部券商自行执行，并把已执行或跳过记录在本地。

V2.1 的发布承诺是：

> 我可以在自己的设备上选择一个看得懂的长期策略，知道本期应该做什么，并准确回看当时的策略版本、依据和自己报告的操作。

它不是多用户 SaaS、公开策略社区、通用量化平台或自动交易终端。70/20/10 是保留的历史研究策略，不是 V2.1 的产品总定义。

## 2. 目标用户与唯一主闭环

首个目标用户是有稳定现金流、维护少量长期股票/ETF 计划、希望理解并坚持投入纪律的个人投资者。他们不需要写 Python、DSL 或配置券商权限。

```text
选择官方策略 → 理解规则与限制 → 建立本地计划
→ 获得本期建议 → 在外部券商手工操作
→ 记录已执行或跳过 → 回看不可变建议与执行历史
```

若一个需求不能改善这条闭环，它不应阻塞 V2.1 发布。

## 3. 产品边界

### V2.1 必须保持

1. **普通用户信息架构**
   - 个人中心回答“本期要做什么、为什么、是否完成”。
   - 普通界面使用“策略、计划、建议和执行记录”，不暴露 Policy、DSL、Admission 等工程概念。
   - AI、OpenD、paper broker、70/20/10 与研究实验留在高级区域。

2. **官方策略目录与版本化采用**
   - 当前官方目录包含一个 Fixed DCA 基准，以及 20 个透明规则家族各 5 档参数生成的 100 个不可变 Formula 预设；MA200 趋势保护和增长/波动平衡的原 policy ID 与公式保持兼容。
   - 前端只展示 20 个家族入口，在家族内直接选择 20 日、50 日或多周期组合等可核验参数；100 个预设不是收益排行，也不表示系统推荐同时使用 100 个策略。
   - 用户必须先明确选择策略；建立计划时冻结官方策略版本和用户参数。
   - 后续模板更新不能静默改变既有计划。

3. **真实回测与研究边界**
   - 策略分析使用服务端真实行情和确定性 runtime，不生成演示收益曲线。
   - 同次比较使用同一标的、时间范围、现金流口径和共同起点；Fixed DCA 是基线。
   - 结果显示数据来源与专业指标，但只代表历史研究，不构成预测或推荐。

4. **Manual-first 执行留痕**
   - `DecisionRecord` 不可变；用户报告的执行结果使用独立 append-only journal。
   - 普通界面只开放“已执行”和“跳过”；历史 `partial` 保持只读兼容。
   - 每条 `due` 建议最多保存一个最终结果，planned、actual 和输入证据不能相互覆盖。

5. **本地优先与故障隔离**
   - SQLite 是 V2.1 唯一公开支持的持久化层；未接线的 PostgreSQL 草稿适配器已从公开构建移除。
   - 无 AI、OpenD、broker、外部 Key 或 Docker 时，Fixed DCA 的 Plan、Decision 与 Audit 仍可用。
   - 行情与 paper broker 独立配置、独立初始化、独立报告 capability；真实连接失败不得伪装成 Mock。

### V2.1 明确不做

- 多用户、登录、云同步、公开发布、评论、关注、排行榜和 marketplace；
- 任意 Python、JavaScript、Pine Script 或第三方仓库代码执行；
- IBKR、QMT、Futu/moomoo 实盘自动下单或 scheduler 无人值守交易；
- 高频、Tick、盘口、自动优化、税务、公司行动处理和通用 Portfolio 引擎；
- 因本轮新增动态标的而承诺“策略适合所有股票”或承诺数据供应商覆盖全部证券。

## 4. 当前事实账本

| 能力 | 当前事实 | V2.1 判断 |
| --- | --- | --- |
| 个人中心 | 读取真实 plan、`due` decision 和 manual execution；展示策略方法、基础预算、下一评估日与历史 | 当前只投影下一次评估；有限期限日历见后续 P1 |
| 我的计划 | 查看、创建、选择、暂停、继续和删除真实计划；常驻新建入口跳转策略中心，精确选中策略后才返回显示配置表 | 已完成；已移除重复策略卡、symbol/币种硬编码并加入 Formula 预检 |
| 手工执行 journal | append-only SQLite/API/前端审计闭环；同一建议只允许一个最终结果 | 事实记录已完成；实际金额尚未结算入后续资金状态 |
| 官方策略目录 | Fixed DCA 与 20 家族 × 5 Formula 预设均来自 `GET /strategy-catalog`；家族、参数档、标签、来源与校验等级由服务端提供 | 已完成；前端按家族浏览，DSL 不作为普通入口 |
| 策略分析 | `POST /strategy-backtests` 返回 US/HK/SH/SZ 自选标的真实轨迹、指标、完整模拟执行记录、Formula 规则命中标记与来源，支持 1m/3m/6m/1y/3y/5y/all | 已完成；净值图、标的走势/规则触发点、逐日回撤、资金拆分和公式代入值共享同一响应，无数据时明确失败 |
| 市场数据与 Formula 决策 | Formula 运行只读取价格历史；OpenD 日线 adapter、本地 canonical store 与创建前数据充足性校验已存在 | 已完成；provider 不可用、历史不足或过期时创建失败且不落半成品计划 |
| 可选能力隔离 | OpenD 行情、OpenD paper broker、AI 独立报告；失败不阻塞 SQLite 核心 | 已完成 |
| PostgreSQL | 从 V2.1 公开构建和上游跟踪移除；server 只接受 SQLite | 已完成收口，不作为隐藏能力 |
| 自动实盘交易 | 不存在，也不属于 V2.1 | 保持不做 |
| 用户任务验证 | 尚无 3–5 位目标用户的完整任务证据 | 本轮工程收口后的首要工作 |

旧计划中“策略卡、个人中心金额、完成状态、策略采用和策略分析曲线仍是演示”“官方策略目录、append-only 手工记录和统一回测 API 尚未提供”等描述均已失效，不再作为待办。

## 5. 本轮工程收口：动态标的计划

### Goal

让能够被现有 market-data 层规范解析、且满足所选策略数据要求的 US/HK/SH/SZ 股票或 ETF，既能运行真实回测，也能建立计划并生成相同确定性 runtime 的建议。

### Delivered state

- 回测、Formula 决策和计划创建使用同一个 `Instrument` 与 `HistoricalPriceProvider` 边界处理 market-qualified symbol。
- 策略目录以支持市场和最小收盘价观测数表达能力；弃用的 `supported_symbols` 为空，不再参与准入。
- 服务端复核市场与币种并保存规范化代码；浏览器只做即时提示，不再把 `USD` 当作全市场事实。
- Formula 在计划写入前检查最小历史窗口和最近数据时效；历史不足返回 `400`，provider 不可用返回 `503`。Fixed DCA 保持无行情依赖。

### Desired behavior

```text
用户输入标的
→ 服务端解析并规范化 canonical symbol
→ 从标的/数据集确定 market、currency、timezone、instrument type
→ Fixed DCA：不依赖行情即可创建
→ Formula：按策略声明预检历史数据窗口
→ 通过后冻结策略版本与计划参数
→ 使用同一 PriceHistoryProvider 和确定性 runtime 生成建议
```

### Architecture constraints

- SDK 类型不得泄漏到领域层；远端 provider 只负责显式导入/更新，本地 `PriceHistoryProvider` 是策略、Today 与回测的读取边界。
- canonical dataset 必须保留 provider、market、instrument type、currency、timezone、adjustment、as-of、dataset version 与 checksum。
- 标的解析、市场/币种推导与 Formula 数据充分性由服务端决定，前端校验不是安全边界。
- Fixed DCA 不得因未配置 OpenD 或历史行情而失去零依赖可用性。
- 预检失败不得静默回退到 DCA、替换 provider 或伪造默认 USD。

### Explicit non-goals

- 不自动向券商下单；“可以运行”只指生成可审计建议。
- 不保证任意证券都有数据，也不判断策略是否适合某只股票。
- 不在本轮引入新 provider、分钟/Tick 行情、跨资产组合或任意策略代码。
- 不在本轮拆分资金周期与观察频率。

### Acceptance criteria

1. 策略目录、计划创建和前端不再用静态 symbol 数组决定 Formula 可用性。
2. `US.*`、`HK.*`、`SH.*`、`SZ.*` 的合法股票/ETF 由同一解析器规范化；不支持市场或非法代码明确失败。
3. 计划保存并返回正确的 canonical symbol 与 currency；market 由 symbol 和行情审计元数据确定，浏览器不再把 USD 写成全市场默认事实。
4. Fixed DCA 在无 market-data provider 时仍可为合法标的创建计划。
5. Formula 创建前验证其声明的有效日线窗口；历史不足、缺失、过期或 provider unavailable 返回可解释错误，且不落半成品计划。
6. 通过预检的 Formula 计划能沿现有 automatic decision 路径生成真实建议，不另建一套计算实现。
7. US/HK/SH/SZ 成功路径及历史不足、provider unavailable、非法标的和 Fixed DCA 无行情路径均有聚焦测试。

### Deliverables

- 动态策略适用性与 instrument metadata 契约；
- 计划创建的 canonical symbol/market/currency 服务端校验；
- Formula 历史数据预检与明确错误模型；
- 前端动态提示与错误呈现；
- API 文档、聚焦测试和 `CHANGE_LOG.md` 记录。

### Rollback

每个 Push 保持现有 SQLite schema 和 `DecisionRecord` 兼容。若 Formula 预检出现 provider 兼容问题，可单独回滚 Formula 动态采用入口，但不得恢复浏览器伪造币种或影响 Fixed DCA 核心闭环。

## 6. 本轮 Push 与合并顺序

以下 Push 已按依赖顺序完成并进入整体验证：

| Push | 状态 | 完成条件 |
| --- | --- | --- |
| A — instrument contract | 已完成 | 删除静态 symbol 白名单；声明支持市场和数据要求；服务端输出 canonical symbol/currency，market 由解析与审计元数据给出 |
| B — Formula preflight | 已完成 | 写入 plan 前验证策略所需历史；失败不持久化；Fixed DCA 无行情仍通过 |
| C — Web closeout | 已完成 | 任意合法 market-qualified symbol 可提交；币种动态显示；预检错误可理解 |
| D — integration audit | 已完成 | API/Rust/Web 检查通过，无白名单残留，无自动交易范围扩张 |

共享契约必须串行合并；多个 Agent 使用独立 worktree，不同时修改同一文件。金额、币种、持久化与 Formula 预检需要独立 reviewer。

## 7. 数据与策略适用性表达

V2.1 对外不说“所有策略都适用于任何股票”，而说：

> 该标的可被系统识别，并具备当前策略计算所需的数据。历史规则结果不代表策略适合该资产，也不构成投资建议。

| 策略 | 创建条件 | 行情依赖 |
| --- | --- | --- |
| Fixed DCA | 标的可被规范解析 | 无；保持离线基线 |
| 价格/均线、动量、价格位置与风险类 Formula | 标的可解析，且具备精确策略版本声明的有效复权日线窗口 | 创建前预检；评估和回测按不可变 policy ID 读取最近可用 canonical history |
| 复合 Formula（含原增长/波动平衡） | 标的可解析，且所有组成指标均具备所需窗口 | 创建前预检；多个指标读取同一因果日线快照，不引入额外预测数据 |

Formula 的 `base_amount` 是每个资金周期的基础预算，不等于提前承诺的最终投入。最终建议只能在评估日按冻结规则和当时可用的因果数据计算，并受核心桶与单次上限约束。

## 8. 用户配置与部署原则

普通用户只配置策略、标的、基础预算和当前支持的每周/每月评估日，不配置基础设施。

| 配置类别 | 普通用户 | 高级用户 | 部署者/开发者 |
| --- | --- | --- | --- |
| 必填 | 策略、标的、基础预算、评估节奏 | 同左 | 同左 |
| 默认自动处理 | SQLite、策略版本、canonical symbol、市场和币种 | 同左 | 端口与日志可覆盖 |
| 可选 | 无 | 手动触发的 Qwen/GPT/Claude/DeepSeek 策略草案、回测解释和个人摘要；AI Key 与只读 OpenD host/port 可通过高级实验室仅存当前 Rust 进程；新闻情绪仍为高级能力；模拟账户继续独立配置 | Docker、环境变量、备份路径 |
| 永不要求 | API Key、券商密码、数据库 URL、Docker 参数 | 浏览器保存券商凭据 | 向浏览器暴露密钥 |

Docker 是分发/自托管方式，不是产品设置。基础路径必须在无 `.env`、Qwen 或 OpenD 时启动，并使用 SQLite、Fixed DCA 与安全默认值。

## 9. 回测、执行与连接边界

### 回测

```text
strategy_version + dataset_version + assumptions_version → BacktestResult
```

- 相同版本组合重复运行应产生相同结论。
- 同一比较必须共享标的、现金流、成本、成交时点、因果 cutoff 和数据集。
- Fixed DCA 是同口径基线。
- 前端不得自行生成或硬编码收益、回撤、波动率等真实结论。
- 专业研究必须同时公开公式口径、API 返回的中间量与本次代入结果；回撤路径和每期资金拆分必须由服务端同一模拟账本返回，浏览器只负责解释与可视化。

### 执行

| 能力 | V2.1 边界 |
| --- | --- |
| 建议生成 | 基于已持久化计划、策略版本和输入快照，显示金额与理由 |
| 用户报告 | append-only `executed/skipped`，明确标记 `user-reported` |
| Mock/OpenD paper | 可选高级能力，与 manual journal 分账 |
| OpenD 行情 | 可选 Formula 数据来源，不是 Fixed DCA 依赖 |
| 实盘自动交易 | 明确不做；scheduler 不得下单 |

“回测可运行”“计划可生成建议”和“券商已执行”是三个不同事实，界面和审计不得混写。

## 10. 后续 🔔：调度模型升级

当前 `weekly/monthly + 一个评估日` 足以保持 V2.1 可控，但不是 Formula 的最终调度模型。以下进入后续版本，不阻塞本轮动态标的收口：

1. **资金周期与观察频率分离**：例如每月准备一次预算，但每个交易日检查趋势。
2. **交易所日历与时区**：基于标的交易所判断交易日、收盘 cutoff 和数据可用时间。
3. **节假日规则**：明确顺延到下一交易日、回退到上一交易日或使用上一收盘数据，不能由浏览器本地日期猜测。
4. **多个观察点**：支持每日观察、每周/月多个候选评估日和条件触发窗口。
5. **周期级幂等**：即使观察多次，每个资金周期最多生成一次可执行建议；重启、重试和 provider 延迟不能重复建议。
6. **研究版本化**：观察频率、节假日规则或执行窗口变化会改变结果，必须产生新的 assumptions/strategy version 并重新回测。

本轮只做小幅文案与默认值改进：DCA 显示“投入节奏”，Formula 显示“评估节奏”；默认值来自策略目录，不直接把创建当天（可能是周末）当成策略事实。

## 11. V2.1 发布门槛

发布首个公开 Beta `v2.1.0-beta.1` 前必须满足：

1. 干净环境可启动；Fixed DCA 不需要 AI、OpenD、broker、外部 Key 或 Docker 参数。
2. Plan → Decision → user-reported execution → Audit 完整可复现。
3. 可采用策略具备版本、规则、风险、数据要求、真实回测和 Fixed DCA 对照。
4. 动态标的计划不会伪造市场/币种；Formula 数据不足时在创建前失败，Fixed DCA 不受影响。
5. scheduler 不自动下单；paper 与 manual journal 明确分账。
6. Rust 聚焦测试、前端 lint/test/coverage/build、迁移测试和 `git diff --check` 通过。
7. 发布页明确本地优先、研究边界、manual-first、已知限制和不构成投资建议。
8. 3–5 位目标用户完成建立计划、找到建议、记录执行和找回历史的任务测试，并形成 Go / Adjust / Stop 决策。

## 12. 后续 P1：有限期限的计划日程投影

### Goal

让用户在不生成未来建议、不读取未来行情的前提下，查看一段有限期限内的计划评估日期和诚实的金额口径，而不是只能看到“下一次”。

### Current state

- 计划只保存 `monthly/weekly`、`schedule_days`、冻结 IANA `timezone` 与资金边界；没有结束日期、总期数或完整未来日程。
- Web 的 `nextScheduledDate` 只计算一个日期；scheduler 只为已到期日期幂等生成 `DecisionRecord`。
- Fixed DCA 的每期金额在计划建立时已确定；Formula 的弹性额度必须等评估日取得真实历史数据后才能确定。
- 当前没有交易所交易日历，计划日期不能被描述成保证可成交的交易日。

### Desired behavior

1. 新增服务端权威的只读日程投影，例如 `GET /investment-plans/:id/schedule-projection?months=3|6|12`；同时设置最大月份与最大条数，拒绝无限 `all`。
2. 每个条目返回计划本地日期、IANA timezone、周期类型和金额口径，不创建、预占或修改任何 `DecisionRecord`。
3. Fixed DCA 返回确定的计划金额；Formula 返回固定核心金额、弹性额度范围和单次总上限，并标记“评估日确定”，不得返回伪精确金额。
4. Web 使用“计划评估日历”命名，并明确这是计划日期而非交易所成交承诺；暂停计划可查看配置，但标记不会生成新建议。
5. 未来接入交易日历后，顺延/回退规则必须成为版本化 assumptions，不能由浏览器自行猜测。

### Architecture constraints

- 日期生成放在 investment-plan 应用/领域边界，由 API 和 scheduler 复用；React 只展示服务端结果。
- 所有日期按计划冻结的 IANA timezone 解释，DST 不得转成浏览器本地日后再计算。
- 投影是只读事实，不访问未来价格、不运行策略、不占用调度 claim、不提交 broker 请求。
- Formula 金额范围由计划的核心/机会桶和 `max_single_execution` 推导；机会现金滚存存在时需返回可解释上限与来源，不能静默混入。

### Explicit non-goals

- 不预测未来策略命中、收益、价格或真实成交金额。
- 不把周末/节假日自动改成交易日；在 canonical 交易日历落地前只展示“评估日期”。
- 不在 V2.1 引入无限期日历、提醒推送、自动下单或云同步。

### Acceptance criteria

- 3/6/12 个月投影在每月 28 日跨月、跨年、周度多日期、DST 与暂停计划上保持确定且有界；29–31 日仍不属于当前计划模型。
- 同一计划、from/to 和 timezone 得到稳定排序且无重复日期。
- Fixed DCA 的金额与计划一致；Formula 只显示范围及“评估日确定”。
- 调用投影前后 Decision、scheduler claim、period budget 与机会现金表完全不变。

### Tests and deliverables

- 领域测试：月度/周度、多日期、跨年、DST、边界上限和无重复。
- API 测试：合法期限、越界拒绝、暂停计划、Fixed/Formula DTO 与无写入证明。
- Web 测试：期限切换、空状态、金额口径、timezone 与非交易日免责声明。
- 交付物：领域投影函数、只读 API、React Query hook、个人计划日历和公开 API 文档。

## 13. 后续 P2：手工执行记录进入资金结算闭环

### Goal

让用户报告的 `executed/skipped` 结果在保持 `user-reported` 身份和原建议不可变的前提下，幂等影响后续周期预算与机会现金，而不仅是可回看的备注。

### Current state

- manual journal 已 append-only 保存 outcome、actual amount、发生时间与输入来源，同一 Decision 最多一个最终结果。
- 追加 manual event 不修改 `DecisionRecord`、不重算策略，也不调用 broker；这是正确的审计边界。
- 周期预算与机会现金目前只会根据已接受/终态 paper order 结算，手工实际金额不会进入下一期资金状态。

### Desired behavior

1. 新增独立 `ManualExecutionSettlement` 领域输入：引用 manual event、不可变建议快照、planned/core/opportunity 金额、actual amount、币种、计划周期键和发生时间。
2. 由一个结算 Unit-of-Work port 在同一 SQLite 事务内追加事件并写入独立结算账本；以 `manual_event_id`/`decision_record_id` 唯一约束保证重试、刷新和重启不会重复结算。
3. `executed` 按用户报告实际金额在计划硬上限内拆分核心/机会用量；`skipped` 结算为零投入，并按计划冻结的机会现金政策决定到期或滚存。无法无歧义拆分时拒绝结算，不能猜测。
4. 保存结算时使用的 Decision 与 plan 输入快照、算法版本和计算结果；后续计划编辑不得追溯改写历史。
5. manual settlement 与 paper fill ledger 分账并带 provenance；同一资金状态只能选择一个权威结算来源，禁止同时把 user-reported 与 broker-verified 金额计入资金，但两类原始证据可以独立共存。
6. 未来建议读取统一的已结算资金状态，而不是直接扫描 UI 事件；读取失败时明确不可用，不静默回退为零。

### Architecture constraints

- `DecisionRecord` 和 manual event 保持不可变；纠错采用追加 reversal/correction 事件，不做 UPDATE。
- 金额使用 Decimal/newtype 校验，币种必须与计划和 Decision 一致；actual amount 不能越过计划单次及周期上限。
- 结算服务位于应用层，通过聚合 Unit-of-Work/repository port 原子写入 manual event 与 settlement；paper adapter 不依赖 manual DTO，manual 路径也不伪造 broker ack。
- migration 必须兼容既有 journal；老记录默认“未结算”，只能由用户显式确认或受控迁移，不能后台自动猜测。
- settlement 接线必须有本地 feature/config gate；回滚只停用新的资金读取和写入，追加 migration 与既有结算事实继续可读，不删除或改写历史。

### Explicit non-goals

- 不验证用户是否真的在券商成交，不将 `user-reported` 改成 broker verified。
- 不根据备注解析成交、不自动连接实盘账户、不修改历史建议金额。
- 不在完成结算前让未来日历显示 Formula 的精确建议金额。

### Acceptance criteria

- 同一事件重复提交、API 重试与服务重启只产生一次资金影响。
- executed/skipped、少于建议、等于建议、超上限、币种不匹配和历史未结算记录均有明确结果。
- 下一期 Decision 使用结算后的机会现金与周期预算，并保存所读余额版本；删除/暂停计划不破坏历史结算。
- 资金状态对 manual 与 paper 只选择一个权威来源且不会双计；审计页仍可分别展示“用户报告”和“模拟券商确认”的原始证据。

### Tests and deliverables

- 领域测试：金额拆分、不变量、skip/carry/expire、超限、来源互斥和 correction/reversal。
- storage/migration 测试：原子追加、唯一约束、崩溃回滚、旧库升级与重启幂等。
- API 测试：输入快照、409 重试、错误映射、来源标签和下一期读入。
- Web 测试：确认前展示资金影响预览，确认后展示不可编辑结算事实；失败时不把记录伪装成已结算。
- 交付物：领域结算契约、SQLite migration/Unit-of-Work repository、API DTO、下一期资金读取接线、feature/config gate、审计展示与升级/回滚说明。

### Dependency order

1. 先完成结算领域契约与来源互斥规则；金额、migration 与审计需独立 review。
2. 再实现 SQLite 账本和旧数据兼容，验证原子性与幂等。
3. 接通 manual API；仍不改变 scheduler 和 paper 行为。
4. 让新 Decision 读取统一资金状态并补因果/快照测试。
5. 最后让有限日程投影展示结算后的可解释预算范围；没有 P2 时，P1 只能展示计划静态边界。

## 14. 本轮之后

本轮动态标的与 Formula 预检完成后，优先执行用户任务验证，不继续堆叠策略数量。

受限 Simple Builder 已在 V2.1 内按 Formula V1 白名单落地：它只调整机会桶并保存不可变个人版本，不接受自由代码。只有证据明确支持后，才评估：调度模型升级、更多条件/动作模块、更多数据 provider、复杂多资产策略、桌面发行、分享与 fork。IBKR/QMT/Futu/moomoo 自动下单、云账户与公开策略社区仍不属于 V2.1。

V2.1 的成功标准不是跑赢市场或连接更多券商，而是普通长期投资者能够持续理解、执行并复盘自己的策略。
