# PRD 11 — IR shape

**Depends on:** 01 (Interval values). Independent of phase B.
**Modules:** `crates/bumbledb/src/ir.rs`.
**Authority:** `docs/architecture/20-query-ir.md` (§ IR shape — normative, verbatim).

## Goal

The IR data types match `20-query-ir.md`'s normative block exactly. This PRD is
pure data-shape: no validation, no lowering, no execution.

## Technical direction

1. Apply the normative block literally:
   - `Query` gains `negated: Vec<Atom>` (positive list keeps the name `atoms`).
   - `Term` gains `ParamSet(ParamId)`.
   - `Value` gains `IntervalU64(u64, u64)` and `IntervalI64(i64, i64)`. Constructors
     stay dumb data (validation rejects `start ≥ end` — PRD 12); but add
     `From<Interval<u64>>` / `From<Interval<i64>>` impls so hosts construct
     literals through the checked type.
   - `CmpOp` gains `Overlaps` and `Contains`.
   - `AggOp` becomes `Sum | Min | Max | Count | CountDistinct | ArgMax { key: VarId } | ArgMin { key: VarId }`.
     `FindTerm::Aggregate { op, over }` is unchanged in shape; for Arg ops, `over`
     is the carried variable (`Some`), for `Count` it stays `None`, for
     `CountDistinct` it is `Some(counted var)`.
2. **Membership is not a node**: no new binding kind exists. A `(FieldId, Term)`
   binding where the field is `Interval(E)` and the term's type is `E` *means*
   membership — this is a typing rule owned by validation (PRD 12) and lowering
   (PRD 13). Add the rule as a doc comment on `Atom::bindings` citing
   `20-query-ir.md`.
3. **Negation is a position, not a kind**: `negated` reuses `Atom` unchanged. Doc
   comment on the field: safety rule + "binds nothing, only rejects".
4. Keep the representation notes intact: no wildcard variant, dense `VarId`s, one
   `Value` variant per data-model type, no universal integer.
5. Debug rendering: extend whatever `Debug`/display helpers exist to the new
   variants; no bespoke pretty-printer here (statement rendering is PRD 20).

## Out of scope

Validation (12), normalization (13), planning (15), execution (16–18).

## Passing criteria

- `[shape]` `ir.rs`'s type definitions match the normative block in
  `20-query-ir.md` field-for-field and variant-for-variant (a reviewer diffing the
  two must find zero semantic divergence; naming may follow Rust conventions).
- `[shape]` No `Membership` / `In` / `PointBinding` node kind exists anywhere.
- `[shape]` Every exhaustive match over `Term`, `Value`, `CmpOp`, `AggOp` in the
  crate was extended without adding `_ =>` arms.
- `[test]` `From<Interval<i64>> for Value` produces `Value::IntervalI64` with the
  same halves.
