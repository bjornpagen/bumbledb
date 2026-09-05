#!/bin/sh
set -eu

# The packed-tarball import gate: every published package is packed for
# real, installed from its tarball into a bare consumer, and imported in
# a fresh node process, with a strict consumer declaration emit — so
# missing package files, private declaration paths, and duplicated peer
# identities cannot hide behind the repo's source resolution.

# Source conditions, preloads, and global module paths must not turn this
# outside-consumer check into another workspace test.
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
(cd "$ROOT/ts" && pnpm pack --pack-destination "$TMP" >/dev/null)
cmp -s "$ROOT/ts/package.json" "$TMP/core-package.before.json" || {
  echo "packed-import: FAIL — core prepack/postpack did not restore the manifest" >&2
  exit 1
}
(cd "$ROOT/ts-log" && pnpm pack --pack-destination "$TMP" >/dev/null)
for plat in darwin-arm64 linux-arm64 linux-x64; do
  (cd "$ROOT/ts/npm/$plat" && pnpm pack --pack-destination "$TMP" >/dev/null)
done

for tgz in \
  "bjornpagen-bumbledb-$V.tgz" \
  "bjornpagen-bumbledb-log-$V.tgz" \
  "bjornpagen-bumbledb-darwin-arm64-$V.tgz" \
  "bjornpagen-bumbledb-linux-arm64-$V.tgz" \
  "bjornpagen-bumbledb-linux-x64-$V.tgz"; do
  if [ ! -f "$TMP/$tgz" ]; then
    echo "packed-import: FAIL — expected tarball missing: $tgz" >&2
    exit 1
  fi
  tar -tzf "$TMP/$tgz" | grep -q '^package/package.json$' || {
    echo "packed-import: FAIL — $tgz carries no package.json" >&2
    exit 1
  }
done

tar -tzf "$TMP/bjornpagen-bumbledb-log-$V.tgz" | grep -q '^package/dist/index.js$' || {
  echo "packed-import: FAIL — the ts-log tarball does not carry dist/index.js; nitro would inline raw .ts and hosted node cannot load it" >&2
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
(cd "$TMP/consumer" && pnpm install --ignore-scripts --store-dir "$STORE" --prefer-offline --reporter=append-only)

# No skipLibCheck, workspace path aliases, custom conditions, or repo compiler.
(cd "$TMP/consumer" && pnpm exec tsc --strict --target es2024 --module nodenext --types node \
  --declaration --emitDeclarationOnly --outDir declarations packed-consumer.ts)
if grep -Eq '(node_modules|\.pnpm|import\("/|from "/|"(file|link):)' "$TMP/consumer/declarations/packed-consumer.d.ts"; then
  echo "packed-import: FAIL — consumer declarations leaked a private installation path" >&2
  exit 1
fi
(cd "$TMP/consumer" && node packed-consumer.ts)

echo "packed-import: OK — 5 tarballs packed; isolated consumer types, Effect errors, and core/log identity pass at $V"
