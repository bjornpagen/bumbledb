# numeric lane board

Owns: `crates/bumbledb/src/{scalar.rs, exec/kernel.rs, exec/kernel/**, exec/sink/aggregate.rs,
exec/sink/aggregate/**, exec/sink/tests/aggregate.rs, api/prepared/computed.rs,
api/prepared/computed/**, api/prepared/tests/float_aggregates.rs}`, `tests/float_numerics.rs`.

Items: A/E1 (`gather_words`), E3, E4a, E4b, E5 (MIN/MAX), E6, E7, C4 (aggregate spill), bench
kernel seam, timing-pin removal.

## Status

| Item | Status |
|---|---|
| Timing pins and kernel experiment twins out of `cargo test` | in progress |
| A/E1: `gather_words` bounds | in progress |
| E4a: fearless_simd filter/fold/gather | todo |
| E4b: portable Allen, Avx2 compress | todo |
| E3: read-only FP environment check | todo |
| E5: MIN NaN propagation | todo (needs engine-query lowering, see request) |
| E6: xsum exact SUM/AVG | todo |
| E7: columnar computed outputs | todo |
| C4: aggregate spill deletion | todo (staged with engine-query, see their board) |
| Bench kernel seam | todo |

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

None yet.
