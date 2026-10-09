# engine-query lane board

Owns: `crates/bumbledb/src/{ir*, plan*, exec.rs, exec/**, image*, scalar.rs, api/prepared.rs,
api/prepared/** except tests/float_aggregates.rs}`, `tests/{adversarial_ir, point_reads,
reach_finalize_hunt, float_numerics}.rs`. The numeric lane's finished paths (`exec/kernel`,
`exec/sink/aggregate`, `scalar.rs`, `api/prepared/computed`, `tests/float_numerics.rs`) moved here;
the hand-tuned NEON kernels stay.

Items: tests cull, A (plan validator), C3, C4, C5 (query side), C11, C12, C13, ValidationError,
E5 (lowering/fold/residual, MIN/MAX), E8, E3 (consume numeric's check), C7 adapt, C8 adapt, L,
`unreachable_pub`.

## Status

| Item | Status |
|---|---|
| Tests: delete timing pins and host-pinned falsifiers | landed `5f6f95be6` |
| E3: guard block and `numeric_outputs` deleted | landed `7067683fb` |
| C7: `plan/ground.rs` under `feature = "testing"` | landed `23e4ef997` |
| C3: cursor fallback deleted; `check_resident` in image allocation | landed `89ab11f08` |
| ValidationError fold; A (`Unplannable`) | landed `c5ba8f566` (`ParamIdGap`, `AggregateInputType` wait, see requests) |
| C4: RAM-only `SeenSet`; resident-only derived stages; `exec/scratch` deleted | landed `bed4f0e29`, `39dfb8b2d`, `e0df773d0` |
| adversarial_ir sharding | landed `f3cfe1678` |
| E8: exact int/F64 literal comparisons | landed `56e82d87c` |
| E5: F64 order excludes NaN (literals, params, variable pairs) | landed `c153773d4` |
| E5: F64 MIN/MAX lower to `AggSpec::Float` (MIN propagates NaN) | landed `e2a0572fc` |
| C11: one rule runner, `EitherSink` delegation, one `i64_word`; Program/Bound/Runtime | landed `1818965ee`, `d5fdd0e8e` |
| C8 adapt: no query file names `StoreError`/`Error::Store` | landed `c4c3e8f2d` |
| C5 query side: closed images and closed-row filters from sealed rows; `image/decode.rs` deleted | landed `4460daaa8` |
| C12, C13 | in progress |
| L, `unreachable_pub` | todo |

## Requests to other lanes

### engine-storage

1. **C5 query side landed (`4460daaa8`).** Nothing in my paths reads `FactLayout`, `FactView`,
   `SealedRow::fact`, `Relation::layout()` or `encoding::{field_bytes, interval_words, split_halves,
   decode_values_keyed_into, decode_sealed}` any more; HEAD's lib clippy now reports them dead until
   you delete them. `canonical::decode_sealed` is the last caller of my
   `api::prepared::source::work_error`; delete it and I delete `work_error` (it is dead
   transitively until then).
2. **C8 stage B unblocked (`c4c3e8f2d`).** My files convert through `Error::from(WorkError)`,
   `Error::from(RowError)` and `?`, and match `is_cancelled()` / `kind()`. Inside storage callbacks
   that return `StoreResult` I use `work.checkpoint()?` (`From<WorkError> for StoreError`). The
   only `crate::store` alias user left is `image/cache/tests.rs` (`use crate::store::RelationVersion`),
   kept so lib.rs's `#[cfg(test)] use storage::store;` stays used; when you delete that alias, tell
   me and I switch the import to `crate::storage::store::RelationVersion` in the same window.
3. **`ParamIdGap`.** `tests/edge.rs` matches `ValidationError::ParamIdGap { param }`. Match
   `Error::Validation(_)` there (or tell me and I fold it into
   `Param { param, refusal: ParamRefusal::IdGap }` in the same window you switch).

### bridge

- `bumbledb-node/src/query.rs` names `ValidationError::{AggregateInputType, ParamIdGap}`. When I
  fold them (`Aggregate { refusal: AggregateRefusal::InputType }`, `Param { refusal:
  ParamRefusal::IdGap }`) the two arms move into the existing `Aggregate`/`Param` arms.

### consolidator

- `AggregateSink::spill` (my `exec/sink.rs`) and the `spill: None` / `aggregate::spill::GroupSpill`
  stub (now also mine) go together; I do it with C12.
- `api/prepared/tests/float_aggregates.rs` is not in my paths; its uncommitted
  `f64_min_and_max_propagate_nan_through_queries` duplicates
  `tests/aggregates.rs::f64_min_and_max_propagate_nan_on_leaf_outer_and_ungrouped_inputs`.

## API changes (announcements)

- **C5 (query side).** `image::synthesize_closed(schema, rel, &generation, &work)` builds through
  `build_from_scan` over `SealedRow::row` bytes (same decoder, same columns as stored rows);
  `QuerySource::scan` yields a closed relation's `row.row.as_bytes()`. Closed-row σ in
  `plan/ground/evaluate.rs` reads `SealedRow::values` (each value lowers like a literal); closed
  relations hold no text, so it never consults a resolver. Deleted: `image/decode.rs`,
  `plan/ground/evaluate/text_eq.rs`.
- **E5 (MIN/MAX).** Every F64 aggregate lowers to `AggSpec::Float { op, slot }`; MIN folds NaN's key
  as 0 and restores it at output (row, batch, scan and outer paths), MAX is the plain key max.
- **C8 adapt.** `api::prepared::source::store_error` is deleted; `work_error` is `Error::from`.
- **C3.** `PreparedQuery::force_cursor_fallback`, `api/prepared/fallback.rs`, `forced_fallback`,
  `QuerySource::exceeds_resident_positions` and `RESIDENT_ROW_LIMIT` are gone. Every relation image
  (store, selection or derived) with `>= u32::MAX` rows refuses with
  `Error::Capacity(Capacity::ResidentRows)` at allocation.
- **C4.** Derived stages and rec frontiers are resident `Arc<RelationImage>`s. A new distinct row
  past the `WordMap` index limit records `Error::Capacity(Capacity::DistinctRows)` (sticky).
  `exec::sink::SeenSet` replaces `SpillSet`.
- **E3.** `PreparedQuery` no longer enters `NumericalGuard`; `numeric_outputs` is gone.
- **E5 (order).** IEEE order on F64 with equality under which NaN equals itself: `x < NaN` lowers
  to the empty comparison; every F64 variable bounded from below or compared with another variable
  by order gets the companion `x <= +Infinity`; an F64 parameter under an order operator lowers to
  `Const::DenseOrderParam(ParamId)` and a NaN value resolves to the empty range.
- **E8.** `x op literal` with mixed integer/F64 sides is restated exactly in `x`'s own type at
  validation; variable-against-variable int/F64 orders refuse with
  `ComparisonRefusal::MixedNumeric`.
- **ValidationError fold (`c5ba8f566`).** Variants group by the position they cite: `TooMany`,
  `Head`, `Field`, `Variable`, `Param`, `Comparison`, `Aggregate`, `Rec`, plus the unchanged
  singletons and `Unplannable`.
