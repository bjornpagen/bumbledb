# PRD 13 — On-demand distinct stats: stop paying planning tax on the cold path

## Purpose

The cold-after-write profile is dominated by a planning input: the
distinct-count stats pass walks the image (~1.8 ms floor per 150 k rows,
measured in `image_build_split_evidence`) to feed the semantic planner's
estimates — paid eagerly, for every column, before the first query can
run, even when the first query is a guard probe that needs no estimates
at all. The findings frame it correctly: this is a fixed instruction-work
floor (a walk at memory speed), and the only winning move is to not run
it — compute per-column stats lazily, on first demand, for exactly the
columns a plan needs, and cache them for the snapshot's lifetime.

## Technical direction

`crates/bumbledb/src/image.rs` (the stats pass), the planner's estimate
consumption (`crates/bumbledb/src/plan/`), `api/db.rs`/`api/prepared.rs`
(where prepare meets storage).

- **Map the consumers first.** Find every reader of the eager stats
  (plan estimates: distinct counts per column feeding `plan.estimates()`,
  sink capacity hints, cover selection). The end state must deliver
  identical VALUES to every consumer — laziness must be invisible to
  planning outcomes (gate 3 pins this).
- **Rip out the eager pass.** The stats walk leaves the image-build /
  open / first-execute path entirely. No transitional flag, no
  keep-both — delete it.
- **`StatsCache` on the Db handle.** Keyed by (snapshot id, relation,
  column): `get_or_compute(col) -> DistinctStats`. Compute walks ONLY the
  requested column via the existing decode machinery (reuse the
  `decode_plan`/`fill_columns` column-selective path from perf-PRD 12 —
  the walk floor is per-column now, not per-image). Invalidation: a new
  snapshot (any commit) clears or re-keys the cache — correctness first,
  no cross-snapshot reuse in this PRD (incremental maintenance is a
  named follow-up, not scope).
- **Prepare-time, not execute-time.** Stats are consumed at
  prepare/validate (cover choice, hints). Compute-on-first-prepare is the
  design point: execute paths stay zero-alloc and stats-free. A prepared
  query re-executed across snapshots re-validates today (snapshot check);
  its re-prepare path pulls from the cache under the new snapshot.
- **Guard plans need nothing.** Verify by construction: the guard fast
  lane's prepare must not touch the cache (grep/test gate) — the
  cold-write→point-read path becomes stats-free end to end.
- **Threading/borrow discipline** follows the existing Db model (same
  rule as PRD 12: no new locks on hot paths; the cache lives where the
  snapshot state already lives).

## Passing requirements

1. Cold-after-write, traced (the PRD's headline): commit → first point
   execute shows ZERO stats-walk time (the flame row is gone); commit →
   first stats-family execute pays only that plan's columns (recorded
   split: per-column walk ≤ 40% of the old full-image pass on the bench
   store).
2. Measured (vs post-12, min-of-5): cold-suite numbers re-recorded — the
   cold ratio vs SQLite improves (baseline 92× win holds or grows);
   commit_batch p50 improves or holds (the eager pass left the write
   path); warm family p50s unchanged ±2% (stats now hit the cache).
3. Planning equivalence: a test asserts `plan.estimates()` and chosen
   covers are IDENTICAL (values, not just shape) pre/post for every bench
   family and the verify corpus — laziness changed when, never what.
4. Guard-lane isolation: test proves preparing/executing a guard plan
   performs no stats computation (counter or cache-instrumentation under
   `#[cfg(test)]`).
5. No family regresses >5% (confirm-run); verify green; zero-alloc at
   execute holds (cache writes happen at prepare).

## Out of scope

Incremental stats maintenance at commit (named follow-up; needs its own
correctness argument); sampled/approximate stats (rejected: planner
inputs must stay exact — premise from the perf campaign's stats-elision
correction); any stored-format change for stats (cache is in-memory;
humans own persistence decisions).
