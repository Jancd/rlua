# Contributing

## Setup

- Install stable Rust with `cargo`.
- Optional: install `lua5.1` or `luajit` if you want the differential suite to compare against a reference interpreter.
- Clone the repository and work from the workspace root.

## Repository Workflow

1. Start or continue an OpenSpec change before making cross-cutting behavior changes.
2. Make focused edits that preserve the currently documented interpreter and JIT support boundaries.
3. Run the required validation lane before asking for review.
4. Run the extended hardening lane when touching parser, compiler, VM, JIT, or validation infrastructure.
5. If you are implementing an active OpenSpec change, run `openspec validate <change-name> --type change --strict` before syncing or archiving it.

## Required Validation

Use the repository entrypoint:

```bash
sh scripts/validate-required.sh
```

That lane currently runs:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

## Extended Hardening

Use the repository entrypoint:

```bash
sh scripts/validate-hardening.sh
```

That lane replays the checked-in fuzz-style corpora and mutations through ignored hardening tests. Run it when:

- changing parser, compiler, VM, JIT, or stdlib callback behavior
- updating validation scripts or corpus assets
- preparing release or stability-focused review

The hardening assets live under [tests/fuzz/](tests/fuzz/README.md).

## Release Validation

For release-candidate review, use:

```bash
sh scripts/validate-rc.sh
```

That sweep builds on the required and extended lanes, then runs the release-facing trace inspection and benchmark commands documented in [docs/release-candidate.md](docs/release-candidate.md).

## OpenSpec Notes

- Create one change per coherent feature or hardening effort.
- Keep `proposal`, `design`, `specs`, and `tasks` aligned with implementation.
- Sync delta specs into `openspec/specs/` before archiving a completed change.

## Test and Corpus Hygiene

- Put deterministic property tests in normal Rust test targets.
- Put corpus seeds and retained reproducers under `tests/fuzz/`.
- When a hardening run finds a crashing or semantically invalid input, minimize it and check it into `tests/fuzz/reproducers/<surface>/`.
- Prefer small, reviewable reproducers over large raw fuzz artifacts.
