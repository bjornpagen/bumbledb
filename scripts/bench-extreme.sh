#!/usr/bin/env bash
set -euo pipefail

cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 30 --format markdown "$@"
