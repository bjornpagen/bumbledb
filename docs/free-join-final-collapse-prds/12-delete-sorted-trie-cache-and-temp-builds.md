# PRD 12: Delete Sorted Trie Cache And Temporary Builds

## Status

Not started.

## Prerequisite

PRD 11 must be complete. No runtime path may need eager sorted-trie atom construction.

## Current State

`QueryImage` still owns an unbounded `sorted_trie_cache`, `LftjAtomKey`, `SortedTrieBuild`, `CachedSortedTrie`, and `SortedTrieIndex`-based atom sources. Benchmarks and tests still assert cache/build counters. These structures exist only to support the eager fallback targeted by PRD 11.

## Objective

Delete sorted-trie atom cache infrastructure and every temp-build counter/benchmark field that preserved it.

## Code To Delete

- `QueryImage.sorted_trie_cache`
- `LftjAtomKey`
- `SortedTrieBuild`
- `CachedSortedTrie`
- `LftjAtomSource::SortedTrie`
- sorted-trie cache hit/miss/build counters
- atom temp relation counters
- atom trie cache tests
- benchmark JSON/markdown fields for trie cache/build/temp relation mechanics
- `sorted_trie.rs` if no surviving non-test runtime code uses it

## Required Replacement

Lazy access metrics only: slices opened, iterator opens/ups/seeks/nexts/key reads, candidate values, bindings completed, and output facts.

## Passing Criteria

- Zero Rust matches for `sorted_trie_cache`.
- Zero Rust matches for `LftjAtomKey`.
- Zero Rust matches for `atom_temp_relation`.
- Zero Rust matches for `sorted_trie_builds`.
- Zero Rust matches for `SortedTrieIndex` outside deleted tests/docs.
- Bench JSON/markdown no longer emits deleted fields.
- Full validation passes.

## Failure Modes

- Keeping a cache because tests assert hits is failure.
- Renaming sorted trie cache to lazy cache without behavior change is failure.
- Keeping stale counters at zero is failure.
- Keeping `sorted_trie.rs` as dead production code is failure.

## Completion

Delete this PRD and commit.
