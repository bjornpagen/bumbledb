#!/bin/sh
# The gate suite (docs/architecture/00-product.md success criterion 3 and
# 60-validation's sanctioned gates). Run before every commit; CI's check
# lane (.github/workflows/ci.yml) runs exactly this — on macos-arm64 AND
# on x86_64-linux, where the run itself is the engine's scalar-fallback
# and linux-arm coverage.
set -eu

cd "$(dirname "$0")/.."

# A libtest filter that matches nothing still exits 0 — the vacuous
# pass (the same hazard lean.sh's battery-5 guard refuses). Every
# filtered gate below runs through this: the passed counts are summed
# across the package's test binaries and a zero-match filter fails, so
# a module or test rename can never silently drop the coverage the
# filter names.
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

echo "==> cargo fmt --all --check"
cargo fmt --all --check

echo "==> cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> cargo test --workspace"
cargo test --workspace

echo "==> cargo test --workspace --doc"
cargo test --workspace --doc

echo "==> allocation gate (release): steady-state + escalating high-water"
# Both measured windows of the docs/architecture/40-execution.md gate
# protocol run inside the binary's one test: the fixed-set steady-state
# zero assertion and the escalating high-water variant (allocations only
# on new intermediate high-waters; repeats silent; >=1 growth event
# observed or the run is vacuous).
# --test-threads=1: the counting allocator is process-global; the gate
# binary holds one test by invariant (alloc_gate.rs header), and the
# flag keeps even an accidental second test from turning it flaky.
cargo test --features alloc-counter --test alloc_gate --release -- --test-threads=1

# The ground-off test-support feature is load-bearing for the bench
# crate's dual-run grounding/fold differentials
# (crates/bumbledb-bench/src/differential/tests): the engine suite must
# stay green with the off switch compiled in. What the lane actually
# builds: the workspace invocations above already compile bumbledb WITH
# ground-off — the bench crate's dev-dependency turns it on, and cargo
# unifies features across one build graph — so this -p lane pins that
# coverage independently of the bench dep; the trace lane below (a -p
# graph the bench crate is not in) covers the build with ground-off OFF.
echo "==> bumbledb with the ground-off test-support feature (clippy + tests)"
cargo clippy -p bumbledb --all-targets --features ground-off -- -D warnings
cargo test -p bumbledb --features ground-off

# The trace-gated referee pins live in the ENGINE crate — the
# per-relation arm-selection pins (`api/db/append_tests.rs:
# the_write_path_classifies_deletes_per_relation`, `image/cache/tests.rs:
# counters_pin_the_per_relation_arm_selection`) and the rest of
# `trace_tests.rs` — and the bench-crate obs lane below runs bench tests
# only, so without this lane the pins compile in no gate and the
# appended-across-a-delete instrument is inert.
echo "==> bumbledb with the trace feature (tests)"
cargo test -p bumbledb --features trace

# Every engine feature at once, compiled once (the pairwise co-compile
# check): no other lane co-builds trace with the test-support features,
# so a feature pair broken only in combination would land unseen without
# this line. Clippy compiles every target and runs nothing — the
# alloc-counter global allocator and the trace instrumentation are
# proven to build together, never executed here.
echo "==> bumbledb --all-features (clippy, the pairwise co-compile check)"
cargo clippy -p bumbledb --all-targets --all-features -- -D warnings

# The bench crate must build and lint with the engine's observability on
# (docs/architecture/60-validation.md); the harness tests run under both configs.
# `the_engine_trace_pins`: the displaced lane's regime label observed on
# the engine's own colt_force trace — obs-gated, so it only runs here.
echo "==> bumbledb-bench with the obs feature (clippy + harness tests)"
cargo clippy -p bumbledb-bench --features obs --all-targets -- -D warnings
# One guarded run per filter: this lane is the ONLY execution of the
# obs-gated tests, so each of the four names must match something.
filtered_test -p bumbledb-bench --features obs -- harness
filtered_test -p bumbledb-bench --features obs -- trace_out
filtered_test -p bumbledb-bench --features obs -- tripwires
filtered_test -p bumbledb-bench --features obs -- the_engine_trace_pins

# The x86-64 scalar-fallback promise (docs/architecture/00-product.md)
# is EXECUTED, not cross-checked: CI's check lane runs this whole script
# natively on an x86_64-linux runner (.github/workflows/ci.yml), which
# is strictly stronger than the cross `cargo check` that used to sit
# here — that check needed a cross std AND a cross C compiler (the
# engine links LMDB's C at build-script time), so it self-skipped on
# every machine that ever ran it, reference host and CI runner alike.

echo "==> all gates green"
