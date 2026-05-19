# PRD 04: Index Layout And Access Paths

## Status

Draft. This PRD depends on PRD 01 and should account for PRD 03 FK-generated indexes.

## Problem

Current indexes are always covering indexes: leading fields first, then every remaining relation field. This is excellent for narrow BCNF relations, but the behavior is implicit, over-broad, and difficult to reason about as constraints and fast paths grow.

The database should make index shape explicit and performance-oriented.

## Goals

- Make generated and explicit index layouts deterministic and unambiguous.
- Separate leading fields from covering fields and row identity suffixes.
- Keep hot keys small.
- Preserve row reconstruction where it is intentionally chosen.
- Generate indexes for primary, unique, FK, range, equality, and permutation access.
- Stop silent index deduplication surprises.

## Non-Goals

- No vector indexes.
- No FlatBuffer indexes.
- No text prefix/full-text indexes.
- No arbitrary variable-width index keys.
- No one-DBI-per-index design.

## Current Code References

- `RelationDescriptor::index_candidates` in `schema.rs`.
- `RelationDescriptor::covering_components` in `schema.rs`.
- `CurrentIndexLayout` and `IndexComponent` in `schema.rs`.
- `current_index_key` in `storage.rs`.
- `scan_prefix`, `scan_range`, and `scan_encoded_index_prefix` in `storage.rs`.
- `RelationIndexImage` in `query_image.rs`.
- Planner access path enumeration via `StorageSchema::access_paths`.

## Required Model

Replace the implicit all-covering model with explicit layout policy:

```rust
pub struct IndexDescriptor {
    pub name: String,
    pub kind: IndexKind,
    pub leading_fields: Vec<String>,
    pub covering_fields: IndexCoveringPolicy,
}

pub enum IndexCoveringPolicy {
    KeyOnly,
    PrimaryKey,
    Fields(Vec<String>),
    FullRow,
}
```

Generated indexes should use policies intentionally:

| Index | Leading Fields | Covering Policy |
| --- | --- | --- |
| primary | primary key | FullRow |
| unique | unique fields | PrimaryKey |
| foreign key | source FK fields | PrimaryKey |
| range | range field plus primary key suffix | PrimaryKey or selected fields |
| equality | explicit fields | PrimaryKey or selected fields |
| permutation | explicit fields | PrimaryKey or selected fields |

The default for explicit indexes should be `PrimaryKey`, not `FullRow`, unless the schema asks for full coverage.

## Key Layout

Target key layout:

```text
namespace | relation_id | index_id | leading components | covering components | primary identity suffix when needed
```

Every index entry must be unique. If leading plus covering fields do not guarantee uniqueness, append primary-key bytes as an identity suffix.

## Access Path Descriptor

`AccessPathDescriptor` must expose:

- relation name
- index name
- index kind
- leading fields
- covering fields
- identity suffix fields
- whether full row reconstruction is possible from the index alone
- encoded key width

## Query Implications

- Direct paths can use key-only/primary-key indexes and fetch row payload if needed.
- Query image can still build fixed columns from the primary full-row index or row payload.
- Planner should know whether an access path can satisfy projected fields without row fetch.
- The row store becomes more important if non-primary indexes stop covering full rows.

## Storage Implications

- `get_row` should prefer `NS_CURRENT_ROW` or primary full-row index consistently.
- Non-primary index scans may yield row identities, not full rows.
- `IndexScan` may need to fetch rows by primary key for non-covering layouts.
- Segment index images must preserve enough metadata to decode prefixes and identities.

## Deduplication Rules

- Duplicate index names are rejected.
- Duplicate generated leading-field layouts are allowed only if names/kinds differ for a documented purpose.
- Silent deduplication by field vector is forbidden.
- If two generated indexes would be identical, validation must either merge them explicitly with a deterministic name or reject the schema with a clear error.

## Implementation Plan

1. Add explicit covering policy types.
2. Update generated index candidate construction.
3. Update `CurrentIndexLayout` to include leading, covering, and identity suffix roles.
4. Update key construction and decoding.
5. Update current index write/delete paths.
6. Update read scans and row reconstruction.
7. Update query image segment index descriptors.
8. Update planner access path descriptors.
9. Update tests for index layout order, widths, and scan behavior.

## Strict Passing Criteria

- No index silently disappears due to duplicate field-vector deduplication.
- Generated index names are reserved and collision-free.
- Every index entry remains unique.
- Non-primary indexes no longer cover full rows unless explicitly configured.
- Primary index remains full-row reconstructible.
- `scan_prefix` and `scan_range` work for non-covering indexes by fetching row payloads when required.
- Query image build remains deterministic.
- Benchmark gated queries still pass.
- LMDB key-size validation accounts for exact generated layout.

## Verification Commands

```sh
cargo test -p bumbledb-core schema
cargo test -p bumbledb-lmdb storage query_image
cargo test --workspace --all-features
scripts/bench-focused.sh --fail-gates
```
