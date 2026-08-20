# 37 — Deferred finding 044: forced-map telescoped distinct Count

- **Status:** OPEN (final pass; TODO.md "Audit-2026-07 deferred findings").
- **Severity:** performance debt.

## The recorded fact

Finding 044 — the forced-map path can telescope a distinct Count instead of
materializing. Deferred at the campaign close; the COUNT-shaped territory is
the same lane 34 traces (`r6_two_path_count`), so sequence this after 34's
fresh attribution — 34's trace may promote or demote this row.

## Protocol

Take 34's fresh flame first. If the forced-map materialization ranks, land
the telescoped form; verify against the `Count` semantics pins
(distinct-set law) and the differential oracle.

## Acceptance

- Landed with byte-identical answers and oracle agreement, or closed by
  34's trace showing it no longer ranks; TODO.md row closed.
