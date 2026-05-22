# 05 Insert Delete Write Path

## Purpose

Rebuild writes as exact set deltas over the new namespaces. This PRD owns insert/delete semantics, constraint checks, cardinality, and dictionary behavior.

## Required Insert Flow

```text
validate public tuple
encode tuple
check canonical tuple membership
if present: return AlreadyPresent with no state mutation
check unique namespaces
check FK target unique namespaces
intern dictionary values only when insertion will proceed
write canonical tuple
write unique entries
write reverse-FK entries
write access entries
increment cardinalities
commit through LMDB
```

Dictionary interning must not leak values for duplicate or rejected tuples if avoidable. If unavoidable for implementation simplicity, diagnostics must expose and justify it.

## Required Delete Flow

```text
validate public tuple
encode tuple using existing dictionary ids
check canonical tuple membership
if absent: return Absent with no state mutation
check reverse-FK restrict namespaces
delete access entries
delete reverse-FK entries
delete unique entries
delete canonical tuple
decrement cardinalities
commit through LMDB
```

## Required Code Changes

- Rewrite `WriteTxn::insert` and `WriteTxn::delete`.
- Delete `append_history` and `history_seq` from write transactions.
- Delete `record_relation_segment_change` and touched segment publishing.
- Replace `relation_row_count` and `index_entry_count` storage with namespace cardinalities.
- Remove public `alloc_id`.

## Acceptance Gates

- Only insert/delete mutate logical data.
- Exact duplicate insert returns `AlreadyPresent` and leaves all diagnostics unchanged except allowed read-only counters.
- Exact absent delete returns `Absent` and leaves all diagnostics unchanged.
- Unique conflict rejects before canonical tuple insert.
- FK violation rejects before canonical tuple insert.
- Restrict violation rejects before delete.
- Failed transaction leaves no partial namespace changes.

## Tests Required

- Random operation sequence compared to in-memory set model.
- Duplicate insert after successful insert.
- Delete then reinsert exact tuple.
- Delete exact tuple with dictionary values that do not exist returns absent, not dictionary corruption.
- Compound unique and compound FK insert/delete tests.
- Failpoint tests around each namespace write.

## Non-Goals

- No update primitive.
- No audit/history log.
- No generated ID allocator.
