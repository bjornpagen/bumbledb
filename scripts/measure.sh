#!/bin/sh
# The measurement mutex: wraps
# any timing command in an exclusive machine-wide lock so two agents'
# measurements never overlap (the clock proxy catches contamination; this
# prevents the self-inflicted kind). mkdir is atomic; the lock dir names
# the holder for debugging.
set -eu

LOCK="${BUMBLEDB_MEASURE_LOCK:-/tmp/bumbledb.measure.lock}"

while ! mkdir "$LOCK" 2>/dev/null; do
    echo "measure.sh: waiting for $LOCK (held by: $(cat "$LOCK/holder" 2>/dev/null || echo unknown))" >&2
    sleep 5
done
echo "$$ $(date +%s)" > "$LOCK/holder"
trap 'rm -rf "$LOCK"' EXIT INT TERM

"$@"
