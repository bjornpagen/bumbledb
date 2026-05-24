# PRD 07: Index-Backed GHT/COLT Sources

## Purpose

Teach GHT/COLT sources to operate over accelerator candidate sets and row-id views directly, instead of requiring a full base image before every useful lookup.

## Rosetta Alignment

This is private execution machinery. Public query semantics remain exact duplicate-free sets.

## Paper Alignment

The GHT interface is `iter` and `get`. The paper does not require every GHT to be backed by the same physical structure. An accelerator-backed source is valid if it presents the same abstract GHT behavior and preserves COLT laziness.

## Current Problem

Even after filters and better base-image loading, COLT often still builds maps by scanning offsets. JOB is rich in serial joins where an accelerator can answer a `get(key)` directly.

## Required Design

Introduce explicit source variants behind the same `GhtSource` behavior:

```text
ColumnScanColtSource
FilteredViewColtSource
AcceleratorBackedSource
EmptySource
```

Names may differ, but the trace must reveal which access path was chosen.

For accelerator-backed `get(key)`:

- use the durable accelerator prefix to retrieve row IDs;
- create a child source view over those rows;
- load only columns required at the child level;
- avoid forcing a full COLT map when the accelerator can answer the lookup.

For accelerator-backed `iter()`:

- iterate distinct keys from accelerator prefixes if cheaper;
- otherwise fall back to column/COLT iteration;
- label exact vs estimated key counts.

## Required Correctness

- Accelerator-backed and scan-backed sources must produce identical encoded tuples.
- Source replacement and frame undo logic must not care which physical source variant is used.
- Missing accelerator entries must not silently drop rows. Either prove maintenance completeness or fall back.
- Self-joins over the same accelerator-backed relation must remain independent source states.

## Tests Required

- Equality lookup via accelerator returns the same child tuples as forced COLT map.
- Accelerator miss returns empty child and increments miss counters.
- Accelerator-backed source with filters matches scan-backed source.
- Self-join using accelerator-backed sources returns correct exact set.
- Fallback mode produces identical results when accelerators are disabled.

## Trace Requirements

Add or preserve:

- source access path label per atom;
- accelerator-backed gets;
- full COLT forces avoided;
- rows loaded through accelerator child;
- fallback reason.

## Benchmark Passing Criteria

Run full traced JOB sample.

Required evidence:

- `ColtForce` time and `colt_offsets_scanned` drop materially from the post-accelerator baseline.
- `q09`, `q16`, and `q24` reduce binding conflicts by narrowing fact sources before broad iteration.
- Exact SQLite comparisons pass for all 8 JOB sample queries.
- The engine still passes with accelerators disabled.
