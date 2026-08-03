## OVERLAP_CROSSOVER gates on group size alone; once-probed groups pay the sort-and-tree build for a single query, ~8-10x the generic path's leaf work, and no rig lane can see it

perf | medium | CONFIRMED | overlap-join-live
outcome: fixed 8d17ac59

### Summary

The leaf overlap index's admission gate (`crates/bumbledb/src/exec/run/overlap_leaf.rs:73`) admits any group of ≥ `OVERLAP_CROSSOVER = 16` positions, but the index's economics are amortization-shaped: the build cost (start-sort + max-end tree, `interval/overlap.rs:117-141`) is paid per group while the win is per query. The constant's justifying arithmetic (overlap_leaf.rs:25-29, "a 16-group's all-pairs classify ≈ the sort the index pays once") holds only in the self-join shape where each group is queried ~n times. In a shape where the outer side carries one row per key, every group is built for exactly one query, and measurement against the real `OverlapCache` shows the index path costs **8.06x (n=16), 8.70x (n=64), 10.23x (n=256)** a scalar linear classify over the same triples — a conservative proxy for the generic path (the real path uses NEON classify kernels). The gate cannot express this because queries-per-group is not in it, and the mandated measurement rig sweeps only the amortized self-join shape.

### Evidence (verified)

- `crates/bumbledb/src/exec/run/overlap_leaf.rs:73-77` — the gate is size-only: `colt.key_count(cover_cursor).magnitude() < OVERLAP_CROSSOVER || !colt.suffix_scannable(...)`. No queries-per-group signal exists or can exist here.
- `crates/bumbledb/src/exec/run/overlap_leaf.rs:101-124` — the build (`get_or_build` with the triple-feeding closure) fires inside `overlap_enumerate`, i.e. inside the per-outer-row leaf call.
- `crates/bumbledb/src/exec/run/run_node.rs:149-158` and `probe_pass.rs:576-598` — `overlap_enumerate` runs once per parent row (`if leaf { ... self.run_node(plan, node_idx + 1, ...) }` per surviving outer binding), so first touch of a key group = build + one query.
- `crates/bumbledb/src/interval/overlap.rs:114-141` — the build is `sort_unstable_by_key` over the whole group's `(u64, u64, u32)` triples plus a `2p`-word max-end tree, three slab extends, and a directory insert.
- `crates/bumbledb/src/interval/overlap.rs:19-23, 92-99` — the cache resets per `execute`; amortization exists only within one execution.
- `crates/bumbledb/src/exec/run/tests/intervals.rs:893-944, 1311-1334` — the mandated rig (`overlap_profile`, the doc's "re-pin this number from that sweep") sweeps only `keyed_span_query`: the two-occurrence self-join of `RelationId(0)` (`R(a,k,u), R(b,k,v), a<b`), where queries-per-group ≡ group size. No one-query-per-group lane exists anywhere in the rig or bench estate.
- `git log -S OVERLAP_CROSSOVER` → only `be405715` (the introducing commit). The constant has never been re-pinned; its own doc says "16 is provisional... re-pin this number from that sweep, never by inspection".
- **Measured** (temporary test against the real `OverlapCache`, release, 100k groups, build-once + query-once per group vs scalar linear `start < q_end && q_start < end` over the same triples, identical hit counts both sides):
  - n=16: index 33.0ms, generic 4.1ms — **ratio 8.06**
  - n=64: index 118.0ms, generic 13.6ms — **ratio 8.70**
  - n=256: index 520.1ms, generic 50.8ms — **ratio 10.23**

  The scalar comparison is an upper bound on the generic path's per-element cost (the real path is contiguous gathers + NEON classify kernels), so the true regression is at least this large. The measurement rig was reverted after use.

### Failure scenario / impact

Schema `R(id, key, interval)`, `S(id, key, interval)`; R has 100k keys × 1 row, S has 100k groups of ~64 rows; query `R(a,k,u), S(b,k,v), Allen(u,v, INTERSECTS)`. The mask is connected, groups pass the ≥16 gate, endpoints are word columns — the index path fires on every leaf call. Every group is built fresh (triple walk + sort of 64 + 128-word tree + slab writes + directory insert) for exactly **one** `query_into`, then discarded at execute end (per-execution reset). Total leaf enumeration work is ~9x the generic path this replaced, growing with group size. Correctness tests pass (the index is complete), and neither the `overlap_profile` sweep nor any bench lane (`t2_overlap_join` is the per-key self-join) measures this shape — the regression is structurally invisible to the existing rig.

### Suggested fix

Make the gate self-amortizing instead of size-gated: on first touch of a key, the cache records the key with a sentinel dir (the directory probe already runs on every touch — `lookup` at overlap.rs:179), the first query runs the generic enumeration, and the build fires on the SECOND touch of the same key. Worst case becomes generic + O(directory probe); the self-join shape builds on outer row 2 of n (losing one row's amortization, noise at crossover scale); and `OVERLAP_CROSSOVER` then prices only group size against per-query walk cost — exactly what the existing `overlap_profile` sweep measures. Then run the sweep and re-pin 16 as the constant's own doc already demands, and add a one-query-per-group lane (two-relation, 1-row outer keys) to the rig so the unamortized shape stays measured.