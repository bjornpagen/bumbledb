## run_node's prefetch comment claims a residency gate the re-simplify ablation deleted — the code is width-only

category: incoherence | severity: low | verdict: CONFIRMED | finder: perf:rings
outcome: fixed 0f13feff

### Summary

The phase-1.5 prefetch comment in `run_node.rs` says the pass is "Gated on RESIDENCY first (an L2-resident map's prefetch is pure loss) and batch width second", but the code gates only on `!pinned && scratch.survivors.len() >= PREFETCH_WIDTH_FLOOR`. The residency/footprint tier was deleted by the re-simplify commit (S2-PRD 10, `2d112084`), and the `PREFETCH_WIDTH_FLOOR` doc block is an explicit gravestone recording that the tier was ablated twice, measured NEUTRAL, and refused ("width-only stays"). The stale comment resurrects the buried mechanism — and the two sibling prefetch sites both defer to it with "see run_node", so the wrong text is the canonical description.

### Evidence

- `crates/bumbledb/src/exec/run/run_node.rs:229-234` — comment: "Gated on RESIDENCY first (an L2-resident map's prefetch is pure loss) and batch width second (tiny batches never amortize the pass)." followed by `if !pinned && scratch.survivors.len() >= PREFETCH_WIDTH_FLOOR {` — no residency/footprint term.
- `crates/bumbledb/src/exec/run/run_node.rs:200` — `let pinned = matches!(s_cursor, Cursor::Row(_));` with its own comment (:195-199) explaining it flags field-equality probing on a pinned sibling (skip hash work). It is a probe-shape flag, not a cache-residency check, so `!pinned` cannot be read as the comment's "RESIDENCY" clause.
- `crates/bumbledb/src/exec/run.rs:403-421` — the `PREFETCH_WIDTH_FLOOR` doc block: "the ONLY prefetch gate", the former footprint tier (2 MiB → 256 KiB) "was ablated at the bucket-layout probe floor and measured NOTHING at family level", re-ablated 2026-07-17 on the displaced lanes, "NEUTRAL: no regime gives the tier's comparison anything to decide — width-only stays."
- `git show 2d112084` — the re-simplify commit ("Deleted at measured <2%: the prefetch footprint tier ...") removed `&& colts[occ].probe_footprint_bytes() > PREFETCH_L2_BUDGET_BYTES` from this exact gate; the diff shows the "RESIDENCY first" comment lines survived as unchanged context. The stale text dates to 2026-07-07.
- `crates/bumbledb/src/exec/run/probe_pass.rs:185` ("Phase 1.5, width-floor gated — see run_node.") and `crates/bumbledb/src/exec/run/anti_probe.rs:175` (same) — both sibling prefetch sites carry the honest one-liner and redirect the reader to the stale description.

This is consistent with the Free Join paper's role here only as background (the prefetch pass is a repo-local COLT probe optimization, not a paper mechanism); the governing spec is the repo's own ablation record in the `PREFETCH_WIDTH_FLOOR` doc block, and the comment contradicts it.

### Failure scenario

Documentation-only — no runtime effect. But the next tuner reading run_node (or arriving via the two "see run_node" pointers) will search for a residency check that does not exist, and the comment asserts as live design ("an L2-resident map's prefetch is pure loss") precisely the isolation-law claim the silicon2 measurements overturned and the doc block refutes.

### Suggested fix

Rewrite the run_node phase-1.5 comment to match the `PREFETCH_WIDTH_FLOOR` gravestone: width-floor is the only prefetch gate (`!pinned` is the hash-probe applicability condition, not a gate); the residency/footprint tier was ablated twice and measured NEUTRAL — cite the doc block rather than restating the dead mechanism.
