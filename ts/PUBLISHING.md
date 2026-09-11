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

Every release requires all three: publication of the five packages, an
annotated Git tag `v<version>` pushed to GitHub at the verified source commit,
and a published GitHub Release for that tag with release notes, all five
tarballs, and `SHA256SUMS` attached. None substitutes for another. Registry
publication remains an explicit owner action; the complete owner command must
include tagging and GitHub Release creation, not just package publication.

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
   Rust and TypeScript tests, independent oracles, and installed-consumer checks.
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

For 1.3.0, use `/tmp/bumbledb-release-1.3.0` as the fresh staging directory.
From the repository root, after the required jobs and artifact checks pass:

```sh
BUMBLEDB_RELEASE_DIR=/tmp/bumbledb-release-1.3.0
node ts/scripts/stage.ts --out "$BUMBLEDB_RELEASE_DIR"
node ts-log/scripts/stage.ts --out "$BUMBLEDB_RELEASE_DIR"
(cd "$BUMBLEDB_RELEASE_DIR" && shasum -a 256 ./*.tgz > SHA256SUMS)
```

The complete owner command checks the candidate's CI and staged checksums,
publishes the five packages in dependency order, pushes an annotated tag at
that exact commit, and creates the public GitHub Release with those bytes.
Set `BUMBLEDB_CI_RUN` to the candidate's successful CI run ID first. Run from
its clean checkout; the release directory must contain exactly the five
reviewed 1.3.0 tarballs and their checksum file. For a future version, update
both the version and the staging directory. Supply the owner an absolute
checkout path and the verified commit/run IDs when preparing a release command.

```sh
(set -eu; BUMBLEDB_VERSION=1.3.0; BUMBLEDB_RELEASE_DIR=/tmp/bumbledb-release-1.3.0; BUMBLEDB_REVISION=$(git rev-parse HEAD); node scripts/release-ready.mjs "$BUMBLEDB_CI_RUN"; (cd "$BUMBLEDB_RELEASE_DIR" && shasum -a 256 -c SHA256SUMS); (cd ts && pnpm whoami && for p in bumbledb-darwin-arm64 bumbledb-linux-arm64 bumbledb-linux-x64 bumbledb bumbledb-log; do pnpm publish "$BUMBLEDB_RELEASE_DIR/bjornpagen-$p-$BUMBLEDB_VERSION.tgz" --access public --no-git-checks || exit; done); git tag -a "v$BUMBLEDB_VERSION" "$BUMBLEDB_REVISION" -m "BumbleDB $BUMBLEDB_VERSION"; git push origin "refs/tags/v$BUMBLEDB_VERSION"; gh release create "v$BUMBLEDB_VERSION" "$BUMBLEDB_RELEASE_DIR"/*.tgz "$BUMBLEDB_RELEASE_DIR/SHA256SUMS" --verify-tag --title "BumbleDB $BUMBLEDB_VERSION" --notes-file "docs/release-$BUMBLEDB_VERSION.md")
```

Authentication or OTP may be required. If `pnpm whoami` fails, authenticate
with `pnpm --dir ts login` and rerun the command. A successful identity check
alone does not establish permission to publish every package.

If publication stops partway through, inspect each package with
`pnpm --dir ts view PACKAGE@1.3.0 dist --json`, compare the registry bytes,
and resume only the missing packages in order. Never republish or replace
an existing version. `--no-git-checks` permits publishing staged tarballs;
the preceding release check still requires a clean, verified candidate.
If tagging or GitHub Release creation failed after publication, verify the
remote tag resolves to the candidate, then resume that remaining step. Never
move a release tag or replace a released artifact to recover a partial run.

After publication, verify the tag points to the candidate and the Release is
published (not a draft), with the expected title, notes, and six assets. Download
the registry artifacts and compare their hashes with the staged files. Test a
clean installed consumer on each supported platform. Only this establishes
registry distribution; local tarball tests cannot substitute for it.

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
benchmark provenance. The [structural query measurements](../docs/perf/structural-algebra-20260911.md)
record their own pre-release source; neither report is a full 1.3.0 benchmark.
Benchmark completion is separate from the required build/test CI jobs.
