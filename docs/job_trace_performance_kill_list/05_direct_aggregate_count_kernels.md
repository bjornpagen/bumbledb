# 05 Direct Aggregate Count Kernels

Priority: P1

## Problem

`job_movie_link_bridge` is now small and mostly optimized, but it still loses under trace:

- BumbleDB traced avg: `249us`
- SQLite traced avg: `149us`
- Result: one count row over only `36` counted bindings

The remaining overhead is generic Free Join/LFTJ and aggregate plumbing. The query is a fixed count-only bridge/path shape, and it should be served by a direct aggregate kernel.

## Technical Cause

Direct kernels currently reject aggregate outputs:

`crates/bumbledb-lmdb/src/query.rs:1873-1940`

```rust
fn try_direct_kernel(query: &NormalizedQuery) -> Option<DirectKernelPlan> {
    try_direct_prefix_range_kernel(query).or_else(|| try_direct_chain_kernel(query))
}

if query.atoms.len() < 2
    || !query.predicates.is_empty()
    || !matches!(query.output, OutputPlan::Project(_))
{
    return None;
}
```

So `job_movie_link_bridge` uses generic LFTJ:

- `trie_open=332`
- `trie_next=147`
- `trie_seek=309`
- `trie_key_reads=1715`
- `factorized_counted_bindings=36`

## Required Solution

Add direct count kernels for no-input aggregate bridge/path shapes.

### Target Shape

Start with `job_movie_link_bridge`:

- Central relation: `MovieLink(movie1, movie2, link_type)`.
- Existence/dimension checks: `LinkType`, `Title`, `CompanyName`, `InfoType`.
- Fanouts: `MovieCompanies(movie1)`, `MovieCompanies(movie2)`, `MovieInfoIdx(movie1)`, `MovieInfoIdx(movie2)`.
- Output: `count(?movie1)` with no grouping.

### Execution Strategy

1. Iterate `MovieLink` rows or prefixes.
2. For each pair `(movie1, movie2)`, count fanouts from relevant fact relations by prefix.
3. Multiply fanouts.
4. Accumulate count.
5. Emit one aggregate count row.

Use `HashTrieIndex::count(prefix)` or new sorted prefix count APIs.

## Strict Passing Criteria

- `job_movie_link_bridge` steady average drops from `~249us` traced and `~79us` untraced to below SQLite with at least `1.5x` margin.
- Runtime becomes `DirectKernel` or a new `DirectAggregate` runtime.
- `trie_open`, `trie_next`, `trie_seek`, and `trie_key_reads` drop by at least `80%` for the query.
- Output count remains exact.

## Tests

- Synthetic bridge count with multiple fanouts equals reference enumeration.
- Empty bridge returns no row under current aggregate/HAVING-like benchmark semantics if count is zero.
- Dimension existence filters are respected when present.

## Verification Commands

```sh
cargo test -p bumbledb-lmdb direct count --all-targets
cargo run -p bumbledb-bench --release -- --dataset job --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb --scale 10000 --warmup 2 --repeats 10 --query job_movie_link_bridge --format json
```
