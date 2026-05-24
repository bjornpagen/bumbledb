# PRD 09: Dynamic Cover And Access Costing

## Purpose

Make dynamic cover choice choose the cheapest current source access path, not merely the smallest guessed offset length.

## Rosetta Alignment

Dynamic cover choice is private execution strategy. It must preserve exact set output.

## Paper Alignment

The paper chooses the cover with the fewest keys to preserve Generic Join's intersection principle, but it also notes that COLT cannot know exact key counts without forcing and that traditional plans trade runtime against build cost. Bumbledb must expose exact vs estimated counts and account for accelerator-backed access.

## Current Problem

`DynamicMinKeys` currently relies on source `key_count` style estimates. With COLT vectors, estimates may be row counts, not distinct keys. With accelerators, lookup cost can be cheap even when a source is physically large.

## Required Design

Represent cover cost as:

```text
ExactKeys(count)
EstimatedKeys(count, reason)
Unknown(reason)
AccessCost(iter_cost, get_cost, force_cost, source_kind)
```

Cover choice must consider:

- distinct key count if exact;
- survivor row count if only estimate;
- force cost if choosing this cover requires map construction;
- accelerator iteration or lookup cost;
- current prefix/source frame;
- deterministic tie-breaking.

Do not force a COLT only to discover a key count unless the cost model explicitly decides the force is cheaper than a bad cover choice and traces that decision.

## Tests Required

- Exact map key count beats larger estimates.
- Smaller estimate beats unknown when no exact count exists.
- Accelerator-backed source can win even when base relation is large.
- Deterministic tie-breaking is stable.
- Prefix-sensitive cover choice changes after descending into subtries.
- Force-for-count, if implemented, is traced and justified.

## Trace Requirements

Each `CoverChoice` span must include for every candidate:

- atom occurrence;
- source kind;
- exact or estimated key count;
- force cost estimate;
- accelerator availability;
- chosen/rejected marker;
- tie-break marker.

## Benchmark Passing Criteria

Run full traced JOB sample.

Required evidence:

- Cover-choice trace contains no unlabeled estimates.
- Total `colt_offsets_scanned` and `probe_calls` do not regress against PRD 08.
- Exact SQLite comparisons pass for all 8 JOB sample queries.
- Dynamic cover choices are explainable for q09/q16/q24 and broad queries.
