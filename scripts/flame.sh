#!/usr/bin/env bash
# flame.sh — one command, one SVG flamegraph.
#
# Builds the bench binary with the engine's trace instrumentation on
# (the `obs` feature), captures ONE traced warm sample of a read family
# under the measurement mutex (scripts/measure.sh — a flame capture is a
# measurement, it must not overlap another), and renders the folded-stack
# twin the trace already writes (trace_out/fold.rs: `<stem>.folded` beside
# the Chrome `<stem>.json`) into a self-contained SVG flamegraph. No
# network, no flamegraph.pl: scripts/flame.py is the whole renderer.
#
# usage: flame.sh <family> [extra `trace` flags...]
#
#   <family>   a read family name (`bumbledb-bench trace --family` set —
#              run `bumbledb-bench help` for the roster). The optional
#              trailing flags pass straight to `trace` (e.g. `--scale M`,
#              `--seed 7`), so the traced corpus is yours to pick.
#
# output (BUMBLEDB_FLAME_OUT overrides the base, default bench-out):
#   <base>/flame/<family>.folded   collapsed stacks, self-time weighted
#   <base>/flame/<family>.svg      the flamegraph — open it in a browser
# plus the top-10 self-time table on stdout and, unchanged, the warm/cold
# Chrome traces the `trace` command already writes (speedscope/chrome).
set -euo pipefail

if [ "$#" -lt 1 ]; then
    sed -n '2,20p' "$0" >&2
    exit 2
fi

REPO="$(cd "$(dirname "$0")/.." && pwd)"
FAMILY="$1"
shift

OUT_BASE="${BUMBLEDB_FLAME_OUT:-$REPO/bench-out}"
FLAME_DIR="$OUT_BASE/flame"
OBS_TARGET="$REPO/target/bench-obs"
OBS_BIN="$OBS_TARGET/release/bumbledb-bench"

# Build (building is not measurement — kept outside the mutex). The obs
# build shares bench-night's dedicated target dir so neither clobbers the
# plain release binary.
(cd "$REPO" && CARGO_TARGET_DIR="$OBS_TARGET" \
    cargo build --release -p bumbledb-bench --features obs)

# Capture under the mutex, keeping the trace command's own output so we can
# recover the warm Chrome-trace path it prints ("traces: <warm> / <cold>").
CAP="$(mktemp)"
trap 'rm -f "$CAP"' EXIT
"$REPO/scripts/measure.sh" "$OBS_BIN" trace --family "$FAMILY" "$@" >"$CAP" 2>&1

WARM="$(sed -n 's/^traces: \(.*\) \/ .*/\1/p' "$CAP" | tail -n 1)"
if [ -z "$WARM" ] || [ ! -f "$WARM" ]; then
    cat "$CAP" >&2
    echo "flame.sh: could not locate the warm trace JSON from the trace output" >&2
    exit 1
fi

# The trace writes the folded twin beside the JSON (trace_out/fold.rs); that
# is the single folded owner — flame.py only renders it.
FOLDED="${WARM%.json}.folded"
if [ ! -f "$FOLDED" ]; then
    echo "flame.sh: expected the folded twin $FOLDED beside $WARM" >&2
    exit 1
fi

echo "flame.sh: rendering $FOLDED"
python3 "$REPO/scripts/flame.py" render "$FOLDED" "$FLAME_DIR" "$FAMILY"
