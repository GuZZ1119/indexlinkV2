# Push 1 — 个人策略进入统一目录

## Goal

让 `GET /strategy-catalog` 同时投影官方策略和本机已保存的个人受限 Formula 策略，使计划创建、分析和后续 Builder 都只依赖一个精确、版本化的策略身份入口。

## Current state

- 基线：`7efb7ac`，分支 `codex/v2-1-06-personal-plan-ux`。
- 官方目录由 `official_strategies` 生成，包含 `fixed_dca@1` 与 100 个官方 Formula 预设。
- `POST /strategies` 已通过领域构造器校验并将 `(policy_id, policy_version)` 作为 SQLite 主键保存；重复版本不可覆盖。
- `GET /strategies` 能列出个人版本，但普通目录、计划选择与分析入口看不到这些版本。

## Desired behavior

- `GET /strategy-catalog` 保留全部 101 个官方条目，并追加本机已保存、非官方保留 ID 的个人策略版本。
- 每个目录条目都公开精确 `policy`、`origin`、`lifecycle` 与 `status`，调用者无需根据命名猜测身份或可用性。
- 个人条目从持久化的规范化 `StrategySpecDocument` 重建，目录不生成新版本，也不覆盖旧版本。

## Architecture constraints

- 目录只是现有官方注册表与 `SqliteStrategySpecRepository` 的只读投影；不新增数据库表或迁移。
- 个人文档必须再次经过 `into_strategy_spec` 校验，预算边界继续复用领域方法。
- 官方策略保留原顺序和行为；个人版本按存储库既有确定顺序追加。
- `PolicyRef` 仍是唯一策略身份，不能用显示名称、family 或 origin 替代。

## Explicit non-goals

- 不增加更新、删除、发布、共享或云同步。
- 不自动把个人策略标记为通过固定样本研究。
- 不改变计划激活准入、回测运行时或官方 100 preset。
- 不修改前端。

## Acceptance criteria

- 空个人库仍返回原有 101 个官方条目。
- 保存一个个人策略后，统一目录返回对应精确 policy/version、`origin=personal`、规范化 formula 和可解释状态。
- 同一 policy id 的多个不可变版本分别出现；重复保存仍冲突且不会覆盖。
- 个人固定金额动作不会被误标为可用于计划。
- 官方条目标记 `origin=official`，且原目录断言继续成立。

## Tests

- 聚焦 API 集成测试：空库官方计数、个人版本合并、多个版本、不可覆盖、不可用个人策略状态。
- `cargo test -p indexlink-api --test strategy_catalog`
- `cargo test -p indexlink-api --test strategies`
- `cargo test -p core-domain`
- `cargo fmt --all -- --check` 与 `git diff --check`

## Deliverables

- 统一目录响应契约与实现。
- API 管理文档更新。
- 聚焦集成测试。
- 本 Push 不提交、不推送，由主任务统一记录 `CHANGE_LOG.md` 与提交。
