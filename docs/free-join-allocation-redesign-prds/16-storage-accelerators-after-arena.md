# PRD 16: Storage Accelerators After Arena

## Purpose

Introduce optional LMDB-backed accelerators only after COLT/source filtering prove the remaining scan cost is storage-facing.

## Required Preconditions

- Arena COLT complete.
- Source-filter pruning complete.
- No-trace benchmarks prove equality/range source filtering still dominates for selected queries.

## Required Direction

- Accelerators are correctness-optional.
- LMDB remains the only durable store.
- Writes update accelerators atomically with fact state.
- Duplicate inserts and absent deletes must not change accelerator state.
- Queries must produce identical results with accelerators enabled and disabled.

## Required Work

- Decide whether storage format bump is required.
- Add equality accelerator for fixed-width encoded values if justified.
- Add tests for insert, delete, duplicate insert, absent delete, and rollback behavior.
- Add query tests with accelerators enabled and disabled.

## Passing Criteria

- Storage format behavior is explicit and tested.
- Accelerator correctness is optional and test-proven.
- Exact JOB comparison passes with accelerators on and off.
- No-trace allocation and elapsed time justify the accelerator.
- Global gates pass.
