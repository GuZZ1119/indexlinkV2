# IndexLink Web Plan / 前端计划

## V2.1 当前主路径 / Current V2.1 path

Web 的默认入口现为本地优先的消费级外壳：个人中心、策略中心与高级实验室。它先让普通用户理解并采用长期策略；现有 Rust API 页面、审计与 paper-only 能力保留为旧路径和后续接入基础，而不再占据主导航。

The default entry is now a local-first consumer shell: Personal, Strategy Center, and Advanced Lab. It helps ordinary users understand and adopt long-term strategies first. Existing Rust API pages, audits, and paper-only capabilities remain available as legacy routes and future integration foundations rather than the primary navigation.

### 信息架构 / Information architecture

| 页面 / Page | V2.1 用户任务 / V2.1 user task | 当前数据边界 / Data boundary |
| --- | --- | --- |
| 个人中心 / Personal | 从真实计划读取本期待执行建议，只在计划日手工确认完成或跳过，并查看当前建议的 append-only 执行结果 | 使用 React Query 连接 `/investment-plans`、`/investment-plans/:id/decisions` 与 `GET/POST /decisions/:id/manual-executions`；每条 `due` 建议只能确认一次，不自动下单、不覆盖原建议、不显示演示金额 |
| 我的计划 / My Plans | 集中查看、选择、暂停、继续和删除真实计划，并在同一页面建立最小 Fixed DCA 计划 | 路由为 `/plans`，位于个人中心二级导航；计划卡展示真实标的、金额、周期、状态、策略与单次上限 |
| 策略中心 / Strategy Center | 理解、选用、比较固定定投与 70/20/10 等精选策略 | 路由为 `/strategy-center`，避免与 Rust `/strategies` API 前缀冲突；含 `/strategy-analysis` 二级导航：直观视角按统一起点 100 比较本地示例策略，专业研究视角则按需读取后端已保存 DSL 的固定样本准入指标；公开分享/fork 与通用版本化回测等待后续契约 |
| 高级实验室 / Advanced Lab | 了解 Docker、Moomoo/OpenD、Qwen、市场数据等可选能力；按需运行旧 MA200 兼容回放 | 配置入口只展示安全边界；旧回放默认不请求且必须手动触发；不保存密钥、不验证账户、不下单 |

## 页面与契约 / Pages and contracts

| 页面 / Page | 已实现 / Implemented | 主要 API / Main API |
| --- | --- | --- |
| 我的计划 / Plans | 先展示已有计划及真实配置，再提供普通用户最小 Fixed DCA 表单；只要求标的、金额与周期，策略版本、100% 核心桶、风险与机会资金默认值由产品固定；建立后立即准备本期真实建议 | `GET/POST/PATCH/DELETE /investment-plans`, `POST /investment-plans/:id/automatic-decision-preview` |
| 仪表盘 / Dashboard | 自动市场输入、Qwen 情绪、Decision Preview、双桶结果、模拟账户与收益；可选错误在对应卡片内显示 | `/signals/*`, `/market-sentiment/preview`, `/investment-plans/:id/*`, `/paper-*`；旧 MA200 回放已移至高级实验室 |
| 定投标的 / Holdings | V1.1 周期、多个执行日、桶比例、风险模式、滚存、策略版本创建与编辑 | `/investment-plans` |
| 决策 / Decisions | 跨标的记录、计划/动作/日期筛选、分页；详情同时保留原建议证据、所有用户报告的执行流水与审批模式 paper order 确认 | `/decisions`, `/investment-plans/:id/decisions`, `/decisions/:id/manual-executions` |
| 策略 Studio / Strategy Studio | 受限 DSL、验证、准入回测、版本激活 | `/strategies`, `/investment-plans/:id/activate-policy` |

## 运行可观测性 / Runtime observability

顶栏读取 `/health`、`/ready` 与 `/runtime-status`，清楚区分 API 离线、SQLite 未就绪、OpenD/Qwen/市场数据未配置，以及 scheduler 最近一次安全计数。状态展示绝不调用 Qwen 或提交订单。

The top status strip reads `/health`, `/ready` and `/runtime-status`. It distinguishes an offline API, unavailable SQLite, optional OpenD/Qwen/market-data configuration, and safe scheduler counters without invoking Qwen or placing an order.

## 前端约束 / Frontend rules

- React Router 路由页面按需加载，并设置可恢复的 `errorElement`；不得向用户展示框架默认异常页。
- 中英文翻译键必须完全对齐；Vitest 会验证两套 locale 的键集合与非空值。
- 服务端数据必须通过 React Query；手动刷新使用 `refetch`，仍写入同一 query cache。
- 手工执行确认只对 `due` 建议开放，并且同一 decision 只能保存一个最终结果；客户端重试必须复用同一个 `event_id`，任何第二次结果都按 `409 conflict` 重新读取，不能静默重复追加。普通界面只提供“已执行”和“跳过”；历史 `partial` 仅只读兼容。
- 不开放自由策略代码编辑器；策略 Studio 只提交后端白名单 DSL。
- 所有交易交互保持 paper-only；审批模式必须确认已有决策存证，不能重新计算后下单。
- 普通首页不得预取旧 MA200 回放；兼容回放只能在高级实验室由用户明确触发。
- 市场、AI、组合、模拟收益等可选查询失败时，只影响对应卡片，不得升级为页面级核心错误。

## 验证 / Verification

```bash
pnpm --dir apps/web lint
pnpm --dir apps/web test:coverage
pnpm --dir apps/web build
```

## 后续 / Next

1. 让 3–5 位目标用户完成“建立计划 → 找到建议 → 记录执行 → 找回历史”的完整任务并记录证据。
2. 根据阻塞点形成 Go / Adjust / Stop 决策，不在测试前扩展策略。
3. 只有用户反馈支持后，才进入 M2 数据层、第二条策略与公平比较。
