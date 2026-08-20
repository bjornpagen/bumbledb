# 33 — The NOSYNC commit ladder 1.24–1.44: re-baseline on the bench flag, then attribute

- **Status:** OPEN (final pass; depends on 20 — the lane's substrate
  changes).
- **Severity:** performance debt.

## The situation

TODO.md records the NOSYNC commit ladder at 1.24–1.44 versus the campaign
baseline. The purge (20) deletes the ephemeral store kind; the lane
re-anchors on the bench-private NOSYNC open flag over a durable-shaped
store. The old numbers are not comparable across that substrate change.

## Protocol

1. Land 20; wire the ladder onto the `NosyncLane` flag.
2. Re-pin the baseline on the new substrate (fresh reps, same protocol).
3. If the regression survives the re-pin, attribute via the traced twin
   from 32's tooling (same applier/judgment suspects) before any fix.

## Acceptance

- The ladder runs on the flag; a new pinned baseline is recorded in the
  bench docs; the TODO.md row is closed against the new pin or carries the
  trace that keeps it open.
