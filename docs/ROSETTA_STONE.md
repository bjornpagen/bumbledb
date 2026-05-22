# Rosetta Stone

This document is the normative product and architecture contract for Bumbledb v3.

## Product Thesis

Bumbledb is an embedded, typed, schemaful, set-semantic relational database for highly normalized application data.

The target workload is BCNF/ledger-like data with many narrow relations and many joins. The engine is not a general-purpose SQL database, not an OLAP column store, not a schemaless graph database, and not a server.

Bumbledb competes on one battlefield: embedded, read-heavy, single-writer, multi-reader, highly relational, typed query IR workloads over LMDB.

## Current Query Contract

- The current executable query surface is typed query IR.
- The Datalog frontend was deleted intentionally.
- No query text parser is part of v3.
- Logica is the intended future frontend, but it is not implemented in v3.
- Future Logica lowering must target `query_ir`; it must not reintroduce Datalog as an intermediate layer.
- SQL is not supported.

## Core Commitments

- Language: Rust.
- Rust channel: nightly allowed, but not gratuitously.
- Storage backend: LMDB only.
- Server mode: forbidden.
- Network protocol: forbidden.
- Runtime DDL: forbidden.
- Migrations: forbidden.
- Schema changes require ETL into a new database.
- Schema mismatch on open is a hard failure.
- Storage format mismatch on open is a hard failure.
- Nulls: forbidden.
- Floating-point persistence: forbidden.
- Multiple writers: no, inherited from LMDB's single-writer model.
- Multiple readers: yes, inherited from LMDB MVCC.
- Async API: no.
- Unsafe durability flags: not in scope.

## Compatibility Policy

Bumbledb v3 is not compatible with v1 or v2 databases.

There is no migration path except ETL into a new database. There is no attempt to read old fingerprints, translate old descriptors, preserve old index names, or tolerate partial schema drift.

Backwards compatibility is explicitly out of scope while the storage and query architecture is still being made correct.

## Relation Semantics

- Every relation is a set of full tuples.
- Exact duplicate insert is an idempotent no-op.
- Same unique prefix with a different tuple is a uniqueness violation.
- Delete is exact tuple deletion.
- Delete of an absent tuple is non-exceptional.
- Projection output has set semantics.
- There is no `SELECT DISTINCT` concept because distinctness is the default.
- SQL-style bag semantics are intentionally rejected.

## Schema Model

Schemas are declared in Rust and compiled into the binary.

The schema descriptor has these relation-level facts:

```rust
pub struct RelationDescriptor {
    pub name: String,
    pub fields: Vec<FieldDescriptor>,
    pub constraints: Vec<ConstraintDescriptor>,
    pub indexes: Vec<IndexDescriptor>,
}
```

There is no primary key descriptor.

There is no generated ID descriptor.

There is no relation kind enum.

There are no implicit foreign keys from field types.

Every relation must declare exactly one covering unique constraint:

```rust
ConstraintDescriptor::Unique {
    name,
    fields,
    covering: true,
}
```

The covering unique constraint owns the canonical access path for the tuple. It is not a privileged logical primary key. It is the declared physical covering prefix.

Foreign keys are explicit and target named unique constraints:

```rust
ConstraintDescriptor::ForeignKey {
    name,
    fields,
    target_relation,
    target_constraint,
    on_delete: ForeignKeyAction::Restrict,
}
```

Foreign keys are generic exact-key constraints. They can target single-field or compound unique constraints, including enum fields and mixed serial-plus-enum keys. FK compatibility is positional exact `ValueType` equality.

## Serial Model

Nominal serial types are preserved, but `Id` and `Ref` field types are gone.

Serial is a nominal value type:

```rust
ValueType::Serial {
    type_name: "AccountId".to_owned(),
    owning_relation: "Account".to_owned(),
}
```

An `AccountId` is not an `InstrumentId`, even if both are encoded as 64-bit integers. Query building must reject unifying variables across different serial types.

A referencing column uses the target serial type. The fact that the column is a foreign key is expressed only by an explicit foreign-key constraint.

## Primitive Value Types

Supported persistent types:

- `Bool`
- `U64`
- `I64`
- `TimestampMicros`
- `Decimal { scale }`
- `Enum { name }`
- `String`
- `Bytes`
- `Serial { type_name, owning_relation }`

There is no `Code` type and no persistent UUID type. Open numeric domains use `U64`. Closed domains use one-byte `Enum` values. Enum codes must be in `0..=255`.

There are no nulls. Optional facts are represented by absent tuples in separate relations.

## Storage Model

LMDB is the only storage backend.

The physical storage model is full-covering access paths over encoded tuples:

```text
current access key = namespace || relation_id || access_id || encoded tuple permutation
```

Every access path contains every relation field exactly once. Leading fields provide the lookup prefix; unseen fields are appended in declaration order.

There is no current-row payload namespace. There is no primary-row store. Any access path key can decode the full tuple.

String and bytes values are interned. Hot tuple/index keys store intern IDs, not raw strings or raw byte arrays.

## Index Model

Generated access path kinds:

- `Covering`
- `Unique`
- `ForeignKey`
- `Range`
- `Equality`
- `Permutation`

`Covering` is generated from the one covering unique constraint. Non-covering unique constraints and foreign-key constraints still produce full-covering physical access paths.

The engine rejects schemas whose generated LMDB keys exceed the configured max key size. There is no fallback to row payload storage.

## Write Semantics

`insert` returns whether the tuple was inserted or already present.

`delete` returns whether the tuple was deleted or absent.

Bulk load counts only newly inserted tuples.

Foreign-key and uniqueness checks use covering access path prefix probes. There is no unique guard namespace.

History records full old/new tuple bytes, not primary-key bytes.

## Query Execution

Queries are built as typed IR with schema-aware validation.

The query runtime normalizes typed IR, plans access paths, executes direct/hash/LFTJ/factorized paths, and emits set-semantic results through encoded sinks.

Aggregates are set-domain operations. Count semantics must be expressed as explicit domain counts or distinct-value counts; SQL-style hidden witness multiplicity is not part of the contract.

Benchmark-shaped fast paths must be represented as structural plan candidates or executor nodes. Engine code must not depend on JOB-specific relation names.

Global `count` over empty input returns one row containing `0`. Grouped aggregates over empty input return zero rows. Null-like aggregate outputs are not introduced.

## Ownership Contract

- Public API owns user-facing `Row`, `Value`, and strings.
- `StorageSchema` owns compiled relation, field, constraint, and access layout metadata.
- Storage write paths convert public rows to field-order encoded tuples once.
- Query images own immutable encoded column/index bytes and are shared by `Arc`.
- Query execution borrows encoded bytes where possible and owns fixed-width `EncodedOwned` values only when needed for bindings, tries, hashes, or output sink keys.
- Decoding happens at API/output boundaries.

## Modeling Guidance

Use BCNF n-ary relations for natural domain facts.

Example ledger shape:

```text
Holder(id, name)
Account(id, holder, currency)
JournalEntry(id, source, created_at)
Posting(id, entry, account, instrument, amount, at)
AccountTag(account, tag)
```

Natural edge relations are allowed when they represent real domain facts:

```text
OrgParent(child, parent)
AccountTag(account, tag)
Permission(subject, object, permission)
```

Forbidden modeling:

```text
Posting(id, json_blob)
Posting(id, nullable_field)
GenericFact(entity, attribute, value)
```

## Historical Data

Historical logging is part of the storage direction, but as-of query execution is not a v2 requirement.

Event-sourced modeling is preferred for validity, status, ordering, hierarchy, soft delete, and derived aggregates. These should be represented as immutable event facts or derived query predicates, not nullable mutable columns.

## Future Work

- Logica frontend lowering into `query_ir`.
- Recursion.
- Stratified negation.
- As-of execution.
- Check constraints.
- More exact planner statistics where justified by benchmarks.
- Dictionary compaction or GC if needed.
