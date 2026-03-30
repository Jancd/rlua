## Capability: jit-performance-benchmarks (MODIFIED)

M6 carries the benchmark harness forward from performance tuning into release-candidate validation, so its workload set and output can be used directly for RC triage and acceptance decisions.

## MODIFIED Requirements

### Requirement: JIT Benchmark Harness
The project MUST provide a benchmark harness that can execute the supported JIT workload set in both interpreter-only and JIT-enabled modes under comparable host configuration and support targeted workload reruns for triage.

#### Scenario: Benchmark runs both execution modes
- **WHEN** a developer runs the benchmark harness for the supported JIT workload set
- **THEN** each benchmark case is executed at least once in interpreter-only mode
- **AND** the same case is executed in JIT-enabled mode using the same script inputs and host controls

#### Scenario: Workload set is explicit
- **WHEN** benchmark cases are enumerated for release-candidate validation
- **THEN** the harness reports the workload names included in the supported traced subset
- **AND** those workloads remain reusable across repeated benchmark runs

#### Scenario: Slow case can be rerun in isolation
- **WHEN** benchmark review identifies a workload that needs deeper triage
- **THEN** the harness can rerun that workload without running the full suite
- **AND** the isolated rerun uses the same benchmark entrypoint and reporting model as the full suite

### Requirement: Benchmark Result Reporting
The benchmark harness MUST report per-workload timings, aggregate speedup relative to interpreter baseline, and release-oriented status information for the supported workload set.

#### Scenario: Speedup summary is produced
- **WHEN** a benchmark run completes successfully
- **THEN** the harness reports a timing result for each workload in interpreter-only and JIT-enabled modes
- **AND** the harness reports an aggregate median speedup for the workload set

#### Scenario: Target status is surfaced
- **WHEN** benchmark output is generated for RC validation
- **THEN** the harness indicates whether the documented speedup target or acceptance threshold was met
- **AND** the output makes that status visible without requiring manual recomputation

#### Scenario: Slow benchmark is inspectable
- **WHEN** a benchmark workload does not achieve the documented expectation
- **THEN** the reported output includes enough identifying information to locate the slow case
- **AND** the run can be correlated with trace inspection output or JIT debug counters collected for that workload
