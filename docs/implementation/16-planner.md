# PRD 16 — Statistics and the DP Planner

Authority: `docs/architecture/30-execution.md` (planner section: the written-down
estimator, left-deep DP, pin-at-prepare).

## Purpose

The join-order planner: real statistics in, one left-deep atom order out.

## Technical direction

- `plan::planner`. Inputs per occurrence: base row count (PRD 09 `row_count`) or the
  filtered-view survivor count when the occurrence has filters — **views for filtered
  occurrences are built before planning** (the doc's measured-not-estimated rule);
  the planner takes `&[OccStats { occ_id, rows: u64 }]` plus schema constraint
  knowledge, so it is a pure function, trivially testable.
- The estimator, transcribed exactly from the doc: joining accumulated prefix P (est
  |P|) with occurrence R on join vars J — J covers a unique constraint of R → est(|P|);
  J covers a unique constraint of every P-side occurrence contributing J... (keep it
  simple and faithful: the doc defines pairwise L⋈R rules; apply with L = the prefix,
  using the prefix's estimate and R's unique coverage; R-side-unique → min bound
  applies as est = min(est(P), |R|) per doc's min(|L|,|R|) reading); neither →
  est(P) × |R| (pessimism). Join vars J = shared vars between prefix and R. Unique
  coverage check: J ⊇ some unique constraint's field-set of R (schema lookup —
  translate constraint FieldIds to VarIds via the occurrence binding).
- Exhaustive left-deep DP over subsets (bitmask u32 — hard cap **20** occurrences,
  rejected above with a typed error; amended 2026-07-02: the originally-written 32
  is memory-infeasible for a 2ⁿ DP table (~170 GB of state at n=32) while 2²⁰ is
  ~24 MB and instant, and the doc's own envelope is ≤~12 atoms. Raise only if a
  real >20-atom query ever appears):
  `best[mask] = min over last ∈ mask of best[mask\last] extended by last`, cost = sum
  of intermediate estimates; deterministic tie-break by occ_id sequence. Disconnected
  joins (no shared vars) are legal — pessimism prices them.
- Output: `JoinOrder(Vec<OccId>)` + per-step estimates retained for EXPLAIN (PRD 24).

## Non-goals

FJ plan construction (PRD 17). Caching. Any statistic not named here (rule: a
statistic that isn't real doesn't exist).

## Passing criteria

- Unit tests: FK-walk chain orders selective-filtered occurrence first (construct
  stats where the estimator provably prefers it and assert the order); non-key join
  priced |L|×|R| and pushed last; unique-coverage detection through the auto-unique of
  a serial field; DP beats greedy on a constructed 4-atom counterexample (hand-verified
  optimum); determinism across shuffled input order; >32 occurrences rejected.
- Global commands green.
