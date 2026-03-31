## 1. Documentation Surface

- [x] 1.1 Add a repository `README.md` with project overview, validated subset summary, and links to deeper documentation
- [x] 1.2 Add `docs/architecture.md` describing crate responsibilities, execution flow, and supported versus unsupported boundaries
- [x] 1.3 Add `CONTRIBUTING.md` covering setup, OpenSpec workflow, and required versus extended validation commands
- [x] 1.4 Cross-link `README.md`, `docs/architecture.md`, `CONTRIBUTING.md`, and `docs/release-candidate.md` with consistent terminology and limitation wording

## 2. Property-Based Validation

- [x] 2.1 Add test-only property-testing dependencies and shared helpers for deterministic generators and assertions
- [x] 2.2 Add parser and compiler property tests that check stability and invariant-preserving behavior under `cargo test`
- [x] 2.3 Add interpreter-visible runtime property tests that cover semantic invariants without expanding supported language scope

## 3. Fuzz Hardening Workflow

- [x] 3.1 Replace the placeholder `tests/fuzz/` layout with repository-visible fuzz workflow assets and ownership guidance
- [x] 3.2 Add initial fuzz entrypoints or runner scripts for parser, compiler, and runtime-adjacent surfaces with documented commands
- [x] 3.3 Add deterministic crash replay guidance and reproducer retention paths for fuzz-discovered failures

## 4. Validation Entry Points and Release Alignment

- [x] 4.1 Add or update validation scripts and documentation so required PR checks and the extended hardening lane are clearly separated
- [x] 4.2 Update release-facing documentation to reference the new public docs and hardening workflow consistently
- [x] 4.3 Ensure contributor-facing validation guidance matches actual repository entrypoints, command names, and expected outputs

## 5. Verification

- [x] 5.1 Run `openspec validate m7c-quality-hardening-and-docs --type change --strict`
- [x] 5.2 Run targeted verification for the new documentation, property-test, and fuzz-workflow entrypoints
- [x] 5.3 Run final workspace-level validation and confirm the documented commands and limitation statements remain aligned
