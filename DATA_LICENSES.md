# Data licensing and release inventory

IndexLink 的 MIT License 只授权本项目自行编写的代码和文档，**不会**自动授权仓库中来自第三方的行情、指数、估值、波动率或研究数据。本清单记录 2026-09-26 对当前 Git 跟踪文件的发行判断；它不是法律意见，也不代表项目已取得任何供应商的额外许可。

## 状态定义

| 状态 | 含义 | 发行处理 |
| --- | --- | --- |
| 项目自有 | 由 IndexLink 作者编写、不含第三方数据值的代码、规则或元数据 | 可随 MIT 源码发行 |
| 用户自行获取 | 数据由用户通过自己的账户、权限或本机 gateway 取得 | 不进入仓库或官方发行包 |
| 再分发未确认 | 已知来源，但公开条款没有为本项目确认再分发权，或上游数据另有第三方权利 | 失败关闭；取得书面授权或替换为明确兼容的数据前，不应进入正式发行包 |
| 外部代码移植 | 包含第三方代码或可受版权保护的表达 | 只有通过许可证兼容审查并保留 notices 后才能合入 |

“公开可下载”“免费访问”“已经注明来源”“经过清洗”或“生成了 checksum”都不等于可以再分发。

## 当前仓库数据清单

### 再分发未确认：正式发行前需要处理

| Git 跟踪文件 | 来源 | 当前用途 | 结论 |
| --- | --- | --- | --- |
| `crates/strategy-evaluation/data/raw/fred_sp500_daily.csv` | FRED `SP500`；底层系列可能包含第三方权利 | 离线研究，作为 S&P 500 指数代理，不是 SPY ETF 成交价或总回报 | 未确认。FRED 条款明确说明部分系列属于第三方，FRED 提供访问不代表授予再使用权；当前文件不得被视为 MIT 数据 |
| `crates/strategy-evaluation/data/raw/fred_nasdaqcom_daily.csv` | FRED `NASDAQCOM`；底层系列可能包含第三方权利 | 离线研究，作为 NASDAQ Composite 指数代理，不是 QQQ ETF 成交价或总回报 | 未确认；处理同上 |
| `crates/strategy-evaluation/data/raw/fred_dgs10_daily.csv` | FRED `DGS10` | 离线研究中的十年期国债收益率输入 | 未确认。即使原始政府数据可能另有公共使用规则，当前提交快照的取得路径和 FRED 条款仍不足以在这里宣告可再分发 |
| `crates/strategy-evaluation/data/raw/cboe_vix_daily.csv` | Cboe VIX 历史 CSV | 离线研究中的波动率输入 | 未确认。Cboe 的公开资料和数据政策保留数据权利；未取得适用于本仓库的书面再分发授权 |
| `crates/strategy-evaluation/data/raw/shiller_cape_monthly.csv` | Multpl 网页的清洗后月度 Shiller P/E 数值 | 离线估值研究 | 未确认。未找到覆盖该派生快照公开再分发的明确许可；清洗不消除来源权利 |

以下生成物包含或汇总了上表数据，同样不能仅凭生成脚本为 MIT 就推导出数据可再分发：

- `crates/strategy-evaluation/data/generated/calibration-v1.json`
- `crates/strategy-evaluation/data/generated/calibration-v2.json`
- `crates/strategy-evaluation/data/generated/calibration-v1.report.json`
- `crates/strategy-evaluation/data/generated/calibration-v2.report.json`
- `crates/strategy-evaluation/data/generated/calibration-v2-c3-research.report.json`
- `crates/strategy-evaluation/data/generated/calibration-v2-c4-research.report.json`
- `docs/research/calibration/*.md` 与 `docs/research/experiments/*.md` 中根据这些快照生成的数值表格和研究结果

**发布门槛：** 上述快照目前仍被源代码用于可复现研究，直接删除会破坏构建或测试。正式 GitHub Release 不应把它们描述为 MIT 授权数据；在面向公开发行前，应完成下列方案之一：

1. 从相关权利人取得并归档适用于公开仓库和发行包的授权；
2. 改为用户在本机显式导入，并确保下载/缓存方式符合供应商条款；
3. 替换为具有明确兼容许可的公开数据，并记录精确许可证；
4. 对单元测试使用项目自行生成、明确标注为 synthetic 的小型夹具。

### 项目自有元数据与合成夹具

下列文件由项目生成或编写，不等于给它们所描述的外部数据授予许可：

| 文件 | 可发行边界 |
| --- | --- |
| `crates/strategy-evaluation/data/generated/*.manifest.json` | 清单结构、缺失值规则、checksum 和项目说明可随项目发行；其中的供应商名称、链接及事实性元数据仍受相应商标和条款约束 |
| `crates/strategy-evaluation/data/generated/qwen-sensitivity-v1.json` | 项目自行冻结的合成敏感度序列；不是历史 Qwen 输出，不得用于历史收益主张 |
| `tools/generate_calibration_fixture.py` | 项目代码；MIT。脚本许可不覆盖输入或输出中的第三方数据 |

## 用户自行获取的数据

以下数据不应由 IndexLink 官方仓库或 release 代用户分发：

- 用户通过本机 Futu/Moomoo OpenD 和本人账户权限读取的日线；
- 运行时显式更新的 Cboe VIX 或其他外部市场数据；
- 用户导入的 CSV、本机缓存和回测结果；
- 用户调用 QwenCloud、百炼、OpenAI、Anthropic 或 DeepSeek 所产生的输入与输出。

这些内容保存在用户控制的本地环境中，并继续受原供应商条款、账户权限、市场与地区限制约束。IndexLink 不替用户授予使用、缓存、导出或再发布权。

## 官方来源

- [FRED Services Terms of Use](https://fred.stlouisfed.org/legal/terms/)：要求归属说明，并指出第三方系列的权利不因 FRED 提供访问而被覆盖；在超出个人使用前可能需要联系数据权利人。
- [FRED API Terms of Use](https://fred.stlouisfed.org/docs/api/terms_of_use.html)：规定 API 使用、归属和第三方权利边界。
- [Cboe Contracts & Data Policies](https://datashop.cboe.com/data-policies)：Cboe 数据合同与数据政策入口。
- [Futu OpenAPI introduction](https://openapi.futunn.com/futu-api-doc/en/intro/intro.html)：用户侧 OpenD/OpenAPI 能力与账户权限入口。

Multpl 当前仅记录[来源页面](https://www.multpl.com/shiller-pe/table/by-month)；在本次审查中没有确认到允许把清洗后历史表随 MIT 仓库公开再分发的明确许可证，因此维持“再分发未确认”。

## 新增数据的准入要求

任何新增或替换数据都必须在合入前提供：

1. 数据集、系列、供应商和原始 URL；
2. 取得日期、覆盖区间、频率、时区、币种与调整方式；
3. 缺失值、去重、清洗和派生规则；
4. 文件 checksum 与 dataset version；
5. 精确许可证/条款 URL、访问日期和允许再分发的条款依据；
6. 是否包含第三方底层权利、商标、指数或交易所限制；
7. 选择“随仓库发行”“仅由用户获取”或“synthetic test fixture”的明确决定。

没有明确许可证的 GitHub 仓库、网页或 API 按“保留全部权利”处理；不得因为能够下载就提交其数据。

策略思想和代码来源的贡献要求见 [`docs/contributing/strategy-source-policy.md`](./docs/contributing/strategy-source-policy.md)，完整第三方软件与研究引用见 [`THIRD_PARTY_NOTICES.md`](./THIRD_PARTY_NOTICES.md)。
