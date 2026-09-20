# IndexLink Web Plan / 前端计划

> 2026-09-20 状态：M1 的 `Plan → readable Decision → user-reported execution → Audit` 已形成真实前端闭环；官方目录现包含一个 Fixed DCA 基准与 20 个规则家族的 100 个不可变 Formula 预设，真实多市场回测、动态标的建计划与 Formula 历史数据预检均已接入。工程下一步是发布验证，不扩张调度模型或自动交易范围。

## V2.1 当前主路径 / Current V2.1 path

Web 的默认入口是本地优先的消费级外壳：个人中心、我的计划、策略中心、策略分析与高级实验室。普通用户面对“策略、计划、建议和执行记录”；DSL、70/20/10 历史实验、AI、OpenD 状态和 paper broker 留在高级区域。

### 信息架构 / Information architecture

| 页面 / Page | V2.1 用户任务 / V2.1 user task | 当前数据边界 / Data boundary |
| --- | --- | --- |
| 个人中心 / Personal | 从真实计划读取本期建议，理解策略方法、基础预算和下一评估日，手工确认已执行或跳过 | React Query 读取 `/investment-plans`、plan decisions 与 `GET/POST /decisions/:id/manual-executions`；一条 `due` 建议只能确认一次，不自动下单、不覆盖原建议 |
| 我的计划 / My Plans | 查看、选择、暂停、继续或删除已建立计划；常驻“建立新计划”入口统一跳转策略中心，只有带精确 `policy_id` / `policy_version` 返回时才显示所选策略的配置表 | `/plans` 读取 `/strategy-catalog` 与真实 plan API；不再重复平铺策略目录。策略版本和用户参数冻结到计划，并接受 US/HK/SH/SZ 中可解析且满足数据要求的股票/ETF |
| 策略中心 / Strategy Center | 以 Fixed DCA 为基准，按方法家族理解 100 个不可变 Formula 参数预设，搜索、筛选、分析并采用精确版本 | `/strategy-center` 读取 `GET /strategy-catalog`，按 `family` 聚合为 20 个入口、在家族内用实际观察天数选择 `preset`；自适应 70/20/10 暂不进入普通入口，不把受限 DSL 暴露为用户概念 |
| 策略分析 / Strategy Analysis | 在同一自选标的、同一时间范围和同一现金流口径下比较官方策略 | `POST /strategy-backtests` 返回真实行情、归一化轨迹、专业指标和来源元数据；支持 US/HK/SH/SZ 以及 1m/3m/6m/1y/3y/5y/all，无行情时明确失败，不生成演示曲线 |
| 高级实验室 / Advanced Lab | 查看本地能力状态，按需配置 OpenD/Qwen 或运行兼容实验 | 可选能力失败不得影响 Plan、Decision 与 Audit；旧 MA200 回放只可手动触发；不保存密钥、不自动下单 |

## 页面与契约 / Pages and contracts

| 页面 / Page | 已实现或本轮收口 / Implemented or active closeout | 主要 API / Main API |
| --- | --- | --- |
| 我的计划 / Plans | 已有计划列表、常驻策略中心入口、所选策略配置、创建/暂停/继续/删除和创建后建议准备；不在本页重复展示 100 个策略，已补齐 canonical symbol、服务端市场/币种推导和 Formula 创建前历史数据预检 | `GET /strategy-catalog`, `GET/POST/PATCH/DELETE /investment-plans`, `POST /investment-plans/:id/automatic-decision-preview` |
| 个人中心 / Personal | 展示当前计划、本期建议、策略方法、预算边界、下一评估日期和 append-only 执行历史 | `/investment-plans`, `/investment-plans/:id/decisions`, `/decisions/:id/manual-executions` |
| 策略中心 / Strategy Center | 一个 Fixed DCA 基准和 20 家族 × 5 参数档均由服务端目录驱动；前端不平铺 100 张卡，只有可采用的精确版本可以分析或建立计划 | `GET /strategy-catalog` |
| 策略分析 / Strategy Analysis | 选择任意可解析的受支持市场标的、1–3 条官方策略和七档时间范围；直观图与专业指标共享同一次真实响应 | `POST /strategy-backtests`；服务端数据用 React Query，筛选与视角用 Valtio |
| 决策详情 / Decisions | 同时展示不可变原建议、理由和用户报告的执行流水 | `/decisions`, `/decisions/:id/manual-executions` |

## 动态标的收口 / Dynamic instrument closeout

V2.1 不再用 `SPY / VOO / QQQ` 静态数组决定策略能否创建。统一规则是：

1. 服务端解析并规范化 `US.AAPL`、`HK.00700`、`SH.600519`、`SZ.000001` 一类 market-qualified symbol；兼容输入只能在解析成功后转成 canonical symbol。
2. 市场、币种、交易所时区和 instrument type 由解析后的标的及数据集元数据确定，浏览器不得把 `USD` 当成所有计划的真值。
3. Fixed DCA 不依赖行情；只要标的可解析，它仍能在无 OpenD、AI 或市场数据时创建并进入人工执行闭环。
4. Formula 策略创建前由服务端对该标的执行历史数据预检；例如 MA200 必须具备策略声明所需的有效日线窗口。缺失、过期、权限不足或历史不足时明确拒绝，不静默降级成 DCA。
5. “可创建”只表示标的可解析、数据满足确定性 runtime 要求，不表示策略适合该资产，也不构成投资建议。

## 运行可观测性 / Runtime observability

顶栏读取 `/health`、`/ready` 与 `/runtime-status`，区分 API 离线、SQLite 未就绪、OpenD/Qwen/market-data 未配置或不可用，以及 scheduler 最近一次安全计数。状态展示不调用 Qwen、不提交订单；OpenD 行情与 OpenD paper broker 独立配置、独立报告 capability。

## 前端约束 / Frontend rules

- React Router 路由页面按需加载，并设置可恢复的 `errorElement`；不得向用户展示框架默认异常页。
- 中英文翻译键保持完全对齐；Vitest 验证两套 locale 的键集合与非空值。
- 服务端数据通过 React Query；Valtio 只保存当前筛选、选中计划、modal 和图表范围等临时 UI 状态。
- 标的合法性、canonical symbol、币种和 Formula 历史充足性以服务端结果为准；前端只做输入提示和呈现服务端拒绝原因。
- 手工执行确认只对 `due` 建议开放；同一 decision 只能保存一个最终结果，重试复用同一 `event_id`，`409 conflict` 后重新读取。普通界面只提供“已执行”和“跳过”，历史 `partial` 只读兼容。
- 不开放自由策略代码编辑器；DSL 只是受限内部实现。
- 所有券商交互保持 paper-only 与显式确认；V2.1 不自动下单。
- 市场数据、AI、paper broker 或研究查询失败时只影响对应能力，不得升级为核心 Plan、Decision、Audit 不可用。

## V2.1 本轮收口 / Current closeout

- [x] 真实 Plan、Decision、manual execution journal 与 Audit 前端闭环。
- [x] 官方策略目录、20 家族 × 5 不可变 Formula 预设，以及按家族浏览、搜索和精确版本采用入口。
- [x] US/HK/SH/SZ 自选标的真实回测与七档时间范围。
- [x] 个人中心展示策略方法、基础预算和下一评估日。
- [x] 删除策略目录、计划创建和前端中的静态 symbol 白名单。
- [x] 由服务端规范化标的并确定市场/币种；前端不再固定显示 USD。
- [x] Formula 计划创建前完成策略所需历史数据预检，并把失败原因返回给用户。
- [x] 为 US/HK/SH/SZ 的成功、历史不足、provider unavailable 和 Fixed DCA 无行情可用补齐聚焦测试。

## 后续 🔔 / Later

以下内容不阻塞本轮 V2.1 收口，且不能通过只增加一个下拉选项伪装完成：

1. 将“资金周期”与“策略评估/观察频率”拆成独立契约。
2. 引入交易所交易日历、市场时区与节假日顺延/回退规则。
3. 支持每日观察、每周/月多个观察点和候选执行窗口。
4. 无论观察多少次，每个资金周期最多生成一次可执行建议，并用持久化幂等键跨重启保证。
5. 对不同观察频率分别版本化策略研究和回测假设；改变频率不得沿用旧研究结论。

## 验证 / Verification

```bash
pnpm --dir apps/web lint
pnpm --dir apps/web test:coverage
pnpm --dir apps/web build
cargo test -p core-domain
```

完成本轮收口后，再让 3–5 位目标用户完成“建立计划 → 找到建议 → 记录执行 → 找回历史”的任务并形成 Go / Adjust / Stop 证据。分享、fork、自动下单和通用策略 Builder 不属于本轮范围。
