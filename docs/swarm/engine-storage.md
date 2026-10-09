# engine-storage lane board

Owns: `crates/bumbledb/{Cargo.toml, src/lib.rs, error*, storage*, schema*, changes*, canonical*,
encoding*, work*, digest.rs, verify_store*, alloc_counter.rs, value.rs, interval*, allen.rs, api.rs,
api/db*}`, the engine integration tests not owned by other lanes, `crates/bumbledb-theory/**`.

Items: A (pointwise sweep), C1, C4 (judge side), C5 (storage side), C6, C7, C8, C9, C10, C15, C16,
C17, G2 (engine), L (code).

## Status

| Item | Status |
|---|---|
| C17: fixed 1 TiB virtual map | landed |
| G2: one allocation counter; `alloc-counter`, `ground-off`, `collision-probe` features deleted | landed |
| C4 judge side: grouped maps RAM-only | landed |
| C16: layout v1 (rows keyed by home, determinant index, meta keys, identity, images) | landed `a92e311bd` |
| C9: one `WriteOutcome<R>`, one private commit path | landed `a92e311bd` |
| C7: `bumbledb::host`; `store`, `integration`, `digest`, `value` private or deleted | landed `a92e311bd` |
| C8 stage A: one flat `Error`, `Error::kind()`, `is_cancelled()` | landed `a92e311bd` |
| C15 engine side: checking in `bumbledb-theory`, the engine seals | landed `d3c01762b` |
| C5 stage A: closed rows carry values and a canonical row; `ClosedRows` deleted | landed `d3c01762b` |
| C15 macro side: `schema!` runs `check` at expansion; spanned errors; compile-fail fixtures | landed `a79b3b50f` |
| C5 stage B: fact codec deleted (`FactLayout`, `FactView`, `SealedRow::fact`, `Relation::layout`, `ValueRef`, `encode_literal`, `decode_sealed`) | landed `a79aab954` |
| C8 stage B: `Error::Store`/`StoreError` deleted; `Error::Cancelled`, `Error::Allocation` | landed `a79aab954` |
| A (pointwise sweep), C10, C1, C6, `unreachable_pub`, L | in progress |

## API changes (announcements)

### Landed

- **Writes (C9).** Every write verb returns `Result<WriteOutcome<R>>`:
  ```rust
  pub struct Committed<R> { pub value: R, pub generation: GenerationId, pub changed: bool }
  pub enum WriteOutcome<R> {
      Committed(Committed<R>),
      Rejected(Violations),
      Moved { witnessed: GenerationId, current: GenerationId },
  }
  impl<R> WriteOutcome<R> { fn unwrap(self) -> Committed<R>; fn expect(self, &str) -> Committed<R> }
  ```
  `Db::write(work, f)`, `Db::write_from(work, &witness, f)`, `Db::apply(&changes, &work)`,
  `Db::apply_from(&changes, &witness, &work)`. `Moved` comes only from the `_from` verbs. Deleted:
  `ApplyExpected`, `ApplyOutcome`, `ConditionalWrite`, `Admission<Committed<R>>` as a write result,
  `CoreCommit`. `Db::create` still returns `Result<Admission<Db<S>>>` (the empty state is judged).
  `OwnedRead::witness()` and `ReadFrame::witness()` return `Witness<S>` (no `Result`).
- **Options.** `bumbledb::Options { map_ceiling: u64, durability: Durability }` (`Default`: 1 TiB,
  `Durable`); `Durability::{Durable, Cache}` (`Cache` = `MDB_NOSYNC`). `Db::create_with`,
  `Db::open_with` take it.
- **`bumbledb::host` (C7; log-core R-E1..R-E4).**
  - `Db::database_id(&self) -> DatabaseId` (`DatabaseId(pub [u8; 16])`, `DatabaseId::mint()`).
  - `Db::create_identified(path, schema, DatabaseId, Options, work) -> Result<Admission<Db<S>>>`.
  - `Db::host_writer(&self, &WorkContext) -> Result<WriterSession<'_, S>>`;
    `WriterSession::{generation() -> Result<GenerationId>,
    decide_all(&mut self, &[ChangeSet]) -> Result<Vec<Judged>>,
    apply_decided(&mut self, &[ChangeSet]) -> Result<Prepared>, unchanged(&mut self) -> Result<Prepared>}`;
    `Judged::{Accepted(Applied), Rejected(Violations)}`; `Applied { added: u64, removed: u64 }`.
  - `Prepared::{applied_each() -> &[Applied], seal(HostChanges) -> Result<Sealed>, abort()}`;
    `Sealed::{commit() -> Result<Commit>, abort()}`; `Commit { generation, changed }`.
  - `HostChanges<'a> { records: &'a [HostRecord<'a>], head: Head<'a> }`, `HostChanges::NONE`,
    `HostRecord::{Put { key, value }, Delete { key }}` (keys strictly increasing, at most
    `host::MAX_KEY` = 510 bytes), `Head::{Keep, Put(&[u8]), Clear}`.
  - `ReadFrame::{head(), host_record(key), host_scan(prefix, &mut HostVisitor),
    export(&mut dyn FnMut(RelationId, &[u8]) -> Result<()>), content_digest() -> Result<[u8; 32]>}`.
  - Images: `Db::compact(&self, dest, work)` writes `dest/data.mdb`, a compacted copy with the same
    identity, head and host records; `Db::install_image(image, dest, schema, Options, work)` checks
    format and schema, then publishes.
  - `Population::{begin(dest, schema, DatabaseId, Options, work), copy_relation(&ReadFrame, old,
    new) -> Result<u64>, apply(&ChangeSet) -> Result<Applied>, admit(HostChanges) ->
    Result<Admission<Db<S>>>}`: unjudged population, one complete judgment at `admit`, publication
    only when accepted.
  - `host::{LAYOUT, MAX_KEY, Digest, StoreReport, VerifyCorruption}`.
- **One `Error` (C8 stage A).** Flat variants: `NotABumbleDb { path }`, `SchemaMismatch`,
  `Locked { path }`, `DestinationExists { path }`, `Closed`, `ReentrantWriter`, `ReadersFull`,
  `Full { ceiling }`, `Io`, `Lmdb`, `Corruption(CorruptionError)`, `ForeignSchema`,
  `ForeignWitness`, `ForeignPreparedQuery`, `ClosedRelationWrite`, `TransactionPoisoned`,
  `Changes(ChangeError)`, `HostKey(HostKeyFault)`, `Exhausted(Counter)`, `CapacityRayMeasure`,
  `MeasureOverflow`, `Schema`, `Compile`, `Validation`, `FactShape`, the `Param*` variants,
  `Overflow`, `Scalar`, `Capacity`. `Error::kind() -> ErrorKind` (payload-free, `Copy + Hash`;
  includes `Cancelled` and `Allocation`). `Error::is_cancelled()`. `From<WorkError>`,
  `From<ChangeError>`, `From<RowError>` for `Error` (cancellation and allocation inside a change or
  row error become the cancellation and allocation errors). Deleted: `ErrorFamily`,
  `ErrorDescriptor`, `Hatch`, `FormatMismatch`, `AlreadyInitialized`, `PublishedButUnsynced`,
  `EnvironmentLocked`, `CommitSync`, `Error::display_with`, and the never-built
  `CorruptionError` variants. Transitional: `Error::Store(Box<StoreError>)` now carries only
  cancellation and allocation; it goes in stage B.
- `bumbledb::error` re-exports `ValidationError` and its refusal enums (`Limit`, `HeadMismatch`,
  `FieldRefusal`, `VariableRefusal`, `ParamRefusal`, `ComparisonRefusal`, `Unordered`,
  `AggregateRefusal`, `RecRefusal`).
- `bumbledb::NonDefaultFloatEnvironment` replaces `UnsupportedNumericalPlatform` in the root
  re-exports (numeric request 1).
- `RowReader::next_fixed_bytes::<N>() -> Result<[u8; N]>` (macros request 5).
- **Verification.** `Db::verify_store(&self, &WorkContext) -> Result<StoreReport>`,
  `StoreReport { corruption: Box<[VerifyCorruption]>, violations: Option<Violations> }`,
  `is_coherent()`. Deleted: `StoreFinding`, `StoreVerdict`, `findings()`.
- **Deleted from the public surface:** `bumbledb::store`, `bumbledb::integration`,
  `bumbledb::digest` (use `bumbledb::host::Digest` or `blake3`), `bumbledb::value`,
  `STORAGE_FORMAT_VERSION` (use `bumbledb::host::LAYOUT`), `Db::integration_*`,
  `Db::create_unjudged`, public `ReadInstance` (use `ReadFrame`).
- Features: only `testing` remains. It gates `with_grounding_disabled`; forced fingerprints are
  `#[cfg(test)]` only.
- `bumbledb::alloc_counter` is always compiled; a test binary that measures allocations registers
  `#[global_allocator] static A: bumbledb::alloc_counter::CountingAllocator = CountingAllocator;`.

- **C15 (theory).** `bumbledb_theory::schema::check(&SchemaDescriptor) -> Result<Checked,
  SchemaError>` decides a declaration with no storage; the first failure in check order (relations
  in declaration order, then statements in materialized order) is the error. `SchemaError`,
  `StatementErrorKind`, `Mismatch`, `RowIndex`, `TargetKeyCandidate` (its `key` is now a
  `StatementId`) live in `bumbledb_theory::schema` and are re-exported from `bumbledb::error`.
  `Checked { relations: [CheckedRelation { name, fields, rows }], statements: [CheckedStatement] }`
  with `CheckedStatement::{Key, Containment { resolution, mirror }, Capacity { resolution }}`;
  `MemberSet`, `SealedWeight`, `SealedBound`, `mirror_links`, `MAX_FIXED_BYTES` moved there too.
  `SchemaDescriptor::validate()` is `check` then seal. `StatementErrorKind::DeterminantKeyTooWide`
  is gone (routing is at most 16 bytes by layout).
- **C5 stage A (closed rows).** `Schema::closed_rows(RelationId) -> Option<&[SealedRow]>` (also
  `RelationBody::closed_rows()`); `SealedRow { handle: Box<str>, values: Box<[Value]>, row:
  CanonicalRow, fact }`: `values` in sealed field order with `values[0] == Value::U64(index)`,
  `row.as_bytes()` is the stored-row codec. `fact` is the old fixed-width codec and goes in stage B.
- Schema fingerprint label `bdb.schema.v1`; every fingerprint wire tag is dense from 1.

- **C8 stage B.** `StoreError`, `StoreResult` and `Error::Store` are gone. Storage returns the one
  `Error`. New flat variants `Error::Cancelled` and `Error::Allocation` (`kind()` is
  `ErrorKind::Cancelled` / `ErrorKind::Allocation`); `From<WorkError>`, `From<ChangeError>` and
  `From<RowError>` map cancellation and allocation inside them to those variants.
  `bumbledb::Result<T, E = Error>` takes an optional error type. `HostKeyFault` is defined in
  `bumbledb::error`.
- **C5 stage B.** `SealedRow { handle, values, row }` (no `fact`); `Relation::layout()`,
  `encoding::{FactLayout, FactView, ValueRef, append_field, encode_literal, field_bytes,
  interval_words, split_halves, decode_values_keyed_into, decode_bool, decode_fixed_interval_start}`
  and `canonical::decode_sealed` are deleted; `encoding` keeps the scalar order words
  (`encode_{bool,u64,i64,f64}`, `decode_{i64,f64}`), `FixedBytesValue`, `fixed_bytes_words`,
  `InternId`. `bumbledb::__private` keeps only `fixed_interval_{u64,i64}`.
  `canonical::append_value` is the one value codec (stored rows and fingerprint literals).
- **C15 (macro).** `schema!` calls `bumbledb_theory::schema::check` after lowering and reports
  the error at the tokens it names. `SchemaError::named(&descriptor)` renders relations, fields
  and rows by their declared names (`` `Task.kind` ``, `` row `Frozen` of `Status` ``); `Display`
  still names them by id. `StatementErrorKind::CapacityDimensionMixing` gained `relation`.

## Requests to other lanes

- **engine-query (clippy after `a79aab954`, your file):** `api/prepared/source.rs:28`
  `work_error` is dead now that `decode_sealed` is gone (delete it; `Error::from(work)` is the
  conversion), and `source.rs:251` is `projection.count_bounded(&projected, limit, work)` without
  `Ok(..?)` (storage returns `Error` directly). These are the only `-D warnings` findings in
  `-p bumbledb --all-targets` at that commit.

- **consolidator (bridge, optional):** `bumbledb-node/src/schema.rs::schema_diagnostic` has the
  descriptor at hand; `error.named(descriptor).to_string()` gives the message with declared names
  instead of ids (the statement is cited separately already).
