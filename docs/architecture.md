# Architecture

## Layers

`rlua` is organized as a workspace with explicit interpreter-first layering:

- `rlua-core`: bytecode instructions, values, closures, GC types, and shared runtime data structures.
- `rlua-parser`: lexer, token model, AST, and source-to-AST parsing.
- `rlua-compiler`: AST-to-bytecode lowering that produces `FunctionProto`.
- `rlua-vm`: register-based interpreter, call frames, coroutine execution, fallback semantics, and JIT dispatch points.
- `rlua-ir`: trace IR and optimization passes used after recording hot loops.
- `rlua-jit`: hot-loop recording, trace cache lifecycle, side-exit/deopt metadata, and native backend integration.
- `rlua-stdlib`: native library registration, callback bridges, and Lua-visible library functions.
- `rlua-cli`: user-facing runners and inspection tools.

## Primary Execution Flows

### Interpreter Baseline

1. Source text is lexed and parsed by `rlua-parser`.
2. `rlua-compiler` lowers the AST into `FunctionProto`.
3. `rlua-vm` executes bytecode as the correctness reference.
4. `rlua-stdlib` provides the library surface used by conformance, differential, and JIT tests.

This path is the semantic source of truth. Every optimized path must preserve interpreter-visible behavior for the validated subset.

### Tracing JIT

1. `rlua-vm` counts hot loop headers.
2. Once the configured threshold is crossed, `rlua-jit` records a linear trace.
3. `rlua-ir` optimizes the recorded trace.
4. On `x86_64`, supported traces may compile to native code; otherwise they execute through replay or interpreter fallback.
5. Guard failures and side exits use deopt metadata to resume in the interpreter.

## Supported Boundaries

- Interpreter support tracks the validated Lua 5.1-compatible subset covered by the repository suites.
- The standard library support is the exercised `math`, `string`, `table`, and baseline `coroutine` surface.
- JIT support is intentionally narrower than interpreter support and currently targets numeric hot loops with stable arithmetic behavior.
- Trace inspection is exposed as post-run summaries through `trace-inspect`.

## Intentional Limits and Fallback Paths

- The tracing JIT is not a general Lua compiler.
- Coroutine execution stays interpreter-only even when JIT is enabled.
- Yielding across native callback boundaries, including library-driven Lua callbacks, remains unsupported.
- Native code generation is only available on `x86_64`.
- Unsupported trace shapes may record or replay, but they are not part of the performance promise.

Release-facing limitation wording lives in [docs/release-candidate.md](release-candidate.md), and contributor workflow guidance lives in [CONTRIBUTING.md](../CONTRIBUTING.md).
