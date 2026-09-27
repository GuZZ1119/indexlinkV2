# Web CI 与数据/策略许可收口 Hardness

## Goal

在不改变 V2.1 产品运行时、策略公式或数据读取路径的前提下，补齐两项公开发布门槛：让 GitHub 持续验证 Web 质量门槛；让发行者和贡献者能够区分项目代码、第三方数据、研究思想与移植代码的授权边界。

## Current state

- GitHub Actions 只执行 Rust 的格式、Clippy、测试、依赖图和覆盖率检查；前端回归不会阻止合并。
- Web 已有 `lint`、`test:coverage` 和 `build` 脚本，覆盖率阈值已在 Vitest 配置中设为 90%，但没有上游自动执行入口。
- `THIRD_PARTY_NOTICES.md` 记录了主要来源，但没有逐项说明仓库已提交数据的再分发结论。
- 仓库包含 FRED、Cboe 与 Multpl 派生快照。来源可追溯不等于已获得再分发授权；项目 MIT License 不能覆盖这些外部数据。
- 官方策略目录已在 API 中携带来源、授权说明和“独立实现”描述，但没有面向新贡献者的准入规范。

## Desired behavior

- 每次 push 与 pull request 都在固定 Node/pnpm 工具链下，以 lockfile 冻结安装，并依次执行 lint、覆盖率测试和生产构建。
- 数据清单明确标注：项目自有元数据、已确认可用的开源代码参考、必须由用户自行取得的数据、以及再分发权尚未确认的数据。
- 未确认数据不会因为“公开可下载”“注明来源”或“经过清洗”而被描述为可再分发。
- 策略贡献必须声明原创、研究思想独立实现或代码移植；代码移植必须提供许可证、固定版本和所需 notices。

## Architecture constraints

- 不修改 Formula runtime、回测口径、行情 port、SQLite 模型或公开 API。
- CI 使用仓库现有 `apps/web/pnpm-lock.yaml` 与现有 npm scripts，不另造构建路径。
- 数据许可文档只陈述可由公开条款支持的结论；法律状态不确定时采用失败关闭。
- 研究引用不被误写成收益背书，第三方项目名称不被误写成代码依赖。

## Explicit non-goals

- 本任务不替代法律意见，也不代表取得任何数据供应商授权。
- 不删除或重做当前研究夹具；未确认数据是否继续进入正式 release 由后续专门任务处理。
- 不新增行情供应商、下载脚本、策略或前端功能。
- 不修改项目版本、时区、备份、安全配置或 `CHANGE_LOG.md`。

## Acceptance criteria

1. `.github/workflows/web-ci.yml` 固定 Node 22.14.0 与 pnpm 11.15.0，使用 frozen lockfile。
2. Web CI 执行 `pnpm lint`、`pnpm test:coverage`、`pnpm build`，任一步失败即失败。
3. `DATA_LICENSES.md` 覆盖当前所有被 Git 跟踪的研究数据类别，并给出发行决策。
4. `THIRD_PARTY_NOTICES.md` 不再暗示来源记录本身授予再分发权，并链接数据清单与贡献规则。
5. 策略来源规范明确区分原创、思想参考/独立实现、代码移植和数据驱动贡献。

## Tests

- 在 `apps/web` 执行 `pnpm install --frozen-lockfile`。
- 执行 `pnpm lint`。
- 执行 `pnpm test:coverage`，确认 90% 阈值生效。
- 执行 `pnpm build`。
- 人工核对 Git 跟踪的数据文件与 `DATA_LICENSES.md` 清单一致。
- 按仓库规范执行 `cargo test -p core-domain`，确认文档/CI 变更未影响最小领域基线。

## Deliverables

- `.github/workflows/web-ci.yml`
- `DATA_LICENSES.md`
- `docs/contributing/strategy-source-policy.md`
- 更新后的 `THIRD_PARTY_NOTICES.md` 与文档索引

## Baseline, ownership, and rollback

- 基线：开始任务时当前工作树无未提交变更；上游 CI 入口为 `.github/workflows/rust-ci.yml`。
- 文件 ownership：本任务只拥有上述 Deliverables 和本 Hardness 文档；不触碰其他并行任务文件。
- 调用者：GitHub Actions 调用现有 Web scripts；维护者和策略贡献者读取许可文档。
- 回滚：删除新 workflow 与新文档，并恢复 `THIRD_PARTY_NOTICES.md`、`docs/README.md` 的本次链接即可；运行时代码和数据均未改变。
