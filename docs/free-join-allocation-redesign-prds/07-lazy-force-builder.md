# PRD 07: Lazy Force Builder

## Purpose

Rebuild COLT `force` around the arena map, offset pool, and scratch key model.

## Required Work

- Implement force for `Range`, `Singleton`, and `Offsets` node data.
- Use base-image column buffers directly to construct `KeyRef`/scratch bytes.
- Insert keys into flat map table.
- Append child offsets into arena storage.
- Create child nodes as arena records.
- Avoid intermediate grouping maps.
- Preserve lazy behavior: no force until `get` or non-suffix key iteration requires it.

## Passing Criteria

- Force allocation fixture shows allocation calls proportional to forced map/table allocation, not row count.
- Duplicate-heavy fixture allocates one child per distinct key, not one child per row.
- `colt_get_forces_root_once_and_finds_child` equivalent passes on arena path.
- `colt_second_level_lookup_forces_only_selected_child` equivalent passes on arena path.
- q09 exact SQLite comparison passes with arena force path enabled.
- Global gates pass.
