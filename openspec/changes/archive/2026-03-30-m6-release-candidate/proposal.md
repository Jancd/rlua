## Why

M5 delivered a fast and more robust tracing JIT path, but the project still needs a release-focused pass before it can be presented as `v1.0.0-rc`. M6 is the point where the team stops expanding core scope and instead hardens the supported subset, documents what is and is not ready, and adds the tooling needed to triage final regressions confidently.

## What Changes

- Run a release-candidate hardening pass across the supported interpreter and JIT subset, fixing correctness bugs and stability issues found by a final conformance and regression sweep.
- Add release-facing documentation for the supported feature set, known limitations, JIT architecture constraints, benchmark expectations, and recommended validation workflows.
- Add trace-inspection tooling so developers can inspect cached traces, lifecycle state, benchmark-relevant counters, and fallback behavior without relying only on ad hoc debug logging.
- Tighten release validation requirements so the RC is gated by final conformance, regression, diagnostics, and benchmark checks rather than milestone-local spot checks.

## Capabilities

### New Capabilities
- `release-candidate-readiness`: release-oriented documentation, limitation reporting, validation guidance, and RC packaging expectations for the supported runtime/JIT subset.
- `trace-inspection-tooling`: developer-facing trace inspection and reporting surfaces for cache state, invalidation, native/replay usage, and benchmark triage.

### Modified Capabilities
- `engineering-quality-gates`: extend the validation contract from milestone delivery gates to a release-candidate sweep covering conformance, regression, diagnostics, and documented acceptance criteria.
- `jit-performance-benchmarks`: carry the M5 benchmark harness forward into RC validation so benchmark output is usable as a release-readiness signal, not just a tuning aid.
- `tracing-jit-execution-pipeline`: clarify which trace/runtime states must remain observable and supported for RC-grade debugging and fallback analysis.

## Impact

- Affected code: `crates/rlua-cli`, `crates/rlua-jit`, `crates/rlua-vm`, test/benchmark assets, and documentation under `openspec/` plus repository-facing release docs.
- Affected systems: diagnostics surfaces, benchmark/validation workflow, release packaging expectations, and final bug-fix triage for the supported Lua 5.1 subset.
- Dependencies/APIs: no planned expansion of the supported language surface; host-visible tooling and documentation may expand, and release validation requirements will become stricter than M5 milestone-only gates.
