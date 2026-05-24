# PRD 14: Vectorized Free Join Execution

## Purpose

Implement the paper's vectorized Free Join algorithm: batch cover iteration, batch sibling probes, survivor compaction, and controlled recursion after all probes for a batch complete.

## Dependencies

- PRD 12.
- PRD 13.

## Scope

- `iter_batch(batch_size)` for GHT/COLT sources.
- Batch key construction.
- Batch probing or efficient scalar probing inside a batch abstraction.
- Survivor compaction.
- Batch-size configuration and counters.
- Scalar batch size 1 equivalence.
- Preservation of the private execution sink boundary introduced by PRD 12.

## Required Behavior

- Batch size 1 is exactly equivalent to scalar PRD 12 execution.
- Vectorized execution must preserve the same LMDB read snapshot as scalar execution for the full query.
- A cover source can yield up to `batch_size` cover tuples.
- For each probe subatom, construct keys for surviving batch tuples.
- Probe all surviving tuples for that subatom before moving to the next subatom.
- Remove failed tuples from the survivor set.
- After all probes succeed for survivors, recurse for each survivor with its child source set.
- Final partial batches work.
- Empty batches and all-failed batches work.

## Technical Direction

- Add `ExecutionMode::Scalar` and `ExecutionMode::Vectorized { batch_size }` or equivalent internal config.
- Start with scalar `get` in a batch loop if implementing true `lookup_batch` is too large. The algorithmic batch boundary and survivor compaction are mandatory.
- Add a later optimization point for true batched hash lookup/seek.
- Keep batch buffers reusable to avoid excessive allocation.
- Do not change public `QueryResultSet` semantics.
- Do not bypass the PRD 12 sink/fold boundary by materializing vectorized batches directly into public rows.

## Non-Goals

- Do not implement SIMD.
- Do not expose vectorized execution as a stable public API.
- Do not remove scalar mode.
- Do not introduce separate storage reads outside the active LMDB-backed source structures.

## Acceptance Criteria

- Batch sizes 1, 10, 100, and 1000 return identical duplicate-free result sets.
- Survivor compaction handles first-probe, middle-probe, and last-probe failures.
- Empty result queries do not recurse incorrectly.
- Vectorized mode records batch input, survivor, failed, and probe counters.
- Existing correctness suites pass in vectorized mode.
- Benchmarks can select batch size for ablations.
- Vectorized mode feeds the same sink interface as scalar mode, preserving future aggregate/factorized consumers.

## Required Tests

- Scalar versus batch size 1 equivalence.
- Scalar versus batch size 10/100/1000 equivalence.
- All probes succeed.
- Some probes fail.
- All probes fail.
- Final partial batch.
- Empty relation.
- Duplicate witnesses still deduplicate output.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb vectorized --all-features
cargo test --workspace --all-features
```
