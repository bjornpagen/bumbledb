#!/usr/bin/env bash
# Green is the exit code of this script. Every consumer invokes it.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> cargo fmt --all --check"
cargo fmt --all --check

echo "==> cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> cargo nextest run --workspace"
cargo nextest --version || cargo install cargo-nextest --locked
cargo nextest run --workspace

# Lanes 1–3 are gone from scripts/check.sh.
echo "==> scripts/check.sh"
scripts/check.sh

echo "==> scripts/lean.sh"
scripts/lean.sh

# lean.sh runs census; this is the second call until lean drops its internal census.
echo "==> scripts/spec-census.sh"
scripts/spec-census.sh

echo "==> ts/ (test, typecheck, lint)"
(cd ts && pnpm test && pnpm typecheck && pnpm lint)

echo "==> ts-log/ (test, typecheck, lint)"
(cd ts-log && pnpm test && pnpm typecheck && pnpm lint)

echo "==> battery green"
