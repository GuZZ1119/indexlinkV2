# IndexLink Web Plan / 前端计划

## V2.1 当前主路径 / Current V2.1 path

Web 的默认入口现为本地优先的消费级外壳：个人中心、策略中心与高级实验室。它先让普通用户理解并采用长期策略；现有 Rust API 页面、审计与 paper-only 能力保留为旧路径和后续接入基础，而不再占据主导航。

The default entry is now a local-first consumer shell: Personal, Strategy Center, and Advanced Lab. It helps ordinary users understand and adopt long-term strategies first. Existing Rust API pages, audits, and paper-only capabilities remain available as legacy routes and future integration foundations rather than the primary navigation.

### 信息架构 / Information architecture

| 页面 / Page | V2.1 用户任务 / V2.1 user task | 当前数据边界 / Data boundary |
| --- | --- | --- |
| 个人中心 / Personal | 查看正在坚持的策略、下一次行动与近期变化 | 本地演示状态；不伪装为已连接收益或订单数据 |
| 策略中心 / Strategy Center | 理解、选用、比较固定定投与 70/20/10 等精选策略 | 路由为 `/strategy-center`，避免与 Rust `/strategies` API 前缀冲突；含 `/strategy-analysis` 二级导航：直观视角按统一起点 100 比较本地示例策略，专业研究视角则按需读取后端已保存 DSL 的固定样本准入指标；公开分享/fork 与通用版本化回测等待后续契约 |
| 高级实验室 / Advanced Lab | 了解 Docker、Moomoo/OpenD、Qwen、市场数据等可选能力；按需运行旧 MA200 兼容回放 | 配置入口只展示安全边界；旧回放默认不请求且必须手动触发；不保存密钥、不验证账户、不下单 |

## 页面与契约 / Pages and contracts

| 页面 / Page | 已实现 / Implemented | 主要 API / Main API |
| --- | --- | --- |
| 仪表盘 / Dashboard | 自动市场输入、Qwen 情绪、Decision Preview、双桶结果、模拟账户与收益；可选错误在对应卡片内显示 | `/signals/*`, `/market-sentiment/preview`, `/investment-plans/:id/*`, `/paper-*`；旧 MA200 回放已移至高级实验室 |
| 定投标的 / Holdings | V1.1 周期、多个执行日、桶比例、风险模式、滚存、策略版本创建与编辑 | `/investment-plans` |
| 决策 / Decisions | 跨标的记录、计划/动作/日期筛选、分页、审计详情与审批模式 paper order 确认 | `/decisions`, `/investment-plans/:id/decisions` |
| 策略 Studio / Strategy Studio | 受限 DSL、验证、准入回测、版本激活 | `/strategies`, `/investment-plans/:id/activate-policy` |

## 运行可观测性 / Runtime observability

顶栏读取 `/health`、`/ready` 与 `/runtime-status`，清楚区分 API 离线、SQLite 未就绪、OpenD/Qwen/市场数据未配置，以及 scheduler 最近一次安全计数。状态展示绝不调用 Qwen 或提交订单。

The top status strip reads `/health`, `/ready` and `/runtime-status`. It distinguishes an offline API, unavailable SQLite, optional OpenD/Qwen/market-data configuration, and safe scheduler counters without invoking Qwen or placing an order.

## 前端约束 / Frontend rules

- React Router 路由页面按需加载，并设置可恢复的 `errorElement`；不得向用户展示框架默认异常页。
- 中英文翻译键必须完全对齐；Vitest 会验证两套 locale 的键集合与非空值。
- 服务端数据必须通过 React Query；手动刷新使用 `refetch`，仍写入同一 query cache。
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

1. 解耦服务端 market-data 与 paper broker 初始化，使可选能力失败不阻止 SQLite 核心启动。
2. 将 PostgreSQL 从默认依赖图移为显式 opt-in feature。
3. 实现 append-only 手工执行日志，再把 Fixed DCA 的真实 Plan、Decision 与 Today 接入普通首页。
