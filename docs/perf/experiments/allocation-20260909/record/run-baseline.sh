#!/usr/bin/env bash
set -euo pipefail
REPO=/Users/bjorn/Documents/bumbledb
ROUND=/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv
cd "$REPO"
test "$(git rev-parse HEAD)" = b02a641e087364ec09c161a97c67c87cf626e6b2
test -z "$(git status --porcelain)"
test -z "${RUSTFLAGS:-}"
test -z "${CARGO_ENCODED_RUSTFLAGS:-}"
test -z "${CARGO_TARGET_DIR:-}"
git archive --format=tar.gz --output="$ROUND/source.tar.gz" HEAD
{
  date -u
  uname -a
  sw_vers
  sysctl hw.model hw.memsize hw.pagesize machdep.cpu.brand_string
  pmset -g batt
} > "$ROUND/host.txt"
cargo build --locked --release -p bumbledb-bench
mkdir "$ROUND/release"
cp -p target/release/bumbledb-bench "$ROUND/release/bumbledb-bench"
cargo build --locked --profile profiling -p bumbledb-bench
mkdir "$ROUND/profiling"
cp -p target/profiling/bumbledb-bench "$ROUND/profiling/bumbledb-bench"
cp -RL target/profiling/bumbledb-bench.dSYM "$ROUND/profiling/bumbledb-bench.dSYM"
test -z "$(git status --porcelain)"
shasum -a 256 "$ROUND/release/bumbledb-bench" "$ROUND/profiling/bumbledb-bench" > "$ROUND/binaries.sha256"
git rev-parse HEAD > "$ROUND/source-revision.txt"
rustc --version --verbose > "$ROUND/toolchain.txt"
dwarfdump --uuid "$ROUND/profiling/bumbledb-bench" "$ROUND/profiling/bumbledb-bench.dSYM" > "$ROUND/symbol-uuids.txt"
test "$(dwarfdump --uuid "$ROUND/profiling/bumbledb-bench" | awk '{print $2}')" = "$(dwarfdump --uuid "$ROUND/profiling/bumbledb-bench.dSYM" | awk '{print $2}')"
env BENCH_NIGHT_UNDER_LOCK=1 \
  BUMBLEDB_BENCH_BIN="$ROUND/release/bumbledb-bench" \
  BUMBLEDB_BENCH_DATA="$ROUND/data" \
  scripts/bench-night.sh "$ROUND/full" --full --shared --allow-macos-qos
