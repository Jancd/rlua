## 1. Deopt Metadata and Trace Artifacts

- [x] 1.1 Extend `rlua-ir` optimized trace artifacts to derive explicit deoptimization/resume metadata for supported guard and side-exit sites
- [x] 1.2 Update `rlua-jit` native/replay trace artifacts to carry the shared deopt contract needed to restore interpreter-visible state
- [x] 1.3 Expose deopt-oriented debug metadata for cached traces so exits can be inspected in tests and diagnostics

## 2. Trace Lifecycle and Invalidation Policy

- [x] 2.1 Add per-trace lifecycle state in `rlua-jit` for active/native/replay-only/invalidated cache entries, including invalidation reason and generation metadata
- [x] 2.2 Implement counter-based side-exit stability policy that can downgrade or invalidate unstable traces without breaking interpreter fallback
- [x] 2.3 Allow invalidated hot loops to become eligible for replacement trace recording or recompilation without reusing stale lifecycle state

## 3. VM Deopt and Dispatch Integration

- [x] 3.1 Route native and replay exits in `rlua-vm` through the explicit deopt metadata instead of relying only on ad hoc `resume_pc` plus slot sync
- [x] 3.2 Update cached-trace dispatch so the VM only enters active native traces and otherwise falls back deterministically to replay or interpreter execution
- [x] 3.3 Expand host-visible JIT debug state to report deopt outcomes, invalidation events, and per-trace execution mode transitions

## 4. Diagnostics and Regression Coverage

- [x] 4.1 Extend diagnostics feature flags and emitted events to cover deoptimization, invalidation, downgrade, and recompilation behavior
- [x] 4.2 Add JIT regression tests that force supported native and replay side exits and assert interpreter-equivalent resume behavior
- [x] 4.3 Add invalidation-focused tests that prove invalidated traces are bypassed safely and can be replaced when the loop becomes hot again

## 5. Benchmark Harness and M5 Validation

- [x] 5.1 Introduce a benchmark harness and workload set for the M5-supported hot-loop subset with interpreter-only and JIT-enabled execution modes
- [x] 5.2 Report per-workload timings, aggregate median speedup, and enough trace/debug context to explain slow or downgraded benchmark cases
- [x] 5.3 Tune the supported numeric trace path using benchmark feedback to improve entry/exit overhead and fallback policy behavior
- [x] 5.4 Validate the change with `openspec validate m5-deopt-robustness-performance-tuning --type change --strict`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and the benchmark validation path
