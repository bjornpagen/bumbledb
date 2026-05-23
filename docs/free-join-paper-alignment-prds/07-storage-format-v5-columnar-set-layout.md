# PRD 07: Storage Format V5 Columnar Set Layout

## Purpose

Define and introduce the breaking v5 durable layout needed to support paper-compliant COLT over LMDB snapshots while preserving exact set membership, constraints, and Rosetta semantics.

## Dependencies

- PRD 00.
- PRD 01.

## Scope

- Storage format version constants.
- Schema fingerprint namespace/version label.
- New durable key namespaces.
- Storage schema/access descriptor split.
- No compatibility reader for older storage.

## Required Layout

Use a new storage format version and a new schema canonicalization label. The exact namespace bytes may change, but the durable concepts must exist:

| Concept | Purpose |
| --- | --- |
| Canonical membership `T` | Exact relation set membership by full encoded fact. |
| Fact handle lookup `H` | Map relation and content-derived fact handle to encoded full fact. |
| Live rows `L` | Snapshot-visible current fact handles per relation. |
| Durable columns `C` | Per relation, per field, per fact handle encoded field bytes. |
| Unique guards `U` | Named unique constraint keys to fact handle. |
| Reverse FK guards `R` | Restrict-delete checks by target key to source fact handle. |
| Optional accelerators `A` | Persisted physical tuple-key accelerators, never required for correctness. |
| Stats `S` | Fact counts, accelerator counts, and planner statistics. |

## Fact Handle Policy

- Use a content-derived fact handle, preferably the existing 16-byte BLAKE3 relation-plus-fact handle.
- Continue collision checks against full encoded facts.
- Do not expose fact handles as generated public IDs.
- Do not introduce any DB-side generated ID allocator except declared `Serial` field sequences.

## Required Type Split

- Separate logical constraints from physical accelerators.
- `AccessLayout` must no longer be the one type for constraints, query correctness, and physical indexes.
- Unique and FK guard descriptors are constraint infrastructure.
- Accelerator descriptors are optional physical tuning metadata.
- Runtime GHT/COLT schemas are plan-derived and not durable schema layouts.

## Technical Direction

- Bump `STORAGE_FORMAT_VERSION`.
- Change schema canonical bytes label from the current v4 label.
- Add namespace constants and key builders in storage modules.
- Keep old format tests only if they assert mismatch failure. Do not add compatibility readers.
- Add per-serial-field sequence metadata for generated `Serial` values.
- Document ETL-only migration.
- Keep string/bytes interning behavior unless PRD implementation discovers a concrete reason to change it.

## Non-Goals

- Do not implement all write/read behavior here if PRD 08 will do it.
- Do not implement COLT here.
- Do not use this PRD to add a non-LMDB storage backend.

## Acceptance Criteria

- New storage format version is defined.
- Opening an existing v4 database with v5 code fails hard with storage format mismatch.
- Schema fingerprint changes when the v5 canonical label changes.
- Key-builder tests cover every new namespace.
- Existing logical schema descriptors are split or clearly staged for split so query correctness no longer depends on persisted access entries.
- Documentation states there is no compatibility reader and migration is ETL into a new database.

## Required Tests

- New database writes v5 format marker.
- Old/missing mismatched format fails.
- Schema fingerprint v5 label differs from v4 label.
- Key namespace ordering tests for `T/H/L/C/U/R/A/S`.
- Fact handle collision check remains possible.
- Serial sequence metadata exists and is distinct from internal fact handles.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-core schema --all-features
cargo test -p bumbledb-lmdb storage_format --all-features
cargo check --workspace --all-targets --all-features
```
