# PRD 11: Vectorized Arena Batches

## Purpose

Make vectorized execution consume real arena batches without materializing all source tuples and without allocating per tuple.

## Required Work

- Replace any vectorized path that owns `Vec<EncodedTuple>` per source tuple with a bounded `TupleBatch` over arena storage.
- Store batch bytes in reusable buffers where possible.
- Keep batch size explicit and measured.
- Keep scalar fallback behavior unchanged.
- Track batch count, tuple count, survivor count, and allocation counts.

## Required Tests

- Batch size 1 equals scalar output.
- Batch sizes 4, 16, and 1024 return identical sets.
- Partial final batch works.
- Empty source works.
- Duplicate witness output remains duplicate-free.
- A batch fixture proves allocation is bounded by batch size, not source cardinality.

## Passing Criteria

- No vectorized hot path calls a method that materializes all tuples before batching.
- No-trace allocation fixture shows vectorized batch allocation bounded by configured batch size.
- Existing vectorized tests pass.
- q09 exact SQLite comparison passes in scalar and vectorized modes if vectorized mode is exposed to tests.
- Global gates pass.
