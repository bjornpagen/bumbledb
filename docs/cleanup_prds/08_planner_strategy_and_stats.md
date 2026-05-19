# PRD 08: Planner Strategy And Stats

## Status

Draft. This PRD complements PRD 07 and preserves join-heavy performance while improving simple workload choices.

## Problem

Planning currently chooses variable order and physical Free Join candidates using cheap approximate field/index stats. It is good enough for many JOB improvements, but it does not cleanly separate direct, nested-loop, hash, and LFTJ strategies.

Simple workloads need cheap direct planning. True join-heavy cyclic workloads need WCOJ/LFTJ. The planner should choose between runtime families deliberately.

## Goals

- Split query planning into runtime families.
- Make direct/nested-loop plans first-class plan candidates.
- Keep LFTJ for cyclic/high-degree joins.
- Improve parameterized plan caching.
- Improve stats visibility and cost model correctness.
- Remove benchmark-specific special cases when general mechanisms replace them.

## Non-Goals

- No SQL-style exhaustive optimizer.
- No vector planner.
- No document planner.
- No runtime adaptive re-optimization.

## Current Code References

- `plan_query`, `choose_variable_order`, and `optimize_free_join_plan` in `query.rs`.
- `estimate_atom_variable_access` and `variable_probe_eligible` in `query.rs`.
- `PlannerStatsCache` and `OptimizerRelationStats` in `planner_stats.rs`.
- `PlanCandidate`, `CostKey`, and `OptimizerTrace` in `query.rs`.
- `try_execute_factorized_count` and JOB-specific static proofs in `query.rs`.

## Required Planner Architecture

Planner phases:

```text
normalize
classify query shape
collect minimal stats needed for candidate family
generate direct candidates
generate index nested-loop candidates
generate hash-probe candidates
generate Free Join/LFTJ candidates
choose by cost
execute selected runtime
record actual counters
```

Direct candidates must not require full query image if they can execute from current indexes.

Free Join candidates may require query image.

## Cost Model Requirements

Cost model must include:

- Query-image build cost.
- Prepared-plan cache hit/miss cost.
- Hash-trie build rows.
- Sorted-trie build rows and sort cost.
- Direct index prefix/range scan estimated rows.
- Row fetch cost for non-covering indexes.
- Output decode/materialization cost.
- Aggregate materialization cost.

The model must not pretend LFTJ setup is free.

## Stats Requirements

Keep cheap stats but expose limitations:

- Relation row count.
- Field sample distinct/min/max/heavy hitters.
- Index depth/fanout estimates.
- Exact primary/unique cardinality where derivable.
- Segment-backed index byte length and row count.

Add exact cheap facts:

- Primary key is unique.
- Unique constraints produce max one row for full prefix.
- FK generated indexes have known leading field order.
- Range indexes are ordered only by their leading ordered field.

## Parameterized Cache Requirements

Prepared cache key must distinguish:

- Query shape.
- Relation/field IDs.
- Variable/input positions.
- Predicate operators.
- Output shape.

Prepared cache key must not include actual input values.

Literal values may remain in shape cache only if they affect plan shape; otherwise use separate literal encoding cache.

## Special Case Cleanup

Current code includes JOB-specific static-empty/count logic. These may stay temporarily but must be documented as transitional.

Long-term requirement:

- Replace benchmark-specific proofs with generic direct/index-intersection/count mechanisms when possible.
- Add TODOs or tracking tests for each remaining benchmark-specific branch.

## Implementation Plan

1. Add plan family enum.
2. Add direct and nested-loop candidate builders.
3. Add query-image build cost to candidate cost keys.
4. Add parameterized shape cache.
5. Update explain output to show candidate family and excluded candidates.
6. Add actual-vs-estimated diagnostics for chosen runtime.
7. Add tests for family selection.
8. Add benchmark gates for selected runtime kinds on sentinel queries.

## Strict Passing Criteria

- Simple point/range queries are not planned as LFTJ.
- Cyclic triangle remains Free Join/LFTJ or an equally proven WCOJ strategy.
- Parameterized queries reuse prepared shape plans.
- Cost output includes query-image/build/setup costs.
- Explain output clearly states selected runtime family.
- JOB benchmark wins are preserved.
- Non-JOB simple workloads improve materially or have documented blockers.

## Verification Commands

```sh
cargo test -p bumbledb-lmdb query planner_stats
scripts/bench-focused.sh --fail-gates
cargo run -p bumbledb-bench --release -- --scale 10000 --warmup 2 --repeats 10 --dataset joinstress --dataset ledger --dataset sailors --dataset tpch --format markdown
```
