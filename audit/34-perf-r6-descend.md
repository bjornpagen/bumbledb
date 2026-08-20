# 34 — `r6_two_path_count` 1.46: `jp_descend` carries the query

- **Status:** OPEN (final pass; TODO.md row, recorded at campaign close).
- **Severity:** performance debt, attribution-first.

## The recorded facts (TODO.md)

- Ours 131→197 ms on the sink/plan lane's COUNT-shaped territory.
- Flame: `scenarios.rings.r6_two_path_count.warm.diff.svg` — `jp_descend`
  51% + 45%; "descend now carries essentially the whole query."

## Protocol

Trace-reader ranking on the CURRENT tree first (the evaluator consolidation
and `NodePrecompute` landed since the flame was cut — the attribution may
have moved). Then one ranked fix; re-run the rings lane; compare against
the SQLite twin per the scenario protocol.

## Acceptance

- Fresh flamediff recorded; the 1.46 closes to a stated target or is
  re-ruled with the trace attached; TODO.md row closed.
