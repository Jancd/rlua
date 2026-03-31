## ADDED Requirements

### Requirement: Property-Based and Fuzz Hardening Workflow
The project MUST provide both deterministic property-based validation inside normal test execution and a separate fuzz-oriented hardening lane with documented entrypoints.

#### Scenario: Property-based validation participates in standard test runs
- **WHEN** the required repository test workflow is executed
- **THEN** deterministic property-based tests run as part of the ordinary Rust test suite
- **AND** contributors do not need a separate ad hoc process to get that coverage

#### Scenario: Fuzzing is exposed as an extended hardening lane
- **WHEN** a contributor or release reviewer needs deeper hardening coverage
- **THEN** the repository documentation identifies explicit fuzz-oriented commands, scripts, or harnesses
- **AND** that lane is distinguished from the required per-change validation path

### Requirement: Validation Workflow Documentation
The project MUST document the repository's required and extended validation entrypoints in a contributor-visible form.

#### Scenario: Required and extended checks are distinguishable
- **WHEN** a contributor reviews the documented validation workflow
- **THEN** the repository clearly separates required checks for ordinary changes from longer-running hardening or stress workflows
- **AND** the documented commands map to runnable repository entrypoints

#### Scenario: Validation documentation remains aligned with gates
- **WHEN** validation guidance or automation changes
- **THEN** contributor-facing documentation and quality-gate requirements are updated together
- **AND** the repository does not rely on undocumented CI-only knowledge
