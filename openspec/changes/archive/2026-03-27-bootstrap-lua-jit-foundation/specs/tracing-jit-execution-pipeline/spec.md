## ADDED Requirements

### Requirement: Hot Loop Profiling
The VM MUST collect execution counters and detect hot loop headers using configurable thresholds.

#### Scenario: Loop becomes hot
- **WHEN** a loop header counter exceeds the configured threshold
- **THEN** the runtime schedules or triggers trace recording for that loop

### Requirement: Trace Recording with Guards
The JIT pipeline MUST record linear traces with explicit guards for runtime assumptions.

#### Scenario: Dynamic type assumption is recorded
- **WHEN** trace recorder encounters a value-dependent operation
- **THEN** the trace includes a guard that validates the required runtime type before continuing

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
