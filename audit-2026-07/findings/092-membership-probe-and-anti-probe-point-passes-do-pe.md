## Membership-probe and anti-probe point passes re-derive loop-invariant sources per element, against the executor's own instruction diet

category: inelegance | severity: low | verdict: CONFIRMED | finder: engine:run
outcome: fixed 0f13feff

### Summary

The vectorized executor's stated discipline — written at the head of the sibling-probe loop — is that value sources resolve once per (pass, subatom), "never a per-element variable search" (crates/bumbledb/src/exec/run/probe_pass.rs:51-53). Every probe and residual pass honors it except the point-membership probe passes and anti_probe's point-variable helper. Those re-run, per surviving element per part inside the k-loop: `word_base(cover_vars, var, width_of)` — a linear scan over `cover_vars` that calls `width_of`, itself a linear `find` over `var_widths` — plus a `subatoms.iter().position(...)` search to locate the cursor source. All inputs (`spec.parts`, `cover_vars`, `spec.occ`, `node.subatoms`) are fixed for the whole pass. anti_probe additionally erases its `read_slot` accessor to `&dyn Fn(usize, usize) -> u64` on a path whose module header declares "no `dyn` anywhere in the hot path" (crates/bumbledb/src/exec/run.rs:6-7).

### Evidence (all verified in source)

- crates/bumbledb/src/exec/run/run_node.rs:414-424 — inside `for k in 0..n`, per part: `super::word_base(cover_vars, *var, |v| self.width_of(v)).map_or_else(...)`. Then run_node.rs:426-436: per-element `plan.nodes()[node_idx].subatoms.iter().position(|sub| usize::from(sub.occ.0) == spec.occ)` to pick the cursor.
- crates/bumbledb/src/exec/run/probe_pass.rs:385-410 — the line-parallel twin: per-element word_base at 390-394, per-element position search at 397-410.
- crates/bumbledb/src/exec/run.rs:378-391 — `word_base` walks `cover_vars` calling `width_of` per entry; crates/bumbledb/src/exec/run/execute.rs:349-355 — `width_of` is a linear `find` over `var_widths`. So the per-element cost is ~|cover_vars|·|vars| comparisons per part.
- crates/bumbledb/src/exec/run/anti_probe.rs:67-76 — `point_word` takes `read_slot: &dyn Fn(usize, usize) -> u64` and calls `word_base` per invocation; invoked per element from the keyless membership gate (anti_probe.rs:108-121) and the keyed phase-2 arm (anti_probe.rs:203-228). The pass-level parameter is already generic (`read_slot: impl Fn(usize, usize) -> u64`, anti_probe.rs:56) — only `point_word` erases it.
- Contrast, same file(s): the sibling-probe pass resolves `scratch.sources[sub_idx]` once per pass before its k-loop (probe_pass.rs:68-80); the comparison/word/Allen/measure residual passes hoist `word_base` above their k-loops (probe_pass.rs:242-243, 269-273, 303-304, 340-342); anti_probe's own keyed sources build `anti_sources[a_idx]` once per spec (anti_probe.rs:130-146). The point passes are the lone exception.
- Doc check: docs/architecture/40-execution.md:85 records point-membership scans as O(n) image-scan-plus-filter per probe — `any_position_matches` likely dominates per element, so this is a constant-factor tax, matching the low severity. The Free Join paper does not cover membership probes (bumbledb's interval extension); the governing spec is the in-file instruction-diet comment and the module's dyn-free declaration, both of which this code contradicts.

### Bench impact

Any plan with point probes or membership-carrying anti-probes over wide batches pays, per surviving element per filter: ~|cover_vars|·|vars| comparisons for word_base/width_of per part, a subatom position scan, and (anti-probe) an indirect `&dyn Fn` call — where one per-pass resolved (Source, cursor-source) table would leave a single indexed load per element. Point-membership-heavy interval lanes improve by a constant factor; the doctrine cost (the file states a diet it does not keep, and a dyn-free claim it does not keep) is the larger finding.

### Suggested fix

Give the point-probe evaluation the same representation its siblings already use: before the k-loop, resolve `Vec<(start_col, end_col, Source)>` for `spec.parts` (Batch(base) vs Slot(slot), exactly the `anti_sources` shape at anti_probe.rs:130-146) and a small cursor-source enum `{CoverChild, Sibling(sub_idx), Carried(col), Start}` computed once from `spec.occ`; run the k-loop over indexed reads. In anti_probe, make `point_word`'s `read_slot` a generic parameter (it already is at the pass level) so the hot path stays source-level dyn-free as run.rs:6-7 promises.
