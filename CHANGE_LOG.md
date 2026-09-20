# Change Log

## Unreleased

### 2026-09-20 20:30 AEST — 参数化命名与计划入口收口

- 执行模型：GPT-5 Codex。
- 变更类型：策略目录展示契约、我的计划信息架构、策略卡操作层级、公开 API 文档与聚焦测试。
- 涉及文件：`crates/api/src/{official_strategies.rs,routes/strategy_catalog.rs}`、`crates/api/tests/strategy_catalog.rs`、`apps/web/src/{components/v2_1/strategy-card.tsx,pages/{plans/index.tsx,plans/minimal-plan.test.tsx,strategy-center/index.tsx,v2_1-shell.test.tsx}}`、`apps/web/PLAN.md`、`docs/{plans/v2_1_productization_plan.md,reference/api-management.md}`、`CHANGE_LOG.md`。
- 变更内容：保留全部不可变 policy ID、版本和内部公式名称，同时为目录增加由实际窗口生成的参数化展示名；`preset.name` 从“灵敏 / 稳健 / 长期”等定性档位改成 `20日`、`20/50日` 或 `趋势150日 / 波动63日` 等可核验参数，`价格与指数均线` 简化为 `价格与指数`。我的计划页移除重复的 100 张策略选择卡，改为在“你的长期计划”标题旁常驻“建立新计划”并跳转策略中心；只有策略中心携带精确 policy 深链返回时才显示单一配置表。策略中心把分析动作降为左侧文本链接，将建立计划的主按钮固定到卡片右下角；计划卡的策略说明同步读取目录新名称，但保留用户已有计划标题。
- 技能影响：`frontend-design` 用于压缩重复入口并拉开“分析 / 建立”的视觉层级；`vercel-react-best-practices` 用于保持目录数据由 React Query 管理、移除不再需要的页面内选择状态，并继续使用长列表 `content-visibility`。
- 验证：`cargo fmt --all -- --check`、`cargo test -p indexlink-api official_strategies`、`cargo test -p indexlink-api --test strategy_catalog`、`cargo test -p core-domain`、`pnpm --dir apps/web lint`、`pnpm --dir apps/web test:coverage`（54 项；Statements 93.75%、Branches 90.58%、Functions 93.53%、Lines 96.26%）、`pnpm --dir apps/web build` 与 `git diff --check` 通过；本地真实 API 浏览器核验确认计划页不再出现策略卡墙，策略中心参数名与操作区层级符合预期。

### 2026-09-20 AEST — Push 5：百预设策略中心与任意策略真实分析

- 执行模型：GPT-5 Codex（多 Agent：策略中心与动态分析分文件并行；主线程整合深链状态、移除旧演示回退、补足覆盖率和计划文档）。
- 变更类型：前端策略目录信息架构、真实 policy 导航与回测选择、API 类型、聚焦测试和 V2.1 计划状态。
- 涉及文件：`apps/web/src/{api/types.ts,features/v2_1/{model.ts,model.test.ts},pages/{strategy-center/index.tsx,strategy-analysis/index.tsx,v2_1-shell.test.tsx},stores/ui.ts}`、`apps/web/PLAN.md`、`docs/plans/v2_1_productization_plan.md`、`CHANGE_LOG.md`。
- 变更内容：策略中心不再平铺 101 张同质卡片，而把 Fixed DCA 单独作为共同基准，将 100 个 Formula 预设聚合为 20 个方法家族；支持规则/描述/标签搜索、类别筛选、家族内 5 档参数选择，并展示风险、最小日线、服务端校验、公开来源和限制。分析与建计划链接始终携带所选不可变 policy ID；策略分析移除三策略前端白名单，任意目录版本均可进入真实回测并最多比较三条，名称来自服务端、颜色按 policy ID 稳定生成，未知/下线 ID 明确失败且不降级成 DCA。URL 深链只初始化一次，之后尊重用户的比较选择。删除已无生产调用的三策略本地演示曲线和未知策略 DCA 回退。
- 技能影响：`frontend-design` 用于维持低饱和、以家族而非卡片墙组织信息的视觉层级；`vercel-react-best-practices` 用于保持 React Query 管理目录/回测服务端状态、Valtio 只保存临时选择，并用 memo 与 `content-visibility` 控制大目录渲染。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test:coverage`（54 项；Statements 93.88%、Branches 90.50%、Functions 93.65%、Lines 96.55%）、`pnpm --dir apps/web build`、`cargo test -p core-domain` 与 `git diff --check` 通过；生产构建仅保留既有主 chunk 大小提示。

### 2026-09-20 AEST — Push 4：100 个预设的批量质量门

- 执行模型：GPT-5 Codex（多 Agent；质量门 Agent 落地测试，主线程根据实测耗时将固定样本准入压缩为每个家族一档，并完成整体验证）。
- 变更类型：策略批量准入、重复规则检测、动态多市场建计划与真实回测回归测试。
- 涉及文件：`crates/api/src/official_strategies.rs`、`crates/api/tests/{strategy_catalog.rs,strategy_backtests.rs}`、`CHANGE_LOG.md`。
- 变更内容：对全部 100 个 Formula 预设生成仅含规则的规范化指纹，拒绝重复规则结构；逐一验证无固定金额动作、当期 1,000 预算安全，并从 20 个家族各取均衡档运行完整固定样本准入，要求两项资产均可计算且只影响弹性桶。新增预设 `dsl_price_sma_responsive@1` 进一步实际走通 US/HK/SH/SZ 四市场计划创建，以及 SH.600519 的 provider 日线真实回测，证明批量目录不是仅前端可见的静态条目。既有动态回测测试继续锁定相同共同窗口、执行日收盘价不可前视，以及回测与新计划一致的当期预算失效语义。
- 验证：`cargo fmt --all -- --check`、`cargo test -p indexlink-api official_strategies --lib`（含 20 家族完整准入，约 14 秒）、`cargo test -p indexlink-api --test strategy_catalog --test strategy_backtests`、`cargo test -p strategy-evaluation dynamic_backtest`、`cargo test -p core-domain` 与 `git diff --check` 通过。

### 2026-09-20 AEST — Push 3：20 个策略家族与 100 个不可变预设

- 执行模型：GPT-5 Codex（按用户要求启用多 Agent；主线程完成后端策略工厂、目录契约和验证，前端 Agent 并行准备分组浏览）。
- 变更类型：官方 Formula 策略工厂、批量版本注册、策略目录元数据、公开 API 文档与聚焦测试。
- 涉及文件：`crates/api/src/{official_strategies.rs,routes/strategy_catalog.rs}`、`crates/api/tests/strategy_catalog.rs`、`docs/reference/api-management.md`、`CHANGE_LOG.md`。
- 变更内容：在统一注册表上加入 20 个可解释的中长线规则家族，每个家族提供灵敏、偏短期、均衡、稳健、长期 5 档不可变参数，共生成 100 个 Formula V1 策略版本；覆盖 SMA/EMA 趋势、均线交叉、绝对/双周期动量、RSI、价格分位、波动率、回撤和受限复合规则。原有 MA200 与增长/波动 policy ID、版本和公式保持不变；每个预设提供家族、参数档、标签、公开思想来源、适配声明和所需日线数。新增预设只使用通用价格指标，不绑定 SPY/VOO、不复制第三方交易代码，也不突破计划当期预算。目录对 98 个新增预设返回轻量 `compiled_formula` 元数据，按需读取精确公式和执行真实标的预检，避免一次响应重复携带大型研究结果。
- 验证：`cargo fmt --all -- --check`、`cargo check -p indexlink-api`、`cargo test -p indexlink-api official_strategies --lib`（4 项注册表/重建测试）和 `cargo test -p indexlink-api --test strategy_catalog`（6 项目录/动态市场计划测试）通过；`git diff --check` 通过。

### 2026-09-20 AEST — Push 2：回测与真实计划资金契约一致

- 执行模型：GPT-5 Codex（多 Agent 并行审计，主线程完成共享工作区实现与验证）。
- 变更类型：Formula 回测语义、指标预热不变量、聚焦测试与计划文档。
- 涉及文件：`crates/strategy-dsl/src/lib.rs`、`crates/strategy-evaluation/src/dynamic_backtest.rs`、`docs/plans/dynamic_backtest_push2.md`、`CHANGE_LOG.md`。
- 变更内容：修复 RSI `N` 日变化实际需要 `N + 1` 个收盘价、而预检少报一根的错误；动态回测不再固定使用 `carry_forward` 与 1.5 倍单次上限，改为与当前消费级建计划请求一致的 `expire_each_period` 和本期基础预算硬上限，因此未使用的机会预算不会在回测中滚存，超过 1.0 的机会桶倍率也不会产生真实计划无法执行的虚假加码。
- 验证：`cargo fmt --all -- --check`、`cargo test -p strategy-dsl`、`cargo test -p strategy-evaluation`、`cargo test -p core-domain` 与 `git diff --check` 通过；新增 RSI 15 根预热和 1.2 倍动作被真实单期上限约束的回归测试。

### 2026-09-20 AEST — Push 1：统一官方策略注册表

- 执行模型：GPT-5 Codex（按用户要求启用多 Agent；主线程在共享工作区整合后端注册表改造）。
- 变更类型：后端策略注册、目录与回测准入去硬编码、聚焦测试。
- 涉及文件：`crates/api/src/{official_strategies.rs,routes/{strategy_catalog.rs,strategy_backtests.rs}}`、`CHANGE_LOG.md`。
- 变更内容：新增包含 policy/version、产品文案、风险、数据需求、默认计划与 Formula 构造器的单一官方策略描述符注册表；`GET /strategy-catalog`、真实回测策略解析、保留 ID、官方存储映射和计划目录判断均从同一注册表派生，未知策略继续失败关闭；现有 Fixed DCA、MA200 与增长/波动策略的 HTTP 字段和行为保持兼容，为后续批量预设生成留下统一扩展点。
- 验证：`cargo fmt --all -- --check`、`cargo test -p indexlink-api official_strategies --lib`、`cargo test -p indexlink-api --test strategy_catalog --test strategy_backtests` 通过（2 项注册表单测、10 项目录/回测集成测试）；`git diff --check` 通过。

### 2026-09-19 AEST — 动态多市场计划与 Formula 创建前预检

- 执行模型：GPT-5 Codex（按用户要求启动多 Agent 分工；三个子 Agent 因并发额度中断，主线程逐项审查其残留改动并完成实现。使用 `frontend-design` 保持现有低饱和计划页风格，使用 `vercel-react-best-practices` 保持 React Query 服务端状态与事件驱动表单边界）。
- 变更类型：官方策略目录契约、动态标的计划创建、Formula 行情预检、计划页市场/币种体验、小幅评估节奏改进、公开 API 文档与 V2.1 计划状态收口。
- 涉及文件：`crates/api/src/{official_strategies.rs,routes/{investment_plans.rs,strategy_catalog.rs}}`、`crates/api/tests/strategy_catalog.rs`、`apps/web/src/{api/types.ts,pages/{plans/{index.tsx,minimal-plan.test.tsx},personal/{index.tsx,personal-execution.test.tsx},v2_1-shell.test.tsx}}`、`apps/web/PLAN.md`、`docs/{plans/v2_1_productization_plan.md,reference/api-management.md}`、`CHANGE_LOG.md`。
- 变更内容：删除官方 Formula 的 `SPY/VOO` 静态准入列表；`GET /strategy-catalog` 改为声明 US/HK/SH/SZ 支持市场与策略所需的最小日线观测数，并仅保留始终为空的弃用 `supported_symbols`。计划创建由服务端解析 market-qualified symbol、复核市场币种并规范化存储；Fixed DCA 继续在无行情 provider 时可用，Formula 则在落库前通过同一 `HistoricalPriceProvider` 检查最小窗口与十日内最近数据，历史不足/过期返回 `400`，provider 或数据集不可用返回 `503`，不回退 DCA、不保存半成品计划。计划页按代码动态显示 USD/HKD/CNY、数据需求与可理解错误，Formula 将执行文案改为评估文案且周度评估只提供工作日；完整的资金周期/观察频率、交易日历、节假日和周期级幂等仍明确列入后续。同步将策略先行建计划和个人中心方法/预算/下一评估日展示纳入本次提交。
- 验证：`cargo fmt --all -- --check`、`cargo test -p core-domain --locked`（13 项）、`cargo test -p indexlink-api --locked`（含 6 项动态目录/计划集成测试）、`cargo clippy -p indexlink-api --all-targets --locked -- -D warnings`、`pnpm --dir apps/web lint`、`pnpm --dir apps/web test:coverage`（49 项；Statements 92.61%、Branches 90.04%、Functions 92.26%、Lines 96.22%）、`pnpm --dir apps/web build` 与 `git diff --check` 通过；生产构建仅保留既有主 chunk 大小提示。

### 2026-09-19 AEST — 多策略建计划与个人中心计划契约

- 执行模型：GPT-5 Codex（使用 `frontend-design` 技能重整策略先行的建计划层级，使用 `vercel-react-best-practices` 技能保持 React Query 服务端目录状态与事件驱动表单边界）。
- 变更类型：我的计划创建流程、个人中心策略说明、资金边界、失败态与前端聚焦测试。
- 涉及文件：`apps/web/src/pages/{plans/{index.tsx,minimal-plan.test.tsx},personal/{index.tsx,personal-execution.test.tsx},v2_1-shell.test.tsx}`、`CHANGE_LOG.md`。
- 变更内容：“我的计划”新建区改为先读取服务端官方策略目录并显式选择策略，未选择时不再静默创建 Fixed DCA；不可采用、空目录和目录不可用均保持明确状态。选中后才展开该策略允许的标的、每期基础预算和评估节奏，创建请求始终冻结目录 policy/version 和资金桶边界。个人中心将原“计划摘要”扩展为计划方法契约：显示官方方法、限制、本期/下一评估日、基础预算与执行节奏；Fixed DCA 显示确定金额，Formula 明确拆分固定核心额度、最大弹性额度和单次上限，并说明实际建议只在评估日由当日规则计算。策略目录暂时不可用时使用已冻结计划中的 policy 与资金桶进行保守降级展示，不影响建议和执行历史主链路。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test -- --run`（47 项通过）、`pnpm --dir apps/web test:coverage`（Statements 92.43%、Branches 90.18%、Functions 91.71%、Lines 96.21%）、`pnpm --dir apps/web build`、`cargo test -p core-domain --locked`（13 项通过）与 `git diff --check` 通过；同时在本地实际页面验证未选择/已选择建计划状态与 Formula 个人中心金额边界。生产构建仅保留既有主 chunk 大小提示。

### 2026-09-19 AEST — 策略运行时行情依赖解耦（回测 Push 5）

- 执行模型：GPT-5 Codex（多 Agent；子 Agent 留下初版后由主线程审查、补齐生产装配、公开契约与验证）。
- 变更类型：Formula 实时证据、最小数据依赖、运行能力状态、失败语义、API 文档与聚焦测试。
- 涉及文件：`crates/market-data/src/{history.rs,alpaca.rs,opend_history.rs}`、`crates/api/src/{state.rs,routes/{decision_preview.rs,runtime_status.rs,strategy_backtests.rs}}`、`crates/api/tests/{decision_preview.rs,health.rs,strategies.rs,strategy_backtests.rs}`、`apps/server/src/main.rs`、`apps/web/src/api/types.ts`、`docs/{plans/runtime_data_decoupling_push5.md,reference/api-management.md}`、`CHANGE_LOG.md`。
- 变更内容：自动决策按策略类型读取最小数据依赖：Fixed DCA 继续完全离线；官方 Formula 直接通过统一 `HistoricalPriceProvider` 获取自身指标所需的有界规范化日线，不再先请求旧 CAPE、国债与 VIX 整包；旧 `core_opportunity_v1` 保留原兼容链路。Formula 成功证据保存 provider、dataset version、checksum、复权、区间与证据截止日；历史数据未配置、供应商失败、样本不足或需要尚未独立建模的 VIX 时明确返回 `503`，不生成 `waiting` 或任何 DecisionRecord。新增 provider-neutral 的推荐复权能力，Alpaca 美股声明 `all`、OpenD 各市场声明前复权，修复生产 OpenD 被 API 固定美股 `all` 请求拒绝的问题。`/runtime-status` 新增独立 `historical_prices` 三态，并让生产 server 在已配置 adapter 初始化失败时正确报告 `unavailable`；TypeScript 契约和 API 文档同步更新。Scheduler 当前仍按 UTC 日期运行，本 Push 不引入 plan timezone 或交易日历迁移。
- 验证：`cargo test -p market-data --locked`（14 项通过、1 项真实 OpenD/公网 smoke 忽略；两项 loopback 协议测试在沙箱外通过）、`cargo test -p indexlink-api --locked`（含 Formula 仅调用历史价格、数据失败不落记录、策略激活与七档真实回测）、`cargo test -p indexlink-server --locked`（37 项通过、1 项真实下单 smoke 忽略）、`cargo test -p core-domain --locked`（13 项）、`cargo clippy -p market-data -p indexlink-api -p indexlink-server --all-targets --locked -- -D warnings`、`pnpm --dir apps/web lint`、`pnpm --dir apps/web test -- --run`（42 项）、`pnpm --dir apps/web build`、`cargo fmt --all -- --check` 与 `git diff --check` 通过；前端构建仅保留既有主 chunk 大小提示。

### 2026-09-19 AEST — 自选标的真实回测前端（回测 Push 4）

- 执行模型：GPT-5 Codex（使用 `frontend-design` 保持现有低饱和消费级视觉，使用 `vercel-react-best-practices` 约束 React Query 服务端状态、Valtio 本地筛选和派生图表数据）。
- 变更类型：策略分析真实 API 接入、自选标的、七档范围、归一化图表、专业指标、来源披露、错误态与前端测试。
- 涉及文件：`apps/web/src/{api/{queries.ts,types.ts},features/v2_1/model.ts,pages/{strategy-analysis/index.tsx,v2_1-shell.test.tsx},stores/ui.ts}`、`apps/web/{vite.config.ts,vite.config.test.ts,vitest.config.ts,PLAN.md}`、删除的 `apps/web/src/components/v2_1/professional-research-panel.tsx`、`CHANGE_LOG.md`。
- 变更内容：策略分析页删除本地确定性演示曲线，改为调用 `POST /strategy-backtests`；支持输入 `US/HK/SH/SZ` 自选标的、选择 1–3 条官方策略，以及 1m/3m/6m/1y/3y/5y/all。直观视角展示同一共同窗口的真实归一化日线和区间摘要，专业视角直接展示同一响应的年化收益、XIRR、最大回撤、波动率、Sortino 与现金使用率。新增紧凑的数据来源条，公开 provider、数据版本、复权、checksum、币种、时区和有效范围；503/400 均明确说明且绝不回退静态图。服务端数据由 React Query 管理，草稿标的/范围/策略/视角与最近提交请求由 Valtio 管理。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test -- --run`（42 项通过）、`pnpm --dir apps/web test:coverage`（Statements 91.83%、Branches 90.07%、Functions 91.08%、Lines 95.82%）和 `pnpm --dir apps/web build` 通过；生产构建仅保留既有主 chunk 大小提示。

### 2026-09-19 AEST — 任意标的真实策略回测 API（回测 Push 3）

- 执行模型：GPT-5 Codex（多 Agent；本 Agent 负责 HTTP 契约、历史行情 port 注入、路由测试和 API 文档）。
- 变更类型：产品回测 API、可选能力注入、数据来源披露、范围/请求校验与聚焦测试。
- 涉及文件：`crates/api/src/{state.rs,routes/{mod.rs,strategy_backtests.rs}}`、`crates/api/tests/strategy_backtests.rs`、`apps/server/src/main.rs`、`crates/strategy-evaluation/src/dynamic_backtest.rs`、`docs/reference/api-management.md`、`CHANGE_LOG.md`。
- 变更内容：新增 `POST /strategy-backtests`，一次接收一个市场限定 symbol、1–3 个唯一官方策略、`1m/3m/6m/1y/3y/5y/all` 范围、1–28 日的月度检查日与十进制投入金额。ApiState 通过 `Arc<dyn HistoricalPriceProvider>` 显式注入可选历史行情能力；生产 server 将已配置的只读 OpenD 行情与 SQLite 精确快照缓存组合为真实 provider。路由读取同一带来源/复权/版本/checksum 的日线快照，解析官方 Formula 版本并调用纯 `run_dynamic_backtest`，返回共同有效窗口、真实归一化轨迹、指标和完整无密钥 provenance。回测金额字段不绑定 USD，按响应的标的交易币种解释且不做隐式汇兑。未配置/失败的数据源显式返回既有 `503`，非法或历史不足请求返回既有 `400`，不调用 broker、不使用旧 MA200 回放，也不伪造成功数据。
- 验证：`cargo test -p indexlink-api --locked`（含 4 项新回测集成测试，覆盖全部 7 个范围、三策略共同窗口、HK 复权/币种元数据、请求拒绝和 provider 不可用）、`cargo test -p indexlink-server --locked`（37 项通过、1 项真实下单 smoke 忽略；回环启动测试在沙箱外通过）、`cargo test -p strategy-evaluation --locked`（19 项）、`cargo test -p core-domain --locked`（13 项）、`cargo clippy -p indexlink-api -p indexlink-server --all-targets --locked -- -D warnings`、`cargo fmt --all -- --check` 与 `git diff --check` 通过。

### 2026-09-19 AEST — 任意标的统一 Formula 回测内核（回测 Push 2）

- 执行模型：GPT-5 Codex（主线程实现；多 Agent 并行负责行情数据层、HTTP 契约与运行时解耦）。
- 变更类型：纯函数回测、因果证据、统一现金流、归一化轨迹、专业指标与聚焦测试。
- 涉及文件：`crates/strategy-evaluation/src/{lib.rs,dynamic_backtest.rs}`、`docs/plans/dynamic_backtest_push2.md`、`CHANGE_LOG.md`。
- 变更内容：新增与 symbol 和供应商无关的 `run_dynamic_backtest`，接收调用方提供的复权日线，在完全相同的标的、数据快照、月度现金流、5 bps 成本和共同有效窗口中比较 Fixed DCA 与受限 Formula V1。Formula 决策只能读取模拟成交日之前的收盘价；比较必须等所有策略完成指标预热，不能通过默认值或少投周期制造优势。输出共同起止日期、投入次数、每日时间加权归一化指数，以及区间/年化收益、XIRR、最大回撤、年化波动率、Sortino、投入、现金使用率和期末资产。仅有价格时显式拒绝 VIX 等外部指标和精确金额动作，不以 0 冒充真实数据。
- 验证：`cargo test -p strategy-evaluation dynamic_backtest --no-fail-fast`（3 项通过）、`cargo clippy -p strategy-evaluation --all-targets -- -D warnings` 与 `git diff --check` 通过；全 workspace fmt/回归在并行 API 文件冻结后统一执行。

### 2026-09-19 AEST — 任意标的历史日线来源与本地可复现缓存（回测 Push 1）

- 执行模型：GPT-5 Codex（多 Agent：本 Agent 负责独立行情数据层与 SQLite 缓存；官方文档和兼容许可证开源项目仅作契约/架构参考）。
- 变更类型：历史行情 port、Alpaca/OpenD adapter、SQLite migration、本地缓存、数据来源审计与聚焦测试。
- 涉及文件：`crates/market-data/{Cargo.toml,src/{lib.rs,history.rs,alpaca.rs,opend_history.rs}}`、`crates/storage/{Cargo.toml,src/{lib.rs,sqlite.rs,sqlite_market_price_history.rs}}`、`migrations/sqlite/20260919120000_create_market_price_history.sql`、`docs/plans/market_data_push1.md`、`Cargo.lock`、`CHANGE_LOG.md`。
- 变更内容：新增与供应商无关的 `HistoricalPriceProvider`、`PriceHistoryStore` 和严格校验的日线数据集契约，统一保存 market、symbol、instrument type、currency、timezone、adjustment、requested range、provider、dataset version、fetched-at 与 SHA-256。Alpaca adapter 仅支持美股，显式使用 `1Day`、feed、复权和 `next_page_token`；API key 不进入存储或 Debug。OpenD adapter 仅连接字面 loopback，按官方枚举支持 `US.*`、`HK.*`、`SH.*`、`SZ.*` 及 raw/QFQ/HFQ。SQLite 按 provider + instrument + adjustment + exact range 原子保存独立快照，命中时不访问远端，不跨来源、区间或复权方式混拼。既有 `MarketSignalProvider` 保持兼容。
- 外部参考：Alpaca 与 Futu OpenD 官方文档作为协议语义来源；参考 `wmzhai/alpaca-data-rs`（MIT OR Apache-2.0）的凭据脱敏与分页边界，不复制实现；`d-e-s-o/apca`（GPL-3.0）仅作生态调研，未复制代码。
- 验证：`cargo test -p market-data --locked`（14 项通过、1 项需真实 OpenD/公网的 smoke 忽略；其中 2 项 loopback 协议测试由主线程在沙箱外验证）、`cargo test -p indexlink-storage --locked`（34 项通过）、`cargo fmt -p market-data -p indexlink-storage -- --check`、`cargo clippy -p market-data -p indexlink-storage --all-targets --locked -- -D warnings`、`cargo test -p core-domain --locked` 与 `git diff --check` 通过。最终全 workspace fmt check 需在并行 Push 2/API 文件冻结后由主线程统一执行。

### 2026-09-19 AEST — 计划删除确认与策略分析导航修正

- 执行模型：GPT-5 Codex（使用 `frontend-design` 与 `vercel-react-best-practices` 技能约束站内确认层、卡片导航语义与服务端状态边界）。
- 变更类型：计划管理交互、策略分析深链、演示数据披露与前端聚焦测试。
- 涉及文件：`apps/web/src/{components/v2_1/strategy-card.tsx,pages/{plans/{index.tsx,minimal-plan.test.tsx},strategy-center/index.tsx,strategy-analysis/index.tsx,v2_1-shell.test.tsx}}`、`CHANGE_LOG.md`。
- 变更内容：“我的计划”删除操作不再调用浏览器原生确认框，改为带遮罩、计划名、不可恢复范围和券商边界说明的站内确认层；用户确认前不会发送 DELETE，删除进行中锁定关闭与重复提交。策略中心的策略主卡改为语义化链接，直接携带策略 ID 进入“策略分析 / 直观视角”，不再保留没有后续作用的“正在查看”局部状态；采用按钮仍作为独立入口进入建计划。直观视角顶部新增醒目的“演示数据 · 非真实回测”标记，并明确曲线来自前端确定性公式、不连接市场行情；专业研究继续显示后端目录返回的真实固定样本聚合指标。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test -- --run`（41 项通过）、`pnpm --dir apps/web test:coverage`（Statements 92.18%、Branches 90.18%、Functions 92.16%、Lines 96.37%）、`pnpm --dir apps/web build`、`cargo test -p core-domain --locked`（13 项通过）、`git diff --check` 通过；生产构建仅保留既有主 chunk 大小提示。

### 2026-09-19 AEST — 修复策略目录开发代理

- 执行模型：GPT-5 Codex。
- 变更类型：Vite 本地开发代理与回归测试。
- 涉及文件：`apps/web/vite.config.ts`、`apps/web/vite.config.test.ts`、`CHANGE_LOG.md`。
- 变更内容：将新增的 `/strategy-catalog` 正式加入 Vite → `127.0.0.1:8080` 代理清单，并抽取可直接测试的不可变代理映射，避免前端开发服务器把 API 路径回退为 HTML 后再以 JSON 解析失败。按用户要求核对并正常终止旧 `target/debug/indexlink-server` 进程 PID 91955；未删除数据库或其他本地数据。
- 验证：确认 `lsof -nP -iTCP:8080 -sTCP:LISTEN` 无监听进程；`pnpm --dir apps/web lint`、`pnpm --dir apps/web test`（40 项通过）、`pnpm --dir apps/web test:coverage`（Statements 92.57%、Branches 91.22%、Functions 92.99%、Lines 96.56%）、`pnpm --dir apps/web build`、`cargo test -p core-domain --locked`（13 项通过）完成；生产构建仅保留既有主 chunk 大小提示。

### 2026-09-19 AEST — 官方策略采用到个人计划闭环（Push 5）

- 执行模型：GPT-5 Codex（使用 `frontend-design` 与 `vercel-react-best-practices` 技能保持既有视觉、React Query 服务端状态与事件驱动表单边界；多 Agent 尝试因并行额度中断后由主线程完成）。
- 变更类型：策略采用路由、真实计划创建、服务端标的白名单、计划文案、API/前端计划文档与聚焦测试。
- 涉及文件：`apps/web/src/pages/plans/{index.tsx,minimal-plan.test.tsx}`、`crates/api/src/{official_strategies.rs,routes/investment_plans.rs}`、`crates/api/tests/strategy_catalog.rs`、`docs/reference/api-management.md`、`apps/web/PLAN.md`、`CHANGE_LOG.md`。
- 变更内容：策略卡的 policy ID/version 通过查询参数进入“我的计划”，页面重新读取官方目录核对版本、准入、支持标的与默认桶配置后才允许提交。Fixed DCA 保持 100% 核心桶；MA200 与增长/波动公式冻结为 70% 核心、30% 弹性和 approval，成功后继续复用真实 `POST /investment-plans` 与自动建议准备链路。前后端同时拒绝官方 Formula 使用目录外标的；创建、修改策略版本与激活既有计划均不能绕过服务端 SPY/VOO 白名单。旧自适应计划保留只读兼容标签，但不再提供新建入口。
- 验证：`cargo fmt --all -- --check`、`cargo test -p core-domain --locked`（13 项）、`cargo test -p strategy-dsl --all-features --locked`（16 项）、`cargo test -p strategy-evaluation --locked`（16 项）、`cargo test -p indexlink-api --locked`（含 3 项目录/采用集成测试）、`cargo clippy -p indexlink-api --all-targets --locked -- -D warnings`、`pnpm --dir apps/web lint`、`pnpm --dir apps/web test:coverage`（39 项通过；Statements 92.57%、Branches 91.22%、Functions 92.99%、Lines 96.56%）、`pnpm --dir apps/web build` 与 `git diff --check` 通过；构建仅保留既有主 chunk 大小提示。

### 2026-09-19 AEST — 策略中心接入真实官方目录（Push 4）

- 执行模型：GPT-5 Codex（使用 `frontend-design` 与 `vercel-react-best-practices` 技能约束信息层级、交互反馈与服务端状态边界；多 Agent 尝试因并行额度中断后由主线程完成）。
- 变更类型：React Query 数据接入、消费级策略卡、真实专业研究、静态自适应入口隐藏与前端聚焦测试。
- 涉及文件：`apps/web/src/{api/{types.ts,queries.ts},components/v2_1/{strategy-card.tsx,professional-research-panel.tsx},features/v2_1/{model.ts,model.test.ts},pages/{strategy-center/index.tsx,v2_1-shell.test.tsx},stores/ui.ts}`、`CHANGE_LOG.md`。
- 变更内容：策略中心改读 `GET /strategy-catalog`，删除静态自适应/股债卡与演示年化、回撤；卡片只显示服务端规则、限制、固定样本范围和准入状态，整卡浏览继续保持柔和选中反馈。只有 `adoptable` 策略显示醒目的“用这个策略建立计划”，并通过 policy ID/version 导向“我的计划”。专业视角改为直接读取目录附带的真实 admission 报告，不再要求用户先在高级实验室保存 DSL；Fixed DCA 保持同口径对照列。普通分析示例中的旧 70/20/10 也替换为 MA200 与增长/波动 Formula 名称，避免隐藏入口从其他页面重新出现。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test`（37 项通过）、`pnpm --dir apps/web test:coverage`（Statements 93.10%、Branches 90.71%、Functions 92.90%、Lines 96.94%）、`pnpm --dir apps/web build`、`cargo test -p core-domain --locked`（13 项通过）、`git diff --check` 通过；构建仅保留既有主 chunk 大小提示。

### 2026-09-19 AEST — 官方 Formula V1 策略目录（Push 3）

- 执行模型：GPT-5 Codex（多 Agent 尝试因并行额度中断，主线程接管审查与完成）。
- 变更类型：官方策略注册、消费级目录 API、固定样本准入、不可变版本保护、API 文档与集成测试。
- 涉及文件：`crates/api/src/{lib.rs,state.rs,official_strategies.rs,routes/{mod.rs,strategy_catalog.rs}}`、`crates/api/tests/strategy_catalog.rs`、`docs/reference/api-management.md`、`CHANGE_LOG.md`。
- 变更内容：新增 `GET /strategy-catalog`，只发布 Fixed DCA、200 日均线趋势保护、增长与波动平衡三个版本；两条 Formula V1 策略由服务端通过领域构造器生成，只调整 30% 弹性桶，并在响应中附规范公式和真实固定样本 admission。官方 DSL 版本接入既有读取、准入与运行时解析路径，同时拒绝本机策略覆盖同一 ID/version。依赖 AI 降级口径的 `core_opportunity_v1` 不进入普通目录，但未删除历史运行兼容。
- 验证：`cargo test -p core-domain --locked` 通过（13 项）；`cargo test -p indexlink-api --locked` 全部通过（含 2 项新目录集成测试）；`cargo clippy -p indexlink-api --all-targets --locked -- -D warnings` 通过；`cargo fmt --all` 与 `git diff --check` 完成。

### 2026-09-19 AEST — Formula V1 统一研究与实时证据（Push 2）

- 执行模型：GPT-5 Codex（多 Agent 尝试因并行额度中断，主线程接管审查与完成）。
- 变更类型：策略证据契约、实时行情窗口、研究/运行一致性与聚焦测试。
- 涉及文件：`crates/strategy-dsl/src/lib.rs`、`crates/api/src/routes/decision_preview.rs`、`CHANGE_LOG.md`。
- 变更内容：为策略规格增加最少收盘价观察数推导；收益率和波动率按 `window + 1`，其他滚动指标按完整窗口申请数据。实时 Decision Preview 不再只允许 RSI/VIX，也不再使用固定 366 天窗口，而是按策略最长指标窗口请求保守日历天数，并复用准入回测相同的因果 `DslEvidence` 构造器；预热不足继续安全拒绝，不补默认值、不读取未来数据。
- 验证：`cargo test -p strategy-dsl --all-features --locked` 通过（16 项）；`cargo test -p strategy-evaluation --locked` 通过（16 项）；`cargo test -p indexlink-api --locked --test decision_preview` 通过（15 项）；`cargo fmt --all` 完成。

### 2026-09-19 AEST — Formula V1 通用指标领域契约（Push 1）

- 执行模型：GPT-5 Codex（多 Agent 尝试因并行额度中断，主线程接管审查与完成）。
- 变更类型：受限策略 DSL、确定性通用指标、serde 契约与领域测试。
- 涉及文件：`crates/strategy-dsl/src/lib.rs`、`CHANGE_LOG.md`。
- 变更内容：在既有 Close/SMA/EMA/RSI/Drawdown/VIX 白名单上增加周期价格收益率、年化历史波动率、价格经验分位与均线距离四类 Formula V1 指标；所有指标使用同一因果收盘价快照和纯 Decimal 计算，窗口不足时安全拒绝，不读取网络或未来数据。同步增加不可变 JSON document 变体及往返重建，继续复用 `LookbackWindow` 不变量和既有 first-match/默认标准机会桶语义。
- 验证：`cargo test -p strategy-dsl --all-features --locked` 通过（15 项）；`cargo fmt --all` 完成。

### 2026-09-19 AEST — 策略中心真实计划来源与浏览态语义修正

- 执行模型：GPT-5 Codex。
- 变更类型：消费级前端状态来源、策略卡交互反馈、低饱和语义配色与聚焦测试。
- 涉及文件：`apps/web/src/{components/v2_1/strategy-card.tsx,pages/{personal/index.tsx,strategy-center/index.tsx,v2_1-shell.test.tsx}}`、`CHANGE_LOG.md`。
- 变更内容：将个人中心未完成建议卡由近黑褐色调亮为仍保持克制的琥珀褐色，并增强边框、状态胶囊与阴影区分；策略中心顶部不再把前端当前查看的静态策略伪装为“正在坚持”，改为读取真实 `GET /investment-plans` 并列出全部 active 计划，点击计划会进入“我的计划”并设置当前查看项。删除策略中心顶部的临时双策略对比入口；策略卡改为整卡可点击，当前浏览卡使用柔和淡绿背景、边框、光晕和“正在查看”反馈，明确浏览状态不会创建或采用计划。移除“选用这个策略 → 个人中心已同步”的虚假前端闭环；由于“稳中有进组合”尚无后端多资产、股债比例及再平衡计划契约，本轮未伪造全策略一键建计划能力。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test`（36 项通过）、`pnpm --dir apps/web test:coverage`（Statements 92.94%、Branches 90.75%、Functions 92.15%、Lines 96.83%）、`pnpm --dir apps/web build`、`cargo test -p core-domain --locked`（13 项通过）。浏览器自动验收因当前 in-app Browser 无法接管新建本地标签页而未完成，未将其记为通过。

### 2026-09-18 AEST — 本期建议待办/完成视觉状态

- 执行模型：GPT-5 Codex。
- 变更类型：个人中心执行状态层级、低饱和语义配色、确认反馈动画与聚焦测试。
- 涉及文件：`apps/web/src/pages/personal/{index.tsx,personal-execution.test.tsx}`、`CHANGE_LOG.md`。
- 变更内容：在本期建议卡标题行加入紧凑的真实状态胶囊，未确认显示“本期待办 · 未完成”，已执行显示“本期待办 · 已完成”，跳过与迁移前 partial 记录分别显示已跳过和历史部分完成；读取失败时保持“执行状态待确认”，不把未知状态伪装成待办。建议卡按 journal 真实结果使用低饱和暖褐警示、深森林绿完成与中性深蓝已处理配色，并以 700ms 背景色、边框和阴影过渡响应确认结果；`prefers-reduced-motion` 下禁用过渡。状态完全由 React Query journal 数据派生，不新增浏览器持久状态。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test`（36 项通过）、`pnpm --dir apps/web test:coverage`（Statements 93.63%、Branches 91.48%、Functions 92.76%、Lines 97.13%）、`pnpm --dir apps/web build`、`cargo fmt --all -- --check`、`cargo test -p core-domain --locked`（13 项通过）。浏览器在 1280×720 下分别确认 completed 为 `rgb(23, 48, 39)`、pending 为 `rgb(40, 36, 29)`、过渡时间为 `0.7s`，页面无横向溢出且控制台无 warning/error。

### 2026-09-18 AEST — 个人中心与“我的计划”执行语义收口

- 执行模型：GPT-5 Codex。
- 变更类型：消费级信息层级、计划管理导航、一次性手工确认约束、兼容迁移、聚焦测试与文档同步。
- 涉及文件：`apps/web/src/{components/{layout/app-sidebar.tsx,v2_1/page-heading.tsx},i18n/locales/{zh.ts,en.ts},pages/{personal/index.tsx,personal/personal-execution.test.tsx,plans/index.tsx,plans/minimal-plan.test.tsx,v2_1-shell.test.tsx}}`、`apps/web/PLAN.md`、`crates/{api/tests/manual_executions.rs,storage/src/sqlite_manual_executions.rs}`、`migrations/sqlite/20260918090000_limit_manual_execution_to_one_per_decision.sql`、`docs/{plans/v2_1_closeout_hardness.md,reference/api-management.md}`、`CHANGE_LOG.md`。
- 变更内容：解除个人中心页头与计划选择器的同排宽度竞争，标题和说明恢复完整内容宽度；新增醒目的“正在查看 + 计划名”上下文。侧边栏在个人中心下新增“我的计划”，`/plans` 改为先展示真实计划详情和管理动作、再建立新 Fixed DCA 计划，个人中心移除误导性的“管理计划”链接。普通执行确认删除“部分执行”，仅保留“我已执行/这次跳过”；前端只对真实 `due` 建议开放操作，读取到任意既有结果后隐藏按钮。新增 SQLite trigger，保证同一 decision 即使更换事件 ID 也只能追加一个最终结果并返回 `409 conflict`；迁移前多条历史及 `partial` API 值保持只读兼容，不删除、不改写原记录。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test`（35 项通过）、`pnpm --dir apps/web test:coverage`（Statements 93.18%、Branches 90.96%、Functions 92.61%、Lines 96.97%）、`pnpm --dir apps/web build`、`cargo fmt --all -- --check`、`cargo test -p core-domain --locked`（13 项通过）、`cargo test -p indexlink-storage --locked`（33 项通过）、`cargo test -p indexlink-api --locked --test manual_executions`（3 项通过）、`git diff --check`。浏览器在 1280×720 下确认页头与说明各为一行、无横向溢出、“正在查看”与 18px 计划选择器形成连续主信息；“我的计划”可从侧边栏进入并先展示真实卡片，既有结果下无“我已执行/部分执行”按钮，控制台无 warning/error。

### 2026-09-17 AEST — Push 5B：Gate 2 最小 Fixed DCA 收口

- 执行模型：GPT-5 Codex。
- 变更类型：普通用户最小建计划流程、Decision detail 执行流水、调度幂等补强、聚焦测试与 Gate 状态更新。
- 涉及文件：`apps/web/src/{api/queries.ts,components/v2_1/manual-execution-history.tsx,pages/{plans/index.tsx,plans/minimal-plan.test.tsx,personal/index.tsx,personal/personal-execution.test.tsx,decisions/index.tsx,decisions/decision-journal.test.tsx}}`、`apps/web/{PLAN.md,vitest.config.ts}`、`crates/api/src/{state.rs,routes/decision_preview.rs}`、`crates/api/tests/decision_preview.rs`、`docs/{reference/api-management.md,plans/v2_1_closeout_hardness.md}`、`CHANGE_LOG.md`。
- 变更内容：将普通 `/plans` 从高级策略/双桶配置收口为标的、金额、月/周周期和可选名称，隐藏并冻结 `fixed_dca@1`、100% 核心桶、fixed risk、当期失效机会资金和单次金额上限；建立后调用真实自动决策接口，成功进入个人中心，计划已保存但决策准备失败时允许只重试建议而不重复建计划。个人中心在无 `due` 建议时改为解释暂停状态或显示真实节奏与下一计划日。抽取只读 append-only 执行时间线供个人中心和 Decision detail 复用，详情现可同时找回不可变原建议与所有用户报告事件。浏览器重启验收发现自动预览与 Scheduler 使用不同幂等账本会生成同日第二条建议；现由成功的 `due` 自动预览补记同一调度 claim，避免下一 tick/重启重复生成并遮蔽已有 journal，不改变决策公式。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test`（34 项通过）、`pnpm --dir apps/web test:coverage`（Statements 93.37%、Branches 91.26%、Functions 92.61%、Lines 97.28%）、`pnpm --dir apps/web build`、`cargo fmt --all -- --check`、`cargo test -p core-domain --locked`（13 项通过）、`cargo test -p indexlink-api --locked --test decision_preview`（15 项通过，含自动预览/调度同日幂等测试）、`cargo test -p indexlink-api --locked --test manual_executions`（3 项通过）、`git diff --check`。浏览器以全新 SQLite 且禁用 AI/OpenD/broker/市场数据完成 `建立 Fixed DCA → 生成 due 建议 → 记录 partial 400 USD → Decision detail 找回 → 服务重启 → 同一 decision 与 journal 仍可找回`。Gate 2 通过，下一阶段仅进行 3–5 位目标用户任务验证。

### 2026-09-17 AEST — Push 5：个人中心真实手工执行闭环

- 执行模型：GPT-5 Codex。
- 变更类型：前端真实 API 映射、人工执行确认交互、append-only 历史展示、可访问状态与产品化文案。
- 涉及文件：`apps/web/src/{api/{types.ts,queries.ts},pages/{personal/index.tsx,personal/personal-execution.test.tsx,v2_1-shell.test.tsx}}`、`apps/web/PLAN.md`、`docs/plans/v2_1_closeout_hardness.md`、`CHANGE_LOG.md`。
- 变更内容：个人中心移除演示金额、静态近期变化与浏览器会话完成状态，改为从真实 plan 和最新 `due` DecisionRecord 展示本期金额、普通话行动解释与计划事实。新增完成、部分执行、跳过三种二次确认，允许填写实际金额、当地发生时间和可选备注；客户端事件 UUID 在当前表单重试期间保持不变，成功后使对应 journal query 失效刷新，`409 conflict` 按已存在记录重新读取。右侧执行历史读取真实 `GET /decisions/:id/manual-executions`，明确标记为用户报告；不存在计划、没有待执行建议、核心 API 失败和 journal 局部失败均不以演示数据填充。后端审计英文串只保留在原建议详情，个人中心不暴露 policy/bucket 实现细节；Decimal 输入默认值只在展示层整理为两位金额。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test`（29 项通过）、`pnpm --dir apps/web test:coverage`（Statements 95.29%、Branches 92.27%、Functions 94.69%、Lines 98.52%）、`pnpm --dir apps/web build`、`cargo test -p core-domain --locked`（13 项通过）、`git diff --check`。浏览器使用临时 SQLite 和真实 Rust API 完成 `Plan → Fixed DCA due Decision → partial 400 USD → POST journal → GET 历史刷新`，390px 视口无横向溢出且控制台无 warning/error。Gate 2 仍待最小 Plan 表单与 Decision detail journal 完成。

### 2026-09-16 AEST — Push 4：Append-only Manual Execution Journal

- 执行模型：GPT-5 Codex。
- 变更类型：执行留痕领域契约、SQLite migration/repository、HTTP API、审计不变量与回归测试。
- 涉及文件：`crates/decision-records/src/lib.rs`、`crates/storage/src/{lib.rs,sqlite.rs,sqlite_manual_executions.rs}`、`crates/api/src/{error.rs,state.rs,routes/{mod.rs,manual_executions.rs}}`、`crates/api/tests/manual_executions.rs`、`migrations/sqlite/20260916090000_create_manual_execution_events.sql`、`docs/{reference/api-management.md,plans/v2_1_closeout_hardness.md}`、`CHANGE_LOG.md`。
- 变更内容：新增与不可变 DecisionRecord 分离的手工执行事件，支持 `executed`、`skipped`、`partial`，实际金额使用精确 decimal 字符串，发生时间规范化为 UTC，来源固定为 `user_reported`。`plan_id` 与 `currency` 由 SQLite 从原 decision 继承；客户端生成非 nil `event_id`，重复追加返回 `409 conflict`。repository 只暴露 append/list，SQLite trigger 拒绝直接修改和仍有关联计划时的直接删除，同时保留既有计划删除级联契约。新增 `POST/GET /decisions/:id/manual-executions`，不调用 broker、不重算策略、不覆盖原建议。
- 验证：`cargo fmt --all -- --check`、`cargo test -p core-domain --locked`（13 项通过）、`cargo test -p decision-records --locked`（14 项通过）、`cargo test -p indexlink-storage --all-features --locked`（47 项通过）、`cargo test -p indexlink-api --locked`（含 3 项新 HTTP 集成测试，全部通过）、`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`cargo test --workspace --locked`（全部通过；网络/真实 OpenD smoke 按设计忽略）、`cargo llvm-cov -p decision-records -p indexlink-storage -p indexlink-api --all-features --summary-only`（新 route 行覆盖 98.31%，新 SQLite repository 行覆盖 98.33%，decision-records crate 行覆盖 94.68%）、`git diff --check`。

### 2026-09-16 AEST — Gate 1 集成验收与状态收口

- 执行模型：GPT-5 Codex（Push 2 / Push 3 采用并行 worktree；子 Agent 使用额度触顶后由主 Agent 完成审查、验证与集成）。
- 变更类型：V2.1 Gate 状态更新与合并后回归验收。
- 涉及文件：`docs/plans/v2_1_closeout_hardness.md`、`CHANGE_LOG.md`。
- 变更内容：将可选能力隔离与 PostgreSQL opt-in 登记为已完成，确认 Gate 1 通过，并把下一项唯一执行目标收束为 Push 4 append-only Manual Execution Journal。
- 验证：`cargo fmt --all -- --check`、`cargo test --workspace --locked`、`cargo test -p indexlink-storage --locked`（31 项通过）、`cargo test -p indexlink-storage --locked --features postgres`（45 项通过）、`cargo clippy -p indexlink-storage --all-targets --all-features --locked -- -D warnings`、默认/显式 PostgreSQL `cargo tree` 断言、`pnpm --dir apps/web lint`、`pnpm --dir apps/web test:coverage`（22 项通过；Statements 95.83%、Branches 90.00%、Functions 92.94%、Lines 99.22%）、`pnpm --dir apps/web build`、`git diff --check`。

### 2026-09-16 AEST — Push 3：PostgreSQL 改为 storage 显式 opt-in

- 执行模型：GPT-5 Codex（由并行子 Agent 实现并提交，主 Agent 负责集成复核）。
- 变更类型：Rust 依赖边界、storage feature 隔离与 CI 回归门禁。
- 涉及文件：`Cargo.toml`、`crates/storage/{Cargo.toml,src/lib.rs}`、`.github/workflows/rust-ci.yml`、`CHANGE_LOG.md`。
- 变更内容：从 workspace 默认 SQLx features 移除 PostgreSQL；`indexlink-storage` 新增默认空 feature 集合，只有显式启用 `postgres` 时才编译 PostgreSQL 连接、repository adapter 与相关测试。SQLite 仍为 server 默认存储，`StorageError` 仍在默认构建中可用，既有 PostgreSQL/SQLite migrations 未改动。CI 新增依赖图断言，防止默认 server 回归引入 `sqlx-postgres`，并确认 storage 显式 feature 仍包含它。
- 验证：`cargo fmt --all -- --check`、`cargo test -p core-domain --locked`（13 项通过）、`cargo test -p indexlink-storage --locked`（默认 feature，31 项通过）、`cargo test -p indexlink-storage --locked --features postgres`（45 项通过）、`cargo check -p indexlink-server --locked`、`cargo clippy -p indexlink-storage --all-targets --all-features --locked -- -D warnings`、`cargo test --workspace --locked`（沙箱内首次因 localhost 监听权限失败，获准在沙箱外同命令重跑通过）、默认 server 与显式 storage PostgreSQL feature 的两条 `cargo tree` 断言、`git diff --check`。

### 2026-09-16 AEST — Push 2：行情与模拟 Broker capability 解耦

- 执行模型：GPT-5 Codex（多 Agent 实现通道触发使用额度上限后由主 Agent 接管收口）。
- 变更类型：服务端可选能力隔离、运行状态契约、配置兼容与回归测试。
- 涉及文件：`apps/server/src/{config.rs,main.rs}`、`crates/api/src/{state.rs,routes/{decision_preview.rs,decision_records.rs,runtime_status.rs}}`、`crates/api/tests/health.rs`、`apps/web/src/{api/types.ts,components/layout/runtime-status.tsx,i18n/locales/{zh.ts,en.ts}}`、`.env.example`、`readme.md`、`docs/reference/api-management.md`、`CHANGE_LOG.md`。
- 变更内容：将 OpenD 行情和 paper broker 改为独立开关、独立装配与 `not_configured/configured/unavailable` 三态；保留旧 `OPEND_*` 默认兼容。生产 `ApiState` 不再默认安装 MockBroker；未配置或初始化失败的 broker 路由安全返回 503。任一可选 adapter 初始化失败只记录安全状态，不再阻止 SQLite、计划、决策、审计与 HTTP 服务启动。
- 验证：`cargo fmt --all -- --check`、`cargo test -p core-domain --locked`（13 项通过）、`cargo test -p indexlink-api --locked`（全部单元、集成与文档测试通过）、`cargo check -p indexlink-server --locked`、`cargo test -p indexlink-server --locked`（37 项通过、1 项真实 OpenD smoke 按设计忽略）、`pnpm --dir apps/web lint`、`pnpm --dir apps/web test:coverage`（22 项通过；Statements 95.83%、Branches 90.00%、Functions 92.94%、Lines 99.22%）、`pnpm --dir apps/web build`、`git diff --check`。

### 2026-09-15 AEST — Push 1：隔离旧 MA200 回放与首页可选错误

- 执行模型：GPT-5 Codex。
- 变更类型：Web 兼容实验隔离、可选能力错误边界、回归测试与计划状态更新。
- 涉及文件：`apps/web/src/{components/v2_1/legacy-replay-panel.tsx,pages/{dashboard/index.tsx,lab/index.tsx,v2_1-shell.test.tsx},i18n/locales/{zh.ts,en.ts}}`、`apps/web/{PLAN.md,vitest.config.ts}`、`docs/plans/v2_1_closeout_hardness.md`、`CHANGE_LOG.md`。
- 变更内容：从旧 Dashboard 移除硬编码 MA200 历史回放，不删除兼容 API，也不改变 Rust 策略公式；回放改为高级实验室中的明确旧实验，只在用户点击后请求。普通 `/personal` 首页不会请求或展示旧回放。Dashboard 不再把市场、AI、组合、模拟收益和实际收益等可选查询错误汇总成页面级错误，而是在对应卡片内局部说明，核心 Plan/Decision 错误仍保留页面级呈现。新增成功、失败和首页零请求回归测试，并将后续顺序收口到服务端 capability 解耦、PostgreSQL opt-in 与 Fixed DCA 人工执行闭环。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test:coverage`（22 项通过；Statements 95.83%、Branches 90.00%、Functions 92.94%、Lines 99.22%）、`pnpm --dir apps/web build`、`cargo test -p core-domain --locked`（13 项通过）、`git diff --check`。

### 2026-09-15 AEST — V2.1 正式收口 Hardness 与阶段门槛

- 执行模型：GPT-5 Codex（当前环境未提供用户指定的 Aster 6，未声称由其执行）。
- 变更类型：项目级 Agent 约束、V2.1 收口执行门槛、当前代码进度审查与数据来源候选登记；无生产代码变更。
- 涉及文件：`AGENTS.md`、`docs/plans/v2_1_closeout_hardness.md`、`docs/plans/v2_1_productization_plan.md`、`docs/README.md`、`CHANGE_LOG.md`。
- 变更内容：将正式收口手册的主线固化为 `Plan → readable Decision → user-reported execution → Audit`，明确 Fixed DCA、local-first、用户执行权、DecisionRecord 不可变、append-only 手工留痕、可选能力隔离、SQLite 默认、PostgreSQL opt-in、市场数据/回测可复现、前端不得伪造真实结果等 Hardness。以当前 `0ff5920` 对照手册 `0cae8d5` 基线，登记新前端仍为演示壳、OpenD 行情与 broker 初始化仍耦合、价格型 DSL 仍被完整证据阻塞、PostgreSQL 仍在默认依赖图、Manual Execution Journal/产品级本地数据/BacktestService/Tauri 尚未完成。执行顺序改为 0B → 0C → 0D → Manual Journal → 真实 Fixed DCA Today/Plan → 3–5 位用户验证，策略平台 S1–S6 延后到反馈支持的 M2。将 HiThink-Tech Financial-API 登记为 A 股/A 股 ETF 可选导入来源候选，不进入 M0/M1 或核心运行时；要求先关闭授权、历史窗口、复权、幂等、冲突和离线可用性问题。
- 验证：完整提取并逐页检查 13 页正式 DOCX；核对 `0cae8d5..0ff5920` 文件差异与相关前后端调用路径；核对 Financial-API 官方 README、marketdb 文档和 MIT 软件许可证；运行 `git diff --check` 与文档链接检查。未运行 Rust/前端测试，因为本次仅修改执行文档且用户明确要求不落实生产代码。

### 2026-09-10 CST — V2.1 后端映射审计与统一策略目录实施账本

- 执行模型：GPT-5 Codex。
- 变更类型：V2.1 产品与技术执行计划、后端 API 映射审计、闭环缺口登记。
- 涉及文件：`docs/plans/v2_1_productization_plan.md`、`CHANGE_LOG.md`。
- 变更内容：在既有 V2.1 发布过渡计划中登记后端已有但主前端未正确映射的计划、执行预览、决策、DSL、准入研究、市场/AI、paper、运行状态能力；同时记录前端静态策略、浏览器会话完成状态、演示曲线、专业研究 DSL 入口与高级实验室配置壳等未闭环事实。将后续工作固化为六步顺序：统一策略目录与研究结果契约 → 接通 Fixed DCA 与 `core_opportunity_v1` 两条真实策略并标注 `90/10/0` 历史降级 → 策略中心真实 API 化 → 基于采用与参数快照建立我的计划 → 200 日均线趋势保护定投 → 数据能力就绪后扩展股债/轮动/波动率类别。
- 验证：`git diff --check` 通过；已核对新增章节层级、S1–S6 顺序与现有 V2.1 发布门槛不冲突。

### 2026-09-10 CST — 策略分析专业研究视角

- 执行模型：GPT-5 Codex。
- 变更类型：前端专业研究视图、既有后端准入指标接入、测试与前端计划更新。
- 涉及文件：`apps/web/src/{components/v2_1/professional-research-panel.tsx,pages/{strategy-analysis/index.tsx,v2_1-shell.test.tsx},vitest.config.ts}`、`apps/web/PLAN.md`、`CHANGE_LOG.md`。
- 变更内容：策略分析页新增“直观视角 / 专业研究”切换。专业研究视角不复用或包装前端演示曲线，而是按需读取既有 `GET /strategies/:policy_id/:policy_version/admission` 固定样本报告，仅面向已保存 DSL 策略展示策略与 Fixed DCA 的 XIRR、期末净值、最大回撤、年化波动率、Sortino、现金使用率、证据覆盖、观察数和滚动样本外窗口；后端不可用、无策略、准入未通过与样本不足均明确呈现，不伪造数值。内置 Fixed DCA 与 70/20/10 尚无统一公开准入报告，继续与演示曲线严格区分。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test:coverage`（19 项通过；纳入范围 Statements 96.22%、Branches 90.47%、Functions 93.75%、Lines 100%）、`pnpm --dir apps/web build`、`git diff --check` 通过。

### 2026-09-10 CST — V2.1 策略分析与归一化多策略对比

- 执行模型：GPT-5 Codex。
- 变更类型：消费级策略分析页面、归一化图表交互、导航、测试与前端计划更新。
- 涉及文件：`apps/web/src/{App.tsx,components/{layout/app-sidebar.tsx,v2_1/{strategy-card.tsx,strategy-center-nav.tsx}},features/v2_1/{model.ts,model.test.ts},pages/{strategy-center/index.tsx,strategy-analysis/index.tsx,v2_1-shell.test.tsx},i18n/locales/{zh.ts,en.ts}}`、`apps/web/{PLAN.md,vitest.config.ts}`、`CHANGE_LOG.md`。
- 变更内容：在策略中心增加“策略库 / 策略分析”二级导航和侧栏子入口；策略卡可直接进入分析页并把该策略作为初始对比对象。新增策略分析页，复用 Recharts 折线图能力，支持近 1 年、近 3 年和全部样本切换，并可在一张图上选择最多三条策略对比。所有曲线在选定窗口首点重置为 100，摘要显示同口径区间变化与期末指数。当前数据为确定性的本地示例序列，页面与计划均明确其不是真实回测；后续仅替换为带版本、数据集、费用和假设的可复核数据契约。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test:coverage`（17 项通过；纳入范围 Statements 97.52%、Branches 93.10%、Functions 94.64%、Lines 100%）、`pnpm --dir apps/web build`、`git diff --check` 通过；本地浏览器确认多策略选择后折线与摘要同步出现。

### 2026-09-10 CST — V2.1 消费级前端壳交互与布局修正

- 执行模型：GPT-5 Codex。
- 变更类型：前端布局缺陷修复、可交互状态反馈与测试更新。
- 涉及文件：`apps/web/src/{components/v2_1/strategy-card.tsx,pages/{personal/index.tsx,strategy-center/index.tsx,lab/index.tsx,v2_1-shell.test.tsx}}`、`CHANGE_LOG.md`。
- 变更内容：移除策略卡在个人中心栅格中的强制满高，修复卡片压住下方统计区的问题。已选策略改为明确状态，未选策略改为“选用这个策略”；选用后会更新当前策略并显示同步提示。个人中心的“我已完成这次投入”现在会在当前浏览器会话显示完成状态与统计变化，并明确未写入后端。高级实验室的配置预览改为在被点击的卡片内直接展开/收起，避免在页面下方展开而无可见反馈。将暂未实现的创建策略和风险筛选控件改为非按钮提示，避免不可交互的虚假入口。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test:coverage`（15 项通过；纳入范围 Lines / Statements / Functions / Branches 均为 100%）、`pnpm --dir apps/web build`、`git diff --check` 通过。

### 2026-09-10 CST — V2.1 消费级前端壳：个人中心、策略中心与高级实验室

- 执行模型：GPT-5 Codex。
- 变更类型：V2.1 前端信息架构、消费级视觉重构、本地演示交互与前端测试。
- 涉及文件：`apps/web/src/{App.tsx,index.css,stores/ui.ts,components/{layout/**,v2_1/**},features/v2_1/**,pages/{personal/**,strategy-center/**,lab/**,v2_1-shell.test.tsx},i18n/locales/{zh,en}.ts}`、`apps/web/{PLAN.md,vitest.config.ts}`、`CHANGE_LOG.md`。
- 变更内容：主导航重构为个人中心、策略中心与高级实验室；默认入口改为个人中心，呈现“本月航线”、正在坚持的策略、近期变化和本地演示统计。新增面向普通用户的三条精选策略卡、策略限制说明与本地比较交互；当前示例回测明确标注为界面展示，不伪装为实时收益。新增高级实验室壳，暴露 Docker、本地数据、Moomoo/OpenD、Qwen 与市场数据的配置方向，但不保存浏览器密钥、不验证账号、不连接或提交订单。策略中心使用 `/strategy-center`，明确避开既有 Rust `/strategies` API 的开发代理前缀。旧 Dashboard、计划、决策与 DSL Studio 页面/API 层保留为非主导航的后续集成基础。新增 V2.1 页面模型和交互测试，并将其纳入前端 90% 覆盖率门槛。
- 验证：`pnpm --dir apps/web test:coverage`（15 项通过；纳入范围 Lines / Statements / Functions / Branches 均为 100%）；桌面与 390px 本地浏览器视觉检查通过，主行动与移动布局可用；其余 lint、build 与差异检查见本次提交。

### 2026-09-10 CST — V2.1 本地优先发布过渡计划

- 执行模型：GPT-5 Codex。
- 变更类型：产品定位、发布范围与实施计划文档。
- 涉及文件：`docs/plans/v2_1_productization_plan.md`、`docs/README.md`、`CHANGE_LOG.md`。
- 变更内容：将 V2.1 从以页面重构为主的计划收束为单人可控的正式发布过渡版本：定位为本地优先的长期 ETF 策略库、回测与手动执行工作台；明确普通用户零基础设施配置、SQLite 默认、Docker 仅作分发/自托管方式、AI/OpenD 仅作高级可选能力。新增受控开源策略收录流程、策略来源与许可证要求、统一回测数据/假设契约、Manual-first 行动记录、Moomoo/Futu paper-only 边界，以及 P0–P5 实施顺序、发布门槛和 V3 交接条件。明确排除多用户 Cloud、任意代码、IBKR、新 broker 生产接入和实盘自动交易。
- 验证：`git diff --check` 通过；已核对 Markdown 标题层级与文档索引链接。

### 2026-09-08 CST — V2.1 产品化定位、前端信息架构与上线门槛

- 执行模型：GPT-5 Codex。
- 变更类型：产品定位、前端信息架构、路线图与上线标准文档。
- 涉及文件：`docs/plans/v2_1_productization_plan.md`、`docs/README.md`、`CHANGE_LOG.md`。
- 变更内容：新增 V2.1 双语产品化计划，明确 IndexLink 不与 QMT/PTrade 等通用量化交易终端竞争任意代码、全市场行情、实盘自动交易或低延迟执行；定位收束为面向长期指数投资者的本地优先策略治理、决策审计与 paper-trading 工作台。计划定义目标用户、待验证假设、本地优先与 AI/订单安全边界、六页面信息架构、现有 V1 控制台功能的迁移去向、六个前端产品化 Push、必须满足的上线门槛与用户访谈 Beta 门槛；文档索引新增入口。该计划不改变现有策略、数据库、API 或订单行为。
- 验证：`git diff --check`。

### 2026-09-08 CST — AI 审计追踪、决策差异与 Copilot 人工审阅

- 执行模型：GPT-5 Codex。
- 变更类型：AI Evidence 可观测性、决策存证、受限 Copilot 交互边界、前端测试与双语文档。
- 涉及文件：`crates/ai-client/src/{client,news}.rs`、`crates/api/src/{state.rs,routes/{decision_preview,market_sentiment}.rs}`、`crates/api/tests/{decision_preview,market_sentiment}.rs`、`apps/web/src/{api/types.ts,pages/{dashboard/index.tsx,decisions/index.tsx,strategies/{index.tsx,copilot-review.ts,copilot-review.test.ts}},i18n/locales/{zh,en}.ts}`、`apps/web/vitest.config.ts`、`docs/reference/api-management.md`、`readme.md`、`readme.en.md`、`CHANGE_LOG.md`。
- 变更内容：AI Evidence 成功路径记录 UTC 生成时间、端到端耗时和提示词契约版本；旧 `CoreOpportunityV1` 遇到未配置、新闻不可用、超时、拒绝、结构异常或其他 provider 故障时，继续使用既有 `90/10/0` 安全降级，并将无密钥、无底层报错的分类原因持久化为审计快照。Decision Preview 公开 `ai_audit`，并按同一计划上一份记录给出只读决策差异；历史决策页显示该追踪、降级与变化。Copilot 草案不再直接改写表单，先显示元数据及字段/规则差异，用户确认后才应用到本地编辑表单，随后仍必须验证、准入、保存、激活。README 和 API 文档补充 V2 闭环与边界。
- 验证：`cargo fmt --all -- --check`、`cargo test -p ai-client --locked`（99 unit + 23 local integration 通过，2 个真实网络 smoke ignored）、`cargo test -p indexlink-api --locked`（73 通过）、`cargo test -p core-domain --locked`（13 通过）、`cargo check --workspace --locked`、`cargo clippy -p ai-client -p indexlink-api --all-targets --all-features --locked -- -D warnings`、`cargo llvm-cov clean --workspace && cargo llvm-cov -p ai-client --all-features --summary-only`（Lines 98.18%）、`pnpm --dir apps/web lint`、`pnpm --dir apps/web test:coverage`（纳入范围 Lines 100%）、`pnpm --dir apps/web build`、`git diff --check`。

### 2026-09-01 CST — V2 文档归档与历史演示口径澄清

- 执行模型：GPT-5 Codex。
- 变更类型：文档信息架构、双语 README 演示说明与链接维护。
- 涉及文件：`docs/{README.md,architecture/strategy-studio-migration-plan.md,plans/v1_1_plan.md,reference/api-management.md,research/{calibration/**,experiments/**}}`、`readme.md`、`readme.en.md`、`CHANGE_LOG.md`。
- 变更内容：将根目录中的 API 契约、Strategy Studio 迁移计划、V1.1 计划、策略校准报告和历史实验按用途归档至 `docs/`，新增双语文档索引；使用 `git mv` 保留历史。双语 README 同步修复所有活跃链接，并将 YouTube 链接明确标注为 **V1 固定策略** 的历史本地/模拟账户演示，不代表当前 V2 Strategy Studio、多 Provider AI Profile、受限 Copilot 草案、策略准入、真实投资建议、实盘能力或收益承诺。
- 验证：`git diff --check`、活跃 Markdown 链接引用扫描。

### 2026-09-01 CST — Multi-Profile AI Deployment 与可测启动编排

- 执行模型：GPT-5 Codex。
- 变更类型：服务端 AI 适配器、多 Provider 安全配置、启动组合根、AI/RSS 测试与双语文档。
- 涉及文件：`apps/server/{Cargo.toml,src/{config,main}.rs}`、`crates/ai-client/src/{client,copilot,mock,news,provider}.rs`、`crates/ai-client/tests/{openai_compatible_client,news_live}.rs`、`crates/api/src/state.rs`、`.env.example`、`README.md`、`readme.en.md`、`API_MANAGEMENT.md`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`、`Cargo.lock`、`CHANGE_LOG.md`。
- 变更内容：服务端现兼容旧 `DASHSCOPE_*` 单 Qwen 配置，并支持无密钥 `AI_PROVIDER_PROFILES` 清单装配多个 OpenAI-compatible profile；清单只保存 Profile 元数据与 `api_key_env` 名称，要求恰有一个默认 Profile，远程 endpoint 强制 HTTPS（本机 loopback mock 可用 HTTP）。`QwenClient::with_profile` 将兼容协议客户端与安全 Profile 元数据绑定；API 在调用市场证据前二次检查 capability。启动路径拆为可注入 shutdown 的组合函数，覆盖内存 SQLite migration、优雅退出及启用/禁用 scheduler。新增本机 HTTP mock 覆盖 AI 超时、结构化 evidence、只读 Copilot DTO、RSS 成功/失败、Profile 边界；集成测试改名以避免 llvm-cov 将 `tests/client.rs` 与 `src/client.rs` 同名聚合。
- 验证：`cargo fmt --all -- --check`、`cargo test -p ai-client --offline`（96 unit + 16 local integration 通过，2 个真实网络 smoke ignored）、`cargo test -p indexlink-server --offline`（35 通过，1 个真实 OpenD smoke ignored）、`cargo test -p core-domain --offline`、`cargo check --workspace --locked`、`cargo clippy -p ai-client -p indexlink-api -p indexlink-server --all-targets --all-features --locked -- -D warnings`、`cargo llvm-cov -p ai-client --all-features --summary-only`（Lines 90.03%）、`cargo llvm-cov -p indexlink-server --all-features --summary-only`（Lines 90.02%）、`git diff --check`。

### 2026-09-01 CST — V2 README：Copilot Studio 操作与密钥边界

- 执行模型：GPT-5 Codex。
- 变更类型：双语使用文档与安全边界说明。
- 涉及文件：`README.md`、`readme.en.md`、`CHANGE_LOG.md`。
- 变更内容：补充从选择已部署 Provider、生成只读草案、人工编辑、校验、固定样本准入、保存到绑定计划的完整 Studio 操作流程；明确草案不会保存/激活/下单，未配置 Qwen/兼容 Provider 时仅禁用草案生成，手工 DSL 流程仍可用；新增本地 Web 地址和仅服务端保存 Key 的说明。
- 验证：`git diff --check`。

### 2026-09-01 CST — Copilot Studio Interaction：草案到人工准入工作流

- 执行模型：GPT-5 Codex。
- 变更类型：Web Strategy Studio、React Query 服务端缓存、双语交互与文档。
- 涉及文件：`apps/web/src/{api/{queries,types}.ts,pages/strategies/index.tsx,i18n/locales/{zh,en}.ts}`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`、`CHANGE_LOG.md`。
- 变更内容：Studio 读取无密钥 `/ai/providers` registry，仅显示声明 `restricted_policy_drafts` 的服务器已部署 profile。用户填写有界策略目标后可调用只读 `/strategies/copilot-draft`，界面展示生成 profile、解释、风险提示与可信引用，并把规范化 DSL 文档回填表单。生成操作不使策略列表失效、不保存 SQLite、不准入、不激活、不绑定计划或下单；用户仍需明确执行编辑 → 校验 → 固定样本准入 → 保存 → 绑定计划。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test`、`pnpm --dir apps/web build`、`git diff --check`。

### 2026-09-01 CST — Restricted Copilot Draft：只读 DSL 候选边界

- 执行模型：GPT-5 Codex。
- 变更类型：AI Provider 能力、受限策略草案 API、边界校验、测试与双语文档。
- 涉及文件：`crates/ai-client/src/{client,copilot,error,lib,mock,provider}.rs`、`crates/api/src/{state.rs,routes/strategies.rs}`、`crates/api/tests/strategies.rs`、`API_MANAGEMENT.md`、`readme.md`、`readme.en.md`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`、`CHANGE_LOG.md`。
- 变更内容：新增 `AiCopilotDraftRequest`、`AiCopilotDraft` 与封闭证据引用 DTO；Qwen/Mock profile 声明 `restricted_policy_drafts` 后才可提供此能力。Qwen 使用独立的 JSON-only Copilot system prompt，只允许白名单指标、条件、机会桶动作和服务器给定的证据 ID。新增 `POST /strategies/copilot-draft`：输入已部署 profile、`dsl_` policy ID/version 与有界目标；输出仅为经 `StrategySpecDocument -> StrategySpec` 重建校验后的规范化文档、解释、警告、无密钥 profile 和可信证据引用。该接口不会写 SQLite、创建决策存证、运行准入、保存/激活策略或提交订单；模型产生非法 DSL、错版本或未知证据 ID 时安全返回 `503`。
- 验证：`cargo fmt --all -- --check`、`cargo test -p ai-client --locked`、`cargo test -p indexlink-api --test strategies --locked`、`cargo test -p core-domain --locked`、`cargo check --workspace --locked`、`cargo clippy -p ai-client -p indexlink-api -p indexlink-server --all-targets --all-features --locked -- -D warnings`、`git diff --check`。

### 2026-08-31 CST — AI Evidence 解耦与已部署 Provider Registry

- 执行模型：GPT-5 Codex。
- 变更类型：AI 适配器边界、策略兼容、HTTP 契约、审计快照与双语文档。
- 涉及文件：`crates/ai-client/src/{client,lib,mock,news,provider}.rs`、`crates/api/src/{state.rs,routes/{decision_preview,market_sentiment,runtime_status}.rs}`、`crates/api/tests/{decision_preview,health,market_sentiment}.rs`、`API_MANAGEMENT.md`、`readme.md`、`readme.en.md`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`、`CHANGE_LOG.md`。
- 变更内容：新增 `AiProviderId`、`AiProviderProfileId`、无授权能力声明和仅保存无密钥元信息的 `AiProviderRegistry`。Qwen 注册为 `qwen-default` profile；key、endpoint、账户和 secret-manager 引用仍只允许位于 server 环境变量或部署侧 secret manager，永不进入 HTTP、日志或审计。新增只读 `GET /ai/providers` 与可选 `profile_id` 的 `POST /market-sentiment/preview`，仅允许服务器已注册 profile，未知 profile 安全返回 `400`。通用 `AiEvidence` 现携带 profile、结构化分析和原始 RSS headline；仅 `CoreOpportunityV1` 将其中 score 兼容映射为旧 10% 情绪输入，Fixed DCA 与 DSL Runtime 不读取它来改变推荐。Decision Record 的兼容 `sentiment_snapshot` 列继续使用，但 JSON 内明确存为 `ai_evidence` 与无密钥 provider 快照。当前能力仅为解释/警告，不提供 Copilot 保存、激活或下单权限。
- 验证：`cargo fmt --all -- --check`、`cargo test -p ai-client --locked`、`cargo test -p indexlink-api --locked`、`cargo test -p indexlink-server --locked`、`cargo test -p core-domain --locked`、`cargo check --workspace --locked`、`cargo clippy -p ai-client -p indexlink-api -p indexlink-server --all-targets --all-features --locked -- -D warnings`、`git diff --check`。

### 2026-08-31 CST — 不可变技术历史夹具 Push 3：全指标策略准入与公平对照

- 执行模型：GPT-5 Codex。
- 变更类型：策略准入回测、因果数据口径、Web Studio 展示与双语文档。
- 涉及文件：`crates/strategy-evaluation/src/lib.rs`、`crates/api/tests/strategies.rs`、`apps/web/src/{api/types.ts,pages/strategies/index.tsx,i18n/locales/{zh.ts,en.ts}}`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`、`readme.md`、`readme.en.md`、`CHANGE_LOG.md`。
- 变更内容：`GET /strategies/:policy_id/:policy_version/admission` 现在会依据策略实际引用的 Close、SMA、EMA、RSI、Drawdown、VIX，从已校验、编译期嵌入的 `technical-v1` 原始快照构造完整 `as_of` 证据；预热不足的早期观察被排除，任一资产无完整证据时拒绝激活。计划日期与预算仍来自版本化 `calibration-v2`，但策略与 Fixed DCA 都在 t+1 首个可用交易日、相同外部现金流和成本下成交。准入返回每资产的证据覆盖范围、XIRR、期末净值、最大回撤、年化波动、Sortino、现金使用率以及滚动 24 期/12 期步长样本外窗口；这些是透明比较信息而非收益承诺，跑赢 DCA 不是激活条件。补充全白名单指标准入与固定 1.0 倍机会桶等价 Fixed DCA 的回归测试；Studio 同步展示证据覆盖、滚动窗口、XIRR 和 Sortino，并补齐中英文翻译。
- 验证：`cargo fmt --all -- --check`、`cargo test -p strategy-evaluation --locked`、`cargo test -p indexlink-api --locked`、`cargo test -p core-domain --locked`、`cargo clippy -p strategy-evaluation -p indexlink-api -p strategy-dsl --all-targets --all-features --locked -- -D warnings`、`pnpm --dir apps/web lint`、`pnpm --dir apps/web test`、`pnpm --dir apps/web build`、`git diff --check`。

### 2026-08-31 CST — 不可变技术历史夹具 Push 2：因果 `as_of` 技术证据

- 执行模型：GPT-5 Codex。
- 变更类型：策略运行时数据边界、离线研究因果性与 API 联调测试。
- 涉及文件：`crates/{market-data,strategy-dsl,strategy-evaluation}/src/lib.rs`、`crates/api/src/routes/{market_data.rs,strategies.rs,decision_preview.rs}`、`crates/api/tests/{strategies.rs,decision_preview.rs,market_data.rs}`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`、`readme.md`、`readme.en.md`、`CHANGE_LOG.md`。
- 变更内容：新增 `TechnicalClose`、`TechnicalVix`、`TechnicalMarketSnapshot` 与 `DslEvidence::from_as_of_market_snapshot`。快照构造拒绝晚于 `as_of` 的价格/VIX、重复或倒序日线、非正收盘价和负 VIX；Close、SMA、EMA、RSI、Drawdown、VIX 继续由同一纯 Decimal 公式计算。`MarketSignalInput` 新增由 Cboe CSV 最后有效交易日解析的 `vix_as_of`，自动审计来源与市场输入 API 也公开该日期，避免将股票 `as_of` 误写为 VIX 时间。实时 Strategy Simulation 与自动 Decision Preview 已改用该入口，离线 `technical-v1` 测试在 t 日生成证据且只允许首个严格更晚交易日作为成交点。补强两条 scheduler/API 测试：月末日期不再构造非法月度第 29–31 日计划，改为等价的当日周度计划。固定样本准入范围仍为 RSI(14)/VIX；本次不改变生产策略、收益结论、订单行为或激活范围。
- 验证：`cargo fmt --all -- --check`、`cargo test -p market-data --locked`、`cargo test -p strategy-dsl --locked`、`cargo test -p strategy-evaluation --locked`、`cargo test -p indexlink-api --locked`、`cargo clippy -p market-data -p strategy-dsl -p strategy-evaluation -p indexlink-api --all-targets --all-features --locked -- -D warnings`、`git diff --check`。

### 2026-08-31 CST — 不可变技术历史夹具 Push 1：原始数据谱系与完整性

- 执行模型：GPT-5 Codex。
- 变更类型：离线研究数据完整性、安全准入基础与文档。
- 涉及文件：`Cargo.toml`、`Cargo.lock`、`crates/strategy-evaluation/{Cargo.toml,src/lib.rs,data/generated/technical-v1.manifest.json}`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`、`README.md`、`readme.en.md`、`CHANGE_LOG.md`。
- 变更内容：新增 `technical-v1` 不可变 manifest，独立记录已提交 FRED S&P 500 / NASDAQ Composite 日线作为 SPY / QQQ 的指数代理及 Cboe VIX 日线的来源、适用条款说明、日期格式、缺失行处理、共同覆盖范围与 SHA-256。`strategy-evaluation` 新增仅用 `include_str!` 嵌入快照的完整性校验：拒绝 manifest/hash 篡改、未覆盖声明区间、重复或倒序日期、非正价格和 CSV 结构错误；空值/`.` 明确计为 source gap，绝不填充或插值。此 Push 不修改生产策略、指标公式、策略准入范围或收益结论；下一个 Push 将建立全部 DSL 指标共享的 `as_of` 证据。
- 验证：`cargo fmt --all -- --check`、`cargo test -p strategy-evaluation --locked`、`cargo test -p core-domain --locked`、`cargo clippy -p strategy-evaluation --all-targets --all-features --locked -- -D warnings`、`git diff --check`。

### 2026-08-28 CST — V2 文档口径修正与 70/20/10 可复现基线披露

- 执行模型：GPT-5 Codex。
- 变更类型：策略研究、调度文档与双语 README 真实性更新。
- 涉及文件：`apps/server/src/config.rs`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`、`README.md`、`readme.en.md`、`CHANGE_LOG.md`。
- 变更内容：修正 server 配置中“固定月度”scheduler 的过时 rustdoc，明确它服务于每周或每月的固定计划日期；更新策略迁移计划顶部状态及 DSL 行，反映 PR 7a–7d 的保存、Studio、准入和激活闭环已实现。README 新增原始 `CoreOpportunityV1` 的版本化 `calibration-v1` 研究披露：历史收益严格使用 90/10/0 AI 不可用降级线，SPY/QQQ 指数代理在相同 USD 1,000 月度现金流、5 bps 成本、零现金利息和无未来函数条件下，相比 Fixed DCA 呈现较低回撤/波动但也有约 17% 未部署现金与较低期末净值；因此只作为可复现风险观察和审计能力说明，不构成稳定性或收益优势承诺。补充完整研究报告链接。
- 验证：`cargo fmt --all -- --check`、`cargo test -p core-domain --locked`、`git diff --check`。

### 2026-08-28 CST — README 实现边界表述收敛

- 执行模型：GPT-5 Codex。
- 变更类型：双语项目文档结构修正。
- 涉及文件：`README.md`、`readme.en.md`、`CHANGE_LOG.md`。
- 变更内容：将“当前实现与策略研究”改为“实现与策略研究”，并将双语能力表由“能力 / 当前状态 / 边界”重构为“实现 / 实现细节与边界”。删除“已完成”等状态列，只保留计划、市场证据、审计、调度、双桶、paper trading、统一策略 resolver 与受限 DSL 的实际实现方式、证据限制和安全边界。
- 验证：`git diff --check` 通过。

### 2026-08-28 CST — IndexLink V2 README 状态、演示与贡献者说明

- 执行模型：GPT-5 Codex。
- 变更类型：项目文档、演示入口与贡献者归属更新。
- 涉及文件：`README.md`、`readme.en.md`、`CHANGE_LOG.md`。
- 变更内容：将中英文 README 更新为 IndexLink V2 当前状态：版本化策略、Strategy Studio、固定样本准入、统一策略 resolver 与 Web 运行状态提示均已完成接入，继续明确单用户、paper-only、无收益承诺边界。新增 YouTube 项目演示链接 `https://www.youtube.com/watch?v=t8TCjlqE7D0`；修正英文路线图中过时的 Studio/激活“未实现”表述。贡献者部分明确 Jame 的项目发起、架构/70-20-10 基本面与趋势层、前端、PR 审阅与维护职责；Xuanzhou Gu 的 V2 后端、策略、部署、测试、文档与演示闭环职责；Yucong Peng 的 AI 层设计与实现职责。
- 验证：`git diff --check` 通过。

### 2026-08-28 CST — Strategy Studio 全量双语与前端覆盖率门槛

- 执行模型：GPT-5 Codex。
- 变更类型：前端国际化收尾、测试基础设施与质量门槛。
- 涉及文件：`apps/web/src/pages/strategies/index.tsx`、`apps/web/src/i18n/locales/{en.ts,zh.ts}`、`apps/web/{package.json,pnpm-lock.yaml,vitest.config.ts,eslint.config.js}`、`.gitignore`、`CHANGE_LOG.md`。
- 变更内容：Strategy Studio 全部可见标签、按钮、确认提示、模拟解释、准入回测指标、条件组及 DSL 白名单编辑器均改为 i18n 键；切换语言后不再保留中文硬编码。扩展 Vitest V8 coverage，新增 `test:coverage` 命令并为当前可确定性测试的前端领域边界（locale 契约和决策筛选器）配置行/分支/函数/语句 **90%** 门槛；本次实测均为 **100%**。生成的 `apps/web/coverage/` 已加入 Git ignore。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web build`、`pnpm --dir apps/web test:coverage`、`git diff --check` 通过。

### 2026-08-28 CST — 前端服务端状态治理、运行可观测性与双语契约

- 执行模型：GPT-5 Codex。
- 变更类型：前后端可观测性、React Query 缓存治理、决策历史检索、路由可靠性与前端测试。
- 涉及文件：`apps/server/src/main.rs`、`crates/api/src/{state.rs,lib.rs,routes/{mod.rs,runtime_status.rs,decision_preview.rs,decision_records.rs},tests/health.rs}`、`crates/decision-records/src/lib.rs`、`crates/storage/src/sqlite_decision_records.rs`、`apps/web/{PLAN.md,package.json,vite.config.ts,vitest.config.ts,src/{App.tsx,api/{queries.ts,types.ts},components/layout/{app-layout.tsx,runtime-status.tsx},pages/{dashboard,decisions,route-error}.tsx,i18n/{locales/{en.ts,zh.ts},locales.test.ts}}`、`CHANGE_LOG.md`。
- 变更内容：新增只读 `/runtime-status`，安全返回 SQLite、市场数据、Qwen、真实 paper broker 的装配状态以及 scheduler 的启用状态、最近成功计数与失败时间；服务端 scheduler 将成功/失败安全地写入该状态。前端顶栏分别读取 `/health`、`/ready`、`/runtime-status`，明确区分 API 离线、SQLite 未就绪、可选依赖未配置和 scheduler 状态，且不会触发 Qwen 或订单。行情、Qwen、组合、收益账本、真实轨迹、历史回放与价格历史已改由 React Query query cache 管理，手动刷新使用同一缓存；决策创建/审批会失效跨标的记录与相关账本缓存。新增跨计划 `GET /decisions`，决策页支持计划、动作、日期筛选与客户端分页，并保留详情/审批下单入口。路由按页面懒加载并使用自定义可恢复错误页；主入口 bundle 降至约 485KB。新增 Vitest，覆盖筛选纯函数与中英文 locale 键一致性/非空约束；补充 Dashboard 新增可见内容的双语翻译，并更新前端计划为真实 API 现状。
- 验证：`pnpm --dir apps/web lint`、`pnpm --dir apps/web test`（4 项通过）、`pnpm --dir apps/web build`、`cargo fmt --all -- --check`、`cargo test -p core-domain -p indexlink-api -p indexlink-server` 均通过。

### 2026-08-28 CST — V1.1 计划配置、双桶展示与审批下单闭环

- 执行模型：GPT-5 Codex。
- 变更类型：前后端联调、安全审批执行与审计一致性修复。
- 涉及文件：`apps/web/src/{api/{queries.ts,types.ts},pages/{dashboard,decisions,plans}/index.tsx}`、`crates/api/src/routes/{decision_preview.rs,decision_records.rs}`、`crates/decision-records/src/lib.rs`、`crates/storage/src/sqlite_decision_records.rs`、`API_MANAGEMENT.md`、`CHANGE_LOG.md`。
- 变更内容：Dashboard 不再提交会被服务端忽略的临时双桶比例，改为只读显示已持久化计划配置，并补足推荐总额、核心/机会金额、机会倍率、滚存、现金策略与审批状态。Plans 支持创建及编辑多执行日、核心/机会桶、风险模式、现金滚存/上限、周期执行上限和启停；计划的周期类型、标的与币种仍属于账本口径，编辑时明确要求创建新计划而非静默变更。新增 `POST /decisions/:id/approve-paper-order`：仅审批模式且 `due` 的既有存证可提交，金额仅取自不可变审计快照；SQLite 先原子持久化订单意图，再调用 paper-only broker，并阻止重复确认。审批计划的自动 Preview 不再提前写入订单意图或直接下单，前端改为在决策详情确认同一条存证。
- 验证：`cargo fmt --all -- --check`、`cargo test -p decision-records -p indexlink-storage -p indexlink-api --lib`、`cargo test -p core-domain`、`cargo clippy -p indexlink-api -p decision-records -p indexlink-storage --all-targets --all-features -- -D warnings`、`pnpm --dir apps/web build`、`git diff --check` 均通过。

### 2026-08-27 CST — 市场数据、应用状态与优雅关闭覆盖率补全

- 执行模型：GPT-5 Codex。
- 变更类型：测试覆盖率与可测试性改进；不改变生产业务规则。
- 涉及文件：`crates/market-data/src/lib.rs`、`crates/api/src/state.rs`、`apps/server/src/shutdown.rs`、`CHANGE_LOG.md`。
- 变更内容：为本机 OpenD 日线 adapter 增加独立帧完整性、回环地址、宏观解析、历史长度与成功读取测试；为 `ApiState` 增加缺失依赖安全映射、可替换 market-data、SQLite 计划/调度/账本以及价格图表/历史回放组合测试；将 shutdown 的信号等待拆为可控内部入口，覆盖 Ctrl+C、SIGTERM 完成分支与真实 Unix SIGTERM 注册路径。真实信号测试在 handler 安装完成后才向当前测试进程发送 SIGTERM，避免启动竞态。
- 验证：`cargo test -p indexlink-api -p indexlink-server -p market-data --locked`、`cargo llvm-cov --workspace --locked --summary-only` 均通过；全工作区 LLVM 行覆盖率为 **90.73%**（`market-data` 90.70%、`ApiState` 84.13%、`shutdown.rs` **91.04%**）。

### 2026-08-27 CST — Strategy Studio 本地开发代理修正

- 执行模型：GPT-5 Codex。
- 变更类型：前端开发环境 API 联调修复。
- 涉及文件：`apps/web/vite.config.ts`、`CHANGE_LOG.md`。
- 变更内容：为 `/strategies` 增加 Vite 到本机 Rust API `127.0.0.1:8080` 的代理，确保 Strategy Studio 的列表、保存、模拟、准入与激活请求不会错误落在前端 `5173`。
- 验证：`pnpm --dir apps/web build`、`git diff --check`。

### 2026-08-25 CST — DSL 策略固定样本准入与激活门槛

- 执行模型：GPT-5 Codex。
- 变更类型：策略激活安全门槛、固定样本回测对照与 Studio 准入展示。
- 涉及文件：`Cargo.toml`、`Cargo.lock`、`crates/strategy-evaluation/src/lib.rs`、`crates/api/{Cargo.toml,src/state.rs,src/routes/{strategies.rs,investment_plans.rs},tests/strategies.rs}`、`apps/web/src/{api/{types.ts,queries.ts},pages/strategies/index.tsx}`、`API_MANAGEMENT.md`、`README.md`、`README.en.md`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`、`CHANGE_LOG.md`。
- 变更内容：保存 DSL 前继续由领域 DTO 重建并校验；新增只读 `GET /strategies/:policy_id/:policy_version/admission`，以版本化 `calibration-v2` 在一致现金流、成本与 t+1 成交口径下对比候选与 Fixed DCA 的期末净值、最大回撤、年化波动与现金使用率。核心桶/预算检查或历史输入覆盖不足都会阻断激活；当前只为 RSI(14)/VIX 提供完整固定样本，其他白名单指标可以保存和当前数据模拟，但不会用伪造历史输入取得激活资格。Strategy Studio 必须先显示并通过准入结果，才允许绑定计划。
- 验证：`cargo test --workspace --locked -q`（本机 mock HTTP 测试在允许绑定临时端口的环境中通过；3 个真实外部 smoke 保持 ignored）、`cargo test -p core-domain --locked`、`cargo test -p strategy-evaluation --locked`、`cargo test -p indexlink-api --locked`、`cargo check --workspace --locked`、`cargo clippy -p strategy-evaluation -p indexlink-api --all-targets --all-features --locked -- -D warnings`、`pnpm --dir apps/web build`、`cargo fmt --all -- --check` 与 `git diff --check` 均通过。

### 2026-08-25 CST — DSL Runtime V2 与 Studio 规则编辑

- 执行模型：GPT-5 Codex。
- 变更类型：线上/离线共享技术证据计算、当前数据模拟与多规则 Strategy Studio。
- 涉及文件：`crates/strategy-dsl/src/lib.rs`、`crates/strategy-evaluation/src/lib.rs`、`crates/api/{src/state.rs,src/routes/{decision_preview.rs,strategies.rs}}`、`apps/web/src/{api/types.ts,pages/strategies/index.tsx}`、`API_MANAGEMENT.md`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`、`CHANGE_LOG.md`。
- 变更内容：DSL 线上 Runtime 可计算收盘价、SMA、EMA、RSI、回撤与 VIX；`as_of` 前本机 OpenD 日线和 Cboe VIX 快照构成唯一证据，指标计算与离线研究共用纯 Decimal builder。Studio 新增多条有序规则、all/any 条件组、白名单窗口/阈值、版本复制与不产生订单或审计的当前数据模拟；结果说明首条命中规则与实际指标值。
- 验证：`cargo test -p strategy-dsl --features serde --locked`、`cargo test -p strategy-evaluation --locked`、`cargo test -p indexlink-api --locked`、`cargo test -p core-domain --locked`、`cargo check --workspace --locked`、Clippy、前端 build、fmt 与 `git diff --check`。

### 2026-08-25 CST — Strategy Studio 与激活执行闭环

- 执行模型：GPT-5 Codex。
- 变更类型：受限 DSL 策略前端、验证/保存/显式激活与统一 Runtime 接入。
- 涉及文件：`crates/strategy-dsl/src/lib.rs`、`crates/api/{src/state.rs,src/routes/{strategies.rs,investment_plans.rs,decision_preview.rs},tests/strategies.rs}`、`apps/web/src/{App.tsx,api/{types.ts,queries.ts},components/layout/app-sidebar.tsx,pages/strategies/index.tsx,i18n/locales/{zh,en}.ts}`、`API_MANAGEMENT.md`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`、`CHANGE_LOG.md`。
- 变更内容：新增 Studio 可调用的策略校验/不可变保存 API、计划确认激活入口和策略页面；DSL Runtime 已接入自动 Decision Preview、scheduler 与审计的同一计划绑定版本。线上只接受可由当前自动市场数据可靠提供的 RSI(14)/VIX 规则；固定金额动作及其他指标保留离线研究，不能激活。核心桶仍由领域层保留，DSL 只能影响机会桶；审批模式不自动下单且系统仍 paper-only。
- 验证：`pnpm --dir apps/web build`、`cargo test -p indexlink-api --test strategies --locked`、`cargo test -p strategy-dsl --features serde --locked`、`cargo test -p core-domain --locked`、`cargo fmt --all -- --check`、`cargo clippy -p strategy-dsl -p indexlink-api --all-targets --all-features --locked -- -D warnings` 与 `git diff --check`。

### 2026-08-25 CST — PR 7a

- 执行模型：GPT-5 Codex。
- 变更类型：受限 DSL 版本存储与只读 HTTP API。
- 涉及文件：
  - `Cargo.lock`
  - `crates/strategy-dsl/{Cargo.toml,src/lib.rs}`
  - `crates/storage/{Cargo.toml,src/lib.rs,src/sqlite.rs,src/sqlite_strategy_specs.rs}`
  - `migrations/sqlite/20260825100000_create_strategy_specs.sql`
  - `crates/api/{Cargo.toml,src/state.rs,src/routes/mod.rs,src/routes/strategies.rs,tests/strategies.rs}`
  - `API_MANAGEMENT.md`、`readme.md`、`readme.en.md`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 `strategy-dsl` 增加 opt-in `serde` 文档 DTO；JSON 反序列化后必须通过 `StrategySpecDocument::into_strategy_spec` 重走 `PolicyId`、`PolicyVersion`、窗口、表达式、动作和复杂度不变量，不能直接构造运行时策略。
  - SQLite 新增不可变 `strategy_specs` 表，以 `(policy_id, policy_version)` 为主键保存规范化文档；读取时再次验证文档并核对行级元数据，损坏/不一致数据安全拒绝。
  - 新增只读 `GET /strategies` 与 `GET /strategies/:policy_id/:policy_version`。本阶段没有策略创建、更新、删除、激活、计划绑定、实时执行、scheduler 接入或下单路径，因此已保存 DSL 版本仍是惰性的审阅对象。
  - 同步双语 README、迁移计划和 API 清单，明确 PR 7a 完成且下一步为受控验证/回测与 Web Studio。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p core-domain --offline` 通过（13 个测试）。
  - `cargo test -p strategy-dsl --features serde --offline` 通过（9 个测试）。
  - `cargo test -p indexlink-storage --offline` 通过（45 个测试）。
  - `cargo test -p indexlink-api --offline` 通过（含策略只读 API 集成测试）。
  - `cargo check --workspace --locked`、针对 `strategy-dsl` / `indexlink-storage` / `indexlink-api` 的 Clippy 与 `git diff --check` 通过。

### 2026-08-25 CST — PR 5–6

- 执行模型：GPT-5 Codex。
- 变更类型：确定性 DSL runtime 与统一历史评估研究候选。
- 涉及文件：
  - `Cargo.lock`
  - `crates/strategy-dsl/Cargo.toml`
  - `crates/strategy-dsl/src/lib.rs`
  - `crates/strategy-evaluation/Cargo.toml`
  - `crates/strategy-evaluation/src/lib.rs`
  - `readme.md`、`readme.en.md`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - `strategy-dsl` 新增无 IO 的确定性解释器：`DslEvidence` 将同一 `as_of` 已解析指标固定为不可重复的白名单快照；`StrategySpec::evaluate` 以首条命中顺序返回 `DslEvaluation` 与通用 `InvestmentRecommendation`。
  - 对缺失/重复指标、Decimal 算术溢出和调用时周期预算不匹配安全失败；DSL 动作仍只能作用于机会桶，不能生成核心桶否决、网络调用、审计写入或订单。
  - `strategy-evaluation` 新增 `dsl_rsi_opportunity_guard_v1`：直接调用上述 runtime，在决策日读取 RSI-14，低于 35 时机会桶 `1.10x`、高于 65 时 `0.85x`、否则 `1.00x`；核心桶固定，并保持 `t` 日决策、严格更晚观察日成交及对称成本口径。
  - 该候选仅用于离线研究和与固定 DCA 的匹配对照；未接入生产 resolver、SQLite、HTTP、scheduler、前端 Studio 或下单路径，未改变任何现有计划的默认策略和执行行为。
- 验证：
  - `cargo fmt --all` 通过。
  - `cargo test -p strategy-dsl --offline` 通过（9 个测试）。
  - `cargo test -p strategy-evaluation --offline` 通过（9 个测试）。
  - 其余 workspace、核心领域与 Clippy 验证见本次提交记录。

### 2026-08-25 CST — PR 4

- 执行模型：GPT-5 Codex。
- 变更类型：受限策略 DSL/AST 与校验。
- 涉及文件：
  - `Cargo.toml`、`Cargo.lock`
  - `crates/strategy-dsl/Cargo.toml`
  - `crates/strategy-dsl/src/lib.rs`
  - `README.md`、`README.en.md`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增无 IO 的 `strategy-dsl` crate，提供版本化自定义策略定义、白名单指标、有限值表达式、条件树与固定金额/机会桶动作。
  - 对窗口、名称、规则数量、条件树复杂度、除数、固定金额及调用方周期预算建立不变量；DSL 无法表示核心桶否决、任意脚本、网络访问、数据库访问或订单执行。
  - 本次仅定义和验证 AST，尚未向 SQLite、HTTP、前端 Studio、Qwen 或实际策略 runtime 暴露；现有内置策略与执行行为不变。
- 验证：
  - `cargo test -p strategy-dsl --offline` 通过（6 个测试）。
  - `cargo clippy -p strategy-dsl --all-targets --all-features --offline -- -D warnings` 通过。
  - `cargo test -p core-domain --offline` 通过。

### 2026-08-25 CST

- 执行模型：GPT-5 Codex。
- 变更类型：策略版本与决策审计升级。
- 涉及文件：
  - `crates/decision-records/**`
  - `crates/storage/src/sqlite_decision_records.rs`
  - `crates/storage/src/decision_records.rs`
  - `migrations/sqlite/20260825090000_add_decision_policy_evidence.sql`
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/tests/decision_preview.rs`
  - `apps/web/src/api/types.ts`
  - `apps/web/src/pages/plans/index.tsx`
  - `apps/web/src/pages/decisions/index.tsx`
  - `apps/web/src/i18n/locales/{zh,en}.ts`
  - `README.md`、`README.en.md`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`
  - `Cargo.lock`、`CHANGE_LOG.md`
- 变更内容：
  - 新决策记录结构化保存不可变 `policy_id + policy_version` 与不含凭据的通用推荐快照；SQLite 以追加列与写入触发器保证三列成组、版本为正、快照为非空 JSON。
  - 旧决策记录不回填、不删除，读取时显式显示为“迁移前记录”；新的手动预览、自动预览和 scheduler 审计均通过同一结构化记录入口。
  - PostgreSQL 兼容 adapter 不会静默丢弃新策略证据：在未完成对应 schema migration 前安全拒绝该写入；当前生产 SQLite 路径完整支持。
  - 前端计划创建页可显式选择内置 `fixed_dca@1` 或 `core_opportunity_v1@1`，审计详情显示实际策略版本。
  - 本次不改变策略公式、默认固定 DCA、scheduler 自动下单边界或 OpenD paper-only 安全限制。
- 验证：
  - `cargo test -p decision-records -p indexlink-storage -p indexlink-api --offline` 通过。
  - 其余 workspace、核心领域、Clippy、前端构建与 diff 检查见本次提交验证记录。

### 2026-08-24 CST

- 执行模型：GPT-5 Codex。
- 变更类型：固定 DCA 策略、计划策略绑定与统一执行入口。
- 涉及文件：
  - `Cargo.toml`、`Cargo.lock`
  - `crates/strategy-policy/**`
  - `crates/builtin-policies/**`
  - `crates/investment-plans/**`
  - `crates/storage/src/sqlite_investment_plans.rs`
  - `crates/storage/src/investment_plans.rs`
  - `migrations/sqlite/20260824090000_add_plan_policy_bindings.sql`
  - `crates/api/src/state.rs`
  - `crates/api/src/routes/investment_plans.rs`
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/tests/**`
  - `apps/web/src/api/types.ts`
  - `apps/web/src/pages/dashboard/index.tsx`
  - `apps/web/src/pages/decisions/index.tsx`
  - `readme.md`、`readme.en.md`、`STRATEGY_STUDIO_MIGRATION_PLAN.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `FixedDcaPolicy` 与无 IO 的内置策略 resolver；固定策略始终给出 `Standard`/`1.0x` 推荐，不读取 CAPE、趋势、Qwen 或调用方伪造信号。
  - 计划新增不可变 `policy_id + policy_version` 绑定：新建计划默认 `fixed_dca@1`，SQLite migration 将已存在计划回填为兼容的 `core_opportunity_v1@1`。
  - 手动预览、自动预览、scheduler、审计快照和可选 paper-only 下单统一经 resolver；固定 DCA 审计明确记录未使用的市场层，未知策略在 HTTP 边界安全拒绝。
  - 前端将固定 DCA 的空评分显示为“未使用”，避免将未读取的市场层误呈现为零分或触发空值渲染错误。
  - PostgreSQL adapter 仅保留旧版本策略作为兼容 fallback；当前生产 SQLite 路径完整持久化策略绑定。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p strategy-policy --offline` 通过。
  - `cargo test -p builtin-policies --offline` 通过。
  - `cargo test -p investment-plans --offline` 通过。
  - `cargo test -p indexlink-storage --offline` 通过。
  - `cargo test -p indexlink-api --offline` 通过。
  - `cargo test -p core-domain --offline` 通过。
  - `cargo check --workspace --locked`、`cargo clippy -p strategy-policy -p builtin-policies -p investment-plans -p indexlink-storage -p indexlink-api --all-targets --all-features --locked -- -D warnings` 和 `git diff --check` 通过。

### 2026-08-24 CST

- 执行模型：GPT-5 Codex。
- 变更类型：策略契约与 `CoreOpportunityV1` 兼容包装。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/strategy-policy/Cargo.toml`
  - `crates/strategy-policy/src/lib.rs`
  - `crates/builtin-policies/Cargo.toml`
  - `crates/builtin-policies/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增无 IO 的 `strategy-policy` crate，提供带校验的 `PolicyId`、`PolicyVersion`、`PolicyRef`、泛型 `DecisionContext`、`InvestmentRecommendation` 与 `InvestmentPolicy` 契约。
  - 新增无 IO 的 `builtin-policies` crate；`CoreOpportunityV1` 原样调用既有 `evaluate_decision`，将旧动作、倍率与周期预算映射为通用推荐，不修改 70/20/10 公式、降级模式或 `TacticalDelay` 语义。
  - 通过回归测试锁定完整 `DecisionSignal` 与通用推荐的动作、倍率、周期预算；本次不修改计划、SQLite、HTTP、scheduler、broker 或 paper order 行为。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p strategy-policy --locked` 通过（2 个测试）。
  - `cargo test -p builtin-policies --locked` 通过（2 个测试）。
  - `cargo test -p core-domain --locked` 通过（13 个测试）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p strategy-policy -p builtin-policies --all-targets --all-features --locked -- -D warnings` 通过。
  - `git diff --check` 通过。

### 2026-08-24 CST

- 执行模型：GPT-5 Codex。
- 变更类型：产品定位与策略工作台迁移文档。
- 涉及文件：
  - `STRATEGY_STUDIO_MIGRATION_PLAN.md`
  - `readme.md`
  - `readme.en.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将公开定位由“单一 70/20/10 自适应定投模型”调整为“透明、可审计、可扩展的量化定投策略工作台与 paper-trading 执行平台”，明确不承诺跑赢固定 DCA 或市场。
  - 新增双语策略工作台迁移计划，预登记通用策略契约、`CoreOpportunityV1` 兼容包装、`FixedDcaPolicy`、策略版本、受限 DSL、统一评估、API/Web Studio 与 Qwen Copilot 的渐进式小 PR 顺序。
  - 明确现有 70/20/10 与 C1–C4 仅为可复现实验资产；旧计划、旧 API、旧审计记录、scheduler 幂等、paper-only 边界与 OpenD adapter 必须保持兼容。
  - 重写中英文 README，区分当前已实现能力与目标架构，增加策略生命周期、迁移路线、固定 DCA 基准、AI 权限边界和 Alibaba Cloud ECS 运行说明。
- 验证：
  - `git diff --check` 通过。
  - 文档链接、双语章节与 Mermaid 架构图已人工复核。

### 2026-08-21 CST

- 执行模型：GPT-5 Codex。
- 变更类型：V1.1 C4 有期限机会预算调度研究。
- 涉及文件：
  - `crates/strategy-evaluation/src/lib.rs`
  - `crates/strategy-evaluation/src/main.rs`
  - `crates/strategy-evaluation/data/generated/calibration-v2-c4-research.report.json`
  - `STRATEGY_C4_RESEARCH_V1.md`
  - `V1_1_PLAN.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增仅离线研究的 C4：核心桶固定执行；机会桶以仅包含过去 12 个已完成基本面方向分数的历史排名形成 `[0.85, 1.15]` 倾斜；趋势只限制超过 `1.00` 的加码，AI 仅承担解释职责。
  - 将每次少投的机会金额记录为带到期日的研究 lot；最多滚存三个周期，到期强制追赶；高于基线的投入只能释放已有 lot，不能预支未来外部现金流。
  - 机器可读报告新增所有策略的机制目录、C4 现金诊断、比例敏感性中的 C4 结果和全策略公平对照。C4 修复长期现金拖累但没有证明显著 alpha 或风险调整后优势，未改动生产默认策略。
  - 公平对照实际结果：C4 的 SPY/QQQ 相对固定 DCA 期末差分别为 `-0.04%`/`-0.05%`，现金使用率为 `99.83%`/`99.96%`；它是执行边界修正，不得表述为提高指数收益。
  - V1.1 后续先做因子预测有效性检验，再依据预登记留出集的收益、回撤、Sortino、现金使用率与窗口胜率决定是否改动生产策略；Qwen 继续仅作解释/预警，新的机会 lot API/Web 仅在策略门槛通过后实现。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p strategy-evaluation --locked` 通过。
  - `cargo test -p core-domain --locked` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p strategy-evaluation --all-targets --all-features --locked -- -D warnings` 通过。
  - `git diff --check` 通过。

### 2026-08-21 CST

- 执行模型：GPT-5 Codex。
- 变更类型：V1.1 C3 基本面/趋势解耦预登记研究。
- 涉及文件：
  - `crates/strategy-evaluation/src/lib.rs`
  - `crates/strategy-evaluation/src/main.rs`
  - `crates/strategy-evaluation/data/generated/calibration-v2-c3-research.report.json`
  - `STRATEGY_C3_RESEARCH_V1.md`
  - `V1_1_PLAN.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增仅离线研究的 C3：核心桶永久按用户比例投入；基本面决定机会桶 `[0.75, 1.25]` 的有界基础倍率；趋势只把“加码”连续限制到最高 `1.00`，不再输出 `TacticalDelay` 或阻断正常投入。
  - 在不覆盖冻结 `calibration-v2` 报告的前提下生成独立 C3 机器可读结果，并增加 Sortino、最大回撤恢复月数、最差滚动窗口和 `100/0`、`80/20`、`70/30`、`50/50` 比例敏感性。
  - C3 相比当前策略减少闲置现金并缩小终值差距，但仍落后于 C1 与匹配条件的固定 DCA；因此未修改生产默认策略、决策引擎、API、scheduler 或下单行为。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p strategy-evaluation --locked` 通过。
  - `cargo test -p core-domain --locked` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p strategy-evaluation --all-targets --all-features --locked -- -D warnings` 通过。
  - `git diff --check` 通过。

### 2026-08-21 CST

- 执行模型：GPT-5 Codex。
- 变更类型：V1.1 策略研究口径修正与连续趋势上限候选。
- 涉及文件：
  - `tools/generate_calibration_fixture.py`
  - `crates/strategy-evaluation/src/lib.rs`
  - `crates/strategy-evaluation/src/main.rs`
  - `crates/strategy-evaluation/data/generated/calibration-v2.json`
  - `crates/strategy-evaluation/data/generated/calibration-v2.manifest.json`
  - `crates/strategy-evaluation/data/generated/calibration-v2.report.json`
  - `STRATEGY_CALIBRATION_RESEARCH_V2.md`
  - `V1_1_PLAN.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 保留不可变的 `calibration-v1`，新建 `calibration-v2`：每月末可得收盘后计算决策，成交只使用严格晚于决策日的第一个日度价格；不允许以同一收盘价同时决策和成交。
  - 新增研究专用 C2：核心桶固定为周期预算的 70%，趋势只以 `[0.25, 1.00]` 的连续风险上限约束机会桶倍率，且不产生整笔 `TacticalDelay` veto；生产默认策略、权重、API 与下单行为均未改动。
  - 更正 XIRR 的期末估值日期，使期末净值现金流计入最后一次真实估值/成交日，而非错误回填至最后一次入金日。
  - V2 的实际结果显示 C2 主要以现金留存降低回撤和波动，未在匹配固定 DCA 下形成稳定收益优势，故明确保留为负对照，不升级默认策略。
- 验证：
  - `python3 tools/generate_calibration_fixture.py` 通过。
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p strategy-evaluation --locked` 通过。
  - `cargo test -p core-domain --locked` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p strategy-evaluation --all-targets --all-features --locked -- -D warnings` 通过。

### 2026-08-21 CST

- 执行模型：GPT-5 Codex。
- 变更类型：V1.1 核心桶订单门控修正。
- 涉及文件：
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/tests/decision_preview.rs`
  - `crates/strategy-evaluation/src/lib.rs`
  - `crates/strategy-evaluation/data/generated/calibration-v1.report.json`
  - `STRATEGY_CALIBRATION_BASELINE_V1.md`
  - `V1_1_PLAN.md`
- 变更内容：
  - 到期且已校验的 `Skip`/`TacticalDelay` paper order 不再被 API 全局 veto；领域层已计算并保留的核心桶仍进入订单，机会桶仍为零。
  - 自动来源订单同样按核心桶后的建议金额换算数量；零建议金额不会构造或提交空订单；手动预览保持既有显式确认和数量校验契约。
  - 新增 `Skip` 与 `TacticalDelay` 的路由回归测试；离线基线的当前 API 实效线同步为修复后的生产口径。
- 验证：
  - `cargo test -p indexlink-api --test decision_preview --locked` 通过。
  - `cargo test -p strategy-evaluation --locked` 通过。
  - `cargo test -p core-domain --locked` 通过。

### 2026-08-21 CST

- 执行模型：GPT-5 Codex。
- 变更类型：V1.1 评分校准基线（Push 3：预登记候选与样本外对照）。
- 涉及文件：
  - `crates/strategy-evaluation/src/lib.rs`
  - `crates/strategy-evaluation/data/generated/calibration-v1.report.json`
  - `STRATEGY_CALIBRATION_CANDIDATES_V1.md`
  - `V1_1_PLAN.md`
- 变更内容：
  - 仅新增评估专用、非生产的 C1：核心桶维持 70%，机会桶倍率预先固定为 `0.75 + 0.50 × final_score` 且限定在 `[0.75, 1.25]`，评估中不把非中性趋势升级为整笔订单 veto。
  - 在完全相同的冻结数据、外部现金流、5 bps 成本、现金口径和 24 个月滚动样本外窗口下比较 C1 与固定 DCA；结果仍未显示稳定跑赢，因此不提升为默认策略。
  - 明确后续候选须先登记再看留出集，禁止根据收益结果改倍率或筛选窗口。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p strategy-evaluation --locked` 通过。
  - `cargo test -p core-domain --locked` 通过。
  - `cargo run -p strategy-evaluation --locked -- crates/strategy-evaluation/data/generated/calibration-v1.report.json` 通过。

### 2026-08-21 CST

- 执行模型：GPT-5 Codex。
- 变更类型：V1.1 评分校准基线（Push 2：分布、风险与样本外报告）。
- 涉及文件：
  - `crates/strategy-evaluation/src/lib.rs`
  - `crates/strategy-evaluation/data/generated/calibration-v1.report.json`
  - `STRATEGY_CALIBRATION_BASELINE_V1.md`
  - `V1_1_PLAN.md`
- 变更内容：
  - 输出真实生产规则的原始/方向变换/加权层均值、最终分数分位数、动作分布、机会滚存现金、XIRR、期末净值、时间加权最大回撤、年化波动率、现金使用率和 24 个月滚动样本外窗口。
  - 回撤与波动率使用剔除固定月度外部入金影响的时间加权净值，而非把充值误计为收益波动。
  - 基线结果如实显示当前 Core/Opportunity 意图和当前 API 实效口径在两条指数代理上均落后于同现金流固定 DCA；报告同时标出 API 全局 `Skip`/`TacticalDelay` gate 仍会阻断领域层保留的核心金额这一实现差异。
  - 冻结 Qwen 敏感性只比较 70/20/10 与 90/10/0 的同样本分数/动作变化，不构造历史 AI 收益结论。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p strategy-evaluation --locked` 通过。
  - `cargo test -p core-domain --locked` 通过。
  - `cargo run -p strategy-evaluation --locked -- crates/strategy-evaluation/data/generated/calibration-v1.report.json` 通过。

### 2026-08-21 CST

- 执行模型：GPT-5 Codex。
- 变更类型：V1.1 评分校准基线（Push 1：离线数据夹具与评估器）。
- 涉及文件：
  - `Cargo.toml`、`Cargo.lock`
  - `crates/strategy-evaluation/**`
  - `tools/generate_calibration_fixture.py`
- 变更内容：
  - 新增独立、离线的 `strategy-evaluation` crate；它只读取提交到仓库的版本化数据，调用既有 `evaluate_fundamental`、`evaluate_trend`、`evaluate_decision` 与 `TwoBucketContributionSplit`，不访问 API、不读取 Qwen Key、也不下单。
  - 固定 `calibration-v1` 数据集、原始来源快照、生成器和 SHA-256 清单；美国权益范围限定为 S&P 500 与 NASDAQ Composite 指数代理，明确不是 ETF 成交价。
  - 历史因果线固定为 90/10/0 降级；另以独立冻结 JSON 提供 Qwen 敏感性样本，仅观察动作和分数分布，禁止用于历史收益结论。
  - 评估器只将日期 `t` 之前的 60 个按月样本传入生产函数；对照线以相同月度现金流、买入成本和期末现金口径比较固定 DCA、Core/Opportunity 意图与当前 API 实际门控口径。
- 验证：
  - `python3 tools/generate_calibration_fixture.py` 通过。
  - `cargo test -p strategy-evaluation --locked` 通过（3 个聚焦测试）。
  - `cargo run -p strategy-evaluation --locked -- crates/strategy-evaluation/data/generated/calibration-v1.report.json` 通过。

### 2026-08-20 15:30 CST

- 执行模型：GPT-5 Codex。
- 变更类型：V1.1 Push 4 周期执行可靠性与账本闭环。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/src/state.rs`
  - `crates/storage/src/sqlite_opportunity_cash.rs`
  - `crates/storage/src/sqlite_period_execution.rs`
  - `crates/storage/src/sqlite_paper_performance.rs`
  - `migrations/sqlite/20260820150000_add_execution_reliability.sql`
  - `migrations/20260820150000_add_execution_reliability.sql`
  - `V1_1_PLAN.md`、`API_MANAGEMENT.md`、`readme.md`
- 变更内容：
  - 新增 `carry_with_cap` 与正金额机会现金上限；保留 `carry_forward`/`expire_each_period` 兼容语义。
  - 自动来源 paper order 在提交前对同一周/月的累计建议金额执行 SQLite 原子预留；提交失败释放预留，broker 接受后确认。终态 `filled`/`closed` 订单按实际成交量与均价回写周期占用及机会现金；pending/partial 保守保留预估额度。
  - scheduler 现在会补跑当前月或当前周尚未 claim 的固定日期，使用 `(plan_id, scheduled_for)` 维持幂等；补跑仍只生成审计存证，不自动下单。
  - 文档同步更新实施状态与后续顺序：评分校准、Qwen 解释层、API/Web 展示。
- 验证：
  - `cargo test -p investment-plans -p indexlink-storage -p indexlink-api --locked` 通过。

### 2026-08-20 14:20 CST

- 执行模型：GPT-5 Codex。
- 变更类型：本地演示素材忽略规则。
- 涉及文件：
  - `.gitignore`
  - `CHANGE_LOG.md`
- 变更内容：将 `apps/web/public/demo-screenshots/` 与 `apps/web/public/demo-videos/` 标记为本地演示素材，不再进入 Git 工作区待提交列表。
- 验证：`git status --short` 不再列出上述两个目录。

### 2026-08-20 14:00 CST

- 执行模型：GPT-5 Codex。
- 变更类型：V1.1 Push 3 周期调度、机会现金滚存与自动订单金额闭环。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `crates/api/src/lib.rs`
  - `crates/api/src/state.rs`
  - `crates/api/src/routes/investment_plans.rs`
  - `crates/api/src/routes/decision_preview.rs`
  - `apps/server/src/main.rs`
  - `crates/storage/src/sqlite_investment_plans.rs`
  - `crates/storage/src/sqlite_opportunity_cash.rs`
  - `crates/storage/src/sqlite_scheduled_decisions.rs`
  - `migrations/sqlite/20260820130000_add_schedule_days_and_opportunity_cash.sql`
  - `migrations/20260820130000_add_schedule_days_and_opportunity_cash.sql`
- 变更内容：
  - 计划的月度/周度规则支持一个或多个固定日期；`schedule_day` 保留为首项兼容字段，`schedule_days` 是实际调度集合。scheduler 按 UTC 当前月内日或 ISO 星期匹配，并继续用 `(plan_id, UTC 日期)` 本地账本幂等去重。
  - SQLite 新增独立 `opportunity_cash_balances` 与 `opportunity_cash_events`，不混入账户充值/提现 `cash_flows`。仅在自动来源决策的 broker 接受订单后，按 decision record 唯一键原子结算机会桶余额；`carry_forward` 保存未分配机会预算，`expire_each_period` 归零。
  - Decision Preview 会读入滚存余额，并在 `max_single_execution` 硬上限内使用它；核心桶仍不可被趋势或 AI 否决。自动预览的 paper buy 使用建议金额、最新本机 OpenD 收盘价和 broker buying power 换算保守整股数量；前端提供的旧数量只能与后端计算结果一致，不能放大订单。
  - 手动 legacy Decision Preview 保持现有显式数量契约，避免未经前端迁移就改变已有演示流程；新金额到数量闭环用于自动来源决策路径。
- 未完成边界：
  - scheduler 继续只生成审计决策，不会自动向 broker 提交订单；`approval` 仍须用户确认。legacy 手动 preview 为兼容旧 UI 保留显式数量，尚不结算机会现金。
  - market order 只能以最新本机收盘价作预算估计，最终成交价/部分成交由既有 paper performance 对账处理；尚未实现自动按成交回写机会余额、每周期预算上限或 `carry_with_cap`。
  - PostgreSQL migration 保持 schema 对等，但本 MVP 的实际运行、回归测试和现金账本均以本地 SQLite 为准。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p investment-plans -p indexlink-storage -p indexlink-api --locked` 通过。
  - `cargo test -p core-domain --locked` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p investment-plans -p indexlink-storage -p indexlink-api -p indexlink-server --all-targets --all-features --locked -- -D warnings` 通过。

### 2026-08-20 12:00 CST

- 执行模型：GPT-5 Codex。
- 变更类型：V1.1 Push 2 双桶建议金额与机会资金策略占位。
- 涉及文件：
  - `crates/investment-plans/Cargo.toml`
  - `crates/investment-plans/src/lib.rs`
  - `crates/storage/src/sqlite_investment_plans.rs`
  - `crates/storage/src/investment_plans.rs`
  - `migrations/sqlite/20260820110000_add_opportunity_cash_policy.sql`
  - `migrations/20260820110000_add_opportunity_cash_policy.sql`
  - `crates/api/src/routes/investment_plans.rs`
  - `crates/api/src/routes/decision_preview.rs`
  - `API_MANAGEMENT.md`
  - `V1_1_PLAN.md`
  - `README.md`
- 变更内容：
  - 计划配置新增持久化的 `opportunity_cash_policy`：`expire_each_period` 或 `carry_forward`；核心桶为 `100%` 时拒绝无意义的滚存策略。
  - 双桶从静态比例拆分升级为可复用的金额建议：核心桶始终保留，机会桶按已有有界倍率调整；`Skip`/`TacticalDelay` 仅令机会桶建议额为零，不能否决核心桶。
  - 在不存在已审计现金池余额的当前阶段，机会桶建议额不会超过其当期预算；返回机会预算、采用倍率、未分配金额、建议总额、现金策略意图和审批要求。
  - `Decision Preview` 与执行预览均读取计划的持久化配置，预览请求不再允许覆盖计划双桶比例；旧 decision-preview 请求中的比例字段仅为兼容而解析，不影响结果。
  - `carry_forward` 当前只保存用户意图和审计输出；现金池余额、`carry_with_cap`、真实可用现金校验、调度与订单金额联动均明确留待 Push 3。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p investment-plans -p indexlink-storage -p indexlink-api --locked` 通过。
  - `cargo test -p core-domain --locked` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p investment-plans -p indexlink-storage -p indexlink-api --all-targets --all-features --locked -- -D warnings` 通过。

### 2026-08-20 10:30 CST

- 执行模型：GPT-5 Codex。
- 变更类型：V1.1 计划配置与本地数据库迁移。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `crates/storage/src/sqlite_investment_plans.rs`
  - `crates/storage/src/investment_plans.rs`
  - `crates/storage/src/sqlite.rs`
  - `crates/api/src/routes/investment_plans.rs`
  - `crates/api/src/routes/decision_preview.rs`
  - `migrations/sqlite/20260820090000_add_plan_execution_configurations.sql`
  - `migrations/20260820090000_add_plan_execution_configurations.sql`
  - `API_MANAGEMENT.md`
- 变更内容：
  - 新增持久化的计划执行配置：核心/机会桶比例、`fixed` / `autopilot` / `approval` 风险模式，以及月度或周度固定执行日。
  - 用领域构造器保证：核心桶为 `100%` 时只能选择 `fixed`；存在机会桶时必须明确选择 `autopilot` 或 `approval`。旧 HTTP 创建请求默认回退为 `100%` 核心桶和固定模式。
  - 新增 SQLite 与 PostgreSQL migration；SQLite 通过独立配置表回填旧月度计划，避免重建已有主表并确保既有审计、账本和外键记录保持兼容。
  - 周度配置已可读写，但当前 scheduler 与执行预览仍只运行既有月度规则；周度计划会安全等待/跳过，不产生自动订单或错误触发。
  - 更新 API 管理文档，说明比例表示、风险模式不变量、向后兼容和周度调度边界。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p investment-plans -p indexlink-storage -p indexlink-api --locked` 通过。
  - `cargo test -p core-domain --locked` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p investment-plans -p indexlink-storage -p indexlink-api --all-targets --all-features --locked -- -D warnings` 通过。

### 2026-08-20 10:00 CST

- 执行模型：GPT-5 Codex。
- 变更类型：V1.1 策略与执行升级规划。
- 涉及文件：
  - `V1_1_PLAN.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增双语 V1.1 计划，明确当前双桶只是单 ETF 的静态金额拆分；规划升级为核心固定定投与机会自适应定投。
  - 规划用户可选周期、风险模式、机会现金滚存政策、评分校准、AI 决策解释、金额到 paper order 的可审计闭环，以及分阶段验收标准。
  - 明确不引入高频、Martingale、不可审计黑盒训练或真实交易能力；权重和阈值必须先经固定样本与样本外验证，不能为收益结果临时调参。
- 验证：
  - `git diff --check` 通过。
  - 本次仅新增规划文档，未修改策略函数、数据库 schema、订单逻辑或外部账户配置。

### 2026-07-20 11:25 AEST

- 执行模型：GPT-5。
- 变更类型：新增三次 5 交易日复判的历史控制变量实验。
- 涉及文件：
  - `ADAPTIVE_DCA_THREE_RECHECK_EXPERIMENT_2026-07.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在不修改一次复判实验报告的前提下，直接复用其 30 个历史样本、资金画像、SPY 数据与普通定投基准，新增独立的三次复判实验。
  - 对首次为 `TacticalDelay` 或 `Skip` 的周期在 `+5`、`+10`、`+15` 个交易日重算信号；第三次仍不可执行才将本期金额滚存，滚存金额在后续可执行时不占用当月新增额度。
  - 只读重放得到 214 次复判、48 个三次后仍未执行周期；自适应 8/30 组终值较高，平均终值相对差为 `-1.65%`，相比一次复判的 `-1.83%` 改善 `0.18` 个百分点。结果不作为跑赢承诺。
- 验证：
  - 30 组配对离线只读重放完成；未提交、撤销或修改任何 OpenD 订单。
  - `git diff --check` 通过。

### 2026-07-20 11:09 AEST

- 执行模型：GPT-5。
- 变更类型：历史对照实验执行日、延迟复判与现金滚存规则校正。
- 涉及文件：
  - `ADAPTIVE_DCA_EXPERIMENT_2026-07.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将普通定投与自适应定投统一为每月最后一个可用交易日的同一执行机会；普通定投固定投入该月现金流，自适应仅改变该次及滚存资金的投入金额。
  - 对 `TacticalDelay` 或 `Skip` 固定在 5 个交易日后复判一次；复判仍未执行的资金滚存到后续月份，后续执行时滚存部分不占用当月新增月度额度。
  - 使用无前视的日级技术输入和已完成月度基本面序列重跑 30 组历史对照；自适应终值较高为 8/30，平均终值相对差为 `-1.83%`。报告保留旧规则 `-6.11%` 仅用于说明改进幅度，不将任何结果表述为保证跑赢。
- 验证：
  - 离线只读重放完成 30 组配对、102 次固定 5 日复判；未提交、撤销或修改任何 OpenD 订单。
  - `git diff --check` 通过。

### 2026-07-20 10:55 AEST

- 执行模型：GPT-5。
- 变更类型：70/20/10 决策引擎功能回归矩阵与历史降级控制变量实验。
- 涉及文件：
  - `crates/decision-engine/tests/scenario_matrix.rs`
  - `ADAPTIVE_DCA_EXPERIMENT_2026-07.md`
  - `docs/experiments/adaptive_dca_backtest_2026-07.md`（迁移至仓库根目录后删除）
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 30 个带使用者类型、起始金额、月度金额与历史时点名称的冻结历史场景，验证 Qwen 不可用时严格使用 `90/10/0` 降级，不伪造历史 AI 信号。
  - 新增 10 个冻结 Qwen 情绪场景，验证 Qwen 可用时正常使用 `70/20/10`，覆盖谨慎、积极、均衡、下跌风险和过热风险动作。
  - 将实验报告移至根目录，双语记录同一现金流和估值日期下的 30 组普通定投/自适应终值对照、数据边界与不可得出的结论；结果如实记录为自适应 5/30 组终值较高、平均终值相对差 `-6.11%`，不将历史降级结果包装为完整 Qwen 回测。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p decision-engine --locked` 通过（既有单元测试 10 个、场景测试 40 个）。
  - `cargo test -p core-domain --locked` 通过（13 个测试）。
  - `cargo llvm-cov -p decision-engine --tests --summary-only` 通过：decision-engine 行覆盖率 `99.63%`、函数覆盖率 `100.00%`。

### 2026-07-20 01:53 AEST

- 执行模型：GPT-5。
- 变更类型：README 双语问题引入与 ECS 架构关系补充。
- 涉及文件：
  - `README.md`
  - `readme.en.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在中英文 README 开头增加有限预算、执行纪律与可核查决策依据的用户痛点，明确系统不承诺预测市场。
  - 在中英文 Layered Overview 中增加 Web/API 客户端、Alibaba Cloud ECS（Docker Compose + SQLite volume）和 Model Studio DashScope/Qwen 的运行时关系；保留既有分层与决策边界。
  - 将 Rust 后端描述更新为支持本机或 ECS Docker Compose 运行，保持 SQLite 本地持久化模型。
- 验证：
  - `git diff --check` 通过。
  - `cargo test -p core-domain --locked` 通过。

### 2026-07-20 01:35 AEST

- 执行模型：GPT-5。
- 变更类型：Alibaba Cloud ECS 可复现部署入口。
- 涉及文件：
  - `deployment/docker-compose.yml`
  - `deployment/aliyun/ecs-deploy.sh`
  - `deployment/aliyun/README.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - Compose 从仓库根目录、受 Git 忽略的 `.env` 读取 Qwen 等运行时配置，继续以 named volume 持久化 SQLite；密钥不进入镜像、脚本或版本库。
  - 增加 ECS 部署脚本：校验 Docker 和本地 `.env`，构建并后台启动服务，再通过 `/ready` 轮询确认可用。
  - 增加阿里云部署说明与竞赛证明路径，指向实际 DashScope/Qwen provider 实现；明确公网 ECS 不配置 OpenD 或任何交易凭据。
  - 将 Compose 的本地 Vite CORS 默认端口从过时的 `3000` 校正为 `5173`。
- 验证：
  - `bash -n deployment/aliyun/ecs-deploy.sh` 通过。
  - `cargo test -p core-domain --locked` 通过。

### 2026-07-20 00:40 AEST

- 执行模型：GPT-5。
- 变更类型：MVP 状态审查与 README 运行说明校正。
- 涉及文件：
  - `README.md`
  - `readme.en.md`
  - `.env.example`
  - `CHANGE_LOG.md`
- 变更内容：
  - 以当前实现更新 README：明确本机可演示 MVP 已覆盖计划管理、自动 70/20、Qwen 证据、固定月日自动存证、双桶、paper order、SQLite 账本与图表；同时明确 Scheduler 不自动下单、仅 paper trading、固定月执行日和简化历史回放等边界。
  - 修正 Dashboard 演示步骤：前端使用服务端自动市场输入，不再要求手工填写或导入 70/20 JSON。
  - 补充 Qwen/OpenD 本地配置说明，并将 `.env.example` 的 Vite CORS 示例端口校正为 `5173`。
  - 在 README 增加 MIT 发布说明及 Jame、Xuanzhou Gu、Yucong Peng 贡献者信息。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test --workspace --locked` 通过；4 个需要真实 Qwen/OpenD/公开市场数据的 smoke 按预期 ignored。
  - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` 通过。
  - `pnpm --dir apps/web lint` 与 `pnpm --dir apps/web build` 通过；仅保留既有 bundle 大小非阻断警告。

### 2026-07-20 00:15 AEST

- 执行模型：GPT-5。
- 变更类型：Vite 开发代理图表接口修复。
- 涉及文件：
  - `apps/web/vite.config.ts`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 `/market-data` 与 `/paper-performance` 增加到本机 Rust `8080` 的 Vite dev proxy，覆盖历史走势、真实收益轨迹和一年历史回放所使用的 API。
  - 根因：后端接口本身均返回 `200`，但开发服务器此前未转发这两个路径，浏览器请求被 Vite SPA fallback 吞掉，导致页面误报“Rust API 未提供此功能版本”。
- 验证：
  - 临时启动更新后的 Vite 并验证：`GET /market-data/holdings?period=1y` 与 `GET /paper-performance/historical-backtest` 均经代理返回 Rust 的 JSON `200`。
  - `pnpm --dir apps/web lint` 与 `pnpm --dir apps/web build` 通过。

### 2026-07-20 00:05 AEST

- 执行模型：GPT-5。
- 变更类型：Dashboard 模块层级与操作入口视觉修正。
- 涉及文件：
  - `apps/web/src/pages/dashboard/index.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将“刷新真实轨迹”“拉取走势”“运行一年回放”迁至各模块内容区并水平居中；走势周期选择与拉取操作作为同一居中组展示。
  - 为账户账本、真实轨迹、历史走势与一年回放建立由白/浅灰到天蓝、浅蓝、靛蓝的渐进色层，增强模块边界但不改变任何数据语义。
  - 复核本机最新 server：`GET /market-data/holdings?period=1y` 与 `GET /paper-performance/historical-backtest` 均返回 `200`；此前“路由未提供”提示来自旧 server/旧页面状态，刷新后可直接重试。
- 验证：
  - `pnpm --dir apps/web lint` 与 `pnpm --dir apps/web build` 通过。
  - `cargo test -p core-domain --locked` 通过。

### 2026-07-19 23:55 AEST

- 执行模型：GPT-5。
- 变更类型：前端 Qwen 降级状态空值修复。
- 涉及文件：
  - `apps/web/src/api/types.ts`
  - `apps/web/src/pages/{dashboard,decisions}/index.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 API 契约中的 `sentiment_score` 表达为 `number | null | undefined`：历史记录可缺省，当前 Qwen 不可用时 Rust 会序列化为 `null`。
  - 所有 Dashboard 与决策存证展示改为仅在值确为数字时调用 `toFixed()`；`null` 与缺省均显示“AI 暂不可用 / 已降级”，不再触发 React 错误页。
- 验证：
  - `pnpm --dir apps/web lint` 与 `pnpm --dir apps/web build` 通过。
  - `cargo test -p core-domain --locked` 通过。

### 2026-07-19 23:45 AEST

- 执行模型：GPT-5。
- 变更类型：Qwen 结构化情绪输出容量修复。
- 涉及文件：
  - `crates/ai-client/src/{provider.rs,client.rs}`
  - `crates/ai-client/tests/provider.rs`
  - `apps/server/src/config.rs`
  - `.env.example`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Qwen `max_tokens` 默认值及本地示例配置从 `128` 提升到 `256`，为 `score`、`rationale` 与最多五条 `warnings` 的结构化输出保留足够空间。
  - 根因验证：DashScope Key 与 CNBC RSS 均可用；旧上限下完整解释可能在 JSON 闭合前被截断，解析器按安全契约拒绝并令 API 返回 `503`。使用 `256` 后 `POST /market-sentiment/preview` 返回包含理由、风险提示和新闻来源的 `200` 响应。
- 验证：
  - 本机真实 DashScope/Qwen smoke 通过：结构化情绪路由返回 `200`，未输出或持久化 Key。
  - `cargo test -p ai-client -p indexlink-server --locked` 通过。

### 2026-07-19 23:35 AEST

- 执行模型：GPT-5。
- 变更类型：Demo 决策入口去重 / Qwen 可见性与本机 API 故障说明。
- 涉及文件：
  - `apps/web/src/api/queries.ts`
  - `apps/web/src/pages/dashboard/index.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - Dashboard 删除与“定投标的”页面重复的计划创建表单；没有计划时只提供明确的页面跳转入口。
  - 增加独立的“AI 市场情绪”卡片，可直接调用 Qwen 情绪 API 并展示理由、风险提示和新闻来源，无需先生成决策。
  - 自动决策不再在调用自动预览前额外重复拉取一次 70/20 数据，减少一次易受公开数据源波动影响的请求；走势图和一年回放在自身请求失败时保留对应位置的安全、可操作提示。
  - 前端请求错误保留公开 HTTP 状态；自动决策路由返回 404、或旧版 Qwen 仅返回 score/label 时明确提示本机 `indexlink-server` 仍是旧进程，避免将版本未重启误报为模拟账户或市场数据不可用。
- 验证：
  - 以本机正在运行的旧 server 验证：`GET /signals/market-input/VOO`、`GET /market-data/holdings?period=1y`、`GET /paper-performance/historical-backtest` 均返回 `200`；旧进程对新自动决策路由返回 `404`，确认故障原因是未重启到最新 Rust 二进制。
  - `pnpm --dir apps/web lint` 通过。
  - `pnpm --dir apps/web build` 通过；仅保留既有首个 JS bundle 大于 500 kB 的非阻断警告。

### 2026-07-19 23:15 AEST

- 执行模型：GPT-5。
- 变更类型：最小固定月日自动决策 / 可读 AI 决策存证 / Demo 文档校正。
- 涉及文件：
  - `migrations/sqlite/20260719203000_create_scheduled_decision_runs.sql`
  - `crates/storage/src/{lib.rs,sqlite.rs,sqlite_scheduled_decisions.rs}`
  - `crates/api/src/{lib.rs,state.rs,routes/{mod.rs,decision_preview.rs}}`
  - `crates/api/tests/decision_preview.rs`
  - `apps/server/src/{config.rs,main.rs}`
  - `apps/web/src/{api/{queries,types}.ts,pages/{dashboard,decisions}/index.tsx,i18n/locales/{zh,en}.ts}`
  - `.env.example`、`README.md`、`readme.en.md`、`API_MANAGEMENT.md`、`CHANGE_LOG.md`
- 变更内容：
  - 新增 SQLite `scheduled_decision_runs` 幂等 claim；server 默认每 60 秒检查一次到期的活跃 monthly 计划。相同计划在同一 UTC 日期最多生成一条自动决策存证，服务重启不会重复写入。
  - Scheduler 与新的 `POST /investment-plans/:id/automatic-decision-preview` 共用服务器自动行情链路：70%/20% 由 OpenD、Shiller CAPE、国债和 VIX 数据计算，10% 由 Qwen 管线提供；70/20 数据不可用时不创建伪造记录，Qwen 不可用时明确降级为 `90/10/0`。
  - Scheduler 不携带订单数量，也从不自动提交 paper order。自动入口只有在操作者明确提供 `paper_order` 时才通过既有 due/action 门控提交模拟订单。
  - 决策存证新增触发方式、70/20 自动来源披露和 `audit_record_id`；Dashboard 隐藏手填/导入 70/20 信号，改用自动拉取；决策详情页将原始 JSON 改为时间、计划金额、70/20/10 分层、AI 理由/新闻/风险提示、订单意图与回执等可读卡片。
  - README 与 API 管理文档改为反映当前实现：固定月日、UTC、自动审计但不自动下单、仅 paper trading；下一阶段明确列出“每 1–31 天周期审计 + 每计划月度上限 + 跨月/补跑规则”。
- 验证：
  - `cargo fmt --all` 通过。
  - `cargo test -p indexlink-api -p indexlink-storage -p indexlink-server --locked` 通过：API 48 个、storage 34 个、server 27 个测试；一个需要显式确认的真实 OpenD paper smoke 按预期 ignored。
  - `cargo test --workspace --locked` 与 `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` 通过；2 个真实 Qwen/news smoke、1 个真实 OpenD paper smoke 与 1 个本机 OpenD 行情 smoke 按预期 ignored。
  - `pnpm --dir apps/web lint` 与 `pnpm --dir apps/web build` 通过；Vite 仅报告既有首个 JS bundle 大于 500 kB 的非阻断警告。

### 2026-07-19 22:33 AEST

- 执行模型：GPT-5。
- 变更类型：Qwen 结构化情绪依据 / 本地审计快照 / 前端决策解释。
- 涉及文件：
  - `crates/ai-client/src/{client,lib,news,provider,sentiment}.rs`
  - `crates/ai-client/tests/client.rs`
  - `crates/api/src/{state.rs,routes/{market_sentiment,decision_preview}.rs}`
  - `crates/api/tests/{market_sentiment,decision_preview}.rs`
  - `apps/web/src/{api/types.ts,pages/{dashboard,decisions}/index.tsx,i18n/locales/{zh,en}.ts}`
  - `API_MANAGEMENT.md`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - Qwen system prompt 与解析契约从仅 `{"sentiment": number}` 升级为 `{"score": number, "rationale": string, "warnings": string[]}`；分数继续由 `Sentiment` 限制在 `[-1.0, 1.0]`，依据必须非空且受长度限制，风险提示最多五条且逐条校验。
  - RSS 新闻项新增原始 HTTP(S) 链接；`market_sentiment` pipeline 返回模型依据、风险提示和实际送入模型的标题/链接/UTC 发布时间。链接仅从 RSS 原始输入生成，拒绝非 HTTP(S) 协议，模型不能编造来源。
  - `POST /market-sentiment/preview` 与 `POST /investment-plans/:id/decision-preview` 现在返回结构化 AI 情绪依据；Decision Preview 通过既有本地 SQLite `sentiment_snapshot` 保存 score、rationale、warnings 和 headlines，不保存新闻正文、Key、provider URL、账户信息或内部错误。
  - Dashboard 和决策详情页将新决策展示为“AI 情绪依据 / 新闻来源 / 风险提示”，历史只有旧 score 的记录保留明确降级说明，不伪造缺失内容。
- 验证：
  - `cargo test -p ai-client --locked` 通过：81 单元测试、11 本地回环 mock HTTP 集成测试、7 provider 测试、4 doc tests；2 个真实网络 smoke 按预期 ignored，未调用真实 Qwen 或暴露 API Key。
  - `cargo test -p indexlink-api --locked` 通过：36 个 API 测试，覆盖结构化情绪响应、Decision Preview 审计快照与不可用降级。
  - `cargo test -p core-domain --locked` 通过：13 个测试。
  - `cargo check --workspace --locked`、`cargo fmt --all -- --check`、`cargo clippy -p ai-client -p indexlink-api --all-targets --all-features --locked -- -D warnings` 与 `pnpm --dir apps/web build` 通过。

### 2026-07-19 22:05 AEST

- 执行模型：GPT-5。
- 变更类型：本机 OpenD 只读 50 组资金情景回测报告替换。
- 涉及文件：
  - `docs/experiments/adaptive_dca_backtest_2026-07.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 按用户要求删除报告内此前 30 组实验内容，改为 50 组分层资金情景对比；覆盖 1、2、3、4 个资产，以及 SPY、QQQ、IEF、GLD、VT 五类 ETF。
  - 将学生、毕业生、家庭、富裕家庭、高净值与机构资金的初始资金、月入金、普通定投频率和自适应月度上限纳入控制变量；每个情景与五个组合配对，普通/自适应使用完全一致的外部现金流。
  - 允许高净值和机构情景使用无额外月度金额上限的自适应配置；未部署资金仍保留为期末 0% 现金，避免少投入被误报为高回报。
  - 50 组整体普通定投平均 XIRR 为 16.01%，自适应为 15.10%，平均差异 -0.91 个百分点；报告同时给出各行期末值、XIRR、差异、对应市场走势和局限，未对负结果做筛选或优化。
- 验证：
  - 本机 OpenD 11111 回环端口连通；SPY、QQQ、IEF、GLD、VT 在窗口内各读取 250 个日线观测。
  - 所有 50 组使用同一价格窗口、同一策略公式与成对现金流，XIRR/期末价值均可由报告中的参数重算。
  - 本实验没有提交、撤销、修改或模拟订单；读取 GLD/VT 数据的临时计划会在完成后删除。

### 2026-07-19 21:42 AEST

- 执行模型：GPT-5。
- 变更类型：本机 OpenD 只读历史数据实验报告。
- 涉及文件：
  - `docs/experiments/adaptive_dca_backtest_2026-07.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 使用本机 OpenD 返回的 SPY、QQQ、IEF 三年日线，以 2025-07-19 至 2026-07-17 的一年窗口完成单资产、双资产、三资产各 10 组、共 30 组普通定投/自适应定投对比。
  - 固定每组初始资金 $1,200、相同日期和金额的后续入金、相同资产权重与相同估值日；未部署的自适应资金保留为 0% 现金并纳入期末价值，使用 XIRR/MWR 而不是简单累计收益率比较。
  - 报告中明确记录输入价格、规则、参数、30 行完整结果、各组合平均、市场走势、局限和下一步。该价格-MA200 回放不伪造历史 CAPE/ERP/VIX/Qwen 输入，因此不声称是完整 70/20/10 回测。
  - 本次样本期以美股上涨为主：普通定投平均 XIRR 18.83%，自适应平均 16.97%，平均差异 -1.86 个百分点；报告如实记录了未取得平均收益增长的结果。
- 验证：
  - 本机 OpenD 11111 回环端口连通，三只标的各读取到 250 个窗口内日线观测。
  - 已对三种组合各执行 10 组 deterministic parameter replay；报告中现金流、期末价值与 XIRR 可由记录的规则重算。
  - 本实验没有提交、撤销、修改或模拟订单。

### 2026-07-19 21:15 AEST

- 执行模型：GPT-5。
- 变更类型：Fork `main` 多标的长线图、历史回放与定投标的管理。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `crates/storage/src/{lib.rs,sqlite_investment_plans.rs,sqlite_paper_performance.rs}`
  - `crates/market-data/src/lib.rs`
  - `crates/api/src/{lib.rs,state.rs,routes/{investment_plans.rs,market_data.rs,paper_performance.rs}}`
  - `crates/api/tests/investment_plans.rs`
  - `apps/web/src/{api/{queries.ts,types.ts},pages/{dashboard,plans}/index.tsx,i18n/locales/{zh.ts,en.ts}}`
  - `API_MANAGEMENT.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `DELETE /investment-plans/:id`；SQLite 通过既有外键级联删除该定投标的关联的决策记录、本地账本、成交和快照。前端将“定投计划”收敛为可覆盖股票/ETF 的“定投标的”，并提供二次确认删除入口。
  - 新增 `GET /paper-performance/actual`：一次只读 OpenD 模拟账户刷新所有启用标的的本机 SQLite 快照，返回每个标的线及按日去重后的总和线；不提交、撤销或修改订单。
  - 新增 `GET /market-data/holdings?period=3m|6m|1y|3y`：从本机 OpenD 返回所有启用标的的真实日线与本地已确认的模拟买卖标记。多标的前端以区间首日 `100` 归一化同图比较，绿色/红色点只代表本地账本中已观察到的买/卖成交。
  - 新增 `GET /paper-performance/historical-backtest`：使用三年真实 OpenD 日线、每月末价格与真实 200 日均线距离，回放最近一年普通定投和 `0.5x–1.5x` 有界自适应投入的汇总价值线。历史 Qwen 情绪和宏观快照尚无按月审计来源，因此接口与页面明确声明其为价格规则回放，绝不伪造成完整 70/20/10 历史决策或已实现账户收益。
  - Dashboard 增加三张长线图：真实组合轨迹（逐标的 + 总和）、可切换三个月/六个月/一年/三年的多标的走势与买卖点、一年普通定投/自适应价格回放。空数据统一显示“等待首次成交 / 暂无数据”。
  - 已按用户明确授权清空本机 SQLite 的历史计划与关联记录；不删除代码、不改动 `upstream`，只向 Fork `origin/main` 提交。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test -p investment-plans --locked` 通过（31 tests）、`cargo test -p indexlink-storage --locked` 通过（33 tests，含 SQLite 删除契约）、`cargo test -p indexlink-api --locked` 通过（46 tests，含 HTTP 删除契约）、`cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` 通过。
  - `pnpm --dir apps/web lint` 通过。
  - `pnpm --dir apps/web build` 通过；Vite 仅提示现有首个 JS bundle 超过 500 kB，未阻断构建。
  - 本机 `indexlink.db` 清理完成：删除前为 4 个定投标的、8 条决策、1 条本地订单、0 条 fill、0 条现金流、5 条快照；清理后上述六类记录均为 0。

### 2026-07-19 20:24 AEST

- 执行模型：GPT-5。
- 变更类型：Fork `main` Dashboard 可读性与决策解释改进。
- 涉及文件：
  - `apps/web/src/pages/dashboard/index.tsx`
  - `apps/web/src/i18n/locales/{zh.ts,en.ts}`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将自动市场信号、模拟账户和收益账本的刷新按钮改为卡片内居中布局，避免宽屏下操作入口集中在左侧。
  - 移除 Dashboard 中直接展示的原始英文技术 summary；改为可读的基本面、趋势、Qwen 情绪、权重降级和最终倍率解释，不把内部字段串暴露给用户。
  - 市场输入快照说明其含义，并在成功自动拉取后展示 CAPE、ERP、MA200、RSI、VIX 及价格/基本面/波动率来源。
  - 明确现有 Qwen 契约只返回有界情绪分数；页面显示真实新闻源与降级状态，但不虚构模型未返回的逐条新闻理由。
- 验证：
  - `pnpm --dir apps/web lint` 通过。
  - `pnpm --dir apps/web build` 通过；Vite 仅提示现有首个 JS bundle 超过 500 kB，未阻断构建。

### 2026-07-19 20:12 AEST

- 执行模型：GPT-5。
- 变更类型：Fork `main` 前端 OpenD 模拟账户代理修复。
- 涉及文件：
  - `apps/web/vite.config.ts`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将只读 `/paper-portfolio` 加入 Vite 开发服务器到本机 Rust API `127.0.0.1:8080` 的代理，修复 Dashboard 刷新模拟账户时请求未转发而显示 `request failed` 的问题。
  - 不改变 OpenD、订单、账户、数据持久化或实盘保护逻辑；该路由仍只读取模拟账户数据。
- 验证：
  - `pnpm --dir apps/web lint` 通过。
  - `pnpm --dir apps/web build` 通过；Vite 仅提示现有首个 JS bundle 超过 500 kB，未阻断构建。
  - `git diff --check` 通过。

### 2026-07-19 20:01 AEST

- 执行模型：GPT-5。
- 变更类型：Fork `main` 本地模拟账户账本、收益对账与真实曲线。
- 涉及文件：
  - `migrations/sqlite/20260719194000_create_paper_performance_ledger.sql`
  - `crates/storage/src/{lib.rs,sqlite.rs,sqlite_paper_performance.rs}`、`crates/storage/Cargo.toml`
  - `crates/api/src/{state.rs,routes/{mod.rs,decision_preview.rs,paper_performance.rs}}`
  - `apps/web/src/{api/{queries.ts,types.ts},pages/dashboard/index.tsx,i18n/locales/{zh.ts,en.ts}}`
  - `Cargo.lock`、`API_MANAGEMENT.md`、`CHANGE_LOG.md`
- 变更内容：
  - 新增仅本机 SQLite 的 `cash_flows`、`paper_orders`、`paper_fills`、`portfolio_snapshots` migration；金额固定精度存储，时间统一 UTC RFC3339 `Z`，不上传或写入云端。
  - 新增用户确认起始模拟资金接口，以及收益刷新接口；订单被 broker 接受后记录本地意图，后续只读 OpenD 刷新根据累计成交量/均价增量生成幂等 fill，使用 FIFO 计算已实现/未实现收益和本地估值点。
  - 新增自适应定投与普通定投的同执行价基准计算：普通定投仅在本地观察到买入执行后，按计划基准金额建立假想仓位；不补造启用账本前的历史数据。
  - Dashboard 增加起始资金确认、本地账本刷新、净投入/总收益/已实现和未实现收益，以及响应式真实曲线；本地账本与 provider 持仓不一致时明确显示不可完全核验提示。
  - 已完成的 MVP 后端闭环：本地计划、自动/手动信号、Qwen 情绪、70/20/10 决策、双桶、真实 OpenD 模拟订单、订单/持仓读取、决策审计与本地收益账本。前端不再以 mock 数值伪装收益。
  - MVP 仍需手动条件：本机 OpenD 登录并保持虚拟账户可用、配置 Qwen Key、确认每个计划的起始模拟资金；账本只能从启用后持续观测，无法反推既有完整交易/现金流历史。真实交易、云端同步、多用户认证、税费/分红/汇率处理、完整历史成交回补与多计划共享账户精确归因均不属于当前最小 MVP。
- 验证：
  - `cargo test -p indexlink-storage --offline` 通过（32 tests，覆盖 migration、FIFO fill 对账与幂等刷新）。
  - `cargo test -p indexlink-api --offline` 通过（45 tests）。
  - `cargo test -p core-domain --offline` 通过（13 tests）。
  - `cargo test --workspace --offline` 通过；仅跳过需要网络、真实 OpenD 或明确提交模拟订单确认的既有 smoke tests。
  - `cargo clippy --workspace --all-targets --all-features --offline -- -D warnings` 通过。
  - `pnpm --dir apps/web lint` 与 `pnpm --dir apps/web build` 通过；Vite 仅提示现有首个 JS bundle 超过 500 kB，未阻断构建。

### 2026-07-19 19:27 AEST

- 执行模型：GPT-5。
- 变更类型：Fork `main` CI Clippy 修复。
- 涉及文件：
  - `crates/broker/src/opend_session.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 移除 OpenD 模拟账户回执账户不匹配测试夹具中的冗余借用，兼容 GitHub Actions 当前 stable Rust 的 `clippy::needless_borrow` 检查。
  - 不改变 OpenD 请求、回执校验、订单行为或对外 API 契约。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo test -p broker --locked` 通过（38 tests）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `git diff --check` 通过。

### 2026-07-19 19:18 AEST

- 执行模型：GPT-5。
- 变更类型：Fork main OpenD 模拟账户真实读取 / Dashboard 去 Mock。
- 涉及文件：
  - crates/broker/src/{lib.rs,opend_session.rs}
  - crates/api/src/{state.rs,routes/{mod.rs,paper_portfolio.rs}}
  - crates/api/tests/paper_portfolio.rs
  - apps/web/src/{api/{queries,types}.ts,pages/dashboard/index.tsx,i18n/locales/{zh,en}.ts}
  - API_MANAGEMENT.md
  - CHANGE_LOG.md
- 变更内容：
  - 新增只读 GET /paper-portfolio：经既有 paper-only OpenD session 读取资金（2101）、美股持仓（2102）和近期美股订单（2201），严格核验模拟环境与已选择账户；不暴露 account id、原始 provider 文案或凭据。
  - OpenDPaperBroker 新增只读 portfolio port；MockBroker 不制造账户结果，未配置真实 OpenD 时仍返回统一 503 service_unavailable。
  - Dashboard 的账户净资产、可用现金、证券市值、持仓盈亏、持仓和近期订单改为显式刷新真实本机 API 数据，不再使用固定 SPY 数字或伪随机收益；刷新操作不下单、撤单、改价或解锁交易。
  - OpenD 模拟账户不支持成交列表读取，且本地尚未积累完整成交/现金流账本；历史总收益、已实现收益和普通定投对比曲线继续诚实显示“等待首次成交 / 暂无数据”，不伪造数值。
- 验证：
  - cargo test -p broker --locked 通过（37 tests；TCP fake 在本机 loopback 运行）。
  - cargo test -p indexlink-api --test paper_portfolio --locked 通过（2 tests）。
  - cargo check --workspace --locked 通过。
  - pnpm --dir apps/web lint 与 pnpm --dir apps/web build 通过；Vite 仅提示首个 JS bundle 超过 500 kB，未阻断构建。
  - 本机已登录 OpenD 的只读 smoke 成功：GET /paper-portfolio 返回 USD 资金、空持仓及近期 VOO 模拟订单；未发送、修改或撤销订单。

### 2026-07-19 18:18 AEST

- 执行模型：GPT-5。
- 变更类型：Fork `main` Dashboard 视觉布局恢复 / 真实数据联调。
- 涉及文件：
  - `apps/web/src/pages/dashboard/index.tsx`
  - `apps/web/src/i18n/locales/{zh,en}.ts`
  - `CHANGE_LOG.md`
- 变更内容：
  - 恢复 Dashboard 的概览式视觉层级：顶部计划/最近决策概览、70/20/10 分数卡、收益区域、最新决策、风险提示与市场输入快照；保留现有的真实计划创建、自动拉取、信号编辑、Decision Preview 和 paper-order 操作工作台。
  - 计划、最近持久化 decision record、最新决策分数、动作、倍率、计划金额、风险提示及本次自动拉取的原始市场指标均读取现有本机 Rust API 返回值；不恢复此前的固定 SPY、收益、新闻或伪随机曲线。
  - 收益、持仓收益、已实现收益、累计投入和收益对比图在尚无成交/持仓/成本账本时统一明确显示“等待首次成交 / 暂无数据”，不将 Mock 数值伪装为真实回报。
  - 明确剩余缺口：OpenD 订单状态、成交、现金与持仓读取；本地交易成本账本；基于真实成交及历史价格的普通定投/自适应定投回放曲线。自动市场输入仍只在当前浏览器会话显示；页面重载后需要再次拉取或从后续持久化快照读取。
  - 同步修正自动市场信号说明：数据源为本机 OpenD、公开 Shiller CAPE、Cboe 和美国财政部，不再误写为 FRED。
- 验证：
  - `pnpm --dir apps/web lint` 通过。
  - `pnpm --dir apps/web build` 通过；Vite 仅提示首个 JS bundle 超过 500 kB，未阻断构建。
  - `git diff --check` 通过。

### 2026-07-19 17:52 AEST

- 执行模型：GPT-5。
- 变更类型：Fork `main` 自动市场信号拉取 / Dashboard 联调。
- 涉及文件：
  - `Cargo.toml`、`Cargo.lock`
  - `crates/market-data/**`
  - `crates/api/{Cargo.toml,src/state.rs,src/routes/{mod.rs,market_data.rs},tests/market_data.rs}`
  - `apps/server/{Cargo.toml,src/main.rs}`
  - `apps/web/src/{api/{queries,types}.ts,pages/dashboard/index.tsx,i18n/locales/{zh,en}.ts}`
  - `API_MANAGEMENT.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增只读 `market-data` adapter 与 `GET /signals/market-input/:symbol`：通过本机 loopback OpenD 读取美股日线并本地计算 MA200 distance / 14 日 RSI；通过公开 Shiller CAPE 月度表、Cboe VIX 历史 CSV 和美国财政部十年期收益率生成最近 60 个历史快照。ERP 明确为 `100 / CAPE - 10 年期国债收益率` 代理口径，不伪装为前瞻预测。
  - OpenD 历史 K 线支持分页；自动拉取不读取交易账户、不提交订单、不传输或记录凭据。任何外部来源不可用时保持统一安全的 `503 service_unavailable`，内部错误仅写日志。
  - server 在已有 paper-only OpenD 配置存在时注入只读市场数据 provider；未配置 OpenD 时保持自动拉取不可用，不会静默使用模拟数据。
  - Dashboard 增加高可见度“自动拉取市场信号”卡片和刷新按钮，成功后将来源明确的数据填入仍可人工复核的信号字段；paper order 默认关闭且本次功能不改变订单门控。
  - 最终 Decision Preview 继续将使用到的输入快照写入本地 SQLite decision record；自动刷新本身不向云端持久化。
- 验证：
  - `cargo test -p market-data --offline` 通过（2 passed、1 ignored read-only local smoke）。
  - `cargo test -p indexlink-api --test market_data --offline` 通过（2 passed）。
  - `cargo test -p indexlink-api --offline` 通过。
  - `pnpm --dir apps/web lint` 与 `pnpm --dir apps/web build` 通过；Vite 仅提示首个 JS bundle 超过 500 kB，未阻断构建。
  - 真实 OpenD 日线只读协议探测成功；原 FRED public export 在本机 Rust 客户端超时，已替换为 Cboe 与美国财政部的公开 CSV 来源。`cargo test -p market-data local_opend_market_signal_smoke -- --ignored` 已在本机通过，仅读取 VOO/OpenD 与公开 CSV，不提交订单。

### 2026-07-19 17:00 AEST

- 执行模型：GPT-5。
- 变更类型：Fork `main` 本地演示数据保护。
- 涉及文件：
  - `.gitignore`
  - `CHANGE_LOG.md`
- 变更内容：
  - 忽略本机 pnpm 缓存与默认 SQLite 数据库的主文件、WAL 和 SHM 文件，防止演示计划、决策审计记录或本地构建缓存被误提交。
  - 不删除、不迁移或上传任何已有本地数据；仅在你的 Fork 本地 `main` 修改，不向 Jame `upstream` 推送或修改任何状态。
- 验证：
  - `git check-ignore` 确认本机 pnpm 缓存和 SQLite 文件均受忽略规则保护。
  - `git diff --check` 通过。

### 2026-07-19 16:55 AEST

- 执行模型：GPT-5。
- 变更类型：Fork `main` 前端开发代理联调修正。
- 涉及文件：
  - `apps/web/vite.config.ts`
  - `CHANGE_LOG.md`
- 变更内容：
  - Vite 开发服务器新增 `/market-sentiment` 到本机 Rust API `127.0.0.1:8080` 的代理，令 Dashboard 的 Qwen 情绪预览在开发模式下走真实后端，而非被 SPA 回退页处理。
  - 不改变 API 契约、Qwen 配置、订单门控或真实下单行为；仅在你的 Fork 本地 `main` 修改，不向 Jame `upstream` 推送或修改任何状态。
- 验证：
  - `pnpm --dir apps/web lint` 通过。
  - `pnpm --dir apps/web build` 通过；Vite 仅提示首个 JS bundle 超过 500 kB，未阻断构建。
  - 通过 Vite `POST /market-sentiment/preview` 的本机代理联调返回后端 JSON 响应。
  - `git diff --check` 通过。

### 2026-07-19 12:15 AEST

- 执行模型：GPT-5。
- 变更类型：Fork `main` 演示闭环页面完善 / 信号 JSON 导入。
- 涉及文件：
  - `apps/web/src/pages/dashboard/index.tsx`
  - `apps/web/src/i18n/locales/{zh,en}.ts`
  - `readme.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - Dashboard 现在包含可见的五步演示进度：创建或选择计划、输入或导入信号、Qwen 与决策、双桶拆分、paper-order 回执；状态只反映真实页面输入或 API 响应，不生成演示数据。
  - 在同一页面增加真实的计划创建表单，提交后复用既有 `POST /investment-plans`、选中新建计划，并继续进入当前 Decision Preview 流程。
  - 新增 JSON 信号文件导入：支持顶层 API 字段或 `fundamental` / `trend` 分组，导入时只接受有限数值和有限数值数组，并填入可继续手工审查、编辑的字段；后端仍负责不少于 60 条历史样本及全部领域校验。
  - 结果区更明确地标记 Qwen 成功返回的分数、`sentiment_unavailable` 降级，及已请求但因服务端 execution/action 门控而没有 paper-order 回执的情况；不会将没有回执的情况伪装为订单成功。
  - `readme.md` 补充本地演示闭环与导入文件字段说明。所有 HTTP 请求仍仅指向本机 Rust API；paper order 默认关闭。
  - 本次只修改并后续推送你的 Fork `main`，不向 Jame `upstream` 推送或修改其代码、分支、PR 或 `main`。
- 验证：
  - `pnpm --dir apps/web lint` 通过。
  - `pnpm --dir apps/web build` 通过；Vite 仅提示首个 JS bundle 超过 500 kB，未阻断构建。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `git diff --check` 通过。

### 2026-07-19 00:58 AEST

- 执行模型：GPT-5。
- 变更类型：Fork `main` 前端去 Mock 化 / 本机 API 联调。
- 涉及文件：
  - `apps/web/src/api/queries.ts`
  - `apps/web/src/api/types.ts`
  - `apps/web/src/api/mock.ts`
  - `apps/web/src/pages/dashboard/**`
  - `apps/web/src/pages/plans/index.tsx`
  - `apps/web/src/pages/decisions/index.tsx`
  - `apps/web/src/components/layout/{app-header,news-ticker}.tsx`
  - `apps/web/src/{stores/ui.ts,lib/decision.ts,i18n/**}`
  - `apps/web/vite.config.ts`
  - `readme.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 移除浏览器端的 mock 数据源、假收益/估值/新闻/风险卡片及其 Dashboard 子组件；在没有回测或行情持仓 API 的情况下，不再展示或暗示虚构收益。
  - React Query 现在直接调用本机 Rust API：`GET/POST /investment-plans`、两个 `/signals/*/preview`、`POST /investment-plans/:id/decision-preview`、`GET /investment-plans/:id/decisions` 与 `GET /decisions/:id`；统一读取后端安全错误 envelope。
  - Dashboard 成为真实的决策工作台：调用方输入 CAPE、ERP、MA200 距离、RSI、VIX 历史及当前值，先获得 70%/20% 信号，再提交 Decision Preview；页面展示服务端摘要、双桶金额、Qwen 或降级状态及可选 paper-order 回执。Paper order 默认关闭，且只使用 market buy / paper 路径。
  - Plans 页面可创建、列出并选择真实计划；Decisions 页面可选择已有计划并读取 SQLite 持久化的历史与审计快照，预览成功后会失效刷新当前计划的历史缓存。
  - 开发服务器为上述 API 前缀代理至 `127.0.0.1:8080`；README 补充本机 Web 启动方式，以及跨域部署时的 `VITE_API_BASE_URL` / `CORS_ALLOWED_ORIGINS` 配置说明。中英文界面新增实时流程的文案。
  - 本次仅修改你的 Fork 本地 `main`；不会向 Jame `upstream` 推送、修改其 `main`、分支或 PR。
- 验证：
  - `pnpm --dir apps/web install --frozen-lockfile` 通过。
  - `pnpm --dir apps/web lint` 通过。
  - `pnpm --dir apps/web build` 通过；Vite 仅提示首个 JS bundle 超过 500 kB，未阻断构建。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `git diff --check` 通过；已确认 `apps/web/src` 不再引用 mock API 模块。

### 2026-07-19 00:42 AEST

- 执行模型：GPT-5。
- 变更类型：Fork 前端整合 / 冲突解决。
- 涉及文件：
  - `AGENTS.md`
  - `apps/web/**`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Jame 的 `james/feat-frontend` 分支整合到你的 Fork `main`：引入 Vite/React Dashboard、shadcn UI、i18n、路由、React Query mock 数据层和 Plans/Decisions 占位页。
  - 唯一冲突位于 `CHANGE_LOG.md`；保留你的后端变更记录及 Jame 的前端变更记录，并按时间倒序排列。
  - `AGENTS.md` 仅补充前端规划文档入口与既有前端规范，不改变后端约束。
  - 本次只读取 `upstream/james/feat-frontend` 并在本地/Fork 完成整合；不会向 Jame `upstream` 推送或修改其 PR、分支、main 或任何远程项目状态。
- 验证：
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo fmt --all -- --check` 通过。
  - 前端 `pnpm lint` / `pnpm build` 未运行：当前机器未安装 Node.js 与 pnpm，未自动修改机器级开发环境。

### 2026-07-19 00:28 AEST

- 执行模型：GPT-5。
- 变更类型：MVP 后端收口 / Quant Signal HTTP API / Decision Summary。
- 涉及文件：
  - `crates/api/src/routes/signals.rs`
  - `crates/api/src/routes/mod.rs`
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/tests/signals.rs`
  - `crates/api/tests/decision_preview.rs`
  - `API_MANAGEMENT.md`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `POST /signals/fundamental/preview` 与 `POST /signals/trend/preview`：复用既有 `quant-engine` 的 60 个有效月度样本、不变量与分位计算，返回基本面/趋势分数、原始审计分位及趋势 regime；JSON 提取、未知字段和领域校验统一映射为安全的 `400 bad_request`。
  - Decision Preview 的 `summary` 从短句扩展为稳定分层解释，明确 execution 状态、计划金额、已方向规范化的基本面投资适配度、趋势时机和 regime、Qwen 情绪或降级权重、最终分数、倍率/action、双桶拆分与 paper-order 状态。
  - 更新 API 管理与全项目 MVP 文档：除前端、DashScope Key/真实网络 smoke、OpenD GUI 登录/虚拟账户 smoke，以及由前端或经确认数据源提供的月度行情与估值快照外，MVP 后端代码和默认本地 SQLite 配置均已完成；服务端不会擅自接入未选定的第三方市场数据源或保存其凭据。
  - 本次仅在你的 Fork `main` 上修改和后续推送；不修改、不合并或推送 Jame `upstream`。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-api --test signals --locked` 通过（3 tests）。
  - `cargo test -p indexlink-api --test decision_preview --locked` 通过（7 tests）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-api --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo test -p indexlink-server --locked` 通过（26 passed、1 ignored）。
  - `cargo doc -p indexlink-api --no-deps --locked` 通过。
  - `cargo llvm-cov --workspace --all-features --summary-only` 通过（总行覆盖率 93.75%）。

### 2026-07-19 00:21 AEST

- 执行模型：GPT-5。
- 变更类型：CI 回归修正 / Qwen 自动 Decision Preview。
- 涉及文件：
  - `apps/server/src/main.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 更新 server composition-root 的 Decision Preview 测试请求，移除已经废弃且严格拒绝的手工 `sentiment` DTO 字段；该测试继续验证配置的 broker factory 取代默认 mock，且在 broker 不可用时返回统一 `503`。
  - 此修正仅作用于你的 Fork `main`；不修改、不合并或推送 Jame `upstream`。
- 验证：
  - `cargo test -p indexlink-server --locked` 通过（26 passed、1 ignored）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo llvm-cov --workspace --all-features --summary-only` 通过（总行覆盖率 93.70%）。
  - `cargo fmt --all -- --check`、`cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-server -p indexlink-api --all-targets --all-features --locked -- -D warnings` 通过。

### 2026-07-19 00:11 AEST

- 执行模型：GPT-5。
- 变更类型：Decision Preview 自动 Qwen 情绪编排。
- 涉及文件：
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/tests/decision_preview.rs`
  - `API_MANAGEMENT.md`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - `POST /investment-plans/:id/decision-preview` 不再接受调用方提供的 `sentiment`；完成计划与 fundamental/trend 输入校验后，自动通过已注入的 RSS + Qwen market-sentiment pipeline 获取情绪，再执行 70/20/10 决策。
  - Qwen 未配置、新闻源失败或 provider 暂时不可用时，Decision Preview 不伪造情绪值，使用 decision engine 的既有 `90/10/0` fallback，并在响应与决策快照中标记 `sentiment_unavailable`；前端手工 `sentiment` 字段因 DTO 严格拒绝未知字段而安全返回 `400 bad_request`。
  - 自动情绪成功时，审计记录只保存 `source: market_sentiment` 与有界原始 score；不会保存新闻正文、API key、provider URL、OpenD 凭据或账户信息。
  - 修正 API 管理与 MVP 文档中已经过时的“自动 Qwen / 自动 decision record 尚未完成”表述，并更新前端请求契约与剩余 MVP 清单。
- 验证：
  - `cargo test -p indexlink-api --test decision_preview --locked` 通过（7 tests，含自动 provider、未配置或不可用 fallback、手工 sentiment 拒绝、审计快照和下单前存证）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo test -p indexlink-api --locked` 通过（37 tests）。
  - `cargo test -p decision-records --locked` 通过（11 tests）。
  - `cargo test -p indexlink-storage --locked` 通过（31 tests）。
  - `cargo fmt --all -- --check`、`cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-api -p decision-records -p indexlink-storage --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo doc -p indexlink-api -p decision-records -p indexlink-storage --no-deps --locked` 通过。

### 2026-07-18 23:53 AEST

- 执行模型：GPT-5。
- 变更类型：Decision Preview 本地审计持久化 / paper order 安全存证。
- 涉及文件：
  - `crates/api/Cargo.toml`
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/src/state.rs`
  - `crates/api/tests/decision_preview.rs`
  - `crates/api/tests/decision_records.rs`
  - `crates/decision-records/src/lib.rs`
  - `crates/storage/src/decision_records.rs`
  - `crates/storage/src/sqlite_decision_records.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - `POST /investment-plans/:id/decision-preview` 现在为每次已验证的预览自动持久化本地 SQLite decision record，保存执行结果、原始基本面/趋势/情绪输入、有效权重与派生决策、可选订单意图和安全摘要；快照不包含 OpenD 账户、凭据或 provider 内部错误。
  - 可提交的 paper order 采用两步安全存证：先成功写入订单意图，再调用 broker；收到回执后仅补写回执与最终摘要。首写失败时绝不调用 broker；回执补写失败只记录安全服务端日志，不把已确认订单伪装成可重试的失败响应。
  - decision-record port 新增语义受限的 `complete_broker_order` 操作，SQLite/PostgreSQL adapter 均只更新 broker acknowledgement 与 summary；SQLite 测试覆盖完成、缺失记录与静态查询约束。
  - Decision Preview 聚焦测试改为真实使用内存 decision-record fake，并验证 due 订单的输入、执行、决策和回执快照均被完整保存。
- 验证：
  - `cargo test -p indexlink-api --test decision_preview --locked` 通过（5 tests，含存储不可用时绝不调用 broker）。
  - `cargo test -p decision-records --locked` 通过（11 tests）。
  - `cargo test -p indexlink-storage sqlite_decision_records --locked` 通过（7 tests）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo test -p indexlink-api --locked` 通过（35 tests）。
  - `cargo test -p indexlink-storage --locked` 通过（31 tests）。
  - `cargo fmt --all -- --check`、`cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-api -p decision-records -p indexlink-storage --all-targets --all-features --locked -- -D warnings` 通过。

### 2026-07-18 22:30 AEST

- 执行模型：GPT-5。
- 变更类型：Futu/Moomoo OpenD / Part 3：server paper broker 装配与受控 virtual-account smoke。
- 涉及文件：
  - `.env.example`
  - `API_MANAGEMENT.md`
  - `Cargo.lock`
  - `apps/server/Cargo.toml`
  - `apps/server/src/config.rs`
  - `apps/server/src/main.rs`
  - `crates/broker/src/opend_session.rs`
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/src/state.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - server 仅在显式设置 `OPEND_PROVIDER=futu|moomoo` 时读取 loopback OpenD 的 host、port 与可选 account id，并构造固定 `Paper`、live gate 关闭的 `OpenDConnectionConfig`；未配置时保留 `MockBroker`。没有 live environment 配置项。
  - 配置 OpenD 后，server 在监听 HTTP 前建立 `OpenDPaperSession` 并注入 `OpenDPaperBroker`；连接、登录状态或模拟账户选择失败会阻止启动，不会悄然退回 mock broker。
  - `ApiState` 增加受文档约束的 broker 注入入口；`Decision Preview` 的可选订单始终经该 broker port，继续沿用 due/action 门控和安全 API 错误契约。
  - 新增默认 ignored 的真实 paper-order smoke：必须设置 `OPEND_SMOKE_CONFIRM=submit-paper-order`、显式 `OPEND_ACCOUNT_ID`、唯一 idempotency key、symbol 与 quantity，才会以临时内存 SQLite 计划穿过 production composition root 发送一笔虚拟订单。凭据、账户和订单 ID 不进入日志或断言。
  - 该提交只提供可执行的本机 smoke 入口；实际虚拟订单将在本机 OpenD 已登录并配置后单独执行与记录。
  - 审查修正：API 文档中的交互变量读取改用 Bash 兼容的 `read -r -p`；`localhost` 在 server 配置阶段固定规范化为字面 `127.0.0.1`，其他地址必须解析为 `IpAddr` 且满足 `is_loopback()`，同时 raw TCP adapter 复用相同 IP 语义。
  - 审查修正：server composition root 现在接收可替换的异步 broker factory；非 ignored 测试覆盖 factory session 失败阻止启动，以及 factory 成功后 HTTP Decision Preview 实际调用替换后的 broker，而非默认 mock。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-server --locked` 通过（26 passed、1 ignored；含 loopback 边界和 OpenD factory 成功/失败组合根测试）。
  - `cargo test -p indexlink-api --locked` 通过（33 tests，含 broker 注入替换默认 mock 的聚焦测试）。
  - `cargo test -p broker --locked` 通过（36 tests，含 raw TCP loopback IP 语义边界）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-server -p indexlink-api -p broker --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo doc -p indexlink-api --no-deps --locked` 通过。
  - 本机未检测到可执行的 OpenD smoke 配置前，不提交任何虚拟订单；真实 smoke 结果待 OpenD GUI 登录、paper account 与显式确认变量就绪后补记。

### 2026-07-18 21:42 AEST

- 执行模型：GPT-5。
- 变更类型：Futu/Moomoo OpenD / Part 2：paper order gateway。
- 涉及文件：
  - `crates/api/src/error.rs`
  - `crates/broker/src/lib.rs`
  - `crates/broker/src/opend_session.rs`
  - `API_MANAGEMENT.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - `OpenDPaperSession` 现在实现既有 `OpenDOrderGateway`，通过同一条串行 TCP 通道提交 PlaceOrder（2202）；每笔请求携带 OpenD connection id、递增的 anti-replay packet serial、已选模拟账户和 `Paper` 环境。
  - MVP 明确仅支持美股股票/ETF 订单：普通限价单映射 OpenD `Normal`，市价单映射 `Market`；回执必须确认模拟环境、同一账户和美股市场，才会生成 `BrokerOrderAck::Accepted`。
  - idempotency key 不直接写入 provider 备注，而是确定性 SHA-1 摘要，控制在 OpenD 64-byte remark 限制内；该备注只用于关联，当前 adapter 不对网络失败自动重试，也不声称跨请求幂等。不把 provider 的拒绝文案、网络细节或账户信息暴露给调用方。
  - 新增安全 `BrokerError::Rejected`，API 映射为既有统一 `bad_request` envelope；请求自身的环境不匹配仍为 `EnvironmentMismatch` / `bad_request`，只有回执中的账户/环境不匹配、协议畸形等才映射为 `Unavailable`。
  - 当 PlaceOrder 已开始写入后发生写入、flush、读取超时、断连或响应格式异常时，返回不可自动重试的 `OutcomeUnknown`；API 以 `409 order_outcome_unknown` 明确要求客户端不要重试，避免未知结果被 `503` 诱导重复下单。
  - 本 PR 只使用本地协议 fake，不连接真实 OpenD、不提交任何虚拟订单；server 注入和本机虚拟账户 smoke 仍留给 `opend-03-server-wiring-smoke`。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p broker --locked` 通过（35 tests，含 PlaceOrder protocol fake、未发送前置拒绝与结果未知边界）。
  - `cargo test -p indexlink-api --locked` 通过（33 tests）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p broker -p indexlink-api --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo doc -p broker --no-deps --locked` 通过。

### 2026-07-17 23:03 AEST

- 执行模型：GPT-5。
- 变更类型：Futu/Moomoo OpenD / Part 1：paper-only raw TCP session transport。
- 涉及文件：
  - `Cargo.lock`
  - `Cargo.toml`
  - `crates/broker/Cargo.toml`
  - `crates/broker/src/lib.rs`
  - `crates/broker/src/opend_session.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `OpenDPaperSession`，使用 Futu/Moomoo 官方 raw TCP 帧（`FT` 标志、44 字节 little-endian header、JSON body 与 SHA-1 完整性校验）建立会话；本阶段完成 `InitConnect`（1001）、交易登录状态检查（1002）、交易账户列表（2001）和按服务端间隔发送的 KeepAlive（1004），不提交订单。
  - 会话在 `InitConnect` 后确认 `trdLogined=true`；OpenD 的用户登录仍由本机 OpenD 进程负责，IndexLink 不读取、传输或记录 Futu/Moomoo 密码、token 或登录凭据。
  - 仅接受 `Paper` 环境且 live gate 关闭；当未显式指定 account id 时，必须恰好有一个模拟账户，多个候选或无候选都会安全拒绝。显式 account id 也只能匹配模拟账户。
  - raw TCP 暂时只允许 loopback OpenD（`127.0.0.1`、`::1`、`localhost`）；`localhost` 会固定映射为字面回环地址，不依赖系统 hosts 解析。官方可选 RSA packet encryption 尚未实现，拒绝远端明文 TCP 以避免交易元数据跨网络泄露。
  - 通过独立 golden frame 与损坏帧拒绝测试覆盖固定帧编码/完整性校验；本地 TCP protocol fake 覆盖初始化、KeepAlive、登录状态、默认/显式 paper account 选择及远端主机拒绝。没有真实下单、server 注入或 virtual-account smoke，本部分保留给后续两份 OpenD PR。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p broker --locked` 通过（27 tests，含独立 golden/损坏帧、KeepAlive 与 loopback TCP protocol fake）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p broker --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo doc -p broker --no-deps --locked` 通过。

### 2026-07-16 23:58 AEST

- 执行模型：GPT-5。
- 变更类型：阿里云 Qwen 市场情绪 API 接入 / OpenD 后续实施计划。
- 涉及文件：
  - `.env.example`
  - `API_MANAGEMENT.md`
  - `Cargo.lock`
  - `apps/server/Cargo.toml`
  - `apps/server/src/config.rs`
  - `apps/server/src/main.rs`
  - `crates/ai-client/src/news.rs`
  - `crates/api/Cargo.toml`
  - `crates/api/src/error.rs`
  - `crates/api/src/routes/market_sentiment.rs`
  - `crates/api/src/routes/mod.rs`
  - `crates/api/src/state.rs`
  - `crates/api/tests/market_sentiment.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - server 读取可选的 `DASHSCOPE_API_KEY`、base URL、model、timeout、max tokens 与 temperature；Key 未配置时不阻止本地 SQLite 服务启动，市场情绪路由统一返回安全的 `503`。配置和日志不会输出 Key。
  - `ApiState` 新增受控的新闻源与 AI provider 注入点；production composition root 使用 `RssNewsSource + QwenClient`，测试使用 fake adapter，不发起网络请求。
  - 新增 `POST /market-sentiment/preview`，返回 `score` 与稳定 `positive` / `neutral` / `negative` 标签；新闻源和 Qwen 失败统一映射为既有 JSON `service_unavailable` 错误，不向客户端泄露 provider 内部错误。
  - 审查修正：明确该路由尚未自动串联至 Decision Preview；真实 Key smoke 改为 shell 隐藏输入后 export，避免 Key 写入命令历史；server 将 Qwen 装配提取为 helper 并覆盖已配置/未配置两个分支；API 层在安全映射前记录已脱敏的管线错误，便于排障。
  - `ai-client` 的市场情绪管线允许通过 trait object 注入，保持 library 与 HTTP adapter 的六边形边界。
  - 真实 Key smoke 使用已有忽略式 Qwen 新闻集成测试；API 文档补充启动后 HTTP smoke 命令。真实凭据不进入仓库、日志或测试断言。
  - OpenD 按三份 PR 实施：
    1. `opend-01-session-transport`：在 broker adapter 内实现 TCP/SDK transport 边界、连接生命周期、认证与 paper account 选择；以协议 fake 覆盖，不改 server 组合根。
    2. `opend-02-order-gateway`：实现 market/limit 下单请求与 ack 转换、超时、网络错误和安全脱敏；继续强制 paper-only，不引入 live 下单路径。
    3. `opend-03-server-wiring-smoke`：从 server 环境变量构造并注入真实 `OpenDPaperBroker`，替换 production 固定 `MockBroker`，保留未配置时的 Mock 回退；使用虚拟账户执行一次真实 smoke 并记录安全操作步骤。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-api --offline` 通过（31 tests，含 fake provider 注入、未配置 provider 与 provider 错误映射）。
  - `cargo test -p indexlink-server --locked` 通过（21 tests，含可选 Qwen 配置、参数解析与 Key 空值拒绝，以及 Qwen 装配的已配置/未配置分支）。
  - `cargo test -p core-domain --offline` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p ai-client -p indexlink-api -p indexlink-server --all-targets --all-features --locked -- -D warnings` 通过。
  - 已尝试 `cargo test -p ai-client --test news real_cnbc_with_qwen -- --ignored --nocapture`；当前环境未设置 `DASHSCOPE_API_KEY`，测试在网络请求前退出。待本机配置 Key 后按 API 文档命令重跑；不得在 CI 或日志中输出凭据。

### 2026-07-15 23:01 AEST

- 执行模型：GPT-5。
- 变更类型：SQLite runtime 审查修正。
- 涉及文件：
  - `Cargo.lock`
  - `apps/server/src/config.rs`
  - `crates/storage/Cargo.toml`
  - `crates/storage/src/sqlite_decision_records.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 配置层现在只接受非空 `sqlite:` `DATABASE_URL`，会在启动连接前明确拒绝遗留 `postgres://` URL。
  - 提炼 SQLite row 列读取助手，统一 `try_get` 的安全错误映射，保留 UUID、金额、时间和 JSON 的原有解析语义。
  - storage crate 复用 workspace `tracing` 依赖；decision record SQLite adapter 在折叠非 `RowNotFound` SQLx 错误前记录内部 warning，HTTP/领域层仍只得到安全的 `Unavailable`。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-storage --offline` 通过（30 tests）。
  - `cargo test -p indexlink-server --offline` 通过（15 tests）。
  - `cargo test -p indexlink-api --offline` 通过（28 tests）。
  - `cargo test -p core-domain --offline` 通过（13 tests）。
  - `cargo check --workspace --offline` 通过。
  - `cargo clippy -p indexlink-storage -p indexlink-api -p indexlink-server --all-targets --all-features --offline -- -D warnings` 通过。

### 2026-07-15 22:44 AEST

- 执行模型：GPT-5。
- 变更类型：SQLite decision record 审查修正 / 演示 MVP 缺口审计。
- 涉及文件：
  - `crates/storage/src/sqlite.rs`
  - `crates/storage/src/sqlite_decision_records.rs`
  - `migrations/sqlite/20260715012000_reject_null_decision_record_snapshots.sql`
  - `CHANGE_LOG.md`
- 变更内容：
  - 修正 JSON `null` 绕过问题：SQLite adapter 的读取端现在会拒绝 `null` 快照并安全映射为 repository unavailable；可选快照若以 JSON `null` 而非 SQL `NULL` 存储，也同样不会进入领域模型。
  - 新增 SQLite migration，以 insert/update trigger 阻止任一必填 decision record snapshot（execution、fundamental、trend、decision）被直接写入 JSON `null`；既有损坏行不会被静默修复，而会在读取时被拒绝，避免伪造有效审计记录。
  - 全项目演示 MVP 审计结论：本地 SQLite、计划管理、双桶执行预览、70/20/10 纯函数决策、MockBroker 串联和只读 decision history 已可用；但 `apps/web` 当前仍是 Vite 模板，尚未实现演示界面。
  - 演示 MVP 的阻塞项依优先级为：
    1. 将 DashScope/Qwen client 接入 server config、API state 与真实 market sentiment route，并以真实 key smoke test。
    2. 实现 Futu/Moomoo OpenD 的真实 TCP/SDK gateway transport，注入 server，并以 paper/virtual account 提交订单和获取 ack。
    3. 将 Decision Preview 升级为受控服务端编排：接入真实 Qwen、确定 fundamental/trend 的演示输入来源、生成分层 summary，并在成功结果后自动写入本地 decision record。
    4. 由前端负责方把当前 Vite 模板替换为计划、信号、决策、双桶、paper order 与 history 的演示闭环。
    5. 补全真实凭据的端到端 smoke 文档；Docker Compose 的 SQLite named volume 写权限仍需在有 Docker 的环境实测并修正（当前镜像以内置目录 chown，挂载 volume 后权限可能变化）。
  - 自动 Scheduler、成交回报状态机、多用户和 live trading 均不属于本次“演示可用”最小 MVP 的阻塞项。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-storage --locked` 通过（30 tests，覆盖 adapter 读取拒绝 JSON `null`、migration 阻止 insert/update 直接写入 JSON `null`）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-storage --all-targets --all-features --locked -- -D warnings` 通过。

### 2026-07-15 22:30 AEST

- 执行模型：GPT-5。
- 变更类型：SQLite 本地持久化 / Part 3：Decision Record adapter 与 production runtime wiring。
- 涉及文件：
  - `.env.example`
  - `README.md`
  - `API_MANAGEMENT.md`
  - `docs/minimum_mvp.md`
  - `deployment/Dockerfile`
  - `deployment/docker-compose.yml`
  - `apps/server/src/config.rs`
  - `apps/server/src/main.rs`
  - `crates/api/src/state.rs`
  - `crates/storage/src/lib.rs`
  - `crates/storage/src/sqlite.rs`
  - `crates/storage/src/sqlite_decision_records.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `SqliteDecisionRecordRepository`，以静态 SQLite 查询实现审计快照的 create、按计划有上限列表查询与单条查询；金额沿用固定精度文本编码，JSON、UUID、时间或状态快照损坏时安全映射为后端不可用。
  - `ApiState` 生产组合根改用 SQLite plan 与 decision record adapter，旧 PostgreSQL adapter 保留但不再进入默认运行路径。
  - server 使用 SQLite 默认 URL 连接本地文件，并在 HTTP 监听前执行编译期嵌入的 migration；migration 失败将阻止服务启动。
  - 配置、示例环境变量、Dockerfile 与 Compose 改为本地 SQLite。Compose 使用 `sqlite-data` volume 保留数据，不再依赖 PostgreSQL 容器。
  - 更新 MVP 与 API 文档，明确默认本地存储、旧 PostgreSQL adapter 的兼容定位，以及 Decision Preview 自动存证仍是后续工作。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-storage --locked` 通过（30 tests，含 SQLite decision record 的外键、金额精度、UTC `Z` 时间、JSON 快照与 history limit）。
  - `cargo test -p indexlink-api --locked` 通过（28 tests）。
  - `cargo test -p indexlink-server --locked` 通过（14 tests）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-storage -p indexlink-api -p indexlink-server --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo doc -p indexlink-storage --no-deps --locked` 通过。
  - 使用临时 SQLite 文件启动 `indexlink-server`，`GET /ready` 返回 `{"status":"ready","database":"ok"}`；确认 migration 在监听 HTTP 前完成。
  - 未安装 Docker CLI，未能在本机执行 `docker compose ... config`；Compose 文件仅做静态审查。

### 2026-07-15 AEST

- 执行模型：GPT-5。
- 变更类型：SQLite 本地持久化 / Part 2：Investment Plan repository adapter。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/storage/src/lib.rs`
  - `crates/storage/src/sqlite.rs`
  - `crates/storage/src/sqlite_investment_plans.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `SqliteInvestmentPlanRepository`，实现计划的 create、list、get、原子 update 与 set_active。
  - 金额在 SQLite 边界使用固定 12 位整数 + 8 位小数文本编码；不满足原 PostgreSQL `NUMERIC(20, 8)` 范围的值不会写入。
  - update 使用 `BEGIN IMMEDIATE` 获取 SQLite 写锁，在同一事务中读取、合并、校验并更新最终金额，避免并发读改写窗口。
  - 金额编码会归一化仅含尾随零的额外小数位；只有归一化后会改变数值的精度溢出才拒绝写入。
  - `updated_at` 始终由 SQLite 写为 UTC RFC 3339 `Z` 格式，并通过当前时间与前值加 1ms 的较大值保证严格递增；读取端解析并拒绝损坏的时间或金额快照。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-storage --locked` 通过（24 tests）。
  - `cargo test -p investment-plans --locked` 通过（31 tests）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-storage --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo doc -p indexlink-storage --no-deps` 通过。

### 2026-07-15 AEST

- 执行模型：GPT-5。
- 变更类型：SQLite 本地持久化 / Part 1：基础设施与 migration。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/storage/src/lib.rs`
  - `crates/storage/src/sqlite.rs`
  - `migrations/sqlite/20260715010000_create_investment_plans.sql`
  - `migrations/sqlite/20260715011000_create_decision_records.sql`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增独立 `SqliteStorage`，为本地 `.db` 文件提供连接、外键、WAL、busy timeout、健康检查和编译期嵌入的 migration 基础设施；尚未替换现有 PostgreSQL production wiring。
  - 新增 SQLite 专用 baseline schema：UUID、精确金额、时间和 JSON snapshot 使用 TEXT；金额使用固定 12 位整数 + 8 位小数格式，以文本比较保留正数与 `max_single_execution >= base_contribution` 约束；时间强制为 UTC RFC 3339 `Z` 格式。
  - PostgreSQL migration 与 SQLite migration 分目录维护，避免不同数据库执行不兼容 SQL。
- 三个 PR 计划：
  1. **Part 1（本 PR）**：SQLite 连接与 migration 基础设施、SQLite baseline schema、聚焦迁移测试。
  2. **Part 2**：实现 SQLite Investment Plan repository adapter，并验证创建、读取、原子更新、启停及本地持久化。
  3. **Part 3**：实现 SQLite Decision Record repository adapter，并将 server/API/config/Docker 默认 wiring 切换为本地 SQLite 文件；同时移除 MVP 对 PostgreSQL 容器的运行依赖。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-storage --locked` 通过（17 tests，含 SQLite 内存数据库 migration 与金额、时间约束执行）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-storage --all-targets --all-features --locked -- -D warnings` 通过。

### 2026-07-15 00:36 AEST

- 执行模型：GPT-5。
- 变更类型：Decision Record 持久化（Part 3：History Query API）。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/decision-records/src/lib.rs`
  - `crates/api/Cargo.toml`
  - `crates/api/src/error.rs`
  - `crates/api/src/state.rs`
  - `crates/api/src/routes/mod.rs`
  - `crates/api/src/routes/decision_records.rs`
  - `crates/api/tests/decision_records.rs`
  - `crates/api/tests/health.rs`
  - `API_MANAGEMENT.md`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `GET /investment-plans/:id/decisions?limit=` 与 `GET /decisions/:id`，分别查询已存在计划的最新 decision record 列表和单条审计记录。
  - history list 默认返回 50 条、最大 200 条；非法 UUID、query 参数或 limit 统一映射为安全的 `bad_request`，不存在计划或记录映射为 `not_found`。
  - 生产 `ApiState` 注入现有 `PostgresDecisionRecordRepository`；隔离测试状态使用显式 unavailable repository，避免意外访问真实数据库。
  - Decision record 的 `created_at` JSON 序列化改为 RFC 3339 字符串，避免将 `OffsetDateTime` 内部数组暴露给前端。
  - 将 `service_unavailable` 文案改为中性 `service is unavailable`，准确覆盖数据库、broker 与 decision record 后端不可用。
  - 更新 API 管理与 MVP 文档：明确只读 history API 已可用，但 Decision Preview 自动写入审计记录仍留待后续独立 PR。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-api --locked` 通过（28 tests）。
  - `cargo test -p decision-records --locked` 通过（10 tests）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo clippy -p indexlink-api --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过；2 个依赖真实网络/API key 的测试按项目约定 ignored。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-13 23:56 UTC+10

- 执行模型：GPT-5。
- 变更类型：PR review fix。
- 涉及文件：
  - `crates/decision-records/src/lib.rs`
  - `crates/storage/src/decision_records.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - Review fix：新增 `DecisionRecordListQuery`，为 decision record 历史查询提供默认分页上限，避免 `list_by_plan` 长期无限制返回全部记录。
  - Review fix：`PostgresDecisionRecordRepository::list_by_plan` 绑定 `LIMIT` 参数，并保留按 `created_at DESC, id DESC` 的稳定排序。
  - Review fix：将 decision record storage SQL 改为编译期静态常量，避免 `format!()` 拼接查询语句造成 SAST 噪音和运行时分配。
  - 补充聚焦测试，覆盖 list query 边界、bounded list 服务路径和静态 SQL 中的 `LIMIT` 约束。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check -p decision-records --locked` 通过。
  - `cargo check -p indexlink-storage --locked` 通过。
  - `cargo test -p decision-records --locked` 通过。
  - `cargo test -p indexlink-storage --locked` 通过。
  - `cargo clippy -p indexlink-storage --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-13 23:22 UTC+10

- 执行模型：GPT-5。
- 变更类型：Decision Record 持久化（Part 2：PostgreSQL storage adapter）。
- 涉及文件：
  - `Cargo.lock`
  - `crates/storage/Cargo.toml`
  - `crates/storage/src/lib.rs`
  - `crates/storage/src/decision_records.rs`
  - `migrations/20260713093000_create_decision_records.sql`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `decision_records` PostgreSQL migration，用于保存 decision preview / execution 的审计快照。
  - migration 使用 `JSONB` 保存 execution、fundamental、trend、sentiment、decision 与 broker 输入输出快照，并通过 `plan_id` 外键关联 `investment_plans`。
  - 新增 `PostgresDecisionRecordRepository`，实现 `DecisionRecordRepository` 的 create、list_by_plan 与 get。
  - storage adapter 在写入前再次调用 `CreateDecisionRecord::normalize()`，避免绕过服务层时写入未规范化数据。
  - 为 plan + created_at 与全局 created_at 添加查询索引，支持后续 history API。
  - storage adapter 使用 SQL `::jsonb` 写入，并在 Rust 侧解析 JSON snapshot，避免扩大 SQLx workspace feature 面。
- 接下来计划：
  1. Part 3：新增 decision record 查询 API，并更新 `API_MANAGEMENT.md` / `docs/minimum_mvp.md`。
  2. 后续阶段：在 Decision Preview API 中接入持久化写入。
  3. 后续阶段：接入阿里云 Qwen Market Sentiment API 与 Futu/Moomoo OpenD paper gateway transport。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check -p indexlink-storage --locked` 通过。
  - `cargo test -p indexlink-storage --locked` 通过。
  - `cargo clippy -p indexlink-storage --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-12 00:15 UTC+10

- 执行模型：GPT-5。
- 变更类型：Decision Record 持久化（Part 1：领域层）。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/decision-records/Cargo.toml`
  - `crates/decision-records/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `decision-records` crate，定义 `DecisionRecord`、`CreateDecisionRecord`、repository port 与应用服务。
  - 使用 JSON snapshot 字段保存 execution、fundamental、trend、sentiment、decision 与 broker 输入输出，优先保留审计输入快照而不是只保存结论。
  - 自查修正：新增 `DecisionExecutionStatus`，避免执行状态以任意字符串绕过领域边界。
  - 自查修正：新增 `CreateDecisionRecord::normalize()` 与 `DecisionRecordValidationError`，在 repository 前校验 symbol、currency、planned contribution、summary 和必需 JSON snapshot。
  - 自查修正：snapshot 字段 rustdoc 明确不得保存 API key、account id、OpenD 密码或其他 secrets。
  - Review fix：补充 normalize 边界测试，覆盖 symbol、currency、summary、必需 snapshot 与可选 snapshot 的非法分支。
  - 新增领域层单元测试，覆盖 create/list/get 服务路径、repository not found 映射、创建输入规范化和非法输入拒绝。
- 接下来计划：
  1. Part 2：新增 PostgreSQL `decision_records` migration 与 `PostgresDecisionRecordRepository`。
  2. Part 3：新增 decision record 查询 API，并更新 `API_MANAGEMENT.md` / `docs/minimum_mvp.md`。
  3. 后续阶段：接入阿里云 Qwen Market Sentiment API，并在真实执行链路中写入 decision record。
  4. 后续阶段：实现 Futu/Moomoo OpenD paper gateway transport，继续默认 paper trading。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check -p decision-records --locked` 通过。
  - `cargo test -p decision-records --locked` 通过。
  - `cargo clippy -p decision-records --all-targets --all-features -- -D warnings` 通过。

### 2026-07-10 11:53 UTC+10

- 执行模型：GPT-5。
- 变更类型：API 管理文档。
- 涉及文件：
  - `API_MANAGEMENT.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增根目录 `API_MANAGEMENT.md`，面向前端对接整理当前已有 API、请求/响应约定、统一错误格式和对接顺序。
  - 补充待实现 API 清单，包括阿里云 Qwen market sentiment、fundamental/trend signal、Futu/Moomoo OpenD paper trading、Decision Preview 真实上游升级和 decision record/history。
- 验证：
  - 文档变更，无需运行 Rust 测试。
  - `git status --short` 已检查。

### 2026-07-09 23:43 UTC+10

- 执行模型：GPT-5。
- 变更类型：PR review fix。
- 涉及文件：
  - `crates/api/Cargo.toml`
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/tests/decision_preview.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - Review fix：将 `paper_order` DTO 到 broker 领域请求的结构校验前置，避免 waiting 或 `TacticalDelay` 路径静默接受非法 market/limit payload。
  - Review fix：在可替换 broker port 调用外层增加 5 秒超时，超时后返回安全的 `service_unavailable` API envelope。
  - 新增回归测试，覆盖 waiting 与 tactical delay 路径中非法 paper order 仍返回 `bad_request`。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-api --locked` 通过。
  - `cargo clippy -p indexlink-api --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-09 23:10 UTC+10

- 执行模型：GPT-5。
- 变更类型：Decision Preview API + MockBroker 串联。
- 涉及文件：
  - `Cargo.lock`
  - `crates/api/Cargo.toml`
  - `crates/api/src/error.rs`
  - `crates/api/src/routes/mod.rs`
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/src/state.rs`
  - `crates/api/tests/decision_preview.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `POST /investment-plans/:id/decision-preview`，将 investment plan、执行预览、双桶拆分、70/20/10 decision engine 与 broker paper order 串成后端演示闭环。
  - API 入站仍使用 DTO 转换到领域类型，复用 `Percentile`、`PreviewInvestmentPlanExecution`、`BucketAllocationRatio` 与 broker order 构造器的不变量校验。
  - `ApiState` 新增可替换 broker port，生产默认使用 `MockBroker::paper_only()`，测试可注入共享 mock broker 观察订单提交。
  - decision preview 仅在计划 due 且 action 不是 `Skip` / `TacticalDelay` 时提交 paper order；waiting、inactive、跳过和战术延迟都不会触发 broker。
  - 返回执行预览、decision score/multiplier/action、可选 paper order ack 和 demo summary。
  - 新增 broker 错误到 API 安全错误响应的映射，避免向客户端暴露 adapter 内部细节。
  - 新增 HTTP 路由测试，覆盖 due 下单、waiting 不下单、tactical delay 不下单，以及非法 UUID / 非法分位 / 非法 order payload 的统一 `bad_request` envelope。
  - 更新 `docs/minimum_mvp.md`，标记 Decision Preview API + MockBroker 串联已完成，并将后续重点调整为阿里云 Qwen API 与真实 Futu/Moomoo OpenD transport。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check -p indexlink-api --locked` 通过。
  - `cargo test -p indexlink-api --locked` 通过。
  - `cargo clippy -p indexlink-api --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。
### 2026-07-08 18:25 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端全局样式调整。
- 涉及文件：
  - `apps/web/src/index.css`
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在全局 base layer 中新增 `::-webkit-scrollbar-button` 覆盖，隐藏 Chromium/WebKit 滚动条两端的方向箭头按钮，同时保留滚动条本体。
  - 清理 `valuation-card.tsx` 中前序改动遗留的未使用 import，恢复 ESLint 通过。
- 验证：
  - `pnpm lint` 通过。
  - `ReadLints` 检查显示 `index.css` 中 Tailwind v4 专用 `@theme`、`@custom-variant`、`@apply` 为 CSS 语言服务 warning，属于既有框架语法识别问题；`valuation-card.tsx` 无诊断。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。
  - 本次未运行 `pnpm build`。

### 2026-07-08 18:12 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 布局调整。
- 涉及文件：
  - `apps/web/src/pages/dashboard/index.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Dashboard 下半部分从两段独立三列网格改为左右两列流式布局。
  - 左侧两列依次放置 8 个指标卡和 Performance Comparison，右侧一列依次放置 Latest Decision 与 Risk，避免 Latest Decision 较高时让左侧指标卡与下方图表之间产生空白。
- 验证：
  - `pnpm lint` 通过。
  - `ReadLints` 检查 `dashboard/index.tsx` 无诊断。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。
  - 本次未运行 `pnpm build`。

### 2026-07-08 18:06 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 布局调整。
- 涉及文件：
  - `apps/web/src/pages/dashboard/index.tsx`
  - `apps/web/src/pages/dashboard/score-cards.tsx`
  - `apps/web/src/pages/dashboard/returns-cards.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `LatestDecisionCard` 从顶部估值卡右侧移动到下方 8 个指标卡的右侧。
  - Dashboard 顶部改为单独显示 `ValuationCard`；下方新增三列布局，左侧两列放得分卡与收益卡，右侧一列放最近一次决策卡。
  - 得分卡、收益卡的 4 列布局断点从 `xl` 提前到 `lg`，确保桌面下左侧 8 个卡片保持 4 列 x 2 行。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 17:59 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 交互位置调整。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Current Market Valuation 的 `why?` 入口从内容区底部移动到卡片 header 右上角。
  - 使用 shadcn `CardAction` 对齐右上角动作区；展开中或展开后隐藏 `why?`，收回完成后重新显示。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 17:57 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 交互动画。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 Current Market Valuation 的 `why?` percentile 展开区增加拉伸/收回动画。
  - 新增独立渲染状态：展开时先挂载内容再从 `grid-rows-[0fr]` 过渡到 `grid-rows-[1fr]`；收回时播放反向动画，动画结束后卸载内容，避免折叠状态残留高度或隐藏内容可聚焦。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 17:54 UTC+10

- 执行模型：Fable 5。
- 变更类型：前端 Dashboard 视觉/交互调整。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - `why?` 触发入口从 ghost 按钮块改为灰色下划线文字链接（`text-muted-foreground` + `underline` + `cursor-pointer`，hover 变前景色）。
  - 折叠状态移除原先的 `bg-muted/20` 占位区块；并为卡片添加 `self-start`，避免被同行更高的 Latest Decision 卡片拉伸产生大片留白，展开后再自然增高。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 17:52 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 交互调整。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `apps/web/src/i18n/locales/zh.ts`
  - `apps/web/src/i18n/locales/en.ts`
  - `CHANGE_LOG.md`
- 变更内容：
  - Current Market Valuation 默认隐藏 percentile 柱状图，仅显示 `why?` 按钮。
  - 点击 `why?` 后在原卡片内垂直展开柱状图和指标说明问号；底部新增“收回 / Collapse”按钮，点击后恢复折叠状态。
  - 补齐展开/收回按钮的中英文 i18n 文案。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 17:11 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端样式规范（业务语义色 token 集中化）。
- 涉及文件：
  - `apps/web/src/index.css`
  - `apps/web/src/lib/decision.ts`
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `apps/web/src/pages/dashboard/comparison-chart.tsx`
  - `apps/web/src/pages/dashboard/returns-cards.tsx`
  - `apps/web/src/pages/dashboard/score-cards.tsx`
  - `apps/web/src/pages/dashboard/risk-card.tsx`
  - `apps/web/src/components/layout/news-ticker.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在 `index.css` 中新增动作 badge、收益正负、风险提示、实时状态、分位区间、对比图系列等业务语义色 token，并提供 dark theme 对应值。
  - `decision.ts` 的 `actionBadgeClass` 改为使用 `action-*` token，不再直接散落 Tailwind 具体色阶。
  - Dashboard 的收益、分数、风险提示、新闻状态、估值分位柱状图、普通定投 vs 自适应定投图表统一改为调用集中 token。
  - 保留 shadcn chart 的局部 `--color-*` 系列变量机制，但其来源改为业务 token（如 `--chart-dca` / `--chart-adaptive`）。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `ReadLints` 检查相关前端文件无诊断。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-07-08 17:05 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 视觉修复。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 强制覆盖 Current Market Valuation 问号说明 tooltip 的箭头背景和填充色，避免浅色 tooltip 下方继续显示默认黑色菱形。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 17:04 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 视觉修复。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Current Market Valuation 底部问号说明 tooltip 从默认黑底样式局部覆盖为与图表 tooltip 一致的浅色卡片风格。
  - 覆盖 tooltip 箭头颜色，使其与浅色背景保持一致。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 17:02 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 交互调整。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Current Market Valuation 柱状图自身 tooltip 简化为只显示指标名和百分比。
  - 隐藏 Recharts XAxis 原始标签，在图表下方新增指标名 + 问号图标行。
  - 每个问号图标使用 shadcn Tooltip 展示对应 percentile 指标解释，避免长说明受图表 hover 区域影响而消失。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 16:57 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 视觉调整。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Current Market Valuation 柱状图 tooltip 内容卡片最大宽度从 `max-w-80` 调整为 `max-w-40`，使说明卡片约缩窄一半。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 16:55 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 信息补充。
- 涉及文件：
  - `apps/web/src/i18n/locales/zh.ts`
  - `apps/web/src/i18n/locales/en.ts`
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 Current Market Valuation 的 CAPE、ERP、MA200、RSI、VIX 五个 percentile 柱状图 tooltip 增加指标解释。
  - tooltip 现在展示指标名、百分比数值和本地化说明段落。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 16:54 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 视觉调整。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Current Market Valuation 的 percentile 柱状图从空心柱恢复为实心默认前景色柱。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 16:53 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 视觉调整。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Current Market Valuation 的 percentile 柱状图从实心柱改为空心柱。
  - 柱体填充改为透明，保留默认前景色描边。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 16:52 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 视觉调整。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - Current Market Valuation 的 percentile 柱状图取消按分位区间着色。
  - 移除 Recharts `Cell` 分色逻辑，所有柱子统一使用默认前景色。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 16:48 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端修复（Dashboard 卡片布局）。
- 涉及文件：
  - `apps/web/src/pages/dashboard/latest-decision-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Latest Decision 指标区的响应式列数从 5 列改为 4 列，匹配实际 4 个指标卡，避免末尾空列导致水平分布不均。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `ReadLints` 检查 `latest-decision-card.tsx` 无诊断。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-07-08 16:43 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端修复（Dashboard 图表 tooltip）。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 Current Market Valuation 的 Recharts tooltip 增加自定义 formatter。
  - tooltip 现在使用当前柱子的指标标签，并通过 `gap` 分隔标签与数值。
  - percentile 数值统一显示 `%` 后缀，例如 `86%`。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `ReadLints` 检查 `valuation-card.tsx` 无诊断。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-07-08 16:41 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端规范对齐（Dashboard 图表）。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Current Market Valuation 中 CAPE、ERP、MA200、RSI、VIX 五个 percentile 指标由手写 `div` 柱状图改为 Recharts `BarChart`。
  - 使用 `ChartContainer`、`ChartTooltip`、`ChartTooltipContent` 接入 shadcn chart 封装，符合前端图表规范。
  - 保留 0-100 percentile 语义、指标标签、柱顶百分比与原有分位颜色区间（低位 emerald、中位 sky、高位 amber、极高 rose）。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `ReadLints` 检查 `valuation-card.tsx` 无诊断。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-07-08 16:37 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 视觉调整。
- 涉及文件：
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Current Market Valuation 中 CAPE、ERP、MA200、RSI、VIX 五个 percentile 指标从横向进度条改为垂直柱状图。
  - 保留原有分位区间颜色语义，并在柱状图上方直接展示百分比数值。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 16:36 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 语义修正。
- 涉及文件：
  - `apps/web/src/api/types.ts`
  - `apps/web/src/api/mock.ts`
  - `apps/web/src/i18n/locales/zh.ts`
  - `apps/web/src/i18n/locales/en.ts`
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Current Market Valuation 中的 `nextDcaPrice` 修正为 `baseDcaAmount`，避免把下期预计投入误表述为价格。
  - 卡片右侧改为展示“预计执行金额”，由 `baseDcaAmount * suggestedMultiplier` 计算得到，例如 `2000 * 0.75 = 1500`。
  - 更新中英文 i18n 文案，将 “Next DCA price / 下期定投价格” 改为 “Expected amount / 预计执行金额”。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 16:32 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 信息补充。
- 涉及文件：
  - `apps/web/src/api/types.ts`
  - `apps/web/src/api/mock.ts`
  - `apps/web/src/i18n/locales/zh.ts`
  - `apps/web/src/i18n/locales/en.ts`
  - `apps/web/src/pages/dashboard/valuation-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - `MarketOverview` mock 数据新增 `currency`、`nextDcaPrice` 与 `nextDcaTime`，用于展示下期定投价格和定投时间。
  - Current Market Valuation 卡片在执行倍率右侧新增下期定投价格、下期定投时间，并继续保留建议动作展示。
  - 补齐中英文 i18n 文案。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 16:30 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端 Dashboard 信息补充。
- 涉及文件：
  - `apps/web/src/api/types.ts`
  - `apps/web/src/api/mock.ts`
  - `apps/web/src/i18n/locales/zh.ts`
  - `apps/web/src/i18n/locales/en.ts`
  - `apps/web/src/lib/decision.ts`
  - `apps/web/src/pages/dashboard/latest-decision-card.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - `LatestDecision` mock 数据新增 `executionPrice` 与 `executionTime`，用于展示本期定投价格与定投时间。
  - 最近一次决策卡片在倍率右侧新增本期定投价格、定投时间，并保留执行金额展示。
  - 新增价格格式化函数，价格保留两位小数，不影响金额整数展示。
  - 补齐中英文 i18n 文案。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `cargo test -p core-domain --locked` 通过：13 个单元测试全部通过。

### 2026-07-08 15:55 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端修复（Header Logo）。
- 涉及文件：
  - `apps/web/src/components/layout/app-header.tsx`
  - `apps/web/src/components/layout/news-ticker.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Header 左上角 lucide `LineChart` 图标替换为 `public/logo.png` 静态图片。
  - 清理 `news-ticker.tsx` 中未使用的 `t` 变量，恢复 lint/build 通过。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `ReadLints` 检查 `app-header.tsx`、`news-ticker.tsx` 无诊断。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-07-08 15:30 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端布局调整。
- 涉及文件：
  - `apps/web/src/components/layout/app-layout.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将实时新闻滚动条移动到 header 上方，作为应用最顶部的信息条。
  - 保持 `--app-chrome-height` 为新闻条 + header 的总高度，使左侧 sidebar 仍从顶部栏下方开始。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。

### 2026-07-08 15:25 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：前端修复（Sidebar Provider 层级）。
- 涉及文件：
  - `apps/web/src/components/layout/app-layout.tsx`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `SidebarProvider` 上移为 `AppLayout` 的外层布局容器，覆盖 `AppHeader`、`NewsTicker`、`AppSidebar` 与页面内容。
  - 保留侧栏与主内容的内部横向 flex 区域，避免 `AppHeader` 中的 `SidebarTrigger` 在 Provider 外调用 `useSidebar`。
- 验证：
  - `pnpm lint` 通过。
  - `pnpm build` 通过；Vite 仅提示产物 chunk 超过 500 kB 的体积警告。
  - `ReadLints` 检查 `app-layout.tsx` 无诊断。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-07-08 15:15 UTC+10

- 执行模型：Fable 5。
- 变更类型：前端 Dashboard 布局（MVP，纯 mock 数据，不连后端）。
- 涉及文件：
  - `apps/web/vite.config.ts`、`apps/web/tsconfig.json`、`apps/web/tsconfig.app.json`、`apps/web/eslint.config.js`、`apps/web/index.html`、`apps/web/components.json`
  - `apps/web/src/index.css`、`apps/web/src/main.tsx`、`apps/web/src/App.tsx`（删除 `src/App.css`）
  - `apps/web/src/i18n/`（`index.ts`、`locales/zh.ts`、`locales/en.ts`）
  - `apps/web/src/api/`（`types.ts`、`mock.ts`、`queries.ts`）
  - `apps/web/src/stores/ui.ts`
  - `apps/web/src/components/layout/`（`app-layout.tsx`、`app-header.tsx`、`app-sidebar.tsx`、`news-ticker.tsx`）
  - `apps/web/src/components/ui/`、`src/hooks/use-mobile.ts`、`src/lib/utils.ts`（shadcn 生成）
  - `apps/web/src/lib/decision.ts`
  - `apps/web/src/pages/dashboard/`（`index.tsx`、`valuation-card.tsx`、`score-cards.tsx`、`latest-decision-card.tsx`、`risk-card.tsx`、`returns-cards.tsx`、`comparison-chart.tsx`）
  - `apps/web/src/pages/decisions/index.tsx`、`apps/web/src/pages/plans/index.tsx`（占位页）
  - `CHANGE_LOG.md`
- 变更内容：
  - 接入 `@tailwindcss/vite`（Tailwind v4）与 `@` 路径别名；以 `shadcn init`（radix + nova 预设）初始化主题，并添加 button/card/badge/separator/sidebar/chart/tooltip/dropdown-menu/avatar/skeleton/scroll-area/breadcrumb 组件。
  - 按 PLAN.md 完成应用外壳：header 左上 logo、右上 mock 账户下拉与中英切换；header 下方新闻滚动条（CSS marquee，悬停暂停）；左侧可收放 sidebar（icon 折叠 + rail），其余区域为路由页面。
  - 路由使用 react-router（`/`、`/decisions/:id?`、`/plans/:id?`，后两者为占位页）；mock 服务端数据统一经 @tanstack/react-query 查询钩子提供（后续可直接替换为真实 fetch）；图表显示范围等临时 UI 状态使用 valtio。
  - Dashboard 页面：当前市场估值卡（综合分位、建议动作、执行倍率、CAPE/ERP/MA200/RSI/VIX 分位条）、70/20/10 得分卡（基本面/趋势面/AI 情绪/综合）、最近一次决策卡（动作、基准金额、倍率、执行金额、摘要、详情入口）、风险提示卡、收益卡（总收益/持仓收益/确定收益/累计投入）、普通定投 vs 自适应定投累计收益对比图（recharts AreaChart + shadcn chart 容器，支持 1 年/3 年/全部范围切换）。
  - i18next + 浏览器语言检测实现中英双语；mock 数据内文案按语言本地化。
  - eslint 对 shadcn 生成的 `src/components/ui/**` 与 `use-mobile.ts` 豁免 fast-refresh / set-state-in-effect 规则，业务代码不豁免。
- 验证：
  - `pnpm build`（`tsc -b && vite build`）通过。
  - `pnpm lint` 通过。
  - 本次未改动 Rust 代码，仍运行 `cargo test -p core-domain --locked` 通过。

### 2026-07-07 23:47 UTC+10

- 执行模型：GPT-5。
- 变更类型：Decision Engine。
- 涉及文件：
  - `Cargo.toml`
  - `crates/decision-engine/Cargo.toml`
  - `crates/decision-engine/src/lib.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `decision-engine` crate，保持纯函数、零 IO，用于合成 70/20/10 决策。
  - 新增 `DecisionWeights`、`DecisionConfig`、`DecisionInput`、`DecisionSentiment`、`DecisionSignal` 与 `DecisionWeightMode`。
  - 默认 sentiment 可用时使用 `70/20/10`；sentiment 不可用时降级为 `90/10/0`。
  - 将 fundamental、trend 和 sentiment 归一化后合成为 `final_score`、`multiplier` 与 `action`。
  - trend 非中性体制会触发 `TacticalDelay`，避免过热追高或接飞刀。
  - Review fix：`DecisionSignal` 保留原始 `DecisionInput` 快照，便于后续审计、存储和回放。
  - Review fix：sentiment 不可用时在合成公式中使用中性映射值 `0.5`，避免自定义 fallback 权重误把缺失情绪当成极度悲观。
  - Review fix：极端低分会映射到 `Multiplier::MIN`，使 `Action::Skip` 在 Decision Engine 中可达。
  - Review fix：将 multiplier 映射改为连续函数，并复用 `Multiplier::SKIP_BELOW` 的语义，避免 final score 边界附近从 0% 跳到 55%。
  - 新增 Decision Engine 单元测试，覆盖默认权重、非法权重、标准/加码/减量、TacticalDelay 和 AI 降级。
  - 更新 `docs/minimum_mvp.md`，标记 Decision Engine 已完成，并将下一步调整为 Decision Preview API + MockBroker 串联。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p decision-engine --locked` 通过。
  - `cargo clippy -p decision-engine --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-07 22:48 UTC+10

- 执行模型：GPT-5。
- 变更类型：Futu/Moomoo OpenD paper adapter。
- 涉及文件：
  - `crates/broker/src/lib.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `OpenDOrderGateway`，作为后续 Futu/Moomoo OpenD TCP/SDK transport 的最小提交订单接口。
  - 新增 `OpenDPaperBroker`，实现 `BrokerClient`，用于把已校验订单提交到 OpenD paper gateway。
  - OpenD paper adapter 在调用 gateway 前拒绝 live config 和 live order，保持 paper trading 默认安全边界。
  - 新增 adapter 测试，覆盖正常 paper 提交、live config 拒绝、live order 不穿透 gateway、gateway unavailable 安全上抛。
  - 更新 `docs/minimum_mvp.md`，明确下一步后端顺序：先 Decision Engine，再 Decision Preview API + MockBroker 串联，之后接阿里云 Qwen API。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p broker --locked` 通过。
  - `cargo clippy -p broker --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-07 22:03 UTC+10

- 执行模型：GPT-5。
- 变更类型：Futu/Moomoo OpenD paper trading 配置底座。
- 涉及文件：
  - `crates/broker/src/lib.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `BrokerProvider`，区分 Futu 与 Moomoo OpenD 目标 provider。
  - 新增 `OpenDConnectionConfig`，校验 OpenD host、port、paper/live 环境和可选 account id。
  - OpenD 配置默认不允许 live trading；live orders 必须同时满足环境匹配和显式 live gate。
  - OpenD 配置的 account id 可供 adapter 使用，但 debug 输出会脱敏，避免进入日志。
  - 新增配置层测试，覆盖 paper 默认值、非法连接字段、account id 脱敏、环境不匹配和 live gate。
  - 更新 `docs/minimum_mvp.md`，明确演示级前端由 Jame 负责；当前后端分支只提供 API 契约、配置、安全边界和测试。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p broker --locked` 通过。
  - `cargo clippy -p broker --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-07 21:40 UTC+10

- 执行模型：GPT-5。
- 变更类型：Broker paper trading 边界与 Futu/Moomoo MVP 路线。
- 涉及文件：
  - `Cargo.toml`
  - `crates/broker/Cargo.toml`
  - `crates/broker/src/lib.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `broker` crate，定义 provider-neutral `BrokerClient` port，为后续 Futu/Moomoo OpenD adapter 预留边界。
  - 新增 `BrokerEnvironment`，区分 `Paper` 与 `Live`，默认 demo 路径面向虚拟账号 / paper trading。
  - 新增 `BrokerOrderRequest`、`BrokerOrderAck`、`BrokerOrderStatus` 与安全错误类型。
  - 订单请求通过构造器校验 idempotency key、ASCII symbol、正数数量、limit order 价格等不变量。
  - 新增 `MockBroker`，默认只接受 paper orders，拒绝 live orders；用于本地 demo 与后续 decision-to-order 测试。
  - 更新 `docs/minimum_mvp.md`，将 Futu/Moomoo OpenD paper trading、broker ack、live trading 保护开关和演示级前端展示纳入全项目 MVP 路线。
  - Review fix：将 broker crate 的 `missing_docs` 提升为 deny，并明确 MVP 只要求最终 summary，decision record 属于可选存证。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p broker --locked` 通过。
  - `cargo clippy -p broker --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-07 00:37 UTC+10

- 执行模型：GPT-5。
- 变更类型：20% 趋势层真实实现与全项目 MVP 文档补充。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/src/trend/mod.rs`
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/trend/direction.rs`
  - `crates/quant-engine/tests/DEFERRED_TESTS.md`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - `evaluate_trend` 从 `NotImplemented` 升级为真实趋势计算：MA200 distance 与 RSI 原始分位反向计入，VIX 原始分位正向计入。
  - 新增趋势体制判定：`FallingKnife` 优先于 `Overheated`，否则为 `Neutral`。
  - Review fix：对趋势合成分数执行 `[0, 1]` clamp，避免权重和浮点容忍导致边界 composite 略超上限时 panic。
  - 保留 `evaluate_trend_or_stub` 和 `evaluate_trend_stub` 作为兼容入口，但默认测试已覆盖真实 trend 行为。
  - 打开既有 trend 行为测试，不再让核心 20% trend TDD 边界保持 ignored。
  - 更新 deferred 测试说明：剩余场景主要阻塞在 Decision Engine，而不是 trend stub。
  - 更新 `docs/minimum_mvp.md`：标记 20% trend 已可复用，并补充演示级最小前端属于全项目 MVP。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p quant-engine --locked` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-07 00:09 UTC+10

- 执行模型：GPT-5。
- 变更类型：投资计划双桶执行预览 API 与 MVP 清单文档。
- 涉及文件：
  - `crates/api/src/routes/investment_plans.rs`
  - `crates/api/tests/investment_plans.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `POST /investment-plans/:id/execution-preview`，通过 API DTO 接收预览日期与可选双桶比例。
  - 执行预览 API 复用领域构造器校验 `day_of_month`、双桶比例范围与比例合计，不让 serde 直接构造领域类型。
  - due 且提供双桶配置时返回 `bucket_split`；waiting/inactive 不返回投入金额和双桶拆分。
  - 新增 route tests 覆盖 due 拆分、非执行日省略拆分，以及非法 UUID、非法日期、非法比例和非法 JSON 的统一 bad request。
  - 新增 `docs/minimum_mvp.md`，以全项目视角记录 70/20/10 最小 MVP 主线、已完成能力、20% trend/阿里云接入/decision engine/最终 summary 缺口、演示流程和非 MVP 边界。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-06 23:00 UTC+10

- 执行模型：Claude。
- 变更类型：AI 感知层新闻源接入与全链路管线。
- 涉及文件：
  - `Cargo.toml`
  - `crates/ai-client/Cargo.toml`
  - `crates/ai-client/src/lib.rs`
  - `crates/ai-client/src/news.rs`
  - `crates/ai-client/tests/news.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `NewsSource` trait 与 `RssNewsSource`，对接 CNBC US Top News RSS，拉取最近 24h 英文财经新闻。
  - 新增 `NewsItem`、`NewsSourceError`、`PipelineError` 类型。
  - 新增 `format_sentiment_prompt` 将新闻格式化为英文 AI prompt。
  - 新增 `fetch_market_sentiment` 一站式函数：拉取 → 格式化 → AI 分析 → sentiment。
  - `RssNewsSource` 支持 CDATA 解析、HTML 标签穿透、时间过滤（24h）、数量上限（10 条）、句末截断。
  - `lib.rs` 公开导出 news 模块所有类型与函数。
  - 新增 22 个单测覆盖解析/过滤/格式化/pipeline。
  - 新增 2 个 `#[ignore]` 集成测试：`real_cnbc_with_mock`（仅需网络）与 `real_cnbc_with_qwen`（需网络 + `DASHSCOPE_API_KEY`）。
- 待完成：
  - 申请 DashScope API Key，设 `DASHSCOPE_API_KEY` 环境变量，运行 `real_cnbc_with_qwen` 验证真实 Qwen 输出。
- 验证：
  - `cargo test -p ai-client --locked` 通过：106 个测试（77 单测 + 18 集成测试 + 4 doc test + 7 集成测试），含 2 个 ignored。
  - `cargo clippy -p ai-client --all-targets --all-features -- -D warnings` 通过。
  - `cargo fmt -p ai-client --check` 通过。
  - 手动跑过 `real_cnbc_with_mock`，验证 10 条真实 CNBC 新闻正常拉取、描述完整、prompt 格式正确。

### 2026-07-06 23:45 UTC+10

- 执行模型：Claude。
- 变更类型：fix（AI 感知层 code review 修复）。
- 涉及文件：
  - `crates/ai-client/src/news.rs`
  - `crates/ai-client/tests/news.rs`
- 变更内容：
  - `RssNewsSource::new` / `with_config` 改用 `reqwest::Client::builder().timeout(DEFAULT_HTTP_TIMEOUT)`（30 秒），避免 HTTP 请求无超时挂起。
  - `parse_items` 将逐片段 `trim()` 改为条目解析完成后统一起 trim，修复内联 HTML 标签导致词间空格丢失（如 `as<b>investors</b> cheered` → `asinvestors cheered`）。
  - `filter_and_convert` 在 `truncate` 前先 `sort_by_key(|item| Reverse(item.pub_date))`，确保保留最新 N 条，满足 trait 文档「按时间降序」契约。
  - `truncate_at_sentence` 改用 `char_indices().nth(max_chars)` 定位字符边界，修复 `floor_char_boundary` 对多字节字符（中文）的字节/字符语义不一致。
  - 集成测试 `real_cnbc_with_mock` / `real_cnbc_with_qwen` 将 `fetch_market_sentiment` 从重复两次调用改为一次调用复用结果，避免重复网络/API 计费。
- 验证：
  - `cargo test -p ai-client --locked` 22 个 news 单测通过。
  - `cargo clippy -p ai-client --all-targets --all-features -- -D warnings` 通过。
  - 手动跑过 `real_cnbc_with_mock`，10 条真实 CNBC 新闻正常拉取，管道全链路通过。

### 2026-07-06 20:55 UTC+10

- 执行模型：GPT-5。
- 变更类型：投资计划执行预览接入双桶拆分。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - `InvestmentPlanExecutionPreview` 新增可选 `bucket_split`，仅在 due 且调用方提供双桶配置时返回。
  - 新增 `InvestmentPlanService::preview_execution_with_buckets`，复用现有执行日判断并附带 core/opportunity 拆分。
  - 保留原 `preview_execution` 行为，默认不返回双桶拆分，避免影响现有调用方。
  - 新增测试覆盖 due 拆分、非 due 不拆分和 JSON 字符串金额契约。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-05 22:29 UTC+10

- 执行模型：GPT-5。
- 变更类型：投资计划双桶投入拆分领域模型。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `TwoBucketContributionSplit`，按已校验的双桶比例拆分本次计划投入金额。
  - 拆分结果通过构造器生成，保证 core + opportunity 等于原始计划投入金额。
  - 新增 `contribution_for`，按 `InvestmentBucket` 读取对应投入金额。
  - 新增测试覆盖总额守恒、按桶读取、非正金额拒绝和金额 JSON 字符串序列化。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-02 00:24 UTC+10

- 执行模型：GPT-5。
- 变更类型：投资计划双桶配置领域模型。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `InvestmentBucket`、`BucketAllocationRatio` 与 `TwoBucketAllocationConfig`，先定义双桶配置边界，不接执行分配算法。
  - `BucketAllocationRatio` 通过构造器保证比例位于 0..=1，避免公开字段绕过不变量。
  - `TwoBucketAllocationConfig` 要求常规定投桶和机会桶比例合计为 1。
  - 新增测试覆盖比例边界、比例求和和 JSON 字符串序列化契约。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-01 00:19 UTC+10

- 执行模型：GPT-5。
- 变更类型：投资计划执行预览领域骨架。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `PreviewInvestmentPlanExecution`、`ExecutionPreviewStatus` 与 `InvestmentPlanExecutionPreview`，用于表达计划在指定月内日期的轻量执行预览。
  - `InvestmentPlanService::preview_execution` 复用 repository get，区分 `due`、`waiting`、`inactive`，并仅在 due 时返回不超过单次执行上限的计划投入金额。
  - 明确该预览不生成 broker order、不处理成交状态，也不包含双桶资金分配。
  - 新增测试覆盖 due、waiting、inactive 与非法预览日期。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-06-28 18:36 UTC+10

- 执行模型：GPT-5。
- 变更类型：投资计划 API（更新路由）。
- 涉及文件：
  - `crates/api/src/routes/investment_plans.rs`
  - `crates/api/src/lib.rs`
  - `crates/api/tests/investment_plans.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `PATCH /investment-plans/:id`，通过 API DTO 转换到领域 `UpdateInvestmentPlan`，不让 serde 直接构造领域类型。
  - 路径 UUID 与 JSON 解析失败统一映射为 `ApiError::BadRequest`，保持 JSON error envelope。
  - CORS 允许方法补充 `PATCH`。
  - 新增路由测试覆盖字段合并、金额组合校验、非法 ID 与非法 JSON。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-api --locked` 通过。

### 2026-06-27 00:20 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试夹具语义修正（趋势中性历史）。
- 涉及文件：
  - `crates/quant-engine/tests/common/mod.rs`
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/trend/direction.rs`
  - `crates/quant-engine/tests/trend/indicators.rs`
  - `crates/quant-engine/tests/trend/regime.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `neutral_weighted_history` 与 `TREND_NEUTRAL_CURRENT`，用低/高样本成对交错构造加权 ECDF 语义上的趋势中性历史。
  - `neutral_trend_snapshot` 改用加权中性历史，不再把 `standard_history() + 50.5` 标注为“历史中位 / 中性分位”。
  - 新增 `neutral_weighted_history_is_near_half_under_trend_config`，直接验证趋势测试配置下加权分位接近 0.5。
  - 同步修正趋势行为测试中的中性场景注释与夹具使用。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine --test trend direction::neutral_weighted_history -- --nocapture` 通过。
  - `cargo test -p quant-engine --test trend direction::evaluate_trend -- --nocapture` 通过。
  - `cargo test -p quant-engine` 通过：22 passed, 29 ignored。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-27 00:15 UTC+10

- 执行模型：Composer。
- 变更类型：公开 API 语义（趋势层未实现显式化）。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/src/trend/mod.rs`
  - `crates/quant-engine/tests/trend/direction.rs`
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - `evaluate_trend` 由 `todo!()` 改为返回 `QuantError::NotImplemented`；文档说明调用方须降级 stub 或 Skip。
  - 新增 `QuantError::NotImplemented` 及 Display 文案。
  - 新增过渡期入口 `evaluate_trend_or_stub`：`NotImplemented` 时降级为中性 stub，其余错误原样传播。
  - 新增测试 `evaluate_trend_returns_not_implemented`、`evaluate_trend_or_stub_falls_back_to_neutral_stub`。
- 验证：
  - `cargo test -p quant-engine` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。

### 2026-06-27 00:05 UTC+10

- 执行模型：Composer。
- 变更类型：测试策略（趋势层 CI 隔离）。
- 涉及文件：
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/trend/direction.rs`
  - `crates/quant-engine/tests/trend/errors.rs`
  - `crates/quant-engine/tests/trend/indicators.rs`
  - `crates/quant-engine/tests/trend/regime.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `trend_deferred_test!` 宏，为依赖 `evaluate_trend` 的 TDD 边界测试统一标记 `#[ignore]`。
  - CI 默认仅运行 `config` 不变量测试（17）与 stub 契约测试（2）；29 个行为测试保留供实现期本地验证。
  - 本地全量命令：`cargo test -p quant-engine --test trend -- --ignored`。
- 验证：
  - `cargo test -p quant-engine --test trend` 通过：19 passed, 29 ignored。

### 2026-06-26 23:55 UTC+10

- 执行模型：Composer。
- 变更类型：语义对齐（趋势层默认月频契约）。
- 涉及文件：
  - `crates/quant-engine/src/trend/mod.rs`
  - `crates/quant-engine/tests/trend/config.rs`
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/common/mod.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 趋势层默认 `min_len` 由 252 改为 60（5 年月度），与基本面层同源。
  - 模块/`TrendConfig`/`TrendSnapshot` 文档明确默认契约为**月度样本**；日频接入须显式配置 `EwPercentileConfig`。
  - 常量重命名为 `DEFAULT_HALF_LIFE_MONTHS`，消除日频/月频注释矛盾。
  - 新增测试 `default_percentile_config_matches_fundamental`，锁定趋势层与基本面层默认分位配置一致。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine --test trend config::default` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。

### 2026-06-26 23:42 UTC+10

- 执行模型：Composer。
- 变更类型：测试补强（趋势权重和容忍边界）。
- 涉及文件：
  - `crates/quant-engine/tests/trend/config.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `accepts_weight_sum_exactly_one`：权重和 = 1.0 构造成功。
  - 新增 `accepts_weight_sum_within_tolerance`：偏差在 `1e-9` 内构造成功。
  - 新增 `rejects_weight_sum_beyond_tolerance`：偏差超过 `1e-9` 返回 `InvalidWeight`。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine --test trend config::accepts_weight_sum config::rejects_weight_sum` 通过。

### 2026-06-26 23:35 UTC+10

- 执行模型：Composer。
- 变更类型：测试补强（趋势阈值非法）。
- 涉及文件：
  - `crates/quant-engine/tests/trend/config.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `rejects_invalid_falling_knife_threshold` 改为与 `overheated_above` 对称的超界用例（`1.5`）。
  - 新增 `rejects_nan_overheated_threshold`（`overheated_above = NaN`）。
  - 新增 `rejects_negative_threshold`（`overheated_above = -0.1`）。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine --test trend config::rejects` 通过。

### 2026-06-26 23:28 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试补强（趋势体制边界）。
- 涉及文件：
  - `crates/quant-engine/tests/trend/regime.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `strict_boundary_config` 测试辅助配置，将 `overheated_above` 与 `falling_knife_above` 设置为 `1.0`。
  - 补充 `ma_p == overheated_above`、`rsi_p == overheated_above`、`vix_p == falling_knife_above` 三个边界测试，锁定趋势体制判定使用严格 `>`，等于阈值时保持 `Neutral`，避免误触发 TacticalDelay。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine --no-run` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-26 23:23 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：错误语义修正（趋势阈值）。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/src/trend/mod.rs`
  - `crates/quant-engine/tests/trend/config.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `QuantError::InvalidPercentileThreshold { name, value }`，用于表达分位阈值非法，避免 `overheated_above` / `falling_knife_above` 继续复用 `InvalidWeight`。
  - `TrendConfig::new` 在构造 `overheated_above` 与 `falling_knife_above` 时返回带阈值名称的结构化错误。
  - 更新趋势配置测试，并补充 `falling_knife_above` 非法阈值覆盖；更新错误 `Display` 测试锁定新分支文案。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine --no-run` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - `cargo test -p quant-engine --test percentile quant_error_display_is_descriptive` 通过。
  - `cargo test -p quant-engine --test trend config::rejects_invalid` 通过：2 个趋势阈值错误测试全部通过。

### 2026-06-26 23:20 UTC+10

- 执行模型：claude-sonnet-4-5。
- 变更类型：feat（趋势层存根 + 全量测试边界）。
- 涉及文件：
  - `crates/quant-engine/src/trend/mod.rs`
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/common/mod.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - **`trend/mod.rs`**：重写为完整存根（函数签名 + `todo!()`），新增以下公开类型与函数：
    - `TrendWeights`：三指标子权重（MA/RSI/VIX），构造时校验各自在 `[0,1]` 且和 ≈ 1.0。
    - `TrendConfig`：子权重 + 分位配置 + `overheated_above` / `falling_knife_above` 阈值，提供 `Default`。
    - `TrendSnapshot`：三指标历史序列 + 当前读数的输入快照，与 `FundamentalSnapshot` 同构。
    - `TrendRegime`：`Overheated / Neutral / FallingKnife` 离散节奏体制标签，供 Decision Engine 触发 `TacticalDelay`。
    - `TrendSignal`：连续 `score`（`0.0=赶顶, 1.0=接飞刀`）+ 三个未反向审计分位 + `regime`。
    - `evaluate_trend`：纯函数存根，`todo!()` 占位；文档注释完整描述合成公式与体制判定规则。
    - `evaluate_trend_stub`：标 `#[deprecated]`，过渡期保留，`regime` 补充为 `Neutral`。
  - **`lib.rs`**：导出全部新增趋势层公开 API；注释同步说明趋势层现状。
  - **`tests/common/mod.rs`**：新增趋势层夹具常量（权重、阈值、历史长度）与 helper（`neutral/overheated/falling_knife_trend_snapshot`、`trend_balanced_test_config`、`trend_config_with_weights`）。
  - **`tests/trend.rs`**：完整测试边界（38 个测试），覆盖：
    - A 过渡存根契约（2 个）
    - B 方向性（3 个）
    - C 单指标隔离（6 个，验证 MA/RSI 反向、VIX 正向）
    - D 审计字段未反向（3 个）
    - E 节奏体制（5 个，含 FallingKnife 优先级）
    - F 错误传播（7 个：NaN/Inf/历史不足/不等长）
    - G 配置不变量（6 个：构造期校验）
    - H 默认配置契约（4 个）
- 验证：
  - `cargo test -p quant-engine --no-run` 通过，零 warning，零 error。
  - `cargo test -p quant-engine` 执行结果：fundamental 20 个 ✅ / percentile 25 个 ✅ / trend 存根/配置相关 12 个 ✅ / 待实现 26 个以 `todo!()` 正确 panic（符合存根阶段预期）；现有测试无退化。

### 2026-06-26 23:14 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试结构整理（不改变测试语义）。
- 涉及文件：
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/trend/direction.rs`
  - `crates/quant-engine/tests/trend/indicators.rs`
  - `crates/quant-engine/tests/trend/regime.rs`
  - `crates/quant-engine/tests/trend/errors.rs`
  - `crates/quant-engine/tests/trend/config.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 800+ 行的 `tests/trend.rs` 拆为单一集成测试入口 + `tests/trend/` 子模块目录，保留 Cargo test binary 为 `trend`。
  - 按测试关注点拆分为 `direction`（存根契约/方向性）、`indicators`（单指标隔离/审计字段）、`regime`（节奏体制）、`errors`（错误传播）、`config`（配置不变量/默认契约）。
  - 入口文件提供共享 prelude，减少各子模块重复导入，并让 CI 输出出现 `trend::direction::...` 等更精确的失败路径。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。
  - `cargo test -p quant-engine --no-run` 通过，拆分后的 `trend` 集成测试模块编译成功。
  - `cargo test -p quant-engine` 已运行并可编译新模块结构；当前因既有 `evaluate_trend` 仍为 `todo!()`，非配置/存根类趋势边界测试按预期失败，待趋势实现落地后转绿。

### 2026-06-26 22:50 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：结构重构（模块边界调整）。
- 涉及文件：
  - `crates/quant-engine/src/weight.rs`
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `weight` 模块承载跨层共享的 `Weight` newtype，避免趋势层未来复用权重类型时依赖 `fundamental` 模块。
  - 从 `fundamental/mod.rs` 移除 `Weight` 实现，改为引用 crate 共享导出的 `Weight`。
  - `lib.rs` 新增 `pub mod weight` 并从 `weight` 重新导出 `Weight`，保持外部 `quant_engine::Weight` API 路径不变。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：20 个 fundamental 测试、25 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。
  - `cargo llvm-cov -p quant-engine --summary-only --show-missing-lines` 通过：Region / Function / Line 覆盖率均为 100.00%，新增 `weight.rs` 行覆盖率 100.00%。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-26 22:42 UTC+10

- 执行模型：Codex；变更类型：feat/test（Investment Plan API create/list/get）。
- PR 范围：PR 7，仅新增 API DTO、create/list/get routes、safe error mapping 与 route tests；不实现 update/set active routes、Scheduler、Broker、Qwen、订单状态机、`ExecutionPlan` 或双桶逻辑。
- 涉及文件：`Cargo.lock`、`crates/api/**`、`CHANGE_LOG.md`。
- 变更内容：入站 JSON 先反序列化到 DTO 再转领域输入，避免 serde 直接构造领域类型；`ApiState` 持有 `InvestmentPlanService`，production 路径使用 storage adapter，测试路径使用 fake repository。
- Review fix：create 成功返回 `201 Created`；JSON/Path extractor 失败统一映射为项目错误 envelope；补齐转换函数文档注释以满足 docstring coverage。
- 验证：`cargo test -p indexlink-api --locked` 与完整 workspace fmt/check/test/clippy 均通过。

### 2026-06-26 16:43 UTC+10

- 执行模型：Codex。
- 变更类型：feat/db（Investment Plan PostgreSQL migration）。
- PR 范围：PR 6，仅新增 `investment_plans` 表结构 migration；不接 API、不实现 Scheduler、Broker、Qwen、订单状态机、`ExecutionPlan` 或双桶逻辑。
- 涉及文件：
  - `migrations/20260626064200_create_investment_plans.sql`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `investment_plans` 表，字段与 `PostgresInvestmentPlanRepository` adapter 使用的 SQL 契约一致。
  - 使用 `UUID` 主键、`NUMERIC(20, 8)` 金额、`TIMESTAMPTZ` 审计时间和 `monthly` MVP schedule。
  - 增加数据库约束保护领域不变量：名称 trim、symbol 大写 ASCII、currency 三位大写、执行日 1..=28、金额为正且 `max_single_execution >= base_contribution`。
  - 增加按创建顺序和 active schedule 的索引，支撑当前 list 顺序与后续 scheduler 查询。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。
  - 静态检查确认 migration 包含 `investment_plans` 表、领域约束和索引。

### 2026-06-25 22:07 UTC+10

- 执行模型：Codex。
- 变更类型：feat/test（Investment Plan PostgreSQL repository adapter）。
- PR 范围：PR 5，仅新增 storage crate 中的 PostgreSQL repository adapter；不新增 migration、不接 API、不实现 Scheduler、Broker、Qwen、订单状态机、`ExecutionPlan` 或双桶逻辑。
- 涉及文件：
  - `Cargo.lock`
  - `Cargo.toml`
  - `crates/storage/Cargo.toml`
  - `crates/storage/src/lib.rs`
  - `crates/storage/src/investment_plans.rs`
- 变更内容：
  - `indexlink-storage` 新增 `PostgresInvestmentPlanRepository`，实现 `InvestmentPlanRepository` port。
  - 支持 create、list、get、update 与 set active；update 使用事务与 `FOR UPDATE` 在写入路径内合并并校验最终金额组合。
  - SQL 边界使用 PostgreSQL cast 与文本/epoch 映射，避免扩大 sqlx feature 面。
  - 新增 storage adapter 单元测试覆盖 SQLx 错误安全映射与最终金额组合校验。
  - Review fix：为公开 re-export 的 `PostgresInvestmentPlanRepository` 补齐文档注释，满足 public API 文档要求。
- 验证：
  - `cargo test -p indexlink-storage --locked` 通过：8 个 storage 与 adapter 单元测试通过。
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-06-24 23:39 UTC+10

- 执行模型：Codex。
- 变更类型：feat/test（Investment Plan 更新与启停应用用例）。
- PR 范围：PR 4，仅新增 update 与 set active 应用服务契约及 fake repository 测试；不实现 storage adapter、migration、API、Scheduler、Broker、Qwen、订单状态机或 `ExecutionPlan`。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - `InvestmentPlanRepository` port 新增 `update` 与 `set_active`。
  - `InvestmentPlanService` 新增 `update` 与 `set_active`；update 会先规范化输入，再交由 repository 在原子写入路径内校验最终 `base_contribution` / `max_single_execution` 关系。
  - fake repository 支持更新字段、启停状态与 `updated_at` 变化。
  - 新增应用服务测试覆盖字段更新、最终金额上限校验和启停用例。
  - Review fix：将 update 最终金额组合校验移动到 repository 原子写入路径内，避免 service 层读写窗口。
  - Review fix：补齐本 PR 新增 helper、fake repository 与测试函数文档注释，提高 docstring coverage。
- 验证：
  - `cargo test -p investment-plans` 通过：17 个领域、Decimal 与应用服务契约测试通过。
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-06-24 23:11 UTC+10

- 执行模型：Codex。
- 变更类型：feat/test（Investment Plan repository port 与应用服务）。
- PR 范围：PR 3，仅新增 repository port、create/list/get 应用服务契约与 fake repository 测试；不实现 update 用例、storage adapter、migration、API、Scheduler、Broker、Qwen、订单状态机或 `ExecutionPlan`。
- 涉及文件：
  - `Cargo.lock`
  - `crates/investment-plans/Cargo.toml`
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `InvestmentPlanRepository` outbound port，定义 `create`、`list`、`get` 契约。
  - 新增 `PlanRepositoryError` 与 `PlanApplicationError`，保持持久化错误文案安全，不泄露数据库细节。
  - 新增 `InvestmentPlanService`，在 create 用例中先调用领域 `normalize()`，再调用 repository port。
  - 使用 fake repository 测试 create/list/get、NotFound/Unavailable 错误映射，并保持领域类型不直接派生 `Deserialize`。
- 验证：
  - `cargo test -p investment-plans` 通过：14 个领域、Decimal 与应用服务契约测试通过。
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-06-24 22:37 UTC+10

- 执行模型：Codex。
- 变更类型：feat/test（Investment Plan 领域模型与校验）。
- PR 范围：PR 2，仅实现投资计划领域类型、字段规范化与输入校验；不实现 repository、storage adapter、migration、API、Scheduler、Broker、Qwen、订单状态机或 `ExecutionPlan`。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/investment-plans/Cargo.toml`
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `ScheduleKind`、`InvestmentPlan`、`CreateInvestmentPlan`、`UpdateInvestmentPlan` 与 `PlanValidationError`。
  - 创建输入支持 name trim、symbol trim + 大写、currency trim + 大写、monthly day 1..=28、Decimal 金额正数与 `max_single_execution >= base_contribution` 校验。
  - 更新输入禁止修改 symbol / currency / schedule_kind，并拒绝空 PATCH；同时保留 Decimal 字符串 JSON 契约。
  - 领域类型不直接派生 `Deserialize`，避免入站 JSON 绕过 `normalize()`；后续 API adapter 应先反序列化 DTO 再进入领域模型。
  - `symbol` 规范化新增 ASCII 校验，拒绝非 ASCII 标的代码。
  - 新增 `uuid` 与 `time` 作为领域模型 ID 和时间字段类型，未启用 SQLx 对应 feature。
- 验证：
  - `cargo test -p investment-plans` 通过：10 个领域与 Decimal 契约测试通过。
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-06-24 22:08 UTC+10

- 执行模型：Codex。
- 变更类型：chore/test（Investment Plan 模块与金额基础）。
- PR 范围：PR 1A，仅建立 investment-plans crate 骨架与 Decimal JSON 契约；不实现领域模型、repository、migration、API 或执行逻辑。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/investment-plans/Cargo.toml`
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `investment-plans` workspace crate，作为投资计划领域与应用层边界。
  - 声明 `rust_decimal` 依赖；`uuid`、`time` 与 SQLx 对应 feature 因 lockfile 行数限制留到后续更小 PR。
  - 添加 Decimal JSON 字符串契约测试，确保金额从字符串反序列化并以字符串序列化，拒绝 JSON number。
  - 模块文档记录当前 MVP 假设：单用户、仅 monthly、无计划级 timezone、不验证 symbol、不计算本期买入金额或双桶资金分配。
- 验证：
  - `cargo test -p investment-plans` 通过：3 个 Decimal JSON 契约测试通过。
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-06-24 23:30 UTC+10

- 执行模型：Claude。
- 变更类型：test（AI 语义感知层：集成测试）。
- 涉及文件：
  - `crates/ai-client/tests/client.rs`
  - `crates/ai-client/tests/provider.rs`
  - `crates/ai-client/tests/sentiment.rs`
- 变更内容：
  - 本地 axum HTTP mock server 模拟千问，验证 `QwenClient` 请求格式、响应解析、错误降级全链路。
  - `MockAiProvider` 关键词匹配测试（正向/负向/中性/自定义默认值）。
  - `Sentiment` 边界测试（构造、范围、比较、f64 互转）。
  - `AiConfig` 安全测试（api_key 不出现在 Debug/Display 中）。
- 验证：
  - `cargo test -p ai-client`：66 测试全部通过。

### 2026-06-24 23:15 UTC+10

- 执行模型：Claude。
- 变更类型：feat（AI 语义感知层：客户端实现 + Mock）。
- 涉及文件：
  - `crates/ai-client/Cargo.toml`
  - `crates/ai-client/src/client.rs`
  - `crates/ai-client/src/mock.rs`
  - `crates/ai-client/src/lib.rs`
- 变更内容：
  - `QwenClient`：对接 Qwen DashScope API，system prompt 约束模型输出结构化 JSON，三级步进降级。
  - `MockAiProvider`：关键词匹配的本地假 AI（大涨→正向、大跌→负向，未匹配→中性），零网络零成本。
  - `lib.rs` 统一导出 `QwenClient`、`MockAiProvider`、`AiProvider`、`AiConfig`、`Sentiment`。
- 验证：
  - `cargo test -p ai-client`：全部通过。

### 2026-06-24 23:00 UTC+10

- 执行模型：Claude。
- 变更类型：feat（AI 语义感知层：核心类型与接口定义）。
- 涉及文件：
  - `crates/ai-client/src/sentiment.rs`
  - `crates/ai-client/src/error.rs`
  - `crates/ai-client/src/provider.rs`
- 变更内容：
  - `Sentiment` newtype：`[-1.0, +1.0]` 有界情绪值，NaN→0、越界自动截断，Display 安全。
  - `AiClientError`：六种错误变体（Timeout / Transport / HttpStatus / InvalidJson / UnexpectedStructure / ParseFailure / EmptyResponse），所有 Display 不暴露密钥/URL。
  - `AiConfig`：千问连接配置（默认 DashScope `qwen-plus`），Debug 将 api_key 显示为 `<redacted>`。
  - `AiProvider` trait（`async_trait`）：可替换的 LLM 后端抽象，与 `ReadinessCheck` 同模式。
- 验证：
  - `cargo test -p ai-client`：42 单元测试全部通过。

### 2026-06-24 23:00 UTC+10

- 执行模型：Claude。
- 变更类型：chore（workspace 注册 ai-client）。
- 涉及文件：
  - `Cargo.toml`（根 workspace 注册 + reqwest 依赖声明）
  - `Cargo.lock`
- 变更内容：
  - 将 `ai-client` 加入 workspace members。
  - 声明 `ai-client` workspace dependency。
  - 声明 `reqwest` workspace dependency（`json` + `rustls-tls`）。
- 验证：
  - `cargo build --workspace` 通过。
  - `cargo test --workspace` 全部通过。

### 2026-06-21 21:15 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试补强（覆盖率修补）。
- 涉及文件：
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 调查 `cargo llvm-cov -p quant-engine --summary-only --show-missing-lines` 报告，确认缺失覆盖集中在 `weighted_percentile_of` 的总有效权重下溢防御分支。
  - 新增 `weighted_percentile_returns_insufficient_when_all_valid_weights_underflow`，构造 `alpha = 1.0`、最新样本为 `NaN`、旧端有效样本权重归零的场景，锁定该分支返回 `QuantError::InsufficientHistory`。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：20 个 fundamental 测试、25 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo llvm-cov -p quant-engine --summary-only --show-missing-lines` 通过：Region / Function / Line 覆盖率均为 100.00%，缺失行清零。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-21 21:02 UTC+10

- 执行模型：Claude Opus 4.8。
- 变更类型：功能实现（指数加权 ECDF）+ 配套测试修正。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/src/percentile.rs`
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - `lib.rs`：`QuantError` 新增 `InvalidHalfLife { value }` 与 `InvalidDecay { alpha }` 两个结构化分支及对应 `Display`；根级导出 `weighted_percentile_of` 与 `EwPercentileConfig`。
  - `percentile.rs`：新增 `EwPercentileConfig`（`from_half_life` / `from_alpha` 双构造入口，`alpha = 1 - 0.5^(1/H)`，校验半衰期、衰减系数与 `min_len`）与 `weighted_percentile_of`；历史按「旧→新」加权，最新样本权重 1，NaN 跳过但不压缩滞后，并对有效样本不足与权重下溢返回 `InsufficientHistory`；保留原无权 `percentile_of` 不变。
  - `fundamental/mod.rs`：`FundamentalConfig` 以 `percentile_config: EwPercentileConfig` 取代 `min_history_len`，`new` 改为接收 `EwPercentileConfig`，`Default` 采用半衰期 36 个月 + 最少 60 个有效月度样本；`evaluate_fundamental` 改用 `weighted_percentile_of`，ERP 倒置与合成逻辑保持不变。
  - `tests/fundamental.rs`：将 `fundamental_expensive_market` / `fundamental_cheap_market` 的当前读数改为明确超出历史范围的极值，使方向性对任意半衰期稳健（修正旧位置分位魔法数字在加权下失真的问题）；`rate_repricing` 的 CAPE 中性断言由精确容差放宽为近似容差（加权 ECDF 因截断尾项无法精确等于 0.50）。
- 验证：
  - `cargo test -p quant-engine`：fundamental 20 + percentile 24 + trend 1 + doc 1 全部通过。
  - `cargo test -p core-domain`：13 项单元测试通过。
  - `cargo fmt -p quant-engine --check` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。
  - 改动源文件无 IDE linter 错误。

### 2026-06-21 20:48 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试先行（指数加权 ECDF 契约）。
- 涉及文件：
  - `crates/quant-engine/tests/common/mod.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 测试夹具改为通过 `EwPercentileConfig` 构造 `FundamentalConfig`，默认契约对齐 readme：指数加权 ECDF 半衰期 36 个月，最少 60 个有效月度历史点。
  - `fundamental` 集成测试改为断言 `percentile_config`、加权中性位置、ERP 原始分位审计字段，以及新配置构造入口下的非法权重/历史长度错误。
  - `percentile` 集成测试新增指数加权 ECDF 契约：半衰期到 alpha 映射、非法半衰期/衰减系数、单调性、旧→新顺序敏感、NaN 不压缩 lag、最旧样本退出时的平滑变化、错误传播和新增错误类型展示文案。
  - 当前仅修改测试，生产实现尚未新增 `EwPercentileConfig`、`weighted_percentile_of`、`FundamentalConfig::percentile_config` 及对应 `QuantError` 分支。
- 验证：
  - `cargo fmt -p quant-engine --check` 通过。
  - `cargo test -p quant-engine` 预期失败：生产代码尚未实现测试引用的新 API 与错误分支（`EwPercentileConfig`、`weighted_percentile_of`、`InvalidHalfLife`、`InvalidDecay`、`percentile_config`）。

### 2026-06-21 20:18 UTC+10

- 执行模型：Sonnet 4.6。
- 变更类型：文档（设计决策更新）。
- 涉及文件：
  - `readme.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - **指数加权 ECDF**：将历史分位计算方法由无权 ECDF 升级为指数加权 ECDF；以半衰期为唯一旋钮（$\alpha = 1 - 0.5^{1/H}$，默认 $H$ = 36 个月月度数据），消除硬窗口「幽灵跌落」效应，同时保持无分布假设，输出仍为 `[0, 1]` 分位；更新决策管线说明、Quant Engine 模块职责描述、MVP 阶段落地描述。
  - **双桶现金池（Two-Bucket Execution）**：在执行层引入副桶（Buffer Bucket）消除现金拖累；确立四条核心规则（副桶是弹药缓冲池、取出量受余额约束、副桶设累积上限、现金流策略可配置）；新增 `Conservative`（默认）/ `Aggressive` 两种策略对比表及资金流示意；在「分阶段落地」第 4 阶段中纳入双桶；在关键功能列表中新增双桶条目。
  - 上述改动均为文档层面，未修改任何 Rust 代码。
- 验证：
  - 文档改动，未运行测试。

### 2026-06-20 21:36 UTC+10

- 执行模型：Codex。
- 变更类型：后端测试覆盖率提升。
- 涉及文件：
  - `crates/api/Cargo.toml`
  - `crates/api/src/error.rs`
  - `crates/api/src/state.rs`
  - `crates/api/tests/health.rs`
  - `crates/storage/src/lib.rs`
  - `apps/server/src/config.rs`
  - `Cargo.lock`
  - `CHANGE_LOG.md`
- 变更内容：
  - API 集成测试保留 `/health`、`/ready` 与 CORS 跨模块 HTTP 契约；错误序列化、readiness backend 和 Debug 脱敏测试贴近对应源文件放置。
  - 补充 health 不访问数据库、自定义版本、CORS 预检与拒绝、安全 503 JSON、request ID 序列化以及 Storage backend 错误脱敏测试。
  - Storage 补充非法 URL、lazy pool、关闭连接池 ping 映射和结构化错误安全文案测试，并新增 `Storage::from_pool` 作为连接池依赖注入入口。
  - 将 server 环境读取重构为委托给纯 `Config::from_lookup` 解析入口，覆盖默认值、自定义值、非法输入、CORS 列表和敏感信息保护；未改变环境变量名及 `APP_PORT=0` 行为。
  - 未修改 CI workflow；`.github/workflows/rust-ci.yml` 与本次分支基线一致。
- llvm-cov 修改前：
  - `indexlink-api`：region 75.76%，function 82.35%，line 83.00%。
  - `indexlink-storage`：region 43.90%，function 50.00%，line 54.55%。
  - `indexlink-server`：region/function/line 均为 0.00%。
- llvm-cov 修改后：
  - `indexlink-api`：region 98.15%，function 96.15%，line 98.36%。
  - `indexlink-storage`：region 84.71%，function 95.00%，line 91.03%。
  - `indexlink-server`：整体 region 75.83%，function 75.00%，line 79.78%；其中 `config.rs` region 96.17%，function 93.75%，line 98.19%。
- 验证：
  - API 6 项单元测试与 8 项 HTTP 集成测试通过；Storage 6 项单元测试通过；server config 15 项单元测试通过。
  - 三个后端包的 llvm-cov 干净复测通过。
  - HTML 报告生成于本地 `target/llvm-cov/html`，未纳入 Git。
  - `cargo check --workspace --locked` 通过；`cargo test --workspace --locked` 共 86 项测试通过。
  - 三个后端包的 rustfmt check 与严格 Clippy（`-D warnings`）通过。
  - 全 workspace rustfmt check 仍被 `crates/core-domain/src/lib.rs` 三处既有格式阻塞；全 workspace Clippy 仍被该文件两个 `double_must_use` lint 阻塞，按责任边界未修改。

### 2026-06-20 21:30 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：CI 配置重构。
- 涉及文件：
  - `.github/workflows/rust-ci.yml`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Rust CI 从单个串行 `test` job 拆分为独立的 `fmt`、`clippy`、`test`、`coverage` jobs，便于 GitHub Checks 单独定位失败阶段。
  - `fmt` job 仅安装 `rustfmt` 并执行 `cargo fmt --all -- --check`；`clippy` job 仅安装 `clippy` 并执行严格 clippy；`test` job 执行 workspace 测试。
  - 新增 `coverage` job，安装 `llvm-tools-preview` 与 `cargo-llvm-cov`，执行 `cargo llvm-cov --workspace --all-features --summary-only`。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - `cargo llvm-cov --workspace --all-features --summary-only` 通过：workspace 行覆盖率 67.77%，`core-domain` 与 `quant-engine` 行覆盖率均为 100.00%。

### 2026-06-20 21:07 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：代码质量修复（Clippy warnings）。
- 涉及文件：
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 移除 `Weight::complement` 上与 `Weight` 类型级 `#[must_use]` 重复的函数级 `#[must_use]`，修复 `clippy::double_must_use`。
  - 将 `FundamentalConfig::new` 中不必要的 `ok_or_else` 改为 `ok_or`，修复 `clippy::unnecessary_lazy_evaluations`。
  - 将测试中的 `std::iter::repeat(...).take(...)` 改为 `std::iter::repeat_n(...)`，修复 `clippy::manual_repeat_n`。
- 验证：
  - `cargo fmt --all` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。
  - `cargo test --workspace` 通过。

### 2026-06-20 19:38 UTC+10

- 执行模型：Codex。
- 变更类型：后端基础设施建设。
- 涉及文件：
  - `Cargo.toml`
  - `.gitignore`
  - `.env.example`
  - `rust-toolchain.toml`
  - `crates/storage/**`
  - `crates/api/**`
  - `apps/server/**`
  - `deployment/**`
  - `.github/workflows/rust-ci.yml`
  - `readme.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 PostgreSQL storage、Axum API 与 server composition root 注册进 Rust 2021 workspace。
  - 新增带连接超时和结构化错误的 PostgreSQL 连接池基础设施，不包含业务表或 repository。
  - 新增 `/health` 与 `/ready`、统一安全错误响应、可替换 readiness 检查、Trace、CORS 配置入口与请求体上限。
  - 新增环境配置、结构化日志、Ctrl+C/SIGTERM 优雅停机、多阶段 Dockerfile、本地 PostgreSQL Compose 与 Rust CI。
  - 补充后端本地启动、环境变量、Docker Compose 和基础端点文档；未修改 `core-domain` 或 `quant-engine`。
- 验证：
  - 安装并使用 Rust/Cargo 1.96.0、rustfmt 与 clippy；`cargo check --workspace --locked` 通过。
  - 新增后端 crate 的 `cargo fmt --check` 与严格 Clippy（`-D warnings`）通过。
  - `cargo test --workspace --locked` 通过：56 项单元、集成与文档测试全部成功。
  - `docker compose -f deployment/docker-compose.yml config` 通过；本机 Docker daemon 未安装/运行，镜像构建与 HTTP 实测未执行成功。
  - workspace 全量 rustfmt/Clippy 被 `crates/core-domain/src/lib.rs` 的既有格式和两个 `double_must_use` lint 阻塞；按任务边界未修改该 crate。
  - `git diff --check` 通过，且 `core-domain`、`quant-engine` 最终均无修改。

### 2026-06-20 14:05 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试补全（覆盖率提升）。
- 涉及文件：
  - `crates/quant-engine/tests/fundamental.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 `Weight` 公开转换契约补充集成测试：`TryFrom<f64> for Weight` 接受合法原始权重，以及 `From<Weight> for f64` 可回取底层数值。
  - 覆盖此前 `cargo llvm-cov -p quant-engine --test fundamental` 报告中 `fundamental/mod.rs` 未覆盖的转换 trait 行。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：20 个 fundamental 测试、16 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo llvm-cov -p quant-engine --test fundamental --summary-only --show-missing-lines` 通过：`crates/quant-engine/src/fundamental/mod.rs` 行覆盖率、函数覆盖率与 region 覆盖率均为 100%。

### 2026-06-20 13:56 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：实现可读性整理（不改变行为）。
- 涉及文件：
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `FundamentalConfig::default()` 中的默认 CAPE 权重 `0.5` 和默认历史长度 `60` 提升为模块内常量，减少实现侧魔法数字。
  - 保持默认配置行为不变：CAPE/ERP 各半，最少 5 年月度历史数据。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过。
  - `cargo test -p core-domain` 通过。

### 2026-06-20 13:50 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：公开 API 重构（错误类型结构化）。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/src/percentile.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `QuantError::InvalidInput(String)` 拆分为 `InvalidWeight { value }`、`InvalidMinHistoryLen { value }`、`InvalidCurrentValue { indicator, value }`，便于调用方按错误语义精确匹配。
  - 更新 `Weight::new`、`FundamentalConfig::new` 与 `percentile_of`，分别返回对应结构化错误分支。
  - 更新测试断言与 `Display` 测试，避免依赖通用字符串错误来区分输入异常。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：18 个 fundamental 测试、16 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-20 13:47 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：公开 API 易用性增强。
- 涉及文件：
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为公开类型 `Weight` 实现 `Display`，以 `50.0%` 这类百分比格式输出，便于审计日志阅读。
  - 补充 `weight_display_uses_percent_format_for_audit_logs` 测试，锁定默认 CAPE 权重的展示格式。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过。
  - `cargo test -p core-domain` 通过。

### 2026-06-20 13:45 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：公开 API 易用性增强。
- 涉及文件：
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 `FundamentalConfig`、`FundamentalSnapshot`、`FundamentalSignal` 派生 `PartialEq`，方便测试断言和上层审计回放进行逐字段精确比较。
  - 保持不派生 `Eq`，避免为包含浮点语义的类型引入不合适的全等承诺。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过。
  - `cargo test -p core-domain` 通过。

### 2026-06-20 12:12 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：审计修复（金融输入有限性校验）。
- 涉及文件：
  - `crates/quant-engine/src/percentile.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `percentile_of` 的当前读数校验从仅拒绝 `NaN` 收紧为拒绝所有非有限数，`±Inf` 现在返回 `QuantError::InvalidCurrentValue`。
  - 更新 `percentile` 边界测试，不再把 `+Inf` / `-Inf` 锁定为合法极端分位。
  - 更新 fundamental 层传播测试，确认 CAPE/ERP 当前读数为非有限数时向上传播 `InvalidCurrentValue`。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：17 个 fundamental 测试、16 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-20 12:06 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：命名重构（公开 API 语义收敛）。
- 涉及文件：
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 fundamental 层输入快照从 `MarketSnapshot` 重命名为 `FundamentalSnapshot`，避免未来与趋势层快照或上层聚合市场快照混淆。
  - 同步根级导出与 fundamental 集成测试，不保留旧名兼容导出。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过。
  - `cargo test -p core-domain` 通过。

### 2026-06-20 12:04 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：审计修复（分位计算输入校验）。
- 涉及文件：
  - `crates/quant-engine/src/percentile.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在公共函数 `percentile_of` 入口拒绝 `min_len = 0`，避免空历史在长度检查后继续执行并触发 `0 / 0`、NaN 分位和后续 panic。
  - 补充 `zero_min_len_returns_error_before_empty_history_division` 回归测试，锁定 `min_len = 0 + 空历史` 返回 `QuantError::InvalidMinHistoryLen`。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：17 个 fundamental 测试、16 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-20 11:47 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：审计修复（配置不变量保护）。
- 涉及文件：
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `crates/quant-engine/tests/common/mod.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `Weight` newtype，用于表达配置权重并在构造期校验 `[0.0, 1.0]`，避免 `cape_weight` 越界破坏加权平均不变量。
  - 为 `FundamentalConfig` 新增 `new(cape_weight, min_history_len)` 构造函数，并将 `min_history_len` 收紧为 `NonZeroUsize`，防止 0 长度配置进入分位计算。
  - 更新基本面测试：将原先锁定非法权重运行期 panic 的用例改为断言构造期返回结构化错误，并补充 `min_history_len = 0` 的拒绝测试。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：17 个 fundamental 测试、15 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-20 11:36 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试注释补充（不改变行为）。
- 涉及文件：
  - `crates/quant-engine/tests/fundamental.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/common/mod.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在 `quant-engine` 各测试入口顶部补充一句模块说明，明确对应第一层基本面、共享分位工具、第二层趋势存根的测试范围。
  - 在 `tests/common/mod.rs` 顶部补充共享夹具说明，区分其与独立集成测试入口的职责。
- 验证：
  - `cargo test -p quant-engine` 通过：32 个集成测试 + 1 个 doc test 全部通过。
  - 相关测试文件无 IDE linter 错误。

### 2026-06-20 11:34 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试结构整理（不改变行为）。
- 涉及文件：
  - `crates/quant-engine/tests/fundamental.rs`（由 `evaluate_fundamental.rs` 重命名）
  - `crates/quant-engine/tests/percentile.rs`（由 `percentile_of.rs` 重命名）
  - `crates/quant-engine/tests/trend.rs`（由 `evaluate_trend.rs` 重命名）
  - `crates/quant-engine/tests/DEFERRED_TESTS.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `quant-engine` 集成测试入口从函数名导向改为模块/层导向，使测试结构与 `src/percentile.rs`、`src/fundamental/mod.rs`、`src/trend/mod.rs` 一一对应。
  - 保留 `tests/common/mod.rs` 作为共享测试夹具与阈值模块，不引入子目录测试 harness，避免 Cargo 集成测试入口复杂化。
  - 更新 `DEFERRED_TESTS.md` 中利率重估测试的落地文件引用为 `fundamental.rs`。
- 验证：
  - `cargo test -p quant-engine` 通过：32 个集成测试 + 1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - 相关测试文件无 IDE linter 错误。

### 2026-06-20 11:25 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：结构重构（不改变行为）。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/src/percentile.rs`（新增）
  - `crates/quant-engine/src/fundamental/mod.rs`（新增）
  - `crates/quant-engine/src/trend/mod.rs`（新增）
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `quant-engine` 从单文件实现拆分为 `percentile`、`fundamental`、`trend` 三个模块，明确共享分位工具、第一层（70% 基本面）与第二层（20% 趋势）的边界。
  - `fundamental` 模块承载 `FundamentalConfig`、`FundamentalSnapshot`、`FundamentalSignal` 与 `evaluate_fundamental`；`trend` 模块承载 `TrendSignal` 与当前中性存根 `evaluate_trend_stub`；`percentile` 模块承载 `percentile_of`。
  - `lib.rs` 保留 crate 文档、模块声明、跨层 `QuantError`，并通过 `pub use` 维持原有根级 API（如 `quant_engine::evaluate_fundamental`、`quant_engine::percentile_of`）兼容。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：32 个集成测试 + 1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - `quant-engine/src` 相关文件无 IDE linter 错误。

### 2026-06-20 11:15 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试重构（不涉及生产实现）。
- 涉及文件：
  - `crates/quant-engine/tests/common/mod.rs`（新增）
  - `crates/quant-engine/tests/evaluate_fundamental.rs`
  - `crates/quant-engine/tests/percentile_of.rs`
  - `crates/quant-engine/tests/evaluate_trend.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `tests/common/mod.rs`，统一测试侧的领域阈值与常用夹具：中性分位、贵/便宜阈值、默认/测试历史长度、CAPE 权重边界、标准历史序列与测试配置 helper。
  - 将 `evaluate_fundamental`、`percentile_of`、`evaluate_trend` 中反复出现的 `0.50`、`0.80`、`0.20`、`10`、`60`、`0.0`、`1.0` 等语义数字改为命名常量或 helper，提高测试意图一致性。
  - 保留用于构造特定分位的局部夹具数字（如当前值、历史序列缩放），避免过度抽象导致测试可读性下降。
- 验证：
  - `cargo fmt` 通过。
  - `cargo test -p quant-engine` 通过：32 个集成测试 + 1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - 相关测试文件无 IDE linter 错误。

### 2026-06-20 10:37 UTC+10

- 执行模型：Claude Opus 4.8。
- 变更类型：测试补全 + 待办测试登记（不涉及实现）。
- 涉及文件：
  - `crates/quant-engine/tests/evaluate_fundamental.rs`
  - `crates/quant-engine/tests/DEFERRED_TESTS.md`（新增）
  - `CHANGE_LOG.md`
- 变更内容：
  - 评估产品专家提出的 5 条金融场景测试，结论：仅「利率重估」当前可写（落在已实现的基本面层内），其余依赖未实现的趋势层 / Decision Engine / `serde` 快照。
  - `evaluate_fundamental`：新增 `rate_repricing_low_erp_pushes_score_expensive_despite_neutral_cape`，覆盖 CAPE 中性但 ERP 极低（利率重估压缩风险补偿）时综合得分仍偏贵的「背离」场景，验证 ERP 倒置语义在两维背离下正确生效。
  - 新增 `tests/DEFERRED_TESTS.md`，登记暂不能写的场景（高估但趋势强、低估但急跌、审计回放），逐条标注依赖模块、前置条件、建议落地位置与断言要点；并说明「数据缺失」一条已被现有测试覆盖。
- 验证：
  - `cargo test -p quant-engine` 通过：32 个集成测试 + 1 个 doc test 全部通过（原 30 个集成）。
  - `crates/quant-engine/tests/evaluate_fundamental.rs` 无 IDE linter 错误。

### 2026-06-20 10:23 UTC+10

- 执行模型：Claude Opus 4.8。
- 变更类型：测试补全（不涉及实现，表征当前未定义行为的边界）。
- 涉及文件：
  - `crates/quant-engine/tests/percentile_of.rs`
  - `crates/quant-engine/tests/evaluate_fundamental.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 针对报告指出的三处未覆盖边界补充表征测试，并在注释中标明「若后续实现层加入校验需改为断言错误」。
  - `±Inf` 当前读数：`percentile_of` 仅拦 NaN，`+Inf` → 分位 `1.0`、`-Inf` → 分位 `0.0`，均不报错；并补 `evaluate_fundamental` 集成层用例（`+Inf` CAPE 与 `-Inf` ERP 合成历史最贵得分 `1.0`）。
  - `cape_weight` 越界：`2.0` 与 `-1.0` 在极值输入下使 `composite` 跌出 `[0,1]`，触发 `Percentile::new(...).expect(...)` panic，以 `#[should_panic]` 锁定当前行为。
  - 历史序列不等长：等长非必需，各指标独立定位（100/60 点均成功）；较短序列低于 `min_history_len` 时明确指向该指标传播 `InsufficientHistory`。
- 验证：
  - `cargo test -p quant-engine` 通过：30 个集成测试 + 1 个 doc test 全部通过（原 24 个集成）。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - 相关测试文件无 IDE linter 错误。

### 2026-06-20 10:14 UTC+10

- 执行模型：Claude Opus 4.8。
- 变更类型：测试补全（不涉及实现）。
- 涉及文件：
  - `crates/quant-engine/tests/percentile_of.rs`
  - `crates/quant-engine/tests/evaluate_fundamental.rs`
  - `crates/quant-engine/tests/evaluate_trend.rs`（新增）
  - `CHANGE_LOG.md`
- 变更内容：
  - 以 `readme.md` 设计约束为依据审计 `quant-engine` 测试覆盖，补齐缺失条目。
  - `percentile_of`：新增并列值（`<=` 语义）、全 NaN 历史降级为 `InsufficientHistory`、有效点数恰等于 `min_len` 边界、`InsufficientHistory` 字段（`indicator`/`required`/`found`）及 `QuantError` 的 `Display` 文案测试。
  - `evaluate_fundamental`：新增默认配置契约（0.5 / 60）、ERP 审计字段未倒置、审计字段如实记录原始分位、`cape_weight` 极值（1.0 纯 CAPE、0.0 纯倒置 ERP）、历史不足与 NaN 当前值的错误传播（对应熔断/降级链）测试。
  - 新增 `evaluate_trend.rs`，覆盖 20% 趋势层存根应返回中性 `0.5`。
- 验证：
  - `cargo test -p quant-engine` 通过：24 个集成测试 + 1 个 doc test 全部通过（原 11 个）。

### 2026-06-20 9:06 UTC+10

- 执行模型：Composer。
- 变更类型：错误信息语言统一。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `QuantError` 的 `Display` 输出统一为英文，与 `PercentileError` 保持一致，避免审计日志中英混排。
  - 将 `percentile_of` 中 `InvalidInput` 的 NaN 错误消息改为英文。
- 验证：
  - `cargo test -p quant-engine` 通过：11 个测试全部通过（含 1 个 doc test）。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - `crates/quant-engine/src/lib.rs` 无 IDE linter 错误。

### 2026-06-19（文档）

- 执行模型：Composer。
- 变更类型：文档。
- 涉及文件：
  - `readme.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `readme.md` 顶部徽章区从 AuroraView 模板链接替换为 IndexLink（`jamesra26/indexlink`）项目链接。
  - 移除不适用的 PyPI、Python、Codecov、CI workflow、pre-commit、ruff、mypy 等徽章。
  - 补充 Rust workspace、crate 结构、CHANGELOG、AGENTS 及 GitHub 社区类徽章；页脚链接改为 Issue Tracker、LICENSE、CHANGELOG。
- 验证：
  - 文档改动，未运行测试。

### 2026-06-19 23:12 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：领域 API 风格一致性。
- 涉及文件：
  - `crates/core-domain/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 `Multiplier` 增加 `Display` 实现，按百分比格式输出倍率，保持与 `Percentile` 的格式化能力对称。
  - 为 `Multiplier` 的 `Display` 行为增加单元测试。
  - 为 `Action` 增加 `Hash` 派生，便于后续作为 `HashMap` 键或用于去重统计。
  - 保持 workspace edition 为 `2021`，未进行 edition 升级。
- 验证：
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - `cargo llvm-cov -p core-domain --summary-only` 通过：Region / Function / Line 覆盖率均为 100.00%。
  - `crates/core-domain/src/lib.rs` 无 IDE linter 错误。

### 2026-06-19（文档）

- 执行模型：Composer。
- 变更类型：文档。
- 涉及文件：
  - `AGENTS.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在 `AGENTS.md` 顶部补充基于 `readme.md` 提炼的项目一句话描述。
- 验证：
  - 文档改动，未运行测试。

### 2026-06-19 23:02 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：Agent 协作规范。
- 涉及文件：
  - `AGENT.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在 `AGENT.md` 写明其他 agent 应遵循的项目规范，包括中文回复、变更日志记录、Rust crate 分层边界、`core-domain` lint 约束、newtype 不变量、测试覆盖率和审计/serde 原则。
- 验证：
  - 文档改动，未运行测试。

### 2026-06-19 23:00 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试覆盖率补强。
- 涉及文件：
  - `crates/core-domain/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 `PercentileError` 的 `Display` 输出增加单元测试，覆盖 `Nan` 与 `OutOfRange` 两种错误格式化。
  - 为 `Percentile` 的 `Display` 输出增加单元测试，覆盖百分比格式化行为。
- 验证：
  - `cargo test -p core-domain` 通过：12 个单元测试全部通过。
  - `cargo llvm-cov -p core-domain --summary-only` 通过：Region / Function / Line 覆盖率均为 100.00%。
  - `crates/core-domain/src/lib.rs` 无 IDE linter 错误。

### 2026-06-19 22:52 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：代码质量 / 文档约束。
- 涉及文件：
  - `crates/core-domain/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在 `core-domain` 增加 crate 级 `#![forbid(unsafe_code)]`，明确基础领域 crate 禁止 unsafe 代码。
  - 在 `core-domain` 增加 crate 级 `#![warn(missing_docs)]`，要求后续公开领域 API 补齐文档。
  - 补齐 `Multiplier::MIN`、`Multiplier::MAX`、`Multiplier::value` 的公开文档。
  - 补齐 `PercentileError::OutOfRange.value` 字段文档，消除新增 `missing_docs` 警告。
- 验证：
  - `cargo check -p core-domain` 通过。
  - `crates/core-domain/src/lib.rs` 无 IDE linter 错误。

### 2026-06-19 22:49 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：路线图 / 审计能力说明。
- 涉及文件：
  - `readme.md`
- 变更内容：
  - 在第 4 阶段路线图中明确后续为纯数据结构补充 feature-gated `serde` 支持。
  - 说明 `serde` 仅提供数据编码/解码能力，不引入 IO。
  - 说明 `Percentile`、`Multiplier` 等带不变量的 newtype 反序列化必须复用构造校验，避免绕过安全边界。
- 验证：
  - 文档改动，未运行测试。
