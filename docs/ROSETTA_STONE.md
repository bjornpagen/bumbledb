**Rosetta Stone**
This is the normative design for the database unless we explicitly revise it later.

**Product Thesis**
- This is an embedded, typed, schemaful, Datalog-only database for highly normalized application data.
- The target workload is BCNF/ledger-like CRUD-app data with many narrow relations and many joins.
- The engine is not a general-purpose SQL database.
- The engine is not an OLAP column store.
- The engine is not a schemaless graph database.
- The engine is not trying to replace Postgres or SQLite universally.
- The engine is trying to beat them on one battlefield: embedded, read-heavy, single-writer, highly relational, join-heavy, typed Datalog workloads.
- LMDB is the only storage backend.
- There is no storage backend abstraction in v0.
- There is no server mode.
- There is no network protocol.
- There is no SQL.
- There is no migration system.
- Schema changes require full ETL into a new database.

**Core Decisions**
- Language: Rust.
- Rust channel: nightly allowed.
- Nightly usage: allowed but not gratuitous.
- Backend: LMDB only.
- Query language: Datalog only.
- Logical schema model: BCNF typed n-ary relations.
- Physical storage model: LMDB durable segments plus covering sorted indexes.
- User-facing schema: fully typed, fully schemaful, hardcoded at compile time.
- Runtime DDL: forbidden.
- Migrations: forbidden.
- Schema mismatch on open: hard failure.
- Schema evolution: full ETL into a new database.
- Nulls: forbidden.
- Floating-point persistence: forbidden in v0.
- Current-state queries: first-class.
- Historical logging: always enabled.
- As-of query execution: later, not v0.
- Recursion: later, not v0.
- Stratified negation: later, not v0.
- Aggregation: v0.
- Multiple writers: no.
- Multiple readers: yes, inherited from LMDB.
- Async API: no.
- Safe durability: default and only v0 mode.
- Unsafe performance flags: not v0.

**Normal Form Decision**
- BCNF is the logical default.
- 6NF is not globally enforced.
- Graph-normal-form relations are allowed when they are natural domain facts.
- Entity/event-shaped relations stay n-ary.
- Edge-shaped relations may be binary or ternary.
- Optional facts are represented as separate relations, not nullable columns.
- 6NF’s future migration advantage is deliberately rejected because migrations are out of scope.
- The database assumes schema changes are rare enough and explicit enough that ETL is acceptable.
- We do not create unnecessary joins just because the join engine is good.

**Examples Of Intended Modeling**
```text
Posting(id, entry, account, instrument, amount, at)
Account(id, holder, currency)
JournalEntry(id, source, created_at)
Instrument(id, symbol)
Holder(id, name)
```

**Examples Of Natural Edge Relations**
```text
OrgParent(child, parent)
AccountTag(account, tag)
Permission(subject, object, permission)
AccountAlias(account, alias)
```

**Examples Of Forbidden Modeling**
```text
Posting(id, entry, account, instrument, amount, at_nullable)
Posting(id, json_blob)
GenericFact(entity, attribute, value)
```

**Schema System**
- Schemas are declared in Rust.
- Schemas are compiled into the binary.
- Schemas generate relation descriptors.
- Schemas generate column descriptors.
- Schemas generate typed ID wrappers.
- Schemas generate encoders.
- Schemas generate decoders.
- Schemas generate index descriptors.
- Schemas generate constraint metadata.
- Schemas generate query typechecking metadata.
- Schemas generate a canonical schema fingerprint.
- The database stores the schema fingerprint at creation.
- Opening a database compares the stored fingerprint with the compiled fingerprint.
- Fingerprint mismatch returns `SchemaMismatch`.
- There is no attempt to upgrade.
- There is no attempt to downgrade.
- There is no attempt to partially tolerate schema drift.

**Schema Declaration Shape**
```rust
db_schema! {
    database LedgerDb;

    relation Account {
        id: AccountId primary auto,
        holder: HolderId ref Holder,
        currency: Currency,
    }

    relation Posting {
        id: PostingId primary auto,
        entry: JournalEntryId ref JournalEntry,
        account: AccountId ref Account,
        instrument: InstrumentId ref Instrument,
        amount: Money,
        at: Timestamp range,
    }

    relation OrgParent {
        child: OrgId ref Org,
        parent: OrgId ref Org,
        primary(child, parent),
    }
}
```

**Relation Kinds**
- Entity relations have a primary key.
- Event relations have a primary key.
- Edge relations may use composite identity.
- Pure set relations may use composite identity.
- Primary keys are immutable.
- Composite primary keys are immutable.
- All relations are sets.
- Duplicate tuples are forbidden.
- Row identity is explicit.
- Hidden row IDs are forbidden for user-visible relations.

**Null Semantics**
- Null does not exist.
- Missing data means missing relation tuple.
- Optional one-to-one data uses a separate relation.
- Optional one-to-many data uses a separate relation.
- Datalog does not need SQL’s three-valued logic.
- Every stored value has a concrete type.
- Every query variable has a concrete inferred type.
- Every comparison is deterministic.

**Type System**
- Physical equality does not imply logical equality.
- `AccountId` and `InstrumentId` may both be stored as `u64`.
- `AccountId` and `InstrumentId` are not unifiable.
- Typed IDs are newtypes.
- Ref columns point to specific target relations.
- Ref columns enforce foreign-key existence.
- Typechecking happens before planning.
- Query variables unify only when logical types match.
- Numeric implicit casts are forbidden in v0.
- Decimal scales must match exactly in v0.
- Input parameters must match inferred types exactly.
- Constants must match inferred types exactly.

**Primitive Value Types**
- `bool` is supported.
- `u64` is supported.
- `i64` is supported.
- Typed `Id<T>` is supported.
- Typed refs are supported.
- `Timestamp` is supported as UTC microseconds stored in signed 64-bit form.
- `Date` may be supported as days since epoch.
- `Decimal<SCALE>` is supported as fixed-scale signed 128-bit integer.
- `Money<SCALE>` is a first-class decimal-like domain type.
- `Uuid` is supported as 16 bytes.
- `Symbol` is supported as interned or generated numeric value.
- `String` is supported through interning.
- `Bytes` is supported through interning.
- Floats are forbidden in persistent schema v0.
- JSON is forbidden as a first-class indexed type in v0.
- Arbitrary blobs are allowed only as interned opaque values.

**String Semantics**
- String equality is supported.
- String inequality by interned ID is meaningless and forbidden at the query layer.
- String lexical ordering is not supported in v0.
- String prefix search is not supported in v0.
- Full-text search is not supported in v0.
- If string range or prefix search is added later, it gets explicit ordered text indexes.
- Interned strings are never deleted in v0.
- Interned value IDs are never reused.

**Decimal Semantics**
- Decimal is fixed scale.
- Decimal storage is signed `i128`.
- Decimal comparison is exact.
- Decimal sum is exact until overflow.
- Decimal overflow returns an error.
- Decimal scale mismatch is a type error.
- Money should use decimal, never float.
- Aggregating money uses decimal sum.

**Time Semantics**
- Persistent timestamps are UTC.
- Time zones are not stored in timestamp values.
- Time-zone interpretation belongs to the application.
- Timestamp ordering is numeric chronological ordering.
- Timestamp precision is microseconds in v0.
- Leap-second semantics are not modeled in v0.

**LMDB Commitments**
- LMDB is used directly through a Rust binding or thin wrapper.
- LMDB’s sorted B+tree key ordering is the physical foundation.
- LMDB’s MVCC is the reader concurrency model.
- LMDB’s single-writer rule is accepted.
- LMDB values returned from cursors are transaction-scoped.
- Rust lifetimes must encode transaction-scoped access.
- LMDB key-size limit is treated as a hard design constraint.
- Default LMDB key comparison is used.
- Custom key comparators are avoided.
- `MDB_INTEGERKEY` is avoided.
- Explicit big-endian sortable encodings are used instead.
- `MDB_DUPSORT` is avoided for core indexes in v0.
- Composite keys are preferred over duplicate-sorted values.
- Named DBI count is fixed and tiny.
- There is no one-DBI-per-index design.
- There is no one-DBI-per-relation design.

**LMDB Environment**
- 64-bit platforms only.
- 32-bit platforms are unsupported.
- Public open API takes only a path.
- LMDB mapsize is hidden internally.
- Initial mapsize is large.
- Mapsize grows automatically on `MDB_MAP_FULL` when safe.
- `MDB_MAP_RESIZED` is handled internally.
- Maximum readers are set internally.
- Recommended internal max readers: 1024.
- `MDB_NOTLS` is used internally if the Rust binding allows it cleanly.
- Transactions are still exposed as same-thread scoped objects.
- Reader slots are cleaned with periodic reader checks.
- Long readers are documented as harmful.
- Closure-scoped read APIs are preferred.

**LMDB Durability Flags**
- `MDB_NOSYNC` is not used in v0.
- `MDB_NOMETASYNC` is not used in v0.
- `MDB_WRITEMAP` is not used in v0.
- `MDB_MAPASYNC` is not used in v0.
- Unsafe durability modes are out of scope for v0.
- Correctness beats insert microbenchmarks.
- Bulk loading can be optimized without unsafe durability flags.

**LMDB DBI Layout**
- `_meta` stores metadata.
- `_index` stores relation indexes, current tuples, history tuples, tx log, and stats.
- `_dict` stores dictionary forward and reverse entries.
- DBI count is fixed.
- `mdb_env_set_maxdbs` is set internally before opening.
- DBI names are internal and stable.
- Users never configure DBIs.

**Metadata**
- Metadata stores storage format version.
- Metadata stores schema fingerprint.
- Metadata stores database creation timestamp.
- Metadata stores engine compatibility version.
- Metadata stores next transaction ID.
- Metadata stores next generated ID counters.
- Metadata may store a human-readable schema manifest for debugging.
- Metadata is not a mutable catalog.
- Metadata does not permit DDL.

**On-Disk Key Encoding**
- All hot keys are fixed-width or bounded-width.
- No arbitrary-length string appears in a hot key.
- No arbitrary-length blob appears in a hot key.
- Numeric values are encoded so lexical byte order equals logical order.
- `u64` is encoded big-endian.
- Ref IDs are encoded big-endian.
- `i64` is sign-bit-flipped then encoded big-endian.
- `Timestamp` is encoded as sign-bit-flipped `i64`.
- `Decimal<i128>` is sign-bit-flipped then encoded big-endian.
- `bool` is encoded as one byte.
- `Uuid` is encoded as canonical 16 bytes.
- Interned strings are encoded as big-endian `u64`.
- Interned bytes are encoded as big-endian `u64`.
- Type tags are not included in relation index keys.
- Relation schemas already define component types.
- Type tags are used only in generic dictionary or metadata spaces.
- Every generated physical key layout is compile-time checked against LMDB max key size.
- If an index key could exceed LMDB’s max key size, the schema is rejected.

**Index Key Namespaces**
```text
CURRENT_TUPLE | relation_id | index_id | encoded components...
HISTORY_TUPLE | relation_id | index_id | encoded components... | tx_id | op
UNIQUE_GUARD  | relation_id | constraint_id | encoded unique value...
TX_LOG        | tx_id | sequence
STATS         | stats_kind | relation_id | column_id
```

**Current Tuple Indexes**
- Current indexes represent the present database state.
- Normal queries read current indexes.
- Current indexes contain asserted live tuples only.
- Deletes remove current index entries.
- Replaces remove old current entries and insert new current entries.
- Current indexes are optimized for query execution.
- Current indexes are not reconstructed from history during normal reads.

**History Storage**
- Every write appends transaction history.
- History records are enough to audit changes.
- History records are enough to build as-of query support later.
- History does not slow normal current-state queries.
- History indexes are physically separate from current indexes.
- Transaction IDs are monotonic `u64`.
- Transaction IDs are never reused.
- Transaction timestamp is recorded.
- Delete history records distinguish retraction from assertion.
- Full bitemporal semantics are not v0.
- As-of query syntax is reserved for later.

**Dictionary Storage**
- Strings are dictionary-encoded.
- Bytes are dictionary-encoded.
- Large decimals may be dictionary-encoded only if not using fixed-scale decimal.
- Dictionary IDs are monotonic `u64`.
- Dictionary IDs are scoped by value kind.
- Dictionary reverse mapping is ID to raw bytes.
- Dictionary forward mapping uses a strong hash of raw bytes.
- Forward lookup verifies equality against reverse value.
- Hash collisions are handled or produce explicit `HashCollision`.
- Dictionary entries are append-only.
- Dictionary entries are never garbage-collected in v0.
- Dictionary interning happens inside write transactions.
- Read queries compare interned IDs.

**Relation Indexing**
- Every relation has a primary covering index.
- Every ref column gets a leading covering index.
- Every unique column gets a leading covering index.
- Every scalar field marked `range` gets a leading covering index.
- Scalar fields are not all indexed by default.
- Index annotations are schema, not runtime configuration.
- Covering indexes store enough components to produce the full tuple.
- Covering indexes avoid row fetch during query execution.
- Covering indexes increase write amplification.
- Read performance is prioritized over raw insert speed.
- Relation arity is expected to be small.
- Narrow BCNF relations make covering indexes practical.

**Example Physical Indexes**
```text
Posting primary:
  id | entry | account | instrument | amount | at

Posting by_account:
  account | id | entry | instrument | amount | at

Posting by_entry:
  entry | id | account | instrument | amount | at

Posting by_instrument:
  instrument | id | entry | account | amount | at

Posting by_at:
  at | id | entry | account | instrument | amount
```

**Uniqueness**
- Primary key uniqueness is enforced.
- Composite primary key uniqueness is enforced.
- Declared unique constraints are enforced.
- Unique constraints use explicit unique guard keys.
- Unique violation aborts the write transaction.
- Unique checks happen before current index mutation is committed.
- Unique checks include staged writes in the same transaction.
- Unique constraints are immediate, not deferred.

**Foreign Keys**
- Ref fields enforce target existence.
- Foreign-key checks happen inside the write transaction.
- References to rows inserted earlier in the same write transaction are allowed.
- References to preallocated IDs staged in the same transaction are allowed.
- Dangling refs are forbidden.
- Deleting a referenced row is forbidden by default.
- Cascading delete is not v0.
- Deferred foreign-key checking is not v0.
- Foreign keys are typechecked at compile time and enforced at runtime.

**ID Generation**
- Entity/event relations use typed `u64` IDs by default.
- IDs are generated by the engine.
- ID counters are persisted in metadata.
- IDs are monotonic per ID type or relation.
- IDs are never reused.
- Applications can allocate IDs before inserting rows.
- Preallocated IDs can be used to create cyclic references within one transaction if constraints allow it.
- Natural keys are represented as unique constraints, not primary storage identity in v0.
- Composite set relations do not need generated IDs.

**Write API**
```rust
db.write(|tx| {
    let account = tx.alloc::<AccountId>()?;
    tx.insert(Account { id: account, holder, currency })?;
    tx.insert(Posting { id, entry, account, instrument, amount, at })?;
    Ok(())
})?;
```

**Write Operations**
- `insert` inserts a new row.
- `replace` replaces an existing primary-key row.
- `delete` deletes an existing primary-key row.
- `insert_tuple` inserts a composite set tuple.
- `delete_tuple` deletes a composite set tuple.
- `upsert` is not v0.
- `merge` is not v0.
- Partial update is not v0.
- Replace is explicit and whole-row.
- Write operations are batched in one LMDB write transaction.
- The write closure commits on success.
- The write closure aborts on error.
- The write closure aborts on panic.

**Delete Semantics**
- Deletes remove current index entries.
- Deletes append history records.
- Deletes fail if foreign-key restrictions would be violated.
- Deletes of nonexistent rows return `NotFound`.
- Deletes of nonexistent set tuples return `NotFound`.
- Cascading delete is not supported.
- Soft delete is modeled by the application as a relation or field.

**Replace Semantics**
- Replace requires the primary key to exist.
- Primary key cannot change.
- Replace validates all constraints.
- Replace removes old current index entries.
- Replace inserts new current index entries.
- Replace appends history records.
- Replace fails atomically if any constraint fails.

**Bulk Loading**
- Bulk ETL is a first-class path.
- Bulk loader may sort entries per physical index.
- Bulk loader may use LMDB append modes when keys are sorted.
- Bulk loading is allowed to build indexes in phases.
- Bulk loading is the official migration mechanism.
- Bulk loading can skip some per-row overhead while preserving constraints.
- Bulk loading must still produce a valid schema fingerprint.
- Bulk loading must still produce valid current indexes.
- Bulk loading must still produce valid metadata.

**Read API**
```rust
db.read(|snap| {
    let rows = snap.query(query, inputs)?;
    Ok(rows)
})?;
```

**Snapshot Semantics**
- A read closure opens one LMDB read transaction.
- A read closure sees a consistent snapshot.
- Concurrent writes do not affect an active snapshot.
- Active snapshots do not block writers.
- Long snapshots can prevent LMDB page reclamation.
- The API nudges users toward short snapshots.
- Advanced explicit snapshots may be added later.
- Query results are owned by default.
- Internal execution may borrow LMDB memory.
- Borrowed public result APIs are not v0.

**Threading**
- Database handle may be shared if the LMDB wrapper supports safe sharing.
- Transactions are scoped.
- Transactions are same-thread in the simple API.
- Cursors are same-thread in the simple API.
- Query execution happens synchronously.
- Async applications should use their own blocking thread strategy.
- No async runtime dependency is introduced.
- There is no background writer thread in v0.
- There is no connection pool in v0.

**Query Language**
- Query language is Datalog.
- Canonical syntax is named-field relation atoms.
- Positional syntax is not v0.
- SQL syntax is never supported.
- Variables start with `?`.
- Inputs start with `$`.
- Wildcard is `_`.
- Relations use schema names.
- Fields use schema field names.
- Constants are typed.
- Query parser produces typed logical IR.
- Query typechecker rejects ambiguous or invalid queries.

**Example Query**
```text
find ?posting ?amount ?instrument
where
  Posting(id: ?posting, account: ?account, amount: ?amount, instrument: ?instrument, at: ?t)
  Account(id: ?account, holder: $holder)
  ?t >= $start
  ?t < $end
```

**Relation Atom Semantics**
- A relation atom constrains one relation.
- Named fields may be partially specified.
- Unspecified fields are existential wildcards.
- Specified variables bind or join.
- Specified constants filter.
- Specified inputs filter.
- Repeating the same variable inside an atom enforces equality.
- Relation atoms range over current tuples by default.
- Historical relation atoms require future explicit syntax.

**Datalog V0**
- Positive conjunctive queries are supported.
- Input parameters are supported.
- Equality joins are supported.
- Comparison predicates are supported.
- Range predicates are supported.
- Projection is supported.
- Aggregation is supported.
- Rules are not v0.
- Recursion is not v0.
- Negation is not v0.
- Disjunction is not v0.
- User-defined functions are not v0.
- Arithmetic expressions are minimal in v0.

**Aggregation V0**
- `count` is supported.
- `sum` is supported.
- `min` is supported.
- `max` is supported.
- Grouping is determined by non-aggregate projected variables.
- Count returns `u64`.
- Sum over decimal returns decimal.
- Sum over integer returns integer with overflow checking.
- Overflow returns an error.
- Aggregation over empty groups follows explicit semantics.
- Recursive aggregation is not v0.
- Distinct aggregation is not v0 unless trivial from set semantics.

**Future Datalog**
- Rules will be supported after v0.
- Recursion will use semi-naive evaluation.
- Recursive evaluation will deduplicate derived tuples.
- Recursive evaluation will terminate on fixpoint.
- Stratified negation will be supported after recursion foundation exists.
- Negation will require variables to be bound by positive clauses.
- As-of queries will be added as explicit query context.
- User-defined pure functions may be added after deterministic function semantics are defined.

**Typechecking**
- Relation names are resolved against the compiled schema.
- Field names are resolved against relation descriptors.
- Variable types are inferred from field positions.
- Variable type conflicts are hard errors.
- Input types are inferred or declared.
- Input binding mismatch is a hard error.
- Projection variables must be bound.
- Aggregation variables must be bound.
- Comparison operands must have compatible exact types.
- Ref IDs from different relations are incompatible.
- Wildcards do not bind.
- Unused variables are allowed only inside atoms if harmless.
- Unsafe future negation is rejected.

**Logical IR**
```text
Query
RelationAtom
Comparison
Input
Variable
Constant
Projection
Aggregate
```

Typed parser/typechecker IR is not consumed directly by the physical planner or executor. It is normalized first into an executor-friendly IR that resolves dense relation, field, variable, input, atom, and predicate IDs.

**Normalized Query IR**
- `NormalizedQuery` contains normalized variables, inputs, atoms, predicates, output plan, and find-order terms.
- Relation names are resolved to `RelationId` and field names to `FieldId`.
- Literals are encoded before physical planning.
- Runtime inputs are encoded once into `EncodedInputs` before execution.
- Repeated variables inside one atom are represented in normalized fields and enforced by encoded equality during atom image construction.
- Comparison predicates record the earliest variable-order depth where they can run.
- The normalized IR is the boundary for future monomorphic/generated execution.

**Physical Planning**
- Planner works over normalized query IR.
- Planner enumerates access paths per relation atom.
- Access paths come from generated schema index descriptors.
- Planner chooses indexes based on bound prefixes.
- Planner chooses variable order.
- Planner pushes filters as early as possible.
- Planner chooses a Free Join physical plan over typed relation atoms.
- Planner chooses sorted leapfrog, hash probe, hybrid, vector, existence, and aggregate sink node implementations.
- Planner can mix node strategies inside one Free Join plan.
- Planner avoids materializing large intermediates.
- Planner prefers covering indexes.
- Planner uses stats.
- Planner produces explain output.

**Variable Ordering Heuristics**
- Input-bound variables go early.
- Constant-bound variables go early.
- Primary-key variables go early.
- Unique variables go early.
- High-degree join variables go early.
- Low-fanout ref variables go early.
- Equality constraints beat range constraints.
- Range-constrained variables come after equality-constrained variables.
- Projection-only variables go late.
- Aggregate-only variables go late.
- Variables needed by filters are bound before the filter runs.
- Variables needed by future negation must be bound before negation.

**Access Path Choice**
- Indexes are viewed as ordered tries.
- Leading index fields define cheap prefixes.
- Bound constants extend prefix.
- Bound variables extend prefix.
- Unbound leading field can produce a sorted stream.
- Covering index components can satisfy projection.
- Primary index is used for direct primary-key lookup.
- Ref indexes are used for joins.
- Range indexes are used for ordered scalar filtering.
- Unique guard keys are not query access paths.
- History indexes are not used for current queries.

**Execution Engine**
- Execution is QueryImage-first.
- QueryImage is a snapshot-local immutable image keyed by `(schema_fingerprint, tx_id)`.
- QueryImage is built from durable encoded column/index segments when segment metadata is visible.
- Sorted trie and hash trie indexes are in-memory structures over QueryImage relation images.
- Free Join is the single physical plan IR.
- Join execution compares encoded scalar words when possible.
- Decoding is lazy and never required for ordinary encoded comparisons.
- Output decoding happens late.
- Projection and aggregation are tuple sinks inside the execution pipeline.
- Intermediate full binding materialization is avoided where algebraically valid.
- LMDB cursors are not opened inside query variable recursion.
- Large temporary spill to LMDB is not v0.

**Join Algorithms**
- Sorted leapfrog/trie-style multiway join is used for cyclic and broad joins.
- Hash probe nodes are used for lookup-heavy selective plans.
- Hybrid nodes can combine sorted and hash access inside the same Free Join plan.
- Worst-case-optimal join is a core capability, not the only path.
- Binary join trees are not the primary execution model.
- Planner chooses node implementations based on access paths and stats.

**Trie Iterator Abstraction**
```text
open(level)
up(level)
next()
seek(encoded_value)
current_key()
current_range()
count()
```

**Explain Plans**
- Explain is required from day one.
- Explain shows typed variables.
- Explain shows chosen variable order.
- Explain shows chosen Free Join nodes and subatoms.
- Explain shows optimizer candidates and stable cost keys.
- Explain shows estimated cardinalities.
- Explain shows actual node row/candidate counts.
- Explain keeps cursor seek/scanned-key counters as zero-regression gates.
- Explain shows actual yielded rows.
- Explain shows filtered rows.
- Explain shows aggregation counts.
- Explain shows elapsed time per operator.
- Explain is a development feature and benchmark weapon.

**Stats**
- Stats are internal.
- Stats are maintained during writes.
- Stats are not user configuration.
- Stats include row count per relation.
- Stats include index entry count.
- Stats include min/max for range-indexed scalars.
- Stats include approximate distinct count per indexed field when implemented.
- Stats include fanout estimates for refs.
- Stats may be coarse in v0.
- Query plans must remain correct if stats are bad.
- Bad stats may hurt performance but not correctness.
- There is no manual `ANALYZE` in v0.
- A future internal rebuild-stats command may exist.

**Planner Correctness**
- Planner must never change query semantics.
- All optimization is semantics-preserving.
- All index choices are interchangeable at the logical level.
- Every relation atom can fall back to primary scan if needed.
- Fallback scans are correct but may be slow.
- Missing optional scalar indexes do not make queries impossible.
- Missing optional scalar indexes may make queries slower.
- Ref indexes are always present because joins are core.

**Transaction Model**
- One write transaction at a time.
- Many read transactions at a time.
- Write transactions are atomic.
- Write transactions are isolated from readers until commit.
- Read transactions see snapshot isolation.
- There is no user-visible isolation-level choice.
- There are no nested user transactions in v0.
- LMDB nested transactions are not exposed in v0.
- Batch writes are encouraged.
- One write closure equals one committed database transaction.

**Transaction Log**
- Every committed write gets a transaction ID.
- Transaction IDs are monotonic.
- Transaction log records changed relation.
- Transaction log records changed key.
- Transaction log records operation kind.
- Transaction log records old tuple for replace/delete where needed.
- Transaction log records new tuple for insert/replace where needed.
- Transaction log records commit timestamp.
- Transaction log can later power as-of queries.
- Transaction log can later power audit APIs.
- Transaction log is not used for normal current reads.

**Error Model**
- `SchemaMismatch`
- `StorageFormatMismatch`
- `TypeError`
- `QueryParseError`
- `QueryTypeError`
- `QueryPlanError`
- `ConstraintViolation`
- `UniqueViolation`
- `ForeignKeyViolation`
- `DuplicateTuple`
- `NotFound`
- `DecimalOverflow`
- `IntegerOverflow`
- `MapFull`
- `MapResized`
- `HashCollision`
- `Corruption`
- `Io`
- `Lmdb`
- `InternalInvariantViolation`

**API Shape**
```rust
let db = LedgerDb::open(path)?;

db.write(|tx| {
    let account = tx.alloc::<AccountId>()?;
    tx.insert(Account { id: account, holder, currency })?;
    Ok(())
})?;

let rows = db.read(|snap| {
    snap.query(r#"
        find ?posting ?amount
        where
          Posting(id: ?posting, account: $account, amount: ?amount)
    "#, inputs)
})?;
```

**Public API Principles**
- API is generated from schema.
- API is typed.
- API is synchronous.
- API is closure-scoped by default.
- API avoids exposing LMDB details.
- API exposes query explain.
- API exposes backup.
- API exposes compact-copy later.
- API does not expose tuning knobs in v0.
- API does not expose migrations.
- API does not expose SQL.
- API does not expose raw untyped writes.

**Prepared Queries**
- Runtime query strings are supported first.
- Prepared query objects are supported after basic runtime queries.
- Prepared queries are tied to schema fingerprint.
- Prepared queries cache parsed IR.
- Prepared queries cache typechecking.
- Prepared queries may cache plans.
- Cached plans must remain valid across stats changes or be replanned.
- Compile-time query macros are later.
- Compile-time query macros use the same parser/typechecker.

**Query Macro Future**
```rust
let q = datalog! {
    find ?posting ?amount
    where
      Posting(id: ?posting, account: $account, amount: ?amount)
};
```

**Query Macro Decisions**
- Macro is not required for v0.
- Macro must not define a separate language.
- Macro reuses canonical parser.
- Macro typechecks against generated schema metadata.
- Macro emits a prepared query structure.
- Macro catches relation and field typos at compile time.
- Runtime strings remain necessary for debugging and REPL-style use.

**Backup**
- Backup API is supported.
- Backup uses LMDB’s environment copy capabilities where possible.
- Backup is explicit.
- Backup is not automatic.
- Online backup should be safe with readers/writer semantics.
- Backup does not transform schema.
- Backup does not compact unless explicitly requested.

**Compaction**
- LMDB file growth is accepted.
- Online vacuum is not v0.
- Compaction is copy-to-new-path.
- Application swaps compacted database if desired.
- Long-lived readers can increase file growth.
- Reader discipline is part of embedded usage.

**Crash Consistency**
- LMDB provides transactional atomicity.
- We do not bypass LMDB durability in v0.
- Partial index updates cannot commit.
- Write transaction abort leaves database unchanged.
- Commit either persists all relation indexes and logs or none.
- Crash tests are required.
- Metadata updates happen in same write transaction as data changes when relevant.

**Security**
- No encryption in v0.
- No access control in v0.
- No multi-user permission model.
- Sensitive data should not rely on LMDB flags for wiping.
- Application handles file permissions.
- Application handles encryption at rest if needed.

**File Format**
- Storage format version is explicit.
- Storage format version mismatch is hard failure.
- Schema fingerprint mismatch is hard failure.
- Key encodings are deterministic.
- Key encodings are endian-stable at the application-key level.
- LMDB environment file portability follows LMDB limitations.
- There is no cross-engine file format.
- There is no textual dump format in v0 except optional debug tools.
- ETL is the migration/export/import story.

**Testing Strategy**
- Encoding order tests are mandatory.
- Schema fingerprint tests are mandatory.
- Constraint tests are mandatory.
- Foreign-key tests are mandatory.
- Unique tests are mandatory.
- Insert/replace/delete index consistency tests are mandatory.
- Query parser tests are mandatory.
- Query typechecker tests are mandatory.
- Planner golden tests are mandatory.
- Executor correctness tests are mandatory.
- Explain plan tests are mandatory.
- Crash/abort tests are mandatory.
- Differential tests against an in-memory reference engine are mandatory.
- Benchmark tests against SQLite/Postgres are mandatory later.

**Reference Engine**
- A simple in-memory interpreter should exist for testing.
- Reference engine prioritizes correctness, not speed.
- Query results from LMDB engine are compared against reference engine.
- Randomized schemas are not v0.
- Randomized data for fixed benchmark schemas is valuable.
- Property tests should stress joins, filters, and aggregation.

**Benchmark Product Definition**
- Benchmarks define whether the database is succeeding.
- Benchmarks use realistic normalized ledger schemas.
- Benchmarks compare against SQLite with correct indexes.
- Benchmarks compare against Postgres with correct indexes.
- Benchmarks include cold-ish and warm mmap cases.
- Benchmarks include write batches.
- Benchmarks include read snapshots.
- Benchmarks include explain counters.

**Benchmark Schema**
```text
Holder
Account
Instrument
JournalEntry
Posting
PostingTag
Org
OrgParent
AuthorizationEdge
ExchangeRate
SourceDocument
```

**Benchmark Queries**
- Fetch postings for holder over time range.
- Compute balances grouped by instrument.
- Fetch postings for account over time range.
- Fetch all journal entries touching an account set.
- Reverse lookup all facts referencing an entity.
- Traverse organization parent graph later.
- Check authorization reachability later.
- Join postings, accounts, holders, instruments, entries, documents.
- Run 10-plus relation selective join.
- Run cyclic relationship query.
- Reconstruct many entities by primary key.
- Aggregate ledger amounts by account and instrument.
- Compare current-state query performance.
- Compare bulk ETL load performance.

**Performance Philosophy**
- Optimize reads before writes.
- Optimize joins before scans.
- Optimize current-state before history.
- Optimize typed values before dynamic values.
- Optimize sorted cursor access before hash materialization.
- Optimize prefix scans before full scans.
- Optimize planner observability before planner cleverness.
- Avoid magic.
- Prefer predictable performance over surprising heuristics.
- Make bad plans explainable.

**Write Amplification**
- Covering indexes intentionally amplify writes.
- Ref indexes intentionally amplify writes.
- History intentionally amplifies writes.
- Dictionary interning adds write overhead for variable-size values.
- This is acceptable because target workload is read-heavy and embedded.
- Bulk ETL mitigates initial load cost.
- Single-row insert speed is not the primary benchmark.
- Multiway join speed is the primary benchmark.

**Memory Layout In Process**
- Query variables are dense integer slots.
- Runtime values use compact typed words internally.
- `u64`, refs, bools, timestamps fit compact words.
- Decimal and UUID use wider value representation.
- Strings/blobs use interned IDs in joins.
- Decoding to Rust user types happens at API boundaries.
- LMDB key bytes may be compared without decoding.
- Allocations inside hot join loops are avoided.
- Output vectors allocate only for final results.
- Aggregation allocates hash maps when needed.
- Future recursion allocates temporary relation sets.

**Internal Value Representation**
```text
Word64
Word128
InternId
TypedVar
EncodedKeySlice
DecodedValue
```

**Safety**
- Unsafe Rust is minimized.
- Unsafe Rust is isolated around LMDB FFI and byte decoding.
- Encoders and decoders are tested aggressively.
- Returned LMDB memory never outlives transaction lifetimes.
- Public APIs should make misuse hard.
- Internal invariants use debug assertions.
- Corruption or impossible states return explicit internal errors.

**Rust Implementation Principles**
- Generated code should be readable.
- Core engine should not depend on generated code internals more than necessary.
- Schema descriptors bridge generated API and generic engine.
- Keep abstractions thin.
- Avoid premature trait labyrinths.
- Avoid C++-style template metaprogramming in Rust clothing.
- Use traits when there is real polymorphism.
- Use enums when the set is closed.
- Use small structs for hot paths.
- Avoid global mutable state.
- Avoid background magic.

**Crate Shape**
```text
bumbledb
bumbledb-macros
bumbledb-lmdb
bumbledb-core
```

**Crate Responsibilities**
- `bumbledb` is the public facade.
- `bumbledb-macros` provides schema and future query macros.
- `bumbledb-lmdb` wraps LMDB safely.
- `bumbledb-core` contains encoding, planning, execution, and schema descriptors.
- Crate split may happen after initial prototype.
- Initial implementation may be simpler, but boundaries should be respected.

**Layering**
```text
Generated Typed API
Datalog Parser
Typechecker
Logical IR
Planner
Executor
Access Path Layer
Encoding Layer
LMDB Layer
```

**Layer Rules**
- Parser does not know LMDB.
- Typechecker does not know LMDB.
- Planner knows schema indexes but not raw LMDB FFI.
- Executor knows access paths and cursors.
- Access path layer knows encoded keys.
- Encoding layer knows byte order.
- LMDB layer knows transactions and cursors.
- Generated API knows user types and schema descriptors.

**Parser**
- Parser accepts canonical Datalog syntax.
- Parser produces untyped AST.
- Parser reports precise spans.
- Parser error messages matter.
- Parser does not do type inference.
- Parser does not plan.
- Parser does not access schema except maybe for keyword recognition.
- Parser is deterministic.

**Typechecker**
- Typechecker consumes AST and schema descriptors.
- Typechecker produces typed logical IR.
- Typechecker assigns dense variable IDs.
- Typechecker resolves inputs.
- Typechecker validates fields.
- Typechecker validates comparisons.
- Typechecker validates aggregates.
- Typechecker rejects unsupported features.
- Typechecker emits user-facing diagnostics.

**Planner**
- Planner consumes normalized query IR and stats.
- Planner emits physical plan.
- Planner emits explain metadata.
- Planner can be deterministic for stable testing.
- Planner has fallback plans.
- Planner can later become more cost-based.
- Planner should remain inspectable.

**Executor**
- Executor consumes normalized query IR, encoded inputs, a Free Join physical plan, and a QueryImage snapshot.
- Executor emits typed result rows.
- Executor records counters.
- Executor performs late materialization.
- Executor avoids unnecessary decoding.
- Executor owns temporary aggregation state.
- Executor does not mutate database.
- Executor is transaction-scoped.

**Runtime Specialization Boundary**
- `ExecutablePlan` is the internal trait boundary for future generated/specialized execution.
- The first implementation remains interpreted Free Join.
- Future specialized plans should replace dynamic field/type lookups with fixed offsets and monomorphic predicate/aggregate code.
- Specialized plans must emit into the same tuple sinks as interpreted plans.

**Access Path Layer**
- Access paths expose relation indexes generically.
- Access paths hide raw key namespace details from planner.
- Access paths identify sorted/hash trie field order over QueryImage data.
- Access paths identify durable segment index chunks by access ID.
- Access paths know whether index ordering satisfies variable ordering.
- Access paths provide cardinality estimates.
- Access paths do not expose LMDB cursor factories to query recursion.

**Encoding Layer**
- Encoding is centralized.
- Encoding is schema-driven.
- Encoding is tested independently.
- Encoding never allocates in hot loops unless unavoidable.
- Encoding rejects unsupported value sizes.
- Encoding validates key length.
- Encoding version is part of storage format.
- Encoding changes require storage format version bump and ETL.

**Datalog Result Semantics**
- Query results are sets unless projection duplicates arise through aggregation semantics.
- Duplicate result rows are eliminated or avoided according to Datalog set semantics.
- We need explicit decision: v0 returns set semantics.
- Result ordering is not guaranteed unless future `order` syntax is added.
- `limit` is not v0 unless needed.
- Stable order for tests can be applied externally.
- Ordered query output is future syntax, not accidental index order.

**Ordering And Limit**
- No implicit order.
- No SQL-style `ORDER BY` in v0.
- No `LIMIT` in v0 unless needed for debugging.
- Future ordering must require indexed order or explicit sort.
- Sorting large result sets is not a core v0 goal.

**Set Semantics**
- Base relations are sets.
- Datalog output uses set semantics.
- Aggregation groups over sets.
- Duplicate physical index entries are forbidden.
- Duplicate query outputs are removed if the plan can produce duplicates.
- Planner should prefer plans that avoid duplicate generation.

**Constraints Beyond FKs**
- Primary key constraints are v0.
- Unique constraints are v0.
- Foreign-key constraints are v0.
- Required fields are implicit because nulls are forbidden.
- Check constraints are not v0.
- Arbitrary user validation functions are not v0.
- Cross-row complex constraints can be represented as queries initially.
- Future tx functions may enforce richer invariants.

**Tx Functions**
- Transaction functions are not v0.
- Future tx functions must be deterministic.
- Future tx functions must run inside write transaction.
- Future tx functions may perform reads.
- Future tx functions may emit writes.
- Future tx functions are Rust functions, not stored procedures.
- Future tx functions are application code, not database schema migration code.

**History Query Future**
```text
as_of tx 12345
find ?posting ?amount
where
  Posting(id: ?posting, amount: ?amount)
```

**History Query Decisions**
- Current query syntax remains default.
- Historical context must be explicit.
- Historical context may be transaction ID.
- Historical context may later be timestamp resolved to tx ID.
- Historical queries use history indexes.
- Current indexes are not scanned to answer past-state queries.
- Full bitemporal valid-time semantics are not assumed.

**Migration Policy**
- There is no migration DSL.
- There is no `ALTER`.
- There is no schema registry evolution.
- There is no backward compatibility layer for old schemas.
- ETL is the migration story.
- ETL reads old database with old binary.
- ETL writes new database with new binary.
- ETL can be application-specific.
- ETL can use bulk loader.
- This is acceptable because database is embedded.

**Development Reset**
- A dev helper may delete/recreate a database path.
- Dev reset is not migration.
- Dev reset is not automatic.
- Production open never destroys data.
- Schema mismatch never silently recreates.

**Observability**
- Explain plans are required.
- Storage stats are useful.
- Relation row counts are visible.
- Index sizes may be visible.
- Dictionary size may be visible.
- Current transaction ID is visible.
- Schema fingerprint is visible.
- LMDB map usage may be visible.
- These are diagnostics, not tuning knobs.

**Non-Goals**
- SQL compatibility.
- ORM compatibility.
- Multiple writer scalability.
- Distributed replication.
- Server deployment.
- Query federation.
- Full-text search.
- Vector search.
- JSON document database behavior.
- OLAP compression.
- Arbitrary user-defined indexes.
- Runtime schema changes.
- Online migrations.
- Fine-grained access control.
- Transparent encryption.
- Async runtime integration.
- Cross-language ABI in v0.

**First Implementation Milestone**
- Open/create LMDB environment.
- Store metadata and schema fingerprint.
- Generate schema descriptors.
- Encode/decode primitive values.
- Insert rows into current covering indexes.
- Read rows by primary key.
- Enforce primary key uniqueness.
- Enforce foreign keys.
- Enforce unique constraints.
- Maintain tx ID.
- Append tx log.
- Run simple current-state scans.

**Second Implementation Milestone**
- Parse named-field Datalog.
- Typecheck relation atoms.
- Execute single-relation queries.
- Execute two-relation joins.
- Use ref indexes.
- Return typed results.
- Provide explain output.

**Third Implementation Milestone**
- Implement access path enumeration.
- Implement variable ordering heuristic.
- Implement trie/cursor join core.
- Implement multiway joins.
- Implement range predicates.
- Implement aggregation.
- Add stats counters.
- Add benchmark schema.

**Fourth Implementation Milestone**
- Bulk loader.
- Prepared queries.
- More explain counters.
- Differential tests.
- SQLite/Postgres benchmark comparison.
- Dictionary collision handling.
- Backup API.
- Compaction copy helper.

**Deferred Milestones**
- Recursive rules.
- Semi-naive evaluation.
- Stratified negation.
- As-of queries.
- Query macros.
- Ordered output.
- Limit.
- String range indexes.
- Prefix text indexes.
- User-defined pure functions.
- Tx functions.
- Explicit advanced snapshots.

**One-Sentence Architecture**
```text
A hardcoded typed Rust schema generates compact relation/index metadata; writes maintain safe LMDB durability plus encoded query-image segments; typed Datalog queries compile into stats-backed Free Join plans over QueryImage sorted/hash tries and late output sinks; schema or storage-layout changes are handled only by full ETL.
```

**Final Canonical Decision Set**
- BCNF over global 6NF.
- Typed n-ary relations over EAV.
- Schema macro over runtime DDL.
- Full ETL over migrations.
- Null ban over nullable fields.
- Decimal over floats.
- Interned strings over raw string keys.
- Covering indexes over row lookup indirection.
- Ref indexes by default.
- Scalar indexes only by schema annotation.
- Current indexes separate from history.
- Safe LMDB flags over unsafe speed modes.
- One small fixed DBI set over dynamic DBIs.
- Explicit sortable byte encoding over LMDB integer comparators.
- Named-field Datalog over positional syntax.
- Positive Datalog plus aggregation in v0.
- Recursion and negation later.
- Free Join planner over binary-join-only planner.
- Explain plans from day one.
- Synchronous closure-scoped API over async/global handles.
- Embedded assumptions over server/database-generalist assumptions.
