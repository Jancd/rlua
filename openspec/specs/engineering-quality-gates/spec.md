## Capability: engineering-quality-gates (MODIFIED)

Test strategy extended with differential testing layer against reference Lua 5.1.

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
The project MUST maintain test layering for unit, integration, differential, and JIT-specific validation.

#### Scenario: Test tree inspection
- **WHEN** repository tests are enumerated
- **THEN** directories or modules exist for conformance, differential, JIT, and fuzz-oriented tests

### Requirement: Diagnostics Feature Flags
The runtime MUST support feature-gated diagnostics for trace logging, IR dumps, optimizer activity, native code generation, JIT counters, and replay/side-exit instrumentation.

#### Scenario: Diagnostics disabled by default
- **WHEN** runtime is built with default features
- **THEN** diagnostics paths are inactive unless explicitly enabled

#### Scenario: JIT diagnostics enabled
- **WHEN** JIT diagnostics features are enabled for a build or test run
- **THEN** the runtime can emit hot-loop detection, trace recording, optimization, native code generation, replay, and side-exit events without changing default runtime behavior

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
