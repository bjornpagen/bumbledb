# PRD 03: Query-Local Column Cache And Image Views

## Purpose

Stop reloading the same relation columns repeatedly for different atom occurrences and field scopes in one query.

## Rosetta Alignment

The cache remains snapshot-local to `ReadTxn`. It must not cross storage transaction IDs or LMDB read snapshots.

## Paper Alignment

Free Join represents each atom occurrence as a source, but self-joins and repeated relation references do not require physically rereading identical base columns. Sharing physical column vectors while keeping logical atom sources distinct preserves the GHT/COLT abstraction.

## Current Problem

The current base-image cache key is:

```text
schema fingerprint + storage tx id + relation id + exact field scope
```

That means the same relation is reloaded for different scopes.

Trace examples:

- `Title` loaded 9 times for 74.28 ms.
- `CompanyName` loaded 7 times for 50.70 ms.
- `Keyword` loaded 4 times for 28.29 ms.
- `Name` loaded 3 times for 23.05 ms.

## Required Design

Split physical storage from logical source view.

Physical cache:

```text
snapshot + relation id + field id -> ColumnImage
snapshot + relation id -> live row handles
```

Logical image view:

```text
relation id
relation name
shared row handles or survivor handle view
requested field slots pointing to shared columns
row selection/remap if filtered
```

Atom occurrences remain distinct even when they share physical columns.

## Required Behavior

- Loading `Title(id)` and then `Title(id, production_year)` in the same read transaction must reuse `Title(id)`.
- Loading two different atom occurrences over `CompanyName(country_code)` must reuse the same physical column.
- A filtered atom may have a different survivor view from an unfiltered atom, but must share the underlying filter column if already loaded.
- Self-joins must remain logically independent. Source replacement and binding must not alias runtime state incorrectly.

## Suggested Implementation Direction

Replace exact-scope `BaseImageCacheEntry` with per-relation cache entries.

Use compact field slots instead of `BTreeMap<usize, ColumnImage>` in the hot image view if feasible.

Keep the public/private API small:

```rust
RelationColumnCache
RelationImageView
ColumnRef
RowSelection
```

Names may differ, but the separation must exist.

## Tests Required

- Same read transaction, same relation, overlapping scopes: shared column pointer or explicit trace cache hit.
- Same read transaction, same relation, disjoint scopes: live row handles loaded once.
- Different read transactions: no pointer sharing unless snapshot identity and storage transaction ID are proven safe.
- Self-join query returns the same exact result before and after sharing physical columns.
- Filtered and unfiltered atom occurrences can coexist over the same relation.

## Trace Requirements

Add counters:

- relation handle cache hits/misses
- column cache hits/misses
- image view creations
- physical column loads avoided

Existing `BaseImageCacheLookup` must become meaningful for partial field reuse, not just exact-scope reuse.

## Benchmark Passing Criteria

Run the full traced JOB sample.

Required improvements against post-PRD-02 baseline:

- Total `BaseImageLoad` for repeated relations must drop materially.
- `Title`, `CompanyName`, `Keyword`, and `Name` must show physical column cache hits when repeated.
- Exact SQLite comparisons pass for all 8 JOB sample queries.
- No cross-query/process-global cache is introduced.
