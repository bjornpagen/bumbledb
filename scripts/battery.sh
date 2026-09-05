#!/usr/bin/env bash
# Green is the exit code of this script. Every consumer invokes it.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> cargo fmt --all --check"
cargo fmt --all --check

echo "==> cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

# The grammar core compiles dependency-lean: store off drops object_store and tokio.
echo "==> cargo check -p bumbledb-log --no-default-features"
cargo check -p bumbledb-log --no-default-features

echo "==> cargo nextest run --workspace"
cargo nextest --version || cargo install cargo-nextest --version 0.9.143 --locked
cargo nextest run --workspace

# Lanes 1–3 are gone from scripts/check.sh.
echo "==> scripts/check.sh"
scripts/check.sh

echo "==> scripts/lean.sh"
scripts/lean.sh

# lean.sh runs census; this is the second call until lean drops its internal census.
echo "==> scripts/spec-census.sh"
scripts/spec-census.sh

# ts/crate is workspace-excluded (its own build system), so the workspace
# fmt/clippy lanes never see it; the bridge lane is its gate. The build
# lands a fresh dist ahead of the TS lanes, so a stale gitignored ts/dist
# is unrepresentable as their input.
echo "==> bridge: cargo fmt --check (ts/crate)"
cargo fmt --manifest-path ts/crate/Cargo.toml --check

echo "==> bridge: cargo clippy --all-targets -- -D warnings (ts/crate)"
cargo clippy --manifest-path ts/crate/Cargo.toml --all-targets -- -D warnings

echo "==> bridge: Rust tests in the parallel process pool (ts/crate)"
cargo nextest run --manifest-path ts/crate/Cargo.toml --config-file .config/nextest.toml

echo "==> bridge: Rust documentation tests (ts/crate)"
cargo test --manifest-path ts/crate/Cargo.toml --doc

echo "==> bridge: .node build (ts/scripts/build.ts)"
(cd ts && pnpm run build)

echo "==> ts/ (test, typecheck, lint)"
(cd ts && pnpm test && pnpm typecheck && pnpm lint)

echo "==> ts-log/ (test, typecheck, lint)"
(cd ts-log && pnpm test && pnpm typecheck && pnpm lint)

echo "==> packed-tarball import gate (scripts/packed-import.sh)"
scripts/packed-import.sh

echo "==> battery green"
