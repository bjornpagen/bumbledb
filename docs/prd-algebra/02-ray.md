# PRD 02 — The ray: infinity enters the denotation

**Depends on:** nothing.
**Modules:** `crates/bumbledb/src/interval.rs`, `crates/bumbledb/src/ir/validate/`,
`crates/bumbledb/src/encoding/`, docs.
**Authority:** `10-data-model.md` (the denotation), `30-dependencies.md`
(pointwise judgments).
**Representation move:** Dijkstra's half-open interval, completed — the
sentinel becomes a citizen. `MAX_END` today is a silent hack (review finding
F6: the domain's top value can never satisfy any membership test, undocumented).
Blessing ∞ turns the wart into a law and makes *ongoing employment*, *the top
tax bracket*, and *until-forever recurrence* honest values instead of encoding
tricks.

## Context (decided shape)

The point domain is officially `MIN ..= MAX−1` for each element type;
`end == MAX` **denotes the unbounded ray** `[s, ∞)`. Every judgment and every
Allen relation (PRD 03) is well-defined over rays with zero kernel changes — ∞
compares as MAX naturally, which is why the sentinel worked by accident; now it
works by definition. The zero-cost claim is the encoding, not hope: both element
types store order-preserving **unsigned** words (the i64 sign-flip), so ∞ = MAX
participates in every unsigned comparison kernel with no special case — there is
no branch to take, and PRD 04's kernel needs no ray awareness at all. Half-open
and nonempty are re-recorded with their real
justification: they are Allen's algebra's preconditions (JEPD of the 13 basics
fails over empty intervals; *meets* is only clean half-open), not conventions.

Consequences made explicit rather than left to be discovered:

- A **point literal or element binding equal to MAX is a validation error**
  (typed), never a silently-unmatchable query. Parse, don't validate.
- A ray has no finite measure: `Duration` (PRD 10) over an interval term is
  legal only where the value is bounded, and boundedness is not provable at
  validation — so `Duration` of a ray is a **typed execution error**
  (`MeasureOfRay`), the one runtime type error in the engine, documented as
  such. Alternative (silently yield MAX) rejected: it fabricates arithmetic.
- Coverage judgments over rays: a source ray requires target coverage to ∞ —
  the walk's gap check already handles it (MAX-sentinel end noted sound in
  review); re-state as law.

## Technical direction

1. `Interval::new` unchanged (`start < end` admits `end == MAX`). Add
   `Interval::ray(start)` and `is_ray()` — names for what hosts already do.
2. IR validation: reject element-typed literals/params equal to the domain
   ceiling wherever they meet an interval position (membership, Allen
   operands); new `ValidationError` variant naming the rule.
3. Docs: the point-domain law, the ray denotation, the JEPD justification for
   half-open + nonempty, all in `10-data-model.md`; F6's edge is deleted from
   the "surprising behavior" category by becoming definition.

## Passing criteria

- `[test]` Membership of `MAX−1` in `[s, MAX)` is true; a query binding a point
  literal of MAX is rejected at prepare with the typed error.
- `[test]` Pointwise-key and coverage judgments over rays: two rays on one
  group conflict; a bounded span is covered by a ray; a ray source is not
  covered by bounded targets.
- `[shape]` `10-data-model.md` states the point-domain law and the ray
  denotation; the words "sentinel" and "unusable" no longer describe MAX.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`10-data-model.md` (the denotation section), `30-dependencies.md` (pointwise
lifting examples gain a ray), `40-execution.md` (membership kernel note).
