# PRD 04 — The configuration kernel

**Depends on:** 03.
**Modules:** `crates/bumbledb/src/exec/kernel.rs` (+ reference), `exec/run/`
(filter + residual paths), `image.rs` (interval column reads — read-only use).
**Authority:** `40-execution.md` (batching doctrine, port-topology law, unsafe
policy).
**Representation move:** homogeneous coordinates for time — 8192 relations, one
arithmetic. A named-operator design would have shipped up to 13 kernels and a
13-way dispatch (a data-dependent branch, the one misprediction source the
executor exists to delete). The mask design ships **one branchless kernel** for
every relation that exists or ever will.

## Context (decided shape)

Evaluating `Allen(mask)` on a pair is two steps, both branch-free:

1. **Configuration code**: from the four endpoint words `(a.s, a.e, b.s, b.e)`
   compute which of the 13 basics holds. The basics are determined by the signs
   of four comparisons — `a.s ? b.s`, `a.e ? b.e`, `a.e ? b.s`, `b.e ? a.s` —
   composed arithmetically (csel/cmp shapes; mind the port-topology law: flag
   ops are confined to 3 of 6 ALUs, so prefer arithmetic composition of
   comparison results over flag chains where measured).
2. **Membership**: `(1 << code) & mask != 0` — one AND, one test, fed to the
   existing survivor-compaction machinery.

Uniform cost for all masks; the mask lives in a register for the whole batch
(literal) or is loaded once per execution (param). No new NEON widths: the
endpoint gathers are the existing two-word interval column reads doubled, and
the compare/compose shapes are sanctioned kernel forms.

## Technical direction

1. `kernel.rs`: `allen_code_batch` (gathered or strided endpoint words →
   config codes) and `allen_filter_batch` (codes + mask → survivor compaction),
   NEON under `cfg(aarch64)` with the scalar reference implementation beside
   them; both under the unsafe-allowlist law (bit-identity property test across
   randomized inputs including boundary shapes: adjacent, nested, equal, rays,
   lane-multiple ±1 lengths).
2. Wire the filter path (per-atom `Allen` against a literal/param interval on a
   filtered view) and the residual path (var-vs-var `Allen` at the earliest
   bound node) to the kernel; anti-probe interval positions ride the same code
   path inverted, exactly like every other predicate class.
3. EXPLAIN: per-node mask selectivity (est vs actual) lands in the existing
   stats surface — no new instrumentation category.

## Passing criteria

- `[test]` Bit-identity: NEON vs scalar reference vs PRD 03's `classify`, over
  randomized batches including every boundary shape and both element types.
- `[test]` End-to-end: each of the 13 singleton masks, `INTERSECTS`, `COVERS`,
  `DISJOINT`, and 32 random masks agree with the naive model on randomized
  small corpora (rays included).
- `[shape]` No 13-way match/dispatch exists on the hot path; one kernel pair
  serves all masks (grep for per-basic function names finds only the reference
  `classify`).
- `[gate]` Workspace gates green; the alloc gate unchanged (kernel scratch is
  pooled batch state).

## Doc amendments (rule 5)

`40-execution.md`: the sanctioned kernel list gains the configuration kernel;
the vectorization section's interval sentence updated (masks, not ops).
