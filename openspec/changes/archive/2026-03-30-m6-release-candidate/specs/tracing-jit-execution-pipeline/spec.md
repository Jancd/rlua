## Capability: tracing-jit-execution-pipeline (MODIFIED)

M6 keeps the M5 execution semantics intact while making trace lifecycle, fallback, and invalidation state explicitly observable enough for release-candidate debugging and triage.

## ADDED Requirements

### Requirement: Observable Trace Lifecycle State
The runtime MUST preserve stable, inspectable lifecycle state for cached traces so release-candidate tooling can distinguish active, replay-only, and invalidated traces across recompilation generations.

#### Scenario: Cached trace exposes lifecycle details
- **WHEN** a hot loop records or reuses a cached trace
- **THEN** the runtime retains observable metadata for that trace's lifecycle state, generation or identity, and native-availability status
- **AND** inspection tooling can determine whether the trace is eligible for native entry

#### Scenario: Replacement trace does not hide lineage
- **WHEN** an invalidated trace is replaced by a newly recorded or recompiled trace for the same loop header
- **THEN** the runtime exposes the replacement as a distinct generation or trace identity
- **AND** stale lifecycle state from the prior trace is not reported as if it belonged to the replacement

### Requirement: Observable Fallback and Exit Accounting
The runtime MUST preserve aggregate execution-mode accounting that distinguishes native entry, replay entry, and side-exit or fallback activity for supported trace execution.

#### Scenario: Native and replay entry counts remain visible
- **WHEN** a workload enters cached traces through native execution or replay
- **THEN** the runtime updates observable counters or summaries for each execution mode
- **AND** inspection tooling can report those counts after the workload finishes

#### Scenario: Side exits and invalidation-driven fallback remain visible
- **WHEN** a workload leaves a trace through side exits, downgrade, or invalidation-driven fallback
- **THEN** the runtime preserves observable accounting or summary state for that fallback behavior
- **AND** the final debug state is sufficient to correlate the fallback with regression or benchmark triage
