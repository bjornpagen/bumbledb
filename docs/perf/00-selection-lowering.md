# PRD 00 — Selection lowering: Eq-constant predicates become plan data

Authority: `docs/architecture/20-query-ir.md` (lowering), `30-execution.md`
(views and filters), the suite README finding 1.

## Purpose

Give equality-against-a-constant its own name in the plan. Today an
`account = ?0` and an `at >= ?0` are the same thing — a `FilterPredicate` in
`PlanOccurrence::filters`, destined for a scan. After this PRD they are
different things at the type level: **selections** (probeable) and **residual
filters** (scannable). This PRD is representation only; execution behavior is
bit-identical via a shim that PRD 02 deletes.

## Technical direction

- New type in `plan/fj.rs` (or a sibling module it re-exports):

  ```rust
  /// One probeable equality: `field == value`, value constant per
  /// execution (literal word/byte, param slot, or pending intern).
  pub struct Selection {
      pub field: FieldId,
      pub value: crate::image::view::Const,
  }
  ```

- `PlanOccurrence` gains `pub selections: Vec<Selection>`; its `filters` keeps
  **only** non-selection predicates. The split rule, exact:
  - `FilterPredicate::Compare { op: CmpOp::Eq, field, value }` →
    `Selection { field, value }`. Every `Const` variant qualifies (`Word`,
    `Byte`, `Param`, `PendingIntern`) — literals and params are the same
    machine.
  - Everything else stays a filter: `Compare` with `Ne/Lt/Le/Gt/Ge`, and all
    `FieldsCompare` (including `Eq` — a repeated in-atom variable is a
    row-shape constraint, not a constant probe).
  - Two selections on the same field are legal (contradictory Eqs ⇒ empty; the
    probe path handles it naturally — do not special-case).
- The split happens where filters are lowered into `PlanOccurrence` today
  (follow the construction of `filters` through `binary2fj`/`factor` in
  `plan/fj.rs`); selections are ordered by `FieldId` (deterministic plans —
  the plan is `PartialEq`-compared in tests and its Debug feeds
  `families::digest()` indirectly; determinism is load-bearing).
- Plan validation (`plan/fj.rs::validate`) extends its invariants: a selection's
  field must belong to the occurrence's relation and must not also appear in
  `filters` as an Eq compare (unrepresentable now, assert anyway at the
  boundary since `FjPlan` is plain data anyone can construct — mirror the
  existing `PlanError` style with a new variant, e.g.
  `SelectionOnFilteredField { occ }` for a field appearing in both lists with
  `Eq`).
- **The shim (deleted by PRD 02):** one function next to `run_join` in
  `api/prepared.rs`,

  ```rust
  /// PRD 00 transitional shim — PRD 02 deletes this: selections rejoin
  /// the filter list so views behave exactly as before the split.
  fn selections_as_filters(occ: &PlanOccurrence, out: &mut Vec<FilterPredicate>)
  ```

  invoked wherever `occurrence.filters` feeds `resolve_filters`/`apply`, so the
  resolved filter vectors are byte-identical to pre-split. Mark it with a
  comment naming `docs/perf/02-execution-cutover.md`.

## Non-goals

Any behavior change (this PRD's diff must not move a single benchmark number).
Probing (PRD 01/02). Range predicates (they stay filters forever — a range is a
scan by nature and the range family exists to measure exactly that).

## Passing criteria

- Unit tests in `plan/fj.rs` (or its test module): lowering the string-shaped
  query (`Posting(id, amount, memo = ?0)`) yields one selection
  (`memo`, `Const::Param(0)`) and zero filters; the fk_walk shape yields the
  `account = ?0` and `id = ?0` selections on their occurrences; the chain shape
  (`status = Open` literal + `at >= ?0`) yields one selection
  (`status`, `Const::Byte`/`Word`) and one residual filter (`Ge`); a repeated
  in-atom variable stays a `FieldsCompare` filter.
- The new `PlanError` variant is constructed and asserted by a hand-built
  invalid plan.
- Selection ordering determinism: lowering the same query twice yields equal
  plans (`assert_eq!` on the plan).
- The full existing test suite passes unchanged — including the engine's
  differential family, the bench crate's `verify` full-S test, and the eight
  family golden tests. `scripts/check.sh` green (both feature configs).
