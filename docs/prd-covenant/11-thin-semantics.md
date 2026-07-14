# PRD 11 — The deletion, part one: the semantic chapters become reading guides

**Depends on:** 10 (citations must resolve; the census enforces).
**Modules:** `docs/architecture/10-data-model.md`, `20-query-ir.md`,
`30-dependencies.md`, `docs/cookbook.md` (labels re-cited), the
architecture README.
**Authority:** the zero-duplication law. This PRD is the campaign's
point: the docs stop being a second, drift-capable statement of the
semantics.
**Representation move:** none in code. Knowledge de-duplication — the
aggressive one. Expected size: NET-NEGATIVE by hundreds of lines per
chapter.

## Context (decided shape)

The surviving shape of a semantic chapter (each becomes this, no
more):
1. **The reading guide**: what the chapter's domain is, in prose a
   newcomer reads first — intuition sentences, each ending in a
   theorem citation (`lean/Bumbledb/Dependencies.lean:
   keyed_eq_unique_correspondence`). One intuition sentence per
   concept (the law's MOTIVATE allowance).
2. **The surface grammar** (stays): the schema/query notation is a
   host-surface fact — the grammar blocks, the notation examples, the
   macro conventions remain prose.
3. **The decision records** (stay, whole): refusals, triggers,
   acceptance-boundary rationale ("why exact field sets", "why
   equality-only selections"), the recursion refusal, the census law.
4. **DELETED — moved to Lean and cited**: every denotation display
   (the matching equation block, the containment/coverage set-builder
   displays, the keyed-equality decomposition, the aggregate contract
   bullets, the DNF law, the safety rule's formal statement, the
   answer-identity/union statement, the exact-partition conjunction,
   the interval point-set/ray/measure formalism, fact-identity-as-
   canonical-bytes). The banned forms (law 1): display-math
   denotations, semantic truth tables, "means/denotes/iff/exactly
   when" without a citation.

Per chapter, the executor produces a MOVE LEDGER in this PRD's
Results: each deleted block → the theorem name(s) that now own it.
A block with no owning theorem is a policy-5 stop: either PRDs 02–09
missed a statement (fix THERE first — the deletion never outruns the
formalization) or the block was mechanism/decision content that stays.

Cookbook: recipes' `Guarantee:` labels re-cite from prose table names
to `lean/` theorem names where they exist (the epistemics PRD's
labels, upgraded to resolvable citations — `spec-census.sh` now checks
them).

## Technical direction

Chapter order: 30-dependencies (bridge already took its table), then
20-query-ir, then 10-data-model, then the cookbook label pass. Work
section-by-section: classify (guide / grammar / decision / DELETE),
delete with citation, run `scripts/spec-census.sh` continuously (a
citation typo fails fast). The one-intuition-sentence allowance is a
BUDGET, not a floor — where the theorem name is self-explanatory,
delete outright. Record before/after line counts per chapter.

## Passing criteria

- `[shape]` The banned-forms battery: zero display-math denotation
  blocks in the three chapters; every "denotes/means/iff/exactly
  when" line carries a resolving citation (grep + spec-census).
- `[shape]` The move ledger complete in Results: every deleted block
  has its owning theorem; zero unowned deletions.
- `[shape]` Line-count deltas recorded per chapter (expected: each
  chapter shrinks by ≥40%; if a chapter shrinks less, the Results
  explain what stayed and under which surviving duty).
- `[shape]` Decision records and grammar sections byte-preserved
  except where they cited moved content (diff review, listed).
- `[gate]` `scripts/lean.sh` + `spec-census.sh` exit 0; cookbook
  suite green (labels changed, tests didn't).

## Doc amendments

This PRD IS the amendment; the architecture README's chapter blurbs
update to name the new shape ("reading guide over lean/…").
