# PRD 02: Minimize Public LMDB API

## Status

Not started.

## Objective

Make `bumbledb-lmdb` expose only the API required for an embedded fact-set engine and benchmarks. Internal query/image/trie/planner structures must stop being public by default.

## Problem

`bumbledb-lmdb/src/lib.rs` re-exports many implementation details: normalized query structs, query image internals, sorted trie internals, planner trace/cost structures, and access image types. This makes internal architecture sticky and harder to delete.

## Public Surface To Keep

- `Environment`
- `ReadTxn`, `WriteTxn` transaction closure APIs
- `StorageSchema`
- `Fact`, `Value`, `FieldValues`
- `InsertOutcome`, `DeleteOutcome`
- `InputBindings`
- `QueryOutput`, `QueryResultSet`, `ResultColumn`, `ResultFact`
- error types required by callers
- tracing/diagnostic structs explicitly used by benchmarks

## Public Surface To Remove Or Privatize

- `NormalizedQuery`, `Norm*`, `InputId`, `PredicateId`
- `QueryImage`, `RelationImage`, `RelationIndexImage`, `ColumnImage`, `FixedColumn`
- `SortedTrieIndex`, `SortedTrieIter`, `TrieIter`, `LinearIter`, `TrieLevel`, `TrieFrame`
- `PlanCandidate`, `CostKey`, `NodeFactEstimate`, `QueryNodeTiming` unless benchmark output still requires them
- `IndexSpec`, `EncodedOwned` unless public scans require them

## Implementation Steps

1. Search downstream crate usage before making each symbol private.
2. Move benchmark-only fields behind narrow diagnostic structs if needed.
3. Prefer `pub(crate)` over public re-export.
4. Keep test access by colocating tests inside modules rather than widening visibility.
5. Update docs and benchmark imports.

## Passing Criteria

- Public re-export list in `bumbledb-lmdb/src/lib.rs` is visibly small.
- No public export exposes query image or trie internals.
- Benchmarks still compile using allowed diagnostics only.
- Full validation passes.

## Failure Modes

- Making internals public to fix tests is failure.
- Creating a new `prelude` that re-exports internals is failure.
- Breaking benchmarks instead of narrowing imports is failure.

## Completion

Delete this PRD and commit.
