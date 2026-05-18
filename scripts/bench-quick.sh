#!/usr/bin/env bash
set -euo pipefail

cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check --manifest-path fuzz/Cargo.toml
cargo run -p bumbledb-bench --release -- --scale 2000 --repeats 10 --format markdown "$@"
