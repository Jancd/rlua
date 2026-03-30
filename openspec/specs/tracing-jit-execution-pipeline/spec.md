## ADDED Requirements

### Requirement: Hot Loop Profiling
The VM MUST count backward-edge executions by loop header, compare them against a configurable hot threshold, and trigger trace recording without removing the interpreter fallback path.

#### Scenario: Loop becomes hot
- **WHEN** a backward branch reaches a loop header whose counter exceeds the configured threshold
- **THEN** the runtime marks that header as hot
- **AND** trace recording is scheduled or triggered for that loop header

#### Scenario: JIT disabled keeps interpreter mode
- **WHEN** the runtime is configured with JIT disabled
- **THEN** trace recording is not started
- **AND** bytecode execution remains on the interpreter path

### Requirement: Trace Recording with Guards
The JIT pipeline MUST record linear traces rooted at a hot loop header, annotate recorded steps with source bytecode locations, and emit guards plus exit metadata for runtime assumptions.

#### Scenario: Dynamic type assumption is recorded
- **WHEN** the trace recorder encounters a value-dependent operation
- **THEN** the trace includes a guard that validates the required runtime type before continuing
- **AND** the guard carries exit metadata that maps back to the originating bytecode location

#### Scenario: Hot loop trace is cached
- **WHEN** a loop header becomes hot and no cached trace exists for it
- **THEN** the recorder produces a trace rooted at that header
- **AND** the runtime stores the recorded trace for subsequent executions of the same loop header

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

### Requirement: Configurable JIT Policy Controls
The runtime MUST expose configurable JIT enablement and hot-threshold controls that can be set by host configuration or CLI integration.

#### Scenario: Lower threshold makes loop eligible earlier
- **WHEN** the runtime is started with a lower hot-threshold configuration than the default
- **THEN** a loop header becomes eligible for trace recording after fewer executions

### Requirement: Deoptimization Correctness
The runtime MUST reconstruct interpreter-visible state from explicit deoptimization metadata for every supported replay or native side exit and MUST resume from the mapped bytecode location without semantic drift.

#### Scenario: Guard fails during native trace execution
- **WHEN** a guard fails or a supported side exit is taken in compiled trace code
- **THEN** execution exits to the interpreter with restored locals, loop temporaries, and upvalues required by the resume point
- **AND** interpreter execution resumes at the exit metadata's mapped bytecode location

#### Scenario: Guard fails during replay execution
- **WHEN** a replayed trace exits through a guard or supported side-exit path
- **THEN** execution resumes in the interpreter using the same deoptimization contract as the native path
- **AND** observable Lua behavior remains equivalent to interpreter-only execution for that bytecode slice

### Requirement: Targeted Native Backend
The JIT MUST provide an x86_64 backend in M4 that can compile supported optimized traces into executable native artifacts and expose explicit fallback behavior on unsupported architectures.

#### Scenario: Supported x86_64 trace compiles
- **WHEN** the runtime is running on x86_64 and a supported optimized trace is selected for code generation
- **THEN** the backend emits a native trace artifact that can be installed into the trace cache

#### Scenario: Unsupported architecture
- **WHEN** runtime starts on an unsupported CPU architecture for the native backend
- **THEN** native code generation remains unavailable
- **AND** the runtime keeps replay/interpreter execution active for recorded traces

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
The VM MUST prefer native trace execution only for active, non-invalidated compiled traces and MUST fall back to replay or interpreter execution when native execution is unavailable, downgraded, or invalidated.

#### Scenario: Active native trace is executed
- **WHEN** a hot loop has a compiled native trace artifact and the runtime marks that trace active for native entry
- **THEN** the VM enters the native trace path instead of replay for that loop

#### Scenario: Invalidated or downgraded trace falls back
- **WHEN** a cached trace has no active native artifact or runtime policy has downgraded native entry for that trace
- **THEN** the runtime continues with replay or interpreter execution
- **AND** the fallback preserves interpreter-visible Lua behavior

### Requirement: Trace Invalidation Policy
The JIT runtime MUST maintain explicit invalidation state for cached traces and MUST stop entering traces that have been invalidated until they are re-recorded or recompiled.

#### Scenario: Invalidated trace is bypassed
- **WHEN** a cached trace is marked invalid for a loop header
- **THEN** the VM does not enter that trace natively
- **AND** execution continues through replay or interpreter fallback without changing observable Lua behavior

#### Scenario: Invalidated trace becomes eligible again
- **WHEN** an invalidated loop becomes hot again under runtime policy
- **THEN** the runtime may record or compile a replacement trace for that loop header
- **AND** the replacement does not reuse stale invalidation state from the prior trace artifact

### Requirement: Side-Exit Stability Policy
The JIT runtime MUST track repeated side exits per cached trace and MUST apply a deterministic downgrade or invalidation policy when a trace proves unstable.

#### Scenario: Repeated exits downgrade native preference
- **WHEN** a cached trace exceeds the configured side-exit stability threshold
- **THEN** the runtime stops preferring native entry for that trace
- **AND** the runtime continues through replay or interpreter execution until policy allows recompilation or re-entry

#### Scenario: Stable trace keeps native preference
- **WHEN** a cached trace does not exceed the configured side-exit stability threshold
- **THEN** the runtime keeps the trace eligible for native execution
- **AND** normal native entry continues for supported executions of that loop
