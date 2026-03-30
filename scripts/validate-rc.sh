#!/usr/bin/env sh
set -eu

echo "[rc] validating openspec change"
openspec validate m6-release-candidate --type change --strict

echo "[rc] running clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "[rc] running workspace tests"
cargo test --workspace

echo "[rc] validating trace inspection output"
cargo run -p rlua-cli --bin trace-inspect -- \
  --format json \
  --hot-threshold 2 \
  --side-exit-threshold 1 \
  tests/jit/guard_invalidation_recovery.lua

echo "[rc] running release benchmark sweep"
cargo run --release -p rlua-cli --bin jit-bench -- --samples 3
