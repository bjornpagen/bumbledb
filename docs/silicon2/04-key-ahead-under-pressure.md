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

## Result

**Not shipped — the mechanism is refuted for the shipped workload
shape, by exp 18's own data and an in-tree pin.**

The PRD's 21–29% recovery quote is exp 18's **pure-hit-stream** rows
(winP-hit f=25/f=50 at P=24: −24%/−22%). Exp 18's OWN mixed row —
`winP-mix f=50: 12.78 → 12.82 ns (0%)` — is the shape that matches the
sink seen-sets, which are miss-heavy by construction (every distinct
key's first sight is a miss; the wordmap module doc has carried this
law since silicon-03). Its miss rows run free-to-+5%.

The in-tree pin (exp 18's pressure protocol reconstructed: 16 MiB
arity-4 map, 50% hits, 8 MB streaming sweep between 256-op batches,
min-of-5, A/B via a `cfg(test)` knob on the implemented `key_ahead`
gate): **with prefetch 324,579 ns vs without 305,041 ns — key-ahead is
6.4% WORSE under pressure.** Mechanism: the sink's row loop issues
independent probe chains back to back, so the out-of-order window
already saturates memory-level parallelism across iterations — the
`prfm` adds issue-slot pressure to a stream with nothing left to
overlap. Exp 18's single-stream protocol had serialization to recover;
the engine's batched shape does not. (Exp 19's phase-1.5 refutation in
PRD 01's Result is the same law from the other side: batching, not
prefetching, is this engine's latency-overlap mechanism — and it is
already everywhere.)

The implementation (footprint-gated `key_ahead` on both probe entries,
512 KiB budget, module-doc law, the pin) was built, measured, and
REVERTED — the shipped wordmap is byte-identical to post-03 except a
clippy hygiene fix in `hash_core` (range loop → slice iter, same
codegen). No ledger battery: nothing shipped, the ledger is post-03's
by construction.

**Requirement rulings**: 1 (pin ≥15%/±2%): refuted, direction reversed
— recorded above. 2 (spread −2% etc.): moot, nothing shipped. 3 (one
`prfm` in the probe path): moot. 4 (differential/false-tag green): the
corpus ran green with the prefetch in place before the revert — the
refutation is performance, not correctness. Probe-shape tiering stays
rejected as drafted; this Result adds key-ahead itself to the rejected
list, closing the exp-13/18 thread: the window probe ships bare.
