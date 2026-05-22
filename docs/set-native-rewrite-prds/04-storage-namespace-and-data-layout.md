# 04 Storage Namespace And Data Layout

## Purpose

Replace row-shaped full-covering indexes with explicit set-native LMDB namespaces. This is the storage-format break.

## New Namespaces

Canonical tuple set:

```text
T | relation_id | tuple_bytes -> empty
```

Unique keys:

```text
U | relation_id | unique_id | key_bytes -> tuple_fingerprint
```

Reverse FK guards:

```text
R | target_relation_id | fk_id | target_key_bytes | source_relation_id | source_tuple_fingerprint -> empty
```

Access paths:

```text
A | relation_id | access_id | access_key_bytes | tuple_fingerprint -> optional payload_bytes
```

Dictionary:

```text
D remains intern forward/reverse unless a later bulk-ingest PRD replaces it
```

Metadata:

```text
M | storage_format_version
M | schema_fingerprint
M | relation_cardinality | relation_id -> u64
M | access_cardinality | relation_id | access_id -> u64
```

## Tuple Fingerprint

Tuple fingerprint must be stable within a storage format and collision-checked:

- Hash full `relation_id || tuple_bytes` with BLAKE3.
- Store enough bytes for practical collision safety.
- On unique/access insert, collision must be verified against canonical tuple membership where needed.

## Required Code Changes

- Replace `current_index_prefix/current_index_key` with namespace-specific key builders.
- Replace `EncodedTuple` as “row plus relation” with `EncodedFact` or equivalent.
- Store exact tuple once in canonical namespace.
- Store secondary access entries without copying all fields.
- Add access payload only when query workloads prove it is needed.
- Bump `STORAGE_FORMAT_VERSION`.
- Reject old format with no conversion path.

## Acceptance Gates

- One inserted tuple creates exactly one canonical tuple key.
- Secondary access entries do not include undeclared fields unless explicitly configured as payload.
- Exact duplicate insert does not write canonical, unique, reverse-FK, access, cardinality, or dictionary state after membership is known.
- Storage format version changes and old v2 database open fails.
- Relation cardinality equals canonical tuple namespace count after random operations.

## Tests Required

- Direct namespace key encode/decode tests.
- Duplicate insert write-count or diagnostics test.
- Unique conflict with different tuple test.
- Reverse-FK entry appears on insert and disappears on delete.
- Access index prefix scan returns tuple identities and can decode full tuple through canonical namespace.

## Non-Goals

- No segment compatibility.
- No old full-covering index preservation.
