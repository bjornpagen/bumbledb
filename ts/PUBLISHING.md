# Publishing @bjornpagen/bumbledb

The owner-run release runbook. Owner-run, from the `ts/` package root, on a
darwin-arm64 host, logged in to npm (`pnpm whoami` answers). Node >=24 is
the floor for `engines` (`ts/`, `ts/npm/*`, `ts-log`) and for the `.ts`
test runner and build scripts; the AL2023 CI cells install `nodejs24`. This repo builds
and verifies the main package and the darwin-arm64 platform package; the
linux-arm64 `.node` arrives from the amazonlinux:2023 CI run. The agent
side does NOT publish. `npm publish` /
`pnpm publish` and the git tag are owner ceremony. The runbook below is the
ONE spelling of the procedure (the old one-liner duplicated it and carried a
stale absolute worktree path). Each publish stops and prompts interactively
for the npm OTP (2FA), so run it in a real terminal with the authenticator
open. The main publish runs `prepublishOnly` → the full build (lockstep
assertion, cargo release build, smoke-load through the by-name loader path,
tarball-manifest verification) before anything uploads.

`0.17.2` is the `internalDescriptor` and two-platform release over
`0.17.1` — one export, one roster widening. The engine's own sealed
descriptor crosses the FFI as `internalDescriptor(spec): SealedDescriptor`
(the `#internal`-doc'd sibling of `internalBlake3`), and its consumer is
`@bjornpagen/bumbledb-log`'s descriptor parse: relation ids, sealed
fields, closed rosters, materialized statements, and the real fingerprint
so the driver's 741-line re-derivation and fingerprint-mirror string-axiom
refusal delete rather than get a second fix. The shipped set widens from
the darwin-arm64 singleton to `{darwin-arm64, linux-arm64}`; the linux
artifact is built in `amazonlinux:2023` (glibc 2.34) and placed by the
owner from the CI run into `ts/npm/linux-arm64/` before publish. Nothing
else moved: the C ABI stays **generation 4** (`bdb_abi_version()` is
unchanged — an added JS export is not an ABI event), storage stays
format **8**, and no fingerprint pin moved. The lockstep is prepared
unpublished in this tree; the publish command sequence is the standing
one below, and the log package's 0.18.0 steps follow it with peer
`^0.17.2`.

`0.17.1` is the `internalBlake3` release over `0.17.0` — one export, one
consumer. The engine-linked blake3 hash crosses the FFI as
`internalBlake3(data: Uint8Array): Uint8Array` (the `#internal`-doc'd
seam `30-engine-seams.md` blesses by name), and its consumer is
`@bjornpagen/bumbledb-log`'s store tier: computed object etags, the
descriptor digest, and the chain hashes all spend it, which is what
keeps the export alive under the consumer-roster law. Nothing else
moved: the C ABI stays **generation 4** (`bdb_abi_version()` is
unchanged — an added JS export is not an ABI event), storage stays
format **8**, and no fingerprint pin moved. This release exists to
unblock `@bjornpagen/bumbledb-log`'s first publish, whose
peerDependency is `^0.17.1`; the publish command sequence is the
standing one below, and the log package's own first-publish steps
follow it.

`0.17.0` is the purge release, and the first published version after
`0.15.0` (`0.16.0` was completed but never published; its changes ship
here). The engine surface was re-derived from its consumers: every
feature now names primer-spec, a bumbledb-log lane, a deployment case,
or a structural gate — or it is gone. Deleted end to end: the
measure/duration query family (`Duration(v)` comparisons, duration
head projections and folds, and the ray-probe second execution pass
that existed only for them — compute `end − start` on the host), the
Explain/Staleness diagnostic stacks, the `db.*` sugar quintet, the free
comparison exports and the `Tx` alias, bool Min/Max folds, the unit
`{N..*}` capacity floor (weighted floors stay legal), and every
harness-only knob (`set_derived_budget`, `admit_measured`,
`disk_size` stays — it is product). Withdrawn by the cheap-and-useful
ruling and fully intact: `abandon()`, `ParamSet`/`inSet`,
`LiteralSet::Many`, the whole interval/Allen family, and the entire TS
type tier. The normative `docs/architecture/` set is deleted — the code
is the spec and the census enforces the one-owner law. **The C ABI bumps to generation 4**: the
D1 deletions renumbered `bdb_error_kind` and `bdb_find_term_kind`, so
generation-3 hosts must recompile against the regenerated header.
Storage stays format **8** — no migration; existing stores open
unchanged. The cross-host fingerprint lock schema gained a
`Duration(active)` weight (its pin regenerated in lockstep on both
sides).

`0.16.0` is the one-representation release over `0.15.0` (completed
2026-08-21, **never published** — folded into `0.17.0` above) — one collection
representation from host to delta (the accepted collection: an arena-backed
shape-proved batch parsed once at the bridge; the column transport is GONE —
`Iterable<Fact<R>>` is the one collection spelling), one cardinality read
(`count` on `ReadInstance`/`OwnedInstance` with the `db.count` symmetry
sugar, `bigint` by wire law, the maintained format-8 counter — never a
scan), the generic full-binding law (`match(relation, v(relation))` at every
site), and containment target-key parity at every boundary (`schema()`
refuses in names what the engine refuses in names-beside-ids). Storage stays
format **8** — no migration: `count` reads a stat every format-8 store
already maintains, so existing stores open unchanged. The C ABI stays **3**
— `bdb_abi_version()` is unchanged; `bumbledb-c` rides the lockstep with no
surface change. The one breaking change is TypeScript-only: `ColumnBatch`
and the column write transport are removed; the replacement is the same
`insert`/`load` call with fact objects, now the fastest path. Engine crates,
`bumbledb-c`, the napi crate, and both npm packages share one spelling.

`0.15.0` is the admitted-instance / ABI-3 / format-8 release over `0.14.0` —
one public engine: **the store** (`Db`, leased `ReadInstance` / `WriteTx`)
and **the value** (`OwnedInstance`, `InstanceBuilder`, `Admission`).
Format 8 and ABI 3 are revised in place under the pre-publish rule: the
`_meta` roster is four keys (format, fingerprint, generation, dict-next);
kind is not data; there is no theory-less open and no public instance
trait. Snapshot-named surfaces are gone. C `bdb_abi_version()` stays 3.
**Pre-publish format-8 stores rebuild from source** — there is no
in-format migration for a roster revised before publish. Engine crates,
`bumbledb-c`, the napi crate, and both npm packages share one spelling.

`0.14.0` is the write-algebra / ABI-2 release over `0.12.2` — one collection
`insert`/`delete`/`reserve` inside `write`; empty, singleton, and many are one
collection; ETL is a host loop of `write` (`scan` then `insert_dyn`);
`bdb_abi_version()` is 2 (collection insert/delete, `reserve`, retirement of
`alloc` / `bulk_load`). Engine crates, `bumbledb-c`, the napi crate, and both
npm packages share one spelling. Wire, manifest, storage format (v7), and
schema fingerprints are UNTOUCHED.

The in-tree collection-write cutover was briefly spelled `0.13.0`; `0.14.0`
is the published identity of that algebra.

`0.12.0` is the Query-sum / signature-v6 / rec-keyword / `.reach()`
release over `0.11.0` — the public Query is `Cq | Reach`, introspection
is `signature` at v6, the host surface spells `rec` / `.reach()`, OccBind
and the C ABI tagged union land with it. Wire, manifest, storage format
(v7), and schema fingerprints are UNTOUCHED.

`0.11.0` is the trusted-layer representation release over `0.10.0` —
trusted-layer sums (Query pipeline, sealed schema, exec Agg/Dedup,
C++/TS dialect IR), `PreparedQuery::signature()` (was `predicate()`),
introspection v6. Manifest, storage format (v7), and schema fingerprints
are UNTOUCHED. Public Query on the wire is the tagged encoding of
`Cq | Reach` (discriminant + payload; CQ does not carry a rec pointer).
Campaign detail is in git history.

`0.10.0` is the bugbash-perf campaign release over `0.9.0` — 44 verified
findings fixed (12 bugs, 3 high), the read path measurably faster (report
reps 0.87–0.94 vs the prior estate on five of six, scenarios 0.88 with the
graph-world regression cluster reversed), and the weighted-capacity judge's
C17 measured choice landed: the value-slot arm won the power-budget lane
(−17%/−21% on the judged surface) and ships as the ONLY form, with C20
(ruled 2026-08-03) blessing its write-time consequence as doctrine — a
ray-valued Duration weight refuses at WRITE time, strictly stronger than
C10's judge-time refusal (the C-series record, `lean/Bumbledb/Capacity.lean`). On the TS
tier the type walls tighten to the engine's: cross-domain
order/`pointIn`/Allen spellings and a unit capacity taking a `duration()`
bound now die at COMPILE time (both were engine refusals at runtime before —
code that stops compiling was already broken), and `explain()` gains R13
execute-symmetry (profile/introspect take the mixed `ParamArg` entry). Wire,
manifest, storage format (v7), and every schema fingerprint are UNTOUCHED —
zero pins moved. Campaign detail is in git history.

Lineage: `0.9.0` was a minor over `0.8.0` that broke nothing — the zero-key
identity comparators (`by()`/`desc()` over exactly the engine-orderable
roster), the R3 bool-order tail closed on the TS query tier, the primer
expressibility pins, and the platform pin's move out of the repo manifest
into pack-time injection (the sdk lane's bootstrap circle died there).
Before it, `0.8.0` was the capacity release, a deliberate backwards-incompatible
hard break over `0.7.0` — the count window dies into the CAPACITY statement
(`Target <=[w]{lo..hi} Source`, the aggregate containment;
rulings 1-6 + C1-C19, the record now carried by `lean/Bumbledb/Capacity.lean`
and `lean/Bumbledb/Subsumption.lean`). The count spelling `<={lo..hi}` survives
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

Before it, `0.7.0` was the previous hard break, over `0.6.0` — the audit
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

## The three packages

| Package | Contents | `os`/`cpu` |
| --- | --- | --- |
| `@bjornpagen/bumbledb` | pure JS + `.d.ts` (no binary) | none (installs everywhere) |
| `@bjornpagen/bumbledb-darwin-arm64` | only `bumbledb.node` | `darwin` / `arm64` |
| `@bjornpagen/bumbledb-linux-arm64` | only `bumbledb.node` | `linux` / `arm64` (AL2023) |

The PUBLISHED main manifest declares every shipped platform package as an
`optionalDependency` pinned EXACT to its own version — but the REPO manifest
carries NO pin: `scripts/pin.ts` injects them at `prepack` and removes them
at `postpack` (the napi prepublish pattern), so every tarball `pnpm pack` /
`pnpm publish` produces carries the pins while the committed tree stays
registry-independent. This kills the sdk lane's bootstrap circle
permanently: a lockfile can never pin the CURRENT unpublished version, so a
committed pin put every release in a red-CI window (`--frozen-lockfile`
refused the unresolvable exact pin) until a post-publish lockfile
regeneration — now impossible to need. npm/pnpm install a platform
package only on a matching host; the main package's loader
(`src/native.ts`) resolves it by name at runtime and throws a typed
unsupported-platform error everywhere else.

## Version lockstep

The version lives in ONE place conceptually: `0.16.0` is the identity of the
layout. The build (`assertVersionLockstep` in `scripts/build.ts`) fails if
any of these diverge:

1. `ts/package.json` `version`
2. every `ts/npm/<shipped>/package.json` `version` (`darwin-arm64`, `linux-arm64`)
3. `ts/crate/Cargo.toml` `version` (`engineVersion()` bakes
   `CARGO_PKG_VERSION` into the shipped binary)
4. every engine-workspace member, parsed from the root `Cargo.toml`
   `members` list (engine, bench, macros, query, query-macros, theory)
5. `crates/bumbledb-c/Cargo.toml` (`bdb_version()` bakes
   `CARGO_PKG_VERSION`; `bdb_abi_version()` is 3 — layout generation, not
   the release spelling)

Engine + C + TS cannot disagree: a new workspace crate, a drifted
`bumbledb-c`, or a napi/npm mismatch fails the build before anything is
produced.

The platform PIN is not a repo value: `scripts/pin.ts` derives it from the
manifest's own `version` at pack time (exact by construction), the gate
REFUSES a committed `optionalDependencies` field outright, and the build's
tarball proof packs the main package for real and asserts the packed
manifest carries the exact-version pin — with the repo manifest restored
pin-free after.

A release bump edits every spelling, then the build enforces the match. All
spellings are `0.17.2` in this tree; `pnpm run build` asserts the lockstep
on every run.

## Runbook (0.17.2, darwin-arm64 host, owner)

```sh
# 0. From the ts/ package root, on a macOS Apple Silicon machine.
cd ts

# 1. The lockstep is already set to 0.17.2 (the build asserts it — the
#    platform pins are NOT repo fields, they inject at pack time):
#    - ts/package.json                    "version": "0.17.2"
#    - ts/npm/darwin-arm64/package.json   "version": "0.17.2"
#    - ts/npm/linux-arm64/package.json    "version": "0.17.2"
#    - ts/crate/Cargo.toml                version = "0.17.2"
#    - crates/bumbledb/Cargo.toml         version = "0.17.2"
#    - crates/bumbledb-c/Cargo.toml       version = "0.17.2"
#    - workspace members (bench, macros, query, query-macros, theory)
#    bdb_abi_version() answers 4 (unchanged: an added JS export is not
#    an ABI event).

# 2. Download Lane C's linux-arm64 artifacts from a green
#    bumbledb-log.yml run (amazonlinux:2023 on ubuntu-24.04-arm). The
#    two artifact names are `bumbledb.linux-arm64.node` and
#    `bumbledb-log-duty`. This hop never ran that workflow; the owner
#    waits for a real GitHub run with both artifacts attached.
#
#    gh run download <run-id> --name bumbledb.linux-arm64.node --dir /tmp/grail-artifacts
#    gh run download <run-id> --name bumbledb-log-duty --dir /tmp/grail-artifacts
#    cp /tmp/grail-artifacts/bumbledb.linux-arm64.node ts/npm/linux-arm64/bumbledb.node
#
#    The duty binary is not an npm package — it ships inside the Lambda
#    Layer. Keep it for examples/lambda/layer/duty/bin/bumbledb-log-duty
#    (mode +x) before the example deploy. Darwin is built on this
#    machine in step 3; linux-arm64 is placed, never rebuilt here.
#    Verify the packed linux-arm64 tarball is exactly LICENSE +
#    bumbledb.node + package.json.

# 3. Build + verify the darwin tree (fails on version drift, unloadable
#    artifact, or a mispacked tarball). Produces dist/ and
#    npm/darwin-arm64/bumbledb.node. The pack proof asserts both
#    platform pins.
pnpm install
pnpm test           # runs the build, then node --test (the ONE test spelling)
pnpm exec tsc --noEmit
pnpm exec biome check .

# 4. Publish BOTH PLATFORM packages FIRST — the main's exact-pinned
#    optional deps must already exist in the registry when the main
#    resolves. (Interactive: npm prompts for the 2FA one-time password.)
pnpm publish --no-git-checks ./npm/darwin-arm64
pnpm publish --no-git-checks ./npm/linux-arm64

# 5. Publish the MAIN package. (`prepublishOnly` reruns the build;
#    another OTP prompt. The prepack hook injects the exact-version
#    platform pins into the published manifest; postpack restores the
#    repo file.) `ts/package.json` already carries "private": false.
pnpm publish --no-git-checks

# 6. Verify the three versions landed in the registry.
pnpm view @bjornpagen/bumbledb-darwin-arm64@0.17.2 version
pnpm view @bjornpagen/bumbledb-linux-arm64@0.17.2 version
pnpm view @bjornpagen/bumbledb@0.17.2 version

# 7. Tag the release commit and push the tag (owner ceremony, like the
#    publishes — the agent side never publishes or tags):
#    git tag -a v0.17.2 <release-commit> -m "bumbledb 0.17.2" && git push origin v0.17.2
```

Public access is mandatory (scoped packages publish restricted by default,
and without it coworkers cannot install) and has ONE spelling: both manifests
carry `publishConfig.access: "public"` — the redundant `--access public` flag
is deleted from the commands. `--no-git-checks` is needed whenever publishing
from a branch other than main (true in a release worktree).

There is NO post-publish lockfile step. The old ritual — regenerate
`ts/pnpm-lock.yaml` after both packages verify in the registry, because the
release commit's manifest pinned a version the registry did not yet carry —
died at 0.9.0 with the pin's move out of the repo manifest: the committed
manifest and lockfile never mention the platform package, so there is
nothing to regenerate and no red-CI window between bump and publish, for
this release and every future one.

Note the release-age lag: pnpm 11's default `minimumReleaseAge` (1440
minutes) refuses any just-published package for ~24h, so consumers who do not
exclude `@bjornpagen/*` (this repo does, in `ts/pnpm-workspace.yaml`) cannot
install a fresh release until a day after publish.

## First publish of @bjornpagen/bumbledb-log 0.18.0 (after the SDK lands)

The log driver publishes AFTER the three 0.17.2 SDK packages verify in
the registry — its peerDependency is `^0.17.2`, unresolvable a minute
earlier. It is a platformless package: pure TypeScript source
(`files` ships `src/` + README, `exports` points at `src/index.ts`), no
napi half, no platform sibling, no pack-time pin injection — so the
whole ceremony is the verification trio and one publish. `ts-log/`'s
manifest already carries `publishConfig.access: "public"` and
`"private": false`; the repo-local `link:../ts` engine link lives in
`devDependencies`, which npm does not publish, so the published manifest
names only the peer range.

```sh
# From the ts-log/ package root, after step 6 above answers 0.17.2 thrice.
cd ts-log
pnpm install          # resolves the ^0.17.2 peer against the registry now
pnpm test             # node --test, the ONE test spelling
pnpm run typecheck
pnpm run lint
pnpm publish --no-git-checks   # interactive OTP, same as the SDK packages

# Verify:
pnpm view @bjornpagen/bumbledb-log@0.18.0 version
```

The package's own version is `0.18.0` — the log driver is not in the
engine lockstep; it rides its peer range instead, and `assertVersionLockstep`
never reads it.

## Post-publish: the primer cutover lands

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
addon. On a host outside the shipped set (`darwin-arm64`, `linux-arm64`)
the install still SUCCEEDS (the main package is pure JS), but the first
load throws the typed unsupported-platform error naming the running
platform-arch and the shipped set.
