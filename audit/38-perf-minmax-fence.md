# 38 — Deferred finding 009 step 2: the per-forced-map min/max fence

- **Status:** OPEN (final pass; TODO.md "Audit-2026-07 deferred findings").
- **Severity:** performance debt, small.

## The recorded fact

Finding 009's step 2 — a per-forced-map min/max fence — was deferred at the
campaign close (the R5 Arg/CountDistinct family it neighbored is killed;
the fence itself survived the kill as still-applicable to the surviving
`Min`/`Max` folds).

## Protocol

Confirm the fence's premise still holds on the current tree (the fold sink
became `GroupState` this pass); if it holds, land the fence; if the
`GroupState` restructure absorbed it, close by inspection note plus the
fold-lane numbers.

## Acceptance

- Fence landed or closed-with-reason; fold lanes not worse; TODO.md row
  closed.
