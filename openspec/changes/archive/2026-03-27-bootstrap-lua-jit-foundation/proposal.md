## Why

当前仓库只有顶层项目规格说明，尚未进入可执行的工程实施阶段。需要把宏观目标拆分为可验证能力、技术决策和任务清单，并启动首批可落地实现。

## What Changes

- 新增 Lua JIT 项目的 OpenSpec 变更，用于驱动从“规格”到“实现”的闭环。
- 建立三个能力域规格：运行时基础、Tracing JIT 执行管线、工程质量门禁。
- 明确依赖最小化策略：核心运行时优先使用 Rust 标准库，第三方依赖仅在必要边界引入。
- 生成可追踪任务列表并启动首批实施（M0/M1 骨架）。

## Capabilities

### New Capabilities
- `lua-runtime-foundation`: 定义 Lua 5.1 子集解释执行基线、字节码模型、值表示和 GC 基础约束。
- `tracing-jit-execution-pipeline`: 定义热点探测、trace 录制、IR 优化、代码生成、deopt 语义要求。
- `engineering-quality-gates`: 定义最小依赖策略、CI 门禁、测试层次和可观测性要求。

### Modified Capabilities
- None.

## Impact

- 新增 OpenSpec 变更目录及能力规格文档。
- 将新增 Rust Cargo workspace 与多 crate 骨架代码。
- 将新增基础测试和 CI 工作流。
- 后续实现将以 tasks.md 为唯一执行跟踪入口。
