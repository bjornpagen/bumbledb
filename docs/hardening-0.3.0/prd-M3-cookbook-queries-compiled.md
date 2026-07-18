# PRD-M3 — Cookbook queries compiled: the comments become `query!`

Wave M · Repo: bumbledb (`docs/cookbook.md` + `crates/bumbledb-query/tests/cookbook.rs`)
· depends on: M1 (same doc file — land the spelling sweep first), M2 (queries
must be written in the final notation)

## Objective

The engine cookbook's ~33 recipe queries live as `//` comments inside the
schema blocks of `docs/cookbook.md` — and the doc↔test pin's `normalize()`
(cookbook.rs ~line 720) STRIPS comments, so the queries are the one part of the
cookbook that can rot silently. Every one of them fits today's `query!` grammar
(verified against the module-doc grammar: punning, `field: var`, `==`
lit/handle/?param, `in ?param`, `?t in v`, `Allen(_, MASK, ?p)`, `!atom`,
comparisons, `Sum/Pack/Count/Duration`, named columns, multi-rule union, the
program form). Promote them all to compiled, pinned `query!` blocks.

## Context (verified)

- ~25 recipes carry query comments; only ~15 have compiled twins (the
  `pin()`-style round-trip tests for r01/r03/r06/r07/r09/r14/r15/r17/r18/r19/
  r22/r24/r25/r27/r28), all hand-synced. The two engine-native program forms
  (r24/r25) sit in ```text fences, also unpinned.
- The sync test asserts exactly ONE rust fence per recipe today; markdown
  cannot be `include!`d at item position (the file header says so) — the
  duplicate-and-pin mechanism stays.
- Bare handles (`priority == Urgent`) resolve through the emitted host enum —
  each compiled twin imports it (`use r06::Priority;` — the existing twins
  show the pattern).

## Work

1. **The doc**: in `docs/cookbook.md`, move each recipe's query comment(s) out
   of the schema block into its own ```rust fence immediately after, as
   compiling code: `let q = query!(RecipeSchema { ... });` (program form for
   r24/r25 — promote their ```text fences to rust). Keep the surrounding prose;
   the notation inside is unchanged except M1/M2 respellings.
2. **The sync test** (`crates/bumbledb-query/tests/cookbook.rs`):
   - Teach `doc_blocks()` to classify fences: a recipe now has one SCHEMA fence
     (starts `bumbledb::schema!` — the existing assertion narrows to this
     class) and zero-or-more QUERY fences (start `let ` / `query!` /
     `program`-shaped). Comment-stripping `normalize()` stays for schema
     blocks; query fences are pinned WITHOUT comment-stripping (they are code).
   - Extend `recipe!`/the roster with per-recipe `QUERIES: &[&str]`
     (stringified query source), asserted token-equal to the doc's query
     fences — the same duplicate-and-pin law the schemas obey.
   - Every pinned query — all ~33, not just the current ~15 — goes through the
     existing `pin()` discipline: prepared against a real `Db` built from the
     recipe schema, plus the `ir::render` round-trip golden.
3. Add the ~18 missing compiled twins; import the emitted enums each needs.
4. Roster arithmetic: the counts the test asserts (fences per recipe, total
   recipes) update to the new shape.

## Technical direction

- The queries' MEANING must not change in this PRD — transcribe, don't
  improve. If a comment-query turns out NOT to compile as written (contradicts
  the verified claim), that is a finding: fix the DOC to the working spelling
  and note it in the commit body; never weaken `query!`.
- `normalize()`'s comment-stripping must NOT apply to query fences, or the pin
  is vacuous for them.
- No new macro features; this PRD consumes M1/M2's final grammar.

## Passing criteria

- `grep -n '^\s*//.*|' docs/cookbook.md` (and a manual pass over every recipe)
  finds zero query-shaped comments remaining in schema blocks; every recipe
  that documents a query has it in a ```rust fence.
- The doc↔test pin covers query fences token-for-token: editing a query in the
  doc fails `cargo test -p bumbledb-query`; editing the twin fails it too.
- Every one of the ~33 queries compiles via `query!`, prepares against a real
  store of its recipe's schema, and round-trips through `ir::render` (the
  `pin()` law) — including r24/r25's programs.
- The one-schema-fence-per-recipe assertion is replaced by the classified
  form; total-recipe count stays 29.
- `cargo test -p bumbledb-query` green. Commit in the repo's voice; push.
