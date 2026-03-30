## Context

当前仓库已经完成 M3：解释器能识别热循环、录制 trace、缓存 trace，并在 replay executor 中执行受支持的数值循环，同时保留 guard failure 回退解释器的路径。现状的限制也很明确：`rlua-ir` 仍以 trace metadata 和原始 bytecode 为主，`rlua-jit` 只有 recorder/cache/replay 辅助结构，没有 optimizer 和 native backend，`rlua-vm` 也还没有真正的 native trace 执行入口。

M4 需要把 JIT 从“语义闭环”推进到“端到端 native execution”：
hot loop -> recorded trace -> optimized trace -> x86_64 native artifact -> native trace execution -> fallback on failure/unavailability。

核心约束：
- 解释器与 replay 仍然是 correctness baseline，native path 只能在它们之上增量引入。
- 继续遵守最小依赖策略；如果需要可执行内存/页保护接口，应限制在孤立模块内。
- M4 聚焦 optimizer + native backend + VM 集成，不提前完成 M5 的完整 deopt/perf 收尾。

## Goals / Non-Goals

**Goals:**
- 在 `rlua-ir` 中建立足够支撑优化和 codegen 的 trace IR 结构，并实现 M4 基础 passes。
- 在 `rlua-jit` 中实现最小 x86_64 backend、可执行 trace artifact 和 trace cache 安装路径。
- 在 `rlua-vm` 中优先执行 native trace，同时在不支持的平台、编译失败或不支持的 trace 上回退到 replay/interpreter。
- 补齐 optimizer/codegen/native smoke tests 与相关 diagnostics。

**Non-Goals:**
- 本次不实现完整寄存器分配器或通用 backend framework。
- 本次不追求完整 deopt map 覆盖或 trace invalidation 策略收尾。
- 本次不承诺达到 M5 的性能目标，只确保 native execution 路径成立并可验证。

## Decisions

### Decision 1: 保留 replay 作为 native backend 的验证与回退路径
- Rationale: 现有 replay 已经能提供解释器等价的 correctness baseline；M4 应该把 native path 叠加在 replay 之上，而不是替换掉唯一可验证路径。
- Alternatives:
  - 直接把 replay 删除，所有 compiled trace 都必须走 native：一旦 backend 有误，排错成本过高。
  - 继续只做 replay 不接 native：无法真正进入 M4 的端到端执行目标。

### Decision 2: 第一版 native backend 只覆盖受控的数值热循环子集
- Rationale: 当前 recorder/replay 已经在数值 arithmetic + loop control 上稳定；先把这类 trace 编译成 native code，能把 backend 风险限制在最小闭环。
- Alternatives:
  - 一开始覆盖 tables/metatables/calls：会显著放大 backend 复杂度和 deopt 面。
  - 做纯 encoder demo、不和真实 trace 相连：阶段价值不足。

### Decision 3: trace cache 需要同时容纳 replay metadata 和 native artifact
- Rationale: M4 的 cache 既要保存原始 trace/guards/exits，也要保存优化后/native 编译后的工件，VM 才能在同一热循环入口上选择 native 或 replay。
- Alternatives:
  - 分离成两个无关联 cache：会让 invalidation、stats 和 fallback 管理更复杂。

### Decision 4: x86_64 backend 采用内部最小 encoder + executable buffer
- Rationale: 这与项目的最小依赖、可审计 unsafe 边界和元编程优先原则一致；backend 行为也更容易和 trace IR 一一对应。
- Alternatives:
  - 引入大型 assembler/JIT framework：不符合依赖策略，而且会掩盖 codegen 细节。
  - 手写无抽象的裸字节拼接：可维护性和测试性都较差。

### Decision 5: M4 diagnostics 扩展到 optimizer/codegen/native execution
- Rationale: 一旦 native path 接入，只有 trace-recording 级 diagnostics 已不足以定位问题；需要看到优化、编译、安装和执行路径。
- Alternatives:
  - 只保留现有 trace log：native backend 出错时几乎没有定位面。

## Risks / Trade-offs

- [x86_64 backend 只覆盖窄子集] → 通过在 spec/tasks 中明确“supported trace subset”，先把闭环做实，再逐步扩面。
- [可执行内存与 unsafe 边界带来实现风险] → 把 executable buffer/page protection 封装到孤立模块，并为编码与安装路径加单测。
- [optimizer 变换引入语义漂移] → 优化前后都保留 replay/interpreter 对照测试，优先验证 correctness 再扩大变换范围。
- [native path 与 replay path 数据模型分叉] → 让两者共享 trace key、guard/exit metadata 与 counters，降低分叉风险。

## Migration Plan

1. 扩展 `rlua-ir` 以表示可优化、可 codegen 的 trace IR，并补优化 pass 单测。
2. 在 `rlua-jit` 中实现 x86_64 encoder、executable buffer 和 native trace artifact 安装逻辑。
3. 扩展 trace cache，使其同时保存 replay metadata 与 native artifact。
4. 在 `rlua-vm` 中优先执行 native trace，并在不满足条件时回退到 replay/interpreter。
5. 补齐 `tests/jit/` native smoke cases、backend unit tests、diagnostics coverage，并完成 `openspec validate` 与全量测试。

回滚策略：
- optimizer、backend、VM 集成都可以按模块独立关闭或回滚。
- 若 native execution 不稳定，可保留 IR/pass/codegen 代码，但让 VM 强制回到 replay 路径。

## Open Questions

- executable buffer 在 macOS / Linux 上是否统一走 `libc` 封装，还是需要平台拆分实现？
- 第一版 optimizer 是否只做 constant folding + dead code elimination，还是同时纳入局部 CSE？
- native trace smoke suite 是否需要在非 x86_64 平台上增加显式 skip/expect-fallback 断言？
