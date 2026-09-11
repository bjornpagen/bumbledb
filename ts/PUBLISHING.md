# Publishing BumbleDB

Every release publishes five version-matched packages, an annotated Git tag
`v<version>` at the verified commit, and a GitHub Release with release notes,
the five tarballs, and `SHA256SUMS`.

| Package | Contents |
|---|---|
| `@bjornpagen/bumbledb-darwin-arm64` | Apple Silicon native addon |
| `@bjornpagen/bumbledb-linux-arm64` | Linux ARM64 native addon |
| `@bjornpagen/bumbledb-linux-x64` | Linux x64 native addon |
| `@bjornpagen/bumbledb` | Core Effect SDK, declarations, source types, and guides |
| `@bjornpagen/bumbledb-log` | History SDK, schema/migration subpaths, and migration CLI |

Use **pnpm only** for JavaScript installation, builds, packing, registry
inspection, and publication. Run package commands in `ts/` or with
`pnpm --dir ts` to select its pinned package manager. The SDKs require Node 24+
and Effect `4.0.0-rc.112`. Linux addons target Amazon Linux 2023 / glibc 2.34.
Rust crates are distributed from Git with `publish = false`.

## Prepare the release commit

The root `[workspace.package] version` owns the version. Align the manifests
in `scripts/version-roster.txt`, example pins, lockfiles, and the log's exact
core peer. Staging injects exact native-package dependencies into the core
tarball. Never reuse a published version or mix native and SDK versions.

Write `docs/release-<version>.md`, run `scripts/battery.sh`, commit, and push
to `main`. Wait for these jobs in that commit's `ci` push run to finish
successfully:

- `check / darwin`
- `check / linux-arm64`
- `check / linux-x64`

Each includes builds, the complete platform correctness battery, and installed
consumer checks. Missing, running, skipped, cancelled, or failed required jobs
block publication. **Static Linux ARM64 and Miri are supplemental:** their
status does not block release; the overall workflow need not be green.

Set `BUMBLEDB_CI_RUN` to that run ID. From the clean release checkout, enforce
the required jobs and matching commit with:

```sh
node scripts/release-ready.mjs "$BUMBLEDB_CI_RUN"
```

Recheck after any new commit. Benchmark reports keep their measured source
revision; benchmark completion is separate from the build/test gate.

## Build and stage

Build both SDKs with `pnpm --dir ts run build` and
`pnpm --dir ts-log run build`. Then download these artifacts from the verified
CI run, including each accompanying provenance JSON:

- `bumbledb.darwin-arm64.node`
- `bumbledb.linux-arm64.node`
- `bumbledb.linux-x64.node`

For each platform, install the binary as `ts/npm/<platform>/bumbledb.node`
and the provenance as `ts/npm/<platform>/.native-provenance.json`.
Do this **after** SDK builds, which create a local addon. Use the matching CI
binaries for final staging; retain their run ID, commit, and digests. Staging
checks source, specification, platform, and binary hashes. Do not restamp old
binaries for a new commit.

Run `scripts/packed-import.sh` with all three artifacts installed. Set
`BUMBLEDB_RELEASE_DIR` to a fresh absolute directory outside the checkout,
then assemble the release:

```sh
(
  set -eu
  : "${BUMBLEDB_RELEASE_DIR:?Set an absolute staging directory}"
  node ts/scripts/stage.ts --out "$BUMBLEDB_RELEASE_DIR"
  node ts-log/scripts/stage.ts --out "$BUMBLEDB_RELEASE_DIR"
  (cd "$BUMBLEDB_RELEASE_DIR" && shasum -a 256 ./*.tgz > SHA256SUMS)
)
```

Inspect all five tarballs, versions, dependency pins, platform headers, and
provenance. Staging copies built outputs into isolated package trees and uses
`pnpm pack`; it does not change source manifests. `--host-only` and
`--skip-binary` are development options, not full-release assembly.

## Publish, tag, and create the GitHub Release

The owner runs this from the clean, verified checkout with `BUMBLEDB_CI_RUN`
and `BUMBLEDB_RELEASE_DIR` set. When preparing a copyable release command,
supply the absolute checkout path and verified commit, run ID, and directory.
The command publishes native packages first, then core, then log, stopping
on failure. It also pushes the annotated tag and creates the published GitHub
Release; both are required parts of publication.

```sh
(
  set -eu
  : "${BUMBLEDB_CI_RUN:?Set the verified CI run ID}"
  : "${BUMBLEDB_RELEASE_DIR:?Set the verified absolute staging directory}"
  BUMBLEDB_VERSION=$(node -p "require('./ts/package.json').version")
  BUMBLEDB_REVISION=$(git rev-parse HEAD)
  node scripts/release-ready.mjs "$BUMBLEDB_CI_RUN"
  test -f "docs/release-$BUMBLEDB_VERSION.md"
  set -- "$BUMBLEDB_RELEASE_DIR"/*.tgz
  test "$#" -eq 5
  (cd "$BUMBLEDB_RELEASE_DIR" && shasum -a 256 -c SHA256SUMS)
  (
    cd ts
    pnpm whoami
    for p in bumbledb-darwin-arm64 bumbledb-linux-arm64 bumbledb-linux-x64 bumbledb bumbledb-log; do
      pnpm publish "$BUMBLEDB_RELEASE_DIR/bjornpagen-$p-$BUMBLEDB_VERSION.tgz" --access public --no-git-checks || exit
    done
  )
  git tag -a "v$BUMBLEDB_VERSION" "$BUMBLEDB_REVISION" -m "BumbleDB $BUMBLEDB_VERSION"
  git push origin "refs/tags/v$BUMBLEDB_VERSION"
  gh release create "v$BUMBLEDB_VERSION" "$BUMBLEDB_RELEASE_DIR"/*.tgz "$BUMBLEDB_RELEASE_DIR/SHA256SUMS" --verify-tag --title "BumbleDB $BUMBLEDB_VERSION" --notes-file "docs/release-$BUMBLEDB_VERSION.md"
)
```

If authentication fails, use `pnpm --dir ts login`; publication may also
require an OTP. If publication stops partway, inspect each package with
`pnpm --dir ts view PACKAGE@VERSION dist --json`, compare registry artifacts
with the staged bytes, and publish only the missing packages in order. Resume
tagging or Release creation separately if those steps failed. Never move a
release tag or replace a published artifact to recover a partial run.

Verify the remote tag resolves to the release commit and the GitHub Release
is published with the notes and six assets. Compare registry downloads with
the staged hashes and test clean installed consumers on each supported
platform. Local tarball checks do not establish registry distribution.

Release notes should report support boundaries and actual evidence from the
[benchmark reports](../docs/perf/results.md) and
[structural measurements](../docs/perf/structural-algebra-20260911.md).
Real-S3/IAM, Graviton performance, and larger-than-memory workloads have not
been qualified. Generated hosted migration orchestration is unsupported.
The evidence inventory and validator live in `.config/obligation-inventory.json`
and `scripts/release-results.mjs`.
