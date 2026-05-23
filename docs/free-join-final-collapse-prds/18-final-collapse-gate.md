# PRD 18: Final Collapse Gate

## Status

Not started.

## Current State

This is the final audit after PRDs 10 through 17. It must verify that the codebase is not merely passing tests, but has no source/doc residue from deleted architectures or mechanics.

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
SubAtom
free_join_subatom
SortedTrieIndex
sorted_trie_cache
LftjAtomKey
build_lftj_sorted_trie
atom_temp_relation
sorted_trie_builds
sorted_trie_cache_hits
sorted_trie_cache_misses
hash_build_facts
CostKey
PlanCandidate
OptimizerTrace
PlanEstimates
VariableEstimate
NodeFactEstimate
chosen_plan
candidate_plan
free_join_estimates
iterator_ops
build_facts
cursor_seeks
facts_scanned
facts_matched
uses_indexed_multiway_join
```

Allowed exceptions must be listed in this PRD before deletion, but the target is zero for deleted mechanics. `QueryImage`/`RelationImage` may remain only as private implementation names after PRD 14; they must not be public API exports or benchmark-facing concepts.

## Final Public API Must Not Export

- query image internals
- relation image internals
- raw encoded components
- physical plan internals beyond stable diagnostics
- forged mutable typed IR fields
- backup/compact helpers unless explicitly retained as embedded operational API

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
