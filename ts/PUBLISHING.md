# Publishing @bjornpagen/bumbledb

## The 0.4.0 one-liner

Owner-run, from the release worktree, on a darwin-arm64 host, logged in to npm
(`pnpm whoami` answers). Platform first, then main — one `&&` chain; EACH
publish stops and prompts interactively for the npm OTP (2FA), so run it in a
real terminal and have the authenticator open:

```sh
cd /Users/bjorn/Documents/bumbledb/.claude/worktrees/host-idiom-040/ts && pnpm publish --access public --no-git-checks ./npm/darwin-arm64 && pnpm publish --access public --no-git-checks
```

`--no-git-checks` is REQUIRED here: pnpm refuses to publish from a non-main
branch, and this worktree sits on `worktree-host-idiom-040`. The main publish
runs `prepublishOnly` → the full build (lockstep assertion, cargo release
build, smoke-load through the by-name loader path, tarball-manifest
verification) before anything uploads.

The owner-run release runbook (host-idiom-0.4.0 PRD-V1). This repo builds and
verifies both packages; the agent side does NOT publish. `npm publish` /
`pnpm publish` and the git tag are owner ceremony.

`0.4.0` is a deliberate backwards-incompatible hard break over `0.3.0` (the
drizzle law: database idioms arrive as modern TypeScript idioms) — closed
handles are string-literal unions on every surface, `Kind.match`/`fromId`/the
handle constants/`oneOf()` are gone, dispatch is native `switch` narrowing,
set membership is a plain array, and closed fields left the
orderable/foldable set. The wire, manifest, and fingerprint are UNTOUCHED:
zero fingerprint pins moved (the cross-host lock and the T5 cookbook goldens
are byte-identical to the 0.3.0 tree).

## The two packages

| Package | Contents | `os`/`cpu` |
| --- | --- | --- |
| `@bjornpagen/bumbledb` | pure JS + `.d.ts` (no binary) | none (installs everywhere) |
| `@bjornpagen/bumbledb-darwin-arm64` | only `bumbledb.node` | `darwin` / `arm64` |

The main package declares the platform package as an `optionalDependency`
pinned EXACT to its own version. npm/pnpm install the platform package only on
a matching host; the main package's loader (`src/native.ts`) resolves it by
name at runtime and throws a typed unsupported-platform error everywhere else.

## Version lockstep

The version lives in ONE place: `ts/package.json` `version`. Three values must
match exactly, and the build (`assertVersionLockstep` in `scripts/build.ts`)
fails if they diverge:

1. `ts/package.json` `version`
2. `ts/package.json` `optionalDependencies["@bjornpagen/bumbledb-darwin-arm64"]`
3. `ts/npm/darwin-arm64/package.json` `version`

A release bump edits all three, then the build enforces the match. All three
are set to `0.4.0` in this tree, and `pnpm run build` has confirmed the
lockstep (`bumbledb build: version 0.4.0 (main == platform ==
optionalDependencies pin)`).

## Runbook (0.4.0, darwin-arm64 host, owner)

```sh
# 0. From the ts/ package root, on a macOS Apple Silicon machine.
cd ts

# 1. The lockstep is already set to 0.4.0 in all THREE places (done in this
#    tree; the build asserts it):
#    - ts/package.json                    "version": "0.4.0"
#    - ts/package.json                    optionalDependencies pin -> "0.4.0"
#    - ts/npm/darwin-arm64/package.json   "version": "0.4.0"

# 2. Build + verify both trees (fails on version drift, unloadable artifact,
#    or a mispacked tarball). Produces dist/ and npm/darwin-arm64/bumbledb.node.
pnpm install
pnpm run build
node --test $(find test -name '*.test.ts')
pnpm exec tsc --noEmit
pnpm exec biome check .

# 3. Publish the PLATFORM package FIRST — the main's exact-pinned optional dep
#    must already exist in the registry when the main resolves.
#    (Interactive: npm prompts for the 2FA one-time password.)
pnpm publish --access public --no-git-checks ./npm/darwin-arm64

# 4. Publish the MAIN package SECOND. (`prepublishOnly` reruns the build;
#    another OTP prompt.) `ts/package.json` already carries "private": false —
#    there is no toggle to flip since 0.1.0 shipped.
pnpm publish --access public --no-git-checks

# 5. Verify both versions landed in the registry.
pnpm view @bjornpagen/bumbledb-darwin-arm64@0.4.0 version
pnpm view @bjornpagen/bumbledb@0.4.0 version

# 6. Tag v0.4.0 (owner ceremony; the release-staged commit is already pushed).
```

`--access public` is mandatory: scoped packages publish restricted by default,
and without it coworkers cannot install. Both manifests also carry
`publishConfig.access: "public"`, so the flag is belt-and-suspenders.
`--no-git-checks` is needed whenever publishing from a branch other than main
(true in the release worktree).

## Post-publish, step one: the bumbledb lockfile regeneration

TODO.md's standing release-flow note (recurs every version): the version-bump
commit pins the exact platform optional-dep BEFORE that package exists in the
registry, so the CI sdk lane's `--frozen-lockfile` install fails between bump
and publish. Immediately after both packages verify in the registry:

```sh
cd ts && pnpm install --no-frozen-lockfile
# commit the regenerated pnpm-lock.yaml (one commit, the known bootstrap gap)
```

Note the release-age lag: pnpm 11's default `minimumReleaseAge` (1440
minutes) refuses any just-published package for ~24h, so consumers who do not
exclude `@bjornpagen/*` (this repo does, in `ts/pnpm-workspace.yaml`) cannot
install a fresh release until a day after publish.

## Post-publish, step two: the primer cutover lands

Primer's 0.4.0 sweep (host-idiom-0.4.0 PRD-P1) is staged on its own branch
with `@bjornpagen/bumbledb` pinned exactly `0.4.0` and its lockfile
deliberately stale. After both packages verify in the registry, follow the
runbook in that branch's PR body — publish 0.4.0 → `pnpm update -i` (or
`pnpm install --no-frozen-lockfile`) → typecheck → commit the lockfile →
merge. The steps live there, not here.

## The 0.4.0 pre-publish proof (already executed)

Before publish, both packages were packed and scratch-installed from tarballs
— the same proof shape as 0.1.0/0.2.0/0.3.0, upgraded to exercise the 0.4.0
HOST IDIOM:

```sh
# Pack both into /tmp/rel-040 (manifests verified via tar -tzf: main = dist/ +
# src/ + COOKBOOK/README/LICENSE/package.json, NO .node; platform =
# bumbledb.node + manifest + license only).
cd ts && pnpm pack --out /tmp/rel-040/bumbledb-0.4.0.tgz
cd ts/npm/darwin-arm64 && pnpm pack --out /tmp/rel-040/bumbledb-darwin-arm64-0.4.0.tgz

# Fresh scratch project: the platform tarball satisfies the main's exact
# 0.4.0 optional-dep pin via a pnpm-workspace.yaml override (pnpm 11 ignores
# package.json#pnpm.overrides; the registry has no 0.4.0 yet).
mkdir /tmp/bumbledb-smoke-040 && cd /tmp/bumbledb-smoke-040
# pnpm-workspace.yaml:
#   packages: ['.']
#   overrides:
#     '@bjornpagen/bumbledb-darwin-arm64': file:/tmp/rel-040/bumbledb-darwin-arm64-0.4.0.tgz
pnpm add /tmp/rel-040/bumbledb-0.4.0.tgz

# The NEW surface end to end, real values asserted: a closed vocabulary with
# payload columns; an insert spelled with string handles (kind:
# "DirectPass"); a wrong-string insert asserted to THROW the pointed marshal
# error naming the vocabulary and its roster; a prepared query whose result
# row's closed column strict-equals the handle name; a native `switch` over
# that value made exhaustive with `satisfies never`; a plain-array
# membership match ({ kind: ["DirectPass", "JudgedPass"] }); the typed
# `Kind.axioms` readback. This ran green on 2026-07-19 (node 24 runs the
# .ts smoke via type stripping — `satisfies` is erasable).
node smoke.ts   # prints: SMOKE OK: packed 0.4.0 tarballs — string-handle
                # insert, wrong-string marshal throw, named result row,
                # native switch (satisfies never), array membership,
                # Kind.axioms readback, end to end
```

## Provenance (CI only)

`npm publish --provenance --access public` attaches a signed provenance
attestation, but requires a CI runner (a macOS-arm64 GitHub Actions runner that
builds the `darwin-arm64` artifact). It is NOT available from a plain local
publish. If/when CI is added, publish order stays platform-first, main-second.

## Verifying a published install

On a clean darwin-arm64 machine:

```sh
mkdir /tmp/bumbledb-smoke && cd /tmp/bumbledb-smoke
npm init -y
npm install @bjornpagen/bumbledb
node --input-type=module -e "import { Db } from '@bjornpagen/bumbledb'; console.log(typeof Db)"
```

The optional platform dep resolves automatically and the loader binds the
addon. On any non-darwin-arm64 host the install still SUCCEEDS (the main
package is pure JS), but the first load throws the typed unsupported-platform
error naming the running platform-arch and that only `darwin-arm64` ships.
