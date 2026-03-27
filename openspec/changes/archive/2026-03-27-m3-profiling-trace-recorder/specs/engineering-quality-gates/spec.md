## Capability: engineering-quality-gates (MODIFIED)

M3 将质量门禁从“解释器正确性”扩展到“recorder/replay 正确性”和相关诊断能力。

## ADDED Requirements

### Requirement: JIT Replay Equivalence Tests
The project MUST add JIT-focused tests that compare interpreter-only execution with M3 trace replay execution on the same hot loops.

#### Scenario: Replay matches interpreter
WHEN a hot-loop script is executed once with JIT replay enabled and once with JIT disabled
THEN both runs produce identical observable Lua results
AND any captured stdout output matches exactly

#### Scenario: JIT suite location
WHEN M3 JIT tests are enumerated
THEN replay-equivalence cases are located under `tests/jit/`
AND they are runnable through `cargo test`

### Requirement: Guard Failure Fallback Tests
The project MUST cover guard-failure side exits with regression tests that prove execution returns to the interpreter correctly.

#### Scenario: Guard failure resumes in interpreter
WHEN a JIT replay test mutates value shape or type so that a recorded guard fails
THEN execution exits to the interpreter
AND the script still completes with interpreter-equivalent results

## MODIFIED Requirements

### Requirement: Diagnostics Feature Flags
The runtime MUST support feature-gated diagnostics for trace logging, IR dumps, JIT counters, and M3 replay/side-exit instrumentation.

#### Scenario: Diagnostics disabled by default
WHEN the runtime is built with default features
THEN diagnostics paths are inactive unless explicitly enabled

#### Scenario: JIT diagnostics enabled
WHEN JIT diagnostics features are enabled for a build or test run
THEN the runtime can emit hot-loop detection, trace recording, replay, and side-exit events without changing default runtime behavior
