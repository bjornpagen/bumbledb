# PRD-P1 — Primer: the store schema goes derived

Wave P · Repo: **primer** (`/Users/bjorn/Documents/primer`) · depends on: K8
(the final SDK surface, locally linked) · hard break

## Objective

Primer's graph-builder store schema
(`src/tools/graph-builder/store/schema.ts` — audited 2026-07-18: 22 `.as(`
sites, 57 `contained(` statements of which 55 are derivable — 43 single-field
refs + 12 closed-vocab refs; keeps: 2 composite containments, 3 `mirrors`, 31
keys, 32 windows) is the first real consumer of the coordinate kernel. Cut it
to the 0.3.0 idioms fully: derived coordinates, `ref`/`cites`, no hand
statement that a `ref` derives. The audit's bin map is the plan of record:
all 22 owner `.as` constants become derived fresh coordinates (each is a K3
construction error if left); ~40 reference fields become `ref`; the three
assumption-DU arms (`assumptionFromMember.assumption`,
`assumptionFromEdge.assumption`, `assumptionPreCourse.assumption`,
schema.ts ~773–796) are the mandated `cites` sites — their only lawful link is
the selected `mirrors` glue; bin-4 (`.as` kept for shared value domains) is
EMPTY at HEAD. Record in the map (a comment at the site): `capability: str`
is the custody spelling joined across four relations and `str` is outside the
labelable kinds — deliberately unlabeled, policed by the composite
containments; `task.subject: u64` is a nine-way sum-domain pointer and stays
deliberately bare (B-min working as designed — do NOT ref or label it).

## Context / constraints

- Primer is dev-only through the bun TUI; its stores are rebuildable
  artifacts. The schema rewrite WILL move the store fingerprint (derivation
  reorders/replaces statements). That is accepted: humans rebuild dev stores;
  no migration work in this PRD, and the commit body must SAY the fingerprint
  moved.
- Work against the LOCALLY LINKED SDK build (the 0.2.0-train procedure:
  `pnpm install` while the manifest still points at the published version,
  then symlink `node_modules/@bjornpagen/bumbledb` → the bumbledb worktree's
  `ts/` and the darwin-arm64 package → `ts/npm/darwin-arm64`; never run
  install after the link). P2 owns the version pin; this PRD may develop
  linked without touching `package.json`.
- Other agents work in primer — stay inside the graph-builder store cluster
  and its tests; ignore unrelated dirt.

## Work

1. Read the whole schema file first and MAP it: every `.as` label → is it an
   owned declaration (→ derived coordinate / `ref`), a reference to a fresh
   id (→ `ref`), a deliberate statement-free link (→ `cites`), or a genuinely
   shared value domain (→ keep `.as`, justify in a comment only if
   non-obvious)? Every `contained` → derivable by a `ref` (→ delete the hand
   statement) or composite/selected/ψ-shaped (→ keep, in the new spellings)?
2. Rewrite the schema: fresh fields drop their `.as` (K3 derives the
   coordinate; keeping the label is now a construction error); references
   become `ref(...)`; deliberate-weak links become `cites(...)`; derivable
   hand containments are DELETED (recipes' dedupe-keeps-hand rule is for
   fingerprint-stable migrations — primer accepts the motion, so go full
   derivation: the schema file is the representation, the statements the
   derivation).
3. ψ adoption: **expect ZERO sites** (audited) — all 10 primer vocabularies
   are bare-tier, and the complement idioms in the schema are open-relation
   face families ψ cannot compress. Do not force it. Record in the map: a
   future "mintable pins only" law would require reshaping `Pin`/`Outcome`/
   `SteerKind` to payload-tier — a design decision out of this packet's scope.
4. **Statement identity — the real work item.** K4-derived statements are
   minted inside `schema()`; primer holds no object references to them, and
   three consumers key by identity today:
   - `store/diag-map.ts` (~lines 43–51): a `Map<Statement, RepairMapping>`
     built from `laws.X` references that THROWS at load on any unmapped
     statement — re-key by the K4-blessed identity, the `renderStatement`
     string, so all 55 derived statements resolve;
   - `driver/dispatch.ts` (~lines 190–203): `buildStaticHints` walks EVERY
     `runStoreSchema.statements` entry through `diagForStatement` at module
     load — dies at import unless diag-map covers the derived tail;
   - `store/schema-ledger.test.ts` (~lines 75–80): `violation.statement ===
     statement` identity comparisons — move to canonical-string comparison or
     look derived statements up from `runStoreSchema.statements`; four of its
     pinned laws are hand statements this PRD deletes in favor of derivation
     (`candidateVerdictAssumptionRef`, `confusableBRef`, `grpConfusableBRef`,
     `programEdgeDependentEdgeRef`) — re-pin them against the derived copies.
   `schema.test.ts` statement-count/shape assertions follow the 68-written +
   55-derived split and the pinned tail order.
5. Ergonomics adoption in the store cluster (`gates.ts` 30 queries,
   `derive.ts` 9, `observe.ts` 4 — 238 `r.var(` sites) is THIS PRD's to do or
   decline, not P2's: `r.var`/method comparisons remain compiled surface (K5
   adds, deletes nothing), so adoption is style. Where `vars()` is adopted,
   rename variables that collide with primer's imported relation identifiers
   (`program`, `member`, `capsule`, … — the destructured var would SHADOW the
   relation value inside the rule closure); never shadow.

## Passing criteria

- `pnpm exec tsc --noEmit -p tsconfig.json` shows ZERO errors in the store
  cluster's files (the wider repo may be red only from OTHER agents' in-flight
  files — list any such file in the report and prove it's not yours by paths).
- The store cluster's test files pass under the repo's own test invocation.
- Grep the schema file: no `.as(` site remains that the map in step 1
  classified derivable; no `contained(` remains whose exact statement a `ref`
  in the same file derives; every `cites` has the selected/deliberate
  statement it defers to nearby.
- The commit body names the fingerprint motion and the rebuild consequence.
- Commit with `--no-verify` in primer's voice; push the branch.
