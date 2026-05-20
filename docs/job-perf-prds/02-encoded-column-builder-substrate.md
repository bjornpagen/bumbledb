# PRD 02: Encoded Column Builder Substrate

## Status

Proposed.

## Motivation

The largest allocation cliffs all come from the same bad representation: fixed-width encoded values are repeatedly stored as heap-owned `Vec<u8>` cells.

Fixed-width encoded values already have only three legal widths in query execution:

- 1 byte for bool-like values.
- 8 bytes for `u64`, IDs, refs, timestamps, interned strings/bytes, enum/code domains.
- 16 bytes for decimals and UUIDs.

The engine should not allocate one heap object per encoded cell. It should append directly into typed vectors of fixed arrays.

This PRD creates the shared substrate used by later PRDs:

- Query-image flat segment decoding.
- LFTJ atom temporary relation builds.
- Indexed-prefix streaming builders.
- Future durable-index-backed trie sources.

## Evidence

| Trace finding | Representation root cause |
|---|---|
| `job_broad_cast_keyword_company`: 32.72M query-image allocation calls | `ColumnImage::from_segment_bytes` creates `Vec<Vec<u8>>` before typed vectors |
| q09: 16.58M `lftj_build` allocation calls and 2.649 GB allocated | `build_lftj_sorted_trie` creates `Vec<Vec<u8>>` raw columns |
| q24: 920k `lftj_build` allocation calls and 107.7 MB allocated | Same LFTJ temporary row/column representation |
| q09/q24 `scan_filter_copy` dominates cold build | Atom extraction copies every retained field into heap `Vec<u8>` |

## Current Code Anchors

| Code | Current problem |
|---|---|
| `crates/bumbledb-lmdb/src/query_image.rs:737-781` | `ColumnImage` accepts `Vec<Vec<u8>>`; segment bytes are chunked into heap vectors |
| `crates/bumbledb-lmdb/src/query_image.rs:1124-1164` | Current-index fallback builds `raw_columns: Vec<Vec<Vec<u8>>>` and clones it into `ColumnImage` |
| `crates/bumbledb-lmdb/src/query.rs:5488-5559` | LFTJ atom build uses `raw_columns = vec![Vec::<Vec<u8>>::new(); variables.len()]` |
| `crates/bumbledb-lmdb/src/query.rs:5681-5729` | Indexed atom extraction returns `Vec<Vec<u8>>` |
| `crates/bumbledb-lmdb/src/query.rs:5835-5884` | Row atom extraction returns `Vec<Vec<u8>>` |
| `crates/bumbledb-lmdb/src/sorted_trie.rs:5-43` | `EncodedOwned` already models fixed-width scalar ownership |
| `crates/bumbledb-lmdb/src/query_image.rs:68-89` | `EncodedRef` already models fixed-width borrowed scalar references |

## Goals

- Introduce a single typed builder abstraction for fixed-width encoded columns.
- Make all code that needs temporary encoded columns append `[u8; N]`, not `Vec<u8>`.
- Accept both flat byte slices and per-value byte slices without per-cell heap allocation.
- Provide clear corrupt-width validation.
- Make the builder output existing `ColumnImage` without a conversion through `Vec<Vec<u8>>`.
- Keep the first implementation owned and simple. Do not store borrowed LMDB slices in cached images.

## Non-Goals

- Do not implement query-image scoping in this PRD.
- Do not implement LFTJ streaming in this PRD.
- Do not optimize trie traversal in this PRD.
- Do not add compatibility wrappers that keep accepting `Vec<Vec<u8>>` indefinitely.

## Proposed Types

Add these near `ColumnImage` in `crates/bumbledb-lmdb/src/query_image.rs` or in a small new internal module if the file becomes unwieldy.

```rust
pub(crate) enum EncodedColumnBuilder {
    Bool {
        field: FieldId,
        values: Vec<[u8; 1]>,
    },
    Fixed8 {
        field: FieldId,
        values: Vec<[u8; 8]>,
    },
    Fixed16 {
        field: FieldId,
        values: Vec<[u8; 16]>,
    },
}
```

Required methods:

```rust
impl EncodedColumnBuilder {
    pub(crate) fn new(field: FieldId, width: usize) -> Result<Self>;
    pub(crate) fn with_capacity(field: FieldId, width: usize, capacity: usize) -> Result<Self>;
    pub(crate) fn append_bytes(&mut self, bytes: &[u8]) -> Result<()>;
    pub(crate) fn extend_flat_bytes(&mut self, bytes: &[u8]) -> Result<()>;
    pub(crate) fn len(&self) -> usize;
    pub(crate) fn is_empty(&self) -> bool;
    pub(crate) fn byte_len(&self) -> usize;
    pub(crate) fn finish(self) -> ColumnImage;
}
```

Design details:

- `append_bytes` must copy into `[u8; N]`, not allocate a `Vec<u8>`.
- `extend_flat_bytes` must validate `bytes.len().is_multiple_of(width)` and then push typed arrays directly.
- `finish` consumes the builder and returns `ColumnImage::Bool`, `ColumnImage::Fixed8`, or `ColumnImage::Fixed16`.
- Unsupported widths must return `Error::internal` or `Error::corrupt` depending on caller context.
- Capacity should be row count where known.

## Required Deletions

Delete or make private-to-tests only:

- `ColumnImage::from_bytes(field, width, values: Vec<Vec<u8>>)`.
- `ColumnImage::from_query_image_bytes(field, width, values: Vec<Vec<u8>>)` as a production path.

If tests still need fixture convenience, add a narrow test-only helper that internally uses `EncodedColumnBuilder`.

No long-term dual path.

## Implementation Plan

### Step 1: Add Builder And Unit Tests

Add builder with unit tests for:

- Width 1 append and flat extend.
- Width 8 append and flat extend.
- Width 16 append and flat extend.
- Invalid width rejection.
- Flat byte length mismatch rejection.
- `finish` returns the expected `ColumnImage` variant.
- `byte_len` equals `len * width`.

Suggested test location:

- `crates/bumbledb-lmdb/src/query_image.rs` tests near existing query image tests.

### Step 2: Add `ColumnImage::from_flat_bytes`

Add a replacement constructor:

```rust
impl ColumnImage {
    pub(crate) fn from_flat_bytes(field: FieldId, width: usize, bytes: &[u8]) -> Result<Self> {
        let mut builder = EncodedColumnBuilder::with_capacity(field, width, bytes.len() / width)?;
        builder.extend_flat_bytes(bytes)?;
        Ok(builder.finish())
    }
}
```

If ownership of `Vec<u8>` is important, accept `Vec<u8>` by value and pass `&bytes`; the main win is eliminating per-cell heap vectors. Later PRDs may remove the segment `Vec<u8>` copy itself.

### Step 3: Add Builder Accessors For LFTJ

LFTJ will need a vector of builders. Provide ergonomic helpers:

```rust
pub(crate) fn encoded_column_builders(fields: &[FieldImage], capacity: usize) -> Result<Vec<EncodedColumnBuilder>>;
pub(crate) fn finish_column_builders(builders: Vec<EncodedColumnBuilder>) -> Vec<ColumnImage>;
```

Do not over-abstract. The helper should just make PRDs 03 and 04 smaller.

### Step 4: Remove Production `Vec<Vec<u8>>` Constructors

After PRDs 03 and 04 consume the builder, delete the old constructors. If this PRD is implemented before consumers, mark old constructors as temporary with a direct TODO pointing to PRDs 03 and 04, but do not leave them after PRD 04.

## Invariants

- Fixed-width bytes are copied exactly once into typed arrays.
- `ColumnImage::len()` must match row count for every column in a relation image.
- Field ID on the builder must become the field ID on the resulting `ColumnImage`.
- Encoded byte ordering must remain unchanged.
- No logical decoding happens in the builder.
- Builder must not own `ValueType`; width is sufficient for raw encoded storage.

## Error Handling

| Situation | Error |
|---|---|
| Unsupported width from schema metadata | `Error::internal("unsupported column width {width}")` |
| Segment byte length not divisible by width | `Error::corrupt("segment column byte width mismatch")` |
| Append byte slice length mismatch | `Error::corrupt("query image column width mismatch")` or a new precise message |

## Testing Plan

### Unit Tests

Add focused tests in `query_image.rs`:

- `encoded_column_builder_appends_width_1`.
- `encoded_column_builder_appends_width_8`.
- `encoded_column_builder_appends_width_16`.
- `encoded_column_builder_extends_flat_bytes`.
- `encoded_column_builder_rejects_bad_width`.
- `encoded_column_builder_rejects_bad_flat_length`.

### Existing Tests That Must Still Pass

- `builds_query_image_from_snapshot_and_matches_diagnostics` in `query_image.rs`.
- `query_image_uses_durable_segments_after_bulk_load` if present in current tests.
- `cargo test -p bumbledb-lmdb query_image`.

## Benchmark Gates

This PRD alone may not change benchmark output until PRDs 03 and 04 use the builder. After PRD 03:

- `job_broad_cast_keyword_company` query-image allocation calls must drop by at least 95% from 32.72M.

After PRD 04:

- q09 `lftj_build` allocation calls must drop by at least 80% from 16.58M.
- q24 `lftj_build` allocation calls must drop by at least 80% from 920k.

## Risks

- A rushed builder can accidentally accept partial encoded values. Length validation must be strict.
- Confusing logical width with physical width can break decimal/uuid paths. Always use `ValueType::encoded_width()` from descriptors.
- Keeping old constructors around after migration will invite future code to reintroduce `Vec<Vec<u8>>`.

## Definition Of Done

- `EncodedColumnBuilder` exists and is tested.
- There is a direct flat-byte-to-`ColumnImage` path.
- No new production code should be written against `Vec<Vec<u8>>` encoded columns.
- Follow-up PRDs can replace query-image and LFTJ raw columns without inventing another builder abstraction.
