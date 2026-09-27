# 策略来源与许可证贡献规范

本规范适用于新增官方策略家族、参数档位、指标公式、研究夹具或从第三方项目借鉴的实现。它的目标不是收集更多策略，而是让每条策略的**来源、实现归属、可运行边界和证据**都能被复核。

## 先选择来源类型

每个策略贡献必须且只能选择一个主要类型，并在 PR 中写明。

### A. 项目原创

规则与实现由贡献者独立设计，没有复制第三方代码或可受版权保护的文字表达。

必须提供：

- 规则公式、指标定义、参数与单位；
- 为什么适合长期、人工确认的 IndexLink 计划；
- 所需最小历史窗口、评估频率和因果 cutoff；
- 与 Fixed DCA 同口径的测试和失败场景；
- `Original IndexLink implementation` 或等价的来源说明。

原创不代表策略有效，也不能省略风险和局限说明。

### B. 研究思想参考、IndexLink 独立实现

参考论文、书籍、指标文档或公开策略思想，但 Formula、预算模型、命名、解释与 Rust 实现由 IndexLink 独立编写。

必须提供：

- 原始标题、作者/组织、稳定 URL 与访问日期；
- `research reference`、`indicator reference` 或 `documentation reference`；
- 明确声明“只参考思想/指标语义，没有复制第三方交易代码”；
- 说明 IndexLink 做了哪些适配，例如把交易择时改成周期检查下的机会桶调整；
- 若引用论文中的数值参数，标明具体表格、章节或推导方式，避免把自选参数冒充论文结论。

论文或文档的引用不表示作者认可 IndexLink，也不表示对应策略有收益保证。

### C. 第三方代码移植或改编

复制、翻译、机械改写或实质性改编第三方源代码，都属于代码移植，不能标记为“思想参考”。

合入前必须提供：

- 上游仓库、固定 commit/tag 与原始文件路径；
- SPDX license identifier 与完整许可证链接；
- copyright、NOTICE、署名和修改说明；
- 从上游文件到 IndexLink 文件的对应关系；
- 许可证兼容性审查结论。

只有“GitHub 上公开”但没有许可证的代码按保留全部权利处理，不得移植。GPL/AGPL/LGPL、商业许可、研究专用或禁止衍生的内容不能仅凭贡献者判断合入；必须交由维护者进行独立兼容性审查。不得通过改变量名或让 AI 重写来规避许可证义务。

### D. 数据驱动策略或研究夹具

只要 PR 新增外部行情、指数、估值、新闻或研究数据，就必须同时满足 [`DATA_LICENSES.md`](../../DATA_LICENSES.md) 的数据准入要求。

来源可追溯不代表允许再分发。许可不明确时，应提供用户本地导入流程或 synthetic test fixture，而不是把数据提交进仓库。

## PR 必填模板

策略 PR 描述至少包含以下内容：

```markdown
## Strategy provenance

- Source type: original | research-reference | code-adaptation | data-driven
- Strategy family / policy id:
- Original source title and author:
- Stable URL:
- Accessed on:
- Upstream commit/tag and file path (code adaptation only):
- SPDX license / terms URL:
- Copied code: yes / no
- Adaptation made for IndexLink:
- Required notices added at:

## Runtime and evidence

- Indicators and exact formula:
- Parameters and units:
- Minimum causal history:
- Evaluation frequency:
- Supported market/instrument assumptions:
- Known failure modes:
- Fixed DCA comparison added:
- Focused tests added:

## Data

- Bundles external data: yes / no
- If yes, DATA_LICENSES inventory entry and redistribution basis:
- Dataset version / checksum / as-of:
```

任何无法填写的许可证项必须写“未确认”；“未确认”不是允许合入数据或代码的依据。

## 目录与 API 表示要求

进入官方策略目录的条目必须让 API 使用者看到：

- 策略家族与不可变 policy ID/version；
- 可读规则和局限；
- 来源名称和稳定 URL；
- `license` 字段的准确含义，例如 `MIT`、`Apache-2.0 code adaptation`、`BSD indicator reference` 或 `research reference`；
- `adaptation` 字段说明是独立实现还是代码移植；
- 最小历史要求和准入校验模式。

不得使用含糊的 `open source` 代替具体许可证。对论文或概念只写 `research reference`，不能把第三方软件的许可证写成论文内容的许可证。

## 审查门槛

维护者在合入前确认：

1. Fixed DCA 仍是同口径基准，策略没有绕过核心投入边界；
2. runtime 只使用白名单、因果指标，不执行用户脚本或网络 IO；
3. 相同输入、dataset version、成本与执行时点得到确定性结果；
4. 来源分类与实际实现一致，所需 notices 已进入 `THIRD_PARTY_NOTICES.md`；
5. 数据发行决策已进入 `DATA_LICENSES.md`；
6. 测试验证公式与边界，而不是只验证某段历史收益较高。

来源或许可证无法澄清时，策略可以停留在讨论/研究文档中，但不能进入官方可建立目录。

## 当前官方目录的解释

当前 100 个 Formula 参数预设是 IndexLink 受限规则与参数的独立组合。QuantConnect LEAN、TA-Lib、Meb Faber 等链接用于指标语义、研究思想或回测边界交叉核对；现有 runtime 声明没有复制这些项目的交易代码。若未来发现实际实现包含第三方代码，应立即重新分类为“代码移植”，补充固定上游版本、许可证和 notices，而不是继续沿用“独立实现”标签。
