#!/bin/sh
set -eu

LOCK="${BUMBLEDB_MEASURE_LOCK:-/tmp/bumbledb.measure.lock}"

while ! mkdir "$LOCK" 2>/dev/null; do
    echo "measure.sh: waiting for $LOCK (held by: $(cat "$LOCK/holder" 2>/dev/null || echo unknown))" >&2
    sleep 5
done
echo "$$ $(date +%s)" > "$LOCK/holder"
trap 'rm -rf "$LOCK"' EXIT INT TERM

"$@"
