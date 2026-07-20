#!/usr/bin/env bash
# bench-night.sh — the one-command night orchestrator.
#
# The night is a TABLE, not a script of branches: an ordered lane registry
# (id | artifact | command), cheap first, adversarial + churn last.
# Skip/resume, availability probing, the manifest, and --plan are all folds
# over that one table.
#
# The measurement mutex is measure.sh's, reused verbatim: this script
# re-execs itself under scripts/measure.sh rather than forking its lock
# logic; the whole night runs inside ONE hold. Before waiting it REFUSES
# if the lock is already held (a night must never queue behind another
# measurement). The lock path is the single overridable parameter
# BUMBLEDB_MEASURE_LOCK, shared with measure.sh, so the refusal is testable
# without ever touching the real lock.
#
# usage: bench-night.sh <out-dir> [--plan]
#
#   --plan  print the manifest-style lane table with the statuses the run
#           WOULD have (RUN / SKIP-EXISTING / SKIP-UNAVAILABLE); no lock,
#           no build, no lane execution, no viz. Exit 0.
#
# Exit codes: 0 night complete (SKIPs are not failures); 1 at least one
# lane RUN-FAILed; 2 usage or lock refusal.
set -euo pipefail

usage() {
    echo "usage: bench-night.sh <out-dir> [--plan]" >&2
    exit 2
}

[ "$#" -ge 1 ] || usage
[ -n "$1" ] || usage

REPO="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$REPO/target/release/bumbledb-bench"
OBS_TARGET="$REPO/target/bench-obs"
OBS_BIN="$OBS_TARGET/release/bumbledb-bench"
LOCK="${BUMBLEDB_MEASURE_LOCK:-/tmp/bumbledb.measure.lock}"

OUT_ARG=$1
shift
PLAN=0
for arg in "$@"; do
    case "$arg" in
        --plan) PLAN=1 ;;
        *) usage ;;
    esac
done

case "$OUT_ARG" in
    -*) usage ;;
    /*) OUT="$OUT_ARG" ;;
    *) OUT="$PWD/$OUT_ARG" ;;
esac

# --- THE LANE TABLE -------------------------------------------------------
# Fields: id|artifact|command. Order is the law: cheap lanes first,
# adversarial + churn LAST (their failures cost nothing upstream).
# SETUP rows (artifact=SETUP) run only when at least one non-SETUP lane
# will actually RUN — gen is cheap-or-required, verify re-earns the stamp
# the bench subcommand refuses without.
#
# PROBED lanes are the expansion lanes: available iff the binary's help
# lists the subcommand (token == lane id). Live spellings as of this
# writing: storage, curves, writes, churn landed (write-throughput landed
# as `writes`; the cold/warm/memoized panel landed as `curves --warmth`,
# so that lane is folded into the curves row rather than kept as a
# never-landing spelling). `adversarial` has not landed yet and keeps its
# contract spelling — the probe reports it SKIP-UNAVAILABLE until it does.
PROBED=" storage curves writes adversarial churn "

lane_table() {
    cat <<EOF
gen|SETUP|"$BIN" gen
verify|SETUP|"$BIN" verify
bench-durable-r1|$OUT/bench-durable-r1/report.json|"$BIN" bench --out "$OUT/bench-durable-r1"
bench-durable-r2|$OUT/bench-durable-r2/report.json|"$BIN" bench --out "$OUT/bench-durable-r2"
bench-durable-r3|$OUT/bench-durable-r3/report.json|"$BIN" bench --out "$OUT/bench-durable-r3"
bench-ephemeral-r1|$OUT/bench-ephemeral-r1/report.json|"$BIN" bench --ephemeral --out "$OUT/bench-ephemeral-r1"
bench-ephemeral-r2|$OUT/bench-ephemeral-r2/report.json|"$BIN" bench --ephemeral --out "$OUT/bench-ephemeral-r2"
bench-ephemeral-r3|$OUT/bench-ephemeral-r3/report.json|"$BIN" bench --ephemeral --out "$OUT/bench-ephemeral-r3"
scenarios|$OUT/scenarios/scenarios.md|"$BIN" scenarios --out "$OUT/scenarios"
sweep-commit|$OUT/sweep-commit/sweep.md|mkdir -p "$OUT/sweep-commit" && "$OBS_BIN" sweep-commit > "$OUT/sweep-commit/sweep.md"
storage|$OUT/storage/storage-report.json|"$BIN" storage --out "$OUT/storage"
curves|$OUT/curves/curves-report.json|"$BIN" curves --warmth --out "$OUT/curves"
writes|$OUT/writes/writes-report.json|"$BIN" writes --out "$OUT/writes"
adversarial|$OUT/adversarial/report.json|"$BIN" adversarial --out "$OUT/adversarial"
churn|$OUT/churn/churn-report.json|"$BIN" churn --out "$OUT/churn"
EOF
}

is_probed() {
    case "$PROBED" in
        *" $1 "*) return 0 ;;
    esac
    return 1
}

# Available iff the binary's help lists the subcommand (the token equals
# the lane id here). A missing binary makes every probed lane unavailable.
lane_available() {
    "$BIN" help 2>/dev/null | awk '/^COMMANDS:/,/^$/' \
        | grep -qE "^[[:space:]]+$1([[:space:]]|\$)"
}

# The planned status of one non-SETUP lane: an existing artifact is never
# rerun (a crashed night resumes by rerunning the same command).
nonsetup_status() { # $1=id $2=artifact
    if [ -e "$2" ]; then
        echo "SKIP-EXISTING"
    elif is_probed "$1" && ! lane_available "$1"; then
        echo "SKIP-UNAVAILABLE"
    else
        echo "RUN"
    fi
}

# --- MUTEX REFUSAL + ACQUISITION (skipped in --plan) -----------------------
# Refuse-before-wait: measure.sh would merely queue; the night must not.
# The refuse-then-exec race window is accepted (measure.sh would wait,
# and refusal-before-wait is the required semantics). The re-exec'd child
# runs under our own hold and must not refuse it.
if [ "$PLAN" -eq 0 ] && [ "${BENCH_NIGHT_UNDER_LOCK:-}" != 1 ]; then
    if [ -d "$LOCK" ]; then
        echo "bench-night: refusing — measurement lock held (holder: $(cat "$LOCK/holder" 2>/dev/null || echo unknown))" >&2
        exit 2
    fi
    export BUMBLEDB_MEASURE_LOCK="$LOCK"
    exec "$REPO/scripts/measure.sh" \
        env BENCH_NIGHT_UNDER_LOCK=1 BUMBLEDB_MEASURE_LOCK="$LOCK" \
        "$0" "$OUT"
fi

# --- BUILD (building is not measurement; skipped in --plan) ----------------
if [ "$PLAN" -eq 0 ]; then
    (cd "$REPO" && cargo build --release -p bumbledb-bench)
    (cd "$REPO" && CARGO_TARGET_DIR="$OBS_TARGET" \
        cargo build --release -p bumbledb-bench --features obs)
fi

# --- PASS 1: does any non-SETUP lane RUN? (decides the SETUP rows) ---------
ANY_RUN=0
while IFS='|' read -r id artifact command; do
    if [ "$artifact" = "SETUP" ]; then
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
    echo ""
}

# --- --plan MODE: the table alone, nothing executed ------------------------
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

# --- EXECUTION: one fold over the table ------------------------------------
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

# --- VIZ (runs even if some lanes failed; bench_viz SKIPs missing lanes) ---
set +e
python3 "$REPO/scripts/bench_viz.py" --night "$OUT" --out "$OUT"
viz_rc=$?
set -e
if [ "$viz_rc" -ne 0 ]; then
    echo "bench-night: warning — bench_viz.py exited $viz_rc" >&2
fi
CHARTS=$( (ls "$OUT"/*.svg 2>/dev/null || true) | wc -l | tr -d ' ')

# --- MANIFEST ---------------------------------------------------------------
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
