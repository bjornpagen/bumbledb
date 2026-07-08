# PRD 12 — IR validation roster

**Depends on:** 11, 03 (schema statement indices for the elision proof come later; only types needed here).
**Modules:** `crates/bumbledb/src/ir/validate/` (all files), `crates/bumbledb/src/error.rs` (validation error enum).
**Authority:** `docs/architecture/20-query-ir.md` (§ validation boundary — the roster is exhaustive; § comparison rules; § params; § param sets; § aggregation; § negation).

## Goal

The validation boundary implements the new roster completely. One pass, one
`ValidatedQuery` witness, no downstream re-checks.

## Technical direction

1. **Typing of terms against interval fields** (the membership rule): during
   variable/param type inference, a binding `(field: Interval(E), term)` anchors
   the term as *either* `Interval(E)` (interval-typed term ⇒ value equality) or
   `E` (element-typed term ⇒ membership). Resolution: a `Var`'s type is unified
   across all its occurrences — if any occurrence anchors it to a scalar field of
   type `E`, the interval binding is membership; if all its anchors are interval
   fields, it is interval-typed (value equality) *unless* that leaves it
   membership-only... which is exactly the roster line: **a point variable bound
   only by membership bindings is invalid**. Implement as: infer with interval
   bindings contributing a *bivalent* anchor `{Interval(E) | E}`; monovalent
   anchors (scalar fields, typed comparisons, literals) collapse it; a variable
   whose anchors are all bivalent resolves to `Interval(E)` (value equality) —
   and then the membership-only rejection can never fire for vars, only for
   *element-typed* vars whose sole anchors are memberships via comparisons; keep
   the roster's diagnostic for the case where a var is element-typed (collapsed by
   a comparison against an element-typed term) but bound in atoms only through
   interval fields. Write this resolution order as a comment block; it is the one
   subtle rule in this PRD.
   Literals and params in interval-field positions: `Value::IntervalX` ⇒ equality;
   element-typed value ⇒ membership. `ParamSet` in an interval-field position
   anchors to the element type (point sets; interval-set params are not a thing —
   reject an interval-typed ParamSet anchor with its own diagnostic).
2. **Comparison rules:** `Eq`/`Ne` all seven types, same-type both sides;
   `Lt/Le/Gt/Ge` U64/I64 only — an interval operand under an order op gets the
   dedicated diagnostic named in the roster ("order operators on intervals" — the
   predictable mistake gets the good error); `Overlaps` two intervals same
   element; `Contains` interval×(same-element interval | element). Constant
   comparisons and self-comparisons rejected as today.
3. **Param sets:** a `ParamId` used as both scalar and set ⇒ error; `ParamSet`
   legal only in atom bindings (positive or negated) and as one side of `Eq`;
   under any other op ⇒ error. Dense param ids across scalars and sets jointly.
4. **Negation:** every variable occurring in a `negated` atom must occur in some
   positive atom; find variables must come from positive atoms; a query with no
   positive atoms ⇒ error (zero-binding negated atoms legal). Negated occurrences
   count toward the occurrence cap.
5. **Aggregates:** existing rules carry over; add — `CountDistinct` legal over
   every type; Arg terms: all share one key var and one direction, key must be
   U64/I64, `over` is the carry (may equal the key), Arg terms and fold aggregates
   may not mix; aggregate-over-group-key unchanged.
6. **Interval literals:** `Value::IntervalX(s, e)` with `s ≥ e` rejected here
   (bindings, comparisons, and — via PRD 03 — selections each diagnose their own
   position).
7. Every roster line in `20-query-ir.md` maps to exactly one error variant; keep
   the existing precise-diagnosis style (enum ordinal errors distinguish binding
   vs comparison sites, etc.).

## Out of scope

Normalization (13). Schema-statement validation (03).

## Passing criteria

- `[shape]` One error variant per roster line; no catch-all.
- `[shape]` The bivalent-anchor resolution is implemented once with the comment
  block; no second inference pass exists.
- `[test]` A reject-corpus with at least one case per roster line added by this
  PRD: order-op-on-interval, ParamSet under Ne, ParamSet as scalar too,
  membership-only variable, negated-atom unbound var, no positive atoms, mixed
  Arg+Sum, differing Arg keys, non-orderable Arg key, inverted interval literal,
  interval-typed ParamSet anchor.
- `[test]` Accept cases pinning the typing rule: (a) var membership-bound in one
  atom and scalar-bound in another (element type); (b) var bound in two atoms'
  interval fields (interval type, value-equality join); (c) literal element value
  in an interval field position (membership filter); (d) `Overlaps` between two
  interval vars from different atoms.
