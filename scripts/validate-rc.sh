#!/usr/bin/env sh
set -eu

echo "[rc] running required validation"
sh scripts/validate-required.sh

echo "[rc] running extended hardening validation"
sh scripts/validate-hardening.sh

echo "[rc] validating trace inspection output"
cargo run -p rlua-cli --bin trace-inspect -- \
  --format json \
  --hot-threshold 2 \
  --side-exit-threshold 1 \
  tests/jit/guard_invalidation_recovery.lua

echo "[rc] running release benchmark sweep"
cargo run --release -p rlua-cli --bin jit-bench -- --samples 3
