# PRD 19 — Guard-probe point lookups over statements

**Depends on:** 06, 12.
**Modules:** `crates/bumbledb/src/api/prepared/` (the access-path dispatch), `crates/bumbledb/src/storage/read/`.
**Authority:** `docs/architecture/40-execution.md` (§ access paths).

## Goal

The read-side fast path — single-atom queries answered by one guard get — is
re-derived from `Functionality` statements, including full-key interval equality,
and correctly refuses membership-bound shapes.

## Technical direction

1. **Eligibility:** a single positive atom, no negated atoms, whose bindings are
   all constants (literals/params — no vars except finds) and either (a) cover
   some key statement's projection **by value** (an interval field counts only if
   bound by an interval-typed term — a membership binding is not a key cover;
   the eligibility check consumes the PRD 12 typing results, never re-infers), or
   (b) bind every field (the `M` full-fact path). Everything else falls through
   to Free Join.
2. **Execution:** derive guard bytes (the shared PRD 06 slicer — statement id from
   the matched key), one `U` get → row_id → one `F` fetch → decode → apply any
   residual bindings/comparisons not consumed by the key (the existing residual-
   check-on-the-fetched-fact logic) → emit finds. `M` path unchanged apart from
   vocabulary.
3. **Per-execution param resolution** carries over (intern miss ⇒ empty result).
   A `ParamSet`-bound field disqualifies the fast path in v0 (k gets would be
   correct but the selection-level path already serves it; comment the decision —
   one sentence, revisit trigger "measured k-get win").
4. Vocabulary: the dispatch code's `unique`-derived naming becomes key/statement
   naming as it is touched.

## Out of scope

New access paths (stabbing accelerators are OPEN).

## Passing criteria

- `[shape]` Eligibility consumes validation's term typing; no second
  membership-vs-equality inference exists in the dispatch.
- `[test]` Point lookup through a scalar key; through a pointwise key with the
  interval bound by value (exact 16-byte guard hit); full-fact `M` lookup with an
  interval field — each answered without an image build (assert via the existing
  no-image-touch instrumentation or cache-state inspection, post-commit cold).
- `[test]` A membership-bound single-atom query does NOT take the fast path
  (executes correctly via scan+filter; assert path taken via EXPLAIN/stats).
- `[test]` Intern-miss param on the fast path yields the empty result without
  error.
