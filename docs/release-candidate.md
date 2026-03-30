# rlua v1.0.0-rc Readiness

This document defines the repository-visible support surface, limitations, and validation path for the `v1.0.0-rc` candidate.

## Supported Runtime Surface

- Interpreter: the validated Lua 5.1-compatible subset covered by the repository conformance and differential suites.
- Standard library: the stdlib surface exercised by the current conformance and differential tests.
- Tracing JIT: the M5-supported hot-loop subset centered on numeric `for` loops and stable numeric arithmetic traces.
- Native backend: x86_64 only. Other architectures remain semantically correct through replay or interpreter fallback.
- Fallback behavior: guard failures, side exits, downgrade, and invalidation remain supported only insofar as they preserve interpreter-equivalent behavior for the validated subset.

## Trace Inspection

Use `trace-inspect` to run a script and emit stable post-run trace state without enabling feature-gated event logging.

Text summary:

```bash
cargo run -p rlua-cli --bin trace-inspect -- \
  --hot-threshold 2 \
  tests/jit/native_side_exit_resume.lua
```

Structured JSON output:

```bash
cargo run -p rlua-cli --bin trace-inspect -- \
  --format json \
  --hot-threshold 2 \
  --side-exit-threshold 1 \
  tests/jit/guard_invalidation_recovery.lua
```

The inspection output is the release-facing source of truth for:
- cached trace identity via function pointer plus loop header pc
- trace generation
- lifecycle state (`active`, `replay-only`, `invalidated`)
- native artifact state
- invalidation reason
- native entry, replay entry, side exit, and invalidated-bypass accounting

## Known Limitations

- The tracing JIT is not a general Lua 5.1 compiler. It only targets the numeric loop subset already validated by the repository JIT tests and benchmark harness.
- Native code generation is not available outside x86_64. Unsupported architectures use replay or interpreter fallback instead.
- Trace inspection is a post-run summary surface, not a streaming trace debugger.
- The RC does not add new Lua language coverage beyond the subset already delivered by M5.
- Unsupported or unoptimized trace shapes may still record or replay, but they are not part of the release-candidate performance promise.

## Benchmark Expectations

The benchmark harness for RC review is:

```bash
cargo run --release -p rlua-cli --bin jit-bench -- --samples 3
```

The supported release workloads are:
- `numeric_sum_large`
- `numeric_descending_large`
- `native_side_exit_resume_large`

Interpret the benchmark output as follows:
- `pass`: the median speedup meets the documented target and no workload is reported as a slow case.
- `investigate`: the median speedup meets the target, but at least one workload is reported as a slow case and should be triaged with `trace-inspect`.
- `fail`: the benchmark run fails, or the median speedup falls below the documented target.

## Release Validation Path

For a full release-candidate sweep, run:

```bash
sh scripts/validate-rc.sh
```

That sweep packages the required repository-native entrypoints:
- `openspec validate m6-release-candidate --type change --strict`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo run -p rlua-cli --bin trace-inspect -- --format json --hot-threshold 2 --side-exit-threshold 1 tests/jit/guard_invalidation_recovery.lua`
- `cargo run --release -p rlua-cli --bin jit-bench -- --samples 3`

The RC is ready only when all of those checks pass and any slow-case output is either absent or understood and accepted within the documented limitations above.
