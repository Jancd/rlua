## Capability: engineering-quality-gates (MODIFIED)

M6 extends the quality bar from milestone-local validation to an explicit release-candidate sweep that combines conformance, regressions, inspection tooling, and benchmark acceptance criteria.

## ADDED Requirements

### Requirement: Release Candidate Validation Sweep
The project MUST define a release-candidate validation sweep that combines the supported conformance, regression, inspection, and benchmark checks into one auditable readiness path.

#### Scenario: Release sweep entrypoints are identifiable
- **WHEN** a contributor prepares the RC validation run
- **THEN** the repository exposes documented commands or harness entrypoints for conformance, differential/regression, trace inspection validation, and benchmark validation
- **AND** those entrypoints align with the release-facing validation guidance

#### Scenario: Release sweep failure blocks readiness
- **WHEN** a required RC validation component fails or produces an out-of-policy result
- **THEN** the candidate release is not considered ready
- **AND** the failure must be fixed or reflected in documented limitations before the RC is accepted

## MODIFIED Requirements

### Requirement: Layered Test Strategy
The project MUST maintain test layering for unit, integration, differential, JIT-specific validation, inspection-oriented validation, and benchmark-oriented validation.

#### Scenario: Test tree inspection
- **WHEN** repository tests, inspection entrypoints, and benchmark assets are enumerated
- **THEN** directories, modules, or documented harness entrypoints exist for conformance, differential, JIT, inspection-oriented, and benchmark-oriented validation

### Requirement: Benchmark Validation Gate
The project MUST maintain a benchmark-driven validation path for the supported JIT workload set and use it as a release-candidate readiness signal rather than only a milestone-local tuning aid.

#### Scenario: Benchmark suite compares interpreter and JIT
- **WHEN** the RC benchmark validation path is executed
- **THEN** it runs the supported workload set in interpreter-only and JIT-enabled modes
- **AND** it reports per-workload timings plus an aggregate median speedup for release review

#### Scenario: Benchmark acceptance criteria are evaluated
- **WHEN** release-candidate performance validation is reviewed
- **THEN** the reported benchmark results can be compared against documented acceptance criteria for the supported workload set
- **AND** workloads that miss the documented expectation are treated as release-triage inputs rather than ignored local noise
