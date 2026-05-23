# PRD 07: Access And Constraint Layout Rebase

## 01. Status

Not started.

## 02. Severity

High storage correctness and physical layout discipline.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must complete PRD 06 first.

The implementer must write schema validation tests before changing layout generation.

The implementer must not preserve redundant generated access paths by default.

The implementer must keep constraint enforcement independent from scan access paths.

## 04. Dependency Order

PRD 05 is mandatory before this PRD.

PRD 06 is mandatory before this PRD.

PRD 08 depends on the final access layout shape from this PRD.

PRD 11 depends on explicit access descriptors from this PRD.

PRD 15 depends on accurate access and guard metadata from this PRD.

## 05. Problem Statement

Access layout generation still conflates several different concepts.

Logical constraints imply enforcement guards.

Query planning may need scan access paths.

Those are different physical needs.

Unique constraints currently generate scan access paths and unique guard entries.

Foreign keys currently generate scan access paths and reverse-FK guard entries.

Generated access names can collide with other generated names.

Generated guard key lengths are not max-key-size validated.

Relation IDs and access IDs can narrow without hard limits.

These gaps are unacceptable for the final storage model.

## 06. Code Map

Primary files:

- `crates/bumbledb-core/src/schema.rs`.
- `crates/bumbledb-lmdb/src/storage_schema.rs`.
- `crates/bumbledb-lmdb/src/storage.rs`.
- `crates/bumbledb-lmdb/src/query.rs` if query path selection changes.

Relevant current regions:

- `schema.rs:772-819` for generated access candidates.
- `schema.rs:536-555` for explicit index validation against generated names.
- `schema.rs:1252-1270` for generated name collection.
- `schema.rs:237-264` for relation and access ID narrowing.
- `storage_schema.rs:44-52` for layout map construction.
- `storage.rs:1463-1491` for guard key construction.

## 07. Current Behavior

Every relation gets a `fact_set` access path.

Every unique constraint gets a generated `unique_{name}` access path.

Every foreign key gets a generated `by_fk_{name}` access path.

Every range-indexed field gets a generated `by_{field}` access path.

Every explicit index gets an access path.

Generated names are collected into a set for explicit-index rejection.

Generated names are not checked for collisions with other generated names.

If two generated layouts have the same relation/name key, the map can silently keep one access ID.

Guard namespaces use variable-length constraint names.

Guard namespace key sizes are not validated during schema compilation.

## 08. Generated Collision Example

Relation has field `fk_parent` marked range-indexed.

Range access name becomes `by_fk_parent`.

Relation also has foreign-key constraint named `parent`.

FK access name becomes `by_fk_parent`.

Both generated access paths now share a name.

The schema validator does not reject this.

The storage schema layout map can silently collapse one path.

Queries or diagnostics can address the wrong access path.

This must become a schema validation error.

## 09. Guard Key Size Example

LMDB has a maximum key size.

Access keys are validated against that maximum.

Unique guard keys include namespace, relation ID, encoded constraint name length, constraint name bytes, and encoded key bytes.

Reverse-FK guard keys include target relation ID, target constraint name, target key bytes, source relation ID, source constraint name, and source fact ID.

A long constraint name or wide compound key can exceed LMDB key size.

Today that failure occurs at write time.

It must fail at schema compilation time.

## 10. Desired Layout Separation

Constraint descriptors are logical declarations.

Guard layouts are internal enforcement structures.

Access layouts are scan structures for query execution.

Unique constraints always need unique guard layouts.

Foreign keys always need reverse-FK guard layouts for restrict checks.

Unique constraints do not automatically need query scan access paths.

Foreign keys do not automatically need query scan access paths.

Scan access generation should be driven by query/planner needs or explicit indexes.

Until planner-driven generation exists, a conservative transition path is acceptable if documented.

## 11. Research Context

Free Join separates iteration/probe structures from logical constraints.

The same logical relation can be accessed through many physical structures.

Constraint enforcement structures are not necessarily good query structures.

Set-engine storage should avoid writing redundant keys unless they serve a proven role.

The current generated unique/FK scan paths are inherited convenience structures.

They should not be treated as a permanent physical contract.

## 12. Desired Invariants

Every generated access name is unique within a relation.

Every explicit access name is unique within a relation.

Every explicit access name avoids every generated name.

Every durable key namespace with variable width is max-key-size validated.

Every relation ID fits in its durable encoding.

Every access ID fits in its durable encoding.

Every field ID used by query images fits in its encoding.

Every guard layout is derived deterministically from schema.

Constraint enforcement never depends on scan access path existence.

## 13. Schema Validation Plan

Add validation for generated-vs-generated access name collisions.

The validation must run before access layout generation completes.

The validation must report relation and generated name.

Add validation for maximum relation count.

Add validation for maximum field count per relation.

Add validation for maximum generated access count per relation.

Add validation for maximum explicit access count per relation.

Add validation for maximum total access count per relation.

Add validation for query-facing ID limits if those IDs are schema-derived.

Return schema errors, not internal errors.

## 14. Guard Layout Validation Plan

Define a helper for unique guard key maximum length.

Define a helper for reverse-FK guard key maximum length.

The helpers must account for namespace byte.

The helpers must account for relation IDs.

The helpers must account for encoded name length fields.

The helpers must account for UTF-8 name byte length.

The helpers must account for encoded key field widths.

The reverse-FK helper must account for source fact ID bytes.

Validate every unique constraint guard key.

Validate every foreign-key reverse guard key.

Validate against the same LMDB max key size used by access layouts.

## 15. Access Generation Plan

Separate guard generation from access candidate generation.

Keep explicit indexes as scan access paths.

Keep range annotations as scan access paths.

Keep `fact_set` or its post-PRD-06 replacement as a scan path if still needed.

Evaluate whether unique constraint scan paths are needed by current query planner.

Evaluate whether FK scan paths are needed by current query planner.

If removing them breaks current query tests, add explicit indexes in test schemas where scan access is actually needed.

Do not use constraint presence as a hidden physical scan-index request long term.

If a temporary retention is required, document it as temporary and add a follow-up note for PRD 15.

## 16. Storage Schema Map Plan

When building `layout_by_relation_name`, detect duplicate keys.

Return an error if a duplicate somehow survives schema validation.

Do not let BTreeMap insertion silently overwrite a prior layout.

Add tests for defensive duplicate detection.

Keep layout IDs deterministic after removing redundant access paths.

Update tests that assume specific access counts.

Do not depend on hash-map iteration order.

## 17. Required Name Collision Tests

Generated range name collides with generated FK name.

Generated range name collides with generated unique name if possible.

Generated unique name collides with generated FK name if possible.

Explicit access name collides with generated range name.

Explicit access name collides with generated unique name.

Explicit access name collides with generated FK name.

Duplicate explicit access names still fail.

Errors name the relation and access name.

## 18. Required ID Limit Tests

Schema with too many relations fails.

Schema with too many fields fails.

Schema with too many access paths fails.

Query with too many variables fails during validation if query IDs narrow.

Query with too many inputs fails during validation if input IDs narrow.

No narrowing cast may silently wrap.

## 19. Required Guard Size Tests

Long unique constraint name exceeding key size fails schema compilation.

Long FK constraint name exceeding reverse guard size fails schema compilation.

Wide compound unique guard exceeding key size fails schema compilation.

Wide compound FK reverse guard exceeding key size fails schema compilation.

Boundary case exactly at max key size passes.

Boundary case one byte over max key size fails.

## 20. Required Behavior Tests

Unique enforcement works without relying on unique scan access.

Foreign-key insert enforcement works without relying on FK scan access.

Restrict delete works through reverse-FK guard namespace.

Queries that need scan access still pass when explicit access exists.

Diagnostics do not list removed redundant generated scan paths.

Access entry counts remain correct for remaining access paths.

## 21. Diagnostics Requirements

Schema errors must distinguish name collision from key-size overflow.

Schema errors must distinguish relation count overflow from access count overflow.

Storage diagnostics should report only actual scan access paths.

Constraint guard counts may be added if useful, but are not required.

Do not expose guard raw key bytes in public diagnostics.

## 22. Passing Criteria

Generated access name collisions are rejected.

Guard keys are max-key-size validated before writes.

ID narrowing cannot silently wrap.

Constraint enforcement is independent from scan access paths.

Storage schema layout map cannot silently overwrite duplicate names.

All existing constraint behavior remains correct.

The global validation gate passes.

## 23. Failure Modes

Letting duplicate generated names survive is a failure.

Only checking explicit names is a failure.

Failing guard key overflow at write time is a failure.

Removing scan paths needed by tests without adding explicit indexes is a failure.

Using relation IDs after unchecked narrowing is a failure.

Adding compatibility aliases for old generated names is a failure.

## 24. Non-Goals

Do not implement full optimizer access selection.

Do not implement query-image compaction.

Do not change aggregate semantics.

Do not add migrations.

Do not add user-facing runtime DDL.

Do not remove explicit user-declared indexes.

## 25. Completion Notes

Update schema docs if generated access behavior changes.

Update tests that hard-code access path lists.

Record any temporarily retained generated constraint scan paths.

This PRD must leave layout generation deterministic.
