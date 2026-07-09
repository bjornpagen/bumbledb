# PRD 15 — Elegance: exec and image

**Depends on:** 14. **The hot-module pass — the README's touch-only-with-cause
discipline binds hardest here.** Every unsafe-allowlisted module change needs a
stated cause in the findings list and a re-bench flag.
**Modules:** `crates/bumbledb/src/exec.rs` + `exec/` (run, colt, sink, kernel,
wordmap, dispatch, explain), `crates/bumbledb/src/image.rs` + `image/`,
`crates/bumbledb/src/obs.rs` + `obs/`.

## Subsystem-specific hunt list (verify, don't assume)

- **The two probe passes:** `run_node.rs` and `probe_pass.rs` both host
  membership-probe and anti-probe evaluation loops with near-identical
  batch/mask/compact structure (visible in their parallel point-check
  assembly). If the bodies differ only in cursor sourcing and binding reads,
  extract the shared pass with the varying parts as parameters — **but only if
  the extraction monomorphizes identically** (no `dyn`, no new indirection in
  the hot loop; verify with `check-asm.sh` after). If it cannot be extracted
  without indirection, normalize the two copies line-for-line and cross-comment
  them so drift is visible — the honest fallback.
- **Scratch struct sprawl:** `scratch.*` fields accreted across PRDs 16–18 of
  the rebuild (masks, point_checks, survivors, parents, pending_*, origins) —
  group by lifecycle (per-node vs per-batch vs per-execution) into named
  sub-structs if the flat struct has stopped reading; do not shuffle fields
  that the hot loops index by offset patterns without checking the generated
  code.
- **Sink helpers:** projection dedup, aggregate seen-set, CountDistinct value
  sets, and Arg row-sets all hash word spans — confirm one span-hashing helper
  serves all four (PRD 18 was instructed to reuse; verify it happened).
- **Kernel composition sites:** PRD 17 composed interval filters from existing
  predicate-scan primitives — check the composition sites for copy-paste
  between the five shapes; a shape table (const array of column/op tuples)
  may collapse them if it costs nothing at monomorphization.
- **EXPLAIN/counters:** the `Counters` trait gained anti-probe, judgment, and
  chase surfaces across eras — check the Noop impl is still exhaustively
  zero-sized and the counting impl has no drift between what it counts and
  what the report prints.
- **Image evaluators:** the view filter evaluator's scalar and NEON arms —
  confirm the scalar arm is the reference the property tests actually compare
  against (not a third copy).

## Passing criteria

As PRD 12's, applied to this subsystem. Additionally:
- `[shape]` Every hot-module diff hunk is justified by name in the findings
  list and carries the re-bench flag.
- `[gate]` `scripts/check-asm.sh` green on a release build after the pass (the
  probe-loop properties are the canary for accidental indirection).
- `[gate]` Workspace gates green, alloc gate + escalating variant included.
