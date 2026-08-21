# primer-adoption — the D10/D11/D12 sweep for `../primer-spec`

The downstream adoption change for release **0.16.0**, produced as a
git-apply-able patch ([primer-adoption.patch](primer-adoption.patch))
because primer-spec is not writable from the build sandbox — the owner
applies it. It is the one-PR ledger sweep [70-deletions.md](70-deletions.md)
couples to the release ("D10, D11, D12 land in Primer's adoption change
against the release carrying 20/40/50 — one Primer PR, one ledger sweep")
and the Wave 3 Primer half of [80-acceptance.md](80-acceptance.md).

**Base commit (primer-spec):** `d4f1efd0c98b13f6fecb1b4e2c3f2143274a2009`
("Centralize every vendor model id in one seat registry."). The patch is
verified with `git apply --check` against that commit; the sha is also
recorded in the patch header.

## What the patch does, entry by entry

### D10 — the packing shim dies (V10)

`src/storage/bumbledb/runtime.ts`: `columnBatch`, `loadColumns`,
`loadBatchSize = 16_384`, `isFactArray`, `isStringRecord` (its only
consumer was `columnBatch`), `isCompleteColumnBatch`, and the local
`ColumnBatch`/`CollectionWrite` types' column arm are deleted. The
runtime's admit load becomes the plain call —
`load: (relation, facts) => nativeBuilder.load(relation, facts)` — and
the `BumbledbCollectionBuilder.load` wrapper signature mirrors the SDK's
one collection spelling, `Iterable<Fact<R>>`. The `AnyRelation` import is
dropped in the same hunk: its only consumers were the shim
(`noUnusedLocals` would otherwise refuse). Runtime.ts never imported
`ColumnBatch` from the SDK — both column types were local respellings —
so no other import moves.

The sweep does not stop at runtime.ts, because upstream acceptance 3
("Primer needs no Bumbledb-specific packing code") covers all of Primer
and two more packing sites feed the same deleted transport — leaving them
would break `pnpm run typecheck` the moment the wrapper narrows to
`Iterable<Fact<R>>`:

- `src/oracle/bumbledb/backend.ts`: `RelationColumnBatch`,
  `isRelationColumnBatch`, and the `loadColumns` member die. The member is
  replaced by `load(builder, relationIdentity, rows)` — rows judged by the
  same `isRelationFacts` heading law every other backend load already
  uses, then the plain `builder.load(named.handle, rows)`. One load
  spelling, one judge.
- `src/stages/normalization/witness.ts`: `NormalizationWitnessWriter`
  drops its parallel column buffers and the `batchCapacity = 16_384`
  mid-stream flushes; it accumulates row facts (`{ input }`, `{ output }`,
  `{ output, input }`) and issues one `load` per relation at `close()`.
  Empty collections are lawful (preserved law 2), so no length guards.
  Holding full row arrays until close is the same pattern
  `src/stages/*/persist.ts` already uses for the 4 M-fact persist — the
  accepted collection makes the one big load the fast path.

### D11 — counting by scanning dies (V6, V10)

`src/storage/bumbledb/runtime.ts::countRelations`:
`counts[name] = BigInt(instance.scan(relation).length)` becomes
`counts[name] = instance.count(relation)` — the new `ReadInstance.count`,
`bigint` by law, same one-lease structure (all relations counted inside
one `database.read`). A sweep of `primer-spec/src`, `scripts`, and
`packages` for `scan(…).length` and full-binding count queries found no
other scan-as-count site (there is no `src/dev/readback.ts` at this
commit; every other `scan` call materializes rows it actually consumes,
which stays lawful — D9 deletes scan-as-*count*, not scan).

### D12 — the generic-binding suppression (V7): vacuous at this commit

A grep of `primer-spec/src`, `scripts`, and `packages` at
`d4f1efd0c98b1` finds **zero** `@ts-expect-error`, `@ts-ignore`,
`@ts-nocheck`, or `biome-ignore` markers — the localized suppression the
upstream report described is already gone from this tree. The dynamic
oracle backend (`src/oracle/bumbledb/backend.ts`) composes `v()`/`match`
through its own structural `DynamicScope`/`DynamicChain` re-typing, which
is not a suppression and predates this set; it stands unchanged. D12
therefore lands as a recorded no-op: the pin is that primer-spec compiles
with zero bumbledb-related suppressions, which it does before and after
the patch.

### The dependency pin

`package.json` pins `"@bjornpagen/bumbledb": "0.15.0"`, so the bump to
`0.16.0` is **included in the patch** (first hunk).

## Applying

From the primer-spec root, at base commit `d4f1efd0c98b1`:

```sh
git apply --check primer-adoption.patch   # dry run — must be silent
git apply primer-adoption.patch
```

Follow-ups — no new dependencies enter the tree; the only manifest change
is the bumbledb pin, so after 0.16.0 is published (owner ceremony,
`ts/PUBLISHING.md`) one lockfile refresh realizes it:

```sh
pnpm install   # picks up @bjornpagen/bumbledb 0.16.0, nothing else
pnpm check     # primer-spec's one gate: typecheck + format + lint
```

`pnpm check` is the whole in-repo gate by primer-spec's own law
(AGENTS.md: the typechecker, formatter, and linter are the check; tests
are banned there, so the pins for D10–D12 live in bumbledb's suites and
in the acceptance run below).

## The acceptance run

Owner ceremony, from primer-spec, per
[80-acceptance.md](80-acceptance.md) (the `verify:*` entry point is the
owner's — primer-spec agents may not add one):

```sh
pnpm run verify:learning-commons
```

Expected outcomes, closing upstream report 1's conditions 3–8:

- Persistence of the 3,993,828 facts across 39 relations falls
  **materially below 27.61 s**; the full verifier improves from 58.02 s;
  peak RSS falls materially below 7.22 GiB.
- Relation counts read through `count` without materializing fact rows —
  the ~250 ms aggregate-query readback and the 4 M-object decode are gone
  from the verifier profile (D11's pin).
- Primer's runtime carries no transpose, no batch size, no column
  assembly (D10's pin; acceptance 3).
- The three canonical digests are **byte-identical** — the stop-ship
  invariant; any drift is a defect in the change, categorically:
  - Source IR
    `27202ace4da1317a592f523c80431c38670d9ec04796b80f0eac2eae6ff0b3d1`
  - Standards Evidence IR
    `efa086b986b1bb7839b45c1407fabc649e2d400e8b3aaf61197fc987e4dc1706`
  - normalization ledger
    `cc1b3ee64ecb01c69acbb4633f4ea961c5a5420da17d1e04568661dd5d6f49d7`

## How the patch was verified (evidence, not assertion)

Against a full copy of primer-spec at the base commit with the patch
applied and the SDK typings carrying the 0.16.0 `ReadInstance.count`
surface:

- `git apply --check primer-adoption.patch` against
  `d4f1efd0c98b13f6fecb1b4e2c3f2143274a2009`: clean.
- `tsc --noEmit -p tsconfig.json` (primer-spec's own strict config,
  `noUnusedLocals` on): zero errors — and zero errors on the unpatched
  baseline, so the delta introduces nothing.
- `biome format` and `biome lint` over the four touched files, plus
  primer-spec's `scripts/dev/fmt.ts` (superfmt) and `scripts/dev/lint.ts`
  (superlint): clean — the patch is format-stable under `pnpm check`.
