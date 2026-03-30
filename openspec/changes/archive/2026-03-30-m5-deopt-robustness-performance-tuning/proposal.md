## Why

M4 made end-to-end native trace execution real for a narrow supported subset, but the runtime still treats deoptimization, side exits, and trace lifetime management as minimal fallback paths rather than hardened execution paths. Before the JIT can claim sustained speedups, M5 needs to make exits and invalidation interpreter-correct under stress and add benchmark feedback that proves the native path is materially faster than replay/interpreter execution.

## What Changes

- Harden deoptimization so supported native and replay traces carry enough resume metadata to reconstruct interpreter-visible state at every supported side exit.
- Stabilize trace invalidation and cache lifecycle behavior so stale or no-longer-valid traces are invalidated, bypassed, or recompiled without semantic drift.
- Add benchmark-oriented instrumentation and harness coverage for the supported hot-loop subset, with tuning work focused on reaching the M5 speedup target against interpreter baseline.
- Extend diagnostics and regression coverage to make deopt behavior, invalidation events, and benchmark regressions observable and testable.

## Capabilities

### New Capabilities
- `jit-performance-benchmarks`: benchmark harnesses and reporting for the M5-supported traced workload set, including baseline comparison against interpreter execution.

### Modified Capabilities
- `tracing-jit-execution-pipeline`: strengthen requirements around deopt maps, side-exit resume behavior, invalidation policy, and native/replay/interpreter state coherence.
- `engineering-quality-gates`: add deopt/invalidation regression coverage and benchmark-driven validation criteria for M5.

## Impact

- Affected code: `crates/rlua-ir`, `crates/rlua-jit`, `crates/rlua-vm`, and JIT-focused tests/bench infrastructure.
- Affected systems: trace cache lifecycle, side-exit handling, native trace execution, diagnostics, and benchmark reporting.
- Dependencies/APIs: no planned public API breakage, but host-visible JIT debug state and diagnostics may expand to expose invalidation and deopt outcomes.
