# PRD 04 — The configuration kernel

**Depends on:** 03 (landed — the mask, `classify`, and the lowered shapes are in; this PRD replaces their scalar evaluation with the configuration kernel).
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

Evaluating `Allen(mask)` on a pair is branch-free, **flag-free**, and
table-driven — each property cited to a ledger fact
(`docs/reference/apple-silicon-performance.md` register):

1. **Predicates**: the 13 basics are determined by four three-valued
   comparisons — `a.s ? b.s`, `a.e ? b.e`, `a.e ? b.s`, `b.e ? a.s` — produced
   as 8 predicate lanes via `cmhi`/`cmeq` pairs on the vector pipes. The i64
   sign-flip encoding makes every comparison unsigned, so one kernel serves
   both element types, rays included (PRD 02).
2. **Classification**: pack the predicates into a ~7-bit signature (strict
   nonemptiness admits exactly 13 valid configurations) and map signature →
   4-bit basic code through a **64-byte nibble table held in q registers via
   `tbl`** — the Allen decision tree as in-register data: zero memory traffic,
   zero branches, zero flags. The measured alternative (a 256×u16 one-hot
   table, 512 B, permanently L1-hot, one pipelined load per pair) trades `tbl`
   arithmetic for load-port pressure; prefer `tbl` in filter position (load
   ports busy streaming columns), the table load in residual position (gather
   context, load ports idle). The choice is a sweep, not a doctrine.
3. **Membership**: `(1 << code) & mask != 0` — the mask lives **broadcast in a
   vector register** for the whole batch (literal or param alike); survivors
   feed the existing branchless compaction (1.00 cy/item cursor-write).

**The flag-free law is load-bearing, not style** (`m2max.core.flag-port-asymmetry`,
`m2max.core.flag-strand-mlp`): a scalar `cmp`/`csel` classify carries 4–5 flag
µops per pair — capped at ~2.8 flag-µops/cycle on the 3-port triad dense, and
**halving sustainable miss lanes (~28 → ~14) when the pairs are gathered** at
DRAM tier. The NEON route keeps dependents on the vector schedulers and
preserves the lanes; this is `m2max.simd.minmax-universal`'s mechanism (2.65×,
port-arithmetic-predicted) applied to Allen. Zero scalar flag µops exist in the
hot path, enforced by the asm gate.

Uniform cost for all 8192 masks — no per-mask codegen, no 13-way dispatch, no
indirect branch; mask-as-param is free by uniformity. No new NEON widths: the
endpoint reads are the existing two-word interval column shapes doubled, and
compare/`tbl`/compaction are sanctioned kernel forms.

**Bind-time mask simplification is recorded as a measured-later lever, not
shipped.** The workload composites collapse (`INTERSECTS` = `a.s < b.e ∧
b.s < a.e` — two compares; `DISJOINT` its complement; `COVERS` three), and on
L2-resident retire-bound filters (`m2max.mem.l2-resident-retire-bound`) a
two-compare kernel beats the uniform one on µop count alone. The structure, if
earned, is bind-time monomorphized selection (the sink-dispatch precedent — no
hot-loop indirection). *Trigger:* PRD 16's calendar family showing the filter
phase owning enough of a family budget to buy it — pin the fraction before
building the lever (`m2max.probe.pass-overhead`'s lesson).

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
  `classify`); the asm gate proves the hot path free of scalar flag-writing
  instructions (`cmp`/`csel`/`adds`/`ccmp`) — the flag-free law, structural.
- `[test]` Falsifier-shaped performance pins (recorded as NOTEs until PRD 16
  earns numbers): the dense uniform kernel within 2× of a hand-written
  two-compare `INTERSECTS` loop at L1 (else the signature packing is fat); a
  gathered Allen residual at DRAM tier within 15% of the flag-free xor-gather
  floor (else a flag µop leaked into the miss shadow — read the disassembly;
  LLVM substitutes).
- `[gate]` Workspace gates green; the alloc gate unchanged (kernel scratch is
  pooled batch state).

## Doc amendments (rule 5)

`40-execution.md`: the sanctioned kernel list gains the configuration kernel;
the vectorization section's interval sentence updated (masks, not ops).
