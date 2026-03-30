## Capability: tracing-jit-execution-pipeline (MODIFIED)

M4 将该能力从 recorder/replay 阶段推进到 optimizer + native x86_64 backend 阶段，同时保留 replay/interpreter 作为验证与回退路径。

## ADDED Requirements

### Requirement: Trace Optimization Passes
The JIT MUST run recorded traces through a deterministic optimization pipeline before native code generation.

#### Scenario: Supported arithmetic trace is optimized
- **WHEN** a supported hot-loop trace is prepared for compilation
- **THEN** the runtime runs the configured M4 optimization passes before code generation
- **AND** the optimized trace remains semantically equivalent to the unoptimized trace

#### Scenario: Unsupported optimization precondition
- **WHEN** a trace contains unsupported operations or metadata for an optimization pass
- **THEN** the pass leaves that portion of the trace unchanged
- **AND** the trace remains eligible for replay fallback

### Requirement: Native Trace Execution
The VM MUST prefer native trace execution when a compiled native artifact is installed for a hot loop and MUST fall back to replay or interpreter execution when native execution is unavailable.

#### Scenario: Native trace is executed
- **WHEN** a hot loop has a compiled native trace artifact on a supported x86_64 runtime
- **THEN** the VM enters the native trace path instead of replay for that loop

#### Scenario: Native trace unavailable
- **WHEN** no native artifact is installed for a hot loop or compilation is skipped
- **THEN** the runtime continues with replay or interpreter execution without changing observable Lua behavior

## MODIFIED Requirements

### Requirement: Interpreter-Equivalent Trace Replay
The runtime MUST retain replay execution for recorded traces as a correctness path and as a fallback when native code generation or native trace execution is unavailable.

#### Scenario: Replay follows the hot path
- **WHEN** a cached trace is available and no native artifact is installed for it
- **THEN** the runtime executes the trace through replay mode
- **AND** observable Lua results remain equivalent to interpreter execution for the same bytecode slice

#### Scenario: Guard fails during replay
- **WHEN** a replayed trace guard fails
- **THEN** the runtime exits replay mode
- **AND** resumes interpreter execution at the exit metadata's mapped bytecode location with restored locals and upvalues

### Requirement: Targeted Native Backend
The JIT MUST provide an x86_64 backend in M4 that can compile supported optimized traces into executable native artifacts and expose explicit fallback behavior on unsupported architectures.

#### Scenario: Supported x86_64 trace compiles
- **WHEN** the runtime is running on x86_64 and a supported optimized trace is selected for code generation
- **THEN** the backend emits a native trace artifact that can be installed into the trace cache

#### Scenario: Unsupported architecture
- **WHEN** runtime starts on an unsupported CPU architecture for the native backend
- **THEN** native code generation remains unavailable
- **AND** the runtime keeps replay/interpreter execution active for recorded traces
