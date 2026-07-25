#!/usr/bin/env bash
# flamediff.sh — cross-run attribution in one command.
#
# Takes two folded profiles (as scripts/flame.sh writes) and renders a
# differential: a `stack before after` folded file plus a red/blue diff
# SVG where every frame is colored by how its self time moved — red grew
# (a regression), blue shrank, drawn on the AFTER profile's widths. Pure
# text in, SVG out; no capture, no measurement, no build needed. The
# renderer is scripts/flame.py (no network, no flamegraph.pl).
#
# usage: flamediff.sh <before.folded> <after.folded> [name]
#
#   before/after   folded files (…/flame/<family>.folded from flame.sh)
#   name           output stem (default: <after-basename>-vs-<before>)
#
# output (BUMBLEDB_FLAME_OUT overrides the base, default bench-out):
#   <base>/flame/<name>.diff.folded   `stack before after`
#   <base>/flame/<name>.diff.svg      the red/blue differential
set -euo pipefail

if [ "$#" -lt 2 ]; then
    sed -n '2,17p' "$0" >&2
    exit 2
fi

REPO="$(cd "$(dirname "$0")/.." && pwd)"
BEFORE="$1"
AFTER="$2"

for f in "$BEFORE" "$AFTER"; do
    if [ ! -f "$f" ]; then
        echo "flamediff.sh: no such folded file: $f" >&2
        exit 1
    fi
done

base_a="$(basename "${AFTER%.folded}")"
base_b="$(basename "${BEFORE%.folded}")"
NAME="${3:-${base_a}-vs-${base_b}}"

OUT_BASE="${BUMBLEDB_FLAME_OUT:-$REPO/bench-out}"
FLAME_DIR="$OUT_BASE/flame"

python3 "$REPO/scripts/flame.py" diff "$BEFORE" "$AFTER" "$FLAME_DIR" "$NAME"
echo "flamediff.sh: wrote $FLAME_DIR/$NAME.diff.folded and .diff.svg"
