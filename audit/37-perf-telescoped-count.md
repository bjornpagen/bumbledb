# 37 — Deferred finding 044: forced-map telescoped distinct Count

- **Status:** **fixed this pass** — skipped: 34's flame demotes
  forced-map materialization (`jp_force_n0` 1.25 µs on
  `r6_two_path_count`).
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

## Trace evidence (from 34)

`r6_two_path_count` warm capture, 2026-08-20, obs release, rings corpus
387_054 facts:

| phase | calls | excl_us |
| --- | ---: | ---: |
| `jp_descend_n1` | 99_950 | 197_689 |
| `jp_iter_n1` | 205_928 | 13_807 |
| `jp_force_n0` | 528 | **1.25** |

`JoinPhase::Force` is "the single biggest non-amortized cost a node
entry can pay" (`exec/run.rs`) — and it does not rank. The Count-shaped
cost is n1 descend bookkeeping, not forced-map ingest. No telescope
landed.

## Acceptance

- Landed with byte-identical answers and oracle agreement, or closed by
  34's trace showing it no longer ranks; TODO.md row closed.
