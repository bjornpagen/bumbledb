# PRD 07 — Alias-hoisted executor loops: stop reloading Vec headers

## Purpose

Exp 19's bounds-check follow-up found a mechanism the campaign's
kernel-level audits could not see: in executor-shaped loops that
interleave reads from one `&mut self` scratch vector with stores to
another, LLVM must reload the `Vec` headers (ptr/len) every iteration —
the stores might alias them — and the measured cost on the emulated
probe shape was **32%** (10.88 → 7.35 ns with the reloads removed).
This is an aliasing problem, not a bounds problem: the fix is
representational — split the scratch into disjoint local slices before
the loop so the borrows prove non-aliasing — and it needs no `unsafe`.

## Technical direction

`crates/bumbledb/src/exec/run.rs` (the probe/hash/residual loops in
`probe_pass` and `run_node`), `crates/bumbledb/src/exec/sink.rs` (the
fold/dedup row loops).

- **The transform, mechanically:** before each hot per-element loop,
  destructure the needed `NodeScratch`/sink fields into local slice
  bindings taken ONCE:
  `let (probe_keys, hashes, mask) = (&mut scratch.probe_keys[..n*ar], &scratch.hashes[..n], &mut scratch.mask[..n]);`
  … then index the LOCALS inside the loop. Disjoint `&mut` field
  borrows through one struct are legal and give LLVM the non-alias
  proof; slices of KNOWN length additionally let the bounds checks
  hoist. Where a loop currently mixes `scratch.x[..]` reads with
  `scratch.y[..]` writes (the probe loop: reads `probe_keys`/`hashes`/
  `parents`/`pending_cursors`, writes `sibling_children`/`mask`), every
  one of those becomes a pre-loop local.
- **Sites, in priority order** (each verified by objdump before/after):
  1. `probe_pass`'s probe loop (the exp-19 shape itself);
  2. `probe_pass`'s hash-gather loop;
  3. `run_node`'s sibling probe + hash loops;
  4. `probe_pass`'s survivor-routing (Descend) loop — reads
     `entry_keys`/`parents`, writes child `pending_*` (cross-struct:
     the child scratch is a DIFFERENT NodeScratch — already
     non-aliasing by construction, but the header reloads may still be
     emitted; check and hoist);
  5. sink `fold_batch_rows` / dedup loop (reads batch, writes
     `binding_scratch` — one struct, check).
- **The gate is the disassembly**, not the source shape: for each site,
  the loop body must not reload the slice base pointers per iteration
  (mechanically: the loop's memory operands address off registers set
  BEFORE the backedge target; `check-asm.sh` gains a heuristic check —
  the probe-loop symbol's instruction count per element drops, and a
  hand-verified before/after excerpt goes in `## Result`; the
  mechanical gate is the family numbers + the existing no-`bl` gates
  staying green).
- **Colt internals** (`probe_hashed` on the bucket layout) already
  index `self.ctrl`/`self.buckets` immutably — no interleaved stores;
  skip unless the 06 disassembly shows header reloads (record either
  way).

## Passing requirements

1. Before/after disassembly excerpts for sites 1–3 committed in
   `## Result`, showing the header reloads gone (base registers hoisted
   out of the loop).
2. Measured (vs post-06, min-of-3): triangle p50 −3% or better (exp 19
   prices the reload class at up to 32% of the loop's non-memory
   cost; the honest engine expectation after 01+06 is single-digit
   percent — gate at −3% with documented-miss); stats −2% or
   documented; chain/spread hold or improve; no family regresses > 5%.
3. Verify green; emits digests unchanged; zero-alloc holds (locals are
   reborrows, not allocations); check-asm green.

## Out of scope

`unsafe` pointer arithmetic (the safe reborrow gets the proof — if a
site resists, record it rather than reaching for raw pointers);
restructuring NodeScratch itself (a follow-up if reborrow ergonomics
demand it, not this PRD).
