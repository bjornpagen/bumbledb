# PRD 02 — Probe instruction diet: the triangle wall, part 1

## Purpose

The triangle probe wall (`jp_probe_n1` ~5.5 ms, flat under 37× batching
and prefetch) is now solved as a mechanism: the COLT map is L2-resident,
the ROB overlaps probes at batch 1, and the 55–60 ns/probe is retire-bound
executor instruction weight. bumblebench's executor-shaped emulation
reproduced the width-insensitivity signature at 17–21 ns with exactly the
suspects we ship: call ceremony into the map, a by-value `Map` copy,
per-probe counter increments, generic key compares, and walk exit
branches. The fix class is instruction removal — 2–4× is on the table.
This PRD removes the structural instruction weight; PRDs 03–04 attack the
walk and the hash.

## Technical direction

`crates/bumbledb/src/exec/run.rs` (`probe_pass`, `pump`),
`crates/bumbledb/src/exec/colt.rs` (`probe_hashed`, `ctrl_tag`,
`unpack_child`, gathers) + a new `scripts/check-asm.sh`.

- **Probe context, built once per (node, batch).** Before the probe loop,
  hoist the map's hot fields into a flat local struct:
  `ProbeCtx { ctrl: *const u8, buckets: *const u64, mask: u64, stride: usize, arity: u8 }`
  (whatever the actual `Map` layout exposes — read `colt.rs::Map` and take
  exactly the fields `probe_hashed` touches). The inner loop must touch
  ONLY this struct — no re-deref of the `Map` handle, no by-value `Map`
  moves across any call boundary (the emulation costed the 48 B stack copy
  as a first-class suspect). If `probe_hashed` currently takes `&Map` or
  `Map`, restructure so the loop body is a free function (or fully inlined
  method) over `ProbeCtx`.
- **Inline the entire probe path.** `#[inline(always)]` on `probe_hashed`
  (or its `ProbeCtx` replacement), `ctrl_tag`, `unpack_child`, and any
  helper the inner loop calls. The GATE is the disassembly, not the
  attribute: the probe inner loop in the release binary must contain no
  `bl`/`blr` and no `bcmp`/`memcmp`.
- **Monomorphic key compares.** If bucket key comparison is a slice
  compare or a loop over arity, replace with fixed-arity unrolled u64
  compares: dispatch ONCE per batch on arity (`match arity { 1 => loop1,
  2 => loop2, 3 => loop3, 4 => loop4, _ => generic }`) so each inner loop
  is straight-line compares. Per the campaign record, bench arities are
  ≤ 4; the generic arm stays for correctness.
- **Counters out of the inner loop.** Per-probe `Counters`/`jp_*`
  increments accumulate in local `u64`s inside `probe_pass` and flush once
  per batch into the real counters. Semantics of emitted counts unchanged
  (tests on EXPLAIN counters must stay byte-identical). The same for any
  per-probe `obs` event that currently fires per element.
- **Call-boundary audit downstream.** The row-push for survivors
  (`push_surviving`) and the pending-append path run per survivor: apply
  the same treatment — inline, hoist the destination pointers, batch the
  bookkeeping. Renaming law: a `bl/ret` in a per-item path costs ~7.5
  cycles on a 2-cycle chain even when the callee is trivial.
- **`scripts/check-asm.sh`.** Create it: objdump the release bench binary,
  extract named hot symbols (start with `probe_pass`), and grep-assert
  per-symbol properties (`no-bl`, `no-bcmp`). Wire a make/check target.
  This script is reused by PRDs 07 and 09.

## Passing requirements

1. Disassembly gate: `check-asm.sh` green — no `bl`/`blr`/`bcmp` inside
   `probe_pass`'s probe inner loop.
2. Measured (traced, vs PRD-00 baseline): triangle `jp_probe_n1` self-time
   ≤ 3,500 µs (baseline ~5,500); triangle p50 ≤ 13,500 µs (baseline
   ~15,100–15,600).
3. skew and chain (also probe-bearing plans) p50 improve or hold (≥ 0%,
   no regress); every other family within 5% (confirm-run protocol).
4. EXPLAIN emits digests byte-identical on all ten families; verify green;
   zero-alloc gate holds (ProbeCtx is stack-local, no allocation).

## Out of scope

Load factor and walk length (03), hash-ahead (04), prefetch gating (10),
cover-stable batch segregation (14).
