## Context

`openspec/specs/spec.md` 已定义 Lua JIT 的全局目标，但当前代码仓库尚无 Rust workspace 和实现代码。为了降低风险，采用“解释器优先、JIT 渐进启用”的分阶段架构：先建立可验证的运行时骨架与接口边界，再逐步填充语义与优化。

核心约束：
- 尽量不引入第三方库。
- 将 `unsafe` 限定在可审计边界（可执行内存与低层编码）。
- 以 OpenSpec tasks 作为推进节奏控制。

## Goals / Non-Goals

**Goals:**
- 落地 Rust workspace 与多 crate 架构骨架。
- 建立宏驱动 opcode 单一事实源和最小解释器执行路径。
- 明确 JIT 管线接口（profiler/recorder/IR/codegen/deopt）并提供可编译 stub。
- 建立最低 CI/测试门禁，确保后续迭代可回归。

**Non-Goals:**
- 本次变更不实现完整 Lua 5.1 语义。
- 本次变更不实现完整机器码后端。
- 本次变更不追求性能指标达成，仅搭建实施地基。

## Decisions

### Decision 1: 采用多 crate workspace（而非单 crate）
- Rationale: 直接映射架构边界，避免后续大规模拆分重构。
- Alternatives:
  - 单 crate + module：初期简单，但后续边界演进成本高。

### Decision 2: opcode 采用 `macro_rules!` 生成
- Rationale: 满足“元编程优先 + 最小依赖”，减少重复定义和维护错误。
- Alternatives:
  - 手写 enum 和分发表：易漂移。
  - proc-macro：复杂度更高，当前收益不足。

### Decision 3: JIT 先定义 trait/interface + stub
- Rationale: 让 VM 与 JIT 解耦，先保证 interpreter 路径可运行。
- Alternatives:
  - 直接做端到端 JIT：阻塞面大，回归与定位困难。

### Decision 4: CI 先固定三项硬门禁
- Rationale: 早期统一质量基线，避免后续技术债。
- Alternatives:
  - 仅本地约定不设门禁：难以保证一致性。

## Risks / Trade-offs

- [初期骨架较多、功能较少] → 通过 tasks 明确“骨架完成定义”，并快速进入解释器语义增量。
- [过早模块化导致样板代码增加] → 通过共享核心 crate 和统一命名规则降低样板摩擦。
- [JIT stub 与未来真实实现偏离] → 在 specs 中固定接口语义（guard/deopt/resume），后续实现必须满足。

## Migration Plan

1. 创建 workspace 与 crate 结构，确保全仓可编译。
2. 引入 opcode 宏与最小 VM 执行循环 stub。
3. 接入 JIT pipeline stub 与 profiler hook。
4. 增加测试目录与 CI 工作流。
5. 后续 change 继续填充 parser/compiler/vm 语义。

回滚策略：
- 每一步为独立小提交，可按 crate 或工作流文件回滚。
- 若 JIT 接口影响过大，可保留 trait 并禁用具体实现路径。

## Open Questions

- Lua parser 手写实现是否先支持表达式子集再扩展到完整语句级语法？
- v1 differential harness 采用系统 Lua 解释器进程对比还是内嵌基线对比？
- 首个 JIT 可执行后端中，是否在 x86_64 汇编码阶段引入最小化辅助 crate？
