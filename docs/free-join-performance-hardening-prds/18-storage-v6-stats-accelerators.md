# PRD 18: Storage V6 Stats And Accelerators

## Purpose

Introduce a breaking storage v6 layout for real planner statistics and optional value accelerators once traces prove scan/load costs dominate.

## Required Preconditions

- PRD 06 trace harvest must identify base-image load, source filter scan, or planner stats as a top bottleneck.
- PRD 08 must already remove base-image dependency from planner stats.
- PRD 12 must already expose source filter survivor counts.

## Required Storage Direction

- Bump storage format version to v6.
- Reject v5 opens with a hard mismatch.
- Add durable relation stats sufficient for planner row counts.
- Add optional value accelerators for equality predicates on fixed-width encoded field values.
- Accelerators must be correctness-optional.
- Writes must update stats and accelerators atomically in the LMDB write transaction.

## Suggested Durable Keys

Reuse or revise namespaces aggressively:

```text
S | relation_id | stat_name -> encoded_stat
A | relation_id | field_id | encoded_value | fact_handle -> empty
```

Compound accelerators may be added only after single-field accelerators are measured and justified.

## Required Semantics

- Duplicate insert no-op must not change stats or accelerators.
- Absent delete no-op must not change stats or accelerators.
- Failed writes leave no partial stats or accelerators.
- Accelerators must respect LMDB snapshot visibility.

## Passing Criteria

- Opening a v5 database with v6 code fails hard.
- Insert/delete tests prove stats and accelerators are updated atomically.
- Source equality filter can retrieve candidate handles from an accelerator when present.
- Query correctness is identical with accelerators enabled and disabled.
- JOB q09 exact output remains unchanged.
- Global acceptance from PRD 00 passes.
