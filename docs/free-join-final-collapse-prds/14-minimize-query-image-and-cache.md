# PRD 14: Minimize Query Image And Cache

## Status

Not started.

## Current State

`QueryImage` is crate-internal but still large and broad. Query image scope currently loads all columns and accesses for referenced relations. The scope machinery has relation/field/access concepts, but partial scopes are not truly safe because relation images and builders still assume field IDs index directly into loaded columns.

`QueryImageCache` is unbounded and can retain old snapshot images. Before PRD 12, each image also owns a sorted-trie cache; after PRD 12, this PRD must still bound/minimize the image cache itself.

## Objective

Make query images compact, field/access scoped, safe under partial scopes, and bounded enough for embedded use.

## Implementation Steps

1. Audit every `QueryImage`, `RelationImage`, `RelationIndexImage`, `FieldImage`, and cache method.
2. Delete or privatize methods used only by tests/diagnostics.
3. Change `query_image_scope_for_query` so it requests only relations, fields, and access paths needed by the normalized query, lazy access, comparisons, and projection decoding.
4. Fix relation image storage so `FieldId` lookups work under partial field scopes without indexing a dense `columns[field_id]` vector incorrectly.
5. Ensure builders never iterate unloaded fields while assuming dense field ID positions.
6. Add or enforce a bounded cache policy, or explicitly keep only current-snapshot images needed by active `Arc` holders.
7. Add tests proving a focused query loads fewer fields/accesses than a full relation image.
8. Add a guard for `FactId(u32)` overflow or replace it with a width that cannot truncate relation image fact counts.

## Passing Criteria

- Query image cache keys include relation, field, and access scope.
- Focused query tests prove scoped images load fewer fields/accesses than full relation images.
- Partial scope paths are safe; no `columns[field_id]` dense-index assumption remains for sparse images.
- Query image cache cannot grow without bound across write snapshots in normal embedded use.
- Public API no longer exposes query image internals.
- Full validation passes.

## Failure Modes

- Loading all fields/accesses for every referenced relation is failure.
- Making internals public to satisfy tests is failure.
- Caching scoped images under non-scoped keys is failure.
- Leaving `FactId` truncation possible is failure.

## Completion

Delete this PRD and commit.
