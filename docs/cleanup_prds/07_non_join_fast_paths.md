# PRD 07: Non-Join Fast Paths

## Status

Draft. This PRD is required before returning to broad database work because SQLite currently wins many non-join-heavy workloads.

## Problem

Simple point, unique, prefix, range, and light chain queries can pay heavyweight query-image, planning, hash-trie, or LFTJ setup costs. SQLite is extremely good at tight covering-index nested loops for these shapes.

Bumbledb must bypass heavyweight join machinery for simple workloads.

## Goals

- Add direct execution before query-image/LFTJ setup when query shape allows it.
- Make point/range/selective queries competitive with SQLite.
- Keep Free Join/LFTJ for true join-heavy/cyclic workloads.
- Preserve zero LMDB cursor-seek and row-scan benchmark counters where existing gates require it.
- Avoid building full hash tries for selective one-shot lookups.

## Non-Goals

- No vector search.
- No FlatBuffer/document traversal.
- No SQL planner.
- No cost-based exhaustive optimizer.
- No unsafe durability shortcuts.

## Current Code References

- `ReadTxn::execute_query` in `query.rs`.
- `try_direct_kernel`, `try_direct_prefix_range_kernel`, and `try_direct_chain_kernel` in `query.rs`.
- `execute_direct_prefix_range` and `DirectChainExecutor` in `query.rs`.
- `direct_hash_index` in `query.rs`.
- `RelationIndexImage::entries_with_prefix` in `query_image.rs`.
- `scan_prefix` and `scan_range` in `storage.rs`.
- Benchmark non-join pain points in ledger, sailors, and tpch datasets.

## Required Runtime Families

Add explicit direct runtime kinds:

```rust
DirectPrimaryLookup
DirectUniqueLookup
DirectPrefixScan
DirectRangeScan
DirectCount
DirectAggregate
IndexNestedLoop
HashProbe
FreeJoinLftj
```

`QueryRuntimeKind` and explain output must distinguish these.

## Required Query Classifier

Add a cheap classifier before query-image acquisition when possible:

```text
TypedQuery/NormalizedShape -> DirectPlanCandidate
```

Classifier must detect:

- Single relation primary-key lookup.
- Single relation unique lookup.
- Single relation equality prefix scan.
- Single relation range scan over ordered leading field.
- Count over primary/unique/prefix/range scan.
- Simple projection/filter over one index range.
- Acyclic chain where each step is a primary/unique/FK lookup.

## Direct Storage Path

Direct paths should use current LMDB indexes and row store directly, not query-image relation images, when that is cheaper.

Required direct APIs:

- exact current row lookup by primary key
- exact unique guard/index lookup
- encoded prefix scan by access path
- encoded range scan by access path
- row fetch by primary identity
- projection decode without building full query image

## Prepared Shape Cache For Inputs

Parameterized queries should cache their physical shape.

Current prepared plan caching returns `None` when inputs exist. That must change.

Required behavior:

- Cache by query shape, not input values.
- Input values are encoded per execution.
- Direct plan shape can be reused across input executions.

## Direct Aggregate Requirements

Direct aggregate paths must support:

- `count` over prefix/range without materializing rows.
- `sum` over a projected scalar when index or row fetch can stream values.
- `min`/`max` via ordered index when available.

Count fast paths are highest priority.

## Target Workloads

Must improve or preserve:

- `ledger/postings_for_holder_range`
- `ledger/balances_by_instrument`
- `sailors/sailor_range_reserves`
- `joinstress/chain4_from_a`
- `tpch/supplier_nation_orders`
- `tpch/revenue_by_customer_range`

## Implementation Plan

1. Add direct query-shape classifier.
2. Add direct plan structs outside the Free Join plan.
3. Add direct LMDB current-index execution methods.
4. Prefer direct path before query-image build for eligible shapes.
5. Add parameterized prepared shape cache.
6. Add direct count and range paths.
7. Add index nested-loop path for simple acyclic chains.
8. Update explain/timing/counter output.
9. Add differential tests against reference/SQLite.

## Strict Passing Criteria

- Single-relation primary lookup does not build `QueryImage` on cold execution.
- Single-relation unique lookup does not build hash trie or LFTJ trie.
- Single-relation prefix/range query does not build full relation hash trie.
- `sailors/sailor_range_reserves` uses a direct runtime kind.
- `joinstress/chain4_from_a` uses direct or index-nested-loop runtime, not LFTJ.
- Parameterized query shapes are cached.
- Direct count path does not materialize intermediate rows.
- Existing JOB wins are not regressed by choosing direct paths for cyclic/multiway workloads.
- Benchmark output exposes direct runtime counters clearly.

## Verification Commands

```sh
cargo test -p bumbledb-lmdb query
cargo test --workspace --all-features
scripts/bench-focused.sh --fail-gates
cargo run -p bumbledb-bench --release -- --scale 10000 --warmup 2 --repeats 10 --dataset ledger --dataset sailors --dataset tpch --format markdown
```
