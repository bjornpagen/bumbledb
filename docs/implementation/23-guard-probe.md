# PRD 23 — Guard-Probe Access Path Dispatch

Authority: `docs/architecture/30-execution.md` (access paths — the guard-probe
decision), `40-storage.md` (U/M readers).

## Purpose

The point-lookup fast path: route qualifying queries around the join machinery
entirely.

## Technical direction

- `exec::dispatch`. A pure classification function on `ValidatedQuery` +
  `NormalizedQuery`: **guard-probe eligible** iff exactly one atom occurrence, no
  residuals, and the occurrence's Eq-filter/bound fields cover a unique constraint
  (including serial auto-uniques) or the full fact — computed at prepare time and
  stored in the prepared query as a two-variant plan enum:
  `ExecPlan::{GuardProbe(GuardPlan), FreeJoin(ValidatedPlan)}` (a representation —
  the dispatch branch exists once, at prepare).
- `GuardPlan { relation, constraint, key_field_order, remaining_filters, projection }`.
  Execution: build the guard key from literal/param words (PRD 15's `Const` forms; a
  `PendingIntern` miss ⇒ empty result) → `unique_row`/`fact_row` (PRD 09) → `fetch` →
  evaluate any remaining filters on the fact bytes (fields outside the unique key may
  still be constrained) → feed the single binding through the ordinary sink (one-row
  set or aggregate over one binding — sinks are reused, not special-cased).
- No images are touched; works identically on a cold, just-committed database (the
  latency property the decision exists for).

## Non-goals

Multi-atom index intersection. Range probes (time-range stays O(n) by decision).

## Passing criteria

- Unit tests: classification — fully-unique-bound single atom → GuardProbe; same
  query plus a second atom or a residual → FreeJoin; partially-bound unique →
  FreeJoin; serial auto-unique qualifies. Execution — hit, miss, hit-but-residual-
  filter-rejects → empty; param-driven key; PendingIntern miss → empty without
  touching the dictionary write path; results identical to the FreeJoin path run on
  the same query (equivalence test — the two paths must agree by construction);
  aggregate over a point lookup folds one binding.
- A test asserting no image build occurs on the guard path (cache watermark
  unchanged).
- Global commands green.
