# PRD 11 — Handles everywhere: render, notation, cookbook

**Depends on:** 05 (the post-enum surface), 07 (folds to report); lands last.
**Modules:** `ir/render` (handle printing), `api/stats.rs` (EXPLAIN fold
lines), `docs/cookbook.md` + its compile-test module, repo `README.md`,
`docs/architecture/` residue sweep.
**Authority:** PRD-algebra 20 (the renderer is the notation's spec) and 23
(the query notation), rule 5.
**Representation move:** the surface catches up with the theory. Handles are
the vocabulary's names; every place a row id of a closed relation appears —
rendered queries, EXPLAIN, errors, the cookbook — prints the handle, not the
number, so the one notation stays human end to end.

## Context (decided shape)

1. **`ir::render` prints handles.** The renderer already takes the schema;
   when a literal/param-resolved word sits in a position whose field is a
   closed-relation reference (or the closed relation's own id), print the
   handle (`kind == DirectPass`, `severity in {Warning, Critical}`), falling
   back to the number only for out-of-range words (which render as
   `KindId(7?)` — visibly wrong, since rendering hides nothing). Membership
   sets over closed refs render as handle sets.
2. **EXPLAIN's fold lines** (PRD 07) render through the same path:
   `folded: Kind{mastered == true} → {DirectPass, JudgedPass, Recovered}`.
3. **Round-trip discipline extends**: the query notation accepts handles as
   literals (PRD-algebra 23's grammar already admits them as id-constant
   values through the macro's resolution); the render goldens now include a
   closed-reference query, keeping parser/renderer welded.
4. **The cookbook** gains three recipes and loses one: ADD *the vocabulary*
   (tier-1 closed relation replacing the enum idiom in every existing recipe
   that used one — the mechanical rewrite from PRD 05's rule), *the
   classification* (the fused Kind/mastered form, replacing the seeded
   classification-relation recipe outright), and *the sub-vocabulary* (the
   ψ-selected containment, `Escalation(severity) <= Severity(id | pages ==
   true)`); DELETE the seeded classification recipe (its lesson is now a
   grammar feature). The compile-test roster count updates accordingly.
5. **Repo README**: the theory-grammar section's type table goes to six
   rows + the closed-relation production with a tier-2 example; the
   staging-law summary paragraph (three sentences, pointing at
   `40-execution.md` for the ladder) joins "Why it's fast."
6. **Architecture residue sweep**: grep the eight chapters for enum
   residue PRD 05's per-chapter amendments may have missed (`variant`,
   `ordinal`, `enum` outside host-emission contexts) and fix in place —
   this PRD is the set's rule-5 backstop, not a substitute for the
   per-PRD amendments.

## Technical direction

1. Render: the field→closed-relation resolution is a schema walk done once
   at renderer construction (a `FieldId → Option<RelationId>` table from
   declared containments whose target is closed and whose source projection
   is that single field — the same inference the manifest uses); handle
   lookup is extension indexing.
2. Goldens: extend PRD-algebra 20's byte-exact render goldens with the
   closed-reference query and one fold line; extend the cookbook
   compile-test with the new recipes (the counting assertion moves to the
   new roster total).
3. The sweep: mechanical, grep-driven, with the found-and-fixed list in the
   commit body.

## Passing criteria

- `[test]` Render goldens: handle literals, handle sets, the fold line, and
  the out-of-range fallback each render byte-exactly.
- `[test]` The cookbook compile-test passes at the new roster count; the
  sub-vocabulary recipe's schema validates and its violating insert aborts
  (the recipe doubles as the PRD-04 worked example, cross-referenced).
- `[shape]` `grep -rn "seeded\|seeding" docs/cookbook.md` returns nothing;
  the classification recipe is the fused form; the repo README table has
  six type rows + the closed production.
- `[shape]` The architecture sweep's residue list is in the commit body and
  the greps it names return clean.
- `[gate]` Workspace gates green — and this PRD closing means the full gate
  suite (`fmt`/`clippy`/`test`/`check.sh`) must be green across the whole
  workspace, the set's terminal condition.

## Doc amendments (rule 5)

This PRD is largely amendments; the load-bearing ones: `40-execution.md`
gains the staging-law ladder as a named section (the seven stages, the
boundary clause "folding produces data, never code," the pins-acknowledge-
never-refixing-inputs note) — the law the whole set implements, written
down where the executor's doctrine lives.
