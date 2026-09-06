#!/usr/bin/env bash
# Everyday/static/fault spine. Exit 0 is not all-platform qualification.
# Required S3/Graviton/G15 cells stay NotRun until they actually execute
# against the candidate. See .config/obligation-inventory.json.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> release-evidence checker regressions (not release qualification)"
node --test scripts/release-results.test.mjs

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

echo "==> one current-addon build (or proven matching artifact)"
if [ "${BUMBLEDB_SKIP_NATIVE_BUILD:-}" = 1 ]; then
  node scripts/release-results.mjs --verify-native-provenance
  echo "    reused native artifact with matching candidate/spec provenance"
else
  (cd ts && pnpm run build)
  node scripts/release-results.mjs --write-native-provenance
fi

echo "==> cargo nextest run --workspace"
cargo nextest --version || cargo install cargo-nextest --version 0.9.143 --locked
cargo nextest run --workspace

# Feature-gated core/bench lanes. L20 owns flame.py; this is a renderer
# golden, not a G15 timing cell.
echo "==> scripts/check.sh"
scripts/check.sh

# Lean kernel + constructor correspondence (scripts/lean.sh already
# runs scripts/spec-census.sh). lean.sh is not a cargo-test owner —
# correspondence::OWNED_CASES live in crates/bumbledb-bench and run
# under workspace nextest above. No dyn/wording census.
# Current identity goldens run in the workspace conformance_v3 test target.
echo "==> scripts/lean.sh"
scripts/lean.sh

echo "==> bridge: Rust tests in the parallel process pool (ts/crate)"
cargo nextest run --manifest-path ts/crate/Cargo.toml --config-file .config/nextest.toml

# The bridge is cdylib-only; rustdoc cannot run doctests for that target.
# Public Rust examples run in the workspace doctests above, and SDK examples
# run through the TypeScript and packed-consumer lanes below.

echo "==> ts/ (test, typecheck, lint; no second native rebuild)"
(cd ts && node --test 'test/**/*.test.ts' && pnpm typecheck && pnpm lint)

echo "==> ts-log/ (test, typecheck, lint; no second native rebuild)"
(cd ts-log && pnpm run build && node --test 'test/**/*.test.ts' && pnpm typecheck && pnpm lint)

echo "==> packed-tarball import gate (scripts/packed-import.sh)"
# Rust consumer, D07 tiny-collect refusal, D27 addon-unavailable
# authoring, and Notes specimens/routes run inside packed-import.
scripts/packed-import.sh --host-only

echo "==> battery complete for this host — not all-platform qualification; evidence remains NotRun until pre-promotion validates real cells"
