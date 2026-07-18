# PRD-P1 — Primer: the store schema goes law-typed

Wave P · Repo: **primer** (`/Users/bjorn/Documents/primer`) · depends on: K8
(the final SDK surface, locally linked) · hard break · NOTE: option 2 made
this PRD radically smaller than its earlier drafts — read the consequences.

## Objective

Cut primer's graph-builder store schema
(`src/tools/graph-builder/store/schema.ts` — audited 2026-07-18: 22 `.as(`
sites, 57 `contained(`, 31 keys, 3 `mirrors`, 32 windows, 10 bare-tier closed
vocabularies) to the option-2 surface: relation declarations go pure
structure, EVERY statement stays exactly where it is, and `schema()` law-types
the columns.

## The option-2 consequences (why this PRD shrank)

- **No statement changes at all.** Statements are sacred and now do the
  typing; the 57 containments, 3 mirrors, 31 keys, 32 windows stay
  line-for-line. The earlier drafts' plan to delete 55 derivable containments
  is DEAD — deletion would now UN-TYPE the columns.
- **Fingerprint provably unchanged.** The only edits are `.as` deletions —
  labels lower to the wire `newtype`, which the engine drops before hashing;
  names, structural types, generation flags, and statements are untouched.
  Assert it: fingerprint the schema before and after in a scratch check and
  put the equal hex in the commit body. No store rebuild, no migration note.
- **Statement identity never moves** — nothing is synthesized, so
  `store/diag-map.ts`'s identity-keyed `Map`, `driver/dispatch.ts`'s
  `buildStaticHints` walk, and `schema-ledger.test.ts`'s `===` comparisons
  all keep working UNTOUCHED. The earlier re-key work item is dissolved; do
  NOT touch those files.

## Work

1. Delete the 22 domain-label constants and every `.as(` call in
   `schema.ts`; fields become bare structural descriptors (`u64.fresh`,
   `u64`, `interval(i64)`, …). The class map now computes: fresh generators
   name their classes (`"sheet.id"`-style coordinates), the containments
   propagate them, `capability: str` becomes class-typed by its own composite
   containments (the audit's wished-for custody wall — verify it landed:
   the class map on the schema value shows the capability slots sharing one
   class), and `task.subject` stays bare (in no law — verify it shows
   `undefined` in the class map).
2. Verify the one-generator wall passes over the whole schema (it should —
   the statements were already coherent; if it FIRES, that is a real found
   bug in primer's theory: report it, do not "fix" the schema silently).
3. `store/schema.test.ts` / `rebirth.test.ts`: update only what the kernel
   break forces — `.as` mentions and any type-level assertions on descriptor
   domains move to class-map assertions. Statement-count/order assertions are
   UNCHANGED (nothing moved).
4. Ergonomics adoption in the store cluster (`gates.ts` 30 queries,
   `derive.ts` 9, `observe.ts` 4 — 238 `r.var(` sites) is THIS PRD's to do
   or decline, not P2's: `r.var`/method comparisons remain compiled surface
   (K5 adds, deletes nothing), so adoption is style. Where `vars()` is
   adopted, rename variables that collide with primer's imported relation
   identifiers (`program`, `member`, `capsule`, … — the destructured var
   would SHADOW the relation value inside the rule closure); never shadow.
5. Query domain-typing now flows from the class map — re-run the cluster's
   tests; any query that previously joined via matching HAND labels across
   fields that NO law connects will now refuse (bare-pairs-bare or
   cross-class). Each such refusal is a finding: either the theory was
   missing a law (report it — statements are the owner's) or the query was
   wrong (fix it, say so).

## Passing criteria

- Zero `.as(` in primer (grep repo-wide; the two `rebirth.test.ts`
  mini-schemas are P2's, coordinate with it).
- The before/after fingerprint hexes are EQUAL (in the commit body).
- `diag-map.ts`, `dispatch.ts`, `schema-ledger.test.ts` untouched
  (`git diff` proves it).
- The class-map spot checks pass: capability slots share a class;
  `task.subject` is bare; every fresh id names its own class.
- `pnpm exec tsc --noEmit -p tsconfig.json`: zero errors in the store
  cluster's files (foreign in-flight files listed with proof of
  non-ownership); the cluster's tests pass under the repo's own invocation.
- Commit with `--no-verify` in primer's voice; push the branch.
