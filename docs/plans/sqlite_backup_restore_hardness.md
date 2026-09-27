# SQLite 备份与恢复收口 Hardness

## Goal

为 V2.1 唯一公开持久化层 SQLite 提供可验证、失败关闭的本地备份与恢复路径，并覆盖原生运行和 Docker named volume。

## Current state

- 原生运行默认写入仓库根目录 `indexlink.db`。
- Docker Compose 写入容器 `/data/indexlink.db`，由 `sqlite-data` named volume 持久化。
- 项目此前没有统一备份、恢复、完整性校验或升级前检查说明。

## Desired behavior

- 原生备份使用 SQLite online backup，不在运行时裸复制主文件。
- 恢复必须显式确认服务已停止，并拒绝打开的数据库或残留 WAL/SHM/journal。
- 恢复前自动生成当前数据库的安全备份。
- Docker 备份/恢复先停止服务、检查 sidecar，再通过 `docker compose cp` 操作；不假定运行镜像含有 `sqlite3`。
- 备份与恢复后执行 `PRAGMA integrity_check`，服务恢复后通过 `/ready` 验证。

## Architecture constraints

- 不新增持久化层，不修改 migration、领域模型或公开 API。
- 不把备份放入 Git；默认输出目录为 gitignored 的 `backups/`。
- 不把运行中 SQLite 主文件当成一致快照复制。
- 脚本只使用宿主机 `sqlite3` 校验；Docker 镜像无需增加数据库 CLI。

## Explicit non-goals

- 不提供云端备份、加密密钥托管、增量备份或跨数据库迁移。
- 不保证跨 SQLite 大版本或向旧版 IndexLink 降级兼容。
- 不自动寻找或结束正在运行的 IndexLink 进程。

## Acceptance criteria

1. 原生在线备份可在数据库打开时生成一致快照。
2. 原生恢复在未确认停服、数据库仍被占用或 sidecar 存在时拒绝执行。
3. 恢复前保留旧库，恢复后完整性检查通过。
4. Docker 流程不调用容器内 `sqlite3`，并在复制前停服和检查 sidecar。
5. 文档说明路径、升级前动作、恢复、校验与 Docker volume 流程。

## Tests

- `sh -n` 检查四个脚本。
- 临时 SQLite 数据库：在线备份、修改源库、停服恢复、行数核对和 `integrity_check`。
- 负向测试：缺少 `--confirm-stopped` 时恢复失败；存在 `-wal` 时恢复失败。
- `cargo test -p core-domain`（本任务不修改 Rust 行为，但遵守仓库最低验证门槛）。

## Deliverables

- `scripts/sqlite-backup.sh`
- `scripts/sqlite-restore.sh`
- `scripts/docker-sqlite-backup.sh`
- `scripts/docker-sqlite-restore.sh`
- `docs/operations/backup-and-restore.md`

## Ownership and rollback

- 基线：`cf265cd93e95e199f6bed5174b3de230eb80a0f0`。
- 调用者：本地运维用户；应用运行路径不调用这些脚本。
- 文件 ownership：仅工具脚本、备份指南和 `backups/` 忽略规则。
- 回滚：删除上述新增文件并移除 `.gitignore` 的 `/backups/`；不涉及 schema 或数据迁移。
