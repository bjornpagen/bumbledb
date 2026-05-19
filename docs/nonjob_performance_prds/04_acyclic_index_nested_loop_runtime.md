# PRD 04: Acyclic Index Nested-Loop Runtime

## Status

Draft.

## Problem

Many non-JOB losses are selective acyclic joins where SQLite's covering-index nested loops are the right execution model. Bumbledb often routes these through LFTJ, building temporary sorted tries and paying WCOJ setup costs that are not justified for acyclic/selective shapes.

Target examples:

```text
ledger/postings_for_holder_range
ledger/balances_by_instrument
sailors/red_boat_sailors
sailors/high_rating_red_boats
```

## Root Cause Analysis

Current behavior:

```text
normalized query -> variable order -> Free Join candidates -> often LFTJ -> temp atom relation build -> sorted trie build -> leapfrog traversal
```

For cyclic joins, this is good. For selective acyclic chains/stars, this can be wasteful.

Waste pattern from trace:

- `lftj.build.scan_filter_copy` copies relation-image bytes into temporary atom images.
- `sorted_trie.build` sorts temporary columns.
- LFTJ traversal performs intersections where an index nested loop could directly probe by bound key.
- For broad output joins, sink finish dominates after traversal.

Example after range-cost fix:

```text
rows = 2000
candidate count dropped from huge to reasonable
still uses LFTJ
still slower than SQLite by ~1.5x
```

## Goal

Add a runtime family for selective acyclic joins using index nested loops over current or relation-image indexes, without building full temporary sorted tries.

## Non-Goals

- No replacement for cyclic WCOJ/LFTJ.
- No SQL optimizer.
- No bushy join search.
- No adaptive runtime re-optimization.
- No vector/document semantics.

## Eligibility

A query is eligible when:

- Join graph is acyclic or has a tree/star/chain shape.
- There is a seed atom with static/input/literal prefix or selective unique/primary lookup.
- Each next atom can be reached through an index prefix using already-bound variables.
- Predicates are equality/range comparisons that can be checked after each binding or at leaf.
- There is no cyclic triangle-like all-degree-greater-than-one structure.

Eligible examples:

```text
Account(holder: $holder) -> Posting(account: ?account)
Supplier(nation: $nation) -> LineItem(supplier: ?supplier) -> Orders(id: ?order)
Boat(color: $color) -> Reserve(boat: ?boat) -> Sailor(id: ?sailor)
```

Ineligible examples:

```text
EdgeAB(a,b), EdgeAC(a,c), EdgeBC(b,c)
```

## Technical Design

Add runtime family:

```rust
PlanFamily::IndexNestedLoop
QueryRuntimeKind::IndexNestedLoop
```

Plan representation:

```rust
IndexNestedLoopPlan {
    steps: Vec<IndexNestedLoopStep>,
    output: OutputPlan,
}

IndexNestedLoopStep {
    atom_id: AtomId,
    relation: RelationId,
    index_name: String,
    prefix_terms: Vec<NormTerm>,
    bind_fields: Vec<FieldId>,
    residual_predicates: Vec<PredicateId>,
}
```

Execution algorithm:

```text
seed from most selective static/input atom
for each row in seed index prefix/range:
  bind introduced variables
  evaluate ready predicates
  recurse into child atoms by index prefix
  emit at leaf
```

The runtime should use:

- current index prefix/range scans for small query-scoped execution,
- durable relation index images if query image is already available and cheaper,
- row payload fetch for non-covering indexes.

## Planner Requirements

Candidate generation must produce an `index_nested_loop` candidate with a cost including:

- seed estimated rows,
- fanout per step,
- row fetch cost,
- output materialization cost,
- no sorted-trie build cost,
- no relation-wide hash-trie build cost.

The candidate must lose to LFTJ for cyclic WCOJ shapes.

The candidate should beat LFTJ for selective acyclic paths.

## Required Counters

Add counters or expose through direct counters:

- nested-loop index probes,
- nested-loop rows returned,
- nested-loop residual predicate checks,
- nested-loop row payload fetches,
- nested-loop early exits.

## Required Tests

Add tests for:

- Chain query selects `IndexNestedLoop` instead of LFTJ when all steps have indexes.
- Star query selects `IndexNestedLoop` for selective dimension predicate.
- Triangle query does not select `IndexNestedLoop`.
- Nested-loop output equals existing LFTJ/reference output.
- Range predicate in nested loop is evaluated correctly.
- Repeated relation aliases work correctly.

## Required Benchmark Gates

Target queries:

```text
ledger/postings_for_holder_range
ledger/balances_by_instrument
sailors/red_boat_sailors
sailors/high_rating_red_boats
```

Gate requirements:

- `joinstress/triangle_count` remains `FreeJoinLftj`.
- Selective chain/star target queries use `IndexNestedLoop` or `Direct` unless a documented benchmark artifact proves LFTJ is faster.
- `sorted_trie_builds == 0` for queries that choose nested loop.
- `hash_index_builds == 0` for queries that choose nested loop.
- `cursor_seeks == 0` and `rows_scanned == 0` remain true.

Performance targets:

```text
ledger/postings_for_holder_range <= 3x SQLite
ledger/balances_by_instrument <= 3x SQLite
sailors/red_boat_sailors <= 1.5x SQLite
sailors/high_rating_red_boats <= 1.5x SQLite
```

These are initial targets, not permanent product promises.

## Strict Passing Criteria

- `IndexNestedLoop` runtime exists and is visible in `QueryPlan` and benchmark JSON/markdown.
- At least three target non-JOB acyclic queries move off LFTJ to `IndexNestedLoop` or `Direct`.
- `joinstress/triangle_count` remains `FreeJoinLftj`.
- Target query exact results match the reference executor/materialized path.
- No JOB query regresses below SQLite at latest `scale=10000`, `warmup=2`, `repeats=30` suite.
- Full workspace test/clippy/fuzz gates pass.

## Verification Commands

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
cargo run -p bumbledb-bench --release -- --scale 10000 --warmup 2 --repeats 30 --format json --dataset ledger --dataset sailors --dataset tpch
cargo run -p bumbledb-bench --release -- --scale 10000 --warmup 2 --repeats 30 --format json --dataset joinstress --query triangle_count
```
