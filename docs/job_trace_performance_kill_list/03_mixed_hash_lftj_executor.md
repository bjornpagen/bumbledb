# 03 Real Mixed HashProbe And LFTJ Executor

Priority: P0

Primary affected queries:

- `job_broad_cast_keyword_company`: chosen plan `hash_probe`, runtime `MixedFallback`, `hash_probe_calls=0`.
- `job_q09_voice_us_actor`: chosen plan `hash_probe`, runtime `MixedFallback`, `hash_probe_calls=0`.

## Problem

The optimizer can create a plan where some nodes are `HashProbe` and some nodes are `SortedLeapfrog`. It labels the candidate `hash_probe`, and the cost model prices it as if hash-probe work will happen. The runtime cannot execute mixed plans. If any node is not `HashProbe`, dispatch falls back to full LFTJ.

This produces misleading observability and wasted optimization work:

- The chosen plan says `hash_probe`.
- Runtime says `MixedFallback`.
- Hash counters are zero.
- All nodes run through LFTJ atom plans and sorted trie traversal.

## Trace Evidence

`job_broad_cast_keyword_company`:

- Chosen plan: `hash_probe`.
- Runtime: `MixedFallback`.
- Steady-state avg: `5.40ms`, slightly slower than SQLite.
- Hash counters: all zero.
- LFTJ enumerates `11,009` bindings to count one row.

`job_q09_voice_us_actor`:

- Chosen plan: `hash_probe`.
- Runtime: `MixedFallback`.
- Steady-state avg: `1.62ms`, faster than SQLite but still dominated by LFTJ exec and plan.
- Hash counters: all zero.

## Current Technical Cause

The dispatch contract is all-or-nothing:

`crates/bumbledb-lmdb/src/query.rs:1313-1347`

```rust
if plan
    .summary
    .free_join
    .nodes
    .iter()
    .all(|node| node.implementation == NodeImpl::HashProbe && node.bind_vars.len() == 1)
{
    plan.summary.runtime_kind = QueryRuntimeKind::HashProbe;
    return execute_hash_probe(image, txn, schema, query, inputs, plan, sink);
}

plan.summary.runtime_kind = if plan.summary.free_join.is_pure_lftj() {
    QueryRuntimeKind::Lftj
} else {
    QueryRuntimeKind::MixedFallback
};
execute_lftj(image, txn, query, inputs, plan, sink)
```

The optimizer still builds a `hash_probe` candidate with a mixed implementation vector:

`crates/bumbledb-lmdb/src/query.rs:3552-3587`

```rust
let probe_impls = probe_node_impls(schema, atoms, variable_order_ids, stats, cyclic)?;
candidates.push(build_plan_candidate(
    "hash_probe",
    schema,
    query,
    atoms,
    variable_order_ids,
    variable_costs,
    stats,
    probe_impls,
    cyclic,
)?);
```

The cost model assigns hash build/probe cost to nodes marked `HashProbe`:

`crates/bumbledb-lmdb/src/query.rs:3773-3830`

```rust
NodeImpl::HashProbe => {
    hash_probe_rows = hash_probe_rows.saturating_add(cost.estimated_candidates.max(1));
    hash_build_rows = hash_build_rows.saturating_add(cost.estimated_candidates.max(1));
}
```

If the selected plan later falls back to LFTJ, those estimates are not the actual runtime behavior.

## Desired End State

A plan with mixed `SortedLeapfrog` and `HashProbe` nodes must do one of two things:

- Execute as a real mixed plan and produce nonzero hash counters.
- Be rejected or relabeled before optimizer selection so it cannot claim hash-probe cost or plan name.

The preferred solution is a real mixed executor because the trace shows exactly the workload shape we want: early LFTJ/domain narrowing followed by late hash existence/probe checks.

## Proposed Technical Solution

Add `execute_mixed_free_join` for Free Join plans containing both `SortedLeapfrog` and `HashProbe` single-variable nodes.

### Runtime Model

The mixed executor keeps the same variable order and recursive structure, but dispatches each node by implementation:

```rust
fn execute_mixed(depth: usize) -> Result<()> {
    let node = &plan.summary.free_join.nodes[depth];
    match node.implementation {
        NodeImpl::SortedLeapfrog => execute_lftj_node(depth),
        NodeImpl::HashProbe => execute_hash_node(depth),
        _ => unsupported_or_fallback(),
    }
}
```

### LFTJ Node Execution

Reuse `LftjExecutor` logic for one depth, not the whole executor. The current LFTJ executor owns a full `LftjRuntime` and recurses internally. Refactor it into reusable operations:

- `open_participants(depth)`
- `leapfrog_candidates(depth, visitor)`
- `close_participants(depth)`

The mixed executor calls those operations and then recurses back into `execute_mixed(depth + 1)`.

### Hash Node Execution

Reuse `HashProbeExecutor` logic for one depth:

- Choose driver by `probe_atom_count`.
- Probe prefix.
- Iterate rows for driver prefix.
- Bind current variable.
- Check other participant atoms as existence probes.
- Evaluate ready predicates.
- Recurse.

This should use the lazy hash index provider from [`02_lazy_hash_probe_indexes.md`](02_lazy_hash_probe_indexes.md), otherwise mixed execution will inherit cold eager-build waste.

### Shared Binding

Both node types need the same `EncodedBinding`. Create a `MixedExecutor` with:

```rust
struct MixedExecutor<'txn, 'input, 'query, 'plan, 'image, S: TupleSink> {
    image: &'image QueryImage,
    txn: &'input ReadTxn<'txn>,
    schema: &'input StorageSchema,
    query: &'query NormalizedQuery,
    inputs: &'input EncodedInputs,
    plan: &'plan mut ExecutionPlan,
    binding: EncodedBinding,
    sink: &'plan mut S,
    lftj_runtime: LftjRuntime<'image>,
    hash_provider: HashIndexProvider<'image>,
}
```

### Dispatch Rule

Update `execute_free_join`:

```rust
if all_hash_probe(...) {
    runtime_kind = HashProbe;
    execute_hash_probe(...)
} else if is_mixed_hash_lftj(...) {
    runtime_kind = Mixed;
    execute_mixed_free_join(...)
} else if is_pure_lftj(...) {
    runtime_kind = Lftj;
    execute_lftj(...)
} else {
    runtime_kind = MixedFallback;
    execute_lftj(...)
}
```

Add `QueryRuntimeKind::Mixed` distinct from `MixedFallback`.

### Plan Contract

`MixedFallback` should become an error in benchmark gates or at least a visible warning in traces. We should not silently ignore selected node implementations.

### Counters

Mixed execution should produce both trie and hash counters:

- LFTJ nodes update `trie_open`, `trie_next`, `trie_seek`, `trie_key_reads`.
- Hash nodes update `hash_probe_calls`, `hash_probe_hits`, `hash_probe_misses`, `hash_rows_returned`, `hash_distinct_emits`.
- Node timings should be populated per node as soon as node execution can be isolated.

## Implementation Plan

1. Extract single-depth LFTJ node execution helpers from `LftjExecutor`.
2. Extract single-depth HashProbe node execution helpers from `HashProbeExecutor`.
3. Introduce `MixedExecutor` and `QueryRuntimeKind::Mixed`.
4. Update dispatch to call mixed executor before falling back.
5. Guard unsupported node impls explicitly.
6. Update optimizer trace to distinguish `hash_probe_all` vs `mixed_hash_lftj` candidate names if needed.

## Tests

Add a minimal query where first variable is LFTJ and second is HashProbe:

- Assert runtime `Mixed`.
- Assert `hash_probe_calls > 0`.
- Assert `trie_next > 0`.
- Assert output equals pure LFTJ reference.

Add JOB-shaped regression tests:

- `job_broad_cast_keyword_company` no longer reports `MixedFallback`.
- `job_q09_voice_us_actor` no longer reports `MixedFallback`.
- Existing pure LFTJ and all-hash tests still select their specialized runtimes.

## Acceptance Criteria

- `MixedFallback` disappears from the traced JOB run or is limited to genuinely unsupported plans.
- `job_broad_cast_keyword_company` shows nonzero hash counters.
- `job_q09_voice_us_actor` shows nonzero hash counters.
- Runtime results match current outputs exactly.
- Optimizer candidate names match actual runtime behavior.

## Risks

- Refactoring recursive executors may introduce binding/unbinding bugs.
- LFTJ and HashProbe use different duplicate-elimination strategies; mixed execution must preserve Datalog set semantics and aggregate multiplicity semantics.
- Lazy hash indexes and mixed execution should be implemented together to avoid trading runtime fallback waste for eager hash build waste.

## Rollout Plan

1. Add mixed runtime skeleton and tests with small fixtures.
2. Implement mixed with eager hash indexes first for correctness.
3. Combine with lazy hash index provider.
4. Rerun JOB traces and validate `MixedFallback=0` for targeted queries.
5. Revisit cost model after real mixed timings are available.
