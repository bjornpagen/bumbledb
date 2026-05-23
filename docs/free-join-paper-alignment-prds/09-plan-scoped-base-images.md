# PRD 09: Plan-Scoped Base Images

## Purpose

Replace pre-plan, access-entry-centered query images with plan-scoped immutable relation base images suitable for GHT/COLT execution over LMDB snapshots.

## Dependencies

- PRD 08.
- PRD 03.

## Scope

- Query image/base image construction.
- Cache keys and scopes.
- Relation column loading from v5 `L/C` namespaces.
- Removal or demotion of eager `RelationIndexImage` as the primary query source.

## Required Model

Introduce an immutable base image similar to:

```rust
struct RelationBaseImage {
    relation: RelationId,
    name: String,
    row_handles: Vec<FactHandle>,
    fields: Vec<FieldImage>,
    columns: BTreeMap<FieldId, ColumnImage>,
    stats: RelationStats,
}
```

The exact type may differ, but it must provide:

- Dense snapshot-local offsets for live facts.
- Column access by field ID and offset.
- Deterministic live-row order under one read snapshot.
- Enough field metadata to build GHT tuple keys.
- No public API exposure.

## Required Behavior

- Build base images from v5 live row and column namespaces.
- Scope loaded fields from the chosen formal Free Join plan, not from a pre-plan atom scan.
- Cache base images by schema fingerprint, storage tx ID, relation IDs, and field scope.
- Optional accelerator images are separate from base images and must not be required for correctness.
- Query image cache invalidation remains tx-ID based.

## Technical Direction

- Rename `QueryImage` only if needed. The important change is semantic: immutable base relation columns first, plan source structures later.
- Build from `L` row handles then load `C` columns for scoped fields.
- Keep fixed-width encoded column representation for supported persistent types.
- Decide whether `row_handles` are always loaded or only loaded when diagnostics/reconstruction need them.
- Plan first, image second. If a temporary bridge still builds images before final planning, it must be deleted before PRD 22.

## Non-Goals

- Do not implement GHT/COLT here.
- Do not implement vectorized execution here.
- Do not expose base images publicly.

## Acceptance Criteria

- Base image construction no longer scans old fact-set access entries as the primary source.
- A relation with no optional accelerators can still produce a base image.
- Base image columns align exactly with live row handles.
- Query/base image scope is derived from a validated plan's required fields.
- Cache keys include storage tx ID and field/relation scope.
- Future Free Join sources must consume base images; no legacy LFTJ adapter is retained.

## Required Tests

- Base image row count equals relation fact count.
- Every loaded field column length equals row handle length.
- String/bytes intern IDs decode through dictionary.
- Deleting a row removes it from new base images after commit.
- A read transaction sees stable base image while a writer commits.
- Cache hit for same tx/scope.
- Cache miss for changed tx or changed scope.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb base_image --all-features
cargo test -p bumbledb-lmdb storage --all-features
cargo test --workspace --all-features
```
