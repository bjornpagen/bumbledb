#!/bin/zsh
# campaign RUN 2 driver — the crud + curves + lawful + storage suites,
# sequential, each under the measurement mutex with the shared-machine
# boost; wall power asserted before AND after every lane (a battery drop
# mid-lane voids that lane — this run exists because the battery era was
# retired). Digests pinned per lane after its report lands.
set -eu
REPO=/Users/bjorn/Documents/bumbledb
BIN=$REPO/target/release/bumbledb-bench
OUT=$REPO/bench-out/campaign-2026-07-23
CORPUS=$REPO/bench-data/6518394f080c2273

assert_ac() { # $1 = phase label
    if ! pmset -g batt | grep -q "AC Power"; then
        echo "!!! POWER FAIL at $1: $(pmset -g batt | head -1)"
        exit 3
    fi
    echo "=== power ok at $1: AC"
}

pin_ref_digests() { # $1 = lane dir (corpus-referencing lanes: storage, curves)
    {
        shasum -a 256 "$CORPUS/db/data.mdb" "$CORPUS/cal-db/data.mdb" \
            "$CORPUS/oracle.sqlite" "$CORPUS/cal-oracle.sqlite" \
            "$CORPUS/verify.stamp" | sed "s|$REPO/||"
    } > "$1/digests.raw"
}

pin_post_digests() { # $1 = lane dir, $2 = twin-store root (post-state lanes: crud, lawful)
    {
        shasum -a 256 "$2/durable/db/data.mdb" "$2/durable/oracle.sqlite" \
            "$2/nosync/db/data.mdb" "$2/nosync/oracle.sqlite" | sed "s|$REPO/||"
    } > "$1/digests.raw"
}

assert_ac "launch"

mkdir -p "$OUT/storage"
echo "=== lane storage start $(date '+%H:%M:%S')"
BUMBLEDB_BENCH_BOOST=1 "$REPO/scripts/measure.sh" "$BIN" storage \
    --out "$OUT/storage" > "$OUT/storage/run.log" 2>&1
assert_ac "storage close"
pin_ref_digests "$OUT/storage"
echo "=== lane storage done $(date '+%H:%M:%S')"

mkdir -p "$OUT/curves"
echo "=== lane curves start $(date '+%H:%M:%S')"
BUMBLEDB_BENCH_BOOST=1 "$REPO/scripts/measure.sh" "$BIN" curves --warmth \
    --out "$OUT/curves" > "$OUT/curves/run.log" 2>&1
assert_ac "curves close"
pin_ref_digests "$OUT/curves"
echo "=== lane curves done $(date '+%H:%M:%S')"

mkdir -p "$OUT/crud"
echo "=== lane crud start $(date '+%H:%M:%S')"
BUMBLEDB_BENCH_BOOST=1 "$REPO/scripts/measure.sh" "$BIN" crud \
    --out "$OUT/crud" > "$OUT/crud/run.log" 2>&1
assert_ac "crud close"
pin_post_digests "$OUT/crud" "$REPO/bench-data/crud"
echo "=== lane crud done $(date '+%H:%M:%S')"

mkdir -p "$OUT/lawful"
echo "=== lane lawful start $(date '+%H:%M:%S')"
BUMBLEDB_BENCH_BOOST=1 "$REPO/scripts/measure.sh" "$BIN" lawful \
    --out "$OUT/lawful" > "$OUT/lawful/run.log" 2>&1
assert_ac "lawful close"
pin_post_digests "$OUT/lawful" "$REPO/bench-data"
echo "=== lane lawful done $(date '+%H:%M:%S')"

echo "=== all four suites complete $(date '+%H:%M:%S')"
