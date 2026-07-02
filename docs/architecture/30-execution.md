# 30 — Execution

The execution engine is Free Join as specified in the paper (Wang, Willsey, Suciu,
SIGMOD 2023 — `docs/free-join-paper/`), run over snapshot-local columnar data, with a
short list of documented deviations. When this doc and the paper disagree and no
`Deviation:` block explains why, this doc is wrong.

## The paper's core, adopted as-is

- **GHT**: a trie whose internal nodes are hash maps keyed by tuples and whose leaves
  are vectors; hash tables and GJ hash tries are its two extremes (§3.1).
- **Plan**: a list of nodes, each a list of subatoms; the plan must partition every
  atom's variables; a node is valid if no two subatoms share a relation and some subatom
  (the cover) contains all variables new to the node (§3.2).
- **Execution**: recurse node by node — iterate the cover, extend the binding, probe the
  sibling subatoms, replace each atom's current trie, recurse; backtrack restores (§3.3).
- **Plan construction**: a cost-based **binary** left-deep plan → `binary2fj` →
  conservative `factor()` hoisting of probe subatoms (all-or-stop per node, preserving
  the optimizer's probe order) (§4.1).
- **COLT**: lazy tries — a node is a vector of offsets into the base columns or a forced
  hash map; the root iterates the base table directly; forcing happens only on `get` or
  non-suffix `iter`; last trie level stays a vector forever (§4.2).
- **Dynamic cover choice**: at each node entry, iterate the cover with the fewest keys;
  unforced COLT vectors expose an estimate (their length), forced maps an exact count
  (§4.4). *Exact and estimated counts must stay labeled as such* — v5 compared
  duplicate-inflated vector lengths against exact map counts and systematically chose
  wrong covers.

## Planner

The paper requires a real cost-based binary plan as input; it used DuckDB's. We grow a
deliberately boring one:

- **Statistics**: per-relation row counts (maintained on write, exact); per-filter
  survivor estimates from cheap heuristics; unique/FK constraint knowledge. No fake
  fields — a statistic that isn't real doesn't exist in the struct.
- **Join ordering**: exhaustive DP (Selinger-style) over the atoms. At this product's
  query sizes (rarely >10 atoms) DP is exact and effectively free. Estimated
  cardinality is the entire cost function.
- Then `binary2fj` + `factor()` exactly per paper, validated once into a
  `ValidatedPlan` witness.

**Alternative:** v5's "candidate families" (FilterAnchored/FactoredBinary/Singleton/…)
scored by hardcoded derivation constants plus terms identical across candidates. **Why
it lost:** it was a static preference ranking wearing a cost model's coat — the
post-mortem's central engine finding. One honest cost model beats five labeled guesses.

## Deviation blocks

**Deviation D1 — data source.** *Paper:* relations are already columnar in main memory.
*We:* durable data lives in LMDB; execution reads **cached columnar images** built once
per (relation, tx-generation) and shared across read transactions (`40-storage.md`).
*Why:* LMDB is the only durable truth; at ≤100s of MB the whole working set caches, so
after warmup execution is exactly the paper's environment. *Reverses if:* image build
cost dominates real traces despite caching — then persist columns instead.

**Deviation D2 — set semantics.** *Paper:* bag semantics; leaves may carry multiplicity;
output is a stream of tuples. *We:* sets everywhere. Trie leaves are membership, never
counts. Projection deduplicates through the sink, and when the remaining plan suffix
can only multiply witnesses of an already-emitted projection (existential-only
variables), the executor skips the subtree — an optimization bag semantics cannot take.
*Reverses if:* never; this is product semantics.

**Deviation D3 — sinks, not output().** *Paper:* `output(tuple)` materializes full join
results; aggregation is post-processing. *We:* the executor emits complete bindings to a
private sink trait; projection-dedup is one sink, aggregate folds (Sum/Min/Max/Count,
grouped by the non-aggregated finds) are another. Aggregation never materializes the
join. *Why:* the ledger workload's primary queries are folds; materialize-then-fold is
pure waste. *Reverses if:* never structurally; individual sinks may change.

**Deviation D4 — vectorization tuned to Apple Silicon.** *Paper:* batch the cover
iteration and probe siblings per batch (§4.3), batch size unspecified, generic hardware.
*We:* the same algorithm with the batch sized to the M-series memory subsystem — enough
independent probes in flight to fill ~28 MLP lanes (batches of 64–256, tuned by
measurement, not faith); probe key computation and hash mixing done for the whole batch
before any lookup so loads issue independently; NEON (128-bit, `cfg(aarch64)`) kernels
for fixed-width predicate scans and survivor compaction; columns 128-byte aligned.
Scalar fallback everywhere, results identical by test. **Vectorized execution is the
default path, not a mode** — v5 shipped a "vectorized" mode that ran the scalar path
per tuple inside a batch and was never turned on; a fake mode is worse than none.
*Reverses if:* measured equal-or-worse than scalar on the ledger suite after honest
tuning.

**Deviation D5 — no DuckDB.** *Paper:* uses DuckDB's optimizer for the binary plan.
*We:* the DP planner above. *Why:* no external SQL engine may be product infrastructure.

## The allocation contract

**Executing a prepared query performs zero heap allocations in steady state.** All
scratch — binding slots, probe key buffers, batch buffers, COLT arenas, sink state —
comes from per-query reusable arenas owned by the prepared query or the transaction
context. The result buffer is the single sanctioned allocation site (and callers can
provide one). CI enforces this with a counting allocator on representative queries; the
counter is a hard gate, one of the very few gates this project keeps.

COLT forcing allocates *within the arena*, proportional to distinct keys — force is
single-pass (the v5 implementation decoded and hashed every row twice to get contiguous
child ranges; contiguity is not worth 2× work — children chain within the arena).

## Observability

`EXPLAIN` (plan + per-node estimated vs actual cardinalities + cover choices) exists
from day one — it is the debugging story for a database with no REPL. Tracing is a
compile-time feature that is **structurally absent from release builds**: no
per-tuple label formatting, no counters accumulating on hot paths, no diagnostics
allocation in the innermost loops. v5 wove ~1,100 lines of span plumbing through the
executor and one piece allocated per-binding in release; the lesson is a rule: the hot
loop's shape is dictated by the algorithm, never by its instrumentation.
