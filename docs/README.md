# IndexLink 文档索引 / Documentation Index

根目录的 [中文 README](../readme.md) 与 [English README](../readme.en.md) 是当前功能、运行方法和产品边界的入口。这里保留仍能解释 V2.1 代码或研究结论的维护者文档；已经完成且与现状冲突的 push 执行稿不再进入公开上游。

## 当前契约 / Current contracts

- [API 管理与接口契约](./reference/api-management.md)
- [Web 信息架构与数据边界](../apps/web/PLAN.md)
- [安全模型与披露政策](../SECURITY.md)
- [第三方项目、研究与数据来源](../THIRD_PARTY_NOTICES.md)
- [变更与验证日志](../CHANGE_LOG.md)

## V2.1 收口 / V2.1 closeout

- [V2.1 本地优先产品化计划](./plans/v2_1_productization_plan.md)
- [V2.1 收口 Hardness 与执行门槛](./plans/v2_1_closeout_hardness.md)
- [2026-09-23 代码与安全审查](./reviews/v2_1_code_audit_2026-09-23.md)

计划文档用于说明约束和剩余工作；若它与公开 API 或 README 冲突，以代码、测试和当前 API 文档为准。

## 策略研究 / Policy research

- [策略校准基线 V1](./research/calibration/STRATEGY_CALIBRATION_BASELINE_V1.md)
- [C1 候选记录](./research/calibration/STRATEGY_CALIBRATION_CANDIDATES_V1.md)
- [C2 研究](./research/calibration/STRATEGY_CALIBRATION_RESEARCH_V2.md)
- [C3 研究](./research/calibration/STRATEGY_C3_RESEARCH_V1.md)
- [C4 研究](./research/calibration/STRATEGY_C4_RESEARCH_V1.md)
- [自适应定投实验（2026-07）](./research/experiments/ADAPTIVE_DCA_EXPERIMENT_2026-07.md)
- [三次复判实验（2026-07）](./research/experiments/ADAPTIVE_DCA_THREE_RECHECK_EXPERIMENT_2026-07.md)

这些文档记录历史实验、失败候选和方法限制，不表示相关策略已进入普通策略目录，也不构成收益承诺。

## 文档维护规则 / Maintenance

1. 新增或修改公开 API 时，同步更新 `docs/reference/api-management.md`。
2. 功能、配置、安全边界或依赖发生变化时，同步更新根 README、`SECURITY.md` 和 `CHANGE_LOG.md`。
3. 研究引用应写清原始来源、日期范围、调整方式、缺失值规则和 checksum。
4. 一次性 push 执行稿完成后应合并进当前计划或移出公开跟踪，避免新贡献者按过期步骤操作。
