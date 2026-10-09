# 中英文界面接线修复 Hardness

- Goal：语言切换立即更新当前 V2.1 页面、导航、图表与格式化说明，并保留当前交互内容。
- Current state：基线 `8e3a8c8`；工作区干净。i18next 已初始化，AppHeader/AppSidebar 已订阅，多个新页面仍使用中文常量；日期/金额固定 `zh-CN`；当前测试仅验证资源键一致。
- Desired behavior：zh/en 都能覆盖当前路由的应用文案、状态、辅助提示、日期金额及官方目录展示。刷新保留所选语言，浏览器地区语言规范化；用户命名、备注、原始审计证据及 AI 正文保留原文。
- Architecture constraints：沿用 i18next/react-i18next，React Query 继续拥有服务端缓存；翻译只作用于展示，不改写 API payload、策略 ID/version、金额或冻结的计划时区。图表语言变化重建 option，不重跑回测。
- Explicit non-goals：不新增在线翻译服务、不调用 AI 翻译、不修改后端计算、数据库或策略契约。
- Acceptance criteria：真实点击切换后各路由的标题、按钮、状态、表单标签及图表说明切换；切换后草稿/选中计划不丢失；zh-CN/en-AU 规范化为支持语言；官方策略展示与用户自定义内容边界明确。
- Tests：语言初始化与持久化、各页 zh→en→zh 组件回归、表单状态保持、图表标签切换、资源完整性；Web lint、coverage、build 和 `cargo test -p core-domain`。
- Deliverables：双语资源、页面接线、展示层目录翻译、格式化修复、回归测试、CHANGE_LOG。
- Ownership：主线程负责相关 Web 文件及文档；本轮不并行修改。
- Rollback：撤回本轮前端与资源改动即可；无 migration 或持久数据变更。

## 验收记录（2026-10-10）

- 确认问题存在：顶栏/侧栏接线有效，但新页面未消费翻译资源，且固定中文日期/金额格式。
- 已接线：个人中心、我的计划、策略中心、策略工坊、策略分析、高级实验室，以及当前卡片、图表、研究公式说明和可读错误。
- 以独立 `ui` namespace 保存原文 key；英文资源必须覆盖引用并保持插值集合。源文件扫描测试阻止新增 JSX/属性硬编码中文。官方规则文字仅在展示副本里翻译，规则参数保持原值；未知文字与用户/审计来源不被改写。
- 切换不重挂页面、不改变服务端 query key；验证了草稿内容、筛选类别、图表数值、缓存原文和无额外回测请求。未接入在线翻译服务或自动 AI 调用。
- 实际运行：Web lint、生产 build、coverage（94 项，四项总覆盖率均 ≥90%）与 core-domain 13 项测试通过。构建的大包提醒保留；真实浏览器视觉验收与真实 AI/行情请求未执行。
- 初次修复只修改本地文件；用户随后授权 push。发布使用从 v2 最新 `main`（`91a058d`）创建的 `codex/v2-1-i18n-fix` 独立分支；该基线与原实现基线的代码树一致。

## 提交与推送检查

- Goal：仅发布已验收的中英文修复，不携带本机运行数据。
- Current state：上一条决策/日程 PR #6 已合并；本次改动未提交，均属于本任务。
- Desired behavior / Deliverables：聚焦提交、v2 修复分支与指向 `main` 的独立 PR。
- Architecture constraints / Explicit non-goals：不改计算/存储/API 契约，不重写远端历史，不直接推送 `main`，不合并 PR，不带入 `.env`、密钥或本机缓存。
- Acceptance criteria / Tests：重跑 Web lint、coverage、build、core-domain 测试；检查 diff、提交文件清单及远端分支 SHA，确认 PR 只含本次变更。
- Ownership / Rollback：主线程串行提交；如需撤销，使用独立 revert 提交，不进行 force push 或删除本地数据。
