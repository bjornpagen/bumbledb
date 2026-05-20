# PRD 06: Relation Index Prefix Count API

## Status

Proposed.

## Motivation

Direct count kernels and static-empty proof paths repeatedly count durable index entries under an encoded prefix by iterating every matching entry:

```rust
index.entries_with_prefix(prefix).count()
```

This is correct but wasteful. `RelationIndexImage` owns a sorted concatenation of fixed-width encoded index entries. It can compute the matching half-open range with two binary searches and return the count directly.

This PRD adds exact prefix range and prefix count primitives.

## Evidence

| Trace/code finding | Anchor |
|---|---|
| `job_movie_link_bridge` performs 4,080 prefix-count probes per sample | `docs/job-trace-analysis/07-job_movie_link_bridge.md:81-95` |
| Movie bridge direct kernel calls `.entries_with_prefix(...).count()` four times per row | `crates/bumbledb-lmdb/src/query.rs:2842-2855` |
| Factorized count calls `.entries_with_prefix(...).count()` for every central value/index | `crates/bumbledb-lmdb/src/query.rs:2744-2761` |
| Static proofs use prefix iterators for existence/intersection checks | `crates/bumbledb-lmdb/src/query.rs:1921-1961`, `1964-2110` |
| Current prefix iterator only binary-searches lower bound and then scans until mismatch | `crates/bumbledb-lmdb/src/query_image.rs:554-610` |

## Goals

- Add exact prefix range and prefix count methods to `RelationIndexImage`.
- Replace direct-kernel `.entries_with_prefix(prefix).count()` calls with `prefix_count(prefix)`.
- Keep `entries_with_prefix` for code that actually needs entry bytes.
- Validate prefix bounds against existing iterator semantics.
- Prepare for direct durable-index trie sources in PRD 13.

## Non-Goals

- Do not change index byte layout in this PRD.
- Do not add prefix-count caches yet unless trivial after the API lands.
- Do not change direct-kernel eligibility or planning yet. That is PRD 07.
- Do not rewrite static proofs yet. They can opportunistically use the API where count/existence is enough.

## Current `RelationIndexImage`

`RelationIndexImage` stores:

```rust
pub struct RelationIndexImage {
    pub access: AccessId,
    pub fields: Vec<FieldId>,
    pub components: Vec<RelationIndexComponent>,
    pub encoded_len: usize,
    pub prefix_len: usize,
    pub bytes: Vec<u8>,
}
```

Anchor: `crates/bumbledb-lmdb/src/query_image.rs:504-519`.

Existing methods:

- `component_bytes`: `query_image.rs:545-552`.
- `entries_with_prefix`: `query_image.rs:554-575`.
- `entry`: `query_image.rs:577-580`.
- `entry_prefix`: `query_image.rs:582-584`.

The bytes are sorted by complete encoded index key. A prefix over leading components corresponds to a contiguous range.

## Proposed API

Add to `RelationIndexImage`:

```rust
pub fn prefix_range(&self, prefix: &[u8]) -> std::ops::Range<usize>;
pub fn prefix_count(&self, prefix: &[u8]) -> usize;
pub fn prefix_exists(&self, prefix: &[u8]) -> bool;
pub fn entry_at(&self, position: usize) -> Option<&[u8]>;
```

Definitions:

- `prefix_range` returns positions in entry units, not byte offsets.
- `prefix_count(prefix) == prefix_range(prefix).len()`.
- `prefix_exists(prefix) == prefix_count(prefix) > 0`.
- `entry_at` can either expose existing private `entry` or remain private if not needed.

## Algorithm

Implement two binary searches:

```rust
fn lower_bound_prefix(&self, prefix: &[u8]) -> usize;
fn upper_bound_prefix(&self, prefix: &[u8]) -> usize;
```

Simpler robust approach:

- `lower`: first entry whose entry prefix of `prefix.len()` is `>= prefix`.
- `upper`: first entry whose entry prefix of `prefix.len()` is `> prefix`.

Then:

```rust
let start = lower_bound_prefix(prefix);
let end = upper_bound_prefix(prefix);
start..end
```

This avoids constructing a synthetic successor prefix, which is error-prone for byte strings ending in `0xff`.

## Prefix Validation

Add debug or runtime validation for invalid prefix lengths.

A prefix is valid when:

- `prefix.len() <= encoded_len - prefix_len`.
- The prefix length is a sum of one or more leading component widths, or zero if callers explicitly want full range.

Current code can pass complete leading value prefixes. It should not pass partial field bytes.

Options:

- Strict runtime validation returning `Result<Range<usize>>`.
- Debug assertion plus private callers guarantee validity.

Prefer strict `Result` for public-ish `pub` methods if used outside query internals. The current `entries_with_prefix` returns an iterator directly, so a breaking change to `Result` may be invasive. Since this codebase is unstable, breaking it is acceptable if it improves correctness.

Recommended final shape:

```rust
pub fn prefix_range(&self, prefix: &[u8]) -> Result<std::ops::Range<usize>>;
pub fn prefix_count(&self, prefix: &[u8]) -> Result<usize>;
pub fn entries_with_prefix<'a>(&'a self, prefix: &'a [u8]) -> Result<RelationIndexPrefixIter<'a>>;
```

If this is too large for one PRD, add validation helper first but keep existing signature temporarily. Do not leave invalid prefixes silently accepted long term.

## Replacement Sites

### Factorized Count

Current:

```rust
let count = index.entries_with_prefix(&central_value).count() as u64;
```

Anchor: `crates/bumbledb-lmdb/src/query.rs:2748-2749`.

Replace with:

```rust
let count = index.prefix_count(&central_value)? as u64;
```

### Movie Link Bridge

Current:

```rust
let c1 = movie_companies_by_movie.entries_with_prefix(movie1).count() as u64;
let c2 = movie_companies_by_movie.entries_with_prefix(movie2).count() as u64;
let i1 = movie_info_by_movie.entries_with_prefix(movie1).count() as u64;
let i2 = movie_info_by_movie.entries_with_prefix(movie2).count() as u64;
```

Anchor: `crates/bumbledb-lmdb/src/query.rs:2851-2854`.

Replace all with `prefix_count`.

### Static Proofs

Use `prefix_exists` or `prefix_count` where code only needs existence.

Examples:

- `company_primary.entries_with_prefix(company_bytes).any(...)` still needs entry payload to inspect country code, so keep iterator.
- Loops that only check empty/non-empty can use `prefix_exists`.

Do not contort proof code in this PRD; use obvious replacements only.

## Tests

Add tests in `crates/bumbledb-lmdb/src/query_image.rs`:

- Empty index prefix count is zero.
- Prefix matching one entry returns one.
- Prefix matching many contiguous entries returns all.
- Prefix before first entry returns zero.
- Prefix after last entry returns zero.
- Prefix between groups returns zero.
- Prefix count equals `entries_with_prefix(prefix).count()` over generated fixture indexes.
- Empty prefix returns full entry count if supported.
- Invalid partial-field prefix returns error if strict validation is implemented.

Add direct-kernel tests if existing query tests can target `movie_link_bridge_count` or factorized count.

## Benchmark Gates

After replacing direct count probes:

- `job_movie_link_bridge` sample `bumbledb.avg_us` should drop from 924 us or at least not regress.
- `job_movie_link_bridge` still reports `direct_kernel_probes = 4080` if probe counter remains semantic. The counter may continue counting logical probes even if implementation uses O(log n) range count.
- `job_broad_movie_info_star` sample avg should improve or not regress.
- Correct counts remain unchanged:
  - `job_broad_movie_info_star` output rows: 1.
  - `job_movie_link_bridge` output rows: 1.
  - `direct_kernel_rows` for movie link remains 149,301 from the traced run scale.

Run:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_broad_movie_info_star \
  --query job_movie_link_bridge
```

## Risks

- Incorrect upper-bound implementation can undercount or overcount joins. Always cross-check against iterator count in tests.
- Prefix length validation can break callers that currently pass partial bytes. That is desirable if those callers are wrong, but tests must expose it early.
- Counting positions in entries, not byte offsets, must be documented and tested.

## Future Follow-Ups

- PRD 07 uses this API in early direct count plans.
- PRD 13 uses prefix ranges to create direct durable-index trie adapters.
- A later cache PRD can memoize prefix counts for repeated movie IDs in bridge queries.

## Definition Of Done

- `RelationIndexImage` exposes tested exact prefix range/count methods.
- Direct count kernels use `prefix_count` instead of iterator `.count()`.
- Prefix-count tests prove equivalence with iterator semantics.
- JOB direct count queries remain correct and do not regress.
