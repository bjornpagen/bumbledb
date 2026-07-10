# PRD 10 — Measure: `Duration`

**Depends on:** 02.
**Modules:** `crates/bumbledb/src/ir.rs` (the measure term), `ir/validate/`,
`ir/normalize/`, `exec/` (one subtraction feeding existing folds/filters),
`plan/selectivity.rs` (range floors apply).
**Authority:** `10-data-model.md` (the denotation), `20-query-ir.md`.
**Representation move:** the one arithmetic the denotation defines. An interval
denotes a point set; a point set has a measure; `|[s,e)| = e − s`. Everything
else that looks like interval arithmetic is endpoint math and stays refused —
`Duration` is not the thin end of a wedge, it is the entire wedge, provably:
the denotation defines nothing else.

## Context (decided shape)

- `Duration(t)` where `t` is an interval-typed term, yielding u64 (the measure
  is nonnegative for both element types; for i64 intervals the measure of the
  order-preserving encoded words equals the true difference — one subtraction,
  exact, no overflow: encoded end > encoded start by the constructor
  invariant).
- Legal positions: a find term; the aggregated input of `Sum`/`Min`/`Max`; one
  side of an order comparison against a u64 term or literal ("meetings longer
  than an hour").
- **Rays have no finite measure** (PRD 02's law): `Duration` of a ray is the
  typed execution error `MeasureOfRay` — the engine's one runtime type error,
  documented as such; hosts exclude rays with `Allen` predicates first.
- `Sum(Duration(...))` accumulates in i128 with the single finalize range
  check, like every `Sum`.
- Selectivity: a `Duration` comparison is a range predicate; the existing
  range keep-fraction floor applies unmodified.

## Technical direction

1. IR: `Term`-level measure wrapper or a `FindTerm`/comparison-side form —
   choose the smallest shape that keeps `Term` plain data (decision recorded
   in the PRD on landing).
2. Normalization lowers to a two-slot read + subtraction feeding the existing
   word machinery; the executor gains one gather+subtract shape (dense case
   NEON per the port-topology law — subtraction is not flag-bound; strided
   stays scalar until measured, per the standing rule).
3. Ray check: the subtraction path tests `end == MAX` on the element domain
   and raises the typed error with the offending fact's interval words.

## Passing criteria

- `[test]` `Duration` in finds, in `Sum`/`Min`/`Max`, and in comparisons —
  differential vs the naive model, both element types, boundary intervals
  (`[x, x+1)`, `[MIN, MAX−1)`).
- `[test]` A ray reaching `Duration` raises `MeasureOfRay`; the same query
  with a `DISJOINT`-from-rays guard succeeds.
- `[test]` `Sum(Duration)` overflow at the i128→u64 boundary is the existing
  typed overflow error.
- `[shape]` No other interval arithmetic exists (grep for endpoint reads
  outside the kernel/normalize modules finds nothing new).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`20-query-ir.md`: the measure term, its positions, the ray error.
`10-data-model.md`: one sentence — the denotation defines exactly one
arithmetic, and this is it.
