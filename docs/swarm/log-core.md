# log-core lane board

Owns: `crates/bumbledb-log/**`, `docs/swarm/log-core.md`.

## Status

| Item | State |
|---|---|
| Delete the old log machine, its tests, conformance fixtures, bins, the bench dev-dependency, object_store/tokio/futures/once_cell_try | done |
| Core types, one frame codec, `Command`, `Entry`, `Receipt`, `Head`, `fold` | landed |
| Sans-IO `Machine` (U11) + seeded fault-injection simulation over a reference replica | landed |
| LMDB `Cache` (the product `Replica`) over the engine | landed on today's `bumbledb::integration`; moves to `bumbledb::host` when it lands |
| Checkpoints (policy, images, cold open, pruning, digest-verified install) | landed |
| Migrations (D6/D7): ledger, four-way comparison, Freeze/Migration/Thaw, population, sticky rejection, `SchemaAdvanced` | landed |
| D20 `bdb` names: frames `bdb.<kind>.v1`, images `*.bdb`, cache generations `<n>.bdb` | landed |

The bridge and the swarm gate: the engine at HEAD currently warns in `exec/scratch.rs`
(engine-query), so this lane lints with `cargo clippy -p bumbledb-log --no-deps`.

## Public API (`bumbledb_log::*`, Rust)

Everything is re-exported at the crate root. The core does no I/O: no tokio, object_store or
credentials. Dependencies: `bumbledb`, `blake3`.

### Identities and coordinates

```rust
pub struct DatabaseId(pub [u8; 16]);   // minted by Genesis (the caller supplies it: Config::create)
pub struct RequestId(pub [u8; 16]);    // idempotency key: decided at most once
pub struct Nonce(pub [u8; 16]);        // per written entry (derived from Config::seed)
pub struct CommandDigest(pub [u8; 32]);
pub struct ImageDigest(pub [u8; 32]);  // blake3 of an image file
pub struct MigrationHash(pub [u8; 32]);
pub struct Seq(NonZeroU64);            // log/{seq}; Seq::GENESIS == 1; Seq::new(u64) -> Option, .get()
pub struct Revision(pub u64);          // committed state changes; preconditions name one
pub struct Millis(pub u64);            // unix ms, ONLY from the store's Date / Last-Modified
```

### Commands, outcomes, receipts

```rust
pub enum Precondition { None, ExactRevision(Revision) }
pub struct CommandRef { pub request: RequestId, pub digest: CommandDigest }
impl Command {
    pub fn seal(request: RequestId, precondition: Precondition, changes: bumbledb::ChangeSet) -> Self;
    pub fn parse(schema: &Schema, bytes: &[u8]) -> Result<Self, FrameError>;
    pub fn encode(&self) -> Vec<u8>;
    pub fn reference(&self) -> CommandRef;   // + request(), precondition(), changes()
}
pub struct Delta { .. }                    // Delta::new(added, removed) -> Option (never both 0); added(), removed()
pub struct Evidence(..);                   // engine canonical violation bytes, never empty; as_bytes()
pub enum Outcome {
    Committed(Delta), NoChange,
    PreconditionFailed { expected: Revision, observed: Revision },
    InvariantRejected(Evidence),
}
pub struct Receipt { pub command: CommandRef, pub seq: Seq, pub revision: Revision, pub outcome: Outcome }
```

`receipt.seq` is the read-your-writes bookmark: a later read is current once the head is >= it.

### The machine (U11)

```rust
pub struct Machine<R: Replica>;
impl<R: Replica> Machine<R> {
    pub fn new(replica: R, config: Config) -> Self;
    pub fn step(&mut self, input: Input) -> Step;
    pub fn replica(&self) -> &R;            // reads (queries) go through the replica between steps
}
pub struct Config {
    pub seed: [u8; 16],                      // random per process
    pub create: Option<DatabaseId>,          // create when log/1 is missing (random id from the caller)
    pub probe_window: u32,                   // parallel tail GETs while catching up (e.g. 32)
    pub checkpoint: CheckpointPolicy { every: u64, keep: u32 },
    pub max_entry_bytes: usize,
}
pub struct Ticket(pub u64);                  // caller-chosen; every ticket settles exactly once
pub enum Input {
    Open(Ticket),                            // hydrate + catch up -> Opened { pending }
    Submit(Ticket, Command),                 // -> Decided(receipt) | Refused(..)
    Sync(Ticket),                            // "latest": -> Synced(seq)
    Resolve(Ticket, RequestId),              // sync, then -> Resolved(Option<Receipt>)
    Freeze(Ticket, u64 /* lease ms */),      // freeze for the next pending migration -> Frozen(seq)
    Migrate(Ticket, Population),             // apply the next pending migration -> Migrated(seq)
    Response(IoResponse),
    Close,                                   // settle everything; later inputs -> Refused(Closed)
}
pub struct Step { pub io: Vec<IoRequest>, pub done: Vec<Done> }
pub struct Done { pub ticket: Ticket, pub settled: Settled }
pub enum Settled {
    Opened { pending: usize },               // pending bundled migrations (0 = current)
    Decided(Receipt), Synced(Seq), Resolved(Option<Receipt>),
    Frozen(Seq), Migrated(Seq),
    Refused(Refusal),
}
pub enum Refusal {
    NotOpen, Closed, NotFound,
    Frozen { deadline: Millis },             // retry after deadline (store time)
    SchemaAdvanced,                          // the log applied a migration this code lacks
    MigrationPending { next: usize },        // writes wait for migrations (bundle index)
    MigrationRejected(Rejection),            // sticky: this bundle carries a rejected migration
    MigrationsDiverged { index: usize },
    NotPending,                              // Freeze/Migrate for a step that is not next
    Stale { head: Seq },                     // recompute the population at `head` (or freeze first)
    RequestReused(CommandRef),               // request id already decided for another command
    ForeignSchema,                           // the command's changes are for another schema than the head's
    TooLarge,
    Unknown,                                 // written, fate unresolved (closed/broke mid-flight): Resolve later
    NotSubmitted,
    Cache(CacheError),
    Corrupt(Seq),
}
```

Every `Refused` of a `Submit` except `Unknown` proves the command is not in the log.

### I/O requests (executed by TS, concurrently)

```rust
pub struct IoId(pub u64);
pub enum Bucket { Log /* S3 Express: log/ */, Checkpoints /* S3 Standard: ckpt/, mig/ */ }
pub struct IoRequest { pub id: IoId, pub bucket: Bucket, pub key: String, pub op: Op }
pub enum Op {
    Get(Target),                             // Target::Memory | Target::File(PathBuf): download into the file
    PutIfAbsent(IoBody),                     // If-None-Match: *; IoBody::Bytes(Vec<u8>) | IoBody::File(PathBuf)
    List { start_after: Option<String>, max_keys: u32 },   // key is the prefix; lexicographic (Checkpoints only)
    Delete,                                  // Checkpoints only; never log/
}
pub struct IoResponse { pub id: IoId, pub date: Option<Millis> /* Date header */, pub result: IoResult }
pub enum IoResult {
    Body { bytes: Vec<u8>, last_modified: Millis },   // Get(Memory) 200
    Saved { last_modified: Millis },                  // Get(File) 200
    Missing,                                          // Get 404
    Created,                                          // PutIfAbsent 200
    Occupied,                                         // PutIfAbsent 412
    Keys(Vec<String>),                                // List (keys relative, same form as requested)
    Deleted,
    Failed,                                           // anything else after TS's own retries of reads:
                                                      // 409, 5xx, timeout, transport. Never retry a PUT in TS:
                                                      // report Failed and the machine reads back and re-PUTs.
}
```

Keys are relative (`log/00000000000000000042`, `ckpt/…`, `mig/…`); TS prepends its prefix. TS may
retry GET/LIST/DELETE itself and must report a PUT's first non-200 as `Occupied` (412) or
`Failed`.

### The cache

```rust
pub type CacheDb = bumbledb::Db<SchemaDescriptor>;
impl Cache {
    pub fn open(root: &Path, bundle: Bundle) -> Result<Cache, CacheError>;  // discards anything unusable
    pub fn db(&self) -> Option<&Arc<CacheDb>>;   // reads; replaced when an image installs
}
impl Replica for Cache { .. }
```

Layout under `root`: `CURRENT` (`"<generation> <schema hex>"`), the live `<generation>.bdb/`
LMDB directory, and scratch `incoming.bdb`, `outgoing.bdb/`, `migration.bdb/`, `stage.bdb/`.
Receipts are host records `r‖request id`; the head is the attachment. Create, image install and
migration build a new generation and swap `CURRENT`. When a machine is replaced, abandon the old
machine's outstanding requests (a stale download could land in `incoming.bdb`).

### Replica and migrations

```rust
pub trait Replica {                          // the product impl is bumbledb_log::Cache (LMDB)
    fn bundle(&self) -> &Bundle;
    fn head(&self) -> Option<&Head>;
    fn receipt(&self, request: RequestId) -> Result<Option<Receipt>, CacheError>;
    fn judge(&self, accepted: &[ChangeSet], next: &ChangeSet) -> Result<Judgment, CacheError>;
    fn apply(&mut self, update: Update<'_>) -> Result<(), CacheError>;
    fn create(&mut self, head: &Head) -> Result<(), CacheError>;
    fn install(&mut self, path: &Path, digest: ImageDigest, schema: SchemaFingerprint) -> Result<(), CacheError>;
    fn image(&mut self) -> Result<Image, CacheError>;
    fn download_path(&self) -> PathBuf;
    fn migrate(&mut self, population: &Population, head: &Head) -> Result<Migrated, CacheError>;
}
pub struct MigrationId { pub name: Box<str>, pub hash: MigrationHash }
impl Bundle { pub fn new(steps: Vec<(MigrationId, SchemaDescriptor)>) -> Result<Self, BundleError>; }
// bundle[0] is the initial schema; a new database starts there and migrates forward.
pub struct Population {
    pub step: MigrationId,                   // must be the next pending migration
    pub base: Seq,                           // the head the population was computed at
    pub copy: Box<[(RelationId /* new */, RelationId /* old */)]>,   // copyUnchanged
    pub rows: ChangeSet,                     // computed rows at the new schema
}
pub enum Comparison { Equal, Behind { next: usize }, Ahead, Diverged { index: usize } }
```

Migration driver (TS), per pending step: compute the population from the cache at head `h`, send
`Migrate`; on `Refused(Stale)` either retry or send `Freeze(lease)` first, then recompute. A
rejected population is recorded in the log (`Thaw::Rejected`) and every later open of a bundle
carrying it settles `Refused(MigrationRejected)`.

## Requests

### to engine-storage (`bumbledb::host`)

The log needs, beyond the planned `host` surface:

- R-E1 **batch judgment**: `WriterSession::decide_all(&mut self, changes: &[ChangeSet]) ->
  Result<Vec<host::Judged>>` with `Judged = Accepted(Applied) | Rejected(Violations)`. Each change
  set is judged against the parent plus the earlier accepted ones, in order (a rejected one rolls
  back alone, e.g. a nested txn); `Applied` is that change set's own net `{added, removed}`. Nothing
  commits. This is group commit: one entry decides many commands in sequence.
- R-E2 **ordered unjudged apply**: `WriterSession::apply_decided(&mut self, changes: &[ChangeSet])
  -> Result<Prepared>` applying the sets in order, with `Prepared::applied_each() -> &[Applied]`
  so the log can check each against its recorded delta.
- R-E3 **cache durability (U4)**: open/create a cache environment with `MDB_NOSYNC` (no
  `WRITEMAP`), e.g. `host::Durability::{Durable, Cache}` on create/open. Without it the log's
  simulation tests must avoid the real cache (every commit fsyncs).
- R-E4 **population for migrations**: build a database at a new schema in a staging directory:
  copy every row of relation `old` from an `OwnedRead` of the old database into relation `new`,
  apply a `ChangeSet` of computed rows, judge the whole state (admission → `Rejected(Violations)`),
  then seal host records + head and install or compact. The existing `UnreadyStore` staging path is
  close; please keep a public route to it under `host`.

Until these land the cache uses today's `bumbledb::integration` writer (one candidate per entry,
over the ordered composition of the entry's change sets).

### to bridge

- Drive `Machine<Cache>` from napi: one machine per open database; `step` per input; hand
  `Step.io` to TS and feed back `IoResponse`s. Reads use `machine.replica()` (the cache's `Db`).
