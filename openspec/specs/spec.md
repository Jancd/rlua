# Rust-Lang Lua JIT Project Specification

## 1. Objective

Build a production-oriented Lua 5.1-compatible JIT runtime in Rust with:

- A portable interpreter baseline and bytecode VM.
- A tracing JIT compiler for hot paths.
- Strong correctness guarantees through differential and property-based testing.
- Minimal dependence on third-party libraries.
- Heavy use of Rust metaprogramming (`macro_rules!`, procedural macros only when justified, const generics, and build-time code generation) to reduce hand-written duplication and improve maintainability.

Out of scope for v1:

- Full Lua 5.2+ feature parity.
- Full C ABI / C module loading compatibility.
- Advanced GC generations or moving GC.

## 2. Product Scope

### 2.1 Functional Scope (v1)

- Lua source parsing to AST.
- Bytecode compiler for Lua 5.1 core language:
  - Numbers, strings, booleans, nil.
  - Tables.
  - Closures and upvalues.
  - Varargs.
  - Metatables (subset required for common patterns).
  - Standard control flow and arithmetic ops.
- Bytecode VM interpreter as the correctness reference.
- Hot loop detection and trace recording.
- Trace IR + optimization passes.
- Machine code backend for at least one architecture (x86_64 first).
- Side exits and deoptimization back to interpreter.
- Minimal standard library subset (`math`, `string`, `table`, `coroutine` subset optional).

### 2.2 Non-Functional Scope

- Deterministic behavior and reproducible builds.
- Good observability: trace logs, counters, and IR dumps behind feature flags.
- Memory safety by default; `unsafe` only in well-audited, isolated modules.
- Target performance goal (v1):
  - >= 2x median speedup vs interpreter baseline on a defined benchmark suite.

## 3. Design Principles

- Interpreter first, JIT second: always maintain a correct fallback path.
- Minimize dependencies:
  - Prefer Rust standard library.
  - Use third-party crates only if they significantly reduce risk/time in isolated layers.
- Metaprogramming over copy-paste:
  - Declarative macro-generated opcode definitions, dispatch tables, IR builders, and test vectors.
- Clear module boundaries and explicit invariants.
- Every optimization must preserve observable Lua semantics.

## 4. Dependency Policy

### 4.1 Default Rule

- No external crate unless it meets all:
  - Provides substantial complexity reduction.
  - Is mature, maintained, and permissively licensed.
  - Is isolated behind an internal abstraction.

### 4.2 Preferred External Usage (if needed)

- `libc` for low-level memory protection and executable page handling.
- Optional parser helper crates are disallowed in v1; implement parser in-house.
- Benchmarking crate optional for developer benchmarking only; not runtime dependency.

### 4.3 Forbidden in Core Runtime

- Full JIT frameworks that hide codegen internals.
- Large runtime-heavy dependencies in hot paths.

## 5. High-Level Architecture

```text
Lua Source
  -> Lexer
  -> Parser (AST)
  -> Bytecode Compiler
  -> Bytecode Chunk + Metadata
  -> Interpreter VM
       -> Profiler/Hot Counter
       -> Trace Recorder
           -> Trace IR
           -> Optimizer
           -> Codegen (x86_64)
           -> Executable Trace Cache
       -> Guard Fail / Side Exit
       -> Deopt + Resume in Interpreter
```

## 6. Repository Layout (Proposed)

```text
/rlua
  /crates
    /rlua-core        # value model, GC, VM types, opcode definitions
    /rlua-parser      # lexer + parser
    /rlua-compiler    # AST -> bytecode compiler
    /rlua-vm          # interpreter engine
    /rlua-ir          # trace IR and passes
    /rlua-jit         # recorder, codegen, trace cache, deopt
    /rlua-stdlib      # minimal stdlib
    /rlua-cli         # repl/runner
  /tests
    /conformance
    /differential
    /jit
    /fuzz
  /bench
  spec.md
```

Single-crate start is acceptable initially, but code should be organized by module as if it will be split.

## 7. Core Runtime Components

### 7.1 Value Representation

- Start with tagged enum for correctness and simplicity.
- Plan optional NaN-boxed representation behind feature flag once semantics stabilize.
- Define strict conversion semantics for arithmetic and comparisons.

### 7.2 Bytecode Instruction Set

- Lua-like register VM instruction format.
- Use macro-generated opcode declarations:
  - Opcode enum.
  - Decoding helpers.
  - Pretty-printers.
  - Dispatch scaffolding.

### 7.3 VM Execution Model

- Call frame stack with explicit register windows.
- Upvalue capture model compatible with closures.
- Error propagation without panics in runtime path.

### 7.4 Garbage Collection

- v1: non-moving mark-sweep GC.
- Root set:
  - VM stack/registers.
  - Globals and interned strings.
  - Open upvalues.
  - JIT metadata references.
- JIT traces must provide precise GC maps for live values at safepoints.

## 8. JIT Architecture

### 8.1 Strategy

- Use tracing JIT first (not method JIT), optimized for Lua dynamic behavior.
- Trigger when loop header exceeds hot threshold.

### 8.2 Trace Recording

- Record linear hot path with guards for dynamic assumptions:
  - Type checks.
  - Table shape/metatable assumptions.
  - Branch outcomes.
- Capture side exits with enough state to reconstruct interpreter frame.

### 8.3 IR Design

- SSA-like trace IR with explicit value types and guard ops.
- Core operations:
  - Load/store locals/upvalues.
  - Arithmetic/logical ops.
  - Table access ops.
  - Calls (initially limited/inlined builtins only).
  - Guards and exits.

### 8.4 Optimization Passes (v1)

- Constant folding.
- Common subexpression elimination (local to trace).
- Dead code elimination.
- Guard simplification.
- Optional lightweight type narrowing.

### 8.5 Code Generation

- x86_64 backend first.
- Internal assembler DSL generated via macros:
  - Instruction encoders.
  - Register definitions.
  - Prologue/epilogue templates.
- Executable memory manager with W^X discipline.

### 8.6 Deoptimization

- On guard failure:
  - Reconstruct VM state from deopt map.
  - Jump back to interpreter at mapped bytecode PC.
- Deopt correctness is mandatory before aggressive optimization.

## 9. Rust Metaprogramming Plan

Use metaprogramming to keep core logic explicit but non-repetitive:

- `macro_rules!` for:
  - Opcode table as single source of truth.
  - VM dispatch boilerplate.
  - IR op definitions and constructors.
  - Guard templates.
  - Repeated conformance test scaffolds.
- Build-time code generation (small internal tool in `build.rs` or `xtask`) for:
  - Opcode docs and mnemonic lookup tables.
  - IR pretty-printer tables.
- Const generics for fixed-size structures where helpful:
  - Register window sizes.
  - Inline cache slots.

Constraint: avoid macro abuse that obscures control flow in safety-critical logic.

## 10. Safety and Unsafe Code Policy

- `unsafe` allowed only in:
  - Executable memory/page protection.
  - Raw code emission buffers.
  - Carefully bounded performance-critical value operations.
- Each `unsafe` block must have:
  - A short invariant comment.
  - Unit tests that stress related assumptions.

## 11. Milestones and Deliverables

### M0: Project Bootstrap (Week 1)

- Cargo workspace and crate/module skeleton.
- CI setup (format, lint, test).
- Architecture docs and coding standards.

Deliverable: builds and runs empty CLI + test harness.

### M1: Parser + Bytecode VM Baseline (Weeks 2-5)

- Lexer/parser for Lua subset.
- Bytecode compiler + disassembler.
- Working interpreter for core language features.

Deliverable: passes initial Lua conformance subset, no JIT.

### M2: GC + Full Baseline Semantics (Weeks 6-8)

- Mark-sweep GC integrated.
- Closures/upvalues/metatables completed for v1 scope.
- Differential testing against reference Lua for supported subset.

Deliverable: stable interpreter with semantic confidence.

### M3: Profiling + Trace Recorder (Weeks 9-11)

- Hot loop detection.
- Trace recording with guard representation.
- Exit stubs that return to interpreter.

Deliverable: traces recorded and replayed in interpreter-equivalent mode.

### M4: IR Optimizer + x86_64 Codegen (Weeks 12-15)

- Implement v1 optimization passes.
- Emit machine code for core traced ops.
- Link trace cache execution into VM.

Deliverable: end-to-end JIT execution on selected benchmarks.

### M5: Deopt Robustness + Performance Tuning (Weeks 16-18)

- Correct deopt maps and side exits.
- Stabilize trace invalidation.
- Benchmark-driven tuning.

Deliverable: target >= 2x median speedup vs interpreter baseline.

### M6: Release Candidate (Weeks 19-20)

- Hardening, docs, trace tooling, bug fixes.
- Final conformance + regression sweep.

Deliverable: v1.0.0-rc with documented limitations.

## 12. Testing Strategy

### 12.1 Test Layers

- Unit tests:
  - Lexer/parser pieces.
  - Bytecode emission.
  - VM instructions.
  - IR transforms.
  - Codegen encoders.
- Integration tests:
  - Script-level semantic behavior.
  - Error handling and edge cases.
- Differential tests:
  - Run identical scripts on `rlua` and reference Lua 5.1.
  - Compare outputs, errors, and key state snapshots.
- Property-based tests:
  - Expression equivalence and arithmetic invariants.
  - Parser round-trip and compiler sanity invariants.
- Fuzzing:
  - Lexer/parser input fuzz.
  - Bytecode verifier fuzz.
  - JIT trace formation fuzz.

### 12.2 JIT-Specific Validation

- Trace vs interpreter equivalence harness:
  - Run hot loops in both modes and compare state after each iteration window.
- Guard-failure stress tests:
  - Randomized type/shape changes to force exits.
- Deopt correctness tests:
  - Assert register/local/upvalue reconstruction at every side exit site.
- Snapshot tests for IR and machine code metadata (not raw bytes across toolchains).

### 12.3 Performance Testing

- Benchmark suites:
  - Numeric loops.
  - Table-heavy workloads.
  - Recursion/closure workloads.
  - Mixed real-world scripts.
- Track:
  - Warm-up time.
  - Steady-state throughput.
  - Trace compilation overhead.
  - Memory overhead.

### 12.4 CI Gates

- Required on each PR:
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
  - `cargo test`
  - Selected differential tests
- Nightly/extended:
  - Fuzz jobs.
  - Full benchmark regression.

## 13. Observability and Tooling

- Feature-gated diagnostics:
  - `trace-log`: record trace lifecycle.
  - `ir-dump`: dump pre/post optimization IR.
  - `jit-stats`: counters for compilations, exits, invalidations.
- CLI flags for JIT policy:
  - enable/disable JIT.
  - hot threshold.
  - max traces per function.

## 14. Risks and Mitigations

- Semantic drift between interpreter and JIT:
  - Mitigation: strict differential harness and trace equivalence checks.
- Deopt complexity:
  - Mitigation: enforce deopt map coverage before advanced passes.
- Unsafe code bugs in codegen/memory management:
  - Mitigation: isolate modules, invariant docs, stress tests, sanitizers where possible.
- Scope creep:
  - Mitigation: freeze Lua feature scope for v1; track extras in post-v1 backlog.

## 15. Definition of Done (v1)

- Supported Lua 5.1 subset documented and validated.
- Interpreter passes conformance and differential test thresholds.
- JIT enabled by default for supported workloads and stable under stress tests.
- Performance target met on benchmark suite.
- Public docs include architecture, limitations, and contributor guide.

## 16. Immediate Next Steps

1. Initialize workspace crates and module skeleton matching Section 6.
2. Implement opcode macro table and baseline bytecode format first.
3. Build parser + interpreter until differential tests are green.
4. Add profiler hooks in VM to prepare non-invasive JIT integration.
