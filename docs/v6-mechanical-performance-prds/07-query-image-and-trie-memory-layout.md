# PRD 07: Query Image And Trie Memory Layout

## Goal

Improve query image and sorted trie memory locality if profiling proves layout is a bottleneck after output/direct/LFTJ mechanics are optimized.

This PRD is allowed to make ambitious breaking memory-layout changes.

## Background

The heavy traces did not identify query image layout as the first query bottleneck for high-output non-JOB materialized queries. But query image and trie structures remain central:

- static proof reads relation images
- LFTJ builds sorted tries over encoded columns
- direct kernels use query image/hash trie structures
- JOB q16/q24/q33 rely on query-image static proof

After PRD 03-06 reduce per-binding overhead, layout/cache locality may become the next bottleneck.

## Explicit Non-Goals

- No backwards compatibility.
- No migrations.
- No old segment readers.
- No dual layout support.
- No preserving current internal storage image APIs.
- No changing logical schema semantics.
- No changing encoded ordering semantics.

## Code Anchors

Expected areas:

```text
query_image.rs
EncodedColumnBuilder
RelationImage
RelationIndexImage
FixedColumn
SortedTrieIndex
sorted_trie.rs
hash_trie.rs where retained for direct kernels
storage_schema.rs
```

## Required Measurement Before Layout Change

Use PRD 02 artifacts. Do not change layout unless at least one is true:

- cache/hardware counters show query image or trie misses dominate hot query runtime
- allocation profiles show query image/trie construction allocates heavily
- benchmark counters show LFTJ build/trie traversal dominates after PRD 03-06
- flamegraph shows `RelationImage`, `FixedColumn`, `SortedTrieIndex`, or encoded column access as top hot path

If none are true, document and delete this PRD as a no-change decision.

## Candidate Layout Improvements

### 1. Structure-of-Arrays Query Image

Ensure each relation image stores columns as compact typed/width-homogeneous arrays:

```text
width 1 columns: Vec<u8>
width 8 columns: Vec<[u8; 8]> or aligned Vec<u64-order-bytes>
width 16 columns: Vec<[u8; 16]>
```

Avoid per-value enum wrappers in column hot paths.

### 2. Access-Path Key Slab

For relation indexes, store keys in contiguous slabs:

```text
key_bytes: Vec<u8>
row_ids: Vec<RowId>
key_width: usize
component_offsets: Vec<usize>
```

Avoid `Vec<Vec<u8>>` or per-key allocation.

### 3. Sorted Trie Level Layout

Represent trie levels as contiguous arrays:

```text
keys: Vec<u8> or width-specialized key array
child_start: Vec<u32>
child_len: Vec<u32>
row_start: Vec<u32>
row_len: Vec<u32>
```

The goal is fast sequential scans and fewer pointer jumps.

### 4. Row ID Compression

Use `u32` row IDs internally where already true. Consider delta or range representation only if counters prove row-list memory dominates.

Do not bit-pack prematurely.

### 5. Alignment

Consider aligning width 8 arrays for vectorization. Do not add unsafe alignment assumptions without tests.

## Required Tests

- Query image build is deterministic.
- Reopened query image remains correct if durable segments are affected.
- All encoded columns decode to same public rows.
- Sorted trie iteration semantics match old tests.
- Prefix/range scans match iterator/reference implementation.
- Static proof q16/q24 remains correct.
- LFTJ joins remain correct across width 1, 8, and 16 fields.
- Fuzz/property tests pass.

## Required Benchmarks

Run:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-layout-nonjob.json
```

Run JOB:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-layout-job-10k.json
```

Also run targeted query-image/static-proof queries:

```text
job_q16_character_title_us
job_q24_voice_keyword_actor
job_q33_linked_series_companies
triangle_count
red_boat_sailors
```

## Performance Targets

Hard gates:

- all existing gates pass

Optimization targets depend on measured bottleneck. At least one must improve if layout work proceeds:

- query image build time improves by at least 20%
- LFTJ build time improves by at least 15%
- static proof q16/q24 improves by at least 10%
- memory allocated by query image/trie build drops by at least 20%

If no target improves, revert or document why the new layout is still simpler enough to keep.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- non-JOB gates pass
- JOB 10k gates pass
- memory/layout RCA documents measured improvement or rejection

## Completion Criteria

- Query image/trie layout is either improved or explicitly defended as not-yet-bottleneck.
- No dual old/new layout remains.
- This PRD is deleted and committed after passing.
