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

1. Set one version in the root `Cargo.toml` `[workspace.package]`, `ts/package.json` and every
   `ts/npm/*/package.json`.
2. Commit on `main`, tag the commit `v<version>`, and push the tag.

`.github/workflows/release.yml` then runs `ci`, `musl` and the AWS S3 lane on the tagged commit.
After the `release` environment approves, it checks that the tag is on `main` and equals every
manifest version, packs the three CI-built `bdb.<platform>.node` addons with
`node scripts/family.mjs pack`, smoke-tests the host family with `node scripts/family.mjs smoke`,
publishes the platform packages and then the core with npm trusted publishing, and creates the
GitHub Release.

## Build a family locally

```sh
node ts/scripts/build.ts release      # optimized host addon into ts/npm/<platform>/bdb.node, plus dist/
node ts/scripts/build.ts stage out/   # packs the core and every platform package holding bdb.node
```

`node ts/scripts/build.ts dist` compiles `dist/` alone, for addons that were built elsewhere.
