# Publishing @bjornpagen/bumbledb

## The 0.3.0 one-liner

Owner-run, from the release worktree, on a darwin-arm64 host, logged in to npm
(`pnpm whoami` answers). Platform first, then main — one `&&` chain; EACH
publish stops and prompts interactively for the npm OTP (2FA), so run it in a
real terminal and have the authenticator open:

```sh
cd /Users/bjorn/Documents/bumbledb/.claude/worktrees/hardening-030/ts && pnpm publish --access public --no-git-checks ./npm/darwin-arm64 && pnpm publish --access public --no-git-checks
```

`--no-git-checks` is REQUIRED here: pnpm refuses to publish from a non-main
branch, and this worktree sits on `worktree-hardening-030`. The main publish
runs `prepublishOnly` → the full build (lockstep assertion, cargo release
build, smoke-load through the by-name loader path, tarball-manifest
verification) before anything uploads.

The owner-run release runbook (hardening-0.3.0 PRD-V1). This repo builds and
verifies both packages; the agent side does NOT publish. `npm publish` /
`pnpm publish` and the git tag are owner ceremony.

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
are set to `0.3.0` in this tree, and `pnpm run build` has confirmed the
lockstep (`bumbledb build: version 0.3.0 (main == platform ==
optionalDependencies pin)`).

## Runbook (0.3.0, darwin-arm64 host, owner)

```sh
# 0. From the ts/ package root, on a macOS Apple Silicon machine.
cd ts

# 1. The lockstep is already set to 0.3.0 in all THREE places (done in this
#    tree; the build asserts it):
#    - ts/package.json                    "version": "0.3.0"
#    - ts/package.json                    optionalDependencies pin -> "0.3.0"
#    - ts/npm/darwin-arm64/package.json   "version": "0.3.0"

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
pnpm view @bjornpagen/bumbledb-darwin-arm64@0.3.0 version
pnpm view @bjornpagen/bumbledb@0.3.0 version

# 6. Tag v0.3.0 (owner ceremony; the release-staged commit is already pushed).
```

`--access public` is mandatory: scoped packages publish restricted by default,
and without it coworkers cannot install. Both manifests also carry
`publishConfig.access: "public"`, so the flag is belt-and-suspenders.
`--no-git-checks` is needed whenever publishing from a branch other than main
(true in the release worktree).

## Post-publish: the primer cutover lands

Primer's 0.3.0 sweep (hardening-0.3.0 PRD-P2) is already staged on its own
branch with `@bjornpagen/bumbledb` pinned exactly `0.3.0` and its lockfile
deliberately stale. After both packages verify in the registry, follow the
runbook in that branch's PR body — publish 0.3.0 → `pnpm update -i` (or
`pnpm install --no-frozen-lockfile`) → typecheck → commit the lockfile →
merge. The steps live there, not here.

## The 0.3.0 pre-publish proof (already executed)

Before publish, both packages were packed and scratch-installed from tarballs
— the same proof shape as 0.1.0 and 0.2.0, upgraded to exercise the 0.3.0
surface:

```sh
# Pack both into /tmp/rel-030 (manifests verified via tar -tzf: main = dist/ +
# src/ + COOKBOOK/README/LICENSE/package.json, NO .node; platform =
# bumbledb.node + manifest + license only).
cd ts && pnpm pack --out /tmp/rel-030/bumbledb-0.3.0.tgz
cd ts/npm/darwin-arm64 && pnpm pack --out /tmp/rel-030/bumbledb-darwin-arm64-0.3.0.tgz

# Fresh scratch project: the platform tarball satisfies the main's exact
# 0.3.0 optional-dep pin via a pnpm-workspace.yaml override (pnpm 11 ignores
# package.json#pnpm.overrides; the registry has no 0.3.0 yet).
mkdir /tmp/bumbledb-smoke-030 && cd /tmp/bumbledb-smoke-030
# pnpm-workspace.yaml:
#   packages: ['.']
#   overrides:
#     '@bjornpagen/bumbledb-darwin-arm64': file:/tmp/rel-030/bumbledb-darwin-arm64-0.3.0.tgz
pnpm add /tmp/rel-030/bumbledb-0.3.0.tgz

# The NEW surface end to end, real values asserted: a law-typed schema (pure-
# structure fields; the class map read off the schema value — Attempt.kind
# lands in class "Kind.id"), a psi statement (contained over
# Kind.where({ mastered: true }) — a Failed-kind certificate REJECTED at
# commit), Db.create + insert, a closed-atom query with a psi face through
# prepare/execute (the minted id and roster rank round-trip), and an
# exhaustive Kind.match dispatch. This ran green on 2026-07-18.
node smoke.mjs   # prints: SMOKE OK: packed 0.3.0 tarballs — law-typed class
                 # map, psi statement judged, closed-atom prepare/execute,
                 # Kind.match dispatch, end to end
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
