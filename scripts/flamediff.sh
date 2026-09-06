#!/usr/bin/env bash
# Compare CPU-weighted folded stacks exported from native captures.
# Red = more sampled CPU, blue = less; this is attribution, not a latency verdict.
set -euo pipefail

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
    echo "usage: flamediff.sh <before.folded> <after.folded> [name]" >&2
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
