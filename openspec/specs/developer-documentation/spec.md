## Capability: developer-documentation (NEW)

M7C adds a repository-facing documentation surface so contributors can understand the validated project scope, architecture, and workflow without relying on private context.

### Requirement: Repository Entry Documentation
The repository MUST provide a top-level documentation entrypoint that explains what rlua is, which runtime and JIT surface is validated, and where contributors should look next for deeper documentation.

#### Scenario: README provides project overview
- **WHEN** a user opens the repository root
- **THEN** a top-level `README.md` describes the interpreter and tracing-JIT project at a high level
- **AND** it summarizes the validated support surface without implying unsupported behavior is available

#### Scenario: README links to deeper docs
- **WHEN** a contributor reads the top-level repository documentation
- **THEN** it links to architecture, release-facing limitations, and contributor workflow documents
- **AND** those links point to repository-local sources of truth rather than external, unpublished guidance

### Requirement: Architecture Documentation
The project MUST provide architecture documentation that explains crate boundaries, major execution flows, and the validated interpreter/JIT layering.

#### Scenario: Architecture doc explains crate responsibilities
- **WHEN** a contributor reads `docs/architecture.md`
- **THEN** the document identifies the purpose and boundaries of the major crates and modules
- **AND** it explains how parser, compiler, VM, stdlib, IR, and JIT layers fit together

#### Scenario: Architecture doc reports supported and unsupported boundaries
- **WHEN** architecture documentation discusses runtime behavior
- **THEN** it distinguishes validated supported paths from intentionally unsupported or fallback-only paths
- **AND** it remains consistent with release-facing limitation reporting

### Requirement: Contributor Workflow Documentation
The project MUST provide contributor-facing workflow documentation for setup, validation, and change management.

#### Scenario: Contributor guide lists required validation
- **WHEN** a contributor prepares a code change
- **THEN** `CONTRIBUTING.md` lists the required local validation commands and repository conventions
- **AND** it distinguishes required checks from longer-running hardening workflows

#### Scenario: Contributor guide covers repository workflow
- **WHEN** a contributor follows the documented workflow
- **THEN** the guide explains how to work with repository-native practices such as OpenSpec changes, tests, and release-validation entrypoints
- **AND** it gives enough information to contribute without private tribal knowledge
