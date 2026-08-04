#!/bin/zsh
# baseline-2026-07-25-post REBENCH driver — the post-fix rerun of exactly
# the lanes the fix lanes targeted (exec/sink/plan/overlap/storage/api
# fixes touch every read lane; storage delta/applier/judgment fixes touch
# every write lane): the six report-class reps, scenarios, storage,
# curves, crud, writes, churn, plus the segment-1 lanes the judgment
# fixes touched (lawful, windowed). Protocol identical to
# bench-out/baseline-2026-07-25/run-full-suite.sh: one lane per
# invocation ($1), measurement mutex + shared-machine boost, wall power
# asserted before AND after every lane (a battery drop mid-lane voids the
# lane), binary prebuilt, oracle re-earned per binary before the first
# timed window.
set -eu
REPO=/Users/bjorn/Documents/bumbledb
BIN=$REPO/target/release/bumbledb-bench
OUT=$REPO/bench-out/baseline-2026-07-25-post
CORPUS=$REPO/bench-data/fa73e680324f9b26
CAP_FAMILIES=commit_capacity_baseline,commit_capacity_sum,commit_capacity_duration
WIN_FAMILIES=commit_window_baseline,commit_window_admission,commit_window_exclusion

assert_ac() { # $1 = phase label
    if ! pmset -g batt | grep -q "AC Power"; then
        echo "!!! POWER FAIL at $1: $(pmset -g batt | head -1)"
        exit 3
    fi
    echo "=== power ok at $1: AC"
}

pin_report_digests() { # $1 = lane dir (report-class reps: both twin pairs)
    {
        shasum -a 256 "$CORPUS/db/data.mdb" "$CORPUS/cal-db/data.mdb" \
            "$CORPUS/naive-db/data.mdb" "$CORPUS/cal-naive-db/data.mdb" \
            "$CORPUS/oracle.sqlite" "$CORPUS/cal-oracle.sqlite" \
            "$CORPUS/verify.stamp" | sed "s|$REPO/||"
    } > "$1/digests.txt"
}

pin_ref_digests() { # $1 = lane dir (corpus-referencing lanes)
    {
        shasum -a 256 "$CORPUS/db/data.mdb" "$CORPUS/cal-db/data.mdb" \
            "$CORPUS/oracle.sqlite" "$CORPUS/cal-oracle.sqlite" \
            "$CORPUS/verify.stamp" | sed "s|$REPO/||"
    } > "$1/digests.txt"
}

pin_scenario_digests() { # $1 = lane dir
    {
        shasum -a 256 $REPO/bench-data/scenarios/*/oracle.sqlite \
            $REPO/bench-data/scenarios/*/db/data.mdb | sed "s|$REPO/||"
    } > "$1/digests.txt"
}

pin_post_digests() { # $1 = lane dir, $2 = twin-store root (post-state lanes)
    {
        shasum -a 256 "$2/durable/db/data.mdb" "$2/durable/oracle.sqlite" \
            "$2/nosync/db/data.mdb" "$2/nosync/oracle.sqlite" | sed "s|$REPO/||"
    } > "$1/digests.txt"
}

run_lane() { # $1 = lane dir under $OUT, rest = bench argv
    local lane=$1; shift
    mkdir -p "$OUT/$lane"
    assert_ac "$lane open"
    echo "=== lane $lane start $(date '+%H:%M:%S')"
    BUMBLEDB_BENCH_BOOST=1 "$REPO/scripts/measure.sh" "$BIN" "$@" \
        --out "$OUT/$lane" > "$OUT/$lane/run.log" 2>&1
    assert_ac "$lane close"
    echo "=== lane $lane done $(date '+%H:%M:%S')"
}

case "$1" in
# --- report-class reps (six, driven one per invocation) ---
bench-durable-r1|bench-durable-r2|bench-durable-r3)
    run_lane "$1" bench
    pin_report_digests "$OUT/$1" ;;
bench-ephemeral-r1|bench-ephemeral-r2|bench-ephemeral-r3)
    run_lane "$1" bench --ephemeral
    pin_report_digests "$OUT/$1" ;;
# --- the scenario suites (full registry, one merged report) ---
scenarios)
    run_lane scenarios scenarios
    pin_scenario_digests "$OUT/scenarios" ;;
# --- the metric + world suites ---
storage)
    run_lane storage storage
    pin_ref_digests "$OUT/storage" ;;
curves)
    run_lane curves curves --warmth
    pin_ref_digests "$OUT/curves" ;;
crud)
    run_lane crud crud
    pin_post_digests "$OUT/crud" "$REPO/bench-data/crud" ;;
writes)
    run_lane writes writes
    pin_ref_digests "$OUT/writes" ;;
# --- the long-lived churn lanes (all three profiles, night-shaped report) ---
churn)
    run_lane churn churn ;;
# --- the segment-1 lanes the judgment/storage fixes touched ---
windowed-durable)
    run_lane windowed/durable bench --families "$WIN_FAMILIES"
    pin_ref_digests "$OUT/windowed/durable" ;;
windowed-ephemeral)
    run_lane windowed/ephemeral bench --families "$WIN_FAMILIES" --ephemeral
    pin_ref_digests "$OUT/windowed/ephemeral" ;;
lawful)
    run_lane lawful lawful
    pin_post_digests "$OUT/lawful" "$REPO/bench-data"
    ;;
*) echo "unknown lane: $1"; exit 2 ;;
esac
