# 35 — Re-pin `OVERLAP_CROSSOVER` and `FLAT_SWEEP_CEILING` from the sweep, on a quiet machine

- **Status:** OPEN (final pass; TODO.md row — "rig-pinned provisional").
- **Severity:** performance debt; **bare-metal item** (needs a quiet
  machine — flag for the owner's box, not a sandbox).

## The recorded facts (TODO.md)

- `OVERLAP_CROSSOVER = 16` (`exec/run/overlap_leaf.rs`) and
  `FLAT_SWEEP_CEILING = 128` (`interval/overlap.rs`) are provisional.
- The finding-013 phase attribution now decomposes build vs walk vs
  residual, "so the sweep finally has its signal."
- The recorded procedure: re-pin **from the `overlap_profile` sweep**,
  never by inspection.

## Protocol

Run the sweep on a quiet machine; take the crossover the phase-decomposed
signal indicates; change the two constants only if the sweep says so;
record the sweep output beside the constants (the pin comment cites the
run).

## Acceptance

- Both constants carry a sweep-derived pin comment with the run date;
  TODO.md row closed. Overlap lanes not worse.
