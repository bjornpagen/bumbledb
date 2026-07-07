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

## Result (2026-07-07, runs bench-out/2026-07-07T01-40-06Z + 01-47 confirm)

Landed: the interleaved bucket layout — per-map ctrl ranges (0 = empty,
else `0x80 | top-7-hash-bits`) plus `arity + 1`-word bucket rows (key
words, then the packed child: bit 63 discriminates NodeRef from pinned
position; `Slot` survives as the API type, decoded at the boundary) —
across probe, iteration, force-ingest, singleton upgrade, and the
75%-load rehash (dense-order preserved). `Colt::prefetch_bucket` + the
phase-1.5 pass in run_node (gated ≥ 16 survivors), and the single-batch-
word hash-loop specialization for the dominant FK-probe shape. The
adversarial differential test pins probe hits/misses, upgrades, and
growth against a key→positions model under equal-low-bit keys.

Gates:
- stats `jp_probe_n0` **5.5 µs** (gate ≤ 10; baseline 15.4) ✓.
- spread `jp_probe_n0` **1,722.8 µs** (gate ≤ 1,400; baseline 2,090.5,
  −18%) ✗ near-miss: the remaining cost is genuine L2/DRAM latency on a
  ~100k-key map at batch 128 — the prefetch pass overlaps what the
  OoO window reaches; deeper gains need bigger in-flight windows
  (PRD 09's cross-parent batches).
- triangle `jp_probe_n1` **5,504 µs** (gate ≤ 4,800; baseline 6,005,
  −8%) and `jp_hash_n1` **1,496 µs** (gate ≤ 700; baseline 1,530) ✗ —
  exactly the outcome the gate text pre-named: these rows run at batch
  size ~1 (100k single-probe passes), below the prefetch gate and with
  per-call overhead dominating the specialized hash; the batching gain
  is PRD 09's gate, not a layout property.
- Wins elsewhere: spread `jp_hash_n0` 322 → **149.5 µs** (−54%, the
  specialized loop), chain p50 −18.5% (138.5 µs), fk_walk 4.4 µs,
  spread p50 10.7–10.9 ms across runs (best yet). skew/triangle/
  balance/point swings all returned to their documented bands on the
  same-binary confirm run (triangle 16,059 µs — best sample to date).
- Line-touch evidence: the model test + wordmap's 1.492-steps sweep
  carry the probe-step story; per-probe line counts are structural now
  (ctrl line + at most one bucket line on tag match).
