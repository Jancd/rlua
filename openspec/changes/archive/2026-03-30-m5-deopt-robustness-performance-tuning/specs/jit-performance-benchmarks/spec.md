## Capability: jit-performance-benchmarks (NEW)

M5 introduces a dedicated benchmark harness for the supported tracing-JIT workload set so interpreter-vs-JIT speedup can be measured and tuned explicitly instead of inferred from ad hoc local runs.

## ADDED Requirements

### Requirement: JIT Benchmark Harness
The project MUST provide a benchmark harness that can execute the M5-supported hot-loop workload set in both interpreter-only and JIT-enabled modes under comparable host configuration.

#### Scenario: Benchmark runs both execution modes
- **WHEN** a developer runs the benchmark harness for the supported JIT workload set
- **THEN** each benchmark case is executed at least once in interpreter-only mode
- **AND** the same case is executed in JIT-enabled mode using the same script inputs and host controls

#### Scenario: Workload set is explicit
- **WHEN** benchmark cases are enumerated for M5 validation
- **THEN** the harness reports the workload names included in the supported traced subset
- **AND** those workloads are reusable across repeated benchmark runs

### Requirement: Benchmark Result Reporting
The benchmark harness MUST report per-workload timings and aggregate speedup relative to interpreter baseline for the supported workload set.

#### Scenario: Speedup summary is produced
- **WHEN** a benchmark run completes successfully
- **THEN** the harness reports a timing result for each workload in interpreter-only and JIT-enabled modes
- **AND** the harness reports an aggregate median speedup for the workload set

#### Scenario: Slow benchmark is inspectable
- **WHEN** a benchmark workload does not achieve the expected speedup
- **THEN** the reported output includes enough identifying information to locate the slow case
- **AND** the run can be correlated with JIT debug counters or diagnostics collected for that workload
