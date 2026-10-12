<p align="center">
  <img src="assets/icons/indexlink-logo.png" alt="IndexLink" width="360">
</p>

<p align="center">
  <strong>把长期投资方法变成看得懂、能回测、可复查的个人计划。</strong>
</p>

<p align="center">
  Rust 本地服务 · React 用户界面 · SQLite 审计存储 · OpenD 真实日线 · 用户自带 AI
</p>

<p align="center">
  中文文档 · <a href="./readme.en.md">English</a>
</p>

<p align="center">
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-MIT-2f7661" alt="MIT License"></a>
  <a href="./Cargo.toml"><img src="https://img.shields.io/badge/version-2.1.0--beta.1-10242c" alt="Version 2.1.0-beta.1"></a>
  <a href="./SECURITY.md"><img src="https://img.shields.io/badge/security-loopback%20only-c19a55" alt="Loopback only"></a>
</p>

<p align="center">
  <a href="./docs/guides/getting-started.zh.md"><strong>图文入门：建立你的第一个长期计划 →</strong></a>
</p>

# IndexLink V2.1 Beta

IndexLink 是一个面向普通长期投资者的**本地策略计划与研究工具**。它把一条投资方法拆成可解释的规则、真实历史回测、定期建议和不可覆盖的执行记录，让用户能回答四个问题：

1. 这条策略在做什么；
2. 在同一只标的、同一段时间和同一资金节奏下，它与其他策略有什么区别；
3. 这期为什么建议投入、减量或等待；
4. 我最终是否执行，历史记录能否复查。

`2.1.0-beta.1` 是**单用户、本地优先、人工执行**的 Beta 版本。计划与执行记录保存在本机；你在自己的券商完成操作，再回来记录结果。它不会代替用户下单，不承诺收益，也不把 AI 变成交易决策者。

> **风险声明：** 本项目仅供学习、策略研究和 paper-trading 演示，不构成投资建议。历史回测不预测未来；数据错误、市场制度变化、税费、滑点和流动性都可能改变结果。

## 技术制作

IndexLink 不是一组静态页面，也不是把策略判断交给大模型的聊天壳。它是一套在本机运行、由确定性 Rust 规则负责计算、由 React 负责解释与操作的完整应用。

| 层次 | 技术与职责 |
| --- | --- |
| 服务端 | Rust 2021、Axum 与 Tokio；负责策略不变量、计划调度、回测、AI/OpenD 适配和 HTTP 契约 |
| 领域与研究 | Rust workspace 模块化单体；受限 Formula AST、因果指标、统一资金账本、不可变策略版本和可复现研究夹具 |
| 本地数据 | SQLx + SQLite；保存计划、建议输入快照、个人策略和 append-only 手工执行记录 |
| Web | React 19、TypeScript、Vite 8、Tailwind CSS v4、TanStack Query 与 Valtio；服务端状态和临时界面状态分离 |
| 图表 | Apache ECharts 用于可缩放净值、价格与执行轨道；Recharts 用于轻量结果展示 |
| 外部适配 | Futu/Moomoo OpenD 提供本机只读日线；QwenCloud、百炼、GPT、Claude、DeepSeek 仅在用户手动触发时调用 |
| 质量门禁 | Rustfmt、Clippy、Cargo workspace tests、ESLint、TypeScript build 与 Vitest；前端四项覆盖率均须 >=90%，Web CI 对全量依赖执行 high 审计门禁 |

架构采用 **Hexagonal Architecture（Ports & Adapters）+ Rust modular monolith**：核心规则不依赖浏览器、数据库、行情源或模型供应商，外部能力可以替换，输入与结论均能留下可审计证据。

## 当前已经能做什么

| 模块 | 当前能力 | 明确边界 |
| --- | --- | --- |
| 个人中心 | 读取真实计划、本期建议、下次评估日和 append-only 执行历史；手动记录“已执行”或“跳过” | 不自动下单；同一到期建议只能形成一条最终确认 |
| 我的计划 | 创建、暂停、继续和删除长期计划；冻结策略版本、标的、预算与评估节奏 | Fixed DCA 不依赖行情；公式策略创建前必须通过历史数据预检 |
| 策略中心 | 1 个 Fixed DCA 基准、20 个规则家族 × 5 组不可变参数，共 101 个官方版本；个人版本单独标注 | “可建立”不代表适合任何资产，也不代表未来有效 |
| 策略工坊 | 用白名单指标、观察窗口、比较符、阈值和机会额度建立个人规则；保存不可变版本 | 不接受任意代码；最多三条优先规则、每条三个条件；核心投入不能被取消 |
| 策略分析 | 对 US/HK/SH/SZ 的自选标的运行真实日线回测；支持 1m/3m/6m/1y/3y/5y/all；展示净值、买点、资金账本和风险指标 | 所有策略使用同一标的、日期、现金流与成本口径；缺少真实数据时明确失败，不生成演示曲线 |
| 高级实验室 | 临时连接本机 Futu/Moomoo OpenD；临时输入 QwenCloud、百炼、GPT、Claude 或 DeepSeek Key | 凭据只留在当前 Rust 进程内存，重启即失效；OpenD 页面配置只授予只读行情能力 |
| 可选 AI | 手动执行“自然语言 → 受限草案”“真实回测 → 普通解释”“近期计划摘要” | 不参与回测计算，不保存/激活策略，不生成订单，不自动运行 |

## 一条完整的本地闭环

```text
策略中心 / 策略工坊
        ↓ 选择不可变策略版本
        ├─ Fixed DCA：无需行情，可直接建立计划
        └─ 公式策略：历史数据预检通过后建立计划
        ↓
本地计划 → 到期时由调度器幂等生成建议
查看建议与依据
        ↓ 用户在自己的券商操作
手工确认“已执行 / 跳过”
        ↓
执行历史与输入快照永久追加、不可覆盖
```

系统中的“执行”默认是**用户报告的事实记录**。可选 OpenD paper broker 仅用于本机模拟账户实验；scheduler 和 AI 都没有自动提交订单的权限。

真实回测是比较策略的研究步骤，需要可用的真实日线；它不是 Fixed DCA 建计划的前置条件。即使没有 AI、OpenD 或行情，Fixed DCA 的计划、建议与人工执行审计仍可使用。

### 当前限制

- 个人中心只显示下一评估日期，尚无完整的 3/6/12 个月日程投影；评估日期按计划冻结时区解释，不保证当天是交易所可成交日。
- 历史待办是已保存的原建议，不是今天重新计算的金额；保存日期不能代替原计划执行日。
- 手工执行记录已形成审计闭环，但实际金额尚未结算到后续周期预算或机会现金；不能把日志理解为完整持仓/资金账本。manual journal 与 paper broker 继续分账。
- AI 返回受限策略草案、解释或摘要，不能直接创建或激活个人计划；Formula 的未来精确投入金额须到评估日才能确定。

有限期限日程投影与手工资金结算仍是[后续计划](./docs/plans/v2_1_productization_plan.md)，不是已交付能力。

## 策略模型

V2.1 的公共策略不是 100 份互不兼容的脚本，而是同一个受限公式框架下的不可变版本：

```text
历史日线 → 因果指标（只读取观察日及以前数据）
         → 按优先级检查规则
         → 调整本期“机会额度”
         → 核心投入保持不变
```

当前白名单覆盖价格/指数均线、双均线、三均线、RSI、历史价格分位、波动率、波动率扩张、回撤、接近高点、趋势与波动组合、动量与波动组合、趋势与回撤组合、趋势与动量组合等家族。公式定义、参数和来源随策略版本一起冻结；第一个命中的规则停止后续判断。

Fixed DCA 始终保留为基准。旧“70/20/10 自适应”研究没有进入普通用户目录；其历史实验与未形成稳定收益优势的结论仍保留在 [研究文档](./docs/README.md)，避免把失败结果从项目历史中抹去。

### 回测口径

- 真实日线来自当前配置的只读 provider；普通运行时目前接入本机 OpenD，并使用 SQLite 缓存命中。
- 相同对比中的策略共享标的、起止日期、投入预算、交易日映射和成本模型。
- 净值指数以共同起点 `100` 归一化，只比较资金路径变化，不代表股价或账户余额。
- 专业视角提供区间收益、年化收益、XIRR、最大回撤、年化波动、Sortino、现金使用率、交易成本、峰值/低点/恢复日以及公式代入值。
- 指标预热不足、行情授权不足、数据过期或 provider 不可用时返回明确错误；不会用静态收益率或随机曲线补位。

## 本地启动

第一次使用可跟随[中文图文入门教程](./docs/guides/getting-started.zh.md)，从无需 AI/OpenD 的 Fixed DCA 计划开始，了解建议、人工记录、历史与备份。

### 依赖

- Rust stable（包含 `cargo`、`rustfmt`、`clippy`）
- Node.js >=22.19.0（推荐受支持的 LTS）与 pnpm `11.15.0`（与 `apps/web/package.json`、CI 一致）；冻结安装需要同时保留 `apps/web/pnpm-workspace.yaml` 和锁文件
- 可选：本机 Futu/Moomoo OpenD，用于 US/HK/SH/SZ 真实日线与模拟账户实验

### 1. 启动后端

```bash
git clone https://github.com/GuZZ1119/indexlinkV2.git
cd indexlinkV2
if [ ! -e .env ]; then cp .env.example .env; fi
APP_HOST=127.0.0.1 APP_PORT=8080 cargo run -p indexlink-server
```

上述命令仅在首次运行时复制配置，不覆盖已有 `.env`；现有安装从项目目录直接执行启动命令即可。默认 SQLite 位于项目根目录 `indexlink.db`，可通过 `DATABASE_URL` 修改。后端监听 `127.0.0.1:8080`；默认 scheduler 只生成到期建议，paper broker 默认关闭。检查服务：

```bash
curl --fail http://127.0.0.1:8080/health
curl --fail http://127.0.0.1:8080/ready
```

### 2. 启动前端

在另一个终端、同一个项目目录执行：

```bash
pnpm --dir apps/web install --frozen-lockfile
pnpm --dir apps/web dev --host 127.0.0.1 --port 5173 --strictPort
```

浏览器打开 `http://127.0.0.1:5173/personal`。分别在前后端终端按 `Ctrl+C` 停止；使用另一种后端启动方式前，先停止已有实例。

常见启动问题：

- `NonLoopbackBindForbidden`：检查旧 `.env` 或 shell 的 `APP_HOST`，改为 `127.0.0.1`；不要通过开启 `ALLOW_UNSAFE_NON_LOOPBACK_BIND` 绕过普通本地安装的安全边界。
- `Address already in use` / `Port 5173 is already in use`：用 `lsof -nP -iTCP:8080 -sTCP:LISTEN` 或对应的 `5173` 检查占用，使用已启动的服务或在其原终端停止后重启；不要直接结束不明进程。
- 依赖安装或构建失败：先检查 `node --version` 与 `pnpm --version`，确认满足上面的版本要求。

### 3. 可选：接入 OpenD

先启动并登录 Futu/Moomoo OpenD，再在“高级实验室”输入回环地址；也可在本机 `.env` 中配置：

```dotenv
OPEND_PROVIDER=moomoo
OPEND_HOST=127.0.0.1
OPEND_PORT=11111
OPEND_MARKET_DATA_ENABLED=true
OPEND_PAPER_BROKER_ENABLED=false
```

没有 OpenD 时，计划管理、Fixed DCA、执行日志和已缓存研究仍可使用；新的任意标的真实回测与需要行情的公式策略会明确提示数据不可用。

### 4. 可选：接入自己的 AI

在“高级实验室”选择供应商、模型并输入 Key，先点击“保存本次连接”，再点击“验证 AI 可用性”。保存只表示凭据进入当前 Rust 进程，不代表供应商验证通过；验证会调用一次最小模型请求，可能产生 API 费用。QwenCloud 与阿里云百炼使用不同供应商地址和 Key，不能混用。Key 不写入浏览器存储、SQLite、`.env` 或日志，后端重启后页面连接需重新配置。策略草案、解释和摘要仍需用户逐次点击。

### 5. 可选：本地 Docker

这是原生 `cargo run` 后端的替代方案，**只启动 Rust API，不包含 Web 服务**。先按首次配置步骤准备 `.env`，停止占用 8080 的原生后端，再执行：

```bash
docker compose -f deployment/docker-compose.yml up --build
```

默认 API 为 `http://127.0.0.1:8080`；Web 仍按第 2 步在另一个终端启动。若修改 Compose 发布端口，须同步调整 Web API 地址或代理，默认前端代理固定为 8080。Compose 的 SQLite 在 `sqlite-data` volume 中，与原生 `indexlink.db` 是独立数据。停止容器可执行 `docker compose -f deployment/docker-compose.yml stop server`，不要为重启删除 volume。

Compose 只把服务发布到宿主机 `127.0.0.1`。本仓库不提供云服务器一键部署脚本，不支持自行反向代理或开放端口。远程访问需要认证、TLS、限流和 CSRF/Origin 防护，不能直接暴露当前 API。

计划、建议快照和执行历史保存在本地 SQLite。升级、切换分支或恢复历史数据前，请先按[本地备份与恢复指南](./docs/operations/backup-and-restore.md)生成并校验备份；不要直接复制仍在运行的 WAL 数据库主文件。

## 安全模型

- 后端默认只监听 loopback；非回环 `APP_HOST` 会启动失败。Docker 仅在容器命名空间内以 `ALLOW_UNSAFE_NON_LOOPBACK_BIND=true` 监听 `0.0.0.0`，宿主端口仍只发布到 `127.0.0.1`。该开关不提供认证，API **不得**直接暴露到局域网或公网。
- HTTP 请求体和 RSS 响应体均有 1 MiB 上限；模型解析错误不记录原始输出。
- AI endpoint 由服务端供应商配置固定，远程地址要求 HTTPS；API Key 不回显。
- OpenD host 必须是回环地址；只配置 provider 默认仅启用只读行情，paper broker 必须通过 `OPEND_PAPER_BROKER_ENABLED=true` 显式开启，页面会话也不会自动授予 broker 能力。
- 领域 newtype、受限 DSL、策略版本和执行 journal 在服务端重新校验；前端输入不是安全边界。
- SQL 写入使用参数绑定；策略 runtime 不执行用户脚本，也不直接访问网络、数据库或 broker。

完整威胁模型、仍未解决的依赖告警和公开披露方式见 [SECURITY.md](./SECURITY.md)；本次收口审查见 [V2.1 代码审查](./docs/reviews/v2_1_code_audit_2026-09-23.md)。

## 架构

IndexLink 使用 **Hexagonal Architecture（Ports & Adapters）+ Rust modular monolith**。领域规则位于内部，SQLite、OpenD、AI、HTTP 与 Web 都是可替换适配器。

```mermaid
flowchart LR
  Web[React Web] --> API[Axum application API]
  Scheduler[Local scheduler] --> API
  API --> Plans[Plans and decisions]
  API --> Runtime[Deterministic policy runtime]
  API --> Backtest[Backtest and research]
  Runtime --> Journal[(SQLite snapshots and journal)]
  Backtest --> Data[OpenD daily bars and cache]
  API -. manual only .-> AI[User-provided AI provider]
  API -. explicit paper only .-> Broker[OpenD paper broker]
```

主要目录：

```text
apps/server                 Rust 组合根、本地 scheduler
apps/web                    Vite + React + Tailwind 前端
crates/core-domain          带不变量的领域类型
crates/investment-plans     计划、周期和预算规则
crates/strategy-policy      统一策略契约
crates/strategy-dsl         受限公式 AST、校验与解释器
crates/builtin-policies     Fixed DCA 与兼容策略
crates/strategy-evaluation  回测、指标、研究夹具
crates/market-data          OpenD/Alpaca 只读数据适配器与缓存契约
crates/ai-client            多供应商 AI 协议与有界输出
crates/broker               Mock/OpenD paper-only 适配器
crates/storage              SQLite 审计存储
crates/api                  HTTP 契约与应用编排
docs                        API、计划、审查与策略研究
```

公开 API 契约见 [API 管理手册](./docs/reference/api-management.md)，前端边界见 [Web Plan](./apps/web/PLAN.md)，完整文档见 [docs/README.md](./docs/README.md)。

## 验证

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
pnpm --dir apps/web lint
pnpm --dir apps/web test:coverage
pnpm --dir apps/web build
pnpm --dir apps/web audit --audit-level=high
pnpm --dir apps/web audit --prod
git diff --check
```

行为变更需要聚焦测试，并在 [CHANGE_LOG.md](./CHANGE_LOG.md) 记录涉及文件、验证结果和模型。前端 statements、branches、functions、lines 均须 >=90%；Web CI 的全量依赖审计会拒绝 high/critical 公告。`cargo audit` 需单独安装 cargo-audit；既有 Rust 锁文件公告与未运行的网络/OpenD 测试边界见 [SECURITY.md](./SECURITY.md) 和发布记录，不应忽略失败或把未验证项描述成通过。

## 开源来源与引用

本项目没有把 QuantConnect LEAN、TA-Lib 或论文中的交易代码直接复制进 runtime；它们用于指标语义、回测边界和策略研究的交叉核对。公式实现、因果性约束、预算模型与审计契约均在本仓库独立实现并测试。

- 架构：Alistair Cockburn 的 [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture)。
- 回测与研究参考：[QuantConnect LEAN](https://github.com/QuantConnect/Lean)（Apache-2.0）、[TA-Lib](https://ta-lib.github.io/)（BSD）、Meb Faber 的 [A Quantitative Approach to Tactical Asset Allocation](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=962461)。
- UI 与图表：[shadcn/ui](https://github.com/shadcn-ui/ui)（MIT，组件模式与少量 Tailwind variant）、[Recharts](https://recharts.github.io/)（MIT）、[Apache ECharts](https://echarts.apache.org/)（Apache-2.0）、[TanStack Query](https://github.com/TanStack/query)（MIT）。
- 行情接口：Futu 官方 [OpenD / OpenAPI 文档](https://openapi.futunn.com/futu-api-doc/en/intro/intro.html)。
- 研究数据：FRED、Cboe 与 Multpl 的来源及校验值记录在 manifest；当前快照的再分发授权尚未确认，不属于项目 MIT 授权范围。使用和分发边界见下方数据许可清单。

逐文件发行判断见 [DATA_LICENSES.md](./DATA_LICENSES.md)，策略贡献分类见[策略来源规范](./docs/contributing/strategy-source-policy.md)，许可证和“参考而非复制”的边界见 [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)。

## 项目状态与不做事项

V2.1 的目标是把本地闭环做完整，不继续扩大成券商托管平台。以下内容不在当前承诺中：

- 多用户账户、云端同步和公网服务；
- 自动实盘、托管资金、自动撤单和收益保证；
- 分钟级/高频策略、任意 Python/JavaScript 策略；
- 社交发布、跟单、策略市场和跨用户排行榜；
- 把 AI 新闻情绪自动注入普通策略。

后续方向包括有限期限日程投影、手工执行资金结算、交易日历和资金/观察频率拆分；这些尚未交付，具体优先级根据实际使用反馈确定。

## License

Copyright © 2026 Jame / Xuan Zhou（与 [LICENSE](./LICENSE) 署名一致）。项目代码以 MIT License 发布。第三方库、研究数据和外部服务分别受其原始许可证与使用条款约束。
