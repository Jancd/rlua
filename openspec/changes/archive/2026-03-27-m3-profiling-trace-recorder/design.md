## Context

当前仓库已经完成 M2 级别的解释器实现与语义加固，`rlua-vm` 能跑较完整的 Lua 5.1 子集，`tests/conformance` 和 `tests/differential` 也已建立。与 JIT 相关的部分仍停留在骨架阶段：`rlua-jit` 只有 `JitRuntime`、`TraceRecorder`、`CodeGenerator`、`Deoptimizer` trait stub，`rlua-ir` 只提供最小 `Trace`/`GuardType` 结构，`tests/jit/` 仍为空目录。

M3 的目标不是提前做机器码后端，而是建立一条闭环路径：
解释器热点探测 -> trace 录制 -> trace 缓存 -> replay 执行 -> guard failure 回退解释器。

核心约束：
- 解释器仍然是语义真源，trace replay 只是更快到达 M4/M5 的中间验证层。
- 不引入新的重量级依赖，继续遵守 std-only / 最小依赖策略。
- M3 必须为后续 codegen/deopt 铺路，但不能把 x86_64 backend 提前塞进本次 change。

## Goals / Non-Goals

**Goals:**
- 在 `rlua-vm` 中增加可配置的 loop hotness profiling，并把热点循环交给 JIT runtime。
- 在 `rlua-jit` / `rlua-ir` 中定义可序列化、可缓存、可 replay 的 trace 数据结构。
- 建立最小 replay executor，使 recorded trace 能在不生成机器码的情况下验证语义正确性。
- 为 guard failure、trace cache、诊断开关和 JIT 配置加上测试与最小 CLI 接口。

**Non-Goals:**
- 本次不实现 x86_64 机器码生成。
- 本次不追求性能指标或 benchmark 结论，只验证闭环正确性。
- 本次不尝试完整 deopt map 压缩、寄存器分配或高级 trace 优化。

## Decisions

### Decision 1: 用 replay executor 作为 M3 的执行后端
- Rationale: M3 的关键是验证 recorder 输出是否保持解释器语义；replay executor 能在不引入机器码内存管理和 backend 复杂度的前提下完成这个目标。
- Alternatives:
  - 直接进入 native codegen：会把问题从“trace 语义正确吗”混成“codegen 正确吗”，调试面过大。
  - 只记录 trace 不执行：无法验证 recorder 输出是否真正可消费，阶段价值不足。

### Decision 2: 热点探测放在 VM 的 loop back-edge 上
- Rationale: 当前解释器主循环已经掌握 PC、跳转和调用边界，在 backward jump 处计数最直接，也最符合主规格中“hot loop header”定义。
- Alternatives:
  - 每条字节码都计数：开销更高，且对 M3 目标没有额外收益。
  - 在 compiler 阶段静态标记热点：不能反映运行时热度。

### Decision 3: trace 先以 bytecode/slot 级元数据建模
- Rationale: 现有 `rlua-ir` 很轻，M3 只需要能表达“读取哪个 slot、做了什么假设、失败时回到哪里”。先把 source pc、slot 和 guard exit id 固化下来，后续再映射到机器码寄存器。
- Alternatives:
  - 直接设计接近机器码的 IR：会让 M3 过早背负 M4 的约束。
  - 继续停留在 `Trace { ops: Vec<IrOp> }` 的极简结构：不足以支撑 replay 和 side exits。

### Decision 4: trace cache 以 `(function, loop_header_pc)` 作为 key
- Rationale: 当前 VM 执行模型天然围绕 closure/proto + pc 组织；用函数原型和 loop header 作为 key 足够表达 M3 的单入口 loop trace。
- Alternatives:
  - 仅用 `pc` 作为 key：跨函数冲突风险高。
  - 提前支持多入口或 trace tree：超出 M3 范围。

### Decision 5: 配置面保持最小化，但必须贯穿 CLI 到 runtime
- Rationale: M3 需要能稳定开启/关闭 JIT、调整 hot threshold、开启 diagnostics；这些入口如果只停留在库内部，测试和验证都会变得绕。
- Alternatives:
  - 完全不暴露 CLI 配置：阶段切换不可观测，测试场景也难驱动。

## Risks / Trade-offs

- [Replay executor 与未来 native backend 行为漂移] → 通过让 replay 和 future codegen 共享 trace/exit metadata 结构，降低分叉风险。
- [热点探测与现有 VM 主循环耦合过深] → 将 profiling 状态封装到独立结构，避免把 recorder 逻辑散落到 opcode 分支中。
- [Guard 设计过窄，后续不够用] → M3 先覆盖类型/shape 级 guard，并为 guard id + resume pc 预留扩展字段。
- [新增 CLI 和测试矩阵导致维护成本上升] → 限定 M3 只暴露少量配置项，并优先用 feature flag 驱动诊断而不是默认输出。

## Migration Plan

1. 扩展 `rlua-ir` 与 `rlua-jit` 的 trace/guard/exit metadata 结构，并补基础单元测试。
2. 在 `rlua-vm` 接入 loop hotness profiling 与 trace lookup/record trigger。
3. 实现 replay executor 和 side-exit fallback，先跑通最小热循环脚本。
4. 在 `rlua-cli` 接入 JIT 开关与阈值配置，并补 `tests/jit/` 等价性与 guard failure 回归。
5. 运行 `openspec validate`、`cargo test`、必要的 feature-flag 测试，确认 M3 开工基线成立。

回滚策略：
- replay executor、profiling、CLI 配置项都以小步提交引入，可按 crate 独立回滚。
- 若 replay 路径不稳定，可保留 recorder 数据结构和 profiling 接口，暂时强制所有热点继续走解释器。

## Open Questions

- M3 的第一版 trace recorder 是否只覆盖数值算术 + 循环控制，再逐步扩到 table/metatable 相关操作？
- hot loop 计数器是否需要在 trace 安装后衰减/冻结，还是简单地跳过已缓存 header 即可？
- `tests/jit/` 是否直接比较 stdout + 返回值即可，还是需要补更细的状态快照工具？
