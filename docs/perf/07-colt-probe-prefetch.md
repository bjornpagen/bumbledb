# PRD 07 — COLT probes: one line per bucket, prefetched

## Purpose

Phase 2's bucket loads are the engine's real memory traffic. Two costs
show in the baseline: layout — a COLT map probe touches the `slots` array
(enum tag), the `keys` slab (separate allocation), and on hit reads a child
`Slot` — two-to-three dependent lines per probe (spread `jp_probe_n0`:
21 ns/probe on a ~100k-key map, batch 128); and scheduling — the phase-2
loop's linear-probe branches sit between independent probes, so the OoO
window is doing all the overlap work unaided. Fix the layout (one line per
bucket step), then make the overlap explicit with a prefetch pass.
(Triangle's 60 ns/probe is a batch-size-1 problem — PRD 09's; this PRD
still improves its per-probe cost.)

## Technical direction

All in `exec/colt.rs` + the phase-1/2 loop in `exec/run.rs`.

- **Bucket layout.** Restructure `Map` storage in the shared pools:
  per-map contiguous bucket slab where a bucket is
  `[ctrl: u8-packed separately] + [key words: arity × u64] + [child: u64]`
  — concretely, keep three per-map ranges as today but reorder so a probe
  step reads ONE line: interleave keys and child per slot
  (`keys_start`-style slab of `(arity + 1)` words per slot: key words then
  a packed child word), plus a `ctrl: Vec<u8>` range with 7-bit hash tags
  (as PRD 06). Pack `Slot` into one u64 word: bit 63 = node/row tag,
  low 32 = position or NodeRef index, 0 = empty is NOT usable (position 0
  is legal) — the ctrl byte carries emptiness, so the child word needs no
  reserved value; keep `Slot`'s enum as the *API* type, decoded from the
  packed word at the boundary. The singleton→chunk upgrade logic
  (`Single(pos)` → `Node`) becomes a rewrite of the packed word; the
  force-ingest append path updates in place as today.
- **Probe** (`probe_hashed`): ctrl byte first (tag mismatch/empty — no
  key line touched), then one line holds key words + child. Unchecked
  indexing after the pow2 mask, per the 00 law, with the old probe kept as
  the `#[cfg(test)]` reference for a randomized differential test
  (force + probe sequences over adversarial keys, all arities, growth
  across the 75% rehash boundary, singleton upgrades).
- **The prefetch pass (phase 1.5).** `Colt::prefetch_bucket(&self, cursor,
  level, hash)`: computes the masked slot index for a *forced* node and
  issues `prfm pldl1keep` on the bucket's key/child line (and the ctrl
  byte's line — usually the same for the run). No-op for `Cursor::Row` and
  unforced nodes. In `run_node`'s sibling pass, after phase 1 fills
  `scratch.hashes`, run a third loop between phase 1 and phase 2:
  `for k { colts[occ].prefetch_bucket(...) }` — then phase 2 as today.
  This is cheap insurance at batch 128 (the OoO window covers ~28 loads;
  the prefetch loop covers all 128 regardless of branch behavior between
  them). Gate it on `scratch.survivors.len() >= 16` — tiny batches gain
  nothing and pay the loop.
  Implement `prfm` via inline asm in a tiny helper in kernel.rs
  (`pub fn prefetch_read(ptr: *const u8)`), no-op on non-aarch64.
- **Force-ingest prefetch**: in `force`'s position walk, prefetch the
  *next* position's key words one iteration ahead (chunked lists: also the
  next chunk, as PRD 04). Triangle's `jp_force_n1` (261.6 µs) and
  first-touch force costs shrink; measure, don't over-tune.
- **Phase-1 hash loop**: with sources resolved per batch (PRD 01/04
  discipline), specialize the common all-`Batch`-sources and
  single-`Slot`-source cases to remove the per-word `match` (two loop
  variants chosen once per sibling per batch). This is the `jp_hash_*`
  rows (spread 321.9 µs, triangle 1,529.9 µs at 15 ns/hash — target ~5).

## Passing requirements

1. Differential property tests vs the reference probe green; colt's
   existing suite (tokens, staleness asserts, forced-capacity, laziness
   watermarks) adapted and green; functional gates green.
2. Measured (traced samples vs baseline):
   - spread `jp_probe_n0` ≤ 1,400 µs (baseline 2,090.5; ≤ ~14 ns/probe).
   - stats `jp_probe_n0` ≤ 10 µs (baseline 15.4).
   - triangle `jp_probe_n1` ≤ 4,800 µs (baseline 6,005.1 — layout-only
     gain; the batching gain is PRD 09's gate).
   - triangle `jp_hash_n1` ≤ 700 µs (baseline 1,529.9).
   - No family regresses >5% (watch fk_walk/balance p95 — small-batch
     paths must not pay for the prefetch loop; the ≥16 gate is the guard).
3. `## Result` records probe-step and line-touch counts before/after from
   a test-instrumented run.

## Out of scope

Sink maps (06), batch sizes at deep nodes (09/10), map sizing policy
(unchanged), the wordmap hash function (shared and unchanged).
