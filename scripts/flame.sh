#!/usr/bin/env bash
# flame.sh — one command, one SVG flamegraph.
#
# Builds the bench binary with the engine's trace instrumentation on
# (the `obs` feature), captures ONE traced warm sample under the
# measurement mutex (scripts/measure.sh — a flame capture is a
# measurement, it must not overlap another), and renders the folded-stack
# twin the trace already writes (trace_out/fold.rs: `<stem>.folded` beside
# the Chrome `<stem>.json`) into a self-contained SVG flamegraph. No
# network, no flamegraph.pl: scripts/flame.py is the whole renderer.
#
# usage: flame.sh <family> [extra `trace` flags...]
#        flame.sh <scenario> <query> [extra `scenarios` flags...]
#
#   <family>            a read family name (`bumbledb-bench trace --family`
#                       set — run `bumbledb-bench help` for the roster).
#                       Trailing flags pass straight to `trace` (`--scale M`,
#                       `--seed 7`), so the traced corpus is yours to pick.
#   <scenario> <query>  a scenario lane and one of its queries (`joins
#                       j1_filmography`, `graph g2_two_hop`, ...): runs
#                       `scenarios --trace --only <scenario>` (gated, one
#                       timing sample — this is a profile, not a timing)
#                       and renders that query's warm capture. Trailing
#                       flags pass to `scenarios` (`--seed 7`).
#
# output (BUMBLEDB_FLAME_OUT overrides the base, default bench-out):
#   <base>/flame/<name>.folded   collapsed stacks, self-time weighted
#   <base>/flame/<name>.svg      the flamegraph — open it in a browser
# (<name> is <family> or <scenario>.<query>) plus the top-10 self-time
# table on stdout and, unchanged, the warm+cold Chrome traces the capture
# already writes (speedscope/chrome).
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

# Build (building is not measurement — kept outside the mutex). The obs
# build shares bench-night's dedicated target dir so neither clobbers the
# plain release binary.
(cd "$REPO" && CARGO_TARGET_DIR="$OBS_TARGET" \
    cargo build --release -p bumbledb-bench --features obs)

CAP="$(mktemp)"
trap 'rm -f "$CAP"' EXIT

# Runs one capture command under the mutex, keeping its output in $CAP;
# on failure the captured output surfaces instead of vanishing into set -e.
capture() {
    if ! "$REPO/scripts/measure.sh" "$@" >"$CAP" 2>&1; then
        cat "$CAP" >&2
        echo "flame.sh: the traced capture failed (output above)" >&2
        exit 1
    fi
}

if [ "$#" -ge 2 ] && [ "${2#--}" = "$2" ]; then
    # The lane form: <scenario> <query>. The scenario's traced pass writes
    # every query's warm+cold pair under <run>/trace/scenarios/<scenario>/;
    # we render the named query's warm folded. --samples 1 keeps the timing
    # half vestigial (a later flag wins if you want real samples too).
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
    # The family form: one traced warm+cold pair from the `trace`
    # subcommand; its stdout names the warm Chrome path ("traces: <warm>
    # / <cold>") and the folded twin sits beside it (trace_out/fold.rs,
    # the single folded owner — flame.py only renders).
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
