# PRD 05: Storage V6 Columnar Read Layout

## Purpose

Break the storage format to make Bumbledb physically query-native instead of adapting a row-handle layout forever.

This PRD is allowed to be destructive. v5 compatibility readers are forbidden.

## Rosetta Alignment

Rosetta explicitly rejects compatibility readers and in-place upgrades for the v5 rebuild line. A v6 format mismatch must be a hard failure. Durable storage remains LMDB only.

## Paper Alignment

The paper assumes column-oriented raw relation data. v6 must make that true as a durable storage adaptation, not just a query-time reconstruction step.

## Required Format Direction

Bump storage format version to 6.

Reject v5 opens with `StorageFormatMismatch`.

Keep canonical set membership and fact handles for exact insert/delete semantics.

Add query-native physical structures keyed for sequential relation/field reads and value lookups.

Required durable namespaces may reuse existing bytes only if the key layout is clean. Otherwise define new namespaces.

Required logical structures:

```text
canonical fact membership
fact handle lookup
live row identity
relation row ordinal or stable physical row id
column cell by relation, field, physical row id
physical row id by fact handle
fact handle by physical row id
relation stats
serial sequence
unique guards
reverse FK guards
optional accelerators
```

## Required Properties

- Duplicate insert remains idempotent.
- Absent delete remains idempotent.
- Failed writes commit nothing.
- Deleted physical row IDs are not reused unless the design proves snapshot safety.
- Read snapshots remain stable across concurrent writes.
- Column scans are sequential by relation and field.
- Source filters can retrieve physical row IDs without decoding facts.
- Fact handles remain content-derived or the replacement must prove equivalent set identity.

## Suggested Layout

One acceptable layout:

```text
T | relation_id | fact_bytes -> fact_handle
H | relation_id | fact_handle -> fact_bytes
L | relation_id | row_id -> fact_handle
P | relation_id | fact_handle -> row_id
C | relation_id | field_id | row_id -> encoded_field_bytes
Q | relation_id | field_id -> next_serial
U | relation_id | constraint_name | unique_key -> fact_handle
R | target_relation | target_constraint | target_key | source_relation | source_constraint | source_fact_handle -> empty
S | relation_id | stat_name -> encoded_stat
A | relation_id | field_id | encoded_value | row_id -> empty
```

The exact namespace names may differ, but the read path must no longer need hash-handle ordered random access to reconstruct a column.

## Tests Required

- New database writes v6 marker.
- Opening v5 data with v6 code fails hard.
- Insert/delete/duplicate/absent semantics remain identical.
- Serial generation and rollback semantics remain identical.
- FK and restrict semantics remain identical.
- Read snapshot survives concurrent write.
- Reopen verifies counts and facts.
- Column prefix scan returns row IDs in physical order with aligned values.
- Deleting a row removes it from live row and column scans under new snapshots.

## Benchmark Passing Criteria

Run full traced JOB sample on v6-loaded data.

Required evidence:

- Exact SQLite comparisons pass for all 8 JOB sample queries.
- `BaseImageLoad` improves beyond the best v5 PRD result.
- The trace shows physical row-id column scans, not fact-handle point reconstruction.
- Storage v5 compatibility code does not exist except tests that assert hard rejection.
