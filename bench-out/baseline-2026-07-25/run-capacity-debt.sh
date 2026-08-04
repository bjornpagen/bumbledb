#!/bin/zsh
# baseline-2026-07-25 SEGMENT 1 driver — the frozen bench debt:
# (1) the C17 slot-vs-fetch measured choice (power-budget capacity lane
#     under BOTH `measure_children` arms, four cells:
#     {fetch,slot} × {durable,ephemeral} — same families, same protocol,
#     two binaries differing in exactly the one CAPACITY_WEIGHT_SLOT
#     constant),
# (2) the calendar capacity lane (fresh twin world,
#     `commit_capacity_duration` — rides every cell),
# (3) the windowed + lawful re-pins under the capacity spelling.
# One lane per invocation (`$1`), each under the measurement mutex with
# the shared-machine boost; wall power asserted before AND after (a
# battery drop mid-lane voids the lane). The corpus stamp is fresh
# (verify: 2889 cases) before any timed window.
set -eu
REPO=/Users/bjorn/Documents/bumbledb
OUT=$REPO/bench-out/baseline-2026-07-25
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

pin_ref_digests() { # $1 = lane dir
    {
        shasum -a 256 "$CORPUS/db/data.mdb" "$CORPUS/cal-db/data.mdb" \
            "$CORPUS/oracle.sqlite" "$CORPUS/cal-oracle.sqlite" \
            "$CORPUS/verify.stamp" | sed "s|$REPO/||"
    } > "$1/digests.txt"
}

run_bench() { # $1 = lane dir, $2 = binary, $3 = families, $4 = extra flags
    mkdir -p "$OUT/$1"
    assert_ac "$1 open"
    echo "=== lane $1 start $(date '+%H:%M:%S')"
    BUMBLEDB_BENCH_BOOST=1 "$REPO/scripts/measure.sh" "$2" bench \
        --families "$3" ${=4} --out "$OUT/$1" > "$OUT/$1/run.log" 2>&1
    assert_ac "$1 close"
    pin_ref_digests "$OUT/$1"
    echo "=== lane $1 done $(date '+%H:%M:%S')"
}

case "$1" in
# --- (1) the C17 four cells: two binaries, same protocol ---
c17-fetch-durable)   run_bench capacity-c17/fetch-durable  /tmp/c17/bench-fetch "$CAP_FAMILIES" "" ;;
c17-slot-durable)    run_bench capacity-c17/slot-durable   /tmp/c17/bench-slot  "$CAP_FAMILIES" "" ;;
c17-fetch-ephemeral) run_bench capacity-c17/fetch-ephemeral /tmp/c17/bench-fetch "$CAP_FAMILIES" "--ephemeral" ;;
c17-slot-ephemeral)  run_bench capacity-c17/slot-ephemeral  /tmp/c17/bench-slot  "$CAP_FAMILIES" "--ephemeral" ;;
# --- (3) the windowed re-pin (unit instance under the capacity spelling) ---
windowed-durable)    run_bench windowed/durable   "$REPO/target/release/bumbledb-bench" "$WIN_FAMILIES" "" ;;
windowed-ephemeral)  run_bench windowed/ephemeral "$REPO/target/release/bumbledb-bench" "$WIN_FAMILIES" "--ephemeral" ;;
# --- (3) the lawful re-pin (home-turf world, judged admission; oracle-gated inline) ---
lawful)
    mkdir -p "$OUT/lawful"
    assert_ac "lawful open"
    echo "=== lane lawful start $(date '+%H:%M:%S')"
    BUMBLEDB_BENCH_BOOST=1 "$REPO/scripts/measure.sh" \
        "$REPO/target/release/bumbledb-bench" lawful \
        --out "$OUT/lawful" > "$OUT/lawful/run.log" 2>&1
    assert_ac "lawful close"
    {
        shasum -a 256 "$REPO/bench-data/durable/db/data.mdb" "$REPO/bench-data/durable/oracle.sqlite" \
            "$REPO/bench-data/nosync/db/data.mdb" "$REPO/bench-data/nosync/oracle.sqlite" | sed "s|$REPO/||"
    } > "$OUT/lawful/digests.txt"
    echo "=== lane lawful done $(date '+%H:%M:%S')"
    ;;
*) echo "unknown lane: $1"; exit 2 ;;
esac
