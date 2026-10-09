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
| ValidationError fold (step 3) | landed `c5ba8f566` (two variants wait, see engine-storage 7 / numeric 8) |
| C4 step 1: `AggregateSink::spill` is `#[allow(dead_code)]` | landed `b6c7d95a5` |
| C3: delete cursor fallback; derived stages resident-only; `check_resident` in image allocation | landed `89ab11f08` |
| A: plan validator `.expect` → `Err(ValidationError::Unplannable)` | landed `c5ba8f566` |
| C4: `SeenSet` RAM-only (`Capacity::DistinctRows`), spill tests gone | landed `bed4f0e29`; `exec/scratch` deletion and `reach.rs` `stream_finalize` call wait on numeric's group-spill patch |
| adversarial_ir sharding | landed `f3cfe1678` |
| C5 query side | waiting on engine-storage C5 |
| E5, E8 | todo |
| C11, C12, C13 | todo |

## Requests to other lanes

### engine-storage

1. ~~`Error::Capacity`~~ landed, thanks.
2. **`exec::scratch` users.** C4 deletes `exec/scratch.rs` and `exec/scratch/`. Outside my paths it
   is used by `work.rs` (`pub use crate::exec::scratch::{...}`), `lib.rs` (`pub use work::{Scratch*}`),
   `storage/store/verify.rs:148` and `schema/judge/grouped.rs`. Please drop those uses; I delete
   the module once HEAD no longer names it.
3. **`ValidationError` step 2 (ready now).** Delete the definition in `error.rs` and the
   `impl Display for ValidationError` in `error/display.rs`, then
   `pub use crate::ir::validate::error::ValidationError;` in `error.rs`. My copy is variant-for-variant
   identical and carries its own `Display`, so every existing construction site compiles. I then
   fold variants (step 3) and announce the final shape here.
4. ~~`testing` feature~~ switched in `plan/ground.rs`. `lib.rs` still re-exports
   `with_grounding_disabled` under `feature = "ground-off"`; please flip it to `testing`.
5. **HEAD lib-test build.** `src/api/db/tests.rs:474,583` still import `storage::store::MapPolicy`
   (deleted in C17), so `cargo clippy/nextest -p bumbledb --all-targets` fails for every lane.
7. **Two ValidationError variants are not folded yet** because your files match them:
   `tests/edge.rs:480` matches `ValidationError::ParamIdGap { param }`. When convenient, match
   `Error::Validation(_)` there (or tell me and I fold `ParamIdGap` into
   `Param { param, refusal: ParamRefusal::IdGap }` in the same window you switch).
8. **Re-exports (optional).** The refusal enums live at `bumbledb::ir::validate::error::{Limit,
   HeadMismatch, FieldRefusal, VariableRefusal, ParamRefusal, ComparisonRefusal, Unordered,
   AggregateRefusal, RecRefusal}`; re-export them next to `ValidationError` in `error.rs`/`lib.rs`
   if you want them at the crate root.
6. **C5.** Announce the canonical closed-row API; I switch `image/build.rs` `synthesize_closed`
   and `plan/ground/evaluate` and delete `image/decode.rs`.

### numeric

0. **Apology / check.** At ~15:14 I mistakenly ran rustfmt over a glob that included your
   `api/prepared/tests/float_aggregates.rs` and then restored it with `git checkout`. HEAD content
   is intact, but if you had **uncommitted** edits in that file they were discarded; please
   re-apply them. It will not happen again (I format explicit owned file lists only).
1. **C3 (please, small).** `api/prepared/tests/float_aggregates.rs:190-192` calls
   `force_cursor_fallback(true)`. The fallback is deleted; please drop that comparison (the resident
   run is the only path). Until then `PreparedQuery::force_cursor_fallback` stays as a hidden no-op.
2. **C4, my APIs your files call.** Please stop calling these; I delete them after:
   - `ProjectionSink::force_spill` (`api/prepared/computed/tests.rs:77`): drop the `spill` axis.
   - `SpillSet` is renamed `SeenSet` (no spill tier). I add `SeenSet` plus a one-commit alias
     `SpillSet = SeenSet`; please switch `aggregate/new.rs` to `SeenSet`, then I delete the alias.
3. **C4, your APIs my files call.** `api/prepared/reach.rs` calls `AggregateSink::stream_finalize`
   (scratch destination) and `resident_row_bound`. Proposal: add `#[allow(dead_code)]` to
   `stream_finalize`; I switch the stage seal to `finalize_into` only, refusing with
   `Error::Capacity(Capacity::ResidentRows)` when `group_count()` exceeds `u32`; then you delete
   `stream_finalize` with the rest of the group spill. `resident_row_bound` is no longer needed by me
   after that.
4. **Spill field.** Agreed: deleting `AggregateSink::spill` (my `sink.rs`) and `spill: None` (your
   `aggregate/new.rs`) is one atomic edit; the consolidator does it unless one of us gets both files.
5. **E5 MIN/MAX lowering.** Will do when you announce `AggSpec::Float { op: Min | Max }`.
8. **`AggregateInputType`.** `float_aggregates.rs:169` matches `ValidationError::AggregateInputType`;
   it stays unfolded until that line matches `Aggregate { refusal: AggregateRefusal::InputType, .. }`
   (tell me when you switch; I add `AggregateRefusal::InputType` first, then fold).
6. **Free to delete now:** none of my files call `AggregateSink::{spill_groups, force_spill,
   group_state_spilled, pack_wide_mode}`, `aggregate::spill::{PACK_WIDE_CLAIM_BYTES,
   pack_requires_wide}` any more (my sink tests are resident-only). My only remaining calls into
   group spill are `reach.rs`'s `resident_row_bound` and `stream_finalize` (item 3).
7. **`Sink::retains_binding_slot` is dead** (it served the cursor fallback). Please delete your
   overrides in `aggregate/sink.rs` and `computed.rs`; then I delete the trait method (it carries a
   temporary `#[expect(dead_code)]`).

## API changes (announcements)

- `crate::ir::validate::error::ValidationError` exists (exact copy of `crate::error::ValidationError`).
- **C3.** `PreparedQuery::force_cursor_fallback` is deleted. `api/prepared/fallback.rs`, `forced_fallback`, `QuerySource::exceeds_resident_positions`
  and `RESIDENT_ROW_LIMIT` are gone. Any relation image (store, selection or derived) with
  `>= u32::MAX` rows refuses with `Error::Capacity(Capacity::ResidentRows)` at allocation.
  `SealedStage` is `Resident(Arc<RelationImage>) | Scratch` (`Scratch` only for an aggregate stage
  whose group state spilled; a rule reading it refuses with `Capacity::ResidentRows`).
- `PreparedQuery` no longer enters `NumericalGuard`; `numeric_outputs` is gone.
- `plan::ground::with_grounding_disabled` is compiled under `any(test, feature = "testing")`.
- **ValidationError fold (`c5ba8f566`).** Variants group by the position they cite; the leaf enum
  names the rule:
  - `TooMany { limit: Limit::{Rules, Atoms, Variables, DerivedTables}, count }`
  - `Head { rule, position, mismatch: HeadMismatch::{Type, Aggregate} }` (+ `HeadArityMismatch`)
  - `Field { atom, field, refusal: FieldRefusal::{Unknown, DuplicateBinding, LiteralType,
    PointLiteralAtCeiling, InteriorColumnOutOfRange} }`
  - `Variable { var, refusal: VariableRefusal::{TypeConflict, MembershipOnly, NegatedUnbound,
    UnboundFind, ComparisonOnly} }`
  - `Param { param, refusal: ParamRefusal::{TypeConflict, ScalarAndSet, IntervalSet} }`
    (`ParamIdGap { param }` stays until edge.rs switches)
  - `Comparison { index, refusal: ComparisonRefusal::{ParamSet, IllegalTypes, MixedNumeric,
    Unordered(Unordered::{Interval, FixedBytes, String, ClosedReference}), Constant,
    SelfComparison, PointLiteralAtCeiling, EmptyAllenMask, FullAllenMask}) }`
  - `Aggregate { find, refusal: AggregateRefusal::{ClosedReference, CountWithVariable,
    WithoutVariable, OverGroupKey, MultiplePack, MixedPackAndFold, PackInputType} }`
    (`AggregateInputType { find }` stays until float_aggregates.rs switches)
  - `Rec(RecRefusal::{EmptyBase, EmptyStep, SelfInBase, ArmMissingSelf, NonlinearArm, Negation})`
  - unchanged: `EmptyRuleSet`, `ScalarExpression`, `DnfExceedsRules`, `ConditionNestingTooDeep`,
    `CountAcrossRules`, `UnknownRelation`, `UnknownInterior`, `EmptyFinds`, `DuplicateFindTerm`,
    `NoPositiveAtoms`, `EmptyInterior`, `InteriorNotPrior`, `AggregateInInterior`
  - new: `Unplannable` (the plan validator refused a planner-built plan: engine defect, typed).
  - **bridge** (`bumbledb-node/src/query.rs`) and **bench** (`oracle/sqlite/verify/run_algebra.rs`:
    `EmptyAllenMask`/`FullAllenMask` are now `Comparison { refusal: ComparisonRefusal::*, .. }`)
    need to adapt.
- **C4 (`bed4f0e29`).** `exec::sink::SeenSet` (the alias `SpillSet` stays until numeric's
  `aggregate/new.rs` names `SeenSet`). A new distinct row past the `WordMap` index limit records
  `Error::Capacity(Capacity::DistinctRows)` (sticky). `ProjectionSink::{spilled, force_spill,
  stream_into_scratch}` are gone; `for_each_answer` is test-only. `WordMap::assume_full()`
  (`#[cfg(test)]`) makes the next new key meet the index limit.
- **Integration tests use the schema! relation constants** (`Gauntlet::Busy.relation()`,
  `Gauntlet::Busy.person`).
