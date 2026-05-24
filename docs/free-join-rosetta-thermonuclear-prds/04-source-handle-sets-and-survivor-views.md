# PRD 04: Source Handle Sets And Survivor Views

## Purpose

Represent filtered sources as survivor handle/offset views over shared columns instead of copying or loading full-column images for rows that are already dead.

## Rosetta Alignment

This preserves snapshot visibility and set semantics. It changes only the internal physical representation of source candidates.

## Paper Alignment

COLT leaves are offsets into a base relation. This PRD makes those offsets point into a survivor view when filters or accelerators have narrowed a source. The GHT interface remains `iter` and `get` over encoded tuples.

## Current Problem

After PRD 02, source filtering may produce survivor handles before all fields are loaded. If the implementation still expands survivors into copied full images eagerly, it wastes memory and risks replacing one bottleneck with another.

## Required Design

Introduce a compact source row selection type:

```text
AllRows
HandleSet(sorted handles)
OffsetSet(sorted offsets)
Range(start, len)
Empty
```

COLT root data should be initialized from this selection without duplicating handles unnecessarily.

Column access should resolve:

```text
source offset -> base row offset -> shared column value
```

For an unfiltered full relation, this must stay as cheap as today or cheaper.

For a filtered relation, non-filter columns should be loaded only for survivors when that is cheaper than full-column load.

## Required Heuristic

Use a simple deterministic rule first:

- If survivors are empty, return empty source.
- If survivors are less than 50% of live rows, load plan columns by survivor handles.
- If survivors are at least 50%, scan full column prefix and build a survivor view.

Trace which path was chosen.

The heuristic may be replaced later by PRD 09 costing.

## Tests Required

- Empty survivor view yields no tuples and builds no maps.
- Sparse survivor view loads only survivor plan values.
- Dense survivor view preserves full scan behavior.
- Source offsets map back to the correct shared column values.
- COLT `get` and `iter` produce identical tuples for full and survivor views where logically equivalent.
- Allocation does not scale with eliminated rows for sparse survivor views.

## Trace Requirements

Add counters:

- survivor views created
- survivor handles
- eliminated handles
- sparse column loads
- full column loads selected by heuristic

## Benchmark Passing Criteria

Run the full traced JOB sample.

Required evidence:

- Filtered queries show fewer non-filter column values loaded than PRD 02.
- `q09`, `q16`, and `q24` eliminate rows before node-0 recursive work when filters allow it.
- Exact SQLite comparisons pass for all 8 JOB sample queries.
- The trace must explain each sparse/dense choice.
