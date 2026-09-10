# Release and package publishing

The release family has five npm packages. All use the workspace version;
Effect is pinned to `4.0.0-rc.112`, and the SDKs require Node 24 or newer.

| Package | Contents |
|---|---|
| `@bjornpagen/bumbledb-darwin-arm64` | Apple Silicon native addon. |
| `@bjornpagen/bumbledb-linux-arm64` | Linux ARM64 addon, built in Amazon Linux 2023. |
| `@bjornpagen/bumbledb-linux-x64` | Linux x64 addon, built in Amazon Linux 2023. |
| `@bjornpagen/bumbledb` | Core Effect SDK, declarations, source types, and guides. |
| `@bjornpagen/bumbledb-log` | History SDK, schema/migration subpaths, and migration CLI. |

The Linux build targets glibc 2.34; it is not a musl build. Rust workspace
crates have `publish = false` and are consumed from source/Git. There is no
C package or public Rust log SDK to publish.

A Git tag, a GitHub Release, and npm publication are separate actions.
Creating a GitHub Release does not put packages in the registry. Registry
publication remains an explicit owner action.

## Version and source

The root `[workspace.package] version` owns the version. Keep every manifest
in `scripts/version-roster.txt`, the log's exact core peer, example dependency
pins, and lockfiles aligned. Never reuse an npm version that already exists.

`ts/scripts/build.ts` checks the version roster and Effect pin, rebuilds the
native addon, checks its embedded engine version, emits isolated declarations,
and verifies the staged package shape.

The released core manifest gets exact-version optional dependencies on all
three native packages. These pins are injected into staging, not the checkout.
The log gets an exact-version core peer. A consumer must never load a native
addon from a different package version.

## Verification and native artifacts

1. Finish the source and documentation, then run the repository correctness
   battery. Preserve benchmark source provenance separately from the final
   release revision; a documentation/version-only change does not turn an
   older report into a measurement of a different binary.
2. Push the final candidate and require its exact-commit CI checks to pass.
   The `bumbledb-log` workflow builds the Linux artifacts in Amazon Linux
   2023 on their respective architectures.
3. Download `bumbledb.darwin-arm64.node`, `bumbledb.linux-arm64.node` and
   `bumbledb.linux-x64.node`, with their accompanying provenance JSON, from
   that run. Place each binary as `ts/npm/<platform>/bumbledb.node` and its
   provenance as `ts/npm/<platform>/.native-provenance.json`; retain the
   run ID, source revision, and artifact digests. Do not substitute a macOS
   binary or an earlier candidate's artifact.
   Staging checks the recorded candidate, specification, platform, and binary
   hash before assigning the current release version to a native tarball.
4. Build both SDKs and verify the Apple Silicon addon against the release
   version. Stage all platforms and run the installed-consumer check.
   Per-host CI uses `--host-only`;
   release assembly requires all three real platform binaries.

Useful checks from the repository root:

```sh
scripts/battery.sh
scripts/packed-import.sh
```

A passing local battery does not prove real-S3 behavior, Graviton performance,
a larger-than-memory workload, or public-registry installation. Required
evidence and its validator remain in `.config/obligation-inventory.json`
and `scripts/release-results.mjs`. Do not relabel missing evidence as passed.

## Immutable staging

After building and checking the candidate, stage into a fresh output directory:

```sh
(cd ts && node scripts/stage.ts --out /absolute/path/to/release)
(cd ts-log && node scripts/stage.ts --out /absolute/path/to/release)
```

The first command requires all three native binaries by default. The
`--host-only` and `--skip-binary` options are development conveniences, not
full-release assembly. Staging copies built outputs into temporary package
trees and records package provenance; it does not rewrite source manifests.

Inspect the five tarballs, their versions, dependency pins, native platform
headers, and provenance. Record SHA-256 digests of the exact files. Attach
those tarballs and checksums to the GitHub Release so the owner can publish
the verified bytes without rebuilding.

## Registry order and confirmation

Publish the three native platform tarballs first, then core, then log. Use
the staged tarball paths, not the source package directories. Every command
must stop on failure; never proceed with a partially published dependency
family as though it were complete. Authentication or OTP may be required.

After publication, download the registry artifacts and compare their hashes
with the staged files. Test a clean installed consumer on each supported
platform. Only this establishes registry distribution; local tarball tests
cannot substitute for it.

pnpm's minimum-release-age policy can delay fresh packages. Consumers should
make any scoped exception deliberately; do not turn off registry safeguards
globally as part of a database install.

## Release scope

The release covers the embedded core and local log. Hosted generated
migration orchestration remains unsupported through the TypeScript/native
bridge. Real-S3/IAM and Graviton qualification remain explicitly deferred by
the owner, not passed. The September 8, 2026 full local benchmark measured
1.1.0 source `548193d4`, using one timed lane at a time and macOS
QoS steering rather than hard affinity. The evidence retains that revision when
later documentation commits produce the final release revision. See the
[full benchmark report](../docs/perf/results.md) for coverage and regressions.
Release notes must state these limitations. A published release is not a
claim of universal production readiness or complete external qualification.
