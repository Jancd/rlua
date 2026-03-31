#!/usr/bin/env sh
set -eu

echo "[hardening] replaying checked-in hardening corpora"
cargo test -p rlua-compiler --test hardening -- --ignored --nocapture
