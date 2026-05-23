# PRD 18: Final Collapse Gate

## Status

Not started.

## Objective

Run the final audit proving the codebase is a minimal Free Join set engine with no deleted architecture residue.

## Required Audits

1. Source hygiene gate from README.
2. Docs hygiene gate from README.
3. Public export inventory.
4. Line-count gate from PRD 17.
5. Benchmark JSON/markdown renderer tests.
6. Query-focused validation gate.
7. Full global validation gate.
8. `git diff --check`.

## Final Source Must Have Zero Matches For

```text
PreparedQuery
QueryResultCardinality
CardinalitySink
AggregateFunction
AggregateSink
NodeImpl
SortedTrieIndex
sorted_trie_cache
build_lftj_sorted_trie
atom_temp_relation
hash_build_facts
uses_indexed_multiway_join
QueryImage
RelationImage
```

Allowed exceptions must be listed in this PRD before deletion, but the target is zero.

## Final Public API Must Include Only

- environment open/open_with_schema
- read/write closures
- fact insert/delete
- set query execution
- schema descriptors and storage schema
- facts/values/input bindings/result sets/errors
- benchmark/tracing support outside production public API

## Passing Criteria

- Every prior PRD file in this suite has been deleted.
- This file is the last remaining PRD and is deleted after its checks pass.
- All validation gates pass.
- Worktree is clean after commit.

## Failure Modes

- Leaving this PRD file after commit is failure.
- Documenting exceptions instead of fixing them is failure unless the code is required by benchmarks/tracing/storage and explicitly not a deleted architecture remnant.

## Completion

Delete this PRD and commit.
