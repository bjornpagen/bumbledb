# PRD 09: Base Image Columnar Layout

## Purpose

Replace allocation-heavy base-image columns with tight column-oriented buffers that match COLT's offset model and the paper's column-oriented premise.

## Current Problem

`ColumnImage` stores `values: Vec<Vec<u8>>`. Fixed-width values allocate per cell, which is hostile to scans, filtering, tuple construction, and NEON comparison.

## Required Design

- Store each loaded column as one contiguous byte buffer plus width.
- Expose zero-copy `value_at(offset) -> &[u8]`.
- Validate that all field widths are fixed and match schema width.
- Track loaded byte count in base-image spans.
- Keep row handles separate from column buffers.
- Use field IDs, not names, on hot paths.

## Suggested Shape

```rust
pub(crate) struct ColumnImage {
    pub(crate) field_id: usize,
    pub(crate) width: usize,
    pub(crate) values: Vec<u8>,
}

impl ColumnImage {
    pub(crate) fn value_at(&self, offset: usize) -> Option<&[u8]>;
    pub(crate) fn row_count(&self) -> usize;
}
```

## Required Breaking Changes

- Delete direct public/internal access to `Vec<Vec<u8>>` column values.
- Rewrite tuple construction, predicate filtering, planner stats, tests, and sink code to use `value_at`.
- Split files if any implementation file exceeds line limits.

## Passing Criteria

- Tests prove string and bytes columns still store 8-byte intern IDs.
- Tests prove contiguous columns return correct values by offset.
- Base-image load allocation count drops on a small fixture compared with the previous per-cell allocation pattern, measured by allocation profiler.
- JOB q09 exact output remains unchanged.
- Global acceptance from PRD 00 passes.
