## ADDED Requirements

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
The runtime MUST support feature-gated diagnostics for trace logging, IR dumps, and JIT counters.

#### Scenario: Diagnostics disabled by default
- **WHEN** runtime is built with default features
- **THEN** diagnostics paths are inactive unless explicitly enabled
