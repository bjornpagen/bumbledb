# 36 — Deferred finding 014: per-parent leaf batch-of-1

- **Status:** OPEN (final pass; TODO.md "Audit-2026-07 deferred findings").
- **Severity:** performance debt, attribution-first.

## The recorded facts (TODO.md)

Finding 014 — the per-parent leaf runs batch-of-1. The campaign's pinned-run
fold `a75d1e65` "lands the adjacent mechanism, but o4's lane was not
re-benched."

## Protocol

Re-bench the o4 lane on the current tree first — the adjacent fold may
already have absorbed the win. If the batch-of-1 still shows, batch it
along the landed fold's mechanism; if it does not, close the row with the
re-bench numbers.

## Acceptance

- The o4 lane re-benched and recorded; the finding fixed or closed-by-
  measurement; TODO.md row closed.
