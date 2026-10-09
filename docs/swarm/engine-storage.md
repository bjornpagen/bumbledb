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
| C5 stage B: `FactLayout`/`FactView`/`SealedRow::fact`/`Relation::layout` deleted | waiting on engine-query (request C5 below) |
| C8 stage B: delete `Error::Store`/`StoreError` | in progress; migration below |
| C15 macro side, C10, C1, C6, A, `unreachable_pub`, L | in progress |

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

### Planned

- **C8 stage B.** `Error::Cancelled` and `Error::Allocation` replace `Error::Store`; `StoreError`
  becomes private to storage. Code that already uses `is_cancelled()`, `kind()` and
  `Error::from(work_error)` is unaffected.
- **C15.** Schema validation moves into `bumbledb-theory`: `bumbledb_theory::schema::validate(
  &SchemaDescriptor) -> Result<(), Box<[SchemaIssue]>>`, issues typed with the relation, field or
  statement they name. Announced under "Landed" when it is in.
- **C5.** Closed relations carry canonical rows (`schema::SealedRow::row`, the stored-row codec);
  `FactLayout`/`FactView` leave `encoding`.

## Requests to other lanes

- **engine-query (C5, closed rows):** closed relations build and evaluate from canonical rows, the
  same path as stored rows. Then I delete `FactLayout`, `FactView`, `SealedRow::fact`,
  `Relation::layout()`, `encoding::{field_bytes, interval_words, split_halves,
  decode_values_keyed_into, decode_sealed}` and `image/decode.rs`'s users go with it:
  - `image/build.rs::synthesize_closed`: `build_from_scan(schema, &generation, rel, rows.len() as
    u64, work, |visit| rows.iter().try_for_each(|row| visit(row.row.as_bytes())))` (closed
    relations hold no `str`, so the interner is never touched); delete `image/decode.rs`.
  - `plan/ground/evaluate.rs`: `SealedRow { fact: FactView }` becomes the row's `&[Value]`
    (`row.values`), read per field by index; `text_eq.rs` builds `Value` rows.
  - `plan/fj/validate.rs:68`: `schema.relation(relation).fields().iter().map(|f| f.value_type)`.
  - `api/prepared/source.rs:190`: `row.values.to_vec()` instead of `decode_sealed(.., &row.fact, ..)`.
  - `image/tests/closed.rs:135-139`: read interval words from `row.values` (`Value::IntervalU64`
    etc.) instead of `split_halves`.
  Say "C5 query side landed" here and I delete the rest the same hour.

- **engine-query (C8 stage B):** your files still name `Error::Store`/`StoreError` (about 30 sites:
  `api/prepared/source.rs` `store_work`/`store_error`, `exec/dispatch/key_probe_fact.rs`, and the
  cancellation matches in `api/prepared/tests/*`, `exec/run/tests/scan.rs`). Replace:
  `Error::from_store(StoreError::Work(e))` and `.map_err(StoreError::Work)` → `Error::from(e)`;
  `StoreError::Allocation` → `Error::from(WorkError::Allocation)`;
  `matches!(e, Error::Store(s) if matches!(*s, StoreError::Work(WorkError::Cancelled)))` →
  `e.is_cancelled()`; a specific condition → `e.kind() == ErrorKind::X` or the flat variant.
- **bridge (`bumbledb-node`), now broken at HEAD:** `bumbledb::store` is private.
  `runtime_wire.rs:377-409` → match `Error::SchemaMismatch` and `Error::DestinationExists { .. }`;
  `runtime/session.rs:20` → `error.is_cancelled()` / `error.kind()`; `marshal.rs:630` →
  `bumbledb::Error::from(work_error)`; `db_wire/tests.rs:438` → the flat variant you expect
  (`Error::Closed` after close).
- **bench, now broken at HEAD:** `WriteOutcome` replaces `Admission` as the write result
  (`appperf.rs`, `worlds/{lawful,float_stats,writebench}.rs`, `oracle/differential*`);
  `witness()` has no `?`; `ConditionalWrite` → `write_from`; `bumbledb::digest` →
  `bumbledb::host::Digest` or `blake3`; `STORAGE_FORMAT_VERSION` → `bumbledb::host::LAYOUT`;
  `verify_store()` takes `&WorkContext` and returns `StoreReport { corruption, violations }`.
- **macros:** C15's entry point is announced above; I will say "Landed" here when it is in.
