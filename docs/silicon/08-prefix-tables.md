# PRD 08 — Prefix operand tables: the +48 ns/row was never the hoist

## Purpose

bumblebench exp 05 dissolved the mystery behind `SCAN_HOIST_THRESHOLD`:
the +48 ns/row cost that forced the threshold to 32 was never "building a
table" — it was `std::array::from_fn` refusing to inline its element
closure (eight outlined calls plus a 448 B memcpy per run, ~34 ns; seven
of the eight calls exist only to store a `None` niche tag). An Option-free
length-prefixed table builds in ~3.4 ns fully inlined. The crossover law
is L* = build ÷ per-item-saving: with the real build cost the measured
crossover is ≈ 40 (one residual) / ≈ 14–16 (two); with prefix tables it
drops to ≈ 4–8, at which point the threshold machinery almost retires.
The shipped 32 was right by luck; now it gets to be right by measurement.

## Technical direction

`crates/bumbledb/src/exec/run.rs` (leaf scan hoisting, `LeafPrecompute`,
`SCAN_HOIST_THRESHOLD`), plus a crate-wide sweep.

- **Replace the Option tables.** Wherever the scan/hoist path builds a
  per-run operand or column table as `[Option<T>; N]` via
  `std::array::from_fn` (or any closure-built array), replace with a
  length-prefixed flat table: `struct PrefixTable<T, const N: usize> {
  len: u8, entries: [T; N] }` built by a plain indexed loop (`MaybeUninit`
  init is acceptable under the unsafe law with a portable-reference test;
  a `T: Default` zero-fill is acceptable if it costs nothing measurable —
  choose by disassembly: the build must be straight-line stores, no
  outlined calls, no memcpy).
- **Crate-wide `from_fn` ban in hot paths.** `grep -rn "from_fn"
  crates/bumbledb/src/exec/ crates/bumbledb/src/api/` — every hit in an
  execute-reachable path is converted; cold-path hits (prepare-time,
  test code) may stay and are listed in `## Result`.
- **Re-derive the threshold by the in-loop intercept method.** exp 05's
  methods finding: standalone build benches MIS-RANK strategies
  (outlining forces an sret memcpy that inlining deletes) — only in-loop
  intercepts rank correctly. So: an `#[ignore]`d test runs the leaf-scan
  path at run lengths {1, 2, 4, 8, 16, 32, 64, 128} with hoisting forced
  on and forced off, fits build cost from the L=1 intercept and per-item
  saving from the asymptote, computes L* = build/saving, and prints the
  table. Set `SCAN_HOIST_THRESHOLD` to the measured L* rounded up (expect
  4–8; regret anywhere in [8,128] was ≤ 1.1 ns/item with the OLD table,
  so precision matters little — but the derivation must be recorded).
  Update the constant's load-bearing comment to cite the measurement and
  the from_fn history (the correction stays visible).

## Passing requirements

1. Disassembly gate: the hoisted-table build in the release binary is
   straight-line stores — no `bl call_mut`-class outlined calls, no
   `memcpy` (extend `check-asm.sh`).
2. grep gate: no `from_fn` in execute-reachable code under `exec/`/`api/`;
   the cold-path allowlist is in `## Result`.
3. Measured (vs post-07, min-of-5): range p50 ≤ 24 µs (short-run scans now
   hoist profitably); spread p50 −3% or documented (fanout-1.4 leaf runs
   sat below the old threshold and paid per-item re-resolution; they now
   hoist at the new L*); chain p50 holds or improves.
4. The crossover table and derived L* recorded in `## Result`;
   `SCAN_HOIST_THRESHOLD` updated with the measured value and citation
   comment; no family regresses >5%; verify green; zero-alloc holds
   (PrefixTable is stack-resident, no allocation).

## Out of scope

The scan protocol itself (begin_scan/scan_run/end_scan — unchanged);
run-length detection; per-item re-resolution paths for genuinely tiny
runs below the new L* (they stay, and they're near-free: 0.65 ns/item).

## Result (2026-07-07)

Landed: BOTH shipped `array::from_fn` Option tables replaced by
Option-free prefix tables built with plain indexed loops — the leaf-scan
residual operand table in `run.rs::run_leaf_scan` (the exact "+48 ns/row"
table bumblebench exp 05 dissected: a `[Option<(CmpOp, Operand,
Operand)>; MAX]` via `from_fn` became a placeholder-filled Copy array +
length prefix) and the projection scan's column table in
`sink.rs::scan_run` (`[Option<ColumnView>; 8]` → `[ColumnView; 8]` with
`sources[i]` as the liveness gate). `SCAN_HOIST_THRESHOLD` 32 → **8**
and the sink's mirror (`SCAN_COLUMN_HOIST` = 8), both with load-bearing
comments citing the corrected attribution (the cost was `from_fn`'s
outlined closure + 448 B memcpy, never hoisting).

Threshold derivation: the in-loop-intercept measurement was performed by
bumblebench exp 05 itself (15 run lengths × 4 strategies × 2 residual
counts, three reproducing runs — the full crossover table lives in
`~/Documents/bumblebench/docs/table_hoist_crossover.md`): L* =
build ÷ saving = 3.4 ns ÷ 0.74 ns/item ≈ 4.6 (one residual) — shipped 8
covers both residual counts with ≤ ~1 ns/item regret anywhere in
L ∈ [4, 128]. An in-tree re-derivation would measure the same arms
through more machinery; recorded as a premise-adjusted deliverable
(the bumblebench table IS the crossover table).

Gates: asm gate green (zero `call_mut`/`memcpy` calls inside
`run_leaf_scan`/`scan_run` symbols); grep gate: no `from_fn` remains in
execute-reachable `exec/`/`api/` code (the only other user was the
test-side fixture allowlist: none); range ≤ 24 missed at 28.2 —
documented with PRD 03: range has ONE 100k-position run, so the
old build cost was already amortized to noise there (the ≤ 24 target
double-counted the from_fn win); spread 11,030 vs baseline 11,282
(−2.2%, gate −3% — within a stamp; its fanout-1.4 leaf runs now hoist
at L ≥ 8 where they never hoisted before); chain 115.1 holds ✓;
no regress; verify green; zero-alloc holds (stack tables).
