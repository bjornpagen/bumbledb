# PRD 06: Canonical Fact Storage Single Owner

## 01. Status

Not started.

## 02. Severity

High storage architecture and performance.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must complete PRD 05 first.

The implementer must write raw-layout tests before changing durable namespaces.

The implementer must bump storage format when durable layout changes.

The implementer must not add old-format readers.

## 04. Dependency Order

PRD 05 is mandatory before this PRD.

PRD 07 depends on this PRD for final access and guard key assumptions.

PRD 08 depends on this PRD because query images must stop depending on full fields in `fact_set` access keys.

PRD 13 depends on this PRD because lazy GHT/COLT should reference fact identity, not duplicate fact bytes.

## 05. Problem Statement

Canonical fact bytes are still duplicated across durable namespaces.

The canonical namespace stores full encoded fact bytes.

The fact-id lookup namespace stores full encoded fact bytes as values.

The `fact_set` access path can store every field as access-key components.

This creates at least two and often three durable copies of the same encoded fact.

It also makes query-image construction depend on a scan access path that carries full fact data.

That is contrary to the target storage model.

The target model has one owner for fact bytes and separate access structures containing keys plus fact identity.

## 06. Code Map

Primary files:

- `crates/bumbledb-lmdb/src/storage.rs`.
- `crates/bumbledb-core/src/schema.rs`.
- `crates/bumbledb-lmdb/src/query_image.rs`.
- `crates/bumbledb-lmdb/src/lib.rs` for storage format version.

Relevant current regions:

- `storage.rs:409-420` for canonical fact and fact-id lookup writes.
- `storage.rs:425-431` for canonical delete.
- `storage.rs:1501-1533` for fact-id helpers.
- `storage.rs:1556-1571` for access key construction.
- `schema.rs:775-779` for `fact_set` access using all fields.
- `query_image.rs:1431-1465` for columns built from `fact_set` key components.

## 07. Existing Layout

Canonical fact key stores namespace, relation ID, and full fact bytes.

The canonical value is empty.

Fact-id key stores namespace, relation ID, and fact ID hash.

The fact-id value stores full fact bytes.

Access key stores namespace, relation ID, access ID, declared key fields, and fact ID.

For `fact_set`, declared key fields are currently all relation fields.

Therefore `fact_set` duplicates full fact bytes inside access keys.

Query image column building scans `fact_set` and reads components from that key.

This makes full-field `fact_set` shape part of query-image construction.

That coupling must be removed.

## 08. Target Layout

There must be exactly one durable owner of encoded fact bytes.

Access paths must contain access key bytes and durable fact identity.

Constraint guards must contain constraint key bytes and durable fact identity where needed.

Full relation scans must be able to recover fact bytes from the canonical owner.

Prefix scans must be able to recover fact bytes from access key fact identity.

Query-image column construction must read from canonical owner or a fact-byte iterator, not from full `fact_set` access components.

Exact membership must remain collision-safe.

Deletes must remove canonical owner and all dependent keys atomically.

## 09. Preferred Design

Use fact ID as the canonical durable key.

Canonical key: `NS_CANONICAL_FACT | relation_id | fact_id`.

Canonical value: `fact_bytes`.

Exact membership check reads canonical value and compares bytes.

Collision detection rejects different fact bytes with same fact ID.

Access keys store `declared_key_bytes | fact_id`.

Unique guard values store fact ID.

Reverse-FK guard keys store source fact ID.

`fact_set` access stores only fact ID or a minimal canonical-order key plus fact ID.

This design removes full fact bytes from access keys.

This design keeps one full fact-byte copy.

## 10. Alternative Design

Canonical key remains full fact bytes.

Fact-id lookup stores only a pointer-like marker or no duplicate bytes.

This preserves exact membership by key.

This still duplicates full fact bytes in LMDB keys.

This is less preferred.

Use this alternative only if the preferred design creates unacceptable ordering or lookup complexity.

If this alternative is chosen, document why it is temporary.

## 11. Collision Requirements

Fact ID is derived from relation ID and encoded fact bytes.

Fact ID collisions must not alias facts silently.

On insert, if canonical key exists and value bytes differ, return hash collision error.

On exact exists, if canonical key exists but value bytes differ, return hash collision error.

On delete, if canonical key exists but value bytes differ, treat as collision, not absent.

Tests need a collision injection path if natural collisions are impractical.

Collision handling must be deterministic.

## 12. Storage Format Requirements

Changing canonical namespace layout requires a storage format bump.

The old format must be rejected.

No old-format reader may be added.

No in-place upgrade may be added.

No migration code may be added.

Bulk ETL into a new database remains the only transition path.

Tests must verify the previous format version is rejected.

Docs must mention the new format incompatibility.

## 13. Insert Path Requirements

Validate fact values before mutating storage.

Encode fact once.

Compute fact ID once.

Check canonical owner for exact presence or collision.

Check foreign keys using unique guard namespaces.

Check unique constraints using unique guard namespaces.

Insert canonical owner.

Insert unique guard entries.

Insert reverse-FK guard entries.

Insert access entries.

Update stats.

Ensure PRD 05 poison behavior covers every error after mutation begins.

## 14. Delete Path Requirements

Validate fact values before mutating storage.

Encode fact using existing dictionary entries.

Compute fact ID once.

Check canonical owner for exact presence or collision.

Check delete restrictions using reverse-FK guard namespace.

Delete access entries.

Delete reverse-FK guard entries.

Delete unique guard entries.

Delete canonical owner.

Update stats.

Absent delete remains a successful no-op.

## 15. Read Path Requirements

Exact fact exists must use canonical owner and byte comparison.

Access scans must decode fact ID from access key and load fact bytes from canonical owner.

Full relation scans must not depend on `fact_set` containing every field.

Range scans must still use declared range access keys.

Prefix scans must still return full facts.

Snapshot stability must remain unchanged.

Reopen behavior must remain unchanged for new format databases.

## 16. Query Image Requirements

Column image construction must not require full field components inside `fact_set` access key.

Build columns by scanning canonical owner or scanning minimal fact identity and loading canonical fact bytes.

The selected method must preserve deterministic fact order inside relation images.

If fact order changes, update tests to assert set equality where order is not part of public contract.

If fact ID hash order is used, document that image fact IDs are snapshot-local ordering IDs, not durable fact IDs.

## 17. Required Raw Layout Tests

Insert a fact and scan raw LMDB keys by namespace.

Assert exactly one namespace stores the full encoded fact bytes.

Assert access entries do not contain non-key field bytes.

Assert `fact_set` access does not contain every field as encoded components.

Assert unique guard contains constraint key and fact ID only.

Assert reverse-FK guard contains target key and source fact ID only.

Assert delete removes canonical owner and all dependent keys.

Assert reopen preserves exact facts and scans.

## 18. Required Behavioral Tests

Insert and scan relation.

Insert exact duplicate and get no-op.

Delete exact fact and verify absence.

Delete absent fact and get no-op.

Prefix scan returns expected facts.

Range scan returns expected facts.

Unique violation still fails.

Foreign-key violation still fails.

Restrict delete still fails.

Bulk load still dedups exact duplicate facts.

Read snapshot remains stable after later write.

Reopen database and scan facts.

## 19. Required Collision Tests

Add test-only collision injection if feasible.

Insert first fact with injected fact ID.

Insert different fact with same injected fact ID.

Assert hash collision error.

Assert no partial second fact state commits.

Delete with collision mismatch returns collision error.

Exact exists with collision mismatch returns collision error if reachable.

## 20. Diagnostics Requirements

Canonical fact count must still report exact fact count.

Relation fact count must match canonical count.

Access entry counts must remain per access path.

Storage diagnostics should not double count fact bytes if byte stats exist.

If new byte diagnostics are added, distinguish canonical bytes from access key bytes.

## 21. Passing Criteria

There is exactly one durable owner of full encoded fact bytes.

Access entries store only declared key bytes plus fact identity.

`fact_set` no longer requires all relation fields as key components.

Exact membership remains collision-safe.

All insert/delete/scan/reopen/snapshot tests pass.

Storage format version is bumped.

Previous storage format is rejected.

The global validation gate passes.

## 22. Failure Modes

Keeping full fact bytes in both canonical owner and fact-id lookup value is a failure.

Keeping full fact bytes in `fact_set` access key is a failure unless explicitly documented as temporary and accepted by reviewer.

Silent fact-ID collision aliasing is a failure.

Breaking prefix scans is a failure.

Breaking query image build is a failure.

Adding old-format readers is a failure.

Adding migration code is a failure.

## 23. Non-Goals

Do not compact query images in this PRD beyond decoupling from full `fact_set` keys.

Do not implement COLT.

Do not rewrite Free Join.

Do not add dictionary garbage collection.

Do not change public fact APIs.

Do not add compression.

## 24. Completion Notes

Update `ROSETTA_STONE.md` storage model.

Update any raw-layout tests to use new namespace definitions.

Document durable fact ID versus query-image fact ID if both exist.

Keep raw byte duplication tests permanent.

This PRD is the storage foundation for compact access images and lazy tries.
