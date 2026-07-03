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

echo "==> allocation gate (release)"
cargo test --features alloc-counter --test alloc_gate --release

# The bench crate must build and lint with the engine's observability on
# (docs/benchmarks/13); the harness tests run under both configs.
echo "==> bumbledb-bench with the obs feature (clippy + harness tests)"
cargo clippy -p bumbledb-bench --features obs --all-targets -- -D warnings
cargo test -p bumbledb-bench --features obs harness

# The x86-64 scalar-fallback check (docs/architecture/00-product.md): needs
# a cross std for the pinned toolchain; report skip-vs-pass honestly. The
# sysroot probe (not `rustup target list`) is the truth about whether the
# cross std actually exists for a source-built toolchain.
SYSROOT="$(rustc --print sysroot)"
if [ -d "$SYSROOT/lib/rustlib/x86_64-unknown-linux-gnu/lib" ] \
    && ls "$SYSROOT/lib/rustlib/x86_64-unknown-linux-gnu/lib"/libstd-*.rlib >/dev/null 2>&1; then
    echo "==> cargo check --workspace --target x86_64-unknown-linux-gnu"
    cargo check --workspace --target x86_64-unknown-linux-gnu
else
    echo "==> SKIPPED: x86_64-unknown-linux-gnu cross check (no cross std for this toolchain)"
fi

echo "==> all gates green"
