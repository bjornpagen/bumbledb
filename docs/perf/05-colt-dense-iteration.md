# PRD 05 — COLT dense iteration and honest map sizing

Authority: `30-execution.md` (COLT forcing, the zero-alloc reuse discipline),
suite README finding 3 (the iteration half). Independent of PRDs 00–04 in
logic; land after 02 to avoid textual conflicts in `colt.rs`.

## Purpose

Two defects in one seam, measured together as balance's 220 µs-for-800-rows:

1. `force` (`colt.rs`, the `capacity = count.max(1) * 2` line) sizes a level's
   open-addressed map by **ingested positions**, not distinct keys — Posting's
   account level holds 500 keys in a 200,000-slot table (0.25 % occupancy).
2. `iter_map` walks **every slot** of that table (`while … slot_idx <
   m.capacity`), so iterating a forced level costs O(capacity) — 200k slot
   visits to yield 500 keys — on every batch drain, every execution.

Fix both: maps grow from small by rehash-doubling during force (occupancy-
driven capacity), and iteration walks a **dense occupied list** (O(keys)
always, whatever the capacity).

## Technical direction

- **Dense list.** `Map` gains `dense_start: usize, /* len == m.len */` into a
  new shared slab `dense: Vec<u32>` on the COLT (recycled through `reset` like
  `slots`/`keys`/`chunks`). During `force`, every *newly occupied* slot index
  is appended; a slot promoted `Single → Node` (chain growth) does not append
  again. `iter_map` walks `dense[dense_start .. dense_start + len]`, reading
  each slot by index — the `BatchToken` for maps becomes a dense-list index
  (the token is documented as opaque; audit the one producer/consumer pair,
  `iter_map`'s `(yielded, BatchToken(slot_idx))` return and resume, and the
  suffix/chunk token variants which are untouched).
- **Growth sizing.** Replace the `count * 2` pre-size with:
  - initial capacity `next_pow2(max(16, count / 8))` — a guess that is right
    for near-unique levels and cheap for skewed ones;
  - rehash-double when `len * 4 >= capacity * 3` (75 % load) during ingestion.
  Rehashing allocates a fresh slot/key/dense range at the slab tail and
  abandons the old range for the rest of the generation (reclaimed by
  `reset`'s recycle) — document this transient ≤ 2× slab overhead where the
  slabs are declared; it is the price of not knowing distinct counts up front,
  and `force` is already the sanctioned allocation window.
- **Probe path unchanged**: `probe`/`probe_hashed`/`get_prehashed` read
  `m.capacity` as before; only sizing and iteration change.
- `key_count` unchanged (`Exact(m.len)` was already honest).
- Audit `Slot::Empty` skipping logic — after this PRD, `iter_map` must never
  observe an `Empty` slot (dense entries are occupied by construction); make
  that a `debug_assert!`, not a silent skip, so regressions scream.

## Non-goals

Changing the hash function, probe sequence, or slot representation. Sizing
from planner statistics (PRD 07 makes estimates honest, but force must stay
correct with zero statistics). Shrinking after force (growth-only within a
generation; `reset` is the reclamation point).

## Passing criteria

- Pure-COLT unit tests:
  - Force 100,000 positions carrying 500 distinct keys: post-force
    `capacity ≤ 8 × 500` (the growth bound), and draining via `iter_batch`
    with `max = 64` takes exactly `ceil(500 / 64)` calls, each yielding 64
    (last: remainder) — the O(keys) iteration pin, no wall clock involved.
  - Force with near-unique keys (10,000 positions, 10,000 keys): iteration
    yields all 10,000 exactly once, in dense order; capacity ≤ 4 × keys.
  - Resume-token correctness across the dense change: drain in `max = 1`
    steps, interleaved with probes, equals a single-shot drain (order and
    content).
  - Recycle: `reset` after growth reuses slabs; a second force reaching the
    same shape performs zero slab reallocation (assert capacity pointers/lens
    stable via test accessors, mirroring the existing retention tests).
- The existing full suite (differential, verify-S, families) green — forced
  iteration order may change (dense order ≠ slot order); results are sets, so
  nothing may depend on it. If any test pinned iteration order, fix the test
  and say so in the commit message.
- Alloc gate still green in release (growth happens only during force windows).
- `scripts/check.sh` green.
