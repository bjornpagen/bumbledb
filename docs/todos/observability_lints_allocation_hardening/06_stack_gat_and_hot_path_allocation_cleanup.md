# 06: Stack, GAT, And Hot-Path Allocation Cleanup

**Goal**
- Reduce avoidable heap churn in the query runtime using stack-backed buffers, borrowed streaming iterators, and generic associated types where they make APIs simpler and faster.
- Do the first targeted cleanup only; leave larger algorithmic specialization to `performance_kill_list/05_direct_selective_query_kernels.md` and later PRDs.

**Current Hot Allocation Evidence**
- `EncodedValue` stores encoded bytes in `Vec<u8>` even though supported encoded widths are fixed at 1, 8, or 16 bytes.
- `EncodedBinding` stores `Vec<Option<EncodedValue>>`; the variable count is usually tiny and known at query execution start.
- LFTJ binds values using `value.as_bytes().to_vec()` at each candidate.
- `LftjExecutor::participants` allocates a `Vec<usize>` at each recursion depth.
- `HashProbeExecutor::participants` allocates a `Vec<usize>` at each recursion depth.
- `LeapfrogState` owns an allocating `Vec<usize>` per variable binding step.
- `hash_prefix` builds `Vec<EncodedOwned>` and probe methods then build `Vec<EncodedRef>`.
- `probe_atom_rows` uses `rows_owned()` and can materialize broad fanout row IDs before filtering.
- LFTJ and hash cache keys are strings built with `format!`/`write!` in hot planning paths.
- `build_lftj_sorted_trie` builds `Vec<Vec<Vec<u8>>>` raw columns and clones raw columns into `ColumnImage`.

**Allowed Dependencies**
- Add `smallvec` or `arrayvec` at the workspace level if measurements justify it.
- Prefer `smallvec` for buffers whose length is usually tiny but not statically capped.
- Prefer `arrayvec` only where the maximum length is a hard invariant.
- Do not add a bump allocator or arena until allocation profiling proves it is needed.

**Required Refactors**
- Replace fixed-width encoded bytes in hot bindings with `EncodedOwned` or an equivalent stack-sized enum.
- Change `EncodedBinding` to avoid per-bind heap allocation for 1/8/16-byte values.
- Use stack-backed buffers for prefix construction, participant lists, leapfrog iterator IDs, and small row/key temporary collections.
- Precompute participants by variable once in runtime setup for LFTJ and HashProbe.
- Replace `rows_owned()` in HashProbe broad paths with visitor or streaming iteration.
- Keep `for_each_row` for early-exit existence checks and extend it where useful.
- Replace string cache keys with typed cache keys for sorted trie and hash trie caches.
- Avoid cloning raw columns in `build_lftj_sorted_trie` where ownership can be moved safely.
- Keep projection and aggregation set semantics correct while reducing allocation pressure.

**GAT Requirements**
- Define borrowed streaming traits where they remove `Vec` materialization without boxing.
- Use a GAT shape similar to:

```rust
pub trait PrefixRows {
    type Rows<'a>: Iterator<Item = RowId> + 'a
    where
        Self: 'a;

    fn rows_for_prefix<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> Self::Rows<'a>;
}
```

- Implement the streaming row iterator as concrete enums over empty, one, slice, range, and recursive hash-node traversal cases.
- Do not use `Box<dyn Iterator>` in hot paths.
- Keep lifetimes tied to the borrowed index/image, following the GAT motivation from streaming iterators.

**Measurement Requirements**
- Use allocation counters from PRD 05 before and after each hot-path cleanup.
- Track allocation calls and bytes for at least `chain4_from_a`, `sailor_range_reserves`, `tag_lookup_join`, `red_boat_sailors`, `triangle_count`, and `supplier_nation_orders`.
- Track latency alongside allocation metrics; fewer allocations are not a win if latency regresses without explanation.
- Prefer small local changes that remove measured allocation pressure over broad rewrites.

**Tests**
- Existing query result tests pass unchanged.
- HashProbe tests still show all-hash plans use hash counters and zero trie counters.
- LFTJ tests still show cached sorted trie counters behave correctly.
- New GAT row iterator tests cover empty, one, slice, range, and nested-prefix traversal.
- Typed cache key tests prove equivalent query shapes hit cache and different input/literal values miss cache when required.

**Passing Requirements**
- Allocation-profile benchmark shows reduced allocation calls or bytes for targeted hot queries.
- No query result changes versus SQLite/reference tests.
- No new public query engine path is introduced.
- No hot path uses `Box<dyn Iterator>` to satisfy GAT APIs.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes under the strict lint policy.

**Stop Conditions**
- Stop if stack buffers require unsafe code; use safe `smallvec`/`arrayvec` first.
- Stop if a GAT abstraction makes code harder to reason about without removing allocation or improving lifetime correctness.
- Stop if typed cache keys risk cache collisions or incorrect hits.
- Stop if allocation reduction regresses focused latency by more than 5% without documented cause.
