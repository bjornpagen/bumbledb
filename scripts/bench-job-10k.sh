#!/usr/bin/env bash
set -euo pipefail

JOB_DIR=${1:-/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb}
OUT=${2:-/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v3-job-10k.json}
ERR=${3:-/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v3-job-10k.stderr.log}

cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir "${JOB_DIR}" \
  --open-limit 10000 \
  --format json \
  >"${OUT}" \
  2>"${ERR}"
