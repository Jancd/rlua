## 1. Workspace Bootstrap

- [x] 1.1 Create Cargo workspace and all target crates (`rlua-core`, `rlua-parser`, `rlua-compiler`, `rlua-vm`, `rlua-ir`, `rlua-jit`, `rlua-stdlib`, `rlua-cli`)
- [x] 1.2 Wire crate dependencies and public exports for interpreter-first runtime path
- [x] 1.3 Add runnable CLI stub that initializes runtime and prints execution mode

## 2. Runtime Foundation Skeleton

- [x] 2.1 Add macro-driven opcode table in `rlua-core` and generate enum + metadata helpers
- [x] 2.2 Add baseline value model and bytecode chunk structures
- [x] 2.3 Add VM execution loop stub that can execute `NOP`/`HALT` and return status

## 3. JIT Pipeline Interfaces

- [x] 3.1 Define profiling counter and hot-loop threshold interfaces in VM
- [x] 3.2 Define trace recorder/IR/codegen/deopt trait boundaries in `rlua-jit` and `rlua-ir`
- [x] 3.3 Integrate JIT capability detection and interpreter fallback behavior

## 4. Quality Gates and Tests

- [x] 4.1 Add workspace CI workflow for fmt/clippy/test
- [x] 4.2 Add baseline unit tests for opcode generation and VM `NOP`/`HALT` execution
- [x] 4.3 Create test directory scaffolding for conformance/differential/jit/fuzz

## 5. OpenSpec Tracking

- [x] 5.1 Validate change with `openspec validate bootstrap-lua-jit-foundation --type change --strict`
- [x] 5.2 Generate apply context and keep tasks synced with implementation progress
