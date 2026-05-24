# Rosetta Stone

This document is the normative product and architecture contract for the Bumbledb v5 rebuild line.

## Product Thesis

Bumbledb is an embedded, typed, schemaful, set-semantic relational database for highly normalized application data.

The target workload is BCNF/ledger-like data with many narrow relations and many joins. Bumbledb is not SQL, not a server, not a document store, not a vector store, and not an OLAP engine.

Bumbledb competes on one battlefield: embedded, read-heavy, single-writer, multi-reader, typed relational query workloads over LMDB.

## Core Commitments

- Language: Rust.
- Storage backend: LMDB only.
- Query surface: unstable Rust typed query IR and builder API.
- Runtime DDL: forbidden.
- Server mode: forbidden.
- Network protocol: forbidden.
- SQL frontend: forbidden.
- Nulls: forbidden.
- Floating-point persistence: forbidden.
- Async API: forbidden.
- Multiple writers: no, inherited from LMDB.
- Multiple readers: yes, inherited from LMDB MVCC.

## Compatibility Policy

Bumbledb v5 is not compatible with v1, v2, v3, or v4 databases.

Storage format mismatch on open is a hard failure. Schema mismatch on open is a hard failure. There is no migration path except explicit ETL into a new database. There are no compatibility readers, no old layout translators, no compatibility aliases, and no in-place upgrades.

The Rust API is also not a frozen public interface. The typed query IR is an internal/product work surface exposed for current development and tests; external stability, field sealing, and long-term API compatibility are not goals in this phase. Malformed IR must still be rejected at execution boundaries.

## Relation Semantics

- Every relation is a set of full facts.
- Exact duplicate insert is an idempotent no-op.
- Delete is exact fact deletion.
- Delete of an absent fact is an idempotent no-op.
- There is no update operation.
- DB-generated IDs exist only through declared `Serial` fields.
- Projection output has set semantics.
- SQL-style multiset behavior is out of scope.
- There is no `SELECT DISTINCT` concept because distinctness is the default.

## Schema Model

Schemas are declared in Rust and compiled into the binary.

The schema descriptor has relation-level facts:

```rust
pub struct RelationDescriptor {
    pub name: String,
    pub fields: Vec<FieldDescriptor>,
    pub constraints: Vec<ConstraintDescriptor>,
}

pub struct FieldDescriptor {
    pub name: String,
    pub value_type: ValueType,
    pub generation: FieldGeneration,
}
```

There is no primary-key descriptor and no relation-kind enum. Generated IDs exist only as field-level `Serial` generation policy.

Canonical fact membership is implicit for every relation. It is not modeled as a primary key or as a required all-field unique constraint.

Unique constraints are named logical constraints:

```rust
ConstraintDescriptor::Unique { name, fields }
```

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

Serial values are database-generated monotonic `u64` sequence values for explicitly declared serial fields. They are nominal: an `AccountId` is not an `InstrumentId`, even if both encode as 64-bit integers.

```rust
ValueType::Serial {
    type_name: "AccountId".to_owned(),
    owning_relation: "Account".to_owned(),
}
```

Serial allocation is transactional under LMDB's single-writer model. Aborted writes do not advance the committed sequence. Deleted serial values are not reused. Explicit ETL-supplied serial values are allowed only when they preserve constraints, and successful explicit inserts advance the relation's sequence high-water mark as needed to avoid future generated collisions.

Query building rejects unifying variables across different serial types.

## Primitive Value Types

Supported persistent types:

- `Bool`
- `U64`
- `I64`
- `Enum { name }`
- `String`
- `Bytes`
- `Serial { type_name, owning_relation }`

Open numeric domains use `U64` or `I64`. Timestamp-like values are application-level `I64` conventions. Fixed-point money-like values are application-level scaled `I64` conventions. Closed domains use one-byte `Enum` values. String and bytes values are interned. There are no nulls; optional facts are represented by absent facts in separate relations.

## Storage Model

LMDB is the only storage backend.

The v5 storage target has canonical fact membership, content-derived fact handles, live row handles, per-field column entries, serial sequence metadata, constraint guards, optional physical accelerators, and statistics:

```text
canonical fact = T | relation_id | fact_bytes -> fact_handle
fact handle lookup = H | relation_id | fact_handle -> fact_bytes
live row = L | relation_id | fact_handle -> empty
column entry = C | relation_id | field_id | fact_handle -> encoded_field_bytes
serial sequence = Q | relation_id | field_id -> next_u64
unique guard = U | relation_id | constraint_name | unique_key_bytes -> fact_handle
reverse FK guard = R | target_relation_id | target_constraint | target_key_bytes | source_relation_id | source_constraint | source_fact_handle -> empty
optional accelerator = A | relation_id | accelerator_id | tuple_key | fact_handle -> empty
stats = S | relation_id | stat_name -> encoded_stat
```

The canonical fact namespace owns exact fact membership. Live rows and column entries are the source for snapshot-local base images and COLT. Unique constraints and reverse foreign-key delete checks use dedicated guard namespaces. Optional accelerators may speed access but must never be required for correctness.

Base images are built from current live rows and column entries under an LMDB read snapshot. Durable historical relation snapshots and history/audit records are not part of the v5 write path.

## Write Semantics

`insert` returns whether the fact was inserted or already present.

`delete` returns whether the fact was deleted or absent.

Bulk load is an ETL convenience that applies insert semantics in one write transaction and counts only newly inserted facts.

Successful logical writes advance the Bumbledb storage transaction ID. Duplicate inserts and absent deletes do not change logical storage state.

Failed writes leave no partial canonical fact, live row, column entry, serial sequence, constraint guard, dictionary, or cardinality state committed.

## Query Semantics

Queries are built as typed IR with schema-aware builder support and execution-boundary validation. The IR shape is intentionally unstable and may change freely while the engine is still being collapsed.

The logical solution of a query is a set of variable bindings. Projection returns the set of projected facts. Existential variables do not multiply projected output.

## Query Execution

The retained execution target is formal Free Join over GHT/COLT plus fact-native projection/storage paths. Legacy non-result-set query APIs and scalar caches were deleted because they encoded witness multiplicity or preserved mechanics outside the minimal set engine.

Projection materialization uses a result-set sink. Duplicate projected facts are rejected before final output decoding.

Base images, GHTs, and COLTs are internal snapshot-local execution structures. They are scoped to required fields/accesses and are not a public API contract.

## Public Output Contract

Query execution returns `QueryResultSet` directly:

```rust
pub struct QueryResultSet {
    pub columns: Vec<ResultColumn>,
    pub facts: Vec<ResultFact>,
}
```

`QueryResultSet` is duplicate-free and canonicalized. Result-set execution is the current query output contract. Future plan, explain, timing, or trace diagnostics must be separate observability surfaces, not wrappers required to access result facts.

## Benchmark Contract

Benchmarks must validate exact Bumbledb result values against SQLite before timing numbers matter.

SQLite projection references use `SELECT DISTINCT`. Benchmark comparisons must compare exact projected values, not just returned fact counts.

## Golden Examples

Golden examples are permanent non-regression fixtures for:

- Ledger.
- Sailors.
- Joinstress.
- TPC-H subset.
- IMDb/JOB subset.
- Lahman subset.
- LDBC subset.

They cover duplicate witnesses, exact projection sets, duplicate insert no-ops, absent delete no-ops, and constraint behavior.

## Validation Contract

Required validation includes:

- Full workspace formatting, check, clippy, and tests.
- Fuzz crate check.
- Insert/delete operation-sequence property tests.
- Query-vs-reference differential tests.
- Failpoint tests around dictionary, access, canonical fact, stats, and commit stages.
- Golden example tests.
- Exact benchmark correctness checks.

## Modeling Guidance

Use BCNF n-ary relations for natural domain facts.

Example ledger shape:

```text
Holder(id, name)
Account(id, holder, currency)
JournalEntry(id, source, created_at)
Posting(id, entry, account, instrument, amount, at)
PostingTag(posting, tag)
```

Natural edge relations are allowed when they represent real domain facts:

```text
OrgParent(child, parent)
Permission(subject, object, permission)
Knows(person1, person2)
Likes(person, post)
```

Forbidden modeling:

```text
Posting(id, json_blob)
Posting(id, nullable_field)
GenericFact(entity, attribute, value)
```

Historical, temporal, validity, status, ordering, hierarchy, soft delete, and derived summaries should be represented as immutable event facts or derived query predicates, not nullable mutable columns.
