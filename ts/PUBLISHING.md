# Publishing @bjornpagen/bumbledb

The owner-run release runbook. Owner-run, from the `ts/` package root, on a
darwin-arm64 host, logged in to npm (`pnpm whoami` answers). This repo builds
and verifies both packages; the agent side does NOT publish. `npm publish` /
`pnpm publish` and the git tag are owner ceremony. The runbook below is the
ONE spelling of the procedure (the old one-liner duplicated it and carried a
stale absolute worktree path). Each publish stops and prompts interactively
for the npm OTP (2FA), so run it in a real terminal with the authenticator
open. The main publish runs `prepublishOnly` → the full build (lockstep
assertion, cargo release build, smoke-load through the by-name loader path,
tarball-manifest verification) before anything uploads.

`0.8.0` is the capacity release, a deliberate backwards-incompatible hard
break over `0.7.0` — the count window dies into the CAPACITY statement
(`Target <=[w]{lo..hi} Source`, the aggregate containment;
`docs/design/capacity-laws.md` §8 rulings 1-6 + §8b C1-C19, both design docs
stamped LANDED). The count spelling `<={lo..hi}` survives
character-for-character as the unit-weight instance; weighted measures land
(`weigh(f(...))`, Duration weights — calendar capacity as one statement);
dependent hi-bounds read the target row (`ref("supply")` — the power-budget
shape); path weights refuse typed naming the pinned-column composition
idiom. `count.ts` dies whole: the five count constructors become the one
positional `capacity()` builder with `within()`/`weigh()`/`ref()`/
`duration()`, the ban table reborn per-aggregate. The violation payload
becomes `measure` (bigint, whole). The storage format crosses to v7 (the
capacity cutover: the schema encoding moved — statement-form tag 4 under the
v5 label — and the `R` namespace gained the weighted value-slot arm) — old
stores are refused, not migrated. The fingerprint statement: EVERY schema
fingerprint moves (the v4→v5 encoding label is the hash stream's first
bytes) — the cross-host lock and every cookbook golden re-derived in-tree.

Lineage: `0.7.0` was the previous hard break, over `0.6.0` — the audit
campaign (22 rulings R1-R22 + 158 findings: the `WriteResult` sum honoring
`abandon()`, `Tx.insert`'s changed bit, disposable lifetimes, `explain()`,
closed-column const accessors; storage v6 merged the id allocators). Before
it, `0.6.0` broke `0.5.0` — VARS BECOME
VALUES: `v(relation)` mints a record of fresh, class-typed query variables
built for ES destructuring, variable identity moves from name to OBJECT
REFERENCE (reusing the same var value across binding positions IS the join),
and `select(strings)` dies into `find({ key: varOrAgg })`; `r.var` removed,
no shim, zero fingerprint pins moved. Before it, `0.5.0` broke `0.4.0` — it
removed the plural variable mint (`r.var` became the sole variable
constructor) and landed
the pre-1.0.0 surface pair: the keyed point read `get()` and host-side answer
ordering (`by()`/`desc()`; the engine still never orders), adding exactly one
fingerprint pin (`r30`, the keyed-read recipe). Before it, `0.4.0` was a hard
break over `0.3.0` (the drizzle law:
database idioms arrive as modern TypeScript idioms) — closed handles became
string-literal unions on every surface, `Kind.match`/`fromId`/the handle
constants/`oneOf()` died, dispatch became native `switch` narrowing, set
membership a plain array, and closed fields left the orderable/foldable set.
That break left the wire, manifest, and fingerprint UNTOUCHED: zero
fingerprint pins moved (the cross-host lock and the T5 cookbook goldens
stayed byte-identical to the 0.3.0 tree).

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

The version lives in ONE place: `ts/package.json` `version`. Four values must
match exactly, and the build (`assertVersionLockstep` in `scripts/build.ts`)
fails if they diverge:

1. `ts/package.json` `version`
2. `ts/package.json` `optionalDependencies["@bjornpagen/bumbledb-darwin-arm64"]`
3. `ts/npm/darwin-arm64/package.json` `version`
4. `ts/crate/Cargo.toml` `version` (finding 139: `engine_version()` bakes
   `CARGO_PKG_VERSION` into the shipped binary — the one version string
   readable at runtime)

A release bump edits all four, then the build enforces the match. All four
are set to `0.8.0` in this tree; `pnpm run build` asserts the lockstep on
every run (`bumbledb build: version 0.8.0 (main == platform ==
optionalDependencies pin == crate manifest)`).

## Runbook (0.8.0, darwin-arm64 host, owner — staged 2026-07-25; recurs as the template for the next version)

```sh
# 0. From the ts/ package root, on a macOS Apple Silicon machine.
cd ts

# 1. The lockstep is already set to 0.8.0 in all FOUR places (done in this
#    tree; the build asserts it):
#    - ts/package.json                    "version": "0.8.0"
#    - ts/package.json                    optionalDependencies pin -> "0.8.0"
#    - ts/npm/darwin-arm64/package.json   "version": "0.8.0"
#    - ts/crate/Cargo.toml                version = "0.8.0"

# 2. Build + verify both trees (fails on version drift, unloadable artifact,
#    or a mispacked tarball). Produces dist/ and npm/darwin-arm64/bumbledb.node.
pnpm install
pnpm test           # runs the build, then node --test (the ONE test spelling)
pnpm exec tsc --noEmit
pnpm exec biome check .

# 3. Publish the PLATFORM package FIRST — the main's exact-pinned optional dep
#    must already exist in the registry when the main resolves.
#    (Interactive: npm prompts for the 2FA one-time password.)
pnpm publish --no-git-checks ./npm/darwin-arm64

# 4. Publish the MAIN package SECOND. (`prepublishOnly` reruns the build;
#    another OTP prompt.) `ts/package.json` already carries "private": false —
#    there is no toggle to flip since 0.1.0 shipped.
pnpm publish --no-git-checks

# 5. Verify both versions landed in the registry.
pnpm view @bjornpagen/bumbledb-darwin-arm64@0.8.0 version
pnpm view @bjornpagen/bumbledb@0.8.0 version

# 6. The v0.8.0 tag is already pushed with the release commit (the 0.8.0
#    campaign close staged commit + tag together); both publishes remain
#    owner ceremony — the agent side never publishes.
```

Public access is mandatory (scoped packages publish restricted by default,
and without it coworkers cannot install) and has ONE spelling: both manifests
carry `publishConfig.access: "public"` — the redundant `--access public` flag
is deleted from the commands. `--no-git-checks` is needed whenever publishing
from a branch other than main (true in a release worktree).

## Post-publish, step one: the bumbledb lockfile regeneration

The standing release-flow gap (recurs every version): the version-bump
commit pins the exact platform optional-dep BEFORE that package exists in the
registry, so the CI sdk lane's `--frozen-lockfile` install fails between bump
and publish. Immediately after both packages verify in the registry:

```sh
cd ts && pnpm install --no-frozen-lockfile
# commit the regenerated pnpm-lock.yaml (one commit, the known bootstrap gap)
```

For 0.6.0 this landed (4b2b3a0c, 2026-07-20). For 0.7.0 the regeneration
never landed — the lockfile simply carried no platform entry from the 0.7.0
release commit onward (pnpm drops an unresolvable optional dep), and the
0.8.0 release commit re-derives the same shape; the debt clears when this
step runs after the 0.8.0 publish. One sharp edge learned at 0.6.0:
with a warm `node_modules`, `pnpm install` may answer "Already up to date"
without re-resolving — remove `node_modules` first if the lockfile refuses
to move.

Note the release-age lag: pnpm 11's default `minimumReleaseAge` (1440
minutes) refuses any just-published package for ~24h, so consumers who do not
exclude `@bjornpagen/*` (this repo does, in `ts/pnpm-workspace.yaml`) cannot
install a fresh release until a day after publish.

## Post-publish, step two: the primer cutover lands

Primer main is already cut over to `^0.5.0` (the 0.5.0 cutover merged). The
0.6.0 adoption is staged at the primer `bumbledb-060` worktree (branch
`worktree-bumbledb-060`) with `@bjornpagen/bumbledb` pinned `^0.6.0` and its
`bun.lock` deliberately untouched — the same documented bootstrap gap, now
UNBLOCKED: both 0.6.0 packages are in the registry (published + tagged
`v0.6.0`, 2026-07-20). Remaining: install (the lockfile moves) →
typecheck → commit the lockfile → merge. The steps live there, not here.

## The pre-publish proof (executed for 0.4.0; re-run the same shape for 0.6.0)

The 0.6.0 rerun keeps this exact tarball proof shape but must exercise the NEW
surface in place of the 0.4.0 host-idiom checks: a destructured `v()` mint
joined by reference (`const { id, toGrp } = v(candidateEdge)`, the same var
value reused across binding positions to spell the join) and a
`find({ ... })` renamed result row whose keys strict-equal the answer's named
columns.

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

`npm publish --provenance` attaches a signed provenance (access stays
public through `publishConfig.access`, the one spelling)
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
