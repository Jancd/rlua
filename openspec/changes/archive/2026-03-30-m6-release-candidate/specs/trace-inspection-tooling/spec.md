## Capability: trace-inspection-tooling (NEW)

M6 adds a stable CLI inspection surface so contributors can examine cached trace state, lifecycle transitions, and benchmark-relevant counters without relying on ad hoc Rust test code or feature-gated event logging alone.

## ADDED Requirements

### Requirement: Trace Inspection CLI
The project MUST provide a CLI entrypoint that runs a Lua workload under configurable JIT policy and emits a post-run trace summary derived from stable runtime debug state.

#### Scenario: Inspection run executes the target workload
- **WHEN** a developer invokes the trace inspection CLI for a Lua script
- **THEN** the script runs under the requested JIT policy controls such as enablement and hot-threshold configuration
- **AND** the command emits trace summary output after execution completes

#### Scenario: Inspection works without optional diagnostics
- **WHEN** the runtime is built without feature-gated diagnostics enabled
- **THEN** the trace inspection CLI still reports cached trace state and aggregate counters
- **AND** the output does not require enabling event-stream logging to inspect final runtime state

### Requirement: Human and Machine Readable Reporting
The trace inspection tooling MUST provide a default human-readable summary and an opt-in machine-readable format for the same inspection data.

#### Scenario: Human-readable summary is the default
- **WHEN** a developer runs the trace inspection command without selecting an output format
- **THEN** the command prints a compact text summary of trace lifecycle state, counters, and invalidation details
- **AND** the summary is readable without post-processing

#### Scenario: Structured output is available
- **WHEN** a developer requests machine-readable inspection output
- **THEN** the command emits structured data that preserves per-trace lifecycle state, invalidation reasons, and execution counters
- **AND** the structured output can be consumed by scripts or release-validation tooling

### Requirement: Trace Lifecycle and Fallback Reporting
The trace inspection tooling MUST report enough runtime state to distinguish active, replay-only, and invalidated traces and to correlate slow workloads with fallback behavior.

#### Scenario: Cached trace state is reported
- **WHEN** a workload records or reuses cached traces
- **THEN** the inspection output includes per-trace lifecycle state, generation or identity, native availability, and invalidation reason when present
- **AND** the output identifies whether the trace remained eligible for native entry

#### Scenario: Fallback activity is reported
- **WHEN** a workload exits through replay, side exits, downgrade, or invalidation-driven fallback
- **THEN** the inspection output includes native-entry, replay-entry, and side-exit style counters or summaries
- **AND** a contributor can use that output to correlate the fallback behavior with a slow benchmark or regression case
