# Bumbledb v2 Cleanup Roadmap

## Purpose

This directory is the implementation contract for the next architectural cleanup pass.

The benchmark optimization loop proved that the engine can win on highly normalized join-heavy workloads. It also left the codebase with too many special cases, compatibility bridges, and duplicated concepts. This pass intentionally trades short-term compatibility for a simpler, stricter architecture.

Backwards compatibility is out of scope. Existing databases, fingerprints, Datalog strings, old schema descriptors, and old runtime value variants may be broken or deleted without migration support.

## Product Direction

Bumbledb v2 is an embedded, typed, schemaful, set-semantic relational engine over LMDB.

The new model is:

```text
Relation = set of full tuples
Access path = full covering tuple permutation
Unique constraint = named unique prefix over an access path
Foreign key = named prefix existence check against a target unique constraint
Identity = nominal value type, not implicit primary-key/ref machinery
Query frontend = typed query IR only, until Logica replaces it later
```

## Non-Negotiable Decisions

- Delete the Datalog language frontend now.
- Do not replace Datalog with Logica in this pass.
- Keep `query_ir` as the only query representation.
- Add a schema-aware query IR builder for tests and benchmarks.
- Remove `PrimaryKeyDescriptor` entirely.
- Remove `GeneratedIdDescriptor` entirely.
- Remove `RelationKind` entirely.
- Remove `ValueType::Id` entirely.
- Remove `ValueType::Ref` entirely.
- Remove `ValueType::Code` entirely.
- Remove `Value::Id`, `Value::Ref`, and `Value::Code` entirely.
- Preserve typed identity safety through a new identity value type.
- Make every foreign key explicit and named.
- Make every foreign key target a named unique constraint.
- Make every relation set-semantic.
- Make exact duplicate inserts idempotent no-ops.
- Make same-unique-prefix/different-tuple inserts violations.
- Bump schema canonical serialization to v2.
- Bump storage format if storage keys or persisted meaning change.
- Do not add compatibility shims for old schemas or old databases.

## Current Code Anchors

The next agent should use these as current-state anchors before editing. Line numbers are from the pre-v2 cleanup codebase and will drift after edits.

- Datalog module export: `crates/bumbledb-core/src/lib.rs:6`
- Datalog parser/typechecker: `crates/bumbledb-core/src/datalog.rs:1-1568`
- Query IR: `crates/bumbledb-core/src/query_ir.rs:1-236`
- Schema errors and validation: `crates/bumbledb-core/src/schema.rs:11-160`, `323-747`
- Schema canonical bytes: `crates/bumbledb-core/src/schema.rs:292-305`, `1080-1108`, `1244-1279`, `1389-1417`, `1495-1509`
- Old relation descriptor: `crates/bumbledb-core/src/schema.rs:863-936`
- Old relation kind: `crates/bumbledb-core/src/schema.rs:1111-1122`
- Old value type variants: `crates/bumbledb-core/src/schema.rs:1165-1195`
- Old primary/generated descriptors: `crates/bumbledb-core/src/schema.rs:1282-1320`
- Old constraints: `crates/bumbledb-core/src/schema.rs:1322-1417`
- Old index generation: `crates/bumbledb-core/src/schema.rs:943-1068`
- Storage row/value API: `crates/bumbledb-lmdb/src/storage.rs:35-146`
- Storage current row namespace: `crates/bumbledb-lmdb/src/storage.rs:18-20`
- Storage write APIs: `crates/bumbledb-lmdb/src/storage.rs:428-554`
- Storage primary-key helpers: `crates/bumbledb-lmdb/src/storage.rs:762-779`, `1558-1577`, `1854-1866`
- Storage FK/unique checks: `crates/bumbledb-lmdb/src/storage.rs:796-855`, `944-976`, `1968-1982`
- Query type/value bridges: `crates/bumbledb-lmdb/src/query.rs:2410-2445`, `8427-8477`, `9503-9517`
- Query output plan and set flag: `crates/bumbledb-lmdb/src/query.rs:3152-3208`, `crates/bumbledb-lmdb/src/free_join.rs:128-135`
- Query sinks: `crates/bumbledb-lmdb/src/query.rs:8730-9500`
- Query image current-index fallback: `crates/bumbledb-lmdb/src/query_image.rs:1458-1486`
- Storage schema layouts: `crates/bumbledb-lmdb/src/storage_schema.rs:89-170`
- Benchmark Datalog use: `crates/bumbledb-bench/src/main.rs:554-560`, `832-837`, `1305-1308`
- JOB Datalog query catalog: `crates/bumbledb-bench/src/open.rs:932-1209`
- Test support schemas: `crates/bumbledb-test-support/src/schemas.rs:1-132`
- Test support Datalog workload strings: `crates/bumbledb-test-support/src/workloads.rs:1-28`
- Datalog fuzz target: `fuzz/Cargo.toml:45-50`, `fuzz/fuzz_targets/fuzz_datalog_parser.rs`
- Rosetta Stone old product contract: `docs/ROSETTA_STONE.md:1-220` and later Datalog/query sections.

## Target Architecture Summary

### Core Types

```rust
pub enum ValueType {
    Bool,
    U64,
    I64,
    TimestampMicros,
    Decimal { scale: u32 },
    Uuid,
    Enum { name: String },
    String,
    Bytes,
    Identity {
        type_name: String,
        owning_relation: String,
        allocation: IdentityAllocation,
    },
}

pub enum IdentityAllocation {
    Serial,
    Uuid,
    Application,
}

pub enum Value {
    Bool(bool),
    U64(u64),
    I64(i64),
    Identity(IdentityValue),
    Timestamp(TimestampMicros),
    Decimal(DecimalRaw),
    Uuid(UuidBytes),
    Enum(u64),
    String(String),
    Bytes(Vec<u8>),
}

pub enum IdentityValue {
    Serial(u64),
    Uuid(UuidBytes),
    Application(u64),
}
```

### Schema Descriptor

```rust
pub struct RelationDescriptor {
    pub name: String,
    pub fields: Vec<FieldDescriptor>,
    pub constraints: Vec<ConstraintDescriptor>,
    pub indexes: Vec<IndexDescriptor>,
}

pub enum ConstraintDescriptor {
    Unique {
        name: String,
        fields: Vec<String>,
        covering: bool,
    },
    ForeignKey {
        name: String,
        fields: Vec<String>,
        target_relation: String,
        target_constraint: String,
        on_delete: ForeignKeyAction,
        on_update: ForeignKeyAction,
    },
}

pub enum IndexKind {
    Covering,
    Unique,
    ForeignKey,
    Range,
    Equality,
    Permutation,
}
```

### Storage Descriptor

```rust
pub(crate) struct EncodedTuple {
    relation: RelationId,
    bytes: Vec<u8>,
}

pub(crate) struct FieldLayout {
    pub id: FieldId,
    pub name: String,
    pub value_type: ValueType,
    pub offset: usize,
    pub width: usize,
}

pub(crate) struct AccessLayout {
    pub id: AccessId,
    pub name: String,
    pub kind: IndexKind,
    pub leading_fields: Vec<FieldId>,
    pub components: Vec<FieldId>,
    pub key_prefix: Vec<u8>,
    pub encoded_len: usize,
}
```

Every `AccessLayout::components` contains every field exactly once.

## Ordered PRDs

Implement these in order. Do not skip ahead unless a PRD explicitly says it can run in parallel.

1. `01-query-ir-builder.md`
2. `02-remove-datalog-callers.md`
3. `03-delete-datalog-frontend.md`
4. `04-schema-v2-value-model.md`
5. `05-schema-v2-descriptors-and-constraints.md`
6. `06-covering-access-layouts.md`
7. `07-storage-encoded-tuples.md`
8. `08-set-write-semantics.md`
9. `09-query-v2-type-migration.md`
10. `10-query-architecture-unification.md`
11. `11-tests-benchmarks-fuzz.md`
12. `12-docs-and-final-gates.md`

Mandatory cross-cutting audit:

13. `13-ownership-and-allocation-contract.md`

PRD 13 is not a separate feature phase. It is an audit contract that must be applied throughout PRDs 04 through 12 and verified before the final gates.

## Global Passing Requirements

Every PRD must maintain these unless explicitly stated otherwise:

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`

If an intermediate PRD intentionally breaks the whole workspace, it must say so and define a smaller required gate. The final PRD must pass all global gates.

## Repository-Wide Final Rejection List

The final PRD must make this command return no real code references, except historical docs inside `docs/job-trace-analysis` or this PRD directory:

```text
datalog
Datalog
PrimaryKeyDescriptor
GeneratedIdDescriptor
RelationKind
ValueType::Id
ValueType::Ref
ValueType::Code
Value::Id
Value::Ref
Value::Code
with_ref_foreign_keys
IndexKind::Primary
IndexKind::Ref
ComponentRole::Identity
KeyValues
NS_CURRENT_ROW
NS_UNIQUE_GUARD
```

## Compatibility Policy

No backwards compatibility work is allowed in this pass.

Do not:

- Read v1 schema fingerprints.
- Translate v1 relation descriptors.
- Keep old Datalog parsing behind a feature flag.
- Keep old `Id`/`Ref`/`Code` variants as deprecated aliases.
- Preserve `primary` index names for old callers.
- Preserve old LMDB current-row payload layout.
- Add migrations.
- Add partial-open or best-effort upgrade behavior.

Do:

- Bump canonical schema version to `bumbledb.schema.v2`.
- Bump storage format version when persisted key/value interpretation changes.
- Fail hard on mismatch.
- Treat ETL into a new database as the only migration path.
