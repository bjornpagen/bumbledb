# numeric lane board

Owns: `crates/bumbledb/src/{scalar.rs, exec/kernel.rs, exec/kernel/**, exec/sink/aggregate.rs,
exec/sink/aggregate/**, exec/sink/tests/aggregate.rs, api/prepared/computed.rs,
api/prepared/computed/**, api/prepared/tests/float_aggregates.rs}`, `tests/float_numerics.rs`.

Items: A/E1 (`gather_words`), E3, E4a, E4b, E5 (MIN/MAX), E6, E7, C4 (aggregate spill), bench
kernel seam, timing-pin removal.

## Status

| Item | Status |
|---|---|
| Timing pins and kernel experiment twins out of `cargo test` | landed `78728ec07` |
| A/E1: `gather_words` bounds | landed `78728ec07` |
| E4a: fearless_simd filter/fold/gather | landed `01a432a55` |
| E4b: portable Allen, AVX2 compress | landed `d3b4fe666` |
| E3: read-only FP environment check | landed `dff5ac835`; `UnsupportedNumericalPlatform` deleted after the `lib.rs` swap |
| E7: columnar computed outputs | landed `1028d2823` |
| C4: aggregate spill deletion | landed `9c7ec5452`, `stream_finalize` deleted in `2ac680723` |
| E6: xsum exact SUM/AVG | landed `2ac680723` |
| E5: F64 MIN NaN propagation | landed `f3c4db520` on the aggregate side; live once F64 `Min`/`Max` lower to `AggSpec::Float` (open request 2) |
| Bench kernel seam | landed (`bumbledb::kernels`) |

## Open requests

1. ~~engine-storage: `lib.rs` re-exports `NonDefaultFloatEnvironment`~~ landed `af99940a3`;
   `UnsupportedNumericalPlatform` is deleted.
2. **engine-query:** lower F64 `Min`/`Max` to `AggSpec::Float { op, slot }` (drop the
   `Sum | Mean` guard in `build.rs`) and change the `AggSpec::Float` doc to "F64 argument: Sum/Mean
   exact, Min/Max over order keys with NaN propagating". The aggregate path already runs
   `AggSpec::Float { op: Min | Max }` on the row, batch and scan paths (MIN propagates NaN, MAX is
   unchanged); `AggSpec::seed_acc` is never called for `Float`.
3. ~~engine-query: `AggregateInputType`~~ `float_aggregates.rs` now matches
   `Err(Error::Validation(_))`, so you can fold `AggregateInputType` into
   `Aggregate { refusal: AggregateRefusal::InputType }` in one commit; nothing of mine names it.
4. **consolidator:** `AggregateSink::spill` (engine-query's `exec/sink.rs`) and `spill: None`
   (my `aggregate/new.rs`) go in one edit, together with the `aggregate::spill::GroupSpill` stub
   in `aggregate.rs`.

## API changes (announcements)

- **E6.** `ExactF64Accumulator` is Neal's small superaccumulator (67 chunks, lazy carries);
  sum/mean bits are unchanged (the old 34-limb accumulator is the test oracle).
  `push_keys(impl ExactSizeIterator<Item = u64>)` takes F64 order keys through four lane-private
  accumulators. F64 SUM/AVG take the leaf-scan path and share one exact reduction per input
  column. `encode_into`/`decode_from` are deleted.
- **E7.** Computed outputs compile once per `OutputProgram` (cached across `aim`) into postfix
  programs over 64-lane registers; `emit_batch` runs 64 bindings per SIMD dispatch.
  `OutputProgram`'s fields and `computed::lower` are unchanged. Error identity is unchanged: the
  first failing binding in batch order reports its first error in evaluation order.
- **E3.** The asm install/compute/restore guard and `NumericalGuard` are gone; F64 arithmetic is
  plain `f64` plus canonicalization under a read-only FPCR/MXCSR check.
  - `bumbledb::ScalarError::UnsupportedPlatform` is now `ScalarError::NonDefaultFloatEnvironment`.
  - `exec::kernel::numeric::NonDefaultFloatEnvironment` (`Copy`, `Eq`, `std::error::Error`,
    `control(self) -> u64` = the refused register image). `F64Math::{add, subtract, multiply,
    divide}` return `Result<F64, NonDefaultFloatEnvironment>`. `F64Math::operation` and
    `F64Operation` are deleted.
  - `ScalarEvaluator::new() -> Result<Self, ScalarError>` checks the environment once.
  - Computed outputs check once per execution in `ComputedSink::reset`; F64 arithmetic then fails
    with `Error::Scalar { find, source: NonDefaultFloatEnvironment }` if the check failed.
- **C4.** The group spill tier is gone. `probe_group` refuses a new group past the `WordMap`
  index limit with `Error::Capacity(Capacity::Groups)`; `group_count()` is exact;
  `resident_row_bound()` ignores spill; `aggregate/new.rs` uses `SeenSet`; `stream_finalize` is
  deleted.
- **E4a/E4b.** No `std::simd` in the crate. Kernels dispatch once per call on a `OnceLock`-cached
  `fearless_simd::Level` (`Level::fallback()` under Miri); aarch64 Allen keeps the NEON kernel.
- **Bench kernel seam** (`bumbledb::kernels`): `SimdLevel` (`available()`, lowest first, no scalar
  fallback outside tests; `name()`), every kernel at an explicit level with the entry point's
  arguments after `level` (`filter_*`, `fold_*`, `fold_*_idx`, `allen_*`, `compact_u32_by_mask`),
  and `reference::*`, one scalar twin per kernel.
