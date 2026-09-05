# Publishing @bjornpagen/bumbledb

The owner-run release runbook for the successor packaging design
(immutable staging; see `docs/reference/packaging.md`). Publication is
OWNER CEREMONY under a separate authorization — no agent, CI job or
implementation campaign publishes or tags. Historical 0.x release notes
live in git history of this file; none of their compatibility claims
carry forward across the 1.0 cutover.

## The five packages

| Package | Contents | `os`/`cpu` |
| --- | --- | --- |
| `@bjornpagen/bumbledb` | dist JS + `.d.ts` + `src/` types isolation (no binary) | none |
| `@bjornpagen/bumbledb-log` | dist JS + `.d.ts`, `/schema` + `/migrations` subpaths, the `bumbledb-log` CLI | none |
| `@bjornpagen/bumbledb-darwin-arm64` | only `bumbledb.node` | `darwin` / `arm64` |
| `@bjornpagen/bumbledb-linux-arm64` | only `bumbledb.node` (amazonlinux:2023, glibc 2.34) | `linux` / `arm64` |
| `@bjornpagen/bumbledb-linux-x64` | only `bumbledb.node` (amazonlinux:2023, glibc 2.34) | `linux` / `x64` |

The PUBLISHED main manifest pins every platform package
`optionalDependencies`-EXACT to its own version; the COMMITTED manifest
carries no pin and no pack lifecycle hook. `ts/scripts/stage.ts` derives
the pinned manifest inside an isolated staging tree and packs THERE —
the checkout is never rewritten, an interrupted stage changes nothing,
and a lockfile can never demand the current unpublished version (the old
prepack/postpack injection and its interrupted-restore window are
deleted). `ts-log`'s staged manifest is derived the same way by
`ts-log/scripts/stage.ts` (exact same-version core peer, exact
`effect@4.0.0-rc.112`, the workspace `link:` twin stripped with
devDependencies).

## Version lockstep

One writer: the root `[workspace.package] version`. Every versioned
manifest is a line on `scripts/version-roster.txt`;
`assertVersionLockstep` (`ts/scripts/build.ts`) fails the build unless
every roster entry matches, the roster is sweep-complete, ts-log's core
peer is exact, and the Effect pin is exact. `engineVersion()` bakes the
version into the shipped binary; the loader only ever resolves its
own-version artifact (the FFI ABI is not semver-stable).

## Runbook (darwin-arm64 host, owner)

```sh
cd ts

# 1. Bump the workspace version + roster manifests; the build asserts it.

# 2. Place the linux artifacts from a green CI run of THIS commit:
#    gh run download <run-id> --name bumbledb.linux-arm64.node --dir /tmp/artifacts
#    gh run download <run-id> --name bumbledb.linux-x64.node   --dir /tmp/artifacts
#    cp /tmp/artifacts/bumbledb.linux-arm64.node npm/linux-arm64/bumbledb.node
#    cp /tmp/artifacts/bumbledb.linux-x64.node   npm/linux-x64/bumbledb.node
#    Never rebuild a linux binary on this host; never copy a darwin
#    binary into a linux package.

# 3. Build + verify (lockstep, cargo release build, smoke-load through
#    the by-name loader, STAGED tarball proof: pins exact, no binary in
#    main, platform allowlist exact, checkout untouched).
pnpm install
pnpm test
pnpm exec tsc --noEmit
pnpm exec biome check .

# 4. Full repo gates + the packed-import gate (stages all five tarballs
#    into an isolated consumer, typechecks the chapter 34 fixtures,
#    runs the runtime smoke):
(cd .. && scripts/battery.sh)

# 5. Stage the release tarballs — these EXACT files are what publishes;
#    nothing is rebuilt at publish time.
node scripts/stage.ts --out /tmp/release
(cd ../ts-log && node scripts/stage.ts --out /tmp/release)

# 6. Publish platform packages FIRST (the main's exact pins must resolve),
#    then the main, then the log (its exact core peer must resolve).
#    Interactive OTP each time; access is public via publishConfig.
V=<version>
pnpm publish --no-git-checks /tmp/release/bjornpagen-bumbledb-darwin-arm64-$V.tgz
pnpm publish --no-git-checks /tmp/release/bjornpagen-bumbledb-linux-arm64-$V.tgz
pnpm publish --no-git-checks /tmp/release/bjornpagen-bumbledb-linux-x64-$V.tgz
pnpm publish --no-git-checks /tmp/release/bjornpagen-bumbledb-$V.tgz
pnpm publish --no-git-checks /tmp/release/bjornpagen-bumbledb-log-$V.tgz

# 7. Distribution proof (PKG-07B): download the actual registry
#    artifacts, verify digests match the staged files, clean-install in
#    an empty project, and only then declare the release complete. A
#    mismatch is a release incident, never retroactive qualification.
pnpm view @bjornpagen/bumbledb@$V dist.shasum
shasum /tmp/release/bjornpagen-bumbledb-$V.tgz

# 8. Tag (owner ceremony): git tag -a v$V <commit> && git push origin v$V
```

Notes:

- pnpm 11's default `minimumReleaseAge` (1440 min) delays consumers that
  do not exclude `@bjornpagen/*`; this repo's own workspaces do.
- `npm publish --provenance` needs a CI runner; if adopted, order stays
  platform-first.
- Verifying a published install on a clean host: `npm install
  @bjornpagen/bumbledb` in an empty project, then import it — the
  platform dep resolves by host; outside the shipped set the install
  succeeds and the first load throws the typed unsupported-platform
  error naming the shipped roster.
