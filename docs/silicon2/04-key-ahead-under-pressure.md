# PRD 04 — Key-ahead prefetch in the window probe: fix the in-cache-only lever

## Purpose

Three round-two experiments independently converged on the window
probe's hidden cost: SWAR candidate resolution (`ldr` ctrl word →
`rbit/clz` → key load) makes the key-load ADDRESS data-dependent on the
ctrl load. In cache, that dependency is invisible; when the map's lines
are cold — DRAM-tier seen-sets (exp 13: 1.4–1.7× regression vs branchy,
+48% at 100% hits) or nominally-resident maps under co-residency
pressure (exp 18: 3× pressure-slope inversion vs per-slot probing;
exp 19: the same mechanism inside the executor) — the two miss legs
serialize where a branchy probe's predicted control flow fetches them
in parallel. Exp 18 measured the one-instruction fix: **key-ahead
`prfm` on the home bucket's key line recovers 21–29% of hit cost under
pressure and is free at rest** (its cost at P=0 measured ≈ 0). The
alternative (probe-shape tiering back to branchy past a footprint
budget) is strictly more complex and loses the window probe's L2 wins;
exp 18's fix keeps one probe shape.

## Technical direction

`crates/bumbledb/src/exec/wordmap.rs` (the probe/insert cores from
PRD 03).

- **The prefetch**: at probe entry, after computing the home index,
  issue `kernel::prefetch_read` on the home slot's KEY line
  (`&self.keys[idx * K]`) — before the ctrl window load. The ctrl line
  itself is loaded immediately (no point prefetching it); the key line
  is the one whose demand load is address-serialized behind the ctrl
  resolution. One `prfm` per probe, unconditional within the gated
  path.
- **The gate**: footprint-tiered, like the campaign's colt gate but
  with round-two numbers: fire when the map's slab footprint
  (`ctrl.len() + keys.len()*8 + values-bytes`) exceeds a named
  `KEY_AHEAD_BUDGET_BYTES` const. Exp 18's surface says pressure
  effects appear from ~2–4 MB co-residency on maps ≥ 1 MB; and exp 18
  measured the prefetch FREE at rest — so the budget can be generous:
  **512 KiB** (below it the map is L1/L2-trivial and the extra µop is
  pure waste on the hottest tiny maps like group keys). Add a
  `#[inline(always)] fn footprint_bytes(&self)` and cache the boolean
  at... no caching — the comparison is two loads and a compare,
  amortized; keep it per-call and let the branch predict.
- **Do NOT add hash-ahead back** — exp 13's br+ha finding is about the
  branchy probe shape, which bumbledb no longer ships; the key-ahead
  `prfm` is the window-probe-native fix (exp 18 tested exactly this
  pairing).
- **Microbench pin** (`#[ignore]`d): a 16 MB map, 50% hits, with an
  interleaved 8 MB streaming sweep between op batches (exp 18's
  pressure protocol, reconstructed small): key-ahead ≥ 15% better than
  without (exp 18: 21–29%); AND at a 256 KB map with no pressure:
  within ±2% (the free-at-rest claim).
- **Doc**: the wordmap module header's probe section gains the
  in-cache-only law and the fix, citing exps 13/18/19.

## Passing requirements

1. Microbench pin green (both halves: ≥ 15% under pressure, ±2% at
   rest).
2. Measured (vs post-03, min-of-3): spread p50 −2% or better (its
   dedup seen-set is the biggest DRAM-tier-adjacent map in the ledger);
   stats holds or improves; skew p95 holds or improves; triangle holds
   (its seen-set is small — likely under the budget; record which side
   the bench maps land on, with footprints, in `## Result`); no family
   regresses > 5%.
3. Disassembly: the gated probe path shows exactly one `prfm` ahead of
   the ctrl window load (check-asm: symbol contains `prfm` — and the
   probe-class no-`bl` gate still green).
4. Differential corpus + false-tag contract green (a prefetch changes
   no semantics — the tests are the tripwire against accidental
   reordering of the actual loads); verify green; zero-alloc holds.

## Out of scope

Probe-shape tiering (rejected: one shape + `prfm` is simpler and
measured sufficient — record as considered-and-rejected); colt probes
(05/06 replace that layout entirely); the sink-side deletion (done in
02).
