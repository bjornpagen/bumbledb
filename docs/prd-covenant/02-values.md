# PRD 02 — Values: the universe, the encodings, the intervals

**Depends on:** 01.
**Modules:** `lean/Bumbledb/Values.lean`, `Countermodels.lean` (its
first residents).
**Authority:** `10-data-model.md`'s semantic content (this PRD builds
its replacement); the audited artifact's interval/vacuity results; the
engine's exhaustive encoding suites (the theorems here are what those
suites sample — state them fully).
**Representation move:** Level 0 for values. After PRD 11, the ONLY
normative statement of what a bumbledb value is.

## Context (decided shape) — required definitions and theorems

Definitions (names indicative; executor keeps them law-compliant):
- `ValueType` — the six structural types (Bool, U64, I64, Str as
  intern identity, FixedBytes n, Interval of element type); `Value`
  the dependent sum. Str is modeled as an opaque intern id with
  equality only (NO order — the order-refusal is a typing fact, not a
  missing feature; state it in the module doc).
- `Interval α` for `α ∈ {U64, I64}` element domains: a structure with
  `start end : α` and the invariant `start < end` carried as a field
  (`h : start < end`) — nonemptiness by construction, mirroring the
  Rust `Interval`. `MAX_END` as the domain ceiling; `isRay iv ↔ iv.end
  = MAX_END`; the point domain is `[MIN, MAX_END)`.
- `points : Interval α → Set α` — the half-open denotation.
- `measure : Interval α → Option Nat` — `none` on rays (the
  MeasureOfRay law), `some (end − start)` otherwise.
- `encode : Value → Word*` at the ABSTRACT level: not bytes — an
  order-embedding claim. Model each scalar encoding as a function into
  a linearly ordered word domain.

Theorems (the module's spine; each name lands in Bridge):
1. `interval_nonempty : ∀ iv, (points iv).Nonempty` — the premise the
   Rust constructor discharges (Bridge row: `crate::Interval::new`).
2. `points_halfopen : x ∈ points iv ↔ iv.start ≤ x ∧ x < iv.end`.
3. `ray_is_unbounded_tail : isRay iv → (x ∈ points iv ↔ iv.start ≤ x)`
   over the point domain — "∞ is a value of the representation" made a
   theorem.
4. `measure_ray_none : isRay iv → measure iv = none`;
   `measure_finite : ¬isRay iv → measure iv = some (iv.end − iv.start)`.
5. `encode_i64_order_embedding : a ≤ b ↔ encodeI64 a ≤ encodeI64 b` —
   the sign-flip law, stated over the abstract embedding (Bridge row:
   `encoding/encode.rs::encode_i64`, sampled exhaustively by the
   engine's order suite).
6. `encode_interval_order` — the two-half encoding preserves the
   (start, end) lexicographic order used by the determinant walks.
7. `value_eq_iff_encode_eq` — canonical-bytes identity: the fact-
   identity law, abstractly.
Countermodels (ported/adapted from the artifact):
- `empty_interval_vacuous` — an empty point set satisfies any coverage
  obligation vacuously (re-stated against the in-tree `points`; the
  artifact's `empty_nat_interval_has_no_points` shape). Lives in
  `Countermodels.lean` with the note: unrepresentable in-tree because
  `Interval` carries `h`, which is the POINT.
- The str-order refusal note: no `LinearOrder` instance exists for the
  intern domain — a deliberate absence, documented.

## Technical direction

Finite-set machinery: prefer functions-into-`Prop` (`Set α`) for
denotations and `List`-based finite enumerations only where
decidability is needed (PRD 13 will want computable forms — write
`points` as `Set`, and add `mem_decide : Decidable (x ∈ points iv)`
via the boundary comparisons now, cheaply). No mathlib: `Set`,
`Decidable`, and order type-classes from core suffice. Where the
artifact proved a statement over `Nat` intervals, generalize only as
far as the two real element domains need — over-generality is a law-5
smell.

## Passing criteria

- `[shape]` All seven theorems + both countermodel entries present,
  named, and checked; `scripts/lean.sh` exit 0; zero sorry/axioms.
- `[shape]` `Interval` carries the invariant as a field (grep the
  structure); no constructor bypasses it.
- `[shape]` Module docs state the two deliberate absences (str order,
  empty intervals) with their reasons.
- `[gate]` CI lean lane green.

## Doc amendments

None yet — PRD 11 does the deletion against this module's names.
