# Publishing bumbledb

A release publishes four version-matched packages and a GitHub Release holding their tarballs and
`SHA256SUMS`:

| Package | Contents |
|---|---|
| `@bjornpagen/bumbledb` | The SDK (`dist/`, `src/`, `COOKBOOK.md`) and the `bumbledb` CLI |
| `@bjornpagen/bumbledb-darwin-arm64` | `bdb.node` for Apple Silicon |
| `@bjornpagen/bumbledb-linux-arm64` | `bdb.node` for Linux arm64 (amazonlinux:2023, glibc 2.34) |
| `@bjornpagen/bumbledb-linux-x64` | `bdb.node` for Linux x64 (amazonlinux:2023, glibc 2.34) |

The core package pins each platform package as an exact optional dependency. Rust crates are not
published.

## Cut a release

Releases are published by hand; CI never publishes. Every push to `main` builds the three addons
with fat LTO in CI and keeps them as the `bdb.<platform>.node` artifacts of that `ci` run.

1. Set one version in the root `Cargo.toml` `[workspace.package]`, `ts/package.json` and every
   `ts/npm/*/package.json`. Commit and push to `main`, and wait for its `ci` run to pass.
2. From a clean checkout of that commit, with Node 26, pnpm, `gh` and an npm login that can
   publish the four packages:

```sh
version=$(node -p 'require("./ts/package.json").version')
run=$(gh run list --workflow ci --branch main --commit "$(git rev-parse HEAD)" --status success \
  --json databaseId --jq '.[0].databaseId')
gh run download "$run" --pattern 'bdb.*.node' --dir artifacts
mkdir natives && cp artifacts/*/bdb.*.node natives/
pnpm --dir ts install --frozen-lockfile
node scripts/family.mjs pack natives family
node scripts/family.mjs smoke family
for p in darwin-arm64 linux-arm64 linux-x64; do npm publish "family/bjornpagen-bumbledb-$p-$version.tgz"; done
npm publish "family/bjornpagen-bumbledb-$version.tgz"
git tag "v$version" && git push origin "v$version"
gh release create "v$version" family/*.tgz family/SHA256SUMS --verify-tag --title "v$version" --generate-notes
```

`pack` refuses an addon for an unknown platform; check that `natives/` holds all three, because the
core pins every platform package at its own version. Publish the platform packages before the core.

## Build a family locally

```sh
node ts/scripts/build.ts release      # optimized host addon into ts/npm/<platform>/bdb.node, plus dist/
node ts/scripts/build.ts stage out/   # packs the core and every platform package holding bdb.node
```

`node ts/scripts/build.ts dist` compiles `dist/` alone, for addons that were built elsewhere.
