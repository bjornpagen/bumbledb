# PRD 06 — Sink map: one line per probe, no rehash in the loop

## Purpose

`WordMap` (seen-sets, group maps) probes touch two arrays per step —
`values: Vec<Option<V>>` occupancy plus the `keys` slab — and key-compares
on every occupied collision. Spread inserts 100k distinct 2-word tuples per
execution through it (inside `jp_descend_n0`/leaf time); first executions
additionally pay ~14 rehash-doublings growing to capacity (visible as
`WORDMAP_GROW` events in a cold-prepared trace). Rebuild it as a
tag-byte-controlled single-probe-line map and presize it from plan
information.

## Technical direction

All in `exec/wordmap.rs` (added to the 00 unsafe allowlist). The public API
(`new`, `len`, `clear`, `get_or_insert_with`, `insert`, `iter`) and the
insertion-order dense rule are invariants — downstream determinism depends
on them.

- **Layout.** SwissTable-lite, linear probing kept, no groups/SIMD scan
  (M2 loads are wide enough; the win is line count, not probe SIMD):
  - `ctrl: Vec<u8>` — 0 = empty, else `0x80 | (hash >> 57)` (top 7 hash
    bits). One byte per slot; 128 slots per cache line means the ctrl walk
    for a linear-probe run is one line essentially always.
  - `keys: Vec<u64>` — unchanged slab, `capacity * arity`.
  - `values: Vec<V>` — **no `Option`**: slots are uninitialized until
    ctrl marks them occupied. Use `Vec<MaybeUninit<V>>`; the occupied set
    is exactly the dense list + ctrl bytes, `clear()` drops occupied
    values via the dense list (V is `Copy`-ish in practice — but write the
    Drop-correct loop anyway; a `V: Copy` bound is acceptable if it
    simplifies, since both uses are `()` and `usize` — take
    `V: Copy + Default` and document it).
  - Probe: hash → slot; compare ctrl byte (empty → miss-insert; tag
    mismatch → next slot, **no key load**); tag match → compare key words.
    7-bit tags make false key compares ~1/128 of collisions.
- **Presizing.** `WordMap::with_capacity_hint(arity, hint)`: round
  `hint * 2` up to a power of two, allocate once. Call sites:
  - `AggregateSink`/`ProjectionSink::new` currently size lazily; thread a
    hint from the prepared query: the planner's root estimate
    (`JoinOrder::estimates` — the plan already carries per-node estimates;
    use the last node's estimate, clamped to `[16, 1 << 21]`) for
    seen-sets and projection outputs; group maps hint
    `min(estimate, 4096)` (groups are few).
  - `clear()` keeps capacity (unchanged), so warm executions were already
    rehash-free — the hint kills the *first-execution* rehash storm and,
    more importantly, mid-measurement growth on cold-prepared scenario
    queries.
- **Growth** stays (hints are estimates): same doubling, re-probe in dense
  order (the existing order-preserving rehash), rewritten for the new
  layout. `WORDMAP_GROW` event stays.
- **Unsafe discipline**: `MaybeUninit` reads gated by ctrl-byte occupancy;
  `get_unchecked` on ctrl/keys after the power-of-two mask (index provably
  < capacity). Keep a portable-safe reference implementation — the OLD
  map, moved to `#[cfg(test)] mod reference` — and a randomized
  differential property test: identical `(inserted, value, iteration
  order, len)` behavior across operation sequences including growth
  boundaries, arities {0, 1, 2, 4}, adversarial keys (equal hashes mod
  capacity: craft by masking), and `clear()` cycles.
- **Iteration order across grow** keeps the in-place dense rewrite
  guarantee (existing test `grow_rewrites_the_dense_list_in_place` must
  survive semantically; adapt to the new fields).

## Passing requirements

1. Differential property tests vs the reference green; existing wordmap
   tests adapted and green; functional gates green.
2. A cold-prepared traced spread execution shows **zero `WORDMAP_GROW`
   events** (hint sized it); warm executions show zero (as today).
3. Measured (vs post-05 recorded numbers): spread p50 improves ≥ 400 µs
   further; skew and stats do not regress (their group maps are small —
   this PRD must not slow the small-map case: assert balance/skew p95
   within noise).
4. `## Result` records probe-line evidence: average probe steps and false
   key-compare rate from a test-instrumented run (a `#[cfg(test)]` counter
   is fine), before/after.

## Out of scope

COLT's forced maps (07 — different structure, same idea), the sinks' fold
logic (02/03), cross-node batching.
