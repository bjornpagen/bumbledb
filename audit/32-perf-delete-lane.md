# 32 — `cold_containment_walk_delete` 3.1–3.4×: trace the untraced twin, then fix

- **Status:** OPEN (final pass; TODO.md storage-lane regression cluster,
  recorded at campaign close).
- **Severity:** performance debt, attribution-first.

## The recorded facts (TODO.md)

- `cold_containment_walk_delete` 3.1–3.4× in all six reps.
- Traced suspects from b100: `apply_deletes` self-time 2.1× under the
  cursor-fold applier; `judgment_source` +70% under the T8 walk at small
  batch (the sweep only priced 1k–4k parents).
- **The delete lane has no traced twin** — the reps' write set is untraced
  by protocol.

## Protocol (house rule: data first, no intuition fixes)

1. Light the delete lane's traced twin (same protocol as the existing
   flamediffs; the close's diffs live in git history:
   `writes.durable.delete_b100.diff.svg`).
2. Rank by trace-reader; fix the top attribution only; re-run the six reps.
3. Note: the T8 walker is now the `SortedGets` trait impl and the applier
   sits under `MutationCore` — re-trace on the CURRENT tree before
   believing the campaign-close suspects.

## Acceptance

- The lane's traced twin exists and is repeatable.
- The 3.1–3.4× either closes to a stated target or is re-ruled with the
  trace attached; TODO.md row closed either way.
