# PRD 03 — Map geometry: misses are the expensive case

## Purpose

bumblebench inverted our intuition about open addressing on this core:
misses cost MORE than hits (9.2 vs 6.1 ns at load factor 0.38 — longer
walks and a mispredicted exit branch), dropping load factor to 0.05 takes
misses to 2.8 ns, and a branchless window probe is 4.6× faster at
hit-rate 0. bumbledb's two hot map sites have opposite profiles: COLT
probes during joins are hit-heavy (build side is dense); the seen-set /
dedup wordmap is miss-heavy by construction (first occurrence of every
distinct key is a miss). Geometry and probe shape should differ per site,
and today they don't.

## Technical direction

`crates/bumbledb/src/exec/wordmap.rs`, `crates/bumbledb/src/exec/colt.rs`.

- **Expose load factor as a named constant per structure.** Find the
  current grow threshold in `wordmap.rs` (grow path emits `WORDMAP_GROW`)
  and in the colt build path. Make both explicit
  (`const MAX_LOAD_NUM/DEN`), then set:
  - wordmap (miss-heavy insert-once sites: seen-set, dedup, group probe):
    max load 25%. Capacity hints already flow (`with_capacity_hint`, plan
    estimates) — hint math must account for the new factor so steady-state
    executions still never grow (the `a_covering_hint_never_grows` test
    pins this; update its arithmetic).
  - colt (hit-heavy): 50% max load, unchanged unless measurement says
    otherwise — the win here comes from PRD 02+04, not geometry.
  Justify final constants by measurement: run the family ledger at
  {50%, 33%, 25%} for wordmap and record the table in `## Result`; keep
  the winner (expected: 25%).
- **Branchless window probe for the miss-heavy site.** Implement a
  branchless ctrl-scan probe for wordmap insertion: load a 16-byte ctrl
  window (`ldr q` / two u64 loads — portable reference first, per the
  unsafe law), compute match-mask and empty-mask branchlessly, resolve
  slot index with ctz — one well-predicted loop-exit branch per window
  instead of one branch per slot. Select this shape statically at the
  seen-set/dedup call sites (they know they are insert-or-first-touch);
  the hit-heavy lookup path keeps the early-exit shape (branch exits are
  profitable at high hit rates).
- **Memory discipline.** Halving load factor doubles table bytes. Bound
  it: record peak wordmap bytes on the worst family (triangle seen-set)
  via the `WORDMAP_GROW`/capacity events; the gate caps growth at 2× the
  baseline peak. If a family's table would leave L2 because of the new
  factor (bytes > 12 MB), that site keeps the old factor — L2 residency
  is worth more than walk length (law: batching/prefetch only pay past
  L2; we are deliberately keeping maps inside L2).
- **Pinned metrics.** Update `probe_steps` (currently pinned ~1.49 mean)
  to the new expected mean (≤ 1.2 at 25%); keep the differential test
  against the reference model green.

## Passing requirements

1. Measured (vs PRD-00 baseline, min-of-5): skew p50 ≤ 24 µs (baseline
   ~28); range p50 ≤ 26 µs (baseline ~29); chain p50 ≤ 120 µs (baseline
   ~130–132); stats p50 improves ≥ 5% (dedup wordmap is on its hot path).
2. Triangle p50 improves or holds vs post-02 (the seen-set is on its
   path); no family regresses >5% (confirm-run protocol).
3. Peak wordmap bytes on triangle ≤ 2× baseline peak, recorded in
   `## Result`; no map that was L2-resident at baseline leaves L2.
4. The load-factor sweep table ({50,33,25}% × affected families) is
   recorded in `## Result`; `probe_steps` and `a_covering_hint_never_grows`
   updated and green; branchless window probe has a portable reference +
   bit-identity differential test; verify green.

## Out of scope

Hash function and hash scheduling (04, 05); colt bucket layout changes;
any auxiliary index structure (still banned).
