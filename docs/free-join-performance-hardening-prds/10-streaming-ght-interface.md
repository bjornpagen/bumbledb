# PRD 10: Streaming GHT Interface

## Purpose

Make Bumbledb's GHT interface match the Free Join paper's iterator model instead of eager vector materialization.

## Current Problem

`GhtSource::iter` returns `Vec<EncodedTuple>`, and `iter_batch` chunks that vector. This defeats COLT laziness, allocates heavily, and prevents true vectorized execution.

## Required Design

- Replace eager `iter` with streaming iteration.
- Replace `iter_batch` with true bounded batch production.
- Avoid allocating one `EncodedTuple` per source tuple unless a downstream operation needs ownership.
- Keep tuple bytes fixed-width and reusable where possible.

## Suggested Shape

Use one of these designs:

```rust
fn for_each_tuple(&self, f: impl FnMut(EncodedTupleRef<'_>) -> Result<ControlFlow>);
fn fill_batch(&self, batch: &mut TupleBatch) -> BatchStatus;
```

or an explicit iterator type with lifetimes if it stays ergonomic.

## Required Migration

- Update COLT root/vector iteration.
- Update scalar executor cover loops.
- Update vectorized executor batch loops.
- Update tests that compare `Vec<EncodedTuple>` to collect explicitly outside hot code.
- Update counters to count yielded tuples and batches.

## Passing Criteria

- No hot-path trait method named `iter` returns `Vec<EncodedTuple>`.
- `rg "fn iter\(&self\) -> Vec<EncodedTuple>|iter_batch.*collect|chunks\(batch_size\)" crates/bumbledb-lmdb/src` returns no hot-path matches.
- A test proves batch size limits are respected without materializing all source tuples first.
- Allocation trace for a small join shows fewer tuple allocations in cover iteration.
- Global acceptance from PRD 00 passes.
