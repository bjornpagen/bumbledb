#!/bin/zsh
# baseline-2026-07-25 SEGMENT 2 driver — the full-suite baseline (TODO
# Phase 3 item 2): scenarios, report-class durable+ephemeral ×3, writes,
# churn (all three profiles, one report like the night's), crud, curves,
# storage. The lawful suite is the segment-1 re-pin (e511b540) and is not
# re-run. One lane per invocation (`$1`), each under the measurement
# mutex with the shared-machine boost; wall power asserted before AND
# after every lane (a battery drop mid-lane voids the lane). Strictly
# sequential — the binary is prebuilt, nothing else runs during timed
# windows.
set -eu
REPO=/Users/bjorn/Documents/bumbledb
BIN=$REPO/target/release/bumbledb-bench
OUT=$REPO/bench-out/baseline-2026-07-25
CORPUS=$REPO/bench-data/fa73e680324f9b26

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

pin_ref_digests() { # $1 = lane dir (corpus-referencing lanes: storage, curves, writes)
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

pin_post_digests() { # $1 = lane dir, $2 = twin-store root (post-state lanes: crud)
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
*) echo "unknown lane: $1"; exit 2 ;;
esac
