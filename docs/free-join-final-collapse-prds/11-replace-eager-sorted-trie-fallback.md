# PRD 11: Replace Eager Sorted Trie Fallback

## Status

Not started.

## Current State

`query/lftj_access.rs` first tries `lazy_lftj_access_slice`, then falls back to cached/eager `SortedTrieIndex` construction. Lazy access currently rejects important shapes:

- atoms with more than two variables
- atoms with local comparison predicates
- repeated variables inside one atom
- shapes not covered by one durable access path in the required order

Fallback builds are visible through `build_lftj_sorted_trie`, `build_durable_lftj_sorted_trie`, `LftjAtomSource::SortedTrie`, sorted-trie counters, and atom-temp-relation counters. This is the primary remaining eager atom materialization path.

## Objective

Make LFTJ atom access lazy/durable for every supported positive query shape, or reject unsupported shapes before execution. Do not materialize atom-local temporary sorted tries.

## Required Direction

The target runtime source is a lazy iterator over existing query-image access bytes. If an atom needs filtering for literals, inputs, repeated variables, wildcards, or local comparisons, the filter must be performed lazily while iterating durable access data.

No full atom column copy is allowed just to build a trie.

## Implementation Steps

1. Add focused tests for each current fallback trigger: three-variable atom, repeated variable atom, local comparison atom, wildcard atom, and access-order mismatch.
2. Extend `LazyAccessSlice`/`LazyAccessIter` so it can expose all variable depths required by an atom, not just one or two variables.
3. Add lazy repeated-variable equality checks without building a temp relation.
4. Add lazy local comparison filtering without building a temp relation.
5. Add lazy literal/input/wildcard handling from durable access prefixes and row-level checks.
6. Replace or reject any shape that still reaches `build_lftj_sorted_trie`.
7. Delete `build_lftj_sorted_trie` and any now-unused eager atom relation builders.

## Passing Criteria

- Zero Rust matches for `build_lftj_sorted_trie`.
- Query execution has no path that builds temporary relation images for atom access.
- Existing query tests and new fallback-trigger tests pass using lazy/durable access only.
- Benchmarks expose lazy access/runtime counters, not eager build counters.
- Full validation passes.

## Failure Modes

- Keeping eager fallback for rare shapes is failure.
- Copying full atom columns before proving need is failure.
- Dropping correctness for repeated-variable or local-comparison atoms is failure.
- Returning wrong set semantics to avoid building a trie is failure.

## Completion

Delete this PRD and commit.
