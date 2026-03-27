## Why

M2 已经把解释器基线语义、GC、metatable、标准库和 differential testing 补齐，当前仓库具备进入 JIT 主线增量的条件。现有 `tracing-jit-execution-pipeline` 规格仍停留在接口与目标级描述，代码中也只有 `rlua-jit`/`rlua-ir` stub；要正式开启 M3，需要把“热点探测 + trace record + 解释器等价 replay”收敛成可实施、可验证的 change。

## What Changes

- 在 `rlua-vm` 中实现可配置的热点循环计数与 loop header 探测，形成从解释器到 trace recorder 的触发路径
- 在 `rlua-jit` 和 `rlua-ir` 中把现有 trait/stub 落地为最小可运行的 trace recording pipeline，包括 trace 元数据、guard 表达和 side-exit 恢复点
- 增加 interpreter-equivalent trace replay 路径，在不引入机器码后端的前提下验证 recorder 输出与解释器语义一致
- 扩展 CLI / diagnostics 配置面，支持启停 JIT、设置 hot threshold，并在 feature flag 下输出 trace / guard / replay 诊断信息
- 为 `tests/jit/` 补齐 M3 级别测试，覆盖热点探测、guard 失败回退和 trace replay 等价性

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `tracing-jit-execution-pipeline`: 将现有接口级要求收敛为 M3 可交付要求，明确 hot-loop profiling、trace recording、guard/side-exit 元数据，以及 interpreter-equivalent replay 行为
- `engineering-quality-gates`: 增加针对 M3 的 JIT 测试与诊断门禁，要求 `tests/jit/` 覆盖热点探测、trace replay 和 guard failure 回退场景

## Impact

- `crates/rlua-vm`: 需要接入热度计数、loop header 标记、trace 触发与 replay 切换点
- `crates/rlua-jit`: 需要从接口 stub 升级为可记录 trace、维护 trace cache / exit metadata 的最小运行时
- `crates/rlua-ir`: 需要扩展 trace/guard IR 数据结构，支持 recorder 输出和 replay 消费
- `crates/rlua-cli`: 需要暴露 JIT 开关和阈值等最小运行参数
- `tests/jit` 与相关单元测试: 需要新增 recorder、replay、guard exit 的等价性与回退验证
