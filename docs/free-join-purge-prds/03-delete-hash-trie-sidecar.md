# PRD 03: Delete Hash Trie Sidecar

## Status

Not started.

## Severity

High architecture cleanup.

## Prerequisite

PRD 02 must be complete. No direct runtime may depend on hash trie.

## Problem

The paper's GHT is a unified data structure under Free Join. Bumbledb's current `hash_trie.rs` is not that. It is a sidecar hash index with separate cache keys, leaf modes, prefix-probe traits, and direct-kernel usage.

Once direct kernels are deleted, this sidecar should die rather than be preserved as an almost-GHT. A real GHT/COLT implementation belongs in the broader rebase PRDs under Free Join node execution.

## Code To Delete

- `crates/bumbledb-lmdb/src/hash_trie.rs`
- `crates/bumbledb-lmdb/src/query_access.rs`
- `HashTrieIndex`
- `HashTrieKey`
- `HashTrieStats`
- `LeafMode`
- `PrefixProbe`
- `PrefixFacts`
- hash trie cache in `QueryImage`
- `cached_hash_trie`
- `build_hash_trie_index`
- hash trie benchmark counters and JSON fields
- hash trie tests

## Required Replacement

None in this PRD. Queries must use LFTJ/static proof until GHT/COLT is implemented through Free Join.

## Implementation Steps

1. Remove hash trie module from `lib.rs`.
2. Remove public exports.
3. Remove `HashTrieKey` and cache from query image.
4. Remove query access module.
5. Remove hash trie counters from `PlanCounters` and benchmarks.
6. Remove hash trie allocation phases from benchmark output if they become empty.
7. Delete hash-trie-specific tests.
8. Rewrite tests that verified behavior through hash trie to verify final query results through LFTJ.

## Required Tests

- Former hash trie direct lookup query returns same result through LFTJ.
- Former prefix-count behavior is covered by query-image access prefix tests or removed if no longer public.
- Benchmarks compile without hash trie fields.

## Strict Passing Criteria

- Zero Rust matches for `HashTrie`.
- Zero Rust matches for `LeafMode`.
- Zero Rust matches for `PrefixProbe`.
- Zero Rust matches for `PrefixFacts`.
- Zero Rust matches for `hash_trie`.
- Full validation gate passes.

## Failure Modes

- Keeping hash trie as a renamed GHT without Free Join integration is failure.
- Keeping hash trie cache fields with zero values is failure.
- Keeping hash-trie benchmark fields is failure.

## Non-Goals

- Do not implement COLT here.
- Do not add a replacement hash structure here.
