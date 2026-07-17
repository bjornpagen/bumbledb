# PRD-S5 — The SDK cookbook

Wave 1 · Repo: bumbledb `ts/` · depends on: S4 (the whole surface must be real) · blocks: —

## Objective

Ship the SDK's own translated cookbook: the 29 recipes rendered in the structural
API, **rot-proofed by a compile test** the way the engine's cookbook is pinned by
`crates/bumbledb-query/tests/cookbook.rs`, and wired to travel with the published
package. A recipe that stops compiling against the SDK fails the build — the
cookbook can never drift from the surface.

## Scope (files)

`ts/COOKBOOK.md` (the prose + code, the npm/docs artifact), `ts/test/cookbook.test.ts`
(the compile-and-run pin), and a one-line link from `ts/README.md`. Do not edit the
SDK source (S1–S4 own it); if a recipe cannot be expressed, that is a finding
against S1–S4, reported, not worked around.

## Context

- The engine cookbook is `docs/cookbook.md` — 29 recipes, each a worked schema
  (some with query snippets), each carrying a `Guarantee:` label. The authoritative
  translations were drafted in the design session and approved; this PRD lands them
  as real, compiling TS.
- The engine's rot-proofing: `cookbook.rs` duplicates each block token-for-token
  and a sync test pins the duplication. The SDK analog is stronger and simpler:
  the recipes ARE real TS in `cookbook.test.ts` (or imported by it), so they
  compile and their schemas `Db.create`/fingerprint as part of the test run.

## Work

1. **Author `ts/COOKBOOK.md`**: the 29 recipes in the structural API (the approved
   translations), each with its one-line guarantee and its TS block, grouped as the
   engine cookbook groups them (Foundations, Vocabularies, Structure, Time and
   coverage, The write side, Host-driven closure, Operating the store, Composition).
   Faithful to the ratified surface: `.as("Domain")`, `on(R, "x" | ["a","b"])`,
   free-function statements, the `query(S).rule(r => …)`/`program` query shape,
   bare values.
2. **Land `ts/test/cookbook.test.ts`**: every recipe's schema is constructed and
   `schema()`-validated (and, where a store is cheap, `Db.create`d on an ephemeral
   store and its fingerprint asserted stable); every query snippet is
   `db.prepare`-lowered (accepted by the engine). This makes the cookbook
   executable documentation — the compile IS the pin. Recipes that assert a
   guarantee an engine test already owns (pointwise disjointness, keyed `==`, etc.)
   need only construct-and-lower here — do NOT duplicate engine semantics tests
   (that would be a test-only PRD, forbidden); the pin is that the SURFACE
   expresses the recipe and the engine accepts its lowering.
3. **Wire it to travel**: link `ts/COOKBOOK.md` from `ts/README.md`; ensure the
   package `files` list ships it (or it lives in the repo docs and the README links
   the GitHub copy — pick the spelling that puts it on the npm page's reach).
4. **Provenance parity**: where a recipe's guarantee cites a Lean theorem, keep the
   citation (the SDK cookbook points at the same `lean/` names the engine cookbook
   does) — the SDK is the same theory in another skin.

## Technical direction

- The cookbook is illustrative, never normative (same status as the engine's) — the
  architecture chapters and the Lean spec win on any disagreement.
- Keep the recipes tight: the schema block + the query snippets, minimal prose
  (the guarantee line carries the "why"). This is a reference, not a tutorial.
- If any recipe forces a cast or an awkward spelling, that is a defect in S1–S4's
  surface, not the cookbook's problem — report it as a finding for the Gate/Review
  phase; do not paper over it in the recipe.

## Passing criteria

- `ts/COOKBOOK.md` contains all 29 recipes in the structural API, grouped and
  guarantee-labeled.
- `ts/test/cookbook.test.ts` compiles and passes: every recipe schema validates,
  every query snippet lowers and is accepted by `db.prepare`; the test is part of
  `node --test $(find test -name '*.test.ts')`.
- `pnpm exec tsc --noEmit` green (the cookbook TS is cast-free like the rest);
  `pnpm exec biome check .` clean.
- `ts/README.md` links the cookbook and the package ships it.
- Zero casts across the recipes (they are the acid test of the surface's elegance —
  a recipe needing `as`/`!` is a Gate/Review finding).
- Commit deferred to the Land phase.
