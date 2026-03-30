## 1. Release Candidate Documentation

- [x] 1.1 Add repository-facing release-candidate documentation that describes the supported interpreter and JIT subset for `v1.0.0-rc`
- [x] 1.2 Document known limitations and unsupported cases so RC-facing docs do not imply broader support than the validated subset
- [x] 1.3 Document the RC validation path and benchmark interpretation guidance, including pass/investigate/fail expectations for the supported workload set

## 2. Trace Inspection CLI

- [x] 2.1 Add a CLI entrypoint that runs a Lua workload under configurable JIT policy and emits a post-run trace summary from stable runtime debug state
- [x] 2.2 Implement the default human-readable inspection summary for trace lifecycle state, invalidation details, and execution-mode counters
- [x] 2.3 Add an opt-in machine-readable inspection format that exposes the same per-trace lifecycle and counter data for automation

## 3. Runtime Observability and Benchmark Integration

- [x] 3.1 Extend the host-visible JIT debug state so cached traces expose stable lifecycle, generation or identity, native availability, and invalidation metadata for inspection
- [x] 3.2 Preserve observable accounting for native entry, replay entry, side exits, and invalidation-driven fallback so inspection output can explain slow or unstable workloads
- [x] 3.3 Update the benchmark harness to support isolated workload reruns and to surface release-oriented target status using the same supported workload set

## 4. RC Validation and Regression Coverage

- [x] 4.1 Add tests for the trace inspection CLI covering default summary output, machine-readable output, and runs without optional diagnostics enabled
- [x] 4.2 Add regression coverage that proves lifecycle, invalidation, and fallback state remain observable and semantically correct across replacement traces and side exits
- [x] 4.3 Package the conformance, regression, inspection, and benchmark entrypoints into one explicit release-candidate validation sweep

## 5. Hardening and Final Verification

- [x] 5.1 Fix correctness, stability, or tooling issues discovered by the RC sweep without expanding the supported language or JIT surface
- [x] 5.2 Validate the change with `openspec validate m6-release-candidate --type change --strict`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, the trace inspection validation path, and the RC benchmark validation path
