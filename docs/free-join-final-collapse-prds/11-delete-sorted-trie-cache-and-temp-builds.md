# PRD 11: Delete Sorted Trie Cache And Temporary Builds

## Status

Not started.

## Objective

After PRD 10, delete the sorted trie atom cache, temp relation counters, and sorted trie build diagnostics.

## Prerequisite

PRD 10 must be complete.

## Code To Delete

- sorted trie atom cache in `QueryImage`
- `LftjAtomKey`
- `SortedTrieBuild`
- `CachedSortedTrie`
- temp relation counters
- sorted trie build counters
- atom trie cache tests
- benchmark fields for trie cache/builds

## Required Replacement

Lazy access metrics only: access slices opened, probes, seeks, yielded keys, forced lazy nodes if any.

## Passing Criteria

- Zero Rust matches for `sorted_trie_cache`.
- Zero Rust matches for `LftjAtomKey`.
- Zero Rust matches for `atom_temp_relation`.
- Zero Rust matches for `sorted_trie_builds`.
- Bench JSON/markdown no longer emits deleted fields.
- Full validation passes.

## Failure Modes

- Keeping a cache because tests assert hits is failure.
- Renaming sorted trie cache to lazy cache without behavior change is failure.
- Keeping stale counters at zero is failure.

## Completion

Delete this PRD and commit.
