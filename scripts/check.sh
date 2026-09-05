#!/bin/sh
# Remainder scripts/battery.sh calls after workspace nextest.
set -eu

cd "$(dirname "$0")/.."

echo "==> cargo test --workspace --doc"
# nextest does not run rustdoc examples. Rustdoc's normal parallel harness
# handles these; do not force a single test thread.
cargo test --workspace --doc

echo "==> allocation gate (release): steady-state + escalating high-water"

cargo nextest run -p bumbledb --features alloc-counter --test alloc_gate --release

echo "==> bumbledb with the ground-off test-support feature (clippy + tests)"
cargo clippy -p bumbledb --all-targets --features ground-off -- -D warnings
cargo nextest run -p bumbledb --features ground-off

echo "==> bumbledb with the trace feature (tests)"
cargo nextest run -p bumbledb --features trace

echo "==> bumbledb --all-features (clippy, the pairwise co-compile check)"
cargo clippy -p bumbledb --all-targets --all-features -- -D warnings

echo "==> bumbledb-bench with the obs feature (clippy + harness tests)"
cargo clippy -p bumbledb-bench --features obs --all-targets -- -D warnings

# Exercise the entire feature-enabled crate in one process pool, rather
# than a hand-maintained list of filters that can omit new tests.
cargo nextest run -p bumbledb-bench --features obs

echo "==> flame renderer golden selftest"
python3 scripts/flame.py selftest

echo "==> feature-gated check lanes complete (not release qualification)"
