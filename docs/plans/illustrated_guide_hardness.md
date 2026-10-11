# 中文图文入门教程 Hardness

- Goal：用真实网页截图编写可在 GitHub 阅读的中文入门教程，完成 Fixed DCA 的计划、建议、人工记录与审计主线，并连接 README 与文档索引。
- Current state：基线 `c0396ac`，工作区干净；已有中英文 README、备份说明和候选版门槛，没有逐步截图教程。本机已有用户服务，不能借截图改写其数据库或凭据。
- Desired behavior：新用户可按步骤启动并完成首个不依赖 AI/OpenD 的计划；截图来自独立 SQLite 演示实例，说明真实当前能力、日期条件和未交付限制。
- Architecture constraints：仅新增/更新文档和截图；使用当前 Rust API 和 Web，不伪造接口、收益、订单或执行日期，不改源码、策略、migration 或不可变审计。演示 instance 只监听 loopback，AI、行情和 broker 不配置。
- Explicit non-goals：不生成正式 Release、不解决数据许可、不发布投资建议，不调用付费 AI/OpenD，不接触真实计划，不制作英文全文或 PDF，不执行真实券商交易。
- Acceptance criteria：约 8–12 张清晰真实截图，图片无用户密钥、个人数据或私人路径；创建、建议、记录与历史可见；相对链接与命令语法正确，备份和安全边界说明可追溯。截图用浏览器原生 JPEG，保留原图。
- Tests：独立实例 UI 逐步验收与截图视觉检查；本地 Markdown 链接、图片格式/尺寸、Bash 语法、`git diff --check`；`cargo test -p core-domain --locked`。不对真实数据库执行恢复或业务 mutation。
- Deliverables：`docs/guides/getting-started.zh.md`、`docs/guides/images/`、README 入口、文档索引、CHANGE_LOG 和本文；独立文档分支供审查。
- Ownership / callers：主线程串行负责文档和截图，读者是本地候选版用户；依据来自当前页面、API/config、README 与灾备说明。
- Rollback：revert 本次文档提交；停止本次独立进程即可，不改变已有用户服务和数据库。演示临时库不提交。

## 验收结果

- 2026-10-11 已实拍 12 张 JPEG，1280px 原生宽度，约 890 KiB；12 张按教程阅读顺序引用且逐张目视检查，没有真实用户数据或密钥。
- 独立实例实测 Fixed DCA 创建、原建议 100 / 人工报告 90 的证据分账、跳过、暂停/继续与同日下一评估推进；API 只读核对每份建议恰有一条人工事件。没有修改系统时间或市场数据。
- `cargo build -p indexlink-server --locked`、`cargo test -p core-domain --locked`（13 项）、Web 人工执行与原建议聚焦测试（19 项）通过；演示数据库在线备份完整性检查通过。
- 教程/README/索引本地链接与锚点、Bash 语法、图片格式与尺寸、`git diff --check` 通过；未运行全量测试、Docker、付费 AI、真实行情或 broker 验收。
- 原建议详情和下一评估的日期为截屏当日真实运行结果；可选工坊、行情与 AI 仅展示未保存或未连接状态，不伪造成功画面。
- 2026-10-12 演示前后端和新建截图标签已关闭，原有服务不变。临时库及备份不进入提交；独立分支与已合并 PR #9 的 V2 main 代码树一致。
