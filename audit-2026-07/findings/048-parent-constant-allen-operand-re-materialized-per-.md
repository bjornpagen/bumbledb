## Parent-constant Allen operand re-materialized per element in the leaf residual pass though the const-operand kernel already exists

category: unification | severity: medium | verdict: CONFIRMED | finder: perf:olap-temporal
outcome: fixed be405715

### Summary

In `run_node`'s Allen residual pass — the leaf pass, by the entry assert — a side whose source resolves to `Source::Slot` reads the outer bindings, which are constant for the entire call (the file's own anti-probe comment says so verbatim: "Slot reads come from the outer bindings (constant across the batch)", run_node.rs:448-449). Yet the gather loop re-reads that constant `(start, end)` pair and stores it into `b_starts[k]`/`b_ends[k]` for every survivor `k`, then classifies through the general four-stream `allen_code_batch`. The kernel family already contains the exact const-right-operand shape (`codes_into_const` → `neon::allen_code_batch_const_neon`, which broadcasts the constant's two words into the b-side predicate lanes), but its only caller is the dense filtered-view path; the residual path never dispatches to it.

The shape is structural, not incidental: a cross-atom Allen residual's two vars belong to different atoms, so at most one can be among the one cover subatom's vars — at least one side is **always** `Source::Slot` when such a residual lands on the leaf. t2's `Allen(v3, v4, INTERSECTS)`, t3's mixed-mask condition, and r2's pairwise INTERSECTS legs are all cross-atom.

### Evidence (verified)

- `crates/bumbledb/src/exec/run/run_node.rs:28-31` — `assert!(node_idx + 1 == plan.nodes().len(), "run_node is the leaf pass; middle nodes pump")`: `bindings` is fixed for the whole call; residuals run before any descend-phase binding store.
- `crates/bumbledb/src/exec/run/run_node.rs:341-352` — the per-k gather: `Source::Slot(slot) => bindings.get(slot + offset)` executed inside the loop; `b_starts[k] = value(rhs_src, 0); b_ends[k] = value(rhs_src, 1);` writes the batch-constant pair into every lane, then `allen_code_batch` (line 353) reads all four streams.
- `crates/bumbledb/src/exec/run/run_node.rs:448-449` — the sibling anti-probe pass documents the invariant this finding rests on: "Slot reads come from the outer bindings (constant across the batch)."
- `crates/bumbledb/src/exec/kernel/allen.rs:154-172, 229-242` — `allen_filter_columns_const` / `codes_into_const` ("the constant's two words broadcast into the b-side predicate lanes"); `crates/bumbledb/src/exec/kernel/neon.rs:191` — `allen_code_batch_const_neon`. Grep over the crate shows the sole non-test caller is the filtered-view path `crates/bumbledb/src/image/view/apply.rs:612`; no residual path reaches the const variant.
- `crates/bumbledb/src/exec/run/run_node.rs:116-125` — the residual sources are resolved once per node (`word_base(...).map_or(Source::Slot(slot), Source::Batch)`), so the (Batch, Slot)-shape dispatch is available at bind/resolve time with zero per-element cost.
- Bench lane is real: `bench-out/night-2026-07-20/scenarios/scenarios.md:66-67` — `t2_overlap_join ... 162794.7` µs (the 162.8ms lane), with the Allen residual placed at the leaf (cross-atom, second span var bound above the leaf → the other side is `Slot`).
- Spec check: docs/architecture/40-execution.md § vectorized execution mandates the batch classify + branchless compaction shape, which the fix preserves; the Free Join paper (docs/free-join-paper) governs the COLT probe structure, not residual operand sourcing — no spec conflict.

### Correction to the original finding

The claimed "line-parallel twin" at `crates/bumbledb/src/exec/run/probe_pass.rs:310-321` does **not** share the shape. There, Slot reads go through `scratch.pending_bindings[parent * slot_count + slot + offset]` with `parent` varying per element (probe_pass.rs:312), and the pass is explicitly cross-parent ("elements drawn from many pending entries", probe_pass.rs:10-12) — the operand is per-parent, not batch-constant, so the const-operand dispatch does not apply there. The win is confined to leaf-placed Allen residuals (run_node); middle-node Allen residuals in t3/r2 that run through probe_pass correctly keep the general four-stream gather.

### Bench impact

Per classified pair the redundant work is: two constant binding reads + two 8-byte stores in the gather, and the classify kernel streaming four gathered arrays where the const variant streams two plus a register broadcast. At the finder's introspection-reported ~10.5M classified pairs in t2 that is ~168MB of pure-redundant stores plus the matching kernel loads per execution — a coherent low-single-digit-percent mechanism on the 162.8ms lane, with zero semantic surface (same codes, same mask test, same compaction). Caveat: no profile in the repo pins the Allen-residual gather's fraction of t2, and the kernel's own doc (allen.rs:89-96) records the project's discipline of pinning the phase fraction before building bind-time kernel-selection levers — the magnitude should be measured before landing, per that standing rule.

### Suggested fix

At the leaf pass's residual-source resolution (run_node.rs:115-125), dispatch on the already-computed source shapes:
- `(Batch, Slot)` / `(Slot, Batch)`: hoist the constant pair out of the loop, gather only the batch side's two streams, and classify through a public batch wrapper over the existing `codes_into_const` (it already accepts arbitrary slices — the gathered scratch streams feed it as-is; only the `pub fn allen_code_batch_const` wrapper is missing, mirroring `allen_code_batch` at allen.rs:97-110). The `(Slot, Batch)` orientation needs either operand swap + Allen-mask converse (bumbledb-theory owns the converse) or a symmetric const-left kernel twin.
- `(Slot, Slot)`: both sides batch-constant — one scalar `classify_bounds` + mask test hoisted out of the loop entirely; the mask byte is uniform for the batch.
- `(Batch, Batch)`: keep the current four-stream gather (the only shape that needs it).

Do not touch probe_pass's twin — its Slot operands are per-parent and the general gather is the honest shape there.
