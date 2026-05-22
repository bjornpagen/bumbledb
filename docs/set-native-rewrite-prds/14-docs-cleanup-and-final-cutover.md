# 14 Docs Cleanup And Final Cutover

## Purpose

Make documentation match the implemented set-native system and delete obsolete v3/v6 language that describes the old physical model.

## Required Documentation Changes

- Rewrite `docs/ROSETTA_STONE.md` as the normative set-native contract.
- Mark old benchmark RCA docs as historical if they mention deleted systems.
- Update examples to use new aggregate-domain API.
- Update schema examples to remove covering unique constraints.
- Update write semantics to say only insert/delete exist.
- Update storage diagrams to show canonical tuple, unique, reverse-FK, and access namespaces.
- Update benchmark docs to explain value correctness gates.

## Required Code Cleanup

- Delete obsolete tests that assert old segment/history/covering behavior.
- Delete obsolete counters that mention hash probe or bag multiplicity if no longer used.
- Delete compatibility terminology from public docs and comments.
- Ensure storage format version and schema canonical version are final for the rewrite.

## Acceptance Gates

- `ROSETTA_STONE.md` does not mention covering unique as required physical storage.
- `ROSETTA_STONE.md` does not mention updates or `on_update`.
- `ROSETTA_STONE.md` describes aggregate domains exactly as implemented.
- Docs search for old deleted terms has either zero matches or explicitly historical matches.
- Full validation suite passes.
- Final non-JOB and JOB subset benchmark artifacts are recorded.

## Final Completion Checklist

- Old storage format rejected.
- Old covering physical model deleted.
- Old row-id public exports deleted.
- Old ambiguous count semantics deleted.
- Golden examples pass and are richer than before.
- Benchmark correctness compares exact values.
- Fuzz/property/failpoint tests cover the new storage model.

## Non-Goals

- No compatibility appendix for reading old databases.
- No migration guide beyond ETL into a new database.
