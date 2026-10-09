# engine-storage lane board

Owns: `crates/bumbledb/{Cargo.toml, src/lib.rs, error*, storage*, schema*, changes*, canonical*,
encoding*, work*, digest.rs, verify_store*, alloc_counter.rs, value.rs, interval*, allen.rs, api.rs,
api/db*}`, the engine integration tests not owned by other lanes, `crates/bumbledb-theory/**`.

Items: A (pointwise sweep), C1, C4 (judge side), C5 (storage side), C6, C7, C8, C9, C10, C15, C16,
C17, G2 (engine), L (code).

## Status

| Item | Status |
|---|---|
| `Error::Capacity` (engine-query request 1) | landed |
| `testing` feature (engine-query request 4) | landed (`ground-off`, `collision-probe` still declared) |
| C4 judge side: grouped maps RAM-only, drop every `exec::scratch` use | landed |
| G2: one allocation counter, `alloc_census.rs` deleted | landed (feature still declared, see below) |
| C17: fixed virtual map | landed |
| C7: `bumbledb::host`, visibility cutover | in progress |
| C8, C9, C10, C1, C6, A, C5, C15, C16, C17, L | todo |

## API changes (announcements)

### Landed

- `bumbledb::Error::Capacity(bumbledb::Capacity)` with
  `pub enum Capacity { ResidentRows, DistinctRows, Groups, ResultBytes }` (`Copy`, `Eq`).
  `Error::ResultBytesOverflow` goes away in C8: use `Error::Capacity(Capacity::ResultBytes)`.
- Feature `testing = []` on `bumbledb`. It replaces `ground-off` and `collision-probe`; both are
  deleted once nothing names them. engine-query: switch `plan/ground.rs` to
  `#[cfg(any(test, feature = "testing"))]`. bench: depend on `bumbledb` with
  `features = ["testing"]` in `[dev-dependencies]` only.

- **C4 judge side.** `schema::judge` no longer names `exec::scratch`; `work.rs`/`lib.rs` no longer
  re-export `Scratch*`, and `verify.rs` uses a RAM set. HEAD names `exec::scratch` nowhere outside
  engine-query's files. `JudgeScratch`, `ScratchFault`, `store_fault`,
  `judge_final_state_with_scratch` and `JudgeError::Allocation` are deleted; `judge_incremental`
  lost its scratch argument. engine-query: `exec/scratch.rs` now warns (unused `ScratchWideClaimKey`
  import, dead methods) until you delete the module.
- **G2.** `alloc_counter` is always compiled; the lib's unit tests register it with
  `#[cfg(test)] #[global_allocator]`; integration tests register it themselves. `tests/alloc_census.rs`
  is deleted. Every `#[cfg(feature = "alloc-counter")]` in my files is gone. The `alloc-counter`
  feature stays declared in `Cargo.toml` only until no other file names it: engine-query (image/,
  exec/, plan/ tests) and numeric, please drop yours; bench drops `features = ["alloc-counter"]`.

- **C17 fixed map.** `bumbledb::Options { map_ceiling: u64 }` (`Default`: 1 TiB) with
  `Db::create_with(path, schema, Options, work)` and `Db::open_with(path, schema, Options, work)`;
  `Db::create`/`Db::open` use the default. A write past the ceiling is
  `Error::Full { ceiling: u64 }` (`ErrorFamily::Full` until C8's `kind()`), nothing committed.
  Deleted: `store::map`, `MapPolicy`, `MapReport`, `GrowReport`, `Store::grow`,
  `Store::current_map_bytes`, `Store::map_report`, `StoreError::{MapFull, MapGrowthExhausted,
  ResizeBlockedByReaders}`, the gate's parked-reader cache. `Db::disk_size(&self) -> Result<u64>`
  (no work argument). Verified on this Mac: a 1 TiB map leaves a 10k-row `data.mdb` at 1.26 MB.

### Planned (signatures may still move; final shapes are announced under "Landed")

- **`alloc-counter` feature is deleted (G2).** `bumbledb::alloc_counter` is always compiled
  (`CountingAllocator`, `snapshot()`, `reset()`, `count()`, `dealloc_count()`). The engine's own
  unit-test binary registers the counter with `#[cfg(test)] #[global_allocator]`. An integration
  test or another crate's test that measures allocations registers it itself:
  `#[global_allocator] static A: bumbledb::alloc_counter::CountingAllocator = CountingAllocator;`.
  Replace every `#[cfg(feature = "alloc-counter")]` in your files with nothing (unit tests) or a
  local registration (integration tests). Exact nonzero equalities become `<=` budgets or
  structural assertions; `== 0` gates stay. I delete the feature from `Cargo.toml` once no file
  names it.
- **C7 visibility.** `bumbledb::store`, `bumbledb::integration`, `schema::judge`,
  `work::Scratch*`, `verify_store`, `Db::integration_*` and the hidden `*_accepted` verbs leave the
  public surface. `#![deny(unreachable_pub)]` is applied module by module: please make items in
  your `pub(crate)` modules `pub(crate)` (or private) so the crate-wide deny can land at the end.
- **`bumbledb::host` (C7, for log-core and bridge).** The sans-IO log's view of the engine:
  - `Db::<S>::create_with_identity(path, schema, database_id: [u8; 16], work) -> Result<WriteOutcome<Db<S>>>`
    and `Db::<S>::database_id() -> [u8; 16]` (identity comes from the log's Genesis, C16).
  - `Db::<S>::host_writer(&self, work: &WorkContext) -> Result<host::WriterSession<'_, S>>`.
  - `WriterSession::generation(&self) -> Result<GenerationId>`.
  - `WriterSession::decide(&mut self, changes: &ChangeSet) -> Result<host::Decision<'_, 'db, S>>`
    where `Decision = Accepted(host::Prepared) | Rejected(Violations)`: judged private candidate.
  - `WriterSession::apply_decided(&mut self, changes: &ChangeSet) -> Result<host::Prepared<'_, 'db, S>>`:
    applies an already-decided ChangeSet (catch-up), no judgment.
  - `WriterSession::unchanged(&mut self) -> Result<host::Prepared<'_, 'db, S>>`: host-only txn.
  - `Prepared::{applied(&self) -> host::Applied, seal(self, host::HostChanges) -> Result<host::Sealed>, abort(self)}`.
  - `Sealed::{commit(self) -> Result<host::Commit>, abort(self)}`;
    `Commit { generation: GenerationId, applied: Applied { added: u64, removed: u64 }, changed: bool }`.
  - `HostChanges<'a> { records: &'a [HostRecord<'a>], head: Head<'a> }`,
    `HostRecord::{Put { key, value }, Delete { key }}` (keys strictly increasing, at most
    `host::MAX_KEY` bytes), `Head::{Keep, Put(&[u8]), Clear}`.
  - Reads on `ReadFrame`/`OwnedRead`: `host_record(key) -> Result<Option<&[u8]>>`,
    `host_scan(prefix, &mut dyn FnMut(&[u8], &[u8]) -> Result<()>) -> Result<()>`,
    `head() -> Result<Option<&[u8]>>`, `content_digest() -> Result<[u8; 32]>` (canonical rows only,
    deterministic across platforms), `export(&mut dyn FnMut(RelationId, &[u8]) -> Result<()>)`.
  - Images: `Db::compact(&self, dest: &Path, work) -> Result<()>` writes a compacted image
    directory; `host::StagedImage::begin(dest: &Path) -> Result<StagedImage>`,
    `StagedImage::data_path(&self) -> &Path` (write the downloaded image there),
    `StagedImage::install::<S>(self, schema: S, digest: [u8; 32], work) -> Result<Db<S>>`
    (verifies layout, schema, identity and content digest, then renames into `dest`).
  - Stamps: `host::Digest` (blake3 streaming) and `host::LAYOUT: u32`.
- **C8 one `Error`.** `StoreError`, `IntegrationError`, `HostSealError`, `JudgeError`,
  `ScratchFault`, `WorkError` wrapping fold into `bumbledb::Error`; `ErrorFamily` becomes
  `Error::kind() -> ErrorKind`. New: `Error::Full { ceiling: u64 }` (C17), `Error::Cancelled`,
  `Error::NotABumbleDb { path }`. `ValidationError` stays engine-query's (re-exported).
- **C9 one `WriteOutcome<R>`**:
  `Committed { value: R, generation: GenerationId, changed: bool } | Rejected(Violations) | Moved { witnessed, current }`.
  It replaces `Admission<Committed<R>>`, `ConditionalWrite`, `ApplyOutcome`, `CoreCommit`.
- **C5 closed rows.** Closed relations carry canonical rows (`schema::SealedRow::row` is canonical
  row bytes, same codec as stored rows); `FactLayout`/`FactView` are deleted from `encoding`.

## Requests to other lanes

- engine-query: see the `alloc-counter` and `unreachable_pub` notes above for your files.
- numeric: same `alloc-counter` note; I remove `#![feature(portable_simd)]` from `lib.rs` when your
  board says the fearless_simd port has landed.
- bench, bridge, log-core: `bumbledb::store::*` and `bumbledb::integration::*` disappear in C7;
  use `bumbledb::host` (above). Tell me here if something you need is missing from it.
