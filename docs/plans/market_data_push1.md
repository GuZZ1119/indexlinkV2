# Push 1：通用历史行情与本地缓存

## Goal

为任意受支持的美股、A 股和港股标的建立统一的只读历史日线边界：美国市场可通过 Alpaca REST 显式更新，US/HK/SH/SZ 可通过本机 OpenD 显式更新；成功导入的数据以完整来源元数据和校验和写入本地 SQLite，回测与 Today 后续只读取这一端口。

## Current state

- 基线 commit：`63b6f9a1bd304691a47ae932a400205bdaec6c35`。
- `market-data` 只有面向旧 Decision Preview 的 `MarketSignalProvider`，其 OpenD 读取固定为美股、最近七年，并与 CAPE/VIX/国债抓取耦合。
- SQLite 尚无通用历史行情数据集或日线表。
- 回滚方式：删除本 Push 新增的 Rust 模块和最后一条 SQLite migration；不改写既有表或既有 Decision Record。

## Desired behavior

- 统一请求显式携带 market、symbol、起止日期和 adjustment。
- 数据集显式携带 instrument type、currency、timezone、provider、dataset version、fetched-at、coverage 和 checksum。
- Alpaca adapter 使用 `1Day`、显式 feed/adjustment 和 `next_page_token` 分页。
- OpenD adapter 接受 `US.*`、`HK.*`、`SH.*`、`SZ.*`，使用官方市场枚举且只连接字面 loopback 地址。
- SQLite 按 provider + instrument + adjustment + requested range 缓存不可混拼的快照；命中后不访问远端。
- 既有 `MarketSignalProvider` 行为与调用者保持兼容。

## Architecture constraints

- `market-data` 定义 port 与供应商 adapter；`storage` 只实现缓存 port，供应商协议不进入 storage 或策略领域。
- 不引入供应商 SDK，不把 API key 写入数据库、错误或 `Debug`。
- 不补造缺失交易日、不静默前向填充、不跨 provider 混拼。
- 远端来源只负责显式导入/更新；SQLite 是生产消费者的本地事实来源。

## Explicit non-goals

- 不实现回测轨迹、HTTP 回测 API、前端图表、交易日历或自动下单。
- 不支持分钟线、退市证券主数据解析、全球所有市场或 AkShare/Python runtime。
- 不在本 Push 承诺供应商数据的再分发许可。

## Acceptance criteria

- 同一契约能表达 Alpaca 美股和 OpenD US/HK/SH/SZ 日线。
- 非法 symbol/range、供应商不支持的 adjustment、分页 token 循环、非有限/非正价格均显式失败。
- SQLite round-trip 保留全部来源元数据与有序日线；不同 provider/range 不碰撞。
- 旧市场信号测试继续通过。

## Tests

- `cargo test -p market-data --locked`
- `cargo test -p indexlink-storage --locked`
- `cargo test -p core-domain --locked`
- `cargo clippy -p market-data -p indexlink-storage --all-targets --locked -- -D warnings`
- `cargo fmt --all -- --check`

## Deliverables

- 通用历史行情类型、provider/store port、缓存装饰器。
- Alpaca REST 与 OpenD 历史日线 adapter。
- SQLite migration、repository 与聚焦测试。
- API/配置文档草案和 `CHANGE_LOG.md` 记录。

## External references and licensing notes

- Alpaca 官方 Historical Bars 文档是 REST 字段、分页、feed 和 adjustment 的唯一语义依据。
- Futu OpenD 官方历史 K 线、代码格式和 `QotMarket` 文档是 US/HK/SH/SZ 映射依据；使用方仍需自己的行情权限与配额。
- 参考 `wmzhai/alpaca-data-rs`（MIT OR Apache-2.0）对凭据脱敏、分页流和官方 API 术语的处理方式，只借鉴边界设计，不复制实现。
- `d-e-s-o/apca` 为 GPL-3.0，只用于确认生态中存在历史数据 adapter，不复制其代码，以免许可证污染。
- `quantforge` 的 SQLite closed-bar/local-first 思路仅作架构对照；本实现仍使用 IndexLink 自己的 schema 与 port。
