# 08 LFTJ Build Subphase Tracing

Priority: P1

## Problem

The post-kill trace shows `lftj.build` is still a large cold bottleneck for q16/q24/q09, but the trace does not separate scan/filter/copy, temporary column construction, cache lookup, and final sorted trie build.

Examples:

- `q16`: `lftj.build = 27.3ms`, nested `sorted_trie.build = 2.49ms`.
- `q24`: `lftj.build = 12.8ms`, nested `sorted_trie.build = 208us`.

Most of the time is hidden inside `build_lftj_sorted_trie` before the final sort.

## Technical Cause

`build_lftj_sorted_trie` has one coarse caller span and no internal spans:

`crates/bumbledb-lmdb/src/query.rs:3637-3702`

```rust
let mut raw_columns = vec![Vec::<Vec<u8>>::new(); variables.len()];
...
for row in 0..source.row_count { ... }
...
let columns = fields.iter().zip(raw_columns).map(...).collect()?;
...
let trie = crate::query_image::build_sorted_trie_index(...)?;
```

## Required Solution

Add subphase spans and counters:

- `bumbledb.query.lftj.build.scan_filter_copy`
- `bumbledb.query.lftj.build.column_image`
- `bumbledb.query.lftj.build.sorted_trie`
- `bumbledb.query.lftj.build.cache_lookup`

Add counters:

```rust
lftj_atom_source_rows_scanned
lftj_atom_rows_retained
lftj_atom_bytes_copied
lftj_atom_scan_micros
lftj_atom_column_micros
lftj_atom_sort_micros
```

## Strict Passing Criteria

- A traced q16/q24 run attributes at least `95%` of `lftj.build` time to named subspans.
- Benchmark JSON or explain output exposes row/byte/micros counters.
- The data clearly identifies whether q16/q24 remaining build waste is scan/filter/copy, column construction, or sort.

## Tests

- Unit test with one filtered atom reports source rows, retained rows, and copied bytes.
- Trace smoke test can match the new span names.
- Existing benchmark JSON renderer includes new counters or gracefully omits them when zero.

## Verification Commands

```sh
cargo test -p bumbledb-lmdb lftj --all-targets
RUST_LOG='bumbledb_lmdb=trace,bumbledb_bench=debug' cargo run -p bumbledb-bench --release -- --dataset job --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb --scale 10000 --warmup 0 --repeats 1 --query job_q16_character_title_us --trace --trace-format json --format json
```
