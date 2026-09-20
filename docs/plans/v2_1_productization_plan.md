# IndexLink V2.1 本地优先发布过渡计划

> 状态：**工程收口完成，进入发布验证**（2026-09-20 审计）。M0 工程收敛和 M1 的 `Plan → readable Decision → user-reported execution → Audit` 已完成；官方目录现包含一个 Fixed DCA 基准与 20 个规则家族的 100 个不可变 Formula 预设，真实多市场回测、动态标的建计划与 Formula 创建前数据预检均已落地。下一阶段回到 3–5 位目标用户验证，不把完整调度重构重新变成上线前置条件。

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
   - 前端只展示 20 个家族入口，在家族内选择灵敏、偏短期、均衡、稳健或长期档；100 个预设不是收益排行，也不表示系统推荐同时使用 100 个策略。
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
   - SQLite 是默认持久化层；PostgreSQL 仅显式 opt-in。
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
| 个人中心 | 读取真实 plan、`due` decision 和 manual execution；展示策略方法、基础预算、下一评估日与历史 | 已完成；不是浏览器演示状态 |
| 我的计划 | 可从官方目录选择策略，创建、选择、暂停、继续和删除真实计划 | 已完成；已移除 symbol/币种硬编码并加入 Formula 预检 |
| 手工执行 journal | append-only SQLite/API/前端闭环；同一建议只允许一个最终结果 | 已完成 |
| 官方策略目录 | Fixed DCA 与 20 家族 × 5 Formula 预设均来自 `GET /strategy-catalog`；家族、参数档、标签、来源与校验等级由服务端提供 | 已完成；前端按家族浏览，DSL 不作为普通入口 |
| 策略分析 | `POST /strategy-backtests` 返回 US/HK/SH/SZ 自选标的真实轨迹、指标与来源，支持 1m/3m/6m/1y/3y/5y/all | 已完成；无数据时明确失败 |
| 市场数据与 Formula 决策 | Formula 运行只读取价格历史；OpenD 日线 adapter、本地 canonical store 与创建前数据充足性校验已存在 | 已完成；provider 不可用、历史不足或过期时创建失败且不落半成品计划 |
| 可选能力隔离 | OpenD 行情、OpenD paper broker、AI 独立报告；失败不阻塞 SQLite 核心 | 已完成 |
| PostgreSQL | 默认 feature graph 已移除，显式 feature 仍可用 | 已完成 |
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
| 可选 | 无 | Qwen 解释、本机 OpenD 行情或模拟账户 | Docker、环境变量、备份路径 |
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

发布 `v2.1.0` 前必须满足：

1. 干净环境可启动；Fixed DCA 不需要 AI、OpenD、broker、外部 Key 或 Docker 参数。
2. Plan → Decision → user-reported execution → Audit 完整可复现。
3. 可采用策略具备版本、规则、风险、数据要求、真实回测和 Fixed DCA 对照。
4. 动态标的计划不会伪造市场/币种；Formula 数据不足时在创建前失败，Fixed DCA 不受影响。
5. scheduler 不自动下单；paper 与 manual journal 明确分账。
6. Rust 聚焦测试、前端 lint/test/coverage/build、迁移测试和 `git diff --check` 通过。
7. 发布页明确本地优先、研究边界、manual-first、已知限制和不构成投资建议。
8. 3–5 位目标用户完成建立计划、找到建议、记录执行和找回历史的任务测试，并形成 Go / Adjust / Stop 决策。

## 12. 本轮之后

本轮动态标的与 Formula 预检完成后，优先执行用户任务验证，不继续堆叠策略数量。

只有证据明确支持后，才评估：调度模型升级、Simple Builder、更多数据 provider、复杂多资产策略、桌面发行、分享与 fork。IBKR/QMT/Futu/moomoo 自动下单、云账户与公开策略社区仍不属于 V2.1。

V2.1 的成功标准不是跑赢市场或连接更多券商，而是普通长期投资者能够持续理解、执行并复盘自己的策略。
