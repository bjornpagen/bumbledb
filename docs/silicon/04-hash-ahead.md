# PRD 04 — Hash-ahead: unshadow the hash chain

## Purpose

bumblebench exp 02 decomposed wide-key open-addressing inserts on this
core: ~1/3 stores, ~1/3 probe walk, and ~1/3 hash *latency exposed by
probe-exit branch mispredicts* — the flush kills the speculatively started
next operation's hash chain, serializing 6.0-cycle mulxor chains across
operations (45% exposure on fills, ~85% on deep miss probes). The fix is
three lines of software pipelining: compute h(i+1) before probe(i)'s
branches. Measured recovery: 60–65% of the exposure (k=8 miss probe
21.9 → 13.6 ns), and it beats branchless window-8 probing at hit-heavy
sites. This transplants Chen'04/Kocberber'15 group-prefetch logic below
the memory wall — same idea, hiding ALU latency behind branch flushes
instead of DRAM behind misses.

## Technical direction

`crates/bumbledb/src/exec/wordmap.rs`, `crates/bumbledb/src/exec/colt.rs`,
`crates/bumbledb/src/exec/run.rs` (`probe_pass`),
`crates/bumbledb/src/exec/sink.rs` (dedup/group folds).

- **The transform, everywhere a loop probes a sequence of keys:**
  restructure from
  `for k in keys { let h = hash(k); probe(h, k); }`
  to a one-deep pipeline:
  `let mut h = hash(keys[0]); for i in 0..n { let h_next = if i+1 < n { hash(keys[i+1]) } else { 0 }; probe(h, keys[i]); h = h_next; }`
  The `hash(keys[i+1])` computation must be emitted BEFORE `probe(i)`'s
  first conditional branch — verify in disassembly that the mul/eor chain
  of the next hash precedes the current probe's `b.` instructions. Do not
  reorder loads that depend on probe results; only the hash (pure ALU on
  already-loaded keys) moves.
- **Sites, in priority order:**
  1. wordmap batch insert/lookup loops used by the seen-set and
     AggregateSink group probes (`probe_group`,
     `fold_batch_dedup_*` paths in sink.rs — the loops that feed keys in
     batches).
  2. colt `probe_hashed` consumers in `probe_pass` (hash the next pending
     entry's key while walking the current bucket).
  3. Build-side fills: image → colt map construction, wordmap fills at
     prepare/build time.
- **Tag computation rides along.** Where the ctrl-byte tag derives from
  the hash, compute it in the same pipelined step (`tag(h_next)`),
  keeping the probe loop free of any hash-dependent ALU.
- **Keep single-key paths simple.** The guard fast lane and other one-key
  probes gain nothing — do not contort them. The transform applies only
  where a batch of keys is in hand (`n ≥ 2` statically or dynamically).
- **Microbench pin.** Add an `#[ignore]`d in-tree bench test that fills a
  ≥ 4 M-entry wordmap and measures ns/insert with and without hash-ahead
  (compile both paths behind `#[cfg(test)]` selection), asserting ≥ 25%
  improvement on the miss-heavy fill (bumblebench measured 38% on the
  analogous k=8 fill; 25% is the conservative gate).

## Passing requirements

1. Disassembly gate: in the wordmap batch-insert loop, the next-key hash
   chain (`mul`/`eor` sequence) appears before the current probe's first
   conditional branch (`check-asm.sh` extended with an ordering assertion
   on the symbol).
2. Measured (vs post-03, min-of-5): skew p50 ≤ 20 µs; stats p50 ≤
   1,700 µs (baseline 1,862 — dedup wordmap dominant); spread p50 −5% or
   documented; triangle `jp_probe_n1` −10% further or documented.
3. Microbench pin green (≥ 25% on miss-heavy fill).
4. No family regresses >5% (confirm-run); verify green; emits digests
   byte-identical; zero-alloc holds.

## Out of scope

Changing the hash function itself (05 pins its quality); branchless
window probing (landed in 03); AMAC-style multi-way state machines
(rejected: the win is captured by one-deep pipelining at far lower
complexity — record this as a considered-and-rejected alternative).
