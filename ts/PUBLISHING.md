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

Use **pnpm only** for JavaScript dependency installation, builds, packing,
registry inspection, and publication. The `packageManager` field in `ts/`
selects the pnpm version. Run package commands there (or use `pnpm --dir ts`).

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
2. Push the final candidate. In its `ci` push run on `main`, wait for **all
   three supported-platform build and test jobs** to finish with `success`:
   `check / darwin`, `check / linux-arm64`, and `check / linux-x64`.
   These jobs include the native/SDK builds, complete correctness battery,
   Rust and TypeScript tests, Lean checks, and installed-consumer checks.
   Missing, running, skipped, cancelled, or failed required jobs block release.
   **Do not wait for the entire workflow to turn green:** static Linux ARM64
   (`static-linux-arm64 / musl`) and Miri are supplemental and do not block
   publication, even while running or if unsuccessful. Keep their actual
   results visible; this does not mark them passed or disable them.
   Run `node scripts/release-ready.mjs RUN_ID` from the clean candidate
   checkout to enforce the commit and required-job checks. Recheck after
   every new commit. The `bumbledb-log` reusable workflow builds the Linux
   artifacts in Amazon Linux 2023 on their respective architectures.
3. Download `bumbledb.darwin-arm64.node`, `bumbledb.linux-arm64.node` and
   `bumbledb.linux-x64.node`, with their accompanying provenance JSON, from
   that run. Place each binary as `ts/npm/<platform>/bumbledb.node` and its
   provenance as `ts/npm/<platform>/.native-provenance.json`; retain the
   run ID, source revision, and artifact digests. Do not substitute a macOS
   binary or an earlier candidate's artifact.
   Staging checks the recorded candidate, specification, platform, and binary
   hash before assigning the current release version to a native tarball.
4. Build both SDKs with `pnpm --dir ts run build` and
   `pnpm --dir ts-log run build`, then install the three downloaded artifacts
   from step 3 (the core build creates a local addon, which must be replaced
   by the CI artifact before final staging). Verify the Apple Silicon addon's
   embedded engine version. Stage all platforms and run the installed-consumer check.
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

For 1.2.1, use `/tmp/bumbledb-release-1.2.1` as the fresh staging directory.
From the repository root, after the required jobs and artifact checks pass:

```sh
RELEASE_DIR=/tmp/bumbledb-release-1.2.1
node ts/scripts/stage.ts --out "$RELEASE_DIR"
node ts-log/scripts/stage.ts --out "$RELEASE_DIR"
(cd "$RELEASE_DIR" && shasum -a 256 ./*.tgz > SHA256SUMS)
```

The owner publish one-liner below checks the same CI run and the staged
checksums, publishes the five packages in dependency order with pnpm, and
creates the matching tag/GitHub Release with those exact tarballs. Set
`RUN_ID` to the successful candidate's CI run ID first. The release directory
must contain exactly the five reviewed 1.2.1 tarballs and their checksum file.

```sh
node scripts/release-ready.mjs "$RUN_ID" && (cd /tmp/bumbledb-release-1.2.1 && shasum -a 256 -c SHA256SUMS) && (cd ts && for p in bumbledb-darwin-arm64 bumbledb-linux-arm64 bumbledb-linux-x64 bumbledb bumbledb-log; do pnpm publish "/tmp/bumbledb-release-1.2.1/bjornpagen-$p-1.2.1.tgz" --access public --no-git-checks || exit; done) && gh release create v1.2.1 /tmp/bumbledb-release-1.2.1/*.tgz /tmp/bumbledb-release-1.2.1/SHA256SUMS --target "$(git rev-parse HEAD)" --title 'BumbleDB 1.2.1' --notes-file docs/release-1.2.1.md
```

If publication stops partway through, inspect each package with
`pnpm --dir ts view PACKAGE@1.2.1 dist --json`, compare the registry bytes,
and resume only the missing packages in order. Never republish or replace
an existing version. `--no-git-checks` permits publishing staged tarballs;
the preceding release check still requires a clean, verified candidate.

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

The [1.2.1 benchmark plan](../docs/perf/runs/1.2.1/README.md) prepares a new
full local run. Until it is executed and reviewed, retain the existing
benchmark provenance and make no 1.2.1 performance claim. Benchmark completion
is separate from the required build/test CI jobs.
