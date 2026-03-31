# Fuzz and Hardening Assets

This directory stores repository-visible hardening inputs for parser, compiler, and runtime-adjacent validation.

## Layout

- `corpus/parser/`: parser-focused seeds. These may be valid or intentionally malformed.
- `corpus/compiler/`: valid compiler seeds used as mutation starting points.
- `corpus/runtime/`: valid scripts that should compile and execute successfully before mutation.
- `reproducers/parser/`, `reproducers/compiler/`, `reproducers/runtime/`: minimized retained regressions from hardening discoveries.

## Commands

Run the extended hardening lane:

```bash
sh scripts/validate-hardening.sh
```

Run the ignored hardening replay tests directly:

```bash
cargo test -p rlua-compiler --test hardening -- --ignored --nocapture
```

Property-based validation lives in the ordinary workspace test lane and runs through:

```bash
sh scripts/validate-required.sh
```

## Reproducer Retention

When a hardening run finds a crash or semantically invalid case:

1. Minimize the input to the smallest standalone `.lua` file that still reproduces the issue.
2. Save it under `reproducers/<surface>/`.
3. Keep the filename descriptive enough to explain what failed.
4. Preserve the reproducer even after the runtime bug is fixed so the ignored hardening replay continues to cover it.
