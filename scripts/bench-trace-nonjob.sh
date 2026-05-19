#!/usr/bin/env bash
set -euo pipefail

artifact_dir="${BUMBLED_TRACE_DIR:-${TMPDIR:-/tmp}/bumbledb-nonjob-trace-$(date +%Y%m%d%H%M%S)}"
scale="${BUMBLED_BENCH_SCALE:-10000}"
repeats="${BUMBLED_BENCH_REPEATS:-1}"
warmup="${BUMBLED_BENCH_WARMUP:-0}"

mkdir -p "$artifact_dir"

trace_path="$artifact_dir/nonjob-trace.jsonl"
result_path="$artifact_dir/nonjob-results.json"

RUST_LOG="${RUST_LOG:-bumbledb_lmdb=debug}" cargo run -p bumbledb-bench --release -- \
  --scale "$scale" \
  --warmup "$warmup" \
  --repeats "$repeats" \
  --format json \
  --trace \
  --trace-format json \
  --trace-output "$trace_path" \
  --dataset ledger \
  --dataset sailors \
  --dataset joinstress \
  --dataset tpch \
  > "$result_path"

scripts/summarize-trace-jsonl.sh "$trace_path" "$result_path" > "$artifact_dir/nonjob-summary.txt"

printf 'trace=%s\n' "$trace_path"
printf 'results=%s\n' "$result_path"
printf 'summary=%s\n' "$artifact_dir/nonjob-summary.txt"
