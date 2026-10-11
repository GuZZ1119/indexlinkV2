# README 当前事实同步 Hardness

- Goal：保持现有 Logo、视觉风格和技术分层，同步中英文 README 的发布状态、真实使用流程、启动步骤与已知限制，并推送 V2 独立 PR。
- Current state：本地基线 `96fe755`，工作区干净；PR #8 已合并为 V2 `main` 的 `806a050`，两者代码树相同。README 仍称 Beta 版本，流程图未区别 Fixed DCA 与 Formula，缺少 AI 保存步骤、Docker 后端限定及新增审计命令。
- Desired behavior：明确 Beta 候选而非正式 Release；Fixed DCA 无外部能力也可建计划；首次配置不覆盖已有 `.env`；pnpm/Node 与实际配置一致；说明人工执行只闭合审计、未来日程与金额结算尚未实现。
- Architecture constraints：文档以 config、API、Web、Compose 和发布 Gate 为准；不改源码、API、数据库、依赖、运行进程或数据许可证，不赋予 AI 计划创建/交易权限。
- Explicit non-goals：不实现日程投影、资金结算或聊天 Agent，不重做首页设计，不发布 tag/Release，不合并 main，不读取/上传用户凭据或运行数据。
- Acceptance criteria：中英文关键事实一致，新增本地链接均可解析，启动 shell 示例语法正确，版权与 LICENSE 一致，发布阻塞不被描述成已完成，提交仅包含本次文档。
- Tests：检查 README 相对路径与 shell 语法、逐项对照 package/config/Compose/发布计划；`cargo test -p core-domain --locked`、`git diff --check`。启动示例不直接执行，避免写入用户数据库、覆盖 `.env` 或冲突运行进程。
- Deliverables：两份 README、CHANGE_LOG、本文、独立文档提交与 V2 PR。
- Ownership / callers：主线程串行负责四个文档；读者为本地安装者与开源贡献者，事实来源为既有服务端配置、AI 会话 UI、Web CI、备份指南和发布 Gate。
- Rollback：独立 revert 文档提交即可，无 migration 或运行状态变更。
