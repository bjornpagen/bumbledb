# PRD 08 — Delete the dead batching levers: segregation and the 2× cascade

## Purpose

Exp 14 pinned the per-pass fixed overhead the batching levers were
built to amortize at **11–30 ns — twenty times below the 200–500 ns
the campaign assumed** — which retroactively prices cover-stable
segregation (measured: batch means 37 → 39) and the 2×-batch cascade
(probe passes −6%, time ±0%) as ~1% effects, and rejects cross-call
fill carry before it was ever built (its complete win, computed
exactly, is 0.2–1.2% of triangle p50). The design philosophy is
representation and simplicity to the extreme: machinery that measured
at noise is a maintenance liability with no offsetting asset. This PRD
deletes it.

## Technical direction

`crates/bumbledb/src/exec/run.rs`.

- **Delete cover-stable segregation**: `pump` returns to the single
  in-order walk — per-entry dynamic cover choice AT processing time
  (the pre-segregation shape: choose cover per entry, flush on cover
  change), removing pass 1, the `entry_covers` scratch field and its
  constructor init, the `seen_covers` bitmask, and the two-level group
  loop. Reinstate the straightforward loop from the pre-a02b10c shape
  — but KEEP the structure of per-entry cancellation checks and the
  origin machinery untouched (they predate segregation).
- **Revert the cascade threshold** to one batch
  (`pending_len >= self.batch`) and restore the bounded-two-batches
  comment — exp 14 measured the 2× threshold at 0.0–0.6% and the
  smaller pending buffers are the simpler contract.
- **Record fill carry as rejected-by-measurement** in the run.rs pump
  comment: "cross-call fill carry lifts batch means to ~128 and is
  worth 0.2–1.2% of triangle p50 at the measured 11–30 ns pass
  overhead (bumblebench exp 14) — the entire batch-mean lever class is
  closed."
- **The D2 gauntlet is the tripwire**, exactly as when segregation
  landed: the 200-case randomized subset-projection differential, the
  two-parent interleave fixtures, batch-size equality, and the
  `pipelined_middle_nodes_probe_in_cross_parent_batches` mean
  assertion (which must be re-checked: with segregation gone the mean
  gate is the original ≥ 32 — verify the test's threshold matches the
  restored behavior).
- **Doc**: `docs/silicon/14-endgame.md` Result gains a superseded-by
  note; the silicon2 README's law table already carries the pricing.

## Passing requirements

1. grep gates: no `entry_covers`, no `seen_covers`, no
   `batch * 2` cascade in run.rs; the pump body is the single-pass
   shape.
2. Ledger (vs post-07, min-of-3): every family within ±2% — this PRD
   must be measurement-neutral (exp 14's whole point); any deviation
   > 2% is investigated as a bug in the restoration, not accepted as
   a win or loss.
3. D2 differential (200 cases) green; batch-mean test green at its
   restored threshold; verify green; emits digests unchanged;
   zero-alloc holds.
4. Net line count of run.rs DECREASES; the diff is deletion-dominated
   (recorded in `## Result`).

## Out of scope

Any new batching mechanism; touching the probe passes themselves
(01/06/07 own those); pump's cancellation/origin machinery.
