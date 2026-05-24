#!/usr/bin/env bash
set -euo pipefail

job_dir="${BUMBLED_JOB_DIR:-data/job}"
budget_file="docs/free-join-allocation-redesign-prds/JOB_ALLOCATION_BUDGETS.tsv"

cargo run --release -p bumbledb-bench -- \
  --preset job-sample \
  --job-dir "$job_dir" \
  --open-limit 100000 \
  --format json \
  --repeats 1 \
  --warmup 1 \
  --alloc on \
  --allocation-budget "$budget_file"
