## Context

M7A closed the last obvious baseline language gaps, which shifts the remaining work from feature delivery to confidence and maintainability. The repository still lacks the public-facing documentation and validation depth already implied by the top-level spec: there is no top-level README or contributor guide, `docs/` only contains the release-candidate document, and `tests/fuzz/` is still an empty placeholder despite the project promising property-based and fuzz-oriented testing.

This change is cross-cutting rather than runtime-semantic. It touches repository structure, validation workflow, contributor experience, and how existing quality gates are described. The design therefore needs to keep three constraints in balance:

- do not widen the supported Lua/JIT surface in M7C
- keep new validation tooling out of runtime dependencies and hot paths
- make the documented workflow match what contributors can actually run locally and in CI

## Goals / Non-Goals

**Goals:**
- Add a stable repository-facing documentation set that covers project overview, architecture, validated support boundaries, and contributor workflow.
- Introduce deterministic property-based validation for core parser/compiler/runtime invariants inside the normal Rust test workflow.
- Introduce a fuzzing workflow for parser/compiler/JIT-adjacent surfaces with documented entrypoints, seed corpora, and expectations.
- Clarify which checks are required for every change versus extended or nightly hardening runs.
- Keep release-facing documentation aligned with the newly added public docs and validation guidance.

**Non-Goals:**
- M7C does not change Lua semantics, widen JIT support, or alter release performance targets.
- M7C does not require full fuzz jobs on every pull request if that would make the default workflow too slow or nondeterministic.
- M7C does not introduce runtime dependencies into core crates for documentation or testing convenience.
- M7C does not replace the existing release-candidate document; it reorganizes the broader doc surface around it.

## Decisions

### Decision 1: Split repository documentation into overview, architecture, contributor workflow, and release-facing docs
- Rationale: one document cannot serve new users, contributors, and release review equally well. The current repository has only `docs/release-candidate.md`, which is too narrow to double as a project overview or contributor guide.
- Design:
  - Add `README.md` as the primary landing page with project scope, workspace layout, validated subset summary, and links to deeper docs.
  - Add `docs/architecture.md` for crate/module boundaries, interpreter/JIT layering, and major execution flows.
  - Add `CONTRIBUTING.md` for local workflow, validation commands, style expectations, and when to use extended validation.
  - Keep `docs/release-candidate.md` as the versioned release-facing support/limitation document, linked from README rather than duplicated into it.
- Alternatives:
  - Put everything into `README.md`: rejected because architecture, contributor workflow, and RC limitations would become hard to maintain and easy to contradict.
  - Keep only `docs/` pages without a root README: rejected because repository entrypoints on GitHub should be discoverable without opening multiple files.

### Decision 2: Property-based tests live inside ordinary Rust test targets; fuzzing remains a separate hardening lane
- Rationale: deterministic property tests belong close to the code they validate and should run under normal `cargo test`; long-running fuzz exploration should stay isolated from per-PR latency and flakiness concerns.
- Design:
  - Add property-based tests as crate-local test modules or integration tests for parser/compiler/runtime invariants.
  - Use test-only dependencies for those checks so runtime crates keep their production dependency surface unchanged.
  - Define fuzzing as a separate harness lane with documented commands, seed corpora, and target ownership for parser/compiler/trace formation surfaces.
  - Keep `tests/fuzz/` as the repository-visible home for fuzz assets such as corpora, reproducers, and documentation, while allowing the actual runner implementation to use the tooling that best fits Rust fuzz workflows.
- Alternatives:
  - Make fuzzing part of default `cargo test`: rejected because fuzz jobs are intentionally open-ended and unsuitable as a deterministic per-PR gate.
  - Put property-based tests into the same long-running fuzz harness: rejected because it would hide fast invariant checks behind heavyweight tooling and reduce local developer adoption.

### Decision 3: Quality gates are tiered into required PR checks and extended hardening checks
- Rationale: the top-level spec wants stronger verification, but requiring every expensive validation mode on every PR would slow normal development and make failures harder to triage.
- Design:
  - Keep `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` as the mandatory short-path checks.
  - Add deterministic property-test coverage into the normal workspace test suite so it naturally participates in those required gates.
  - Define fuzzing, longer corpus replay, and other stress-oriented hardening as explicit extended validation entrypoints documented in contributor docs and quality specs.
  - If helpful, add a wrapper script for the extended lane rather than expecting contributors to memorize multiple commands.
- Alternatives:
  - Make every fuzz target required in CI for every PR: rejected because it couples correctness review to nondeterministic runtime and infrastructure cost.
  - Leave fuzz/property testing purely aspirational in docs: rejected because the current gap exists precisely because those expectations were never turned into actionable repository entrypoints.

### Decision 4: Release-readiness docs should reference the broader documentation set rather than remain a standalone island
- Rationale: release-candidate documentation is already present, but it should not become the only maintained source of truth for support boundaries and validation commands.
- Design:
  - Keep `docs/release-candidate.md` focused on the RC subset, limitations, and benchmark/inspection gate interpretation.
  - Make README and contributor docs point to the RC doc when discussing validated scope and release review.
  - Update release-readiness requirements so public docs, limitation reporting, and validation instructions are expected to stay mutually consistent.
- Alternatives:
  - Move all RC material into README: rejected because the RC document is intentionally versioned and narrower than the general project docs.
  - Leave release docs isolated from contributor docs: rejected because it invites divergence between “what the repo says” and “what release guidance says.”

### Decision 5: M7C stays tooling- and documentation-focused; no new runtime semantics are bundled into this change
- Rationale: the remaining runtime limitations, such as yielding across native callback boundaries or broader JIT support, are separate architecture projects. Folding them into M7C would dilute the goal of converting existing promises into real verification and documentation.
- Design:
  - Restrict implementation to docs, tests, harnesses, scripts, and related metadata/spec changes.
  - Treat current runtime limitations as documented boundaries to be reflected clearly in public docs and validation guidance.
- Alternatives:
  - Pair quality hardening with another semantic feature: rejected because it would make the new validation signal noisier and harder to attribute.

## Risks / Trade-offs

- [Documentation drift between README, architecture, contributor guide, and RC doc] -> Mitigation: assign each document a narrow purpose and cross-link them instead of duplicating detailed support text everywhere.
- [New property/fuzz tooling adds dependency or maintenance overhead] -> Mitigation: keep dependencies test-only or tool-only and isolate them from runtime crates and release binaries.
- [Extended validation lane becomes neglected because it is not a required PR gate] -> Mitigation: document explicit entrypoints, keep seed corpora/reproducers in-repo, and incorporate the lane into release/hardening guidance.
- [Property-based tests become flaky if generators are too broad] -> Mitigation: constrain generators to valid semantic domains and bias toward deterministic, shrink-friendly invariants.
- [Repository structure changes may confuse contributors initially] -> Mitigation: use README as the single navigation hub and keep new docs/scripts named predictably.

## Migration Plan

1. Add the public documentation set (`README.md`, `docs/architecture.md`, `CONTRIBUTING.md`) and wire cross-links to the existing release-candidate doc.
2. Add deterministic property-based tests for the highest-value invariants in parser/compiler/runtime layers and integrate them into ordinary workspace tests.
3. Introduce fuzzing entrypoints, corpora/reproducer layout, and supporting documentation under the repository’s hardening workflow.
4. Update validation scripts or contributor-facing commands so required and extended checks are both easy to discover.
5. Sync the new expectations into quality-gate and release-readiness specs, then verify the documented commands against the actual repository state.

Rollback strategy:
- Documentation additions can land independently of fuzz/property tooling if the validation layer needs more iteration.
- Extended fuzz entrypoints can ship before any CI integration, as long as docs clearly label them as manual or nightly hardening steps.

## Open Questions

- Should the extended hardening workflow be exposed through a single wrapper script, or is documented raw cargo/tooling invocation sufficient?
- Do we want property-based testing to start only in parser/compiler crates, or should the first cut include VM/JIT invariants immediately?
- Should release-facing docs link to a stable “validation matrix” page in `docs/`, or is `CONTRIBUTING.md` enough as the operational source of truth for now?
