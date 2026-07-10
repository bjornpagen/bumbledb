# PRD 03 — Allen: the interval-pair coordinate system

**Depends on:** 02.
**Modules:** `crates/bumbledb/src/ir.rs`, `ir/validate/`, `ir/normalize/`,
`crates/bumbledb/src/interval.rs` (mask type), `plan/selectivity.rs`.
**Authority:** `20-query-ir.md`, `10-data-model.md`.
**Representation move:** choose the coordinates. The 13 Allen basic relations
are jointly exhaustive and pairwise disjoint — every configuration of two
intervals is exactly one of them — so the set of all interval-pair predicates
*is* the powerset 2¹³. One operator parameterized by a 13-bit mask replaces the
operator vocabulary permanently: the vocabulary can never grow again, because
there is nothing outside the coordinate system.

## Context (decided shape)

`CmpOp::Overlaps` and `CmpOp::Contains` (interval⊇interval form) are **deleted,
no aliases**. The replacement:

- `AllenMask(u16)` — a newtype over the low 13 bits; bit *i* = basic relation
  *i* in the **palindromic order** (before, meets, overlaps, starts, during,
  finishes, **equals**, finished-by, contains, started-by, overlapped-by,
  met-by, after): each basic's converse sits at the mirrored position, so
  `converse(mask)` is the 13-bit reversal — one `rbit` plus a shift, scalar or
  vector. The bit order is a **specified representation**, not an
  implementation detail: the algebra's involution costs one instruction because
  the bits are laid out as the algebra's symmetry.
- One IR comparison form for interval pairs: `Allen { mask }` between two
  interval terms of one element type.
- **Named constants, not sugar** (they are values of the algebra): the 13
  singletons under Allen's own names, plus the workload composites —
  `INTERSECTS` (9 bits: point-sets share a point), `COVERS` (equals ∪ contains
  ∪ started-by ∪ finished-by), `COVERED_BY` (its converse), `DISJOINT`
  (before ∪ meets ∪ met-by ∪ after — under half-open, *meets* shares no point).
- Validation rejects the empty mask (write no query) and the full mask (write
  no predicate); both are distinct typed errors naming the vacuity.
- **The mask is paramable**: `Allen { mask: Term }` admits a param of a new
  value shape `Value::AllenMask` — the temporal relation as a bind-time
  argument. Same ∅/full rejection at bind.
- Interval `Eq`/`Ne` survive only as the derived facts they are: `Eq` ≡
  `Allen(EQUALS)`, `Ne` ≡ `Allen(¬EQUALS)`; normalization canonicalizes both to
  masks so exactly one interval-pair form reaches the planner.
- **Point membership is untouched.** Allen is a pair-of-intervals algebra; the
  membership typing rule is a different judgment and stays as is.
- Interval-position anti-probes and residuals carry masks like any predicate.

Constraint-side unification (docs only, no semantics change): the pointwise key
judgment's meaning — per-group pairwise disjointness — is re-stated as "every
pair satisfies `DISJOINT`"; the checker's neighbor probe is its O(log n)
enforcement plan. One vocabulary, both sides of the engine.

## Technical direction

1. `interval.rs`: `AllenMask` with `converse()`, `contains(basic)`, the named
   constants, and `classify(a, b) -> Basic` (the reference implementation —
   total, branch-free-able, property-tested; PRD 04 owns the batch kernel).
2. IR: replace the two deleted ops; validation typing rule (two interval terms,
   one element type, mask non-vacuous); normalization lowers `Allen` to a word
   residual/filter shape carrying the mask (the four endpoint slots + mask).
3. Selectivity: keep-fraction floor as a documented function of mask popcount
   (`popcount/13` clamped to the existing floor ladder) — honest, cheap, and
   replaced by measurement when views are concrete, exactly like every other
   predicate.

## Passing criteria

- `[shape]` `Overlaps`/`Contains`-on-pairs appear nowhere; `grep -r "Overlaps"`
  hits only the Allen basic of that name.
- `[test]` `classify` against a brute-force point-set oracle over randomized
  and boundary pairs (adjacent, nested, equal, rays): the returned basic's
  point-set definition holds and no other does (JEPD property).
- `[test]` `converse(converse(m)) == m`; `classify(a,b)` basic's converse ==
  `classify(b,a)`'s basic, for all pairs.
- `[test]` ∅ and full masks rejected at validation and at bind (param form).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`20-query-ir.md`: the Allen operator replaces the interval-comparison section;
the three-confinement disjunction law is stated here (mask / membership /
rules, cross-referencing PRD 05). `30-dependencies.md`: the pointwise key
re-stated as `DISJOINT`. `10-data-model.md`: mask value shape.
