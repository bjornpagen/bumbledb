# PRD 07 — The recursion design: a paper proof with a seam ledger

**Depends on:** 04–06 (the design is written against the predicate-era
shapes and cross-references the recorded refusal).
**Modules:** one new document, `docs/reference/recursion-design.md`
(reference/, not architecture/ — it specifies an UNBUILT feature and must
never masquerade as the record of a built one; the architecture chapters
describe what is).
**Authority:** the refusal recorded in PRD 06 (this paper is its
execution plan, pre-paid); the no-speculative-structure refusals in the
set README (this paper is where those cuts live INSTEAD of in code).
**Representation move:** "prepared for recursion" becomes a checkable
claim: every future diff is mapped onto a named, existing seam with an
estimated blast radius. Zero code.

## Context (decided shape) — the paper's required sections

1. **The IR cut.** `Program { predicates: Vec<PredicateDef>, output:
   PredId }` as a wrapper whose degenerate form is today's `Query`;
   `PredicateDef = { predicate: Predicate, rules }` — PRD 04's type IS
   the IDB typing rule, verbatim. Atom sources: `Edb(RelationId) |
   Idb(PredId)` — the deferred one-line sum with its consumer list
   (validation typing, normalize, chase, view binding) and estimated
   diff per consumer.
2. **Stratification.** The predicate dependency graph; SCC condensation;
   mutual recursion within a stratum; negation/aggregation THROUGH a
   cycle refused with typed errors (name them); negation/aggregation OF
   lower strata allowed; measures in recursive heads refused (the
   error-timing ruling). The safety theorem: set semantics + heads
   projecting bound vars only + finite domains ⇒ termination.
3. **The delta rewrite.** Per recursive rule, k plan variants (one per
   recursive atom as the delta); prepared once on the selectivity
   ladder's floors (the param-plan precedent); the `ResolvableFilter`/
   `ClassifiedComparison` pattern as the shape for typed variants.
4. **Transient images.** Per-iteration delta images built from tuple
   buffers on the `synthesize_closed` precedent; NEVER memoized — the
   view memo's generation axiom ("a view is valid for its whole
   generation") survives untouched because delta images live entirely
   outside it. State this as the design's load-bearing invariant-
   preservation argument.
5. **The driver.** Per-stratum semi-naive loop over the existing
   run-rule machinery; the frontier IS the sink's seen-set with a
   per-round watermark (the one future hook, one method); the
   iteration/tuple budget with its typed error (the one new trust
   boundary).
6. **The oracles.** Naive model: the ten-line naive fixpoint. SQLite:
   linear recursion via `WITH RECURSIVE`; non-linear naive-only (the
   ψ-subset division-of-labor precedent). Generator: the recursive-shape
   arm. The shipping law restated: the oracle lands before the
   evaluator.
7. **The notation.** Named heads (`path(x, z) | edge(x, y), path(y, z);`)
   with bare clauses remaining the output predicate — text-level
   backward compatible; renderer round-trip implications.
8. **The research item, honestly fenced.** Chain-window computation
   (interval intersection along paths) requires value creation in heads
   (`max/min` over endpoint lattices) — beyond this design; recorded as
   the open theory question with its termination argument sketch.
9. **The seam ledger** (the deliverable's spine): a table — every
   section's cut × the seam it lands on (post-crucible shape) × files ×
   estimated diff size × the invariant it must preserve. The claim the
   ledger proves: the campaign after the trigger fires is a 6–8 PRD set,
   not a comptime-sized one.

## Technical direction

Write it in house voice with the architecture chapters' citation
discipline (every claim about existing machinery cites its chapter/
module by mechanism name). Where the paper disagrees with a chapter,
policy 5 applies — that is half the point of writing it. Cross-reference
PRD 06's refusal (each trigger clause → the ledger rows it activates).

## Passing criteria

- `[shape]` The document exists at `docs/reference/recursion-design.md`
  with all nine sections and the ledger table complete (no TBD cells);
  its header states it specifies an unbuilt feature and is subordinate
  to the architecture chapters.
- `[shape]` Zero code changes in this PRD (`git diff --stat` shows the
  one file).
- `[shape]` `20-query-ir.md`'s refusal record gains the pointer to the
  paper (one line).
- `[gate]` N/A beyond drift protection.

## Doc amendments (rule 5)

The pointer line only — the paper itself is reference material, not the
record.
