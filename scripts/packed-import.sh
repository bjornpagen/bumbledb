#!/bin/sh
set -eu

# The packed-tarball import gate: every published package is packed for
# real, installed from its tarball into a bare consumer, and imported in
# a fresh node process — so a module-relative read that escapes a
# package's files roster cannot publish. The repo tree never enters the
# consumer's resolution: what installs is exactly what npm would serve.

cd "$(dirname "$0")/.."
ROOT="$PWD"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/packed-import.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

V="$(node -p "require('$ROOT/ts/package.json').version")"
STORE="$(cd "$ROOT/ts-log" && pnpm store path --silent)"

(cd "$ROOT/ts" && pnpm pack --pack-destination "$TMP" >/dev/null)
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
		"@bjornpagen/bumbledb-log": "file:../bjornpagen-bumbledb-log-$V.tgz"
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

(cd "$TMP/consumer" && pnpm install --store-dir "$STORE" --prefer-offline --reporter=append-only)

(cd "$TMP/consumer" && node --input-type=module -e "
const eng = await import('@bjornpagen/bumbledb')
if (typeof eng.schema !== 'function' || typeof eng.internalBlake3 !== 'function') {
	throw new Error('packed engine surface incomplete after tarball install')
}
const log = await import('@bjornpagen/bumbledb-log')
if (typeof log.openReplica !== 'function' || typeof log.openWriter !== 'function' || typeof log.storeKey !== 'function') {
	throw new Error('packed bumbledb-log surface incomplete after tarball install')
}
log.storeKey('manifest')
")

echo "packed-import: OK — 5 tarballs packed, engine + bumbledb-log import from an installed consumer at $V"
