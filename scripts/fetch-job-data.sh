#!/usr/bin/env bash
set -euo pipefail

target_dir="${1:-data/job}"
url="${BUMBLED_JOB_URL:-https://event.cwi.nl/da/job/imdb.tgz}"
archive="$target_dir/imdb.tgz"

mkdir -p "$target_dir"
if [ ! -f "$archive" ]; then
  curl -L --fail --retry 3 -o "$archive" "$url"
fi
tar -xzf "$archive" -C "$target_dir"

echo "JOB data ready in $target_dir"
echo "Run with: BUMBLED_JOB_DIR=$target_dir cargo run -p bumbledb-bench -- --preset job-sample --format json --repeats 1 --warmup 0"
