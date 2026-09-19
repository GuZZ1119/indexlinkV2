# Push 5：运行时行情依赖解耦

## Goal

让线上计划决策按策略声明读取最小数据依赖：Fixed DCA 不读取行情；Formula 只读取自身所需的规范化历史日线；旧 Core/Opportunity 兼容策略才读取 CAPE、国债、VIX 与技术信号整包。行情不可用必须成为显式失败与 capability 状态，不能生成或伪装成 `waiting` 决策。

## Current state

- `fixed_dca@1` 已有提前返回路径，不读取 `MarketSignalProvider`。
- 所有非 Fixed DCA 策略随后都会先调用 `market_signal_input`，因此只依赖价格的 Formula 仍被 CAPE、国债或 VIX 故障阻塞。
- 产品回测已有独立 `HistoricalPriceProvider`，但线上 Formula 尚未复用该规范化日线 port。
- `/runtime-status` 只公开旧 `market_data` 能力，没有单独公开 Formula/回测实际使用的历史价格 provider 状态。
- Scheduler 会把输入失败计入 `unavailable` 且不保存决策，但需要测试锁定“到期 Formula 数据失败不产生 waiting 记录”的语义。

## Desired behavior

- Fixed DCA 自动预览和 Scheduler 在未配置任何行情 provider 时仍可生成到期决策。
- Formula 根据 `StrategySpec::required_close_observations` 请求一个有界历史日线窗口，并复用 `DslEvidence` 的确定性指标计算。
- 当前官方 Formula 不访问 `MarketSignalProvider`、CAPE、国债或 VIX。
- 需要外部 VIX 的自定义 Formula 在尚无独立外部指标 port 时显式失败，不用零值冒充真实证据。
- 历史价格 provider 的 `not_configured`、`configured`、`unavailable` 状态通过 `/runtime-status` 单独公开。
- 到期 Formula 的数据失败返回 `503`，Scheduler 计入 `unavailable`，不写入 `waiting` 或其他 DecisionRecord。

## Architecture constraints

- 策略和 Today 只通过 provider-neutral `HistoricalPriceProvider` 读取规范化日线；供应商协议不进入 API 决策逻辑。
- 复权能力由 `HistoricalPriceProvider::preferred_adjustment` 声明；Alpaca 美股使用 `all`，OpenD 各市场使用前复权，API 不按 provider 名称猜测协议枚举。
- 不修改 `DecisionRecord`，成功决策继续保存输入来源和数据集版本/checksum。
- 不让可选行情失败阻止 Fixed DCA、SQLite 或 API 启动。
- 不静默填补价格、不混拼 provider、不构造伪 VIX。

## Explicit non-goals

- 不增加 plan timezone 或 migration。
- 不修改前端，也不新增自动下单。
- 不删除旧 Core/Opportunity 兼容策略的宏观整包。
- 不实现 VIX 的独立 canonical store。
- 不修改回测计算或行情供应商 adapter。

## Acceptance criteria

1. Fixed DCA 在两个行情 port 都缺失时仍可完成 automatic preview。
2. 官方 Formula 只调用 `HistoricalPriceProvider::fetch_history`，即使旧 `MarketSignalProvider` 会失败也能完成。
3. Formula 历史行情缺失/失败时 automatic preview 返回 `503`，不保存 DecisionRecord；Scheduler summary 增加 `unavailable` 而不是 `created`。
4. Formula 审计来源包含 provider、dataset version、checksum、adjustment 与证据截止日。
5. `/runtime-status` 单独返回 `historical_prices` capability 状态。

## Tests

- `cargo test -p indexlink-api --test decision_preview`
- `cargo test -p indexlink-api --test health`
- `cargo test -p indexlink-api`
- `cargo test -p core-domain`

## Deliverables and rollback

- Ownership：`crates/api/src/routes/decision_preview.rs`、`crates/api/src/routes/runtime_status.rs`、`crates/api/src/state.rs`、对应 API 集成测试与 API 文档。
- 回滚：逐文件撤销本 Push，不涉及 schema 或已有数据迁移。
- `CHANGE_LOG.md` 由主任务在合并五个 Push 后统一记录。
