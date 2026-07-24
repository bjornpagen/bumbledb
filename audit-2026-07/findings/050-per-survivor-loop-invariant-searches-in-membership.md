## Per-survivor loop-invariant searches in membership/anti-probe and routing passes

category: inappropriate-branching | severity: medium | verdict: CONFIRMED | finder: cross:branching

### Summary

The pipelined executor documents its own hot-loop discipline at the head of the sibling passes: "value sources resolve once per (pass, subatom) — never a per-element variable search" (crates/bumbledb/src/exec/run/probe_pass.rs:51-55). Every residual arm in the same function obeys it — word bases are hoisted outside the per-survivor loop (probe_pass.rs:242-243, 269-273, 303-304, 340-342). Three code paths break it:

1. **Membership (point) probes** recompute `word_base` per survivor per part and run a linear `position()` search over `node.subatoms` per survivor, though both depend only on (node, cover choice, spec).
2. **The routing `assemble` closure** repeats the same `position()` search per routed survivor per assembled occurrence in the Descend loop — this one runs for every multi-node plan, not just membership/negation plans.
3. **Anti-probe point parts** call `word_base` per element via the `point_word` closure, while the same function correctly hoists its KEY sources per spec — an inconsistency inside one function.

The deeper representational miss: `VarId` is dense by design ("Variables carry dense ids only", crates/bumbledb/src/ir.rs:6-7), yet every width/offset lookup is a linear scan of an association list instead of a `Vec` indexed by the id.

### Evidence (all verified against the code)

- **probe_pass.rs:385-414** — inside `for k in 0..n` (per survivor), per point part: `let point = super::word_base(cover_vars, *var, |v| self.width_of(v)).map_or_else(...)` (line 390); then per survivor, `node.subatoms.iter().position(|sub| usize::from(sub.occ.0) == spec.occ)` (lines 399-402) to relocate a plan-static occurrence.
- **probe_pass.rs:242-243, 269-273, 303-304, 340-342** — the contrast: ordinary, word, Allen, and duration residuals all hoist `word_base` outside the k loop, once per residual per pass. The point-probe arm is the deviation, not the rule.
- **probe_pass.rs:509-528** — the `assemble` closure, invoked per routed survivor per occ (leaf arm: every leaf subatom + leaf point probe, lines 540-550; middle arm: every carried occ, lines 573-576), performs the same `subatoms.iter().position(...)` search at 516-519 before falling through to `tables.carried_col[node_idx][occ]` — which is already a dense occ-indexed table.
- **run_node.rs:414-441** — the line-parallel twin: `word_base` per survivor per part at 419-423, `position()` per survivor at 428-431.
- **anti_probe.rs:67-76** — `point_word` closure calls `word_base(cover_vars, var, width_of)` per element; used per element at 111-117 (keyless membership gate) and 216-222 (keyed probe with memberships). The same function hoists its keyed sources once per spec at 130-145 (`sources.clear(); for (var, slot, width) in &spec.parts { match word_base(...) ... }`).
- **execute.rs:349-355** — `width_of` is `self.var_widths.iter().find(|(v, _)| *v == var)`, a linear association-list scan.
- **run.rs:378-391** — `word_base` re-accumulates widths linearly over `cover_vars`, calling `width_of` (itself linear) per cover variable — O(|cover_vars| × |vars|) per call.
- **fj.rs:383-392** — `ValidatedPlan::slot_of` recomputes a linear prefix sum over `self.slots` per call.
- **ir.rs:6-7** — "Variables carry dense ids only" — the dense-id representation exists; the lookup tables it licenses do not.
- **pipe_tables.rs:27-43** — `carried_col` is a `vec![None; n_occ]` per node, a dense occ-indexed table: the suggested representation already half-exists and `assemble`/the point-probe cursor arm consult it only after the linear search fails.
- **Cover choice is runtime but small-domain**: `better_cover` (cover.rs:15-21) picks the cover per pass from the node's static subatom set, so a per-(node, cover-subatom) precomputed table covers every runtime choice.

**Paper/doc check**: Free Join §4 vectorized execution (docs/free-join-paper/arXiv-2301.10841v2/tex/04-optimizations.tex, sec:vectorized-execution) exists precisely to amortize per-tuple overheads across a batch; per-survivor recomputation of pass-invariant facts runs against that purpose. The membership/anti-probe/routing passes are bumbledb extensions beyond the paper, so the binding specs are the code's own instruction-diet comment (probe_pass.rs:51-55) and docs/design/representation-first.md (§2: branches guarding states a precise representation would erase; the doctrine explicitly ranks "table" among the enforcement mechanisms prose may not substitute for). Both are violated.

### Bench impact

Plans with interval point bindings (membership probes) or negated membership atoms pay O(survivors × parts × |cover_vars| × |vars|) redundant scans plus a per-survivor `position()` search in the Residual phase of both `run_node` and `probe_pass`; the routing Descend loop pays the `position()` search per survivor per assembled occ in **every** multi-node plan. These are the same inner loops the file itself tunes at single-digit-percent granularity (the const-arity hash dispatch at probe_pass.rs:97-104 was kept for measured 1.7-5.5% wins), so the overhead is in-scope for the codebase's own bar, though its exact magnitude is unmeasured here. The refuted batching experiment recorded at probe_pass.rs:447-460 armed a different mechanism (run-cached `load_row`/row copies) and does not cover these searches.

### Suggested fix

Precompute at `Executor` construction, per (node, cover-subatom):
- the word base of every variable referenced by that node's residuals, point probes, and anti-probes (`Option<usize>`, indexed lookup);
- an occ-indexed cursor-source table (`CoverChild | Sibling(sub_idx) | Carried(col) | Start`) consulted by the point-probe cursor arm and the routing `assemble` — the occ→sub_idx part is plan-static per node and independent of the cover choice except for the one `occ == cover_occ` compare; `carried_col` already demonstrates the shape.

Replace `var_widths`'s association list and `slot_of`'s per-call prefix sum with `Vec<u32>` tables indexed by the dense `VarId` (computed once in `ValidatedPlan`/`Executor::with_batch_size`). The runtime cover choice then indexes a table instead of re-deriving sources, and every per-element search in the survivor loops disappears — restoring the file's own stated invariant.
