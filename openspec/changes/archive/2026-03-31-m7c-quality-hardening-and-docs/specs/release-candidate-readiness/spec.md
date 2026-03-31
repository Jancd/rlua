## MODIFIED Requirements

### Requirement: Release Candidate Scope Documentation
The project MUST provide versioned, repository-visible release-candidate documentation that describes the supported runtime and JIT subset for `v1.0.0-rc`.

#### Scenario: Supported subset is documented
- **WHEN** a contributor reviews the release-candidate documentation
- **THEN** the docs enumerate the supported interpreter behaviors, JIT-supported workload shape, and architecture assumptions for the candidate release
- **AND** the described support surface matches the subset validated by the repository test and benchmark workflow

#### Scenario: Unsupported behavior is reported as a limitation
- **WHEN** behavior remains intentionally out of scope at RC time
- **THEN** the documentation lists that behavior under known limitations or unsupported cases
- **AND** the repository does not imply support for that behavior through release-facing text

#### Scenario: Public docs align with RC scope
- **WHEN** top-level repository docs and architecture/contributor docs describe the project surface
- **THEN** they remain consistent with the release-candidate support and limitation reporting
- **AND** contributors can navigate from those docs to the versioned RC guidance without ambiguity

### Requirement: Release Candidate Validation Guidance
The project MUST document a repository-native validation path and acceptance criteria for the release candidate.

#### Scenario: Validation path is explicit
- **WHEN** a contributor prepares to validate the release candidate
- **THEN** the release-facing docs identify the commands or entrypoints for conformance, regression, trace inspection, and benchmark validation
- **AND** the docs explain which checks define release-candidate readiness

#### Scenario: Benchmark expectations are documented
- **WHEN** the release-candidate benchmark guidance is reviewed
- **THEN** it identifies the supported workload set used for RC evaluation
- **AND** it explains how benchmark output is interpreted as pass, investigate, or fail for the candidate release

#### Scenario: Required and extended validation lanes are documented
- **WHEN** release-facing validation guidance is reviewed
- **THEN** it distinguishes ordinary required validation from deeper hardening workflows such as fuzz-oriented execution
- **AND** the guidance remains aligned with contributor-facing validation documentation
