#!/bin/sh
# Serialize G15 timing on one host. Deterministic scorecard counters
# (app-perf --plan, verify, storage census) do not need this lock.
# Do not start timing while implementation workers are still churning.
set -eu

LOCK="${BUMBLEDB_MEASURE_LOCK:-/tmp/bumbledb.measure.lock}"

while ! mkdir "$LOCK" 2>/dev/null; do
    echo "measure.sh: waiting for $LOCK (held by: $(cat "$LOCK/holder" 2>/dev/null || echo unknown))" >&2
    sleep 5
done
echo "$$ $(date +%s)" > "$LOCK/holder"
trap 'rm -rf "$LOCK"' EXIT INT TERM

"$@"
