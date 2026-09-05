#!/bin/sh
set -eu

# D07/D22/D27 packed-tarball import gate: every published package is
# STAGED and packed for real (immutable staging — the checkout is never
# mutated), installed from its tarball into a bare consumer, typechecked
# as a strict downstream (core-ts, log-ts, native-ledger), and run under
# ManagedRuntime.make(NativeRuntime.layer(...)) — specimens no longer
# self-provide. A second isolated project imports Scalar authoring with
# the native addon unavailable. Rust consumer and Notes specimens/routes
# run in this path; missing Notes migrations fail, never skip green.
# Local packing is not PKG-07B.

unset NODE_OPTIONS NODE_PATH

cd "$(dirname "$0")/.."
ROOT="$PWD"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/packed-import.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

V="$(node -p "require('$ROOT/ts/package.json').version")"
EFFECT="$(node -p "require('$ROOT/ts/package.json').peerDependencies.effect")"
TYPESCRIPT="$(node -p "require('$ROOT/ts-log/package.json').devDependencies.typescript")"
NODE_TYPES="$(node -p "require('$ROOT/ts-log/package.json').devDependencies['@types/node']")"
STORE="$(cd "$ROOT/ts-log" && pnpm store path --silent)"

cp "$ROOT/ts/package.json" "$TMP/core-package.before.json"
cp "$ROOT/ts-log/package.json" "$TMP/log-package.before.json"
node "$ROOT/ts/scripts/stage.ts" --out "$TMP"
node "$ROOT/ts-log/scripts/stage.ts" --out "$TMP"
cmp -s "$ROOT/ts/package.json" "$TMP/core-package.before.json" || {
  echo "packed-import: FAIL — staging mutated ts/package.json" >&2
  exit 1
}
cmp -s "$ROOT/ts-log/package.json" "$TMP/log-package.before.json" || {
  echo "packed-import: FAIL — staging mutated ts-log/package.json" >&2
  exit 1
}

for tgz in \
  "bjornpagen-bumbledb-$V.tgz" \
  "bjornpagen-bumbledb-log-$V.tgz" \
  "bjornpagen-bumbledb-darwin-arm64-$V.tgz" \
  "bjornpagen-bumbledb-linux-arm64-$V.tgz" \
  "bjornpagen-bumbledb-linux-x64-$V.tgz"; do
  if [ ! -f "$TMP/$tgz" ]; then
    echo "packed-import: FAIL — expected tarball missing: $tgz (platform binaries must be built)" >&2
    exit 1
  fi
  tar -tzf "$TMP/$tgz" | grep -q '^package/package.json$' || {
    echo "packed-import: FAIL — $tgz carries no package.json" >&2
    exit 1
  }
done

tar -tzf "$TMP/bjornpagen-bumbledb-log-$V.tgz" | grep -q '^package/dist/index.js$' || {
  echo "packed-import: FAIL — the ts-log tarball does not carry dist/index.js" >&2
  exit 1
}
tar -tzf "$TMP/bjornpagen-bumbledb-log-$V.tgz" | grep -q '^package/pack-provenance.json$' || {
  echo "packed-import: FAIL — the ts-log tarball does not carry pack provenance" >&2
  exit 1
}
tar -tzf "$TMP/bjornpagen-bumbledb-darwin-arm64-$V.tgz" | grep -q '^package/pack-provenance.json$' || {
  echo "packed-import: FAIL — the darwin-arm64 tarball does not carry pack provenance" >&2
  exit 1
}
PROVENANCE="$(tar -xzOf "$TMP/bjornpagen-bumbledb-$V.tgz" package/pack-provenance.json)"
CANDIDATE="$(node "$ROOT/scripts/release-results.mjs" --candidate-digest)"
SPEC="$(node "$ROOT/scripts/release-results.mjs" --specification-revision)"
node -e "
  const p = JSON.parse(process.argv[1]);
  if (p.candidateSourceDigest !== process.argv[2]) {
    console.error('packed-import: FAIL — tarball candidateSourceDigest does not match the current candidate inventory');
    process.exit(1);
  }
  if (p.specificationRevision !== process.argv[3]) {
    console.error('packed-import: FAIL — tarball specificationRevision does not match obligation inventory');
    process.exit(1);
  }
" "$PROVENANCE" "$CANDIDATE" "$SPEC" || exit 1
tar -tzf "$TMP/bjornpagen-bumbledb-log-$V.tgz" | grep -q '^package/dist/migrations/bin.js$' || {
  echo "packed-import: FAIL — the ts-log tarball does not carry the bumbledb-log CLI" >&2
  exit 1
}

mkdir "$TMP/consumer"
cat > "$TMP/consumer/package.json" <<JSON
{
	"name": "packed-import-consumer",
	"private": true,
	"type": "module",
	"dependencies": {
		"@bjornpagen/bumbledb": "file:../bjornpagen-bumbledb-$V.tgz",
		"@bjornpagen/bumbledb-log": "file:../bjornpagen-bumbledb-log-$V.tgz",
		"effect": "$EFFECT"
	},
	"devDependencies": {
		"@types/node": "$NODE_TYPES",
		"typescript": "$TYPESCRIPT"
	}
}
JSON
cat > "$TMP/consumer/pnpm-workspace.yaml" <<YAML
packages:
  - "."
overrides:
  "@bjornpagen/bumbledb-darwin-arm64": "file:../bjornpagen-bumbledb-darwin-arm64-$V.tgz"
  "@bjornpagen/bumbledb-linux-arm64": "file:../bjornpagen-bumbledb-linux-arm64-$V.tgz"
  "@bjornpagen/bumbledb-linux-x64": "file:../bjornpagen-bumbledb-linux-x64-$V.tgz"
YAML

cp "$ROOT/scripts/packed-consumer.ts" "$TMP/consumer/packed-consumer.ts"
mkdir -p "$TMP/consumer/core-ts" "$TMP/consumer/log-ts" "$TMP/consumer/native-ledger"
cp "$ROOT/examples/consumers/core-ts/consumer.ts" "$TMP/consumer/core-ts/consumer.ts"
cp "$ROOT/examples/consumers/log-ts/consumer.ts" "$TMP/consumer/log-ts/consumer.ts"
cp "$ROOT/examples/consumers/native-ledger/consumer.ts" "$TMP/consumer/native-ledger/consumer.ts"
(cd "$TMP/consumer" && pnpm install --ignore-scripts --store-dir "$STORE" --prefer-offline --reporter=append-only)

# No skipLibCheck, workspace path aliases, custom conditions, or repo compiler.
(cd "$TMP/consumer" && pnpm exec tsc --strict --exactOptionalPropertyTypes --target es2024 \
  --module nodenext --types node --allowImportingTsExtensions \
  --declaration --emitDeclarationOnly --outDir declarations \
  packed-consumer.ts core-ts/consumer.ts log-ts/consumer.ts native-ledger/consumer.ts)
if grep -REq '(node_modules|\.pnpm|import\("/|from "/|"(file|link):)' "$TMP/consumer/declarations"; then
  echo "packed-import: FAIL — consumer declarations leaked a private installation path" >&2
  exit 1
fi
# D22: programs that no longer self-provide run under ManagedRuntime.
(cd "$TMP/consumer" && node packed-consumer.ts)

# D27: second isolated project — no platform overrides; optional native off.
mkdir "$TMP/pure"
cat > "$TMP/pure/package.json" <<JSON
{
	"name": "packed-pure-authoring",
	"private": true,
	"type": "module",
	"dependencies": {
		"@bjornpagen/bumbledb": "file:../bjornpagen-bumbledb-$V.tgz",
		"effect": "$EFFECT"
	},
	"devDependencies": {
		"@types/node": "$NODE_TYPES",
		"typescript": "$TYPESCRIPT"
	}
}
JSON
cat > "$TMP/pure/.npmrc" <<EOF
optional=false
EOF
cat > "$TMP/pure/pnpm-workspace.yaml" <<YAML
packages:
  - "."
YAML
cp "$ROOT/scripts/packed-pure-authoring.ts" "$TMP/pure/packed-pure-authoring.ts"
(cd "$TMP/pure" && pnpm install --ignore-scripts --store-dir "$STORE" --prefer-offline --reporter=append-only --config.optional=false)
for plat in darwin-arm64 linux-arm64 linux-x64; do
  if [ -e "$TMP/pure/node_modules/@bjornpagen/bumbledb-$plat" ]; then
    echo "packed-import: FAIL — D27 pure cell resolved native package @bjornpagen/bumbledb-$plat" >&2
    exit 1
  fi
done
(cd "$TMP/pure" && pnpm exec tsc --strict --exactOptionalPropertyTypes --target es2024 \
  --module nodenext --types node --allowImportingTsExtensions \
  --declaration --emitDeclarationOnly --outDir declarations \
  packed-pure-authoring.ts)
(cd "$TMP/pure" && node packed-pure-authoring.ts)

# Rust consumer lives in this packed-import path (D07 tiny collect refuses).
cargo run --manifest-path "$ROOT/examples/consumers/rust/Cargo.toml"

# Notes specimens + routes: missing generated migrations FAIL, never skip green.
(cd "$ROOT/examples/notes" && node --test test/specimens.test.ts test/routes.test.ts)

echo "packed-import: OK — 5 staged tarballs; ManagedRuntime consumer; D07 tiny collect refuses; D27 addon-unavailable authoring; Rust + Notes fail-closed at $V (not PKG-07B)"
