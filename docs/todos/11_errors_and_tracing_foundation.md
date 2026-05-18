# 11: Errors And Tracing Foundation

**Goal**
- Refactor errors into an idiomatic layered Rust error model and add a consistent `tracing` instrumentation story before further planner/executor optimization.

**Why This Stage Exists**
- The planner and executor are now the main bottleneck, and optimizing them without structured observability will be guesswork.
- The current storage error enum mixes user mistakes, LMDB failures, schema problems, query failures, constraint violations, corruption, test failpoints, and internal bugs.
- Planner work will add more failure modes. If error boundaries are not cleaned up first, future planner work will make the API harder to use and the internals harder to debug.
- Tracing and errors are foundational: they make benchmark-driven optimization measurable, debuggable, and supportable.

**Design Position**
- Do this before real WCOJ/triejoin work.
- Do this before cost-based planner work.
- Do this before prepared queries or recursive rules.
- Keep the public API stable enough for users, but this is still early enough to make a clean breaking refactor.
- Treat this as library-quality infrastructure, not cosmetic cleanup.

**Current Problems**
- `bumbledb-lmdb::Error` is too broad.
- Query parse/typecheck errors live in `bumbledb-core`, while query execution errors live in `bumbledb-lmdb`, with no coherent public taxonomy.
- Constraint errors and storage errors are mixed.
- LMDB errors are wrapped directly but not categorized by operation.
- Internal invariant failures are plain strings.
- Benchmark/failpoint errors share the same public enum as real user-facing failures.
- There is no consistent way to identify whether an error is user-correctable, storage/environmental, corruption, or an engine bug.
- There are explain counters, but no runtime tracing spans for where time is spent.
- Benchmarks show slowness, but we cannot yet break time down by planning, scans, decoding, filtering, projection, aggregation, dictionary lookup, or LMDB calls.

**Non-Goals**
- Do not change storage format.
- Do not add new query semantics.
- Do not optimize the planner in this stage.
- Do not initialize a tracing subscriber inside library crates.
- Do not make tracing mandatory for users.
- Do not log every row at normal levels.
- Do not expose raw LMDB handles or low-level internals through errors.
- Do not create an enormous general-purpose diagnostics framework.

**Crate Boundary Decision**
- `bumbledb-core` owns schema, encoding, parser, typechecker, and logical IR errors.
- `bumbledb-lmdb` owns storage, transaction, query execution, backup, ETL, and LMDB-backed errors.
- The future public `bumbledb` facade should expose one public `bumbledb::Error` that wraps lower-layer errors without leaking unstable internals.
- Error types that are public should be marked `#[non_exhaustive]`.
- Internal-only error helpers may remain exhaustive.

**Top-Level Public Error Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Open(#[from] OpenError),

    #[error(transparent)]
    Schema(#[from] SchemaError),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    Transaction(#[from] TransactionError),

    #[error(transparent)]
    Constraint(#[from] ConstraintError),

    #[error(transparent)]
    Query(#[from] QueryError),

    #[error(transparent)]
    Backup(#[from] BackupError),

    #[error(transparent)]
    Corruption(#[from] CorruptionError),

    #[error(transparent)]
    Internal(#[from] InternalError),
}
```

**Error Classification**
- `OpenError`: environment open/create failures and fixed DBI setup.
- `SchemaError`: schema descriptors, fingerprints, layout validation, schema mismatch.
- `StorageError`: dictionary, row/index storage, metadata, map size, LMDB storage operations.
- `TransactionError`: read/write transaction lifecycle, commit/abort semantics, reader issues.
- `ConstraintError`: primary key, unique, foreign key, restrict delete, required fields, type mismatch on writes.
- `QueryError`: query parse/typecheck/plan/execute/input/projection/aggregation failures.
- `BackupError`: backup and compact-copy failures.
- `CorruptionError`: persisted state violates expected format or invariants.
- `InternalError`: engine bug, impossible state, invariant violation not caused by user data.

**User-Correctable Errors**
- Unknown relation.
- Unknown field.
- Missing required field.
- Type mismatch.
- Duplicate tuple.
- Unique violation.
- Foreign-key violation.
- Restrict-delete violation.
- Missing query input.
- Query input type mismatch.
- Parse/typecheck diagnostics.
- Schema mismatch requiring ETL.

**Environmental Or Operational Errors**
- Filesystem IO.
- LMDB map full.
- LMDB map resized.
- LMDB reader slots full.
- Backup target error.
- Compact-copy error.
- Environment already open with incompatible options.

**Corruption Errors**
- Metadata has invalid width.
- Row payload width does not match schema.
- Index key width does not match layout.
- Index key prefix does not match layout.
- Index key component truncated.
- Index does not cover every relation field.
- Dictionary forward value too short.
- Dictionary ID width invalid.
- Dictionary reverse value missing.
- Dictionary hash collision with mismatched raw bytes.
- Stored string dictionary bytes are invalid UTF-8.

**Internal Errors**
- Missing generated ID metadata for generated-ID operation.
- Missing index layout that schema generation should guarantee.
- Metadata counter overflow/underflow.
- Too many history records in one transaction.
- Projection of an unbound variable after typechecker accepted query.
- Aggregation state receives wrong runtime type.
- Unreachable planner/executor state.

**OpenError Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenError {
    #[error("failed to create database directory {path}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to open LMDB environment at {path}")]
    EnvironmentOpen {
        path: PathBuf,
        #[source]
        source: heed::Error,
    },

    #[error("failed to open fixed LMDB database {name}")]
    FixedDatabaseOpen {
        name: &'static str,
        #[source]
        source: heed::Error,
    },

    #[error("storage format version metadata is missing")]
    MissingStorageFormatVersion,

    #[error("storage format version mismatch: expected {expected}, found {found}")]
    StorageFormatMismatch { expected: u32, found: u32 },
}
```

**SchemaError Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SchemaError {
    #[error("relation {relation} references unknown field {field}")]
    UnknownField { relation: String, field: String },

    #[error("index key too large for {relation}.{index}: {actual} bytes exceeds max {max} bytes")]
    KeyLayoutTooLarge {
        relation: String,
        index: String,
        actual: usize,
        max: usize,
    },

    #[error("schema fingerprint mismatch: expected {expected}, found {found}")]
    SchemaMismatch { expected: String, found: String },
}
```

**StorageError Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StorageError {
    #[error("LMDB operation {operation} failed")]
    Lmdb {
        operation: &'static str,
        #[source]
        source: heed::Error,
    },

    #[error("bulk load target already contains a database: {path}")]
    BulkLoadTargetExists { path: PathBuf },

    #[error("dictionary value not found for {kind}")]
    DictionaryValueNotFound { kind: DictionaryKind },

    #[error("dictionary hash collision for {kind}")]
    HashCollision { kind: DictionaryKind },

    #[error("metadata counter {name} overflowed")]
    CounterOverflow { name: &'static str },

    #[error("metadata counter {name} underflowed")]
    CounterUnderflow { name: &'static str },
}
```

**ConstraintError Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConstraintError {
    #[error("duplicate tuple in relation {relation}")]
    DuplicateTuple { relation: String },

    #[error("tuple not found in relation {relation}")]
    NotFound { relation: String },

    #[error("unique constraint {relation}.{constraint} violated")]
    UniqueViolation { relation: String, constraint: String },

    #[error("foreign key {relation}.{field} references missing {target_relation}")]
    ForeignKeyViolation {
        relation: String,
        field: String,
        target_relation: String,
    },

    #[error("cannot delete {relation}; referenced by {referenced_by}.{field}")]
    RestrictViolation {
        relation: String,
        referenced_by: String,
        field: String,
    },

    #[error("missing field {relation}.{field}")]
    MissingField { relation: String, field: String },

    #[error("unknown field {relation}.{field}")]
    UnknownField { relation: String, field: String },

    #[error("type mismatch for {relation}.{field}: expected {expected}, got {actual}")]
    TypeMismatch {
        relation: String,
        field: String,
        expected: String,
        actual: &'static str,
    },

    #[error("foreign key target {target_relation} must have a single-field primary key")]
    UnsupportedCompositeForeignKey { target_relation: String },
}
```

**QueryError Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum QueryError {
    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Typecheck(#[from] TypecheckError),

    #[error(transparent)]
    Plan(#[from] PlanError),

    #[error(transparent)]
    Execute(#[from] ExecuteError),

    #[error(transparent)]
    Aggregate(#[from] AggregateError),
}
```

**Query Diagnostic Requirements**
- Parser errors must include source span.
- Typechecker errors must include source span when available.
- Unknown relation errors must include relation name.
- Unknown field errors must include relation and field name.
- Variable type conflict errors must include variable name, existing type, incoming type.
- Input type conflict errors must include input name, existing type, incoming type.
- Unsupported feature errors must name the deferred feature explicitly.
- Query execution errors must include query input names where relevant.

**PlanError Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlanError {
    #[error("no access path for relation {relation}")]
    NoAccessPath { relation: String },

    #[error("missing primary index for relation {relation}")]
    MissingPrimaryIndex { relation: String },

    #[error("unknown index {relation}.{index}")]
    UnknownIndex { relation: String, index: String },

    #[error("range index {relation}.{index} has no leading field")]
    InvalidRangeIndex { relation: String, index: String },

    #[error("index prefix for {relation}.{index} is not contiguous")]
    NonContiguousPrefix { relation: String, index: String },
}
```

**ExecuteError Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExecuteError {
    #[error("missing query input ${input}")]
    MissingInput { input: String },

    #[error("query input ${input} expected {expected}, got {actual}")]
    InputTypeMismatch {
        input: String,
        expected: String,
        actual: &'static str,
    },

    #[error("variable {variable} is unbound at projection")]
    UnboundProjectionVariable { variable: usize },

    #[error("typed literal does not match literal value")]
    LiteralMismatch,
}
```

**AggregateError Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AggregateError {
    #[error("integer overflow during {operation}")]
    IntegerOverflow { operation: &'static str },

    #[error("decimal overflow during {operation}")]
    DecimalOverflow { operation: &'static str },

    #[error("aggregate {function} received unexpected value kind {actual}")]
    TypeMismatch {
        function: &'static str,
        actual: &'static str,
    },
}
```

**TransactionError Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TransactionError {
    #[error("failed to begin read transaction")]
    BeginRead { #[source] source: heed::Error },

    #[error("failed to begin write transaction")]
    BeginWrite { #[source] source: heed::Error },

    #[error("failed to commit write transaction")]
    Commit { #[source] source: heed::Error },

    #[error("reader cleanup failed")]
    ReaderCleanup { #[source] source: heed::Error },
}
```

**BackupError Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BackupError {
    #[error("failed to create backup directory {path}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to copy LMDB environment to {path}")]
    Copy {
        path: PathBuf,
        compact: bool,
        #[source]
        source: heed::Error,
    },
}
```

**CorruptionError Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CorruptionError {
    #[error("metadata {name} has invalid width: expected {expected}, got {actual}")]
    MetadataWidth {
        name: &'static str,
        expected: usize,
        actual: usize,
    },

    #[error("row payload width for {relation} does not match schema: expected {expected}, got {actual}")]
    RowPayloadWidth {
        relation: String,
        expected: usize,
        actual: usize,
    },

    #[error("index key width for {relation}.{index} does not match layout")]
    IndexKeyWidth { relation: String, index: String },

    #[error("dictionary forward value for {kind} is too short")]
    DictionaryForwardTooShort { kind: DictionaryKind },

    #[error("dictionary string is not valid UTF-8")]
    InvalidUtf8DictionaryString,
}
```

**InternalError Shape**
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InternalError {
    #[error("internal invariant failed: {message}")]
    Invariant { message: String },

    #[error("missing generated ID metadata for relation {relation}")]
    MissingGeneratedId { relation: String },

    #[error("too many history records in one transaction")]
    TooManyHistoryRecords,

    #[error("unsupported internal state: {message}")]
    UnsupportedState { message: String },
}
```

**TestFailpointError Decision**
- Failpoints are test-only behavior.
- In normal library builds, failpoint APIs should not be public.
- Under `test-failpoints`, injected failpoint errors may use `InternalError::InjectedFailpoint` or a dedicated `TestError` hidden behind the feature.
- Do not make failpoint errors part of stable user-facing API documentation.

**Result Aliases**
- Keep `pub type Result<T> = std::result::Result<T, Error>` at public crate boundaries.
- Inside modules, prefer domain-specific aliases only where helpful.
- Avoid `anyhow` in library crates.
- Benchmarks/tests may use `Box<dyn std::error::Error>` or `anyhow` later if desired, but the library should not.

**Conversion Strategy**
- Implement `From<OpenError> for Error`, `From<StorageError> for Error`, etc. through enum variants.
- Avoid blanket conversion from `heed::Error` directly into public top-level `Error`; require operation context.
- Replace plain `Error::Internal(String)` with structured `InternalError` variants.
- Preserve source chains with `#[source]`.
- Use helper constructors for repeated LMDB operation contexts.

**Tracing Design Principles**
- Use the `tracing` crate in library code.
- Do not initialize a subscriber in library code.
- Benchmarks, examples, tests, and user applications choose subscribers.
- Spans should make benchmark profiles explainable.
- Do not trace every row at `debug` or `info`.
- Per-row/per-cursor detail is allowed only at `trace` level.
- Every trace event/span should use structured fields, not only formatted strings.
- Hot-path tracing should be guarded by level checks or kept minimal.

**Tracing Dependencies**
- Add `tracing` to workspace dependencies.
- Add `tracing-subscriber` as a dev/bench dependency, not as a required runtime dependency for the library.
- The benchmark binary may support `--trace` or `RUST_LOG`/`BUMBLEDB_TRACE` via `tracing_subscriber::EnvFilter`.

**Tracing Levels**
- `info`: environment open, schema verification, bulk-load summary, backup summary, compact-copy summary.
- `debug`: query plan summary, chosen access paths, per-operator summary counters, transaction summaries.
- `trace`: per-scan open/close, cursor prefixes, range bounds, per-atom candidate counts, dictionary interning detail, row decode detail when needed.
- `warn`: recoverable operational anomalies, such as stale readers cleared or benchmark result mismatch before returning an error.
- `error`: only before returning errors with meaningful context, not for expected user mistakes in hot paths.

**Required Spans**
- `bumbledb.open`
- `bumbledb.open_fixed_databases`
- `bumbledb.verify_schema`
- `bumbledb.read_txn`
- `bumbledb.write_txn`
- `bumbledb.commit`
- `bumbledb.bulk_load`
- `bumbledb.insert`
- `bumbledb.replace`
- `bumbledb.delete`
- `bumbledb.dict_intern`
- `bumbledb.index_put`
- `bumbledb.index_delete`
- `bumbledb.query.parse`
- `bumbledb.query.typecheck`
- `bumbledb.query.plan`
- `bumbledb.query.execute`
- `bumbledb.query.execute_atom`
- `bumbledb.query.scan`
- `bumbledb.query.filter`
- `bumbledb.query.project`
- `bumbledb.query.aggregate`
- `bumbledb.backup`
- `bumbledb.compact_copy`

**Span Field Standards**
- Always include `db.path` when opening or copying databases if available.
- Include `relation` for relation operations.
- Include `field` for field-level errors or dictionary work.
- Include `index` for index scans/writes/deletes.
- Include `constraint` for unique/constraint errors.
- Include `tx.id` when known.
- Include `query.id` if/when prepared query IDs exist later.
- Include `rows`, `rows_scanned`, `rows_matched`, `output_rows` for summaries.
- Include `cursor_seeks` for query execution summaries.
- Include `elapsed_us` or rely on subscriber timing layers where possible.

**Query Tracing Requirements**
- `query.execute` span includes number of variables, clauses, relation atoms, comparisons, projections, and aggregate count.
- `query.plan` event includes variable order and atom order.
- `query.plan` event includes chosen index and prefix fields for each atom.
- `query.execute_atom` span includes relation, chosen index, prefix fields, and whether range bounds were used.
- At `debug`, emit one summary event per atom with rows scanned and rows matched.
- At `trace`, optionally emit scan prefix/range bytes lengths, not raw values by default.
- `query.aggregate` span includes aggregate function count and group count.

**Storage Tracing Requirements**
- `write_txn` span includes whether commit or abort occurred.
- `bulk_load` span includes attempted rows, inserted rows, dictionary entries after load, storage transaction ID, and elapsed time.
- `dict_intern` span includes dictionary kind and whether value already existed, but not raw string/blob contents by default.
- `index_put` and `index_delete` trace events include relation and index only at trace level.
- `backup` and `compact_copy` spans include target path and compact flag.

**Benchmark Integration**
- Add optional benchmark CLI flag `--trace` or rely on `RUST_LOG`.
- Add benchmark output mode that prints query explain and tracing summaries together.
- Benchmarks should not enable per-row tracing by default.
- Benchmark docs should show commands like:
```sh
RUST_LOG=bumbledb_lmdb=debug cargo run -p bumbledb-bench --release -- --dataset joinstress --scale 2000 --repeats 10
```

**Error/Tracing Relationship**
- Errors should carry stable structured context.
- Tracing should add dynamic execution context.
- Do not rely on logs to understand errors.
- Do not put large payloads in errors or tracing fields.
- Error variants should be usable in tests without parsing strings.
- Trace events may include display strings for humans, but tests should assert structured error variants.

**Migration Plan: Errors**
- Create new error modules without changing behavior first.
- Add new enums in `bumbledb-core` and `bumbledb-lmdb`.
- Map old variants to new structured variants.
- Replace call sites gradually from broad `Error::X` variants to layered variants.
- Update tests to assert new structured variants.
- Remove obsolete broad variants.
- Update docs and public facade re-exports.

**Migration Plan: Tracing**
- Add `tracing` dependency.
- Add spans around high-level operations first: open, read, write, bulk load, query execute.
- Add query planning and atom execution spans.
- Add storage write and dictionary spans.
- Add benchmark subscriber support.
- Add tests that install a test subscriber and verify key span/event names are emitted for one query and one write.
- Add docs for `RUST_LOG` usage.

**Implementation Order**
- Add `tracing` dependency and lightweight instrumentation scaffolding.
- Add new error modules and enums.
- Refactor `bumbledb-core::datalog::DatalogError` into parse/typecheck layers while preserving existing diagnostics.
- Refactor `bumbledb-lmdb::Error` into layered enums.
- Update storage write paths to return `ConstraintError`, `StorageError`, `CorruptionError`, and `InternalError` correctly.
- Update query execution to return `QueryError` variants.
- Update backup/ETL/open APIs to return appropriate variants.
- Update tests and trybuild expectations.
- Add tracing spans to open/read/write/query/storage operations.
- Add benchmark tracing setup and docs.
- Run full test, ignored test, clippy, and fuzz gates.

**Testing Requirements**
- Unit tests for error display and source chains.
- Tests asserting constraint failures return `ConstraintError` variants.
- Tests asserting schema mismatch returns `SchemaError::SchemaMismatch`.
- Tests asserting corrupt metadata returns `CorruptionError` variants.
- Tests asserting missing query inputs return `QueryError::Execute` variants.
- Tests asserting aggregation overflow returns `AggregateError` variants.
- Tests asserting LMDB failures include operation context where feasible.
- Tests with a tracing subscriber verifying key spans are emitted.
- Benchmarks must still run without a tracing subscriber.
- Existing fuzz and property tests must continue to pass.

**Backward Compatibility Decision**
- This is pre-public-API stabilization, so a breaking error refactor is acceptable.
- Public user-facing names should be chosen carefully now to avoid churn later.
- Add `#[non_exhaustive]` to public error enums to preserve future compatibility.

**Performance Considerations**
- Tracing must not materially slow benchmarks when disabled.
- Avoid allocation-heavy trace field construction on hot paths.
- Use `tracing::enabled!` or structured simple fields for expensive debug data.
- Per-row trace events must be `trace` only.
- Error context should avoid cloning large values.

**Documentation Requirements**
- Add `docs/ERRORS_AND_TRACING.md` or update this file after implementation with final public shapes.
- Update `docs/TESTING.md` with tracing test commands.
- Update `docs/BENCHMARKS.md` with tracing commands.
- Update `docs/ROSETTA_STONE.md` if the public error model materially changes canonical decisions.

**Out Of Scope**
- Planner improvements.
- WCOJ/triejoin implementation.
- New access paths.
- Prepared query caching.
- Recursive rules.
- As-of query execution.
- Public async API.
- Runtime schema changes.

**Passing Criteria**
- Public and internal error taxonomy is layered and documented.
- Public error enums are marked `#[non_exhaustive]` where appropriate.
- Existing broad storage error variants are removed or hidden behind layered variants.
- Error source chains preserve `heed::Error` and `std::io::Error` causes.
- Tests assert structured variants rather than string matching for important errors.
- Tracing spans exist for open, read, write, bulk load, query plan, query execute, atom execution, scans, projection, aggregation, backup, and compact copy.
- Library crates do not initialize a tracing subscriber.
- Benchmark binary can emit tracing output when requested.
- Normal benchmark output remains clean when tracing is disabled.
- `cargo test --workspace` passes.
- `cargo test -p bumbledb-test-support -- --ignored` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- Existing fuzz targets pass a bounded smoke run.

**Success Definition**
- When a broad join is slow, we can tell whether time went to planning, scans, row decoding, filtering, projection, aggregation, or LMDB cursor operations.
- When an operation fails, users and tests can tell whether the cause is user input, constraints, schema mismatch, storage/LMDB, corruption, or an internal engine bug.
- Planner/executor work can proceed with real observability and clean failure modes.
