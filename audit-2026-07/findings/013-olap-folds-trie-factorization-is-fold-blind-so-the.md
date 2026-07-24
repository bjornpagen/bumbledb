## OLAP folds: trie factorization is fold-blind, so the scan-fold pushdown never fires for single-atom GROUP BYs (o3/o5 run 16-18x behind o1)

category: missing-free-feature | severity: high | verdict: CONFIRMED | finder: perf:olap-temporal
outcome: fixed 73215a30 + 99fa5015

### Summary

The aggregate sink has a fully kernelized scan-fold pushdown (`begin_scan`/`scan_run`: SIMD fold kernels measured at ~7.9-8.0 rows/ns), but it must decline whenever a group variable is one of the leaf trie level's key words. The planner never arranges the trie to avoid this: `binary2fj` transcribes the paper's Fig. 7 and gives the first occurrence its full atom as one flat trie level, with no knowledge of the rule's group variables. Any single-atom GROUP BY — the most common OLAP shape — therefore puts the group variable inside the leaf level, the pushdown declines, and every row takes the scratch-fill + WordMap-hash-probe path. Verified live on the bench corpus: `o3_promo_split` and `o5_store_extremes` fold 500k rows per-row at 7.1-8.2ms, while `o1_revenue_by_region` — identical fold volume, but its group variable happens to bind at node 0 through the Store dimension — runs the pushdown and finishes in 446us.

This is an inappropriate-representation finding in the project's own terms (docs/design/representation-first.md): the sink's runtime decline at sink.rs:33-39 is a branch compensating for a representation (the trie schema) that could have made the slow case unrepresentable.

### Evidence

All citations personally verified; introspection reproduced by loading the actual 525k-row olap corpus and calling `Snapshot::introspect` on the three queries.

- **The decline**: `crates/bumbledb/src/exec/sink/aggregate/sink.rs:33-39` — `begin_scan` returns `false` when any group-span word is in `scan.key_slots`. This is the only decliner that fires here: introspection shows `distinct_bindings: proven` (no seen-set) and Sum/Min/Max are not `row_fold_only`.
- **The forfeited kernels**: `crates/bumbledb/src/exec/kernel/fold.rs` (`fold_sum_u64_dense` and kin, ~7.9-8.0 rows/ns per the in-file measurement record), reachable only via `scan_run` (sink.rs:82-126).
- **The per-row fallback taken instead**: `emit_batch`'s varying-group arm (sink.rs:212) → `fold_batch_rows` (`crates/bumbledb/src/exec/sink/aggregate/fold_batch.rs:11-21`): per-row key-slot scratch fill, then `fold_scratch_row` → `load_group_key` → `probe_group`, a WordMap hash probe per row (`fold_row.rs:47-50`, `groups.rs:49-55`).
- **The fold-blind planner**: `crates/bumbledb/src/plan/fj/binary2fj.rs:29-37` — the first occurrence contributes its full atom as one node/level; `crates/bumbledb/src/plan/fj/validate.rs:66-73` derives `trie_schema` purely from subatom var-lists. No pass in `plan/` reads the rule's group variables when shaping levels (grep over `plan/` confirms).
- **Live introspection (reproduced)**:
  - o1: `occurrence 0: relation 3 trie schema [[3], [2, 1]]`, `occurrence 1: relation 0 trie schema [[3, 0]]`, node 1 `entries=200` — group var 0 (region) is outer at the leaf; pushdown fires.
  - o3: `trie schema [[2, 1, 0]]`, 1 node, `entries=1` — group var 0 (promo) is a leaf key word; pushdown declines.
  - o5: `trie schema [[2, 0, 1]]`, 1 node, `entries=1` — group var 0 (store) is a leaf key word; pushdown declines.
- **Bench numbers**: `bench-out/night-2026-07-20/scenarios/scenarios.md:32-36` — o1 445.9us, o3 7132.0us (16.0x), o5 8151.6us (18.3x), same 500k-row fold volume (scenario defs: `crates/bumbledb-bench/src/scenarios/olap.rs:146-172, 230-250, 288-312`).
- **Paper check**: the Free Join paper's GHT schemas are free per-relation level lists chosen by the plan (`docs/free-join-paper/arXiv-2301.10841v2/tex/03-free-join.tex:87, 124-131`; Fig. 7 is what binary2fj.rs:7-11 says it transcribes). §4's optimizations (COLT, vectorization) never consider sink-aware level splitting — the code follows the paper faithfully; the paper simply has no aggregate pushdown, so this is a missing extension, not a spec divergence.

### Bench impact

o3_promo_split (~7.1ms) and o5_store_extremes (~8.2ms) are two of the three slowest OLAP lanes. With the group variable split into its own trie prefix level (o3: [[0],[2,1]], 2 subtries; o5: [[0],[2,1]], 200 subtries), every group becomes scan-constant at the leaf and both lanes enter o1's regime: 2 or 200 kernelized scan-folds over suffix runs instead of 500k scratch fills + WordMap probes. o1 is the measured counterfactual at 446us for the same volume, so sub-ms is the expected landing zone (8-19x). The same rule benefits every single-atom GROUP BY.

### Suggested fix

Thread the rule's group-variable set into trie-schema construction: when a node's opening level mixes group variables with fold-domain variables, split the level so the group variables form their own prefix level (o5: `[[0],[2,1]]`). This is ordinary existing machinery — o1's leaf is already exactly this shape (a `[2,1]` level under an outer-bound group var), executed by the same `begin_scan` path, and `derive_nodes`' cover rule (`plan/fj/derive_nodes.rs:29-49`) accepts the split since each node's opening subatom is exactly its new vars. Cost model: the split multiplies node entries by |groups|, which is precisely the quantity the sink already relies on being small; gate the split on the group column's domain (closed relations and small dimensions give it for free) if a guard is wanted at all — though the representation-first move is to make the split the default for aggregate rules.
