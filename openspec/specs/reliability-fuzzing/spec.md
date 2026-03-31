## Capability: reliability-fuzzing (NEW)

M7C turns the previously promised property-based and fuzz-style hardening work into explicit repository-visible validation entrypoints and retained corpus assets.

### Requirement: Property-Based Validation Harness
The project MUST provide deterministic property-based validation for high-value parser, compiler, and runtime invariants inside the normal Rust test workflow.

#### Scenario: Property tests run under standard test execution
- **WHEN** contributors run the documented workspace test workflow
- **THEN** the property-based validation suite executes as part of ordinary Rust test targets
- **AND** failures are reported through the same repository-native test tooling used for other deterministic checks

#### Scenario: Property tests cover semantic invariants
- **WHEN** property-based validation is reviewed
- **THEN** it includes invariants for parser/compiler/runtime behavior such as expression stability, compilation sanity, or interpreter-visible equivalence constraints
- **AND** the generated cases remain deterministic enough for repeatable CI and local runs

### Requirement: Fuzz Hardening Entry Points
The project MUST provide fuzz-oriented hardening entrypoints for parser/compiler/JIT-adjacent surfaces without making them part of the default per-change gate.

#### Scenario: Fuzz targets are documented and runnable
- **WHEN** a contributor follows the documented hardening workflow
- **THEN** the repository exposes explicit commands or scripts for fuzz-oriented validation
- **AND** those entrypoints identify which surfaces they target, such as lexer/parser input, bytecode/compiler behavior, or trace-formation logic

#### Scenario: Fuzz assets are repository-visible
- **WHEN** fuzzing support is inspected in the repository
- **THEN** `tests/fuzz/` or an equivalent documented location contains corpora, reproducers, or related metadata for the hardening workflow
- **AND** those assets can be used to reproduce or extend prior findings

### Requirement: Fuzz Regression Replay
The project MUST preserve actionable fuzz findings as reproducible regressions.

#### Scenario: Reproducer inputs are retained
- **WHEN** fuzzing discovers a crashing or semantically invalid input
- **THEN** the minimized reproducer can be checked into the repository or otherwise retained in the documented corpus workflow
- **AND** contributors can rerun it deterministically during regression validation
