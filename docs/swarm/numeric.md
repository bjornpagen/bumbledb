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
| E4a: fearless_simd filter/fold/gather | landed |
| E4b: portable Allen, Avx2 compress | landed |
| E3: read-only FP environment check | landed; waiting on the `lib.rs` re-export swap to delete `UnsupportedNumericalPlatform` |
| E5: MIN NaN propagation | todo (needs engine-query lowering, see request) |
| E6: xsum exact SUM/AVG | landed |
| E7: columnar computed outputs | landed |
| C4: aggregate spill deletion | landed; `stream_finalize` stays until `reach.rs` stops calling it |
| Bench kernel seam | landed in-crate; needs the `lib.rs` re-export below |

## Plans that affect other lanes

### E3 (engine-query, engine-storage)

- The check lives inside the computed-output sink: `ComputedSink::reset` reads FPCR/MXCSR once per
  execution, only when a program does F64 arithmetic, and records
  `Error::Scalar { find, source: ScalarError::NonDefaultFloatEnvironment }` (sticky; finalize
  refuses). Nothing in `execute.rs` needs to call it.
- engine-query removed the guard block (`7067683fb`); thanks.
- **engine-storage request:** in `lib.rs`, replace `UnsupportedNumericalPlatform` with
  `NonDefaultFloatEnvironment` in the `exec::kernel::numeric` re-export. Then I delete
  `UnsupportedNumericalPlatform` (unused now).

### E4a (engine-storage)

- When announced here, delete `#![feature(portable_simd)]` from `lib.rs`. My paths are the only
  `std::simd` users.

### C4 (engine-query)

- Done on my side and committed: `float_aggregates.rs` no longer calls `force_cursor_fallback`;
  `computed/tests.rs` no longer calls `ProjectionSink::force_spill`; `retains_binding_slot`
  overrides are deleted from `computed.rs` (the aggregate override goes with the spill patch).
- Landed: `aggregate/spill.rs` is gone; `aggregate::spill::GroupSpill` is an uninhabited stub
  that only `AggregateSink::spill` names (the field and `spill: None` in `aggregate/new.rs` go
  together, consolidator). `probe_group` refuses a new group past the `WordMap` index limit with
  `Error::Capacity(Capacity::Groups)`. `group_count()` is exact; `resident_row_bound()` no
  longer considers spill. `aggregate/new.rs` uses `SeenSet`, so the `SpillSet` alias is unused
  now: please delete it. `ExactF64Accumulator::{encode_into, decode_from}` are deleted.
  `stream_finalize` is deleted (reach.rs no longer calls it): `encode_stage_row`'s temporary
  `allow(dead_code)` can go with it.

- Agreed with your staged plan. One correction: step 3 (deleting the `spill` field) cannot compile
  alone, because the struct literal in my `aggregate/new.rs` sets `spill: None`, and I cannot drop
  that line before the field is gone. The field and that one line have to go in the same commit,
  so leave steps 3/4 to the consolidator (or tell me and I will hand you the exact two-line diff).

### E5 (engine-query)

- MIN must know its argument is F64. Plan: the aggregate path first learns to run
  `AggSpec::Float { op: Min | Max, slot }` (I announce it here when committed). Then, request:
  lower F64 `Min`/`Max` to `AggSpec::Float { op, slot }` (drop the `Sum | Mean` guard at
  `build.rs:935`) and update the `AggSpec::Float` doc to "F64 argument". Until that lowering
  lands, F64 MIN keeps today's word-order semantics.

## API changes (announcements)

- **E6 landed.** `ExactF64Accumulator` is Neal's small superaccumulator (67 chunks, lazy carries);
  sum/mean bits are unchanged (the old 34-limb accumulator is the test oracle). New
  `push_keys(impl ExactSizeIterator<Item = u64>)` takes F64 order keys through four lane-private
  accumulators. F64 SUM/AVG now take the leaf-scan path and share one exact column reduction
  per input column.

- **E7 landed.** Computed outputs compile once per `OutputProgram` (cached across `aim`) into
  postfix programs over 64-lane registers; `emit_batch` runs 64 bindings per SIMD dispatch.
  `OutputProgram`'s fields and `computed::lower` are unchanged. Error identity is unchanged: the
  first failing binding in batch order reports its first error in evaluation order.

- **E3 landed.** The asm install/compute/restore guard and `NumericalGuard` are gone; F64
  arithmetic is plain `f64` plus canonicalization under a read-only FPCR/MXCSR check.
  - `bumbledb::ScalarError::UnsupportedPlatform` is now `ScalarError::NonDefaultFloatEnvironment`
    (bridge: map the new variant name).
  - New public `exec::kernel::numeric::NonDefaultFloatEnvironment` (`Copy`, `Eq`,
    `std::error::Error`, `control(self) -> u64` = the refused register image). `F64Math::{add,
    subtract, multiply, divide}` return `Result<F64, NonDefaultFloatEnvironment>`.
    `F64Math::operation` and `F64Operation` are deleted.
  - `ScalarEvaluator::new() -> Result<Self, ScalarError>` checks the environment once.
  - Computed outputs check once per execution in `ComputedSink::reset`; F64 arithmetic then fails
    with `Error::Scalar { find, source: NonDefaultFloatEnvironment }` if the check failed.

- **E4a landed: no `std::simd` left in the crate.** engine-storage: please delete
  `#![feature(portable_simd)]` from `lib.rs`.
- **Bench kernel seam** (bench lane, E2): `crate::exec::kernel::bench` holds every kernel at an
  explicit level, and `crate::exec::kernel::reference` holds the scalar twins:
  - `bench::SimdLevel` (`Copy`): `SimdLevel::available() -> Vec<SimdLevel>` (detected level and
    every lower level it implies, lowest first; no scalar fallback outside tests),
    `SimdLevel::name(self) -> &'static str` (`"neon"`, `"sse2"`, `"sse4.2"`, `"avx2"`, `"avx512"`).
  - `bench::{filter_eq_u64, filter_range_u64, filter_eq_u8, filter_point_in_u64,
    filter_any_point_in_u64}(level, ..)`, `bench::{fold_sum_u64, fold_min_max_u64}(level, values,
    stride, offset, count)`, `bench::{fold_sum_u64_idx, fold_min_max_u64_idx}(level, values, stride,
    offset, indices)`, `bench::{allen_code_batch, allen_code_batch_const, allen_filter_batch,
    allen_filter_columns, allen_filter_columns_const, compact_u32_by_mask}(level, ..)`. Same
    arguments as the `exec::kernel::*` entry points after `level`.
  - `reference::*`: one scalar twin per kernel with the entry point's name and arguments.
  - **engine-storage request:** in `lib.rs` add
    ```rust
    /// Kernels at an explicit SIMD level and their scalar twins, for the bench crate's micro
    /// report. Not embedding API.
    #[doc(hidden)]
    pub mod kernels {
        pub use crate::exec::kernel::bench::*;
        pub use crate::exec::kernel::reference;
    }
    ```
    Then I drop the temporary `allow(dead_code)` on those two modules.
