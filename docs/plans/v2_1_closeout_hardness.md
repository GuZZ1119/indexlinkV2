# IndexLink V2.1 收口 Hardness 与发布门槛

> 状态：**工程收口完成，等待真实用户任务验证**
>
> 最后核对：2026-09-23
> 当前产品事实以根 README、公开 API、自动化测试和本文件为准。

## 1. 唯一产品定义

IndexLink V2.1 是单用户、本地优先、人工执行的长期策略计划工具。它只承诺一条闭环：

```text
选择/建立受限策略 → 真实回测与数据预检 → 建立本地计划
→ 到期建议 → 用户在外部券商操作 → 手工记录结果 → 回看审计
```

它不是多用户 SaaS、社交策略社区、任意代码量化平台或自动交易终端。70/20/10 自适应模型是历史研究，不是 V2.1 默认产品。

## 2. 不可破坏的不变量

### H1 本地优先

- API 默认绑定 `127.0.0.1`；Docker host 端口也只发布到 loopback。
- SQLite 是唯一公开、受支持的存储后端。
- 无 AI、OpenD、broker、外部 Key 或 Docker 时，Fixed DCA 的 Plan、Decision、manual journal 与 Audit 仍可使用。
- 当前没有认证；任何局域网或公网暴露都不属于受支持场景。

### H2 用户拥有执行权

- scheduler 只能幂等生成建议，不能自动下单。
- 普通路径只记录用户报告的 `executed/skipped`；历史 `partial` 只读兼容。
- OpenD paper broker 是独立、显式的高级实验能力，不得与 manual journal 混写。
- AI 没有保存策略、激活策略、修改计划或提交订单的权限。

### H3 审计不可变

- `DecisionRecord`、策略 ID/version、输入快照和原始建议不可被后续操作覆盖。
- 手工执行为 append-only 事件；同一 `due` decision 最多一个最终结果。
- 重试通过 event/idempotency key 保持幂等；服务重启后约束仍由 SQLite 保持。

### H4 数据与回测真实

- 新的任意标的回测只能使用真实 provider 数据或可复核 cache；缺失时明确失败。
- 同一比较共享标的、时间范围、外部投入、现金、成本、成交时点和因果 cutoff。
- Fixed DCA 始终是匹配现金流的基线。
- 归一化净值起点 `100` 不是股价或账户余额；UI 必须解释计算口径。
- Formula 只读取观察日及以前的证据；预热不足、数据过期或授权失败必须拒绝，不得补造。

### H5 策略安全

- 不执行用户 Python、JavaScript、Pine Script 或第三方仓库代码。
- 普通用户只面对策略、计划、规则和行动；AST/DSL 是内部受限实现。
- 个人策略最多三条优先规则、每条三个白名单条件，只能调整机会额度，不能取消核心投入。
- 计划冻结精确策略版本；目录更新不能静默改变已存在的计划。

### H6 可选能力隔离

- AI、OpenD 行情、paper broker 和新闻实验分别报告 capability。
- 一个可选能力失败不得让 SQLite 核心不可用。
- 真实连接失败不能回退成 Mock 并伪装成功。
- 页面输入的 AI Key/OpenD 会话配置只留在当前 Rust 进程，重启即失效。

### H7 界面不伪造完成度

- 真实卡片、策略可用性、收益、回撤、买点和专业指标必须来自后端契约。
- React Query 保存服务端状态；Valtio 只保存筛选、选择、modal 和图表范围等临时 UI 状态。
- 不存在真实结果时必须显示不可用，而不是演示金额、静态曲线或浏览器伪完成状态。

## 3. 当前事实

| 能力 | 当前状态 | V2.1 判断 |
| --- | --- | --- |
| Plan → Decision → manual execution → Audit | SQLite/API/Web 完整闭环；同一到期建议只能确认一次 | 已完成 |
| 策略目录 | Fixed DCA + 20 家族 × 5 参数的 100 个不可变 Formula 版本 | 已完成；数量不是收益排行 |
| 策略工坊 | 白名单表单、个人不可变版本、目录个人标签、真实回测/计划运行时 | 已完成 |
| 真实回测 | US/HK/SH/SZ、七档时间范围、OpenD 日线/cache、净值/买点/专业指标 | 已完成；无数据明确失败 |
| 动态标的计划 | 服务端 canonical symbol/market/currency；Formula 创建前历史预检 | 已完成 |
| AI 辅助 | QwenCloud、百炼、GPT、Claude、DeepSeek；仅手动草案/解释/摘要 | 已完成；不参与计算或执行 |
| OpenD | 行情与 paper broker 独立；Lab 会话只授予只读行情 | 已完成 |
| 本地安全 | loopback 默认、RSS 上限、依赖升级、凭据不落盘/不回显 | 已完成本地边界 |
| 用户任务验证 | 尚无 3–5 位目标用户的结构化证据 | 发布前主要缺口 |
| 自动实盘、多用户、云同步 | 不存在 | 明确不做 |

## 4. 发布 Gate

### Gate A — 核心闭环

通过。必须持续满足：

1. Fixed DCA 无外部依赖可建立计划；
2. 到期建议幂等；
3. 用户只能最终确认一次；
4. 原建议与执行结果分开保存；
5. 删除/暂停计划不篡改历史。

### Gate B — 策略与回测

通过。必须持续满足：

1. 目录和前端以服务端精确版本为准；
2. Formula 计划创建前检查数据窗口；
3. 真实回测复用确定性 runtime；
4. 同图比较共享标的、日期和现金流；
5. 专业指标与公式代入值来自同一响应。

### Gate C — 本地安全与开源收口

通过本机边界：

- loopback 默认已落实；
- 前端生产依赖审计为 0；
- Rust 运行路径漏洞已修复，保留一个 SQLx 锁文件中未链接的 `rsa` 公告并公开说明；
- 旧页面、旧云脚本、PostgreSQL 草稿和过期 push 文档不再进入公开上游；
- README、Security、第三方引用和 changelog 与当前代码一致。

### Gate D — 目标用户验证

尚未通过。发布 `v2.1.0` 前让 3–5 位目标用户分别完成：

1. 找到并理解一个策略；
2. 对自选标的运行真实回测；
3. 建立计划并找到本期/下期安排；
4. 记录“已执行”或“跳过”；
5. 找回并复述历史记录；
6. 说明最困惑的概念和是否愿意下周期继续使用。

记录任务成功率、阻塞点、误解和 Go / Adjust / Stop 结论。没有这组证据，不用增加更多策略、券商或自动化。

## 5. 发布前验证

每个行为变更必须有聚焦测试并更新 `CHANGE_LOG.md`。候选提交至少运行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
pnpm --dir apps/web lint
pnpm --dir apps/web test:coverage
pnpm --dir apps/web build
pnpm --dir apps/web audit --prod
git diff --check
```

前端 statements、branches、functions、lines 均不得低于 90%。真实网络/OpenD 测试可以保持 `ignored`，但必须在发布记录中说明未自动运行的原因。

## 6. 后续而非 V2.1 阻塞项

- 将资金周期与策略观察频率拆成独立契约；
- 引入交易所日历、时区、节假日顺延和周期级幂等；
- 更完整的小屏布局与图表触控优化；
- 数据许可/再分发专项复核；
- 单平台桌面壳、app-data、sidecar 生命周期和签名发布；
- 只有用户证据支持时再评估策略分享、fork、更多 provider 或更复杂的多资产策略。

IBKR/QMT/Futu/Moomoo 自动实盘、云账户、公开策略社区和任意代码执行不属于 V2.1。
