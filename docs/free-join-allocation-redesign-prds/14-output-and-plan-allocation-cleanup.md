# PRD 14: Output And Plan Allocation Cleanup

## Purpose

Remove remaining non-COLT query allocation hotspots after the arena redesign.

## Required Targets

- Plan candidate generation allocations.
- Repeated `String` labels or diagnostic formatting in hot paths.
- Projection encoding scratch allocation.
- `BTreeSet<Vec<u8>>` or result materialization overhead if it remains material.

## Required Work

- Use no-trace allocation fixtures to identify non-COLT hotspots.
- Add scratch buffers for projection encoding if needed.
- Avoid allocating diagnostic labels when tracing is disabled.
- Keep public `QueryResultSet` canonical and duplicate-free.

## Passing Criteria

- A before/after allocation table identifies every changed hotspot.
- No-trace JOB allocation calls improve on at least one non-empty query.
- q09 and broad exact SQLite comparisons pass.
- Trace output remains correct when query tracing is enabled.
- Global gates pass.
