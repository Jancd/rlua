## Capability: engineering-quality-gates (MODIFIED)

M5 extends the quality gates from optimizer/codegen/native-smoke correctness to deoptimization robustness, trace invalidation stability, and benchmark-driven speedup validation.

## ADDED Requirements

### Requirement: Deoptimization and Invalidation Regression Tests
The project MUST add regression tests that prove deoptimization and trace invalidation preserve interpreter-equivalent behavior for the M5-supported trace subset.

#### Scenario: Native side exit resumes correctly
- **WHEN** a JIT regression test forces a supported native side exit or guard failure
- **THEN** execution resumes in the interpreter at the mapped bytecode location
- **AND** the script completes with the same observable results as an interpreter-only run

#### Scenario: Invalidated trace stays semantically safe
- **WHEN** a regression test invalidates a cached trace and re-executes the same loop
- **THEN** the runtime bypasses the invalidated trace
- **AND** the script still completes with interpreter-equivalent results

### Requirement: Benchmark Validation Gate
The project MUST maintain a benchmark-driven validation path for the M5-supported JIT workload set.

#### Scenario: Benchmark suite compares interpreter and JIT
- **WHEN** the benchmark validation path is executed
- **THEN** it runs the supported workload set in interpreter-only and JIT-enabled modes
- **AND** it reports per-workload timings plus an aggregate median speedup

#### Scenario: Speedup target is evaluated
- **WHEN** M5 performance validation is reviewed
- **THEN** the reported benchmark results can be used to determine whether the supported workload set meets the target median speedup versus interpreter baseline

## MODIFIED Requirements

### Requirement: Diagnostics Feature Flags
The runtime MUST support feature-gated diagnostics for trace logging, IR dumps, optimizer activity, native code generation, deoptimization events, trace invalidation events, JIT counters, and replay/side-exit instrumentation.

#### Scenario: Diagnostics disabled by default
- **WHEN** runtime is built with default features
- **THEN** diagnostics paths are inactive unless explicitly enabled

#### Scenario: JIT diagnostics enabled
- **WHEN** JIT diagnostics features are enabled for a build or test run
- **THEN** the runtime can emit hot-loop detection, trace recording, optimization, native code generation, deoptimization, invalidation, replay, and side-exit events without changing default runtime behavior

### Requirement: Layered Test Strategy
The project MUST maintain test layering for unit, integration, differential, JIT-specific validation, and benchmark-oriented validation.

#### Scenario: Test tree inspection
- **WHEN** repository tests and benchmark assets are enumerated
- **THEN** directories, modules, or documented harness entrypoints exist for conformance, differential, JIT, fuzz-oriented, and benchmark-oriented validation
