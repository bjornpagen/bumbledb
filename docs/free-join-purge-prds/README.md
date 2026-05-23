# Free Join Purge Suite

## Status

Completed.

## Result

The ordered Free Join purge suite has been applied. Completed PRD files were removed according to project policy.

## Current Contract

- `FreeJoinPlan` is the join execution authority.
- Query execution is routed through Free Join/LFTJ runtime code.
- Lazy access slices may avoid eager atom trie construction for supported one- and two-variable atom shapes.
- Eager sorted trie construction remains only as an unsupported-shape fallback.
- Benchmark and explain output use Free Join plan family language.
- Full validation and source hygiene gates must stay green.
