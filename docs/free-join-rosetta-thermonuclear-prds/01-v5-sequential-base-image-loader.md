# PRD 01: V5 Sequential Base Image Loader

## Purpose

Replace the current per-cell LMDB point-get base-image loader with sequential prefix scans over the existing v5 column namespace.

This is the first code PRD because `BaseImageLoad` consumes about 235 ms of the 292 ms traced execution root time.

## Rosetta Alignment

This preserves LMDB as the only durable backend and preserves snapshot-local reads. It changes only how the engine reads v5 storage keys inside one LMDB read transaction.

## Paper Alignment

The Free Join paper assumes relations are available as column-oriented vectors before COLT operates over offsets. This PRD adapts that assumption to LMDB by reading the existing column namespace as sequential column streams instead of random per-cell lookups.

Relevant paper direction:

- COLT raw data is stored column-wise.
- COLT offsets point into base relation columns.
- Disk-backed repeated random access is a known risk.

## Current Problem

`load_relation_base_image` currently does:

```text
row_handles = scan L | relation
for each field:
  allocate values buffer
  for each row handle:
    get C | relation | field | handle
    append bytes
```

This creates one LMDB point lookup per row per requested field.

On the current JOB sample trace, only 5.99 MB of encoded column data creates 749,510 column value loads and 235 ms of base-image time.

## Required Design

Keep v5 storage format unchanged.

Read live handles once using the existing `L | relation | handle` prefix.

For each requested field, scan the column prefix:

```text
C | relation_id | field_id | handle
```

Build the contiguous `ColumnImage::values` buffer from that sequential iterator.

Validate exact alignment against live rows:

- Every live handle must have exactly one column entry for each requested field.
- Every column entry under the prefix must correspond to a live handle.
- Column entries must have exact fixed encoded width.
- The final `ColumnImage` row order must match `RelationBaseImage::row_handles`.

Do not call `data.get(column_key(...))` inside a loop over row handles.

## Suggested Implementation Direction

Add storage-format helpers:

```rust
column_prefix_key(relation_id, field_id) -> StorageKey
decode_column_key_handle(key: &[u8]) -> Result<FactHandle>
```

In `base_image.rs`, replace the nested point-get loop with a merge between sorted live handles and sorted column-prefix entries.

Use the fact that both `live_row_key` and `column_key` are ordered by `FactHandle` after their prefixes.

Reject corruption rather than silently skipping anything.

Keep the public `RelationBaseImage` shape stable for this PRD unless a smaller internal helper is needed.

## Tests Required

- Existing base-image tests still pass.
- Add a test with multiple fields and multiple rows proving prefix-scan loading preserves row/column alignment.
- Add a corruption test for a missing column entry under a live row.
- Add a corruption test for an extra column entry whose handle is not live.
- Add a corruption test for wrong encoded width.
- Add an allocation/trace test proving base-image loading does not allocate per cell.

## Trace Requirements

Add or preserve counters:

- live rows scanned
- column prefix scans performed
- column entries scanned
- column values loaded
- loaded bytes
- missing column entries
- extra column entries

If adding new counters is too invasive for this PRD, at minimum preserve existing counters and add tests that prove point-get removal by behavior and code search.

## Search Gates

These must pass after the implementation:

```bash
rg "column_key\(" crates/bumbledb-lmdb/src/base_image.rs
rg "for handle in &row_handles" crates/bumbledb-lmdb/src/base_image.rs
```

The first search may find tests or helper construction, but must not find a per-row point-get loop.

## Benchmark Passing Criteria

Run the full traced JOB sample.

Required improvements against the suite baseline in `README.md`:

- Total `BaseImageLoad` elapsed must drop by at least 40% from 235.06 ms.
- Exact SQLite comparisons must pass for all 8 JOB sample queries.
- `loaded_bytes` may remain similar because this PRD changes access shape, not pruning.
- `binding_copies` must remain 0.
- No increase in public result rows unless the prior result was wrong.

If the 40% target is missed, the PRD is incomplete unless the trace proves `BaseImageLoad` is no longer dominated by column entry access.
