# rlua

`rlua` is a Rust workspace for a Lua 5.1-compatible interpreter plus a tracing JIT for a validated hot-loop subset.

## Current Scope

- Interpreter: validated against the repository's conformance and differential suites for the supported Lua 5.1-compatible subset.
- Standard library: the exercised `math`, `string`, `table`, and baseline `coroutine` surface.
- Tracing JIT: optimized for numeric `for` loops and stable numeric arithmetic traces.
- Native backend: `x86_64` only. Other architectures stay correct through replay or interpreter fallback.
- Observability: `trace-inspect` provides stable post-run trace summaries without enabling feature-gated logging.

Known boundaries remain explicit:

- The JIT is not a general Lua compiler.
- Coroutine execution remains interpreter-only.
- Yielding across native callback boundaries remains unsupported.

See [docs/release-candidate.md](docs/release-candidate.md) for the versioned RC support and limitation document.

## Workspace Layout

- `crates/rlua-core`: shared value model, bytecode, GC-facing types, and function metadata.
- `crates/rlua-parser`: lexer, parser, AST, and parse errors.
- `crates/rlua-compiler`: AST-to-bytecode compiler plus conformance, differential, JIT, and hardening-oriented tests.
- `crates/rlua-vm`: register VM interpreter, coroutine execution, and JIT integration.
- `crates/rlua-ir`: trace IR and optimization passes.
- `crates/rlua-jit`: trace recorder, cache lifecycle, deopt metadata, and native backend integration.
- `crates/rlua-stdlib`: standard library registration and native callback bridges.
- `crates/rlua-cli`: CLI entrypoints including `rlua-cli`, `trace-inspect`, and `jit-bench`.
- `tests/`: repository-visible conformance, differential, JIT, and fuzz/hardening assets.

## Documentation

- [docs/architecture.md](docs/architecture.md): crate boundaries, execution flow, and supported versus unsupported paths.
- [CONTRIBUTING.md](CONTRIBUTING.md): local workflow, OpenSpec usage, required validation, and extended hardening commands.
- [docs/release-candidate.md](docs/release-candidate.md): release-candidate support surface, limits, trace inspection, and benchmark guidance.
- [tests/fuzz/README.md](tests/fuzz/README.md): corpus layout, reproducer retention, and hardening replay workflow.

## Common Commands

Run a script:

```bash
cargo run -p rlua-cli -- path/to/script.lua
```

Run the required validation lane:

```bash
sh scripts/validate-required.sh
```

Run the extended hardening lane:

```bash
sh scripts/validate-hardening.sh
```

Run the full release-candidate sweep:

```bash
sh scripts/validate-rc.sh
```
