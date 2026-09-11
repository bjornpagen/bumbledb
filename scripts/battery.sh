#!/usr/bin/env bash
# Everyday/static/fault spine. Exit 0 is not all-platform qualification.
# Required S3/Graviton/G15 cells stay NotRun until they actually execute
# against the candidate. See .config/obligation-inventory.json.
set -euo pipefail

cd "$(dirname "$0")/.."

if [ ! -f .git/bumbledb-build.json ]; then
  BUMBLEDB_FAMILY_BASE="$(mktemp -d "${TMPDIR:-/tmp}/bumbledb-battery.XXXXXX")"
  node scripts/build-family.mjs --out "$BUMBLEDB_FAMILY_BASE/ready"
  exec "$BUMBLEDB_FAMILY_BASE/ready/source/scripts/battery.sh"
fi
node scripts/build-family.mjs --check-source
test -f ../family.json
export CARGO_TARGET_DIR="$PWD/target"

echo "==> benchmark scheduler regressions (no measurements)"
python3 -m unittest discover -s scripts -p 'test_bench_*.py'

echo "==> release-evidence checker regressions (not release qualification)"
node --test scripts/release-results.test.mjs
node --test scripts/release-ready.test.mjs

echo "==> isolated consumer toolchain regressions"
node --test scripts/packed-project.test.mjs scripts/build-family.test.mjs

echo "==> product absence gate (ts/scripts/absence-gate.ts)"
node ts/scripts/absence-gate.ts

echo "==> cargo fmt --all --check"
cargo fmt --all --check

echo "==> cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

# The grammar core compiles dependency-lean: store off drops object_store and tokio.
echo "==> cargo check -p bumbledb-log --no-default-features"
cargo check -p bumbledb-log --no-default-features

# ts/crate is workspace-excluded; the bridge lane is its gate.
echo "==> bridge: cargo fmt --check (ts/crate)"
cargo fmt --manifest-path ts/crate/Cargo.toml --check

echo "==> bridge: cargo clippy --all-targets -- -D warnings (ts/crate)"
cargo clippy --manifest-path ts/crate/Cargo.toml --all-targets -- -D warnings

echo "==> native artifact belongs to this immutable build source"
node scripts/release-results.mjs --verify-native-provenance

echo "==> cargo nextest run --workspace"
cargo nextest --version || cargo install cargo-nextest --version 0.9.143 --locked
cargo nextest run --workspace

# Feature-gated core/bench lanes and native-profile exporter checks.
# Renderer checks do not establish measured performance.
echo "==> scripts/check.sh"
scripts/check.sh

# Independent arithmetic and endpoint oracle; native corpus replay runs above.
echo "==> independent structural corpus"
python3 scripts/structural-corpus.py

echo "==> bridge: Rust tests in the parallel process pool (ts/crate)"
cargo nextest run --manifest-path ts/crate/Cargo.toml --config-file .config/nextest.toml

# The bridge is cdylib-only; rustdoc cannot run doctests for that target.
# Public Rust examples run in the workspace doctests above, and SDK examples
# run through the TypeScript and packed-consumer lanes below.

echo "==> ts/ (test, typecheck, lint; no second native rebuild)"
(cd ts && node --test 'test/**/*.test.ts' && pnpm typecheck && pnpm lint)

echo "==> ts-log/ (test, typecheck, lint; no second native rebuild)"
(cd ts-log && node --test 'test/**/*.test.ts' && pnpm typecheck && pnpm lint)

# The family was exposed only after the packed consumer gate passed.
# Its exact tarball hashes and selected source tree are in ../family.json.

echo "==> battery complete for this host — not all-platform qualification; evidence remains NotRun until pre-promotion validates real cells"
