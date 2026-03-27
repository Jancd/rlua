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
Before native code generation exists, the runtime MUST execute recorded traces through a replay executor that preserves interpreter-visible behavior and exits back to the interpreter on guard failure or trace termination.

#### Scenario: Replay follows the hot path
- **WHEN** a cached trace is available and all replay guards hold
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
The runtime MUST reconstruct interpreter-visible state on guard failure and resume from mapped bytecode locations.

#### Scenario: Guard fails during trace execution
- **WHEN** a guard fails in compiled trace code
- **THEN** execution exits to interpreter with restored locals/upvalues and resumes at mapped program counter

### Requirement: Targeted Native Backend
The JIT MUST support at least one native backend (x86_64) in v1 and expose explicit unsupported-target behavior.

#### Scenario: Unsupported architecture
- **WHEN** runtime starts on an unsupported CPU architecture for JIT backend
- **THEN** runtime keeps interpreter mode active and reports that native JIT is unavailable
