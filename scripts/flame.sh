#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ]; then
    sed -n '2,31p' "$0" >&2
    exit 2
fi

REPO="$(cd "$(dirname "$0")/.." && pwd)"

OUT_BASE="${BUMBLEDB_FLAME_OUT:-$REPO/bench-out}"
FLAME_DIR="$OUT_BASE/flame"
OBS_TARGET="$REPO/target/bench-obs"
OBS_BIN="$OBS_TARGET/release/bumbledb-bench"

(cd "$REPO" && CARGO_TARGET_DIR="$OBS_TARGET" \
    cargo build --release -p bumbledb-bench --features obs)

CAP="$(mktemp)"
trap 'rm -f "$CAP"' EXIT

capture() {
    if ! "$REPO/scripts/measure.sh" "$@" >"$CAP" 2>&1; then
        cat "$CAP" >&2
        echo "flame.sh: the traced capture failed (output above)" >&2
        exit 1
    fi
}

if [ "$#" -ge 2 ] && [ "${2#--}" = "$2" ]; then

    SCENARIO="$1"
    QUERY="$2"
    shift 2
    NAME="$SCENARIO.$QUERY"
    RUN_DIR="$FLAME_DIR/$SCENARIO"
    capture "$OBS_BIN" scenarios --samples 1 --trace \
        --only "$SCENARIO" --out "$RUN_DIR" "$@"
    FOLDED="$RUN_DIR/trace/scenarios/$SCENARIO/$QUERY.warm.folded"
    if [ ! -f "$FOLDED" ]; then
        echo "flame.sh: no warm folded for query \`$QUERY\` — the scenario traced:" >&2
        ls "$RUN_DIR/trace/scenarios/$SCENARIO/" >&2
        exit 1
    fi
else

    FAMILY="$1"
    shift
    NAME="$FAMILY"
    capture "$OBS_BIN" trace --family "$FAMILY" "$@"
    WARM="$(sed -n 's/^traces: \(.*\) \/ .*/\1/p' "$CAP" | tail -n 1)"
    if [ -z "$WARM" ] || [ ! -f "$WARM" ]; then
        cat "$CAP" >&2
        echo "flame.sh: could not locate the warm trace JSON from the trace output" >&2
        exit 1
    fi
    FOLDED="${WARM%.json}.folded"
    if [ ! -f "$FOLDED" ]; then
        echo "flame.sh: expected the folded twin $FOLDED beside $WARM" >&2
        exit 1
    fi
fi

echo "flame.sh: rendering $FOLDED"
python3 "$REPO/scripts/flame.py" render "$FOLDED" "$FLAME_DIR" "$NAME"
