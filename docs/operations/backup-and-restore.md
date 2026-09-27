# 本地 SQLite 备份与恢复

IndexLink V2.1 只公开支持 SQLite。计划、不可变建议快照、个人策略和用户报告的执行历史都在同一个数据库中。升级代码、切换分支、修改 Compose volume 或恢复历史数据前，先创建备份。

## 数据库在哪里

| 启动方式 | 默认位置 | 配置来源 |
| --- | --- | --- |
| `cargo run -p indexlink-server` | 仓库根目录 `indexlink.db` | `.env` 的 `DATABASE_URL=sqlite://indexlink.db?mode=rwc` |
| Docker Compose | 容器 `/data/indexlink.db` | Compose 的 `sqlite-data` named volume |

原生运行可通过 `DATABASE_URL` 改为其他本地文件。本文脚本不支持 `sqlite::memory:`，也不提供 PostgreSQL/MySQL 迁移。

默认备份写入仓库根目录的 `backups/`，该目录已被 Git 忽略。备份包含用户的投资计划和执行历史，应按敏感个人数据保护，不要提交到 Git 或公开分享。

## 原生运行：在线一键备份

宿主机必须安装 `sqlite3`。服务器可以继续运行；脚本调用 SQLite online backup API，不会对运行中的 WAL 数据库做不一致的裸 `cp`。

```bash
./scripts/sqlite-backup.sh
```

也可以显式指定数据库 URL/路径和输出文件：

```bash
./scripts/sqlite-backup.sh \
  'sqlite://indexlink.db?mode=rwc' \
  backups/before-v2.1-upgrade.sqlite
```

脚本会对快照执行 `PRAGMA integrity_check`，并在宿主机支持时写入同名 `.sha256` 校验文件。升级前必须保留一份完整性检查通过的备份。

## 原生运行：恢复

恢复会替换当前数据库。先在运行后端的终端按 `Ctrl-C`，确认服务停止，再执行：

```bash
./scripts/sqlite-restore.sh \
  backups/before-v2.1-upgrade.sqlite \
  --confirm-stopped
```

如果目标数据库不是默认路径：

```bash
./scripts/sqlite-restore.sh \
  backups/before-v2.1-upgrade.sqlite \
  'sqlite:///absolute/path/indexlink.db?mode=rwc' \
  --confirm-stopped
```

恢复脚本会失败关闭：

1. 要求显式传入 `--confirm-stopped`；
2. 使用 `lsof` 拒绝仍被进程打开的数据库；
3. 发现 `-wal`、`-shm` 或 `-journal` sidecar 时拒绝恢复；
4. 校验备份 checksum（如存在）和 `integrity_check`；
5. 在 `backups/pre-restore-*.sqlite` 自动保存现有数据库；
6. 先恢复到同目录临时文件，校验成功后再原子替换目标。

恢复完成后重新启动服务并验证：

```bash
cargo run -p indexlink-server
curl --fail http://127.0.0.1:8080/ready
```

再从网页确认计划数量、最近建议与执行历史。`/ready` 只证明数据库可以连接，不能代替业务记录抽查。

## Docker volume：一键备份

Docker 运行镜像不包含 `sqlite3`；脚本只用容器内的 `sh` 检查 sidecar，用宿主机 `sqlite3` 做完整性验证。它会停止 `server`，确认 WAL/SHM/journal 均不存在，再用 `docker compose cp` 复制一致的主数据库；此前若正在运行，备份后会重新启动。

```bash
./scripts/docker-sqlite-backup.sh
```

指定输出：

```bash
./scripts/docker-sqlite-backup.sh backups/before-docker-upgrade.sqlite
```

如果 Compose 文件不在默认位置，可设置 `COMPOSE_FILE`。不要直接复制 Docker volume 中运行中的 `indexlink.db`。

## Docker volume：恢复

恢复要求已至少运行过一次 Compose，使 `server` 容器和 volume 存在：

```bash
./scripts/docker-sqlite-restore.sh \
  backups/before-docker-upgrade.sqlite \
  --confirm-stopped
```

脚本会：

1. 在宿主机校验待恢复文件；
2. 停止 Compose `server`；
3. 通过只挂载 volume 的临时 shell 检查 WAL/SHM/journal；
4. 将现有数据库保存为 `backups/pre-docker-restore-*.sqlite`；
5. 复制并原子替换 volume 内的数据库；
6. 复制回来再次运行 `integrity_check`；
7. 启动服务并轮询 `http://127.0.0.1:8080/ready`。

若 `/ready` 未在 30 秒内通过，脚本会报告预恢复备份位置但不会假装成功。检查 `docker compose -f deployment/docker-compose.yml logs server` 后，可用该预恢复备份再次运行恢复命令。

## 升级检查清单

```text
[ ] 记录当前 Git commit/tag
[ ] 创建带时间戳的 SQLite 备份
[ ] 确认 integrity_check: ok
[ ] 将备份复制到仓库目录之外的安全位置
[ ] 停止服务后再升级代码或容器
[ ] 启动新版本并检查 /ready
[ ] 抽查计划、建议和执行历史
[ ] 确认新版本稳定后再清理旧备份
```

数据库 migration 是向前执行的；不要假设升级后的数据库能被旧版本读取。需要回滚应用版本时，同时恢复升级前备份。
