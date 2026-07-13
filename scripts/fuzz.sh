#!/bin/sh
# The firepower launcher (the crucible packet (git ecec1dc3); the
# fuzzing charter, docs/architecture/60-validation.md § operations).
# The machine is an M2 Max, 12 cores; the all-cores default IS the
# default, not a flag.
#
#   scripts/fuzz.sh                    all five targets, time-sliced,
#                                      libFuzzer fork mode, 12 workers,
#                                      FUZZ_MINUTES (default 10) per slice
#   scripts/fuzz.sh [minutes]          same, with the slice length given
#   scripts/fuzz.sh <target> [minutes] one target, all cores, bounded
#   scripts/fuzz.sh --asan <target> [minutes]
#                                      the sanitizer lane (PRD 15): ASAN
#                                      over one target; query carries
#                                      -rss_limit_mb=4096 (ASAN quarantine
#                                      accounting across the largest
#                                      corpus, dispositioned in
#                                      the crucible packet (git ecec1dc3)
#                                      § Results — a resource knob, not a
#                                      suppression)
#
# Lanes: the default lane builds `-s none` — throughput fuzzing; the
# oracles are debug assertions and harness panics, and ASAN's tax buys
# nothing per-push (PRD 15 measured it). `--asan` is the deliberate
# memory-error lane over the FFI seam. Both lanes run fork mode with
# -ignore_ooms=0 -ignore_crashes=0: any finding stops the session —
# findings are for triaging, not for counting while the fuzzer runs on.
# The rewrites target's one binary gets the grounding bypass through the
# `ground-off` feature declared by `fuzz/Cargo.toml`; the launcher never
# builds a second engine copy.
#
# Corpus discipline (built in, not optional):
#   * REFUSES to start while fuzz/artifacts holds ANY file — an
#     untriaged finding blocks new sessions; triage it into a named
#     regression test (or record its environmental disposition in
#     fuzz/SESSIONS.md) and delete the artifact first.
#   * `cargo fuzz cmin` per target after each session keeps the
#     checked-in seed corpus lean.
#   * one summary line per target appended to fuzz/SESSIONS.md — the
#     honest zero ("0 findings in N executions") is a recorded result.
#
# Env knobs: FUZZ_WORKERS (default 12) — verification/bounded sessions
# on a busy machine set it lower; the default stays all-cores.
set -eu

cd "$(dirname "$0")/.."

WORKERS="${FUZZ_WORKERS:-12}"
MINUTES="${FUZZ_MINUTES:-10}"
ALL_TARGETS="theory ops query rewrites crash"

MODE="none"
if [ "${1:-}" = "--asan" ]; then
    MODE="address"
    shift
    [ $# -ge 1 ] || { echo "usage: fuzz.sh --asan <target> [minutes]" >&2; exit 2; }
fi

case "${1:-}" in
"") TARGETS="$ALL_TARGETS" ;;
[0-9]*) TARGETS="$ALL_TARGETS" MINUTES="$1" ;;
*)
    echo "$ALL_TARGETS" | tr ' ' '\n' | grep -qx "$1" \
        || { echo "unknown target '$1' (targets: $ALL_TARGETS)" >&2; exit 2; }
    TARGETS="$1"
    MINUTES="${2:-$MINUTES}"
    ;;
esac

# The dirty-artifacts refusal: triage is not optional.
DIRTY="$(find fuzz/artifacts -type f 2>/dev/null || true)"
if [ -n "$DIRTY" ]; then
    echo "REFUSED: fuzz/artifacts holds untriaged findings:" >&2
    echo "$DIRTY" | sed 's/^/  /' >&2
    echo "triage each into a named regression test (or record its" >&2
    echo "disposition in fuzz/SESSIONS.md), delete the artifact, rerun." >&2
    exit 1
fi

DATE="$(date -u +%Y-%m-%d)"
SECONDS_PER=$((MINUTES * 60))

for TARGET in $TARGETS; do
    BEFORE="$(ls "fuzz/corpus/$TARGET" 2>/dev/null | wc -l | tr -d ' ')"
    LOG="$(mktemp -t "fuzz-$TARGET")"

    # Per-target libFuzzer knobs. Fork mode verified on this darwin host
    # with cargo-fuzz 0.13.1 (the crucible packet (git ecec1dc3)
    # § Results); the -jobs/-workers fallback recorded there was NOT
    # needed.
    RSS=""
    if [ "$MODE" = "address" ] && [ "$TARGET" = "query" ]; then
        RSS="-rss_limit_mb=4096"
    fi

    echo "==> $TARGET: fork=$WORKERS, ${MINUTES}m, sanitizer=$MODE"
    START="$(date +%s)"
    # shellcheck disable=SC2086  # RSS is one optional flag
    cargo fuzz run "$TARGET" -s "$MODE" -- \
        "-fork=$WORKERS" -ignore_ooms=0 -ignore_crashes=0 \
        "-max_total_time=$SECONDS_PER" $RSS 2>&1 | tee "$LOG" \
        || true # a finding exits nonzero; the summary still runs
    ELAPSED=$(($(date +%s) - START))

    # libFuzzer's own stats, parsed minimally: the last fork-mode status
    # line carries cumulative execs and coverage.
    STATS="$(grep -E '^#[0-9]+:' "$LOG" | tail -1 || true)"
    EXECS="$(printf '%s' "$STATS" | sed -n 's/^#\([0-9]*\):.*/\1/p')"
    COV="$(printf '%s' "$STATS" | sed -n 's/.* cov: \([0-9]*\).*/\1/p')"
    [ -n "$EXECS" ] || EXECS=0
    [ -n "$COV" ] || COV=0
    [ "$ELAPSED" -gt 0 ] || ELAPSED=1
    RATE=$((EXECS / ELAPSED))
    FINDINGS="$(find "fuzz/artifacts/$TARGET" -type f 2>/dev/null | wc -l | tr -d ' ')"

    # Corpus discipline: minimize after every session. cmin runs the
    # throughput build; coverage instrumentation is sanitizer-independent.
    cargo fuzz cmin "$TARGET" -s none >/dev/null 2>&1
    AFTER="$(ls "fuzz/corpus/$TARGET" 2>/dev/null | wc -l | tr -d ' ')"
    DIGEST="$( (cd "fuzz/corpus/$TARGET" && ls | sort | shasum -a 256) | cut -c1-16)"

    echo "==> $TARGET session: $EXECS execs (${RATE}/s) over ${MINUTES}m," \
        "cov $COV, corpus $BEFORE -> $AFTER (post-cmin," \
        "digest $DIGEST), $FINDINGS finding(s)"
    if [ "$FINDINGS" != "0" ]; then
        echo "==> artifacts (triage before the next session):"
        find "fuzz/artifacts/$TARGET" -type f | sed 's/^/  /'
    fi

    printf '| %s | %s | %s | %sm x %s workers | %s | %s | %s | %s -> %s | %s |\n' \
        "$DATE" "$TARGET" "$MODE" "$MINUTES" "$WORKERS" \
        "$EXECS" "$RATE/s" "$COV" "$BEFORE" "$AFTER" "$FINDINGS" \
        >>fuzz/SESSIONS.md
    rm -f "$LOG"
done

echo "==> session logged to fuzz/SESSIONS.md"
