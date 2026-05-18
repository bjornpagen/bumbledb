# 04: Real HashProbe Runtime

**Goal**
- Turn `NodeImpl::HashProbe` from an optimizer/explain label into a real runtime kernel using `HashTrieIndex` and `PrefixProbe`.

**Trace Evidence**
Current explain chooses `hash_probe`, but runtime counters are sorted-trie counters:

| Query | Avg | Plan | Trie Evidence |
|---|---:|---|---:|
| `ledger/tag_lookup_join` | `51.8ms` | `hash_probe` | `trie_open=10002`, `trie_key_reads=60000` |
| `sailors/red_boat_sailors` | `57.6ms` | `hash_probe` | `trie_open=18328`, `trie_key_reads=105789` |
| `tpch/supplier_nation_orders` | `64.4ms` | `hash_probe` | `trie_open=14292`, `trie_key_reads=50013` |

**Current Code Facts**
- `NodeImpl::HashProbe` exists in `free_join.rs`.
- Optimizer can select `hash_probe`.
- `execute_query` still routes through `execute_lftj` unconditionally.
- `hash_trie.rs` has `HashTrieIndex`, `LeafMode`, `PrefixProbe`, and `RowSetRef` but production query execution does not use them.

**Required Design**
- Add `execute_free_join` dispatcher.
- Compile each `PlanNode` into runtime nodes:
  - `SortedLeapfrog`
  - `HashProbe`
  - later `Hybrid`
- Keep one Free Join architecture: shared bindings, predicates, sinks, counters.
- `HashProbe` nodes use cached `HashTrieIndex` instances over `QueryImage` relation columns.

**HashProbe Semantics**
- Build prefix from bound vars, encoded inputs, and encoded literals.
- Probe row sets via `HashTrieIndex::rows/count/exists`.
- Bind newly produced variables from `RelationImage` columns.
- Check repeated variables and residual predicates.
- Deduplicate emitted assignments for node-local set semantics where needed.
- Emit final bindings through existing `TupleSink`.

**Counters**
Add actual hash counters:

```rust
hash_index_builds
hash_index_build_rows
hash_probe_calls
hash_probe_hits
hash_probe_misses
hash_rows_returned
hash_distinct_emits
```

**Implementation Steps**
1. Add Free Join dispatcher instead of unconditional `execute_lftj`.
2. Add runtime hash index cache keyed by image/relation/access/fields/leaf mode.
3. Compile `PlanNode + NormalizedQuery + schema + image` into `HashProbeRuntime`.
4. Implement driver subatom selection.
5. Implement row binding from `RowSetRef`.
6. Implement filter/existence subatoms.
7. Add explain/benchmark hash counters.
8. Keep LFTJ fallback explicit for unsupported shapes.

**Tests**
- HashProbe exact static lookup returns rows.
- HashProbe bound FK lookup binds payload fields.
- Missing prefix returns no rows.
- Repeated variable fields enforce equality.
- Existence-only probe uses `exists/count` without row materialization.
- Forced sorted vs forced hash output equivalence for target queries.
- All-HashProbe plans show zero trie counters and nonzero hash counters.

**Acceptance Criteria**
- `NodeImpl::HashProbe` has a distinct runtime path.
- `tag_lookup_join`, `red_boat_sailors`, and `supplier_nation_orders` return identical rows to SQLite/reference.
- For all-HashProbe versions of those queries: `trie_open=0`, `trie_next=0`, `trie_seek=0`, `trie_key_reads=0`.
- Actual hash counters appear in explain and markdown.
- Warm scale-10000 target queries average under `5ms` or document blockers with phase timings.

**Risks**
- Duplicate row paths can break aggregate semantics if node-local distinctness is wrong.
- Per-query hash index builds can erase gains; cache by `QueryImageKey`.
- Overusing hash on cyclic joins can regress worst-case behavior.
