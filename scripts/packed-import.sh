#!/bin/sh
set -eu

# D07/D22/D27 packed-tarball import gate: every published package is
# STAGED and packed for real (immutable staging — the checkout is never
# mutated), installed from its tarball into a bare consumer, typechecked
# as a strict downstream (core-ts, log-ts, native-ledger), and run under
# ManagedRuntime.make(NativeRuntime.layer(...)) — specimens no longer
# self-provide. A second isolated project imports query authoring with
# the native addon unavailable. Rust consumer and Notes specimens/routes
# run in this path.
# Local packing is not PKG-07B.
# Default: require every release platform. --host-only: require this host's
# real binary for per-host CI; never substitute another platform's binary.

unset NODE_OPTIONS NODE_PATH

cd "$(dirname "$0")/.."
ROOT="$PWD"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/packed-import.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

V="$(node -p "require('$ROOT/ts/package.json').version")"
STORE="$(cd "$ROOT/ts-log" && pnpm store path --silent)"
PLATFORMS="darwin-arm64 linux-arm64 linux-x64"
case "${1:-}" in
  --host-only) PLATFORMS="$(node -p '`${process.platform}-${process.arch}`')" ;;
  "") ;;
  *) echo "usage: packed-import.sh [--host-only]" >&2; exit 1 ;;
esac

cp "$ROOT/ts/package.json" "$TMP/core-package.before.json"
cp "$ROOT/ts-log/package.json" "$TMP/log-package.before.json"
node "$ROOT/ts/scripts/stage.ts" --out "$TMP" "$@"
node "$ROOT/ts-log/scripts/stage.ts" --out "$TMP"
cmp -s "$ROOT/ts/package.json" "$TMP/core-package.before.json" || {
  echo "packed-import: FAIL — staging mutated ts/package.json" >&2
  exit 1
}
cmp -s "$ROOT/ts-log/package.json" "$TMP/log-package.before.json" || {
  echo "packed-import: FAIL — staging mutated ts-log/package.json" >&2
  exit 1
}

TARBALLS="bjornpagen-bumbledb-$V.tgz bjornpagen-bumbledb-log-$V.tgz"
for platform in $PLATFORMS; do
  TARBALLS="$TARBALLS bjornpagen-bumbledb-$platform-$V.tgz"
done
for tgz in $TARBALLS; do
  if [ ! -f "$TMP/$tgz" ]; then
    echo "packed-import: FAIL — expected tarball missing: $tgz (platform binaries must be built)" >&2
    exit 1
  fi
  tar -tzf "$TMP/$tgz" | grep -q '^package/package.json$' || {
    echo "packed-import: FAIL — $tgz carries no package.json" >&2
    exit 1
  }
  tar -tzf "$TMP/$tgz" | grep -q '^package/pack-provenance.json$' || {
    echo "packed-import: FAIL — $tgz carries no pack provenance" >&2
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
tar -tzf "$TMP/bjornpagen-bumbledb-log-$V.tgz" | grep -q '^package/dist/bin.js$' || {
  echo "packed-import: FAIL — the ts-log tarball does not carry the bumbledb-log CLI" >&2
  exit 1
}

mkdir "$TMP/consumer"
node "$ROOT/scripts/packed-project.mjs" "$ROOT" "$TMP/consumer"
cat > "$TMP/consumer/pnpm-workspace.yaml" <<YAML
packages:
  - "."
overrides:
YAML
for platform in $PLATFORMS; do
  printf '  "@bjornpagen/bumbledb-%s": "file:../bjornpagen-bumbledb-%s-%s.tgz"\n' "$platform" "$platform" "$V" >> "$TMP/consumer/pnpm-workspace.yaml"
done

cp "$ROOT/scripts/packed-consumer.ts" "$TMP/consumer/packed-consumer.ts"
cp "$ROOT/ts-log/test/transition.test.ts" "$TMP/consumer/transition.test.ts"
mkdir -p "$TMP/consumer/fixtures"
cp -R "$ROOT/ts-log/test/fixtures/transition" "$TMP/consumer/fixtures/transition"
mkdir -p "$TMP/consumer/core-ts" "$TMP/consumer/log-ts" "$TMP/consumer/native-ledger"
cp "$ROOT/examples/consumers/core-ts/consumer.ts" "$TMP/consumer/core-ts/consumer.ts"
cp "$ROOT/examples/consumers/log-ts/consumer.ts" "$TMP/consumer/log-ts/consumer.ts"
cp "$ROOT/examples/consumers/native-ledger/consumer.ts" "$TMP/consumer/native-ledger/consumer.ts"
(cd "$TMP/consumer" && pnpm install --ignore-scripts --store-dir "$STORE" --prefer-offline --reporter=append-only)

# No skipLibCheck, workspace path aliases, custom conditions, or repo compiler.
# Invoke the already installed compiler: pnpm exec can initiate an unrelated
# second installation, losing --ignore-scripts in a new package-manager version.
(cd "$TMP/consumer" && ./node_modules/.bin/tsc --strict --exactOptionalPropertyTypes --target es2024 \
  --module nodenext --types node --allowImportingTsExtensions \
  --declaration --emitDeclarationOnly --outDir declarations \
  packed-consumer.ts transition.test.ts core-ts/consumer.ts log-ts/consumer.ts native-ledger/consumer.ts)
if grep -REq '(node_modules|\.pnpm|import\("/|from "/|"(file|link):)' "$TMP/consumer/declarations"; then
  echo "packed-import: FAIL — consumer declarations leaked a private installation path" >&2
  exit 1
fi
# D22: programs that no longer self-provide run under ManagedRuntime.
(cd "$TMP/consumer" && node packed-consumer.ts && node --test transition.test.ts)

# D27: second isolated project — no platform overrides; optional native off.
mkdir "$TMP/pure"
node "$ROOT/scripts/packed-project.mjs" "$ROOT" "$TMP/pure" --pure
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
# TypeScript 7's executable is itself an optional native dependency. Use
# the isolated consumer's compiler; the pure project still has no addon.
(cd "$TMP/pure" && ../consumer/node_modules/.bin/tsc --strict --exactOptionalPropertyTypes --target es2024 \
  --module nodenext --types node --allowImportingTsExtensions \
  --declaration --emitDeclarationOnly --outDir declarations \
  packed-pure-authoring.ts)
(cd "$TMP/pure" && node packed-pure-authoring.ts)

# Rust consumer lives in this packed-import path (owned collection and paging).
cargo run --manifest-path "$ROOT/examples/consumers/rust/Cargo.toml"

# Run the actual Notes app against the same packed dependencies in isolation.
# Check the entire app and build the server.
# Keep the initial install's store: /tmp may resolve a different Linux mount.
mkdir -p "$TMP/consumer/examples"
git ls-files --cached --others --exclude-standard -z examples/notes examples/consumers | tar --null -T - -cf - | (cd "$TMP/consumer" && tar -xf -)
node -e '
  const fs = require("node:fs");
  const root = process.argv[1];
  const settings = fs.readFileSync(`${root}/examples/notes/pnpm-workspace.yaml`, "utf8")
    .replaceAll("patches/", "examples/notes/patches/");
  fs.appendFileSync(`${root}/pnpm-workspace.yaml`, `\n${settings}`);
' "$TMP/consumer"
NOTES_DEPS="$(node -e '
  const p = require(process.argv[1]);
  const dependencies = { ...p.dependencies, ...p.devDependencies };
  console.log(Object.entries(dependencies)
    .filter(([name]) => !name.startsWith("@bjornpagen/") && name !== "effect")
    .map(([name, version]) => `${name}@${version}`).join(" "));
' "$ROOT/examples/notes/package.json")"
(cd "$TMP/consumer" && pnpm add --ignore-scripts --store-dir "$STORE" $NOTES_DEPS)
(cd "$TMP/consumer/examples/notes" && \
  ../../node_modules/.bin/tsc --noEmit && \
  ../../node_modules/.bin/bumbledb-log snapshot --schema src/db/schema.ts --export App --out schema.json && \
  node --conditions react-server --test test/specimens.test.ts test/routes.test.ts && \
  NEXT_TELEMETRY_DISABLED=1 ../../node_modules/.bin/next build --webpack)

echo "packed-import: OK — platforms: $PLATFORMS; ManagedRuntime consumer; owned collection and one-shot paging; D27 addon-unavailable authoring; Rust + Notes typecheck/build/routes at $V (not PKG-07B)"
