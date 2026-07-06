#!/bin/sh
# The two-step ritual as one command (docs/architecture/50-validation.md): verify earns
# the stamp, bench spends it. S by default; BENCH_SCALE=L for the claim.
set -eu

cd "$(dirname "$0")/.."

SCALE="${BENCH_SCALE:-S}"
SEED="${BENCH_SEED:-1}"
DIR="${BENCH_DIR:-bench-data}"

echo "==> cargo build -p bumbledb-bench --release"
cargo build -p bumbledb-bench --release

BIN=target/release/bumbledb-bench

echo "==> $BIN verify --scale $SCALE --seed $SEED --dir $DIR"
"$BIN" verify --scale "$SCALE" --seed "$SEED" --dir "$DIR"

echo "==> $BIN bench --scale $SCALE --seed $SEED --dir $DIR"
"$BIN" bench --scale "$SCALE" --seed "$SEED" --dir "$DIR"
