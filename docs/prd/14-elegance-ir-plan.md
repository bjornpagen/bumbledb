# PRD 14 — Elegance: IR and plan

**Depends on:** 13.
**Binding constraints:** the README's elegance-pass block.
**Modules:** `crates/bumbledb/src/ir.rs` + `ir/` (validate, normalize),
`crates/bumbledb/src/plan.rs` + `plan/` (planner, fj, selectivity, chase,
provably_distinct).

## Subsystem-specific hunt list (verify, don't assume)

- **The typing results' journey:** validation infers term typings (bivalent
  anchors, membership vs equality), normalization lowers on them, the planner
  and the point-lookup classifier consume them. Check for re-derivation:
  PRD 19 (rebuild) required the classifier to consume validation's results, but
  normalization/plan may still carry parallel typing maps — one representation
  should travel the whole pipeline.
- **Filter/residual vocabularies:** the per-atom filter kinds (PointIn,
  AnyPointIn, the three interval shapes, FieldsCompare, ranges) and the
  residual word-comparisons were built in two PRDs — check for near-duplicate
  evaluation dispatch (the view evaluator vs the batch residual evaluator both
  match on shapes; the *shapes* should be one enum consumed twice, not two
  enums).
- **Attachment logic:** `earliest_bound_node` serves residuals, word residuals,
  and anti-probes (and had a bug found by PRD 16 of the rebuild) — confirm it
  is one function with one test module, and that the chase's occurrence
  re-indexing (PRD 08 of this set) composes with it rather than duplicating
  index bookkeeping.
- **The witness's width bookkeeping:** slot widths, key widths, ColumnSpans —
  three width maps flowed in from different PRDs. Check whether they are
  derivable from one source at witness construction; if two are projections of
  the third, derive them and delete the stored copies.
- **`chase.rs` freshness:** built last (PRD 08 of this set) against the code
  as-it-was — after 12/13's normalizations, re-read it for idiom fit.
- **Validation roster tests:** the reject corpus grew per-PRD; converge fixture
  styles and kill duplicate coverage (same rejection asserted from two eras'
  tests — merge and redirect).

## Passing criteria

As PRD 12's, applied to this subsystem. Additionally:
- `[shape]` One filter-shape enum consumed by both evaluators, or the findings
  list justifies why two remain.
- `[shape]` `earliest_bound_node` remains single-definition with its regression
  test intact.
- `[gate]` Workspace gates green.
