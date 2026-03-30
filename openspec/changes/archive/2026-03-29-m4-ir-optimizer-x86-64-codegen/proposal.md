## Why

M3 已经把热点探测、trace recording、trace cache 和 interpreter-equivalent replay 跑通，JIT 管线从“空接口”推进到了“可验证闭环”。下一阶段的瓶颈不再是能不能录 trace，而是这些 trace 仍然只在解释器语义层 replay，无法兑现 M4 要求的优化收益和端到端 JIT 执行路径。

## What Changes

- 在 `rlua-ir` 中把当前 trace 表达扩展为可优化的 IR 形态，并实现 M4 所需的基础优化 passes
- 在 `rlua-jit` 中引入最小 x86_64 backend、机器码缓冲和可执行 trace cache，把受支持的 hot loop trace 编译为 native trace
- 在 `rlua-vm` 中接入 native trace 执行入口，使已编译 trace 优先于 replay 路径运行，并在不支持的平台或不支持的 trace 上保持回退行为
- 扩展 JIT diagnostics 和测试层，覆盖 IR 优化、codegen 编码正确性、x86_64 smoke execution 与 native-vs-interpreter 等价性
- 为 M5 的 deopt correctness 和性能调优预留稳定接口，但本次不追求完整性能目标达成

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `tracing-jit-execution-pipeline`: 从 recorder/replay 阶段推进到 optimizer + native x86_64 backend 阶段，要求受支持 trace 能被优化、编译并以 native path 执行
- `engineering-quality-gates`: 增加 M4 所需的 IR/codegen/native-execution 测试与诊断门禁，确保 backend 改动可回归

## Impact

- `crates/rlua-ir`: 需要补 IR op 语义、pass 管线和优化前后的一致性约束
- `crates/rlua-jit`: 需要新增 x86_64 emitter、可执行内存管理、native trace artifact 和 codegen 入口
- `crates/rlua-vm`: 需要把 native trace 执行接入现有热循环入口，并保留 replay/interpreter fallback
- `tests/jit` 与相关单元测试: 需要新增 optimizer、encoder、native trace smoke cases 和平台条件测试
- `openspec/specs/*`: 需要把当前 tracing/backend 规格更新到 M4 交付层级
