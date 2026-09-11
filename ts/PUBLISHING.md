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
| `@bjornpagen/bumbledb-log` | History SDK, schema bindings CLI, and native transitions |

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

## Build and stage one immutable package family

Download these artifacts from the verified CI run, including each accompanying
provenance JSON, into a fresh absolute `BUMBLEDB_NATIVE_DIR`:

- `bumbledb.darwin-arm64.node`
- `bumbledb.linux-arm64.node`
- `bumbledb.linux-x64.node`

The directory must contain `bumbledb.<platform>.node` and
`bumbledb.<platform>.provenance.json` for each platform. Retain the run ID,
commit and digests; never restamp old binaries for a new release.

Set `BUMBLEDB_FAMILY_DIR` to a fresh absolute directory outside the checkout:

```sh
node scripts/build-family.mjs --out "$BUMBLEDB_FAMILY_DIR" --native-artifacts "$BUMBLEDB_NATIVE_DIR" --all-platforms
node scripts/build-family.mjs --check-family "$BUMBLEDB_FAMILY_DIR"
export BUMBLEDB_RELEASE_DIR="$BUMBLEDB_FAMILY_DIR/packages"
```

The build selects one immutable Git tree, including lockfiles, before compiling.
Core, Log, native compilation, installed consumer checks and package staging run
inside its private checkout. Log resolves that attempt's core. The supplied CI
addons replace private build outputs only after their source/platform/binary
provenance is checked. Each attempt has its own native target and TypeScript
outputs; failed or overlapping attempts cannot delete a live consumer's files.

The completed directory contains `source/`, `packages/`, and `family.json`.
It becomes visible only after installed JavaScript/declaration/example checks
and packing succeed. The five tarball hashes are in both `family.json` and
`packages/SHA256SUMS`. Check package contents, exact dependency pins and platform
headers. Matching version strings alone do not prove matching source.

`pnpm --dir ts run build` and `pnpm --dir ts-log run build` also produce a
completed host package family and print its directory; they do not replace
live workspace outputs. `--host-only` packages are for development. Explicitly
install the selected tarballs into consumers; never copy individual SDK outputs
from different attempts.

## Publish, tag, and create the GitHub Release

The owner runs this from the clean, verified checkout with `BUMBLEDB_CI_RUN`
and `BUMBLEDB_FAMILY_DIR` set. When preparing a copyable release command,
supply the absolute checkout path and verified commit, run ID, and directory.
The command publishes native packages first, then core, then log, stopping
on failure. It also pushes the annotated tag and creates the published GitHub
Release; both are required parts of publication.

```sh
(
  set -eu
  : "${BUMBLEDB_CI_RUN:?Set the verified CI run ID}"
  : "${BUMBLEDB_FAMILY_DIR:?Set the verified absolute package family directory}"
  node scripts/build-family.mjs --check-family "$BUMBLEDB_FAMILY_DIR"
  BUMBLEDB_RELEASE_DIR="$BUMBLEDB_FAMILY_DIR/packages"
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
[focused cutover measurements](../docs/perf/imperative-cutover-1.3.1.md).
Real-S3/IAM, Graviton performance, and larger-than-memory workloads have not
been qualified. The TypeScript transition API supports local histories; the
native hosted path uses the existing conditional backend. Schema bindings are generated; all
transformation and backup policy belongs to the application.
The evidence inventory and validator live in `.config/obligation-inventory.json`
and `scripts/release-results.mjs`.
