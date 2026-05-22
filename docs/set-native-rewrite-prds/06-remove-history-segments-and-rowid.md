# 06 Remove History Segments And RowId

## Purpose

Delete the current full relation snapshot segment machinery and remove dense `RowId` from the semantic substrate. LMDB MVCC already provides read snapshots; Bumbledb should not rebuild whole relation snapshots after writes.

## Current Bad Shape

- `append_relation_segment` rebuilds a full visible segment for each touched relation.
- `build_relation_segment` scans the covering index into columns and full index chunks.
- `QueryImage` loads segments as dense column arrays addressed by `RowId`.
- `SortedTrieIndex` and `HashTrieIndex` retain row IDs.

## New Shape

Query execution should consume set/access streams:

```text
access prefix -> tuple fingerprints or key-domain values
tuple fingerprint -> canonical tuple bytes when full tuple decode is required
prefix range -> cardinality and iterator
```

Dense row IDs may exist only as a local temporary implementation detail inside one built query image, not as a public or persisted semantic concept.

## Required Code Changes

- Delete `SegmentDescriptor`, `ColumnSegmentDescriptor`, `IndexSegmentDescriptor`, `IndexStatsSummary` from public storage model.
- Delete visible segment discovery and segment byte loading.
- Remove `RowId`, `RowRange`, and `RowSetRef` from public exports.
- Rework sorted trie levels to point to child key ranges, not row ranges.
- Rework hash trie leaves to store existence/cardinality or tuple fingerprints, not row IDs.

## Acceptance Gates

- No production code publishes relation segments on commit.
- No public API exports `RowId`, `RowRange`, or `RowSetRef`.
- Query image cache key is still snapshot-scoped by LMDB/Bumbledb transaction identity.
- Read transactions remain snapshot-stable across later writes.
- No write path scans a whole relation except explicit bulk-load/compaction code.

## Tests Required

- Snapshot stability with concurrent write.
- Query image after insert sees new tuple only in later read transaction.
- Query image after delete does not see deleted tuple in later read transaction.
- No segment diagnostics remain in public storage diagnostics.

## Non-Goals

- No as-of historical query support.
- No old segment reader.
