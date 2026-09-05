#!/usr/bin/env bash
# BUMBLEDB_MEASURE_LOCK, shared with measure.sh, so the refusal is testable
# --shared the shared-machine night (owner ruling, 2026-07-20): the
set -euo pipefail

print_usage() {
    echo "usage: bench-night.sh <out-dir> [--plan] [--shared]"
    echo "  --plan    print the lane table with planned statuses; run nothing"
    echo "  --shared  shared-machine night: boost every lane (BUMBLEDB_BENCH_BOOST=1),"
    echo "            stamp shared_machine provenance; idle-machine requirement"
    echo "            waived (owner ruling 2026-07-20); the mutex stays mandatory"
}

usage() {
    print_usage >&2
    exit 2
}

case "${1:-}" in
    --help | -h)
        print_usage
        exit 0
        ;;
esac

[ "$#" -ge 1 ] || usage
[ -n "$1" ] || usage

REPO="$(cd "$(dirname "$0")/.." && pwd)"

TARGET_DIR="${CARGO_TARGET_DIR:-$REPO/target}"
BIN="$TARGET_DIR/release/bumbledb-bench"
OBS_TARGET="$REPO/target/bench-obs"
OBS_BIN="$OBS_TARGET/release/bumbledb-bench"
LOCK="${BUMBLEDB_MEASURE_LOCK:-/tmp/bumbledb.measure.lock}"

OUT_ARG=$1
shift
PLAN=0
SHARED=0
for arg in "$@"; do
    case "$arg" in
        --plan) PLAN=1 ;;
        --shared) SHARED=1 ;;
        *) usage ;;
    esac
done

case "$OUT_ARG" in
    -*) usage ;;
    /*) OUT="$OUT_ARG" ;;
    *) OUT="$PWD/$OUT_ARG" ;;
esac

# Compact L20 scorecard. Overlapping curves/heap/primerlane/adversarial
# timing jobs are not default qualification. verify + storage are semantic;
# app-perf timing is G15-only on a quiet host. Three-way / C-* cargo tests
# live in the bench crate (not scripts/lean.sh) and stay NotRun here.
PROBED=" storage "

lane_table() {
    cat <<EOF
scorecard-plan|SETUP|"$BIN" app-perf --plan
verify|SETUP|"$BIN" verify
correspondence-oracles|PREREQ|echo "NotRun: cargo test -p bumbledb-bench correspondence (C-D04/C-D19/C-G03; judge_final_state)"
three-way-conformance|PREREQ|echo "NotRun: cargo test -p bumbledb-bench three_way_conformance -- --ignored (not lean.sh)"
storage|$OUT/storage/storage-report.json|"$BIN" storage --scales S,M --out "$OUT/storage"
app-perf-warm|$OUT/app-perf-warm/app-perf.json|"$BIN" app-perf --regimes warm --out "$OUT/app-perf-warm"
app-perf-cold|$OUT/app-perf-cold/app-perf.json|"$BIN" app-perf --regimes cold-open,post-write --out "$OUT/app-perf-cold"
app-perf-tenants|$OUT/app-perf-tenants/app-perf.json|"$BIN" app-perf --regimes tenant-churn --out "$OUT/app-perf-tenants"
hash-probe|$OUT/hash-probe/hash-probe.json|"$BIN" hash-probe --out "$OUT/hash-probe"
hosted-decision|PREREQ|echo "NotRun: real S3/IAM required (PERF-003); see app-perf --plan"
large-populated|PREREQ|echo "NotRun: >40 GiB allocated blocks + cgroup memory.max (not a sparse map)"
graviton-arm64|PREREQ|echo "NotRun unless this host is real Graviton Linux ARM64"
x86-node|PREREQ|echo "NotRun unless this host is linux-x86-64 Node with the native addon"
EOF
}

is_probed() {
    case "$PROBED" in
        *" $1 "*) return 0 ;;
    esac
    return 1
}

lane_available() {
    "$BIN" help 2>/dev/null | awk '/^COMMANDS:/,/^$/' \
        | grep -qE "^[[:space:]]+$1([[:space:]]|\$)"
}

nonsetup_status() { 
    if [ "$2" = "PREREQ" ]; then
        echo "NOTRUN-PREREQ"
        return
    fi
    if [ -e "$2" ]; then
        echo "SKIP-EXISTING"
    elif is_probed "$1" && ! lane_available "$1"; then
        echo "SKIP-UNAVAILABLE"
    else
        echo "RUN"
    fi
}

# --- MUTEX REFUSAL + ACQUISITION (skipped in --plan) -----------------------
if [ "$PLAN" -eq 0 ] && [ "${BENCH_NIGHT_UNDER_LOCK:-}" != 1 ]; then
    if [ -d "$LOCK" ]; then
        echo "bench-night: refusing — measurement lock held (holder: $(cat "$LOCK/holder" 2>/dev/null || echo unknown))" >&2
        exit 2
    fi
    export BUMBLEDB_MEASURE_LOCK="$LOCK"

    if [ "$SHARED" -eq 1 ]; then
        exec "$REPO/scripts/measure.sh" \
            env BENCH_NIGHT_UNDER_LOCK=1 BUMBLEDB_MEASURE_LOCK="$LOCK" \
            "$0" "$OUT" --shared
    fi
    exec "$REPO/scripts/measure.sh" \
        env BENCH_NIGHT_UNDER_LOCK=1 BUMBLEDB_MEASURE_LOCK="$LOCK" \
        "$0" "$OUT"
fi

# --- SHARED-MACHINE MODE (owner ruling, 2026-07-20) -------------------------
if [ "$SHARED" -eq 1 ] && [ "$PLAN" -eq 0 ]; then
    export BUMBLEDB_BENCH_BOOST=1
    echo "#############################################################"
    echo "##  SHARED-MACHINE NIGHT — scheduler boost ACTIVE          ##"
    echo "##  every lane runs with BUMBLEDB_BENCH_BOOST=1            ##"
    echo "##  (user-interactive QoS; owner ruling 2026-07-20).       ##"
    echo "##  The idle-machine requirement is WAIVED for this run;   ##"
    echo "##  the measurement mutex is still held. Every report      ##"
    echo "##  stamps shared_machine provenance (boost + load avgs).  ##"
    echo "#############################################################"
fi

if [ "$PLAN" -eq 0 ]; then
    (cd "$REPO" && cargo build --release -p bumbledb-bench)
    (cd "$REPO" && CARGO_TARGET_DIR="$OBS_TARGET" \
        cargo build --release -p bumbledb-bench --features obs)
fi

ANY_RUN=0
while IFS='|' read -r id artifact command; do
    if [ "$artifact" = "SETUP" ] || [ "$artifact" = "PREREQ" ]; then
        continue
    fi
    if [ "$(nonsetup_status "$id" "$artifact")" = "RUN" ]; then
        ANY_RUN=1
    fi
done < <(lane_table)

setup_status() {
    if [ "$ANY_RUN" -eq 1 ]; then
        echo "RUN"
    else
        echo "SKIP-UNNEEDED"
    fi
}

header() {
    echo "bumbledb bench night"
    echo "date: $(date '+%Y-%m-%dT%H:%M:%S')"
    echo "rev: $(git -C "$REPO" rev-parse --short HEAD)"
    echo "out: $OUT"
    if [ "$SHARED" -eq 1 ]; then
        echo "mode: shared-machine (boosted; idle-machine requirement waived)"
    fi
    echo ""
}

if [ "$PLAN" -eq 1 ]; then
    header
    while IFS='|' read -r id artifact command; do
        if [ "$artifact" = "SETUP" ]; then
            status="$(setup_status)"
        else
            status="$(nonsetup_status "$id" "$artifact")"
        fi
        printf '%s\t%s\t%s\n' "$id" "$status" "$artifact"
    done < <(lane_table)
    exit 0
fi

mkdir -p "$OUT"
TAB="$(printf '\t')"
NL="
"
LANE_LINES=""
FAILED=0
while IFS='|' read -r id artifact command; do
    if [ "$artifact" = "SETUP" ]; then
        status="$(setup_status)"
    else
        status="$(nonsetup_status "$id" "$artifact")"
    fi
    if [ "$status" = "RUN" ]; then
        echo "[$(date '+%Y-%m-%dT%H:%M:%S')] === lane $id"
        set +e
        eval "$command"
        rc=$?
        set -e
        if [ "$rc" -eq 0 ]; then
            status="RUN-OK"
        else
            status="RUN-FAIL(exit=$rc)"
            FAILED=$((FAILED + 1))
        fi
    else
        echo "[$(date '+%Y-%m-%dT%H:%M:%S')] === lane $id $status"
    fi
    LANE_LINES="${LANE_LINES}${id}${TAB}${status}${TAB}${artifact}${NL}"
done < <(lane_table)

set +e
python3 "$REPO/scripts/bench_viz.py" --night "$OUT" --out "$OUT"
viz_rc=$?
set -e
if [ "$viz_rc" -ne 0 ]; then
    echo "bench-night: warning — bench_viz.py exited $viz_rc" >&2
fi
CHARTS=$( (ls "$OUT"/*.svg 2>/dev/null || true) | wc -l | tr -d ' ')

{
    header
    printf '%s' "$LANE_LINES"
    echo "charts: $CHARTS svg"
    if [ "$FAILED" -eq 0 ]; then
        echo "night: COMPLETE"
    else
        echo "night: INCOMPLETE ($FAILED lanes failed)"
    fi
} > "$OUT/MANIFEST.txt"
cat "$OUT/MANIFEST.txt"

if [ "$FAILED" -eq 0 ]; then
    exit 0
fi
exit 1
