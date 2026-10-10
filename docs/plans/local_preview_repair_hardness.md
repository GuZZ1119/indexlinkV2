# 本地预览回归修复 Hardness

- Goal：修复本地预览中的页面直达/刷新、过期建议日期误导与已知前端依赖漏洞，提交到 V2 独立分支。
- Current state：本地基线 `d4bf2ce`，工作区干净；其 i18n PR 已合并。冒烟测试确认 Vite `/personal` 返回 404、`/decisions/:id` 返回 JSON；首页把历史 `due` 建议描述为今天执行；锁文件仍有 `source-map-js 1.2.1`，Web CI 未执行审计。
- 发布基线：从已合并的 v2 `main`（`c68691f`）创建 `codex/v2-1-local-preview-fixes`；其代码树与 `d4bf2ce` 一致。
- Desired behavior：浏览器 HTML 页面请求返回 SPA，JSON API 调用保留原路径；按计划时区区分今天保存、历史保存和日期无法核验的原建议，不更新原始证据；锁定修复依赖并在 CI 拒绝已知高危漏洞。保存日不推断为执行日：scheduler 补生成的历史建议可能今天才入库，现有审计契约未直接暴露完整 scheduled date。
- Architecture constraints：React Query 继续管理服务端记录；日期文案只做展示派生；DecisionRecord、manual journal、计划金额/时区、Rust API 与确定性 runtime 不变。API 数据请求显式接受 JSON，浏览器文档请求才进入 SPA。
- Explicit non-goals：不改资金结算、migration、broker/AI 权限或回测公式；不删除本地研究数据、不擅自宣告取得数据再分发授权；不伪造真实用户验收；不创建正式 tag/Release；不合并 PR 或直接 push main。
- Acceptance criteria：主页/审计详情直达与刷新正常；API 仍返回 JSON；同日、跨时区、旧建议及非法日期文案有回归；旧待办仍可记录且不被删除/改写；中英文同步；生产依赖审计无已知高危项；只发布任务文件、不含本地凭据和运行数据。
- Tests：Vite 代理聚焦测试与实际 HTTP HTML/JSON 验证；个人中心日期与中英文组件测试；Web lint、全量覆盖率门禁、build、依赖 audit；`cargo test -p core-domain --locked`、`git diff --check`；真实浏览器首页/审计刷新验收。
- Deliverables：代理边界修复、日期展示与双语资源、依赖锁文件和 CI 审计、测试、CHANGE_LOG、修复分支 push/PR。
- Ownership：主线程串行负责 Web 配置/页面/测试、CI 与发布文档；不并行改动 Local 目录。
- Callers / abstractions：`vite.config.ts` → Vite dev/preview proxy；`api/queries.ts` 的 `Accept: application/json` → Rust API；PersonalPage → AdviceCard → 不可变 DecisionRecord 与手工事件；i18next UI namespace。
- Rollback：独立 revert 本次提交；无数据库 migration、原记录更新或数据删除。

## 发布边界

研究原始数据和派生校准快照仍被源码构建/测试依赖。本次只核对并保持 [DATA_LICENSES.md](../../DATA_LICENSES.md) 的失败关闭门槛，不以删除文档或忽略文件的方式假装修复许可证。移出公开源码需要另一个聚焦任务及独立回测 reviewer；3–5 位目标用户任务证据也仍未完成。

## 实际验收（2026-10-10）

- 冻结安装、Web lint、全量 117 项测试及四项 >=90% 覆盖率、生产构建通过；`cargo test -p core-domain --locked` 13 项通过。
- 全量及生产依赖审计均无已知公告；增加全量审计后暴露的开发依赖公告也升级修复，未引入 allowlist。Node 下限与实际依赖对齐为 22.19.0。
- dev 与 preview 各七个只读 HTTP 请求：页面导航返回 HTML，决策与手工日志返回 JSON，全部 200。真实浏览器首页/详情刷新、中英往返、旧待办文案通过；没有写入用户执行记录、调用付费 AI 或下单。
- Rust workspace 与 cargo audit 本轮未重跑；本轮不改 Rust。既有大包提示仍保留。远端 CI 结果须由 PR 检查确认，不能用本机通过代替。
