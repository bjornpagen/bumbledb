# engine-query lane board

Owns: `crates/bumbledb/src/{ir*, plan*, exec.rs, exec/** except kernel and sink/aggregate, image*,
api/prepared.rs, api/prepared/** except computed}`, `tests/{adversarial_ir, point_reads,
reach_finalize_hunt}.rs`.

Items: tests cull, A (plan validator), C3, C4, C5 (query side), C11, C12, C13, ValidationError,
E5 (lowering/fold/residual), E8, E3 (consume numeric's check), C7 adapt, L.

## Status

| Item | Status |
|---|---|
| Tests: delete timing pins and host-pinned falsifiers | landed `5f6f95be6` |
| E3: guard block and `numeric_outputs` deleted | landed `7067683fb` |
| C7: `plan/ground.rs` under `feature = "testing"` | landed `23e4ef997` |
| C3: cursor fallback deleted; `check_resident` in image allocation | landed `89ab11f08` |
| ValidationError fold; A (`Unplannable`) | landed `c5ba8f566` (`ParamIdGap`, `AggregateInputType` wait, see requests) |
| C4: RAM-only `SeenSet`; resident-only derived stages | landed `bed4f0e29`, `39dfb8b2d`; `exec/scratch` deletion waits on numeric deleting `stream_finalize` |
| adversarial_ir sharding | landed `f3cfe1678` |
| E8: exact int/F64 literal comparisons | landed `56e82d87c` (params: see notes) |
| E5: F64 order excludes NaN (literals, params, variable pairs) | landed `c153773d4`; MIN/MAX lowering waits on numeric |
| C5 query side | waiting on engine-storage C5 |
| C11, C12, C13 | todo |
| L | todo |

## Requests to other lanes

### numeric

1. **Delete `AggregateSink::stream_finalize` now** (`39dfb8b2d` stopped calling it, so HEAD's lib
   clippy reports it dead until it goes). Also drop the `exec::scratch` and `encode_stage_row`
   imports in `aggregate/finalize.rs`. Then I delete `exec/scratch.rs`, `exec/scratch/`,
   `encode_stage_row` (it carries a temporary `allow(dead_code)`), and the `allow` on `mod scratch`.
2. **`AggregateInputType`.** `api/prepared/tests/float_aggregates.rs:169` matches
   `ValidationError::AggregateInputType`. When you switch that line to
   `ValidationError::Aggregate { refusal: AggregateRefusal::InputType, .. }`, say so here; I add the
   variant first (both shapes coexist for one commit), then fold.
3. **Spill field.** Deleting `AggregateSink::spill` (my `sink.rs`) and `spill: None` (your
   `aggregate/new.rs`) is one atomic edit; the consolidator does it.
4. **E5 MIN/MAX lowering.** Will do when you announce `AggSpec::Float { op: Min | Max }`.
5. HEAD `api/prepared/computed/tests.rs` has clippy findings (`cast_possible_truncation` at 246,
   295, 309; `collapsible_if` at 383) under `--all-targets -D warnings`.

### engine-storage

1. **HEAD lib-test build.** `src/api/db/tests.rs:474,583` import `storage::store::MapPolicy`
   (deleted in C17), so `-p bumbledb --all-targets` does not build for any lane.
2. **`ParamIdGap`.** `tests/edge.rs:480` matches `ValidationError::ParamIdGap { param }`. Match
   `Error::Validation(_)` there (or tell me and I fold it into
   `Param { param, refusal: ParamRefusal::IdGap }` in the same window you switch).
3. **Re-exports (optional).** The refusal enums are at `bumbledb::ir::validate::error::{Limit,
   HeadMismatch, FieldRefusal, VariableRefusal, ParamRefusal, ComparisonRefusal, Unordered,
   AggregateRefusal, RecRefusal}`; re-export them next to `ValidationError` if you want them at the
   crate root.
4. **`exec::scratch` users outside my paths:** none left in HEAD, thanks.
5. **C5.** Announce the canonical closed-row API; I switch `image/build.rs` `synthesize_closed`
   and `plan/ground/evaluate` and delete `image/decode.rs`.

### bridge, bench

- `ValidationError` changed shape (below). `bumbledb-node/src/query.rs` and
  `bumbledb-bench/src/oracle/sqlite/verify/run_algebra.rs` (`EmptyAllenMask`/`FullAllenMask` are now
  `Comparison { refusal: ComparisonRefusal::{EmptyAllenMask, FullAllenMask}, .. }`) must adapt.

## API changes (announcements)

- **C3.** `PreparedQuery::force_cursor_fallback`, `api/prepared/fallback.rs`, `forced_fallback`,
  `QuerySource::exceeds_resident_positions` and `RESIDENT_ROW_LIMIT` are gone. Every relation image
  (store, selection or derived) with `>= u32::MAX` rows refuses with
  `Error::Capacity(Capacity::ResidentRows)` at allocation.
- **C4.** Derived stages and rec frontiers are resident `Arc<RelationImage>`s (`SealedStage` and
  `api/prepared/derived.rs` are deleted). A new distinct row past the `WordMap` index limit records
  `Error::Capacity(Capacity::DistinctRows)` (sticky). `exec::sink::SeenSet` replaces `SpillSet`;
  `ProjectionSink::{spilled, force_spill, stream_into_scratch}` and `Sink::retains_binding_slot` are
  gone; `ProjectionSink::for_each_answer` is test-only. `WordMap::assume_full()` (`#[cfg(test)]`)
  makes the next new key meet the index limit.
- **E3.** `PreparedQuery` no longer enters `NumericalGuard`; `numeric_outputs` is gone.
- **E5.** IEEE order on F64 with equality under which NaN equals itself:
  - `x < NaN` (any order operator, literal) lowers to the empty comparison `x < -Infinity`;
    `RangeSummary` knows the F64 floor (`-Infinity`'s key), so the fold proves it statically empty.
  - Every F64 variable bounded from below (`>`, `>=`) or compared with another variable by order
    gets the companion `x <= +Infinity`, which excludes NaN rows (ordinary range filter; the fold
    merges it).
  - An F64 parameter under an order operator lowers to `Const::DenseOrderParam(ParamId)` /
    `SealedConst::DenseOrderParam`; a NaN value resolves to the empty range (positive occurrences
    short-circuit; negated ones get `x < 0`).
  - `image::view::DENSE_POS_INF_KEY`.
- **E8.** `x op literal` with an integer `x` and an F64 literal, or an F64 `x` and an integer
  literal, is restated exactly in `x`'s own type at validation (`ir/validate/mixed.rs`, integer-only
  arithmetic on the binary64 fields): `x < 2.5` → `x <= 2`, out-of-domain bounds become "always"
  (dropped) or "never" (`x < MIN`, statically empty). Variable-against-variable int/F64 orders
  refuse with `ComparisonRefusal::MixedNumeric` (message names `toF64Exact`). A parameter keeps one
  anchored type: binding an F64 parameter against an integer variable remains
  `ParamRefusal::TypeConflict`.
- **ValidationError fold (`c5ba8f566`).** Variants group by the position they cite:
  - `TooMany { limit: Limit::{Rules, Atoms, Variables, DerivedTables}, count }`
  - `Head { rule, position, mismatch: HeadMismatch::{Type, Aggregate} }` (+ `HeadArityMismatch`)
  - `Field { atom, field, refusal: FieldRefusal::{Unknown, DuplicateBinding, LiteralType,
    PointLiteralAtCeiling, InteriorColumnOutOfRange} }`
  - `Variable { var, refusal: VariableRefusal::{TypeConflict, MembershipOnly, NegatedUnbound,
    UnboundFind, ComparisonOnly} }`
  - `Param { param, refusal: ParamRefusal::{TypeConflict, ScalarAndSet, IntervalSet} }`
    (`ParamIdGap { param }` until edge.rs switches)
  - `Comparison { index, refusal: ComparisonRefusal::{ParamSet, IllegalTypes, MixedNumeric,
    Unordered(Unordered::{Interval, FixedBytes, String, ClosedReference}), Constant,
    SelfComparison, PointLiteralAtCeiling, EmptyAllenMask, FullAllenMask}) }`
  - `Aggregate { find, refusal: AggregateRefusal::{ClosedReference, CountWithVariable,
    WithoutVariable, OverGroupKey, MultiplePack, MixedPackAndFold, PackInputType} }`
    (`AggregateInputType { find }` until float_aggregates.rs switches)
  - `Rec(RecRefusal::{EmptyBase, EmptyStep, SelfInBase, ArmMissingSelf, NonlinearArm, Negation})`
  - unchanged: `EmptyRuleSet`, `ScalarExpression`, `DnfExceedsRules`, `ConditionNestingTooDeep`,
    `CountAcrossRules`, `UnknownRelation`, `UnknownInterior`, `EmptyFinds`, `DuplicateFindTerm`,
    `NoPositiveAtoms`, `EmptyInterior`, `InteriorNotPrior`, `AggregateInInterior`
  - new: `Unplannable` (the plan validator refused a planner-built plan: an engine defect, typed).
- `plan::ground::with_grounding_disabled` is compiled under `any(test, feature = "testing")`.
- Integration tests use the `schema!` relation constants (`Gauntlet::Busy.relation()`).
