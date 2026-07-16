#!/bin/sh
# The gate suite (docs/architecture/00-product.md success criterion 3 and
# 50-validation's sanctioned gates). Run before every commit; CI, when it
# exists, runs exactly this.
set -eu

cd "$(dirname "$0")/.."

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

# The ground-off and fold-off fuzz-oracle features are load-bearing for
# the fuzz crate's rewrites dual-pipeline differential (the crucible
# packet (git ecec1dc3)): the engine suite must stay green with each off
# switch compiled in. The default workspace test/clippy invocations above
# are the feature-off matrix; these are the feature-on matrix.
echo "==> bumbledb with the ground-off fuzz-oracle feature (clippy + tests)"
cargo clippy -p bumbledb --all-targets --features ground-off -- -D warnings
cargo test -p bumbledb --features ground-off

echo "==> bumbledb with the fold-off fuzz-oracle feature (tests)"
cargo test -p bumbledb --features fold-off

# The fuzz crate is detached from the workspace on purpose (the
# crucible packet (git ecec1dc3)): `cargo fuzz` builds its targets, so
# every --workspace invocation above skips fuzz/src entirely — a
# breakage there would pass this gate unseen (the fixit record). This
# lane compiles and lints it; the corpus REPLAY (plain `cargo test` in
# fuzz/, ~8 min) stays a CI lane, not a per-commit gate.
echo "==> fuzz crate (out-of-workspace): clippy -D warnings"
cargo clippy --manifest-path fuzz/Cargo.toml --all-targets -- -D warnings

# The deterministic crashpoint sweeps — durable and ephemeral — ARE a
# per-commit gate (the fixit record): the ephemeral admission's reversal
# clause ("reverses if the crash sweep ever convicts a crashpoint on an
# ephemeral store") needs a standing executed lane, and the sweeps are
# SECONDS (~1s durable, ~0.5s ephemeral) — this is NOT the ~8-min corpus
# replay, which stays a CI lane. The filter matches both sweep parents
# and excludes the replay test and the ignored crash-child body.
echo "==> fuzz crate: deterministic crashpoint sweeps (durable + ephemeral)"
cargo test --manifest-path fuzz/Cargo.toml --test crash every_crashpoint_recovers

# The WRITEMAP commit-window kill smoke (fuzz/tests/kill.rs): the
# crashpoint sweeps cut everywhere EXCEPT inside mdb_txn_commit itself,
# and inside that window is exactly where WRITEMAP changes the write
# pattern — so ~30 random-timing SIGKILLs (durable control + ephemeral)
# run per commit (~5-8s), each corpse autopsied for all-or-nothing
# recovery. The statistical lane (>= 2,000 kills/kind) is the #[ignore]d
# long variant, recorded in fuzz/SESSIONS.md.
echo "==> fuzz crate: random-timing kill smoke (durable + ephemeral)"
cargo test --manifest-path fuzz/Cargo.toml --test kill random_kills_recover_on_both_kinds_smoke

# The bench crate must build and lint with the engine's observability on
# (docs/architecture/60-validation.md); the harness tests run under both configs.
# `the_engine_trace_pins`: the displaced lane's regime label observed on
# the engine's own colt_force trace — obs-gated, so it only runs here.
echo "==> bumbledb-bench with the obs feature (clippy + harness tests)"
cargo clippy -p bumbledb-bench --features obs --all-targets -- -D warnings
cargo test -p bumbledb-bench --features obs -- harness trace_out tripwires the_engine_trace_pins

# The x86-64 scalar-fallback check (docs/architecture/00-product.md):
# report skip-vs-pass honestly. It needs the WHOLE cross toolchain, not
# just the Rust std: the engine links LMDB's C sources at build-script
# time, so `cargo check` cross-compiles C — a cross std without a cross
# C compiler can only fail environmentally, which is a skip, not a
# verdict. Engine crates only: the portable-fallback promise is the
# engine's — the bench harness is host tooling (SQLite, host-timed).
SYSROOT="$(rustc --print sysroot)"
CROSS_CC="${CC_x86_64_unknown_linux_gnu:-x86_64-linux-gnu-gcc}"
if [ -d "$SYSROOT/lib/rustlib/x86_64-unknown-linux-gnu/lib" ] \
    && ls "$SYSROOT/lib/rustlib/x86_64-unknown-linux-gnu/lib"/libstd-*.rlib >/dev/null 2>&1 \
    && command -v "$CROSS_CC" >/dev/null 2>&1; then
    echo "==> cargo check -p bumbledb -p bumbledb-macros -p bumbledb-query --target x86_64-unknown-linux-gnu"
    cargo check -p bumbledb -p bumbledb-macros -p bumbledb-query --target x86_64-unknown-linux-gnu
else
    echo "==> SKIPPED: x86_64-unknown-linux-gnu cross check (cross std or cross C compiler missing)"
fi

echo "==> all gates green"
