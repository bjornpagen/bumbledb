#!/usr/bin/env bash
set -euo pipefail

scale="${BUMBLED_BENCH_SCALE:-10000}"
repeats="${BUMBLED_BENCH_REPEATS:-30}"

cargo run -p bumbledb-bench --release -- \
  --scale "$scale" \
  --repeats "$repeats" \
  --format markdown \
  --dataset ledger \
  --dataset sailors \
  --dataset joinstress \
  --dataset tpch \
  "$@"
