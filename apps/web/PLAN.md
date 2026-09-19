# IndexLink Web Plan / 前端计划

## V2.1 当前主路径 / Current V2.1 path

Web 的默认入口现为本地优先的消费级外壳：个人中心、策略中心与高级实验室。它先让普通用户理解并采用长期策略；现有 Rust API 页面、审计与 paper-only 能力保留为旧路径和后续接入基础，而不再占据主导航。

The default entry is now a local-first consumer shell: Personal, Strategy Center, and Advanced Lab. It helps ordinary users understand and adopt long-term strategies first. Existing Rust API pages, audits, and paper-only capabilities remain available as legacy routes and future integration foundations rather than the primary navigation.

### 信息架构 / Information architecture

| 页面 / Page | V2.1 用户任务 / V2.1 user task | 当前数据边界 / Data boundary |
| --- | --- | --- |
| 个人中心 / Personal | 从真实计划读取本期待执行建议，只在计划日手工确认完成或跳过，并查看当前建议的 append-only 执行结果 | 使用 React Query 连接 `/investment-plans`、`/investment-plans/:id/decisions` 与 `GET/POST /decisions/:id/manual-executions`；每条 `due` 建议只能确认一次，不自动下单、不覆盖原建议、不显示演示金额 |
| 我的计划 / My Plans | 集中查看、选择、暂停、继续和删除真实计划；可直接建立 Fixed DCA，或接收策略中心传入的官方 Formula 版本并冻结其 70/30 桶配置 | 路由为 `/plans`，位于个人中心二级导航；计划卡展示真实标的、金额、周期、状态、策略与单次上限；Formula 计划只接受目录声明的支持标的 |
| 策略中心 / Strategy Center | 理解并采用 Fixed DCA、MA200 趋势保护与增长/波动平衡三条服务端官方策略；在任意支持的自选标的上运行公平对比 | 路由为 `/strategy-center`，读取 `GET /strategy-catalog`；自适应 70/20/10 暂从普通入口隐藏。`/strategy-analysis` 通过 `POST /strategy-backtests` 读取真实行情、归一化轨迹、专业指标和来源元数据，支持 US/HK/SH/SZ 与 1m/3m/6m/1y/3y/5y/all；不再生成前端演示曲线 |
| 高级实验室 / Advanced Lab | 了解 Docker、Moomoo/OpenD、Qwen、市场数据等可选能力；按需运行旧 MA200 兼容回放 | 配置入口只展示安全边界；旧回放默认不请求且必须手动触发；不保存密钥、不验证账户、不下单 |

## 页面与契约 / Pages and contracts

| 页面 / Page | 已实现 / Implemented | 主要 API / Main API |
| --- | --- | --- |
| 我的计划 / Plans | 先展示已有计划及真实配置，再提供普通用户最小表单；直接进入时建立 100% 核心桶 Fixed DCA，从策略中心进入时读取官方策略版本、支持标的与 70/30 默认配置；建立后立即准备本期真实建议 | `GET /strategy-catalog`, `GET/POST/PATCH/DELETE /investment-plans`, `POST /investment-plans/:id/automatic-decision-preview` |
| 仪表盘 / Dashboard | 自动市场输入、Qwen 情绪、Decision Preview、双桶结果、模拟账户与收益；可选错误在对应卡片内显示 | `/signals/*`, `/market-sentiment/preview`, `/investment-plans/:id/*`, `/paper-*`；旧 MA200 回放已移至高级实验室 |
| 定投标的 / Holdings | V1.1 周期、多个执行日、桶比例、风险模式、滚存、策略版本创建与编辑 | `/investment-plans` |
| 决策 / Decisions | 跨标的记录、计划/动作/日期筛选、分页；详情同时保留原建议证据、所有用户报告的执行流水与审批模式 paper order 确认 | `/decisions`, `/investment-plans/:id/decisions`, `/decisions/:id/manual-executions` |
| 策略 Studio / Strategy Studio | 受限 DSL、验证、准入回测、版本激活 | `/strategies`, `/investment-plans/:id/activate-policy` |
| 策略分析 / Strategy Analysis | 选择自选标的、1–3 条官方策略和七档时间范围；直观图与专业指标共享同一次真实响应 | `POST /strategy-backtests`；服务端数据使用 React Query，草稿筛选与视角使用 Valtio；无行情时明确失败，不回退演示数据 |

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
3. 以真实回测页验证用户能否理解“共同起点、回撤、波动与数据来源”；分享/fork 和复杂外部指标仍等待用户反馈后再进入 M2。
