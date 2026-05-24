# PRD 08: Streaming Iteration And Batching

## Purpose

Make arena COLT iteration and batching paper-shaped and allocation-light.

## Required Work

- Implement `try_for_each_tuple` over arena `Range`, `Singleton`, `Offsets`, and `Map` nodes.
- Implement `fill_batch` without materializing all tuples first.
- Use reusable scratch tuple buffers for scalar iteration where lifetimes allow.
- Use bounded owned batch buffers only for vectorized execution.
- Ensure suffix vector iteration does not force maps.

## Passing Criteria

- No hot-path method returns `Vec<EncodedTuple>` except bounded batches.
- Suffix iteration over a range source allocates zero heap objects beyond bounded scratch setup.
- Batch fill over 1000 rows with batch size 4 allocates proportional to 4, not 1000.
- Existing scalar and vectorized executor tests pass on arena COLT.
- Global gates pass.
