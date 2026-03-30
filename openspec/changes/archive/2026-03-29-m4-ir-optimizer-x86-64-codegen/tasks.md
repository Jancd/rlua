## 1. Trace IR and Optimizer

- [x] 1.1 Refactor `rlua-ir` trace representation so recorded traces can express backend-friendly operations, guards, exits, and source provenance together
- [x] 1.2 Add an optimization pipeline in `rlua-ir` for the M4-supported trace subset, including at least constant folding, dead code elimination, and guard simplification
- [x] 1.3 Add optimizer unit tests proving optimized traces stay semantically equivalent to their input traces and preserve unsupported regions safely

## 2. x86_64 Backend and Executable Trace Artifacts

- [x] 2.1 Introduce a minimal x86_64 encoder/emitter layer in `rlua-jit` for the M4-supported arithmetic + loop-control trace subset
- [x] 2.2 Add executable buffer / native artifact management with isolated `unsafe` boundaries and explicit install semantics for compiled traces
- [x] 2.3 Compile optimized traces into native x86_64 artifacts and install them into the trace cache alongside replay metadata

## 3. VM Integration and Fallback

- [x] 3.1 Extend the VM/JIT runtime boundary so hot loops can request optimized compilation and query native artifacts from the trace cache
- [x] 3.2 Prefer native trace execution when a compiled artifact is present, while preserving replay/interpreter fallback on unsupported platforms, compile skips, or runtime failure
- [x] 3.3 Keep guard exits, counters, and resume metadata coherent across native execution, replay, and interpreter paths

## 4. Diagnostics and Host Controls

- [x] 4.1 Extend diagnostics feature flags to cover optimizer activity, native code generation, native trace installation, and native execution events
- [x] 4.2 Expose the minimal runtime/debug state needed to assert whether a trace was optimized, compiled, installed, and entered natively
- [x] 4.3 Ensure CLI / host configuration keeps existing JIT controls coherent when native backend support is unavailable

## 5. Tests and Validation

- [x] 5.1 Add `rlua-ir` / `rlua-jit` unit tests for optimization passes, x86_64 encoding, executable artifact installation, and cache behavior
- [x] 5.2 Add `tests/jit/` smoke cases that compare native-trace execution against replay/interpreter behavior for the M4-supported trace subset
- [x] 5.3 Add platform/fallback tests proving unsupported architectures or unsupported traces remain on replay/interpreter paths without semantic drift
- [x] 5.4 Validate the change with `openspec validate m4-ir-optimizer-x86-64-codegen --type change --strict` and relevant `cargo test` coverage
