# PRD 04 — Suffix iteration: gather at load speed

## Purpose

`iter_batch`'s suffix arm pays, per position: a `ColumnView` enum match, a
slice bounds check, and a `Cursor::Row` tag write — baseline ~6.5 ns/pos
(stats `jp_iter_n1` 653.7 µs / ~100k positions; balance 271.8 µs). The data
work is one u64 load per key column. Get within striking distance of that
load.

## Technical direction

All in `exec/colt.rs` (added to the 00 unsafe allowlist), suffix paths
(`iter_positions`) first, `iter_map` second.

- **Hoist column resolution out of the position loop.** Per batch, per
  level column: resolve `self.view.image().column(col)` ONCE into a local
  `&[u64]` / `&[u8]` (two loop variants selected outside, or a two-slice
  local enum matched once — the per-position `word_at` call disappears).
  The mixed word/byte-column case is rare; a per-column inner loop
  (column-major fill of `keys_out`) handles it uniformly: for each key
  column, loop positions writing `keys_out[i * arity + w]` — arity is 1 for
  most suffix levels, where column-major == row-major.
- **Unchecked interior.** Positions are u32 indices into columns of the
  image the view was built over — in-bounds by construction (the image
  build sized every column to the row count; views carry only positions it
  produced). `debug_assert!` the bound once per batch
  (`positions.iter().all(...)` on the debug build), then `get_unchecked`
  in the loop. Same for `keys_out`/`children_out` writes (the caller
  asserted capacity at entry — keep that assert, it makes the unchecked
  writes provably in-bounds).
- **Root-view survivor indirection**: the `View::Survivors` arm reads
  `positions[idx]` then gathers — two dependent loads. Resolve the
  positions slice once per batch (it already is one slice — hoist the
  enum match), and unroll the gather by 4–8; the loads across iterations
  are independent.
- **Chunk-chain iteration** (`Positions::Chunks`): copy each chunk's
  positions with a tight loop over `c.positions[..c.len]`; issue
  `prfm pldl1keep` on `self.chunks[c.next]` one chunk ahead (the chain
  walk is the only dependent-load sequence here — one prefetch per 64
  positions is nearly free).
- **`children_out` at the suffix**: every child is `Cursor::Row(position)`
  — the enum tag write per element is information-free. Do NOT change the
  `Cursor` type (that is PRD 09/10 territory); just make the write cheap:
  fill via the same unrolled loop (the compiler will vectorize a
  tag+payload pair store; verify with `--emit asm` and record what it did
  in `## Result`).
- **`iter_map` dense-list walk**: hoist `m` field reads and the dense
  slice; the per-entry `Slot` match stays (it is real information), but
  the `keys` copy becomes an unchecked `copy_from_slice` on a
  once-resolved slab slice, and prefetch `dense[i + 8]`'s slot/key lines
  ahead (map iteration is index-scattered by insertion order).
- **Property/equality tests**: colt tests already pin iteration order and
  batch equality; add a randomized differential test iterating the same
  trie via the old shape (kept as `#[cfg(test)]` reference per the 00 law)
  vs the new, across word/byte columns, chunked and root nodes, batch
  sizes {1, 3, 64, 128}, asserting identical `(keys, children, token)`
  sequences.

## Passing requirements

1. Property/equality tests green; functional gates green.
2. Measured (traced samples vs baseline):
   - stats `jp_iter_n1` ≤ 330 µs (baseline 653.7) — ≤ ~3.3 ns/position.
   - balance `jp_iter_n1` ≤ 150 µs (baseline 271.8).
   - spread `jp_iter_n0` ≤ 250 µs (baseline 336.9).
   - range p50 ≤ 45 µs (from PRD 01's ≤ 50).
   - No family regresses >5%.
3. `## Result` records the asm-level outcome for the suffix gather loop
   (vectorized or not, and why that is acceptable).

## Out of scope

Batch-of-1 call overhead at pinned-row leaves (spread `jp_iter_n1`'s
16 ns/call — that is 05/09/10 structure, not gather cost), map probe
layout (07).
