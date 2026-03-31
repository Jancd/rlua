#!/usr/bin/env sh
set -eu

echo "[required] running rustfmt"
cargo fmt --check

echo "[required] running clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "[required] running workspace tests"
cargo test --workspace
