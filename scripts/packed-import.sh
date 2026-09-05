#!/bin/sh
set -eu

# The packed-tarball import gate: every published package is STAGED and
# packed for real (immutable staging — the checkout is never mutated),
# installed from its tarball into a bare consumer, typechecked as a strict
# downstream (including the chapter 34 consumer fixtures), and imported in
# a fresh node process — so missing package files, private declaration
# paths, duplicated peer identities and source-mutating pack behavior
# cannot hide behind the repo's source resolution.

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
mkdir -p "$TMP/consumer/core-ts" "$TMP/consumer/log-ts"
cp "$ROOT/examples/consumers/core-ts/consumer.ts" "$TMP/consumer/core-ts/consumer.ts"
cp "$ROOT/examples/consumers/log-ts/consumer.ts" "$TMP/consumer/log-ts/consumer.ts"
(cd "$TMP/consumer" && pnpm install --ignore-scripts --store-dir "$STORE" --prefer-offline --reporter=append-only)

# No skipLibCheck, workspace path aliases, custom conditions, or repo compiler.
(cd "$TMP/consumer" && pnpm exec tsc --strict --exactOptionalPropertyTypes --target es2024 \
  --module nodenext --types node --allowImportingTsExtensions \
  --declaration --emitDeclarationOnly --outDir declarations \
  packed-consumer.ts core-ts/consumer.ts log-ts/consumer.ts)
if grep -REq '(node_modules|\.pnpm|import\("/|from "/|"(file|link):)' "$TMP/consumer/declarations"; then
  echo "packed-import: FAIL — consumer declarations leaked a private installation path" >&2
  exit 1
fi
(cd "$TMP/consumer" && node packed-consumer.ts)

echo "packed-import: OK — 5 staged tarballs; isolated consumer types, chapter-34 fixtures, Effect errors, and core/log identity pass at $V"
