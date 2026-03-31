## Capability: engineering-quality-gates (MODIFIED)

M6 extends the quality bar from milestone-local validation to an explicit release-candidate sweep that combines conformance, regressions, inspection tooling, and benchmark acceptance criteria.

### Requirement: Differential Test Harness

ADDED: A test harness that runs Lua scripts against both rlua and reference Lua 5.1, comparing their outputs for equivalence.

#### Scenario: Matching output

WHEN a test Lua script is executed by both rlua and reference Lua 5.1
THEN both produce identical stdout output
AND the test passes

#### Scenario: Divergent output

WHEN rlua produces different output than reference Lua 5.1 for a test script
THEN the differential test fails with a clear diff showing expected vs actual

#### Scenario: Harness location

WHEN differential tests are run
THEN they are located in `tests/differential/` and integrated into `cargo test`

### Requirement: Expanded Conformance Test Suite

ADDED: Conformance tests covering all new M2 features — metatables, standard library functions, and edge cases.

#### Scenario: Metatable conformance tests

WHEN `cargo test` is run
THEN tests covering setmetatable/getmetatable, arithmetic metamethods, comparison metamethods, __index/__newindex chaining, __tostring, __concat, __len, __call all pass

#### Scenario: String library conformance tests

WHEN `cargo test` is run
THEN tests covering string.sub, string.find, string.match, string.gmatch, string.gsub, string.format, string.byte, string.char, string.rep, string.reverse, string.lower, string.upper, string.len all pass

#### Scenario: Math library conformance tests

WHEN `cargo test` is run
THEN tests covering math.floor, math.ceil, math.abs, math.sqrt, math.sin, math.cos, math.tan, math.log, math.exp, math.max, math.min, math.random, math.pi, math.huge, math.fmod, math.modf, math.deg, math.rad all pass

#### Scenario: Table library conformance tests

WHEN `cargo test` is run
THEN tests covering table.insert, table.remove, table.sort, table.concat all pass

#### Scenario: Error handling conformance tests

WHEN `cargo test` is run
THEN tests covering xpcall with handler, error object propagation, error with level parameter all pass

#### Scenario: Tail call conformance tests

WHEN `cargo test` is run
THEN tests confirming tail call optimization (deep recursion without stack overflow) pass

### Requirement: Coroutine and Library Gap Compatibility Coverage
The project MUST add conformance, differential, and regression coverage for coroutine semantics and the remaining library/metamethod gap closures.

#### Scenario: Conformance suite covers coroutine lifecycle
- **WHEN** the conformance suite is executed
- **THEN** it includes cases for `coroutine.create`, `coroutine.resume`, `coroutine.yield`, `coroutine.status`, `coroutine.running`, and `coroutine.wrap`
- **AND** it verifies main-thread yield errors plus suspended/dead coroutine behavior

#### Scenario: Differential suite covers closed semantic gaps
- **WHEN** differential tests are run against reference Lua 5.1
- **THEN** they include coroutine resume/yield behavior, `table.sort` with Lua comparator functions, and Lua-closure `__tostring` behavior
- **AND** divergent observable results fail the suite

#### Scenario: JIT-enabled execution remains semantically safe
- **WHEN** the newly closed coroutine or library/metamethod paths are exercised with JIT enabled
- **THEN** execution remains interpreter-equivalent for the supported subset
- **AND** unsupported JIT interactions fall back safely without changing Lua-visible behavior

### Requirement: Minimal Dependency Policy Enforcement
Core runtime crates MUST default to standard library only and justify any external dependency with documented rationale.

#### Scenario: External dependency proposed for core crate
- **WHEN** a dependency is added to a core runtime crate
- **THEN** a rationale is recorded in project documentation with isolation boundary and risk notes

### Requirement: Required CI Gates
The project MUST provide automated checks for formatting, linting, and tests on each change.

#### Scenario: CI validation run
- **WHEN** CI is triggered for a branch
- **THEN** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` are executed

### Requirement: Layered Test Strategy
The project MUST maintain test layering for unit, integration, differential, JIT-specific validation, inspection-oriented validation, and benchmark-oriented validation.

#### Scenario: Test tree inspection
- **WHEN** repository tests, inspection entrypoints, and benchmark assets are enumerated
- **THEN** directories, modules, or documented harness entrypoints exist for conformance, differential, JIT, inspection-oriented, and benchmark-oriented validation

### Requirement: Diagnostics Feature Flags
The runtime MUST support feature-gated diagnostics for trace logging, IR dumps, optimizer activity, native code generation, deoptimization events, trace invalidation events, JIT counters, and replay/side-exit instrumentation.

#### Scenario: Diagnostics disabled by default
- **WHEN** runtime is built with default features
- **THEN** diagnostics paths are inactive unless explicitly enabled

#### Scenario: JIT diagnostics enabled
- **WHEN** JIT diagnostics features are enabled for a build or test run
- **THEN** the runtime can emit hot-loop detection, trace recording, optimization, native code generation, deoptimization, invalidation, replay, and side-exit events without changing default runtime behavior

### Requirement: JIT Replay Equivalence Tests
The project MUST add JIT-focused tests that compare interpreter-only execution with M3 trace replay execution on the same hot loops.

#### Scenario: Replay matches interpreter
- **WHEN** a hot-loop script is executed once with JIT replay enabled and once with JIT disabled
- **THEN** both runs produce identical observable Lua results
- **AND** any captured stdout output matches exactly

#### Scenario: JIT suite location
- **WHEN** M3 JIT tests are enumerated
- **THEN** replay-equivalence cases are located under `tests/jit/`
- **AND** they are runnable through `cargo test`

### Requirement: Guard Failure Fallback Tests
The project MUST cover guard-failure side exits with regression tests that prove execution returns to the interpreter correctly.

#### Scenario: Guard failure resumes in interpreter
- **WHEN** a JIT replay test mutates value shape or type so that a recorded guard fails
- **THEN** execution exits to the interpreter
- **AND** the script still completes with interpreter-equivalent results

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
The project MUST maintain a benchmark-driven validation path for the supported JIT workload set and use it as a release-candidate readiness signal rather than only a milestone-local tuning aid.

#### Scenario: Benchmark suite compares interpreter and JIT
- **WHEN** the RC benchmark validation path is executed
- **THEN** it runs the supported workload set in interpreter-only and JIT-enabled modes
- **AND** it reports per-workload timings plus an aggregate median speedup for release review

#### Scenario: Benchmark acceptance criteria are evaluated
- **WHEN** release-candidate performance validation is reviewed
- **THEN** the reported benchmark results can be compared against documented acceptance criteria for the supported workload set
- **AND** workloads that miss the documented expectation are treated as release-triage inputs rather than ignored local noise

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

### Requirement: Trace Optimizer Tests
The project MUST add tests that verify optimization passes preserve observable trace semantics and leave unsupported trace regions safe for fallback execution.

#### Scenario: Optimizer preserves supported trace behavior
- **WHEN** an optimization pass rewrites a supported trace
- **THEN** tests confirm the optimized trace produces the same observable behavior as the input trace

#### Scenario: Unsupported region remains safe
- **WHEN** a trace contains operations not rewritten by the optimizer
- **THEN** tests confirm those regions remain intact or explicitly fall back without semantic drift

### Requirement: x86_64 Backend Tests
The project MUST add unit and integration tests that validate x86_64 code emission and native trace installation behavior.

#### Scenario: Codegen unit coverage
- **WHEN** backend unit tests are run on a development machine
- **THEN** instruction encoding, executable buffer setup, and trace installation paths are validated

#### Scenario: Native trace smoke execution
- **WHEN** JIT smoke tests run on x86_64 with native backend enabled
- **THEN** at least one supported hot-loop trace executes through the native path and matches interpreter-visible results
