#!/bin/sh
# Runs a command under a host-wide lock so benchmark timings never overlap.
#
#   scripts/measure.sh <command...>
set -eu

LOCK=/tmp/bdb.measure.lock

while ! mkdir "$LOCK" 2>/dev/null; do
    echo "measure.sh: waiting for $LOCK (held by: $(cat "$LOCK/holder" 2>/dev/null || echo unknown))" >&2
    sleep 5
done
echo "$$ $(date +%s)" > "$LOCK/holder"
trap 'rm -rf "$LOCK"' EXIT INT TERM

"$@"
