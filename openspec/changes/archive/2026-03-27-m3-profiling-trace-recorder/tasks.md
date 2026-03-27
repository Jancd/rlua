## 1. Profiling and Runtime Hooks

- [x] 1.1 Extend JIT/runtime configuration to carry `enabled` and hot-threshold policy from CLI/host setup into `rlua-vm` and `rlua-jit`
- [x] 1.2 Add loop-header hotness tracking in `rlua-vm`, keyed so backward-edge execution can identify a hot loop header deterministically
- [x] 1.3 Trigger trace lookup/record on threshold crossing while preserving interpreter-only execution when JIT is disabled or no trace is installed

## 2. Trace IR and Recorder

- [x] 2.1 Expand `rlua-ir` trace data structures to include source bytecode locations, guard identifiers, and side-exit metadata needed for replay
- [x] 2.2 Implement a minimal trace recorder in `rlua-jit` that records supported hot-loop bytecode into cached trace structures
- [x] 2.3 Add trace-cache install/lookup APIs keyed by function plus loop-header PC and wire them into the VM/JIT runtime boundary

## 3. Replay Execution and Side Exits

- [x] 3.1 Implement a replay executor that can run recorded traces against live VM state without native code generation
- [x] 3.2 Implement guard evaluation and side-exit resume handling so replay failures restore interpreter-visible locals/upvalues and mapped PCs
- [x] 3.3 Integrate replay execution into the VM hot-path so cached traces run when available and fall back cleanly to interpreter execution otherwise

## 4. CLI and Diagnostics

- [x] 4.1 Add minimal CLI/runtime controls for enabling or disabling JIT and overriding the hot-threshold policy
- [x] 4.2 Extend feature-gated diagnostics to emit hot-loop detection, trace recording, replay, and side-exit events without affecting default builds
- [x] 4.3 Expose minimal counters or debug state needed for tests to assert trace installation and replay usage

## 5. Tests and Validation

- [x] 5.1 Add unit tests for trace metadata, guard/exit mapping, and trace-cache behavior in `rlua-ir` / `rlua-jit`
- [x] 5.2 Add `tests/jit/` replay-equivalence cases that compare JIT-enabled hot-loop execution with interpreter-only execution
- [x] 5.3 Add guard-failure regression tests that force replay exits and verify interpreter-equivalent completion
- [x] 5.4 Validate the change with `openspec validate m3-profiling-trace-recorder --type change --strict` and relevant `cargo test` coverage
