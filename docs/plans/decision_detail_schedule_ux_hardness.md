# Decision 详情与下一评估日收口 Hardness

> 基线：`indexlinkv2/main` @ `2d48a464ee820e7945ea97304a0103e4bc04e229`
>
> 范围：只修复已有 Decision 详情展示、paper capability 门控和个人中心下一评估日；有限期限日程投影与手工执行结算仅登记后续计划，不在本轮实现。

## Goal

让普通用户能读懂不可变原建议；未启用 paper broker 时不出现可执行的模拟下单入口；本期已经记录完成或跳过后，个人中心显示严格晚于本期计划日的下一评估日。

## Current state

- Decision 详情直接输出后端英文审计摘要和内部 snapshot 键值，普通视图混入 policy id、trigger、bucket 与未格式化 Decimal。
- paper broker 关闭时，详情仍显示“确认模拟下单”，点击后只得到通用 `service is unavailable`。
- `nextScheduledDate` 只使用当前时间和重复规则；本期完成后，同一天仍把今天显示成“下一评估”。
- 手工执行 journal 只保存用户报告事实，不重算策略、不改写 DecisionRecord，也不参与机会现金结算。

## Desired behavior

1. 默认 Decision 详情只显示行动、计划金额、计划时间、策略方法、理由、限制与手工执行历史；内部英文摘要和原始快照保留在明确折叠的“技术审计信息”中。
2. 金额使用交易币种格式化，普通层不输出内部枚举、policy id 或任意 JSON 字段遍历结果。
3. 只有 runtime status 明确报告 `paper_broker=configured` 时才显示模拟下单操作；未配置或不可用时显示安静、可理解的高级能力说明，不发起请求。
4. 已存在最终 manual event 时，下一评估日以该 Decision 的计划日为排除边界，必须严格晚于该日；未完成时仍可把今天作为本期日。
5. 页面刷新和服务重启后，结果继续由 SQLite journal 和计划规则还原，不依赖浏览器会话状态。

## Architecture constraints

- `DecisionRecord` 不可变；不得改写历史 summary 或 snapshot。
- manual journal 与 paper broker 分账；不得把用户报告伪装为 broker verified。
- capability 通过 `/runtime-status` 与 React Query 读取，不在浏览器猜测环境变量。
- 日期按计划冻结的 IANA timezone 计算，不按浏览器本地时区猜测。
- 本轮不新增 API、不修改 scheduler、不生成未来 DecisionRecord。

## Explicit non-goals

- 不启用或配置 OpenD paper broker，不提交任何订单。
- 不实现交易所节假日顺延、完整未来日历或未来 Formula 精确金额。
- 不让 manual actual amount 直接改写机会现金或周期预算。
- 不删除历史英文审计数据，不迁移既有 DecisionRecord。

## Acceptance criteria

- 普通详情不再直接展示长英文 summary、原始 snapshot 键值和长尾 Decimal。
- 技术审计信息仍可主动展开查看，历史证据无损。
- `paper_broker` 为 `not_configured`/`unavailable` 时没有可点击的模拟下单按钮；`configured` 时保留原有审批门控。
- 本期已完成或跳过后，“下一评估”严格晚于本期计划日；未记录时本期日仍正确显示。
- 同一建议仍最多记录一次，原建议与执行历史仍可找回。

## Tests

- Decision detail：普通/技术分层、金额格式化、三态 capability、配置时审批入口。
- Personal：月度与周度、跨月/跨周、计划时区、未完成包含今天、完成后排除今天。
- `pnpm --dir apps/web lint`
- `pnpm --dir apps/web test`
- `pnpm --dir apps/web test:coverage`
- `pnpm --dir apps/web build`
- `cargo test -p core-domain --locked`
- `git diff --check`

## Deliverables

1. 一个聚焦前端行为提交：Decision 分层、capability 门控、完成后的下一评估日。
2. 一个计划文档提交：登记有限期限日程投影 API/UI 与 manual execution settlement 闭环。
3. `CHANGE_LOG.md` 记录模型、文件、行为与实际验证结果。

## Ownership 与回滚

- Web 实现 worktree 只修改 `apps/web/src/pages/{decisions,personal}` 及相应测试。
- 文档 worktree 只修改 `docs/plans`，不触碰 Web 文件。
- 主 worktree 负责合并、`CHANGE_LOG.md`、完整验证、push 与 PR。
- 回滚按两个提交逆序撤销；两项不涉及 migration 或领域数据，回滚不会破坏 SQLite 历史。
