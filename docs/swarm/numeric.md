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
| E3: read-only FP environment check | todo |
| E5: MIN NaN propagation | todo (needs engine-query lowering, see request) |
| E6: xsum exact SUM/AVG | todo |
| E7: columnar computed outputs | todo |
| C4: aggregate spill deletion | todo (staged with engine-query, see their board) |
| Bench kernel seam | landed in-crate; needs the `lib.rs` re-export below |

## Plans that affect other lanes

### E3 (engine-query, engine-storage)

- The check lives inside the computed-output sink: `ComputedSink::reset` reads FPCR/MXCSR once per
  execution, only when a program does F64 arithmetic, and records
  `Error::Scalar { find, source: ScalarError::NonDefaultFloatEnvironment }` (sticky; finalize
  refuses). Nothing in `execute.rs` needs to call it.
- **engine-query:** you can delete the guard block in `api/prepared/execute.rs` (`_numeric_guard`)
  and the `numeric_outputs` field and its computation in `build.rs` **now**; deleting them compiles
  against today's HEAD. Once that lands I delete `NumericalGuard` and rename
  `ScalarError::UnsupportedPlatform` to `ScalarError::NonDefaultFloatEnvironment`.
- **engine-storage:** after I announce `NonDefaultFloatEnvironment` here, swap the `lib.rs`
  re-export `UnsupportedNumericalPlatform` for `NonDefaultFloatEnvironment`; then I delete the old
  type.

### E4a (engine-storage)

- When announced here, delete `#![feature(portable_simd)]` from `lib.rs`. My paths are the only
  `std::simd` users.

### C4 (engine-query)

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
