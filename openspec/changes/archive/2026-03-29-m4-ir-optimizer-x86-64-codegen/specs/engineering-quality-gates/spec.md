## Capability: engineering-quality-gates (MODIFIED)

M4 将质量门禁从 recorder/replay 正确性扩展到 optimizer、codegen 和 native trace execution 的正确性。

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Diagnostics Feature Flags
The runtime MUST support feature-gated diagnostics for trace logging, IR dumps, optimizer activity, native code generation, JIT counters, and replay/side-exit instrumentation.

#### Scenario: Diagnostics disabled by default
- **WHEN** runtime is built with default features
- **THEN** diagnostics paths are inactive unless explicitly enabled

#### Scenario: JIT diagnostics enabled
- **WHEN** JIT diagnostics features are enabled for a build or test run
- **THEN** the runtime can emit hot-loop detection, trace recording, optimization, native code generation, replay, and side-exit events without changing default runtime behavior
