# Agent 规范

## 项目一句话

> **IndexLink** 是面向长期指数投资者的本地优先策略与手动执行工作台：用户选择可理解的策略，建立计划，获得可审计的周期建议，在外部券商自行执行，并把完成、调整或跳过记录在本地。70/20/10 是保留的历史研究策略，不是 V2.1 的产品总定义。

## V2.1 当前唯一主线

```text
Plan → readable Decision → user-reported execution → Audit
```

- 当前先完成 M0 收敛与 M1 最小人工执行闭环，再由 3–5 位目标用户的真实任务反馈决定是否进入策略比较、Simple Builder、扩展数据源与桌面发行。
- Fixed DCA 是强制基线；它在没有市场数据、AI、OpenD、券商或云服务时仍必须可创建计划、生成决策并回看审计。
- 用户始终保有执行权。V2.1 不做无人值守实盘交易，scheduler 不得自动下单。
- 手工执行记录与 paper trading 是两本独立账；手工记录只能标注为 `user-reported`，不得伪装为 broker verified，也不得改写原始 `DecisionRecord`。
- 普通首页不得展示硬编码 MA200 回放或把它描述为当前所选策略表现；70/20/10、DSL、AI、OpenD 与研究实验进入高级区域。

## 产品与架构硬约束

- 保持 modular monolith 与 hexagonal boundary；领域 crate 不包含 IO。
- SQLite 是 V2.1 唯一公开支持的持久化层；不得把未接线的 PostgreSQL/MySQL 草稿重新放回构建依赖图或正式启动路径。
- 行情、broker 与 AI 必须通过显式 port 独立配置、独立初始化、独立报告 capability；任一可选能力失败不得阻止 Plan、Decision 与 Audit。
- 配置过但初始化失败的 broker 必须显示为 unavailable，不得静默退回成看似真实连接的 Mock。
- 不引入任意用户代码执行，不执行 Python 或 JavaScript 策略；DSL 只是受限内部实现，不是普通用户概念。
- `DecisionRecord` 是不可变审计证据；执行留痕采用 append-only 事件并保存 planned、actual 与输入快照。
- 市场数据供应商 SDK 不得泄漏到领域层。策略、Today 与回测只读取本地 `PriceHistoryProvider`；远端来源仅用于显式导入或更新。
- 市场数据至少保留 provider、market、instrument type、currency、timezone、adjustment type、as-of、dataset version 与 checksum；缺失、过期或冲突数据必须显式失败，不得静默前向填充或混拼。
- 回测必须复用同一生产确定性 runtime，并保证相同标的、现金流、成本、执行时点、因果 cutoff 与 dataset version；Fixed DCA 始终是同口径对照。
- 前端不得生成或硬编码对外表示为真实的收益、回撤、波动率或策略结论；服务端数据由 React Query 管理，Valtio 只保存临时 UI 状态。
- V2.1 不新增云账户、云同步、通用 Portfolio、任意策略代码、IBKR/QMT/Futu/moomoo 自动下单、高频交易、自动优化或未经明确授权的数据再分发。

## 任务与合并 Hardness

- 每个实现任务开始前必须写明 Goal、Current state、Desired behavior、Architecture constraints、Explicit non-goals、Acceptance criteria、Tests 与 Deliverables。
- 开始前确认基线 commit、相关抽象、调用者、文件 ownership 和回滚方式；不得用“大重构”代替聚焦修复。
- 有接口依赖的任务串行；可并行任务必须使用独立 worktree，多个 Agent 不得同时在同一个 Local 工作目录修改文件。
- 每个 worktree 保持 PR-sized；逐个合并，每次 merge 后重跑完整相关检查。金额、migration、backtest 与桌面安全需要独立 reviewer。
- 不得声称未实际运行的构建或测试已经通过；环境不可运行时必须明确登记为未验证。
- 当前收口门槛与执行顺序见 `docs/plans/v2_1_closeout_hardness.md`；它和本文件优先于旧的百分比完成度、固定工期或并行全路线图。

## 必须完成的规范

- 始终使用中文回复。
- 修改代码或文档后，在 `CHANGE_LOG.md` 记录时间、执行模型、变更类型、涉及文件、变更内容和验证结果。
- 尊重分层边界
- 新增公开 API 必须补齐文档。
- 带不变量的 newtype（如 `Percentile`、`Multiplier`）必须通过构造函数或 `TryFrom` 保持校验，不能绕过安全边界。
- 为行为变更补充聚焦测试；改动后至少运行 `cargo test -p core-domain`，必要时运行 `cargo llvm-cov -p core-domain --summary-only`。
- 审计/存储相关能力应优先保存输入快照而非只保存结论；后续 `serde` 支持应使用 feature 开关，且反序列化必须复用不变量校验。

## 外部参考

- 仓库: https://github.com/jamesra26/indexlink
- CHANGELOG: `./CHANGE_LOG.md`

### 前端部分

- shadcn: https://ui.shadcn.com/docs/components
- 前端规划：apps\web\PLAN.md
- 使用vite + react + tailwindcss，配合shadcn的组件库进行快速构建。
- 统一使用 pnpm 管理依赖
- 路由统一使用 react-router；来自 Rust API 的服务端数据、缓存、loading/error/retry、mutation 后失效刷新统一使用 @tanstack/react-query；浏览器本地 UI 状态（当前选中的 plan、筛选条件、modal 开关、图表显示范围、临时交互状态）使用 valtio，不要用 valtio 存长期服务端数据。页面样式使用 Tailwind CSS v4 和 @tailwindcss/vite；通用组件优先使用 shadcn 体系，样式组合用 clsx + tailwind-merge，变体组件用 class-variance-authority，动画辅助用 tw-animate-css，图标使用 lucide-react。图表底层使用 recharts，shadcn chart 只作为样式/容器封装。i18n做多语言。颜色注意统一使用index.css的。代码质量使用 eslint + typescript-eslint + react-hooks / react-refresh 规则，构建脚本保持 tsc -b && vite build。
