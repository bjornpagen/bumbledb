# PRD 03: Query Image Flat Segment Decoding

## Status

Proposed.

## Motivation

The traced JOB run identified the single largest cold allocation cliff in the system:

- `job_broad_cast_keyword_company` first execution built a full query image.
- Query-image build took 899.1 ms.
- Query-image allocation was 32,721,565 allocation calls.
- Query-image allocated 2,591,326,860 bytes.
- Query-image retained 984,714,090 net bytes.

The immediate cause is not LMDB. Storage segments already store flat fixed-width encoded column bytes. The query-image builder expands those flat bytes into one heap `Vec<u8>` per cell, then converts them into typed fixed arrays.

This PRD replaces that representation with direct flat decoding into typed column arrays.

## Evidence

| Evidence | Location |
|---|---|
| Segment descriptors expose flat encoded column chunks | `crates/bumbledb-lmdb/src/storage_schema.rs:27-59` |
| Segment bytes are fetched from LMDB as one flat `Vec<u8>` | `crates/bumbledb-lmdb/src/storage.rs:1392-1397` |
| Query-image segment decode chunks bytes into per-cell `Vec<u8>` | `crates/bumbledb-lmdb/src/query_image.rs:773-781` |
| Then `from_query_image_bytes` converts each cell into fixed arrays | `crates/bumbledb-lmdb/src/query_image.rs:742-768` |
| Relation image build loads every field and every index | `crates/bumbledb-lmdb/src/query_image.rs:1026-1101` |
| Whole-schema image build iterates all relations | `crates/bumbledb-lmdb/src/query_image.rs:948-979` |
| Trace shows top relation image spans: CharName 320 ms, Name 216 ms, PersonInfo 129 ms | `docs/job-trace-analysis/01-job_broad_cast_keyword_company.md:68-96` |
| Trace shows query-image owns 99.741% alloc calls for the first query | `docs/job-trace-analysis/01-job_broad_cast_keyword_company.md:98-115` |

## Goals

- Replace segment column decoding with direct flat byte decoding into typed `ColumnImage` vectors.
- Replace current-index fallback column construction with `EncodedColumnBuilder` from PRD 02.
- Remove production use of `Vec<Vec<u8>>` for query-image columns.
- Preserve current full-schema image behavior for now. Scoped image loading is PRD 12.
- Keep query images owned and `Arc`-cached. Do not store borrowed LMDB slices in cached images in this PRD.

## Non-Goals

- Do not change query-image cache identity yet.
- Do not implement relation-scoped images yet.
- Do not change sorted index image representation yet except where tests require metadata cleanup.
- Do not pin read transactions to keep borrowed segment bytes alive.

## Current Code Map

| Component | Anchor | Current behavior |
|---|---|---|
| `QueryImageCache::get_or_build` | `query_image.rs:885-922` | Builds an `Arc<QueryImage>` by schema fingerprint and tx id |
| `QueryImageBuilder::build` | `query_image.rs:948-979` | Builds every relation in schema order |
| `RelationImageBuilder::build_from_segment` | `query_image.rs:1026-1122` | Loads flat segment bytes, then calls `ColumnImage::from_segment_bytes` |
| `ColumnImage::from_segment_bytes` | `query_image.rs:773-781` | Converts flat bytes into `Vec<Vec<u8>>` |
| `RelationImageBuilder::build_from_current_index` | `query_image.rs:1124-1185` | Uses `Vec<Vec<u8>>`, pushes `bytes.to_vec()`, then clones into `ColumnImage` |
| `ReadTxn::segment_bytes` | `storage.rs:1392-1397` | Clones an LMDB value into one owned `Vec<u8>` |

## Required Changes

### Step 1: Replace `ColumnImage::from_segment_bytes`

After PRD 02, rewrite `ColumnImage::from_segment_bytes` to use the builder:

```rust
pub(crate) fn from_segment_bytes(field: FieldId, width: usize, bytes: Vec<u8>) -> Result<Self> {
    if width == 0 || !bytes.len().is_multiple_of(width) {
        return Err(Error::corrupt("segment column byte width mismatch"));
    }
    let mut builder = EncodedColumnBuilder::with_capacity(field, width, bytes.len() / width)?;
    builder.extend_flat_bytes(&bytes)?;
    Ok(builder.finish())
}
```

This still copies LMDB bytes once into a flat `Vec<u8>` through `segment_bytes`, but it deletes the per-cell heap vector explosion.

### Step 2: Replace Current-Index Fallback Raw Columns

The fallback currently does:

```rust
let mut raw_columns = vec![Vec::<Vec<u8>>::new(); fields.len()];
...
raw_columns[field_id].push(bytes.to_vec());
...
raw_columns[field.id.0 as usize].clone()
```

Replace it with:

```rust
let mut builders = encoded_column_builders(&fields, estimated_row_count_or_zero)?;
...
builders[field_id].append_bytes(bytes)?;
...
let columns = builders.into_iter().map(EncodedColumnBuilder::finish).collect();
```

If row count is unknown before scanning current index, initialize with zero capacity and let vectors grow. Do not keep `Vec<Vec<u8>>`.

### Step 3: Preserve Diagnostics

`RelationStats.encoded_column_bytes` currently sums `ColumnImage::byte_len` at `query_image.rs:1102` and `1165`. This remains correct.

`QueryImageStats.encoded_column_bytes` currently sums relation image bytes at `query_image.rs:119-123`. This remains correct.

Do not change public stats meaning.

### Step 4: Remove Production `from_query_image_bytes`

After both segment and current-index fallback use builders, delete production calls to:

- `ColumnImage::from_bytes`
- `ColumnImage::from_query_image_bytes`

If tests need convenience, keep a `#[cfg(test)]` helper that appends through `EncodedColumnBuilder`.

## Optional But Encouraged In This PRD

### Add `ReadTxn::with_segment_bytes`

The current `segment_bytes` clone at `storage.rs:1392-1397` is one large allocation per column/index value. That is acceptable compared with per-cell allocations, but we can remove one copy for columns by decoding inside a closure:

```rust
pub(crate) fn with_segment_bytes<T>(&self, key: &[u8], f: impl FnOnce(&[u8]) -> Result<T>) -> Result<T> {
    let bytes = self
        .dbs
        .index
        .get(&self.txn, key)?
        .ok_or_else(|| Error::corrupt("segment bytes missing"))?;
    f(bytes)
}
```

Use it only to decode into owned `ColumnImage` values immediately. Do not store `&[u8]` in `QueryImage`.

This can eliminate the flat column `Vec<u8>` allocation too. It does not change index image bytes yet because `RelationIndexImage` currently owns `bytes: Vec<u8>`.

### Keep Index Bytes Owned

`RelationIndexImage` currently stores `bytes: Vec<u8>` at `query_image.rs:504-519`. Keep that for now. PRD 12/13 will decide whether scoped/direct-index APIs require a different shape.

## Acceptance Criteria

- Production query-image column construction does not allocate per cell.
- `grep` for `Vec::<Vec<u8>>::new()` in `query_image.rs` should find no production query-image path.
- `grep` for `.map(|chunk| chunk.to_vec())` in `query_image.rs` should be gone.
- Existing query-image tests pass.
- JOB query-image allocation calls for `job_broad_cast_keyword_company` drop dramatically.

## Tests

### Unit Tests

Add or update tests in `crates/bumbledb-lmdb/src/query_image.rs`:

- Segment-backed query image has same row count and values before/after flat builder.
- Segment byte length mismatch returns corrupt error.
- Current-index fallback builds equivalent columns without `Vec<Vec<u8>>`.
- Empty relation columns produce empty typed vectors.
- Bool, fixed8, fixed16 columns all work.

### Existing Tests

Run:

```sh
cargo test -p bumbledb-lmdb query_image
cargo test -p bumbledb-lmdb storage
cargo test --workspace --all-features
```

## Benchmark Plan

Run a traced allocation smoke for the first JOB query:

```sh
RUST_LOG="bumbledb_lmdb=debug" \
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_broad_cast_keyword_company \
  --trace --trace-format json \
  --trace-output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/job-image-flat-trace.jsonl
```

Gate:

- `allocations.phases.query_image.alloc_calls` should drop at least 95% from 32,721,565.
- `allocations.phases.query_image.bytes_allocated` should drop materially from 2.591 GB.
- `phase_timing.image_us` should drop materially from 899,106 us.
- Query output must remain identical.

## Risks

- If `extend_flat_bytes` is implemented by repeatedly slicing and `try_into`, CPU may still be noticeable, but allocations will be fixed. This is acceptable for this PRD.
- Removing `from_query_image_bytes` too early may break LFTJ until PRD 04. Coordinate order: either implement PRD 04 immediately or keep old helper temporary with a direct removal note.
- `with_segment_bytes` must not return borrowed data outside the closure.

## Future Follow-Ups

- PRD 12 will avoid building unrelated relation images.
- PRD 13 may use durable index bytes directly for LFTJ.
- A later storage PRD may store index images in typed component columns rather than concatenated bytes, but that is not required here.

## Definition Of Done

- Query-image segment columns decode without per-cell heap allocation.
- Current-index fallback columns build without per-cell heap allocation.
- Query-image correctness tests and JOB smoke benchmark pass.
- Trace allocation table proves the query-image allocation cliff is gone or reduced by at least 95% in call count.
