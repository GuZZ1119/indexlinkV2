# IndexLink V2.1 收口 Hardness 与执行门槛

> 状态：**已生效**
> 项目负责人确认日期：2026-09-15
> Gate 1 实现集成基线：`f8bf9ee`（Push 2 / Push 3）
> 外部审查基线：`0cae8d5e00221ff245acf483119c0e8921d37b86`

## 1. 这份 Hardness 解决什么

这份文件把正式《IndexLink V2.1 Codex 收口执行手册》的产品边界、工程不变量、停止条件和验收顺序固化到仓库。它不是新的功能路线图，也不授权一次性实现全部功能。

当前唯一需要尽快验证的闭环是：

```text
Plan → readable Decision → user-reported execution → Audit
```

Fixed DCA 是这条闭环的强制基线。策略中心、统一回测、更多数据源、Simple Builder、Tauri 与 paper broker 都不能抢在这条闭环之前成为新的上线前置条件。

## 2. 资料优先级与冲突处理

发生冲突时按以下顺序执行：

1. 项目负责人在当前任务中确认的正式 V2.1 收口手册；
2. 根目录 `AGENTS.md` 中的 V2.1 产品与架构硬约束；
3. 本文件的阶段门槛和停止条件；
4. `docs/plans/v2_1_productization_plan.md` 的产品能力地图；
5. 旧实验、旧 README、旧 Dashboard 文案和历史百分比估计。

所有“后端约 80%”“整体约 55–60%”“固定 10 周”等估计均失效。完成度只按可复现的验收项判断。

## 3. 当前进度的事实基线

正式手册审查的是 `main@0cae8d5`。当前分支从该提交到 `0ff5920` 的变化集中在前端消费级壳、演示交互和计划文档，没有修改 Rust 后端、SQLite schema、市场数据、broker 初始化或执行日志。

| 区域 | 当前事实 | 判断 |
| --- | --- | --- |
| 普通首页 | `/personal` 已替代旧 Dashboard 作为默认入口，但金额、日期、进度和完成状态仍是前端演示数据 | 视觉壳可复用，M1 未闭环 |
| 旧 MA200 回放 | 普通首页与旧 Dashboard 均不再渲染；旧 API 保留，只有高级实验室可由用户手动触发 disabled query | Push 1 已隔离；端点仅作兼容，产品回测仍须重建 |
| Plan 与 Decision | Rust API、SQLite 与旧 `/plans`、`/decisions` 页面可用，但没有进入普通首页主流程 | 核心可复用，前端未映射 |
| 手工执行留痕 | 没有 append-only manual execution domain、migration、repository 或 API | M1 阻塞项 |
| 可选能力隔离 | Web 错误已局部化；OpenD 行情与 paper broker 现可独立启用、独立装配并报告 `not_configured/configured/unavailable`；失败不会阻止 SQLite 核心启动，也不会回退 Mock | Push 2 已完成 |
| DSL 数据依赖 | 除 Fixed DCA 外仍先拉完整宏观/趋势/VIX；价格型 DSL 不能只依赖价格历史 | M2 前置项，不阻塞 M1 |
| PostgreSQL | server 默认依赖图不再包含 `sqlx-postgres`；storage 仅在显式 `postgres` feature 下编译 PostgreSQL adapter 与测试 | Push 3 已完成 |
| 策略中心 | 三张静态卡、归一化演示曲线与浏览器选择状态已完成 | 仅前端壳，不是真实策略产品 |
| 专业研究 | 只对已保存 DSL 调用 admission API；内置策略没有统一研究结果 | M2 未闭环 |
| 本地市场数据 | 只有现有 market adapter 与研究 fixtures，没有产品级版本化本地历史数据层 | M2 未开始 |
| 桌面发行 | 没有 Tauri 壳和桌面 release 流程；服务默认 `APP_HOST=0.0.0.0` | M3 未开始 |

## 4. 不可破坏的不变量

### H1 本地优先

- 核心流程不依赖 IndexLink 云服务。
- 无 API Key、无 AI、无 OpenD、无 broker、无 Docker 时，Fixed DCA 核心仍可使用。
- Docker 是部署方式，不是普通用户配置项。

### H2 用户拥有执行权

- V2.1 不做无人值守实盘交易。
- scheduler 只能生成可审计建议，不能自动下单。
- paper trading 是可选实验能力，与用户手工执行记录完全分离。

### H3 审计不可变

- 原始 `DecisionRecord`、策略版本与输入快照不可被后续操作覆盖。
- 手工执行采用 append-only 事件，支持 `executed`、`skipped`、`partial`。
- planned 与 actual 同时保留，所有手工事件明确标记 `user-reported`。

### H4 可选能力失败局部化

- AI、市场数据、broker、研究回放分别报告 capability 与错误。
- 可选能力失败不得汇总成核心 Plan、Decision、Audit 不可用。
- 失败的真实 broker 不得静默替换为 Mock；对应交易入口必须明确禁用。

### H5 数据与回测可复现

- 供应商只负责导入，本地 canonical store 负责策略、Today 与回测运行。
- 每个 observation 与结果必须携带来源、口径、`as_of`、`dataset_version` 和 checksum。
- 同一 symbol/date 只有一个显式生效来源；冲突不得静默混拼。
- 调整价不冒充真实成交价；币种、时区和复权口径不得丢失。
- 同一比较必须使用相同外部投入、现金余额、成本、执行时点和因果 cutoff。
- Fixed DCA 始终是匹配现金流的基线；旧 `historical_backtest` 不得作为产品回测。

### H6 策略安全

- 不执行第三方 Python、JavaScript 或任意仓库代码。
- 普通用户面对策略、计划和行动，不面对 Policy ID、AST、DSL 或 Admission JSON。
- 没有真实、版本化研究结果的策略不能标记为可采用。

### H7 界面不伪造完成度

- 演示曲线、演示金额、浏览器会话状态只能明确标记为演示。
- 真实产品卡片、专业指标和采用状态必须来自后端契约。
- UI 不得自行计算并持久化金融结论；服务端状态使用 React Query，Valtio 只保存临时交互。

## 5. 数据来源登记 Financial API

候选项目：[HiThink-Tech Financial-API](https://github.com/HiThink-Tech/Financial-API)

当前登记结论：**保留为 M2 的 A 股和 A 股 ETF 可选导入来源候选，不进入 M0/M1，不成为 IndexLink 核心运行依赖，也不替代 OpenD 的港美股候选位置。**

### 可利用的能力

- 官方公开范围覆盖 A 股日线、公司行动与复权、指数、板块、公募基金和本地 DuckDB；支持 REST、CLI、Python SDK、MCP 与 Agent Skill。
- 代码仓库采用 MIT License，适合参考或复用 adapter/client 层实现。
- 本地 marketdb 提供原始、前复权、后复权视图和数据质量工具，可作为导入阶段的上游缓存或人工研究工具。

### 不得误判的边界

- 当前公开能力明确不含海外行情、分钟 K、tick、宏观数据和新闻原文，因此不能解决首发 US ETF 数据，也不能支撑依赖这些输入的策略。
- 远端能力和 marketdb 更新需要 API Key；它不能成为 V2.1 的零配置启动条件。
- MIT 只覆盖仓库软件，不自动授予供应商数据的缓存、展示或再分发权。未取得明确条款前，不把真实数据随 IndexLink release 打包。
- 不能让 IndexLink runtime 直接依赖 Python、Node、MCP、Agent Skill 或外部 DuckDB。优先实现窄的导入 adapter，把通过校验的数据写入 IndexLink 自己的 canonical market-data store。
- ETF 历史窗口、复权正确性、停牌/缺失语义、额度、限流和版本稳定性必须通过样本验收后才能声明覆盖。

### M2 前的准入问题

1. 账号可访问的精确 capability、历史长度、频率和费用是什么；
2. A 股 ETF、股票和指数能否分别映射到明确 `instrument_type`；
3. 原始价、前复权、后复权和公司行动能否形成一致且可验证的 dataset version；
4. 服务条款是否允许本地长期缓存、产品内展示和向 release 用户分发；
5. provider 失败时是否可保留已导入数据的完全离线查询和回测；
6. 能否用 pinned CLI/API contract 和 fixture 覆盖分页、幂等、冲突、缺失与 checksum。

以上问题未关闭前，只能做 adapter spike 或开发 fixture，不得把该来源写成 V2.1 已支持数据源。

## 6. 阶段门槛

### Gate 0 文档与 ownership

通过条件：

- 产品总定义已从 70/20/10 收束到 M1 人工执行闭环；
- `AGENTS.md`、本文件和产品计划没有相反的当前执行顺序；
- 0B、0C、0D 的文件 ownership、非目标、测试和回滚方式已登记；
- 不修改生产代码。

当前状态：**本次文档变更完成后通过。**

### Gate 1 M0 工程收敛

必须依次关闭：

1. 旧 MA200 回放退出普通 Today，保留兼容端点并完成 caller 检查；
2. 首页可选 query 按 capability/task 启用，错误局部展示；
3. market-data 与 paper broker 独立初始化、独立状态；
4. broker 失败不阻止 SQLite 核心启动，也不静默切换 Mock；
5. PostgreSQL 在默认 feature graph 中关闭，显式 feature 下仍可编译；
6. 历史决策、migration 与 SQLite 数据保持兼容。

未通过 Gate 1，不开始新的数据源、BacktestService 或完整策略目录。

当前状态：**已通过。** Push 1–3 已依序完成；合并后 workspace 测试、storage 默认/显式 PostgreSQL feature、依赖图断言、前端 lint / 90% 覆盖门槛 / production build 均通过。下一项是 Gate 2 的 Push 4：append-only Manual Execution Journal。

### Gate 2 M1 最小人工执行闭环

只做一个标的和 Fixed DCA：

1. 简化 Plan：标的、金额/周期、策略；
2. Today 读取真实 plan 与决策，只回答现在要做什么、金额、日期和原因；
3. append-only Manual Execution Journal；
4. 用户能记录完成、跳过或部分完成；
5. 决策详情同时显示原建议和所有手工事件；
6. 无 AI、OpenD、broker 与市场数据时全流程仍可用。

通过后立即让 3–5 位目标用户完成一次任务，不先扩展策略数量。

### Gate 3 用户任务验证

每位测试用户至少验证：创建 Fixed DCA 计划、找到本期建议、用普通语言复述原因、记录完成/跳过、找回历史记录。记录任务成功率、阻塞点、误解和是否愿意在下一周期继续使用。

只有反馈明确支持“需要比较或定制策略”，才进入 M2。

### Gate 4 M2 两策略公平比较

执行顺序必须串行：

```text
Market Data Dependency Refactor
→ Product Local Market Data Store
→ Product-facing BacktestService
→ 两策略 Compare
→ Backtest adversarial review
```

Fixed DCA 加一个受限规则策略即可。完整策略目录、三到五个策略和 Simple Builder 不是首次用户验证门槛。

### Gate 5 M3 扩展与发行

- 只有用户反馈支持时才扩展策略中心与 Simple Builder。
- 只有安装成为真实测试阻碍时才提前做单平台 Tauri 壳。
- 桌面版必须强制 loopback、限制 sidecar 权限、使用 app-data、处理迁移与崩溃；当前通用 server 的 `0.0.0.0` 默认不能直接沿用到桌面。
- 最终执行 V2.1 release audit，并只审计已约定 milestone，不把延期能力重新变成阻塞项。

## 7. 接下来按 Push 拆分

| 顺序 | Push | 文件 ownership | 完成条件 |
| --- | --- | --- | --- |
| 0 | `docs: establish V2.1 closeout hardness` | `AGENTS.md`、V2.1 计划、文档索引、`CHANGE_LOG.md` | Gate 0 通过；不改生产代码 |
| 1（已完成） | `fix(web): isolate legacy replay and optional errors` | 新普通首页、Dashboard caller、query hooks、路由；不碰 Rust 公式 | 普通首页不请求旧回放；旧回放仅在 Lab 手动触发；可选失败不造成全局错误 |
| 2（已完成） | `refactor(server): decouple market and paper broker capabilities` | `apps/server/src/{main,config}.rs`、必要的 `ApiState` capability 契约 | 无 broker 或 broker 失败时核心启动；真实失败不伪装 Mock |
| 3（已完成） | `build(storage): make PostgreSQL opt in` | workspace Cargo、storage modules/exports/tests | 默认依赖图无 SQLx postgres；显式 feature 仍编译 |
| 4 | `feat(execution): add manual execution journal` | 新 domain/service、SQLite migration/repository、API、API 文档 | append-only executed/skipped/partial；DecisionRecord 不变 |
| 5 | `feat(web): connect minimal Today and Plan flow` | `/personal` 或 `/today`、最小 Plan、Decision detail、React Query hooks | Fixed DCA 真实创建、真实建议、真实手工留痕、真实历史 |
| 6 | `test(product): run M1 user-task validation` | 验收记录，不扩展功能 | 3–5 位用户证据和 Go/Adjust/Stop 决策 |
| 7 | 条件 Push | M2 数据、回测与两策略对比 | 仅在 Gate 3 支持后创建 |

Push 1–3 的共享文件 ownership 必须在开始前再次核对；`apps/server/src/main.rs`、`config.rs` 与 `ApiState` 发生重叠时串行，不并发修改。

## 8. 当前明确不做

- Gate 1 不新增 migration、不运行数据导入；
- 不安装或接入 Financial-API Skill、MCP、CLI、Python SDK 或 DuckDB；
- 不把当前前端演示策略改名后当成真实策略；
- 不先建立三到五个策略、200 日均线策略、股债组合或更多专业指标；
- 不做 IBKR、QMT、Futu/moomoo 自动下单；
- 不启动 Tauri、多 Agent 全路线并行或 V3 功能。

## 9. 每个实现 Push 的统一交付格式

每个 Push 必须交付：

1. 一句话 Goal；
2. 真实 Current state 与调用路径；
3. 可判定 true/false 的 Acceptance criteria；
4. Explicit non-goals；
5. 文件 ownership 与回滚方式；
6. 行为测试和实际运行命令；
7. `CHANGE_LOG.md` 记录；
8. 未验证项、剩余风险与是否允许进入下一 Gate。
