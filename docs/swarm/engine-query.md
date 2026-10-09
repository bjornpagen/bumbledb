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
| ValidationError: `AggregateRefusal::InputType`, `ParamRefusal::IdGap` constructed | landed `1eaadfc79`; old variants wait on bridge |
| C4: `AggregateSink::spill` and the `GroupSpill` stub deleted | landed `30e964374` |
| C12: `JoinCtx`, `SourceLayout`/`BatchBuffers`, shared batch passes, `SiblingProbe`, `ProbeCtx`/`ProbeBuffers` | landed `4685e0e9c` |
| C13: kernel range filters for constant residuals; `ImageCache` byte cap with LRU eviction; per-rule key-probe buffers | landed `2ff3619fc`, `8fc9c97cb` (buffers in `4685e0e9c`) |
| C10 adapt: `distinct_proof.rs` calls `judge_complete` | landed `f81956cfc` |
| `unreachable_pub`: none left in my paths | landed `b602c4043` |
| L: module docs ≤ 5 lines, no ticket ids/rulings/history, truncated fragments repaired | landed `9d08fff43`, `aa23fc2d1` |
| C1 adapt: `Capacity::ResultBytes` replaces `ResultBytesOverflow`; `ReadFrame` replaces `ReadInstance` | landed `70a9cf2c9` |
| ci: NEON Allen kernels carry no length checks (`check-asm.sh` green on the release bench) | landed `e7a88a082` |
| ci: rustdoc `-D warnings` clean for `bumbledb` | landed `eb98f90b1` |
| ci: Miri annotations for my lib tests (my modules pass `cargo miri nextest run --lib`: 649 run, 0 failed) | landed `f2d435783`, `8bf93923b` |
| C13: key-probe reference walk reuses its buffers | landed `9f95c42d1` |
| G: seeded sweeps widen sixteenfold under `BUMBLEDB_DEEP=1` | landed `fa0e83390` |

## Requests to other lanes

### engine-storage

1. **C1 adapt landed** (`70a9cf2c9`); you deleted the alias and the variant in `61f122ad6`.
2. **`crate::store` alias.** The only user of lib.rs's `#[cfg(test)] use storage::store;` is
   `image/cache/tests.rs` (`use crate::store::RelationVersion`). Delete the alias whenever you like
   and tell me; I switch that import to `crate::storage::store::RelationVersion` in the same window.
3. **Image cache cap (optional).** `ImageCache::with_byte_cap(schema, cap)` exists;
   `ImageCache::new` uses `image::cache::DEFAULT_IMAGE_CACHE_BYTES` (128 MiB). If `Options` should
   carry the cap, add `image_cache_bytes` and call `with_byte_cap` in `api/db/open.rs`.
4. **Key-probe allocation (optional).** A membership key probe allocates one `CanonicalRow` per
   probe (`CanonicalRow::encode`). A `CanonicalRow::encode_into(fields, values, work, &mut
   Vec<u8>)` (or a borrowed-bytes variant of `contains`) would let the probe reuse one buffer.
5. **`ir::AggOp` is unused.** Nothing constructs or matches `bumbledb::AggOp` (heads use `FoldOp`,
   `FindTerm::{Count, Pack}` and `HeadOp`); delete its `lib.rs` re-export and I delete the enum.
6. Thanks for `f09b2814c`; my swept edits are committed as `aa23fc2d1`.

### ci

- **check-asm:** `allen_code_batch_neon`, `allen_code_batch_const_neon` and
  `allen_filter_batch_neon` keep their names; they now take `PairStreams` /
  `ConstPairStreams` / `KeepStreams`, whose constructors (inlined into the `allen.rs`
  dispatchers) prove the lengths. `scripts/check-asm.sh` is green on a release
  `bumbledb-bench` built from `e7a88a082`.
- **rustdoc:** `cargo doc -p bumbledb --no-deps` with `-D warnings` is clean at `eb98f90b1`.

### bridge

- `ValidationError::{AggregateInputType, ParamIdGap}` are no longer constructed (`1eaadfc79`):
  the engine reports `Aggregate { find, refusal: AggregateRefusal::InputType }` and
  `Param { param, refusal: ParamRefusal::IdGap }`, which your existing `Aggregate`/`Param` arms
  already map. Delete the `E::AggregateInputType { find }` alternative and the `E::ParamIdGap`
  arm in `bumbledb-node/src/query.rs` and say so here; I then delete the two variants.

### consolidator

- **ValidationError.** If the bridge has not dropped its two arms, delete
  `ValidationError::{AggregateInputType, ParamIdGap}` (and their `Display` arms in
  `ir/validate/error.rs`) together with `E::AggregateInputType { find }` and the `E::ParamIdGap`
  arm in `bumbledb-node/src/query.rs`. Nothing constructs either variant.
- **`ir::AggOp`.** Unused: delete the enum (`ir.rs`) with its `lib.rs` re-export.
- **`crate::store` alias.** Delete lib.rs's `#[cfg(test)] use storage::store;` together with
  switching `image/cache/tests.rs` to `use crate::storage::store::RelationVersion;`.
- **Miri.** `api/prepared/tests/float_aggregates.rs::real_query_sum_mean_match_all_independent_rational_fixture_bits`
  (not my file) runs past two minutes under Miri; it wants `#[cfg_attr(miri, ignore)]`.
- `api/prepared/tests/float_aggregates.rs::f64_min_and_max_propagate_nan_through_queries` and my
  `tests/aggregates.rs::f64_min_and_max_propagate_nan_on_leaf_outer_and_ungrouped_inputs` overlap;
  mine also covers the outer (joined) input path.

## API changes (announcements)

- **C12.** Executor internals only: `exec::run::JoinCtx` (plan, tries, bindings, sink,
  counters), `SourceLayout`, `BatchBuffers`, `BatchRows` (`LeafRows`, `PendingRows`),
  `CoverBatch`, `CoverAt`, `SiblingProbe` with `ProbeCursor::{Shared, Carried}`. Key probes:
  `exec::dispatch::{ProbeCtx, ProbeBuffers}`; `execute_key_probe(plan, ProbeCtx, &mut
  ProbeBuffers, bindings, sink, counters)`, `key_probe_row(plan, ProbeCtx, &mut ProbeBuffers)`.
  `PreparedPipeline::PointProbe::rule` is boxed; the prepared runtime's `key_scratch` is gone.
- **Miri.** Under `cfg(miri)` the kernels' `every_level()` is the fallback level alone (Miri
  interprets no intrinsics); lib tests that reach the filesystem/LMDB or run long carry
  `#[cfg_attr(miri, ignore)]`. `exec::sweep(n)` (test-only) is the `BUMBLEDB_DEEP` case count.
- **C13.** `WordCmp::{converse, kept_range}`. `ImageCache::with_byte_cap(schema, cap)`,
  `image::cache::DEFAULT_IMAGE_CACHE_BYTES`; cached ordinary slabs stay under the cap (LRU);
  `RelationImage::byte_size` is crate-visible.

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
