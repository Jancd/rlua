## Capability: tracing-jit-execution-pipeline (MODIFIED)

M5 hardens the tracing JIT pipeline around deoptimization, side-exit stability, and trace invalidation so native execution can remain interpreter-correct under repeated exits and stale-trace conditions.

## ADDED Requirements

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

## MODIFIED Requirements

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

### Requirement: Native Trace Execution
The VM MUST prefer native trace execution only for active, non-invalidated compiled traces and MUST fall back to replay or interpreter execution when native execution is unavailable, downgraded, or invalidated.

#### Scenario: Active native trace is executed
- **WHEN** a hot loop has a compiled native trace artifact and the runtime marks that trace active for native entry
- **THEN** the VM enters the native trace path instead of replay for that loop

#### Scenario: Invalidated or downgraded trace falls back
- **WHEN** a cached trace has no active native artifact or runtime policy has downgraded native entry for that trace
- **THEN** the runtime continues with replay or interpreter execution
- **AND** the fallback preserves interpreter-visible Lua behavior
