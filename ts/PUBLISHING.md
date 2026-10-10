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

The version in `ts/package.json` is the only version; packing stamps it into every platform
package. To release, change it and push to `main`.

The `release` job in `.github/workflows/ci.yml` runs after `lint`, `test` and every `addon` build
pass on that push (`scripts/ci.sh release`). When npm lacks `@bjornpagen/bumbledb@<version>`, it packs
the three fat-LTO addons that run built, smoke-tests the packed family, publishes the platform
packages and then the core, and creates the `v<version>` GitHub Release with the tarballs and
`SHA256SUMS`. A version with a prerelease suffix (`2.1.0-rc.1`) goes to the `next` dist-tag and a
GitHub prerelease. Every step skips what already exists, so re-running a failed job finishes the
release. While `ts/package.json` says `"private": true`, nothing is released.

npm trusted publishing authorizes the job; no token is stored. Each of the four packages names
`bjornpagen/bumbledb` and the workflow `ci.yml` as its trusted publisher on npmjs.com (Settings,
Trusted Publisher, GitHub Actions, no environment).

## Build a family locally

```sh
node ts/scripts/build.ts release      # optimized host addon into ts/npm/<platform>/bdb.node, plus dist/
node ts/scripts/build.ts stage out/   # packs the core and every platform package holding bdb.node
```

`node ts/scripts/build.ts dist` compiles `dist/` alone, for addons that were built elsewhere.
