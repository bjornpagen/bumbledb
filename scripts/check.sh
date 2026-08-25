#!/bin/sh
# Remainder scripts/battery.sh calls after workspace nextest.
set -eu

cd "$(dirname "$0")/.."

filtered_test() {
    log=$(cargo test "$@" 2>&1) || {
        printf '%s\n' "$log" >&2
        exit 1
    }
    printf '%s\n' "$log"
    passed=$(printf '%s\n' "$log" \
        | sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' \
        | awk '{ s += $1 } END { print s + 0 }')
    if [ "$passed" -eq 0 ]; then
        echo "check.sh: FAIL — 'cargo test $*' matched zero tests (the vacuous pass)" >&2
        exit 1
    fi
}

echo "==> cargo test --workspace --doc"
cargo test --workspace --doc

echo "==> allocation gate (release): steady-state + escalating high-water"

cargo test --features alloc-counter --test alloc_gate --release -- --test-threads=1

echo "==> bumbledb with the ground-off test-support feature (clippy + tests)"
cargo clippy -p bumbledb --all-targets --features ground-off -- -D warnings
cargo test -p bumbledb --features ground-off

echo "==> bumbledb with the trace feature (tests)"
cargo test -p bumbledb --features trace

echo "==> bumbledb --all-features (clippy, the pairwise co-compile check)"
cargo clippy -p bumbledb --all-targets --all-features -- -D warnings

echo "==> bumbledb-bench with the obs feature (clippy + harness tests)"
cargo clippy -p bumbledb-bench --features obs --all-targets -- -D warnings

filtered_test -p bumbledb-bench --features obs -- harness
filtered_test -p bumbledb-bench --features obs -- trace_out
filtered_test -p bumbledb-bench --features obs -- tripwires
filtered_test -p bumbledb-bench --features obs -- the_engine_trace_pins

filtered_test -p bumbledb-bench --features obs -- traced_
filtered_test -p bumbledb-bench --features obs -- the_alloc_pass

echo "==> flame renderer golden selftest"
python3 scripts/flame.py selftest

echo "==> all gates green"
