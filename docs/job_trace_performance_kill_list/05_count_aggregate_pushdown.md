# 05 True Count Aggregate Pushdown

Priority: P0

Primary affected queries:

- `job_broad_movie_info_star`: `47,034` complete bindings for one count row.
- `job_broad_cast_keyword_company`: `11,009` complete bindings for one count row.
- `job_movie_link_bridge`: only `36` bindings, but still demonstrates the same non-factorized aggregate path.

## Problem

The optimizer has an `aggregate_pushdown` candidate, and `AggregateSink` has an `emit_count_range(binding, count)` API. But LFTJ still enumerates every full binding and passes `count=1` at the leaf. This is not true aggregate pushdown; it is late counting with a slightly optimized sink.

For broad many-way joins, this defeats the main advantage of a trie/factorized join representation. The engine can beat SQLite on `job_broad_movie_info_star`, but it still performs far more work than necessary.

## Trace Evidence

`job_broad_movie_info_star`:

- Output rows: `1`.
- `bindings_yielded`: `47,034`.
- `variable_candidates`: `52,512`.
- `trie_seek`: `166,030`.
- `trie_key_reads`: `507,718`.
- Steady-state LFTJ execute: `97.4%` of execution.
- Final aggregate finish: about `1.65us` per run.

`job_broad_cast_keyword_company`:

- Output rows: `1`.
- `bindings_yielded`: `11,009`.
- `variable_candidates`: `14,871`.
- `trie_seek`: `23,036`.
- `trie_key_reads`: `116,679`.

The final aggregate is cheap. The waste is enumerating all complete bindings to feed it.

## Current Technical Cause

LFTJ emits only at full depth:

`crates/bumbledb-lmdb/src/query.rs:2550-2568`

```rust
if depth == self.plan.variable_order_ids.len() {
    if comparisons_ready_pass(
        self.txn,
        &self.plan.comparisons,
        self.query,
        self.inputs,
        &self.binding,
        &mut self.plan.summary.counters,
    )? {
        self.plan.summary.counters.bindings_yielded += 1;
        self.sink.emit(
            self.txn,
            self.query,
            &self.binding,
            &mut self.plan.summary.counters,
        )?;
    }
    return Ok(());
}
```

The count sink has a count-only path, but it increments by exactly one per emitted binding:

`crates/bumbledb-lmdb/src/query.rs:4672-4698`

```rust
fn count_only(&self) -> bool {
    self.terms
        .iter()
        .all(|term| term.function == AggregateFunction::Count)
}

fn emit(...) -> Result<()> {
    if self.count_only() {
        return self.emit_count_range(binding, 1);
    }
    ...
}
```

The `emit_count_range` method supports batch counts, but no executor computes a range count and passes it.

`aggregate_pushdown` is currently just another all-LFTJ candidate:

`crates/bumbledb-lmdb/src/query.rs:3602-3613`

```rust
if has_aggregate(query) {
    candidates.push(build_plan_candidate(
        "aggregate_pushdown",
        ...,
        vec![NodeImpl::SortedLeapfrog; variable_order_ids.len()],
        cyclic,
    )?);
}
```

## Desired End State

For count-only queries, the executor should aggregate multiplicities at the highest safe depth instead of enumerating all suffix bindings.

For broad star joins, the executor should count factorized fanouts:

```text
sum over movie:
  cast_fanout(movie)
  * company_fanout(movie)
  * keyword_fanout(movie)
  * movie_info_fanout(movie)
  * movie_info_idx_fanout(movie)
```

Dimension atoms that only prove existence should be folded into each fanout check or eliminated if FK constraints guarantee existence.

## Proposed Technical Solution

Implement two layers.

### Layer 1: Generic LFTJ Count-Suffix Pushdown

At each depth, determine whether the remaining suffix can be counted without materializing variable bindings.

Safe conditions:

- Output is count-only aggregate.
- No projected variables depend on suffix variables.
- Group key variables, if any, are already bound.
- All comparisons involving suffix variables are either already evaluated or can be evaluated within the suffix count algorithm.
- Remaining atoms are represented by tries with countable ranges.

Add to `LftjExecutor::execute` before recursing:

```rust
if self.can_count_suffix(depth)? {
    let count = self.count_suffix(depth)?;
    self.sink.emit_count_range(&self.binding, count)?;
    self.plan.summary.counters.bindings_yielded += count;
    return Ok(());
}
```

Do not literally increment `bindings_yielded` by `count` if we want `bindings_yielded` to mean materialized bindings. Add a new counter:

```rust
factorized_counted_bindings: u64
```

### Layer 2: Specialized Star Count Kernel

Detect the common JOB shape:

- One central variable appears in many fact atoms.
- Each fact atom has one or more independent suffix variables.
- Dimension atoms constrain or validate suffix variables.
- Output is `count(center)` or `count(any suffix)` with no grouping.

For `job_broad_movie_info_star`, central variable is `?movie`.

Plan:

1. Iterate valid `?movie` values from the intersection of central fact atoms.
2. For each fact relation, compute a fanout count under the `movie` prefix after static/dimension filters.
3. Multiply fanouts.
4. Sum into aggregate.

This avoids enumerating the cartesian product of independent fanouts.

### Required Trie APIs

Sorted trie already has range and count concepts internally, but the public executor path primarily iterates keys. Add APIs:

```rust
impl SortedTrieIndex {
    fn count_prefix(&self, depth: usize, prefix: &[EncodedRef<'_>]) -> usize;
    fn distinct_count_after_prefix(&self, depth: usize, prefix: &[EncodedRef<'_>]) -> usize;
}
```

Hash trie already has `count(prefix)`:

`crates/bumbledb-lmdb/src/hash_trie.rs:178-225`

```rust
pub trait PrefixProbe {
    fn exists(&self, prefix: &[EncodedRef<'_>]) -> bool;
    fn count(&self, prefix: &[EncodedRef<'_>]) -> usize;
    fn rows<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> RowSetRef<'a>;
}
```

The LFTJ side needs comparable prefix count support.

### Aggregate Sink API

Expose count range through the trait, not just as private `AggregateSink` method:

```rust
trait TupleSink {
    fn emit(...);
    fn emit_count_range(..., count: u64) -> Result<()> {
        for _ in 0..count { self.emit(...) }
    }
}
```

`AggregateSink` overrides it efficiently. `OutputSink` can keep the default or reject range counts for non-aggregate output.

### Predicate Handling

For range predicates on suffix variables, a suffix count can be exact only if the trie order allows counting the range without row iteration. Otherwise fall back to normal enumeration.

For `job_broad_movie_info_star` and `job_broad_cast_keyword_company`, the broad count queries have no range predicates. They are ideal first targets.

## Implementation Plan

1. Add `TupleSink::emit_count_range`.
2. Add `PlanCounters.factorized_counted_bindings`.
3. Add `SortedTrieIndex` prefix count API.
4. Implement count-only suffix analysis for pure LFTJ.
5. Implement star-count detection for one central variable.
6. Route `aggregate_pushdown` plans to the new path only when proven safe.
7. Rename or remove `aggregate_pushdown` candidate until it actually uses pushdown.

## Tests

- Count-only join returns same count as full enumeration.
- Grouped count with group variables bound before suffix uses range counts safely.
- Count with projected variables does not use pushdown.
- Count with unsafe comparison falls back.
- `job_broad_movie_info_star` fixture shows `factorized_counted_bindings > 0` and lower `bindings_yielded`.
- Differential tests compare pushdown and reference evaluator.

## Acceptance Criteria

- `job_broad_movie_info_star` `bindings_yielded` drops from `47,034` by at least `80%`, or a separate `factorized_counted_bindings` accounts for most multiplicity.
- `job_broad_cast_keyword_company` `bindings_yielded` drops from `11,009` by at least `80%`.
- Steady-state LFTJ execute share on `job_broad_movie_info_star` drops from `97.4%` substantially.
- Outputs match current Datalog semantics exactly.

## Risks

- Multiplicity semantics must match current aggregate behavior. Datalog projection has set semantics, but aggregate counting currently counts emitted bindings. Pushdown must preserve that.
- Repeated variables and equality constraints can break simple fanout multiplication.
- Optional/fk-eliminated atoms must not change multiplicity.

## Rollout Plan

1. Add trait/counter plumbing.
2. Implement no-comparison, no-group, count-only suffix pushdown.
3. Implement central-star fanout count for JOB broad queries.
4. Add grouped count support later.
5. Re-run firehose trace and validate lower binding and trie-key-read counts.
