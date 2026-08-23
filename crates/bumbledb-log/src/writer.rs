//! The writer: a replica plus the right to create log objects. One
//! commit discipline for both ack modes over the shared pending slot;
//! the loser algebra between the local store and the bucket, governed
//! by one law — a local commit survives in place iff it maps to its own
//! slot. The publish law holds at the only place it could break: a
//! batch reaches the network only if its local application advanced the
//! generation, so every log slot is a state-changing commit and the
//! wholeness identity stays one integer compare.
//!
//! A fully key-disjoint loss takes the republish-without-re-judgment
//! fast path: the winner applies in place (L8 — winner-over-ours equals
//! ours-over-winner), any further occupied slots replay under the same
//! pairwise tests, and the recorded ops republish under a re-addressed
//! header with ops, footprint, and verdict untouched. L7's acceptance
//! form licenses exactly this — the loser algebra only ever republishes
//! a batch its own store accepted, which is the hypothesis L7 carries
//! (its rejected arm is refuted in `lean/Bumbledb/Countermodels`, and
//! nothing here rests on it).

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, TryLockError};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

use bumbledb::schema::{
    FieldId, RelationId, SchemaDescriptor, StatementDescriptor, StatementId, ValueType, Weight,
};
use bumbledb::{Admission, Db, Theory, Value, Violations};

use crate::apply::{Applied, apply};
use crate::braids::BraidId;
use crate::codec::{BatchHeader, ByteSink, Codec, EncodeError, Op, OpKind, append_value};
use crate::footprint::{Entry, FootprintError, footprint};
use crate::intersect::{ConflictCause, LoserDecision, intersect};
use crate::lease::{LeaseRefusal, Leased, Leases};
use crate::manifest::{
    Checkpoint, Head, Manifest, Published, ckpt_json_key, ckpt_mdb_key, create_manifest, log_key,
    manifest_key, publish_checkpoint,
};
use crate::replica::{
    Corruption, Fault, OpenRefusal, Vector, derive_codec, fetch_checkpoint_bytes,
    write_checkpoint_bytes,
};
use crate::sidecar::{Chain, ChainEntry, Pending};
use crate::store::{Create, CreateProbe, ObjectStore, resolve_ambiguous_create, retry_read};

/// Consecutive live losses at the tip before `Err::Contention`; history
/// losses never count.
pub const LOSS_BOUND: u32 = 16;

/// One drain packs at most this many host writes into one batch.
pub const DRAIN_MAX_WRITES: u64 = 512;

/// One drain packs at most this many estimated batch bytes.
pub const DRAIN_MAX_BYTES: u64 = 4 * 1024 * 1024;

/// Checkpoint cadence, vector-sum arm: a checkpoint after this many
/// applied batches since the current one.
pub const CHECKPOINT_EVERY_SUM: u64 = 256;

/// Checkpoint cadence, log-volume arm.
pub const CHECKPOINT_EVERY_BYTES: u64 = 16 * 1024 * 1024;

const DATA_FILE: &str = "data.mdb";

/// The ack's durability, part of the outcome value: `Published` is
/// RPO at zero; `LocalPending` is RPO at publish lag, and the commit can
/// still be lost to a crash or rejected by a conflict loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    Published,
    LocalPending,
}

/// A commit's outcome. `generation` is the braid generation — the slot
/// number, exactly what a `wait_for` session vector carries — never the
/// store-wide sum.
#[derive(Debug)]
pub enum Commit<R> {
    Accepted {
        value: R,
        braid: BraidId,
        generation: u64,
        durability: Durability,
    },
    Rejected(Violations),
}

/// One braid's outcome inside a `commit_split`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BraidOutcome {
    Accepted {
        braid: BraidId,
        generation: u64,
        durability: Durability,
    },
    Rejected {
        braid: BraidId,
        violations: Violations,
    },
}

/// The ack mode: `Published` acks after the slot exists; `Local` moves
/// the ack to the end of the local apply, bounded by the `max_pending`
/// knobs — beyond them the ack stalls to publication instead of letting
/// the loss window grow silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckMode {
    Published,
    Local {
        max_pending_batches: u64,
        max_pending_bytes: u64,
    },
}

/// Writer construction knobs. `linger` delays a drain's queue snapshot
/// so concurrent commits pack; default zero — batch whatever is queued,
/// never wait.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub writer_id: u64,
    pub ack: AckMode,
    pub linger: Duration,
}

impl Options {
    #[must_use]
    pub const fn new(writer_id: u64) -> Self {
        Self {
            writer_id,
            ack: AckMode::Published,
            linger: Duration::ZERO,
        }
    }
}

/// The commit discipline's observable steps, in execution order — the
/// fault-injection seam the conformance crash matrices drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterStep {
    Encode,
    PendingWrite,
    ApplyLocal,
    AckLocal,
    PutLog,
    ChainAdvance,
    PendingClear,
}

/// What the step hook decides after a step completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepControl {
    Continue,
    /// Abort as a crash would: state stays exactly as the step left it,
    /// and the writer is done — reopen the directory to run recovery.
    Crash,
}

/// The fault-injection seam: observes each completed step and may
/// simulate a crash there.
pub trait StepHook: Send + Sync {
    fn observe(&self, step: WriterStep) -> StepControl;
}

/// The production hook: never crashes.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFaults;

impl StepHook for NoFaults {
    fn observe(&self, _step: WriterStep) -> StepControl {
        StepControl::Continue
    }
}

/// Loser-algebra instrumentation, cumulative since open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counters {
    pub re_judgments: u64,
    pub republishes: u64,
    pub subsumptions: u64,
    pub disjoint_verdicts: u64,
}

#[derive(Default)]
struct CounterCells {
    re_judgments: AtomicU64,
    republishes: AtomicU64,
    subsumptions: AtomicU64,
    disjoint_verdicts: AtomicU64,
}

/// What exhausted the contention bound. `HotKey` carries the statement
/// and the loser's own raw determinant values — an operable handle, not
/// a hash; the statement is absent exactly when the terminal conflicts
/// were bare fact-identity races, which name no statement. `SlotRace`
/// carries the tip when fully-disjoint racers out-ran us.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentionCause {
    HotKey {
        statement: Option<StatementId>,
        values: Box<[Value]>,
    },
    SlotRace {
        tip: u64,
    },
}

/// The operational signal a resident writer surfaces when a
/// non-byte-equal `Exists` proves it was deposed: both writer ids, from
/// the batch headers. The response already happened — the loss finished
/// as an ordinary loser and acks dropped to `Published`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deposition {
    pub braid: BraidId,
    pub slot: u64,
    pub resident: u64,
    pub usurper: u64,
}

/// The writer's error sum. Infrastructure failures ride `Fault`; the
/// named arms are 60's own verbs.
#[derive(Debug)]
pub enum Error {
    Fault(Fault),
    /// A re-open inside the disposable law hit a manifest-gauntlet
    /// refusal — the store's declared truths changed under us.
    Refused(OpenRefusal),
    /// `commit` requires one braid; the recorded ops span these.
    SpanningCommit {
        braids: Box<[BraidId]>,
    },
    /// A body that recorded nothing has no braid to commit under.
    EmptyCommit,
    Encode(EncodeError),
    /// The recorded ops could not re-derive their footprint — our own
    /// encoder could never have produced them.
    Footprint(FootprintError),
    /// The live-loss bound: publication retries on the next commit; the
    /// applied batch stays in `pending`.
    Contention {
        braid: BraidId,
        cause: ContentionCause,
    },
    /// The braid is wedged read-only by a corruption-class verdict.
    Wedged {
        braid: BraidId,
    },
    Lease(LeaseRefusal),
    /// The statement is not a capacity whose source relation has the
    /// reservation shape: parent projection + weight field + one u64
    /// expiry field covering the layout.
    ReservationShape {
        statement: StatementId,
    },
    /// The step hook simulated a crash; reopen the directory to recover.
    InjectedCrash {
        step: WriterStep,
    },
    /// A packed neighbor's drain failed; the shared cause.
    Drain(Arc<Error>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fault(fault) => write!(f, "{fault}"),
            Self::Refused(refusal) => write!(f, "re-open refused: {refusal:?}"),
            Self::SpanningCommit { braids } => {
                write!(
                    f,
                    "commit spans braids {braids:?}; commit_split is the verb"
                )
            }
            Self::EmptyCommit => write!(f, "the body recorded no ops"),
            Self::Encode(error) => write!(f, "encode refused: {error:?}"),
            Self::Footprint(error) => write!(f, "footprint refused: {error:?}"),
            Self::Contention { braid, cause } => {
                write!(f, "contention on {braid}: {cause:?}")
            }
            Self::Wedged { braid } => write!(f, "braid {braid} is wedged"),
            Self::Lease(refusal) => write!(f, "id lease refused: {refusal:?}"),
            Self::ReservationShape { statement } => {
                write!(f, "statement {} has no reservation shape", statement.0)
            }
            Self::InjectedCrash { step } => write!(f, "injected crash after {step:?}"),
            Self::Drain(error) => write!(f, "drain failed: {error}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<Fault> for Error {
    fn from(fault: Fault) -> Self {
        Self::Fault(fault)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// The recorded transaction: typed inserts and deletes, id draws from
/// the lease, and the reservation sugar. Recording is pure — the engine
/// judges at apply, and the driver never re-invokes the body.
pub struct Batch<'w, S: ObjectStore> {
    ops: Vec<Op>,
    store: &'w S,
    prefix: &'w str,
    leases: &'w Mutex<Leases>,
    maps: &'w SchemaMaps,
}

impl<S: ObjectStore> Batch<'_, S> {
    pub fn insert<I: IntoIterator<Item = Box<[Value]>>>(&mut self, relation: RelationId, rows: I) {
        self.ops.push(Op {
            kind: OpKind::Insert,
            relation,
            rows: rows.into_iter().collect(),
        });
    }

    pub fn delete<I: IntoIterator<Item = Box<[Value]>>>(&mut self, relation: RelationId, rows: I) {
        self.ops.push(Op {
            kind: OpKind::Delete,
            relation,
            rows: rows.into_iter().collect(),
        });
    }

    /// Draws `count` fresh ids for `(relation, field)` from the lease;
    /// the resulting inserts carry the concrete values, and id
    /// reservations never appear in the log.
    pub fn reserve(
        &mut self,
        relation: RelationId,
        field: FieldId,
        count: u64,
    ) -> Result<Range<u64>> {
        let drawn = lock(self.leases)
            .draw(self.store, self.prefix, relation, field, count)
            .map_err(|err| Error::Fault(Fault::Store(err)))?;
        match drawn {
            Leased::Range(range) => Ok(range),
            Leased::Refused(refusal) => Err(Error::Lease(refusal)),
        }
    }

    /// Sugar over an ordinary insert into the declared reservation
    /// relation of `statement`: parent projection values, `units` into
    /// the weight field, `expiry` into the one leftover u64 field.
    /// Nothing here is special-cased — the row rides the log like any
    /// other, and the spend is an ordinary commit.
    pub fn reserve_capacity(
        &mut self,
        statement: StatementId,
        parent: &[Value],
        units: u64,
        expiry: u64,
    ) -> Result<()> {
        let Some(shape) = self.maps.reservations.get(&statement) else {
            return Err(Error::ReservationShape { statement });
        };
        if parent.len() != shape.projection.len() {
            return Err(Error::ReservationShape { statement });
        }
        let layout = &self.maps.layouts[&shape.relation];
        let mut row: Vec<Option<Value>> = vec![None; layout.len()];
        for (position, field) in shape.projection.iter().enumerate() {
            row[usize::from(*field)] = Some(parent[position].clone());
        }
        row[usize::from(shape.weight_field)] = Some(Value::U64(units));
        row[usize::from(shape.expiry_field)] = Some(Value::U64(expiry));
        let row: Box<[Value]> = row
            .into_iter()
            .map(|value| value.expect("the reservation shape covers every field"))
            .collect();
        self.insert(shape.relation, [row]);
        Ok(())
    }
}

/// Per-statement determinant projections: the sides whose rows
/// project to the statement's footprint key.
type DetSides = Vec<(RelationId, Box<[u16]>)>;

/// The current checkpoint pointer and its parsed document.
type Floor = Option<([u8; 32], Checkpoint)>;

/// One capacity statement's reservation shape, derived at open: the
/// source relation whose layout is exactly parent projection + weight
/// field + one leftover u64 field (the expiry).
struct ReservationShape {
    relation: RelationId,
    projection: Box<[u16]>,
    weight_field: u16,
    expiry_field: u16,
}

/// Descriptor-derived views the writer reads outside the codec: raw
/// layouts for key recomputation, per-statement determinant projections
/// for the `HotKey` extraction, and the reservation shapes.
struct SchemaMaps {
    layouts: BTreeMap<RelationId, Box<[ValueType]>>,
    dets: BTreeMap<StatementId, DetSides>,
    reservations: BTreeMap<StatementId, ReservationShape>,
}

fn schema_maps(descriptor: &SchemaDescriptor) -> SchemaMaps {
    let mut layouts: BTreeMap<RelationId, Box<[ValueType]>> = BTreeMap::new();
    for (index, relation) in descriptor.relations.iter().enumerate() {
        let id = RelationId(u32::try_from(index).expect("relation count fits u32"));
        layouts.insert(
            id,
            relation
                .fields
                .iter()
                .map(|field| field.value_type)
                .collect(),
        );
    }

    let mut dets: BTreeMap<StatementId, DetSides> = BTreeMap::new();
    let mut reservations: BTreeMap<StatementId, ReservationShape> = BTreeMap::new();
    let fields =
        |projection: &[FieldId]| -> Box<[u16]> { projection.iter().map(|field| field.0).collect() };
    for (index, statement) in descriptor.materialized_statements().iter().enumerate() {
        let id = StatementId(u16::try_from(index).expect("statement count fits u16"));
        match statement {
            StatementDescriptor::Functionality {
                relation,
                projection,
            } => {
                dets.insert(id, vec![(*relation, fields(projection))]);
            }
            StatementDescriptor::Containment { source, target } => {
                dets.insert(
                    id,
                    vec![
                        (source.relation, fields(&source.projection)),
                        (target.relation, fields(&target.projection)),
                    ],
                );
            }
            StatementDescriptor::Capacity {
                target,
                weight,
                source,
                ..
            } => {
                dets.insert(
                    id,
                    vec![
                        (source.relation, fields(&source.projection)),
                        (target.relation, fields(&target.projection)),
                    ],
                );
                let Weight::Field(weight_field) = weight else {
                    continue;
                };
                let Some(layout) = layouts.get(&source.relation) else {
                    continue;
                };
                let projection = fields(&source.projection);
                let named: Vec<u16> = projection.iter().copied().chain([weight_field.0]).collect();
                let leftovers: Vec<u16> = (0..u16::try_from(layout.len()).expect("field count"))
                    .filter(|field| !named.contains(field))
                    .collect();
                if let [expiry_field] = leftovers.as_slice()
                    && layout[usize::from(*expiry_field)] == ValueType::U64
                {
                    reservations.insert(
                        id,
                        ReservationShape {
                            relation: source.relation,
                            projection,
                            weight_field: weight_field.0,
                            expiry_field: *expiry_field,
                        },
                    );
                }
            }
        }
    }
    SchemaMaps {
        layouts,
        dets,
        reservations,
    }
}

#[derive(Clone)]
enum Resolved {
    Accepted {
        braid: BraidId,
        generation: u64,
        durability: Durability,
    },
    Rejected(Violations),
    Failed(Arc<Error>),
}

struct Waiter {
    slot: Mutex<Option<Resolved>>,
    cv: Condvar,
}

impl Waiter {
    fn new() -> Self {
        Self {
            slot: Mutex::new(None),
            cv: Condvar::new(),
        }
    }

    fn resolve(&self, resolved: Resolved) {
        let mut slot = lock(&self.slot);
        if slot.is_none() {
            *slot = Some(resolved);
            self.cv.notify_all();
        }
    }

    fn get(&self) -> Option<Resolved> {
        lock(&self.slot).clone()
    }

    fn wait_briefly(&self) {
        let slot = lock(&self.slot);
        if slot.is_none() {
            let _ = self
                .cv
                .wait_timeout(slot, Duration::from_millis(1))
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

struct Request {
    ops: Vec<Op>,
    rows: u64,
    bytes: u64,
    waiter: Arc<Waiter>,
}

struct Core<T: Theory + Clone> {
    db: Option<Db<T>>,
    chain: Chain,
    floor: Floor,
    wedged: BTreeMap<BraidId, Corruption>,
    ack: AckMode,
    deposition: Option<Deposition>,
    ckpt_sum: u64,
    log_bytes: u64,
    cadence_sum: u64,
    cadence_bytes: u64,
    duty_busy: bool,
}

impl<T: Theory + Clone> Core<T> {
    fn db(&self) -> &Db<T> {
        self.db
            .as_ref()
            .expect("an established writer holds a store")
    }

    fn generation(&self) -> std::result::Result<u64, Fault> {
        Ok(self.db().generation().map_err(Fault::Engine)?.value())
    }
}

struct Live {
    losses: u32,
}

enum Settled {
    Accepted {
        generation: u64,
    },
    Rejected(Violations),
    /// Waiters were acked `LocalPending`; publication continues on the
    /// detached publisher, keyed by the pending bytes.
    Detached {
        bytes: Vec<u8>,
    },
}

enum PublishEnd {
    Done(Settled),
    /// The loss discarded and re-established; the caller re-runs the
    /// discipline — the conflict arm's re-judgment of the recorded ops.
    ReJudge,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

struct CountSink(u64);

impl ByteSink for CountSink {
    fn put(&mut self, bytes: &[u8]) {
        self.0 += bytes.len() as u64;
    }
}

/// Outcome of `Writer::open`.
pub enum WriterOpened<T: Theory + Clone, S: ObjectStore, H: StepHook = NoFaults> {
    Ready(Writer<T, S, H>),
    Refused(OpenRefusal),
}

/// The writer handle. Clones of the inner state ride the detached
/// publisher and checkpoint-duty threads; `quiesce` joins them.
pub struct Writer<T: Theory + Clone, S: ObjectStore, H: StepHook = NoFaults> {
    inner: Arc<Inner<T, S, H>>,
}

struct Inner<T: Theory + Clone, S: ObjectStore, H: StepHook> {
    store: Arc<S>,
    prefix: String,
    dir: PathBuf,
    theory: T,
    codec: Codec,
    fingerprint: [u8; 32],
    writer_id: u64,
    linger: Duration,
    hook: H,
    counters: CounterCells,
    leases: Mutex<Leases>,
    maps: SchemaMaps,
    queues: BTreeMap<BraidId, Mutex<VecDeque<Request>>>,
    core: Mutex<Core<T>>,
    threads: Mutex<Vec<JoinHandle<()>>>,
    scratch_seq: AtomicU64,
}

enum MountEnd<T: Theory + Clone> {
    Mounted {
        db: Box<Db<T>>,
        chain: Chain,
        /// A pre-existing directory is in the unproven open phase until
        /// the wholeness identity passes; a seeded or bootstrapped
        /// store is whole by construction.
        pre_existing: bool,
    },
    Discard,
    Refused(OpenRefusal),
}

enum CatchUp {
    Tips,
    Gap,
    RejectedInOpen,
}

enum PendingArm {
    Clear,
    Backlog(BraidId),
    Discard,
}

impl<T, S> Writer<T, S, NoFaults>
where
    T: Theory + Clone + Send + Sync + 'static,
    S: ObjectStore + 'static,
{
    /// Opens a writer with the production hook.
    pub fn open(
        store: S,
        prefix: &str,
        dir: &Path,
        theory: T,
        options: Options,
    ) -> Result<WriterOpened<T, S, NoFaults>> {
        Self::open_hooked(store, prefix, dir, theory, options, NoFaults)
    }
}

impl<T, S, H> Writer<T, S, H>
where
    T: Theory + Clone + Send + Sync + 'static,
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    /// Opens a writer: store birth if the manifest is absent, then the
    /// replica's own establish gauntlet, then pending recovery per the
    /// three forced arms. An open whose backlog publication ends in
    /// `Contention` still comes up `Ready`: the store is whole, reads
    /// serve, and publication retries on the next commit.
    pub fn open_hooked(
        store: S,
        prefix: &str,
        dir: &Path,
        theory: T,
        options: Options,
        hook: H,
    ) -> Result<WriterOpened<T, S, H>> {
        let (codec, fingerprint) = match derive_codec(&theory) {
            Ok(derived) => derived,
            Err(refusal) => return Ok(WriterOpened::Refused(refusal)),
        };
        let descriptor: SchemaDescriptor = theory.clone().descriptor();
        let maps = schema_maps(&descriptor);
        let store = Arc::new(store);

        if store
            .get(&manifest_key(prefix))
            .map_err(|err| Error::Fault(Fault::Store(err)))?
            .is_none()
        {
            let manifest = Manifest {
                fingerprint,
                checkpoint: None,
            };
            let _ = create_manifest(store.as_ref(), prefix, &manifest)
                .map_err(|err| Error::Fault(Fault::Store(err)))?;
        }

        let queues: BTreeMap<BraidId, Mutex<VecDeque<Request>>> = codec
            .braids()
            .components()
            .keys()
            .map(|braid| (*braid, Mutex::new(VecDeque::new())))
            .collect();

        let inner = Arc::new(Inner {
            store,
            prefix: prefix.to_string(),
            dir: dir.to_path_buf(),
            theory,
            codec,
            fingerprint,
            writer_id: options.writer_id,
            linger: options.linger,
            hook,
            counters: CounterCells::default(),
            leases: Mutex::new(Leases::new()),
            maps,
            queues,
            core: Mutex::new(Core {
                db: None,
                chain: Chain {
                    entries: BTreeMap::new(),
                    pending: None,
                },
                floor: None,
                wedged: BTreeMap::new(),
                ack: options.ack,
                deposition: None,
                ckpt_sum: 0,
                log_bytes: 0,
                cadence_sum: CHECKPOINT_EVERY_SUM,
                cadence_bytes: CHECKPOINT_EVERY_BYTES,
                duty_busy: false,
            }),
            threads: Mutex::new(Vec::new()),
            scratch_seq: AtomicU64::new(0),
        });

        {
            let mut core = lock(&inner.core);
            if let Some(refusal) = inner.open_establish(&mut core)? {
                return Ok(WriterOpened::Refused(refusal));
            }
            if core.chain.pending.is_some() {
                match inner.resolve_backlog(&mut core, None, &mut Live { losses: 0 }) {
                    Ok(()) | Err(Error::Contention { .. }) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(WriterOpened::Ready(Self { inner }))
    }

    /// Runs `body` against a fresh batch and commits the recorded ops
    /// under one braid; a spanning batch refuses with the braids named.
    /// The driver never re-invokes `body` — retry is host policy.
    pub fn commit<R>(
        &self,
        body: impl FnOnce(&mut Batch<'_, S>) -> Result<R>,
    ) -> Result<Commit<R>> {
        let (value, ops) = self.record(body)?;
        if ops.is_empty() {
            return Err(Error::EmptyCommit);
        }
        let braid = self.single_braid(&ops)?;
        match self.submit(braid, ops)? {
            Resolved::Accepted {
                braid,
                generation,
                durability,
            } => Ok(Commit::Accepted {
                value,
                braid,
                generation,
                durability,
            }),
            Resolved::Rejected(violations) => Ok(Commit::Rejected(violations)),
            Resolved::Failed(shared) => Err(unwrap_shared(shared)),
        }
    }

    /// The explicit verb for writes to relations no statement relates:
    /// independent per-braid batches, committed sequentially in
    /// first-appearance order, outcomes as the vector of per-braid
    /// results. Splitness is chosen at the call site — partial
    /// completion is representable, never surprising.
    pub fn commit_split<R>(
        &self,
        body: impl FnOnce(&mut Batch<'_, S>) -> Result<R>,
    ) -> Result<(R, Vec<BraidOutcome>)> {
        let (value, ops) = self.record(body)?;
        if ops.is_empty() {
            return Err(Error::EmptyCommit);
        }
        let mut parts: Vec<(BraidId, Vec<Op>)> = Vec::new();
        for (index, op) in ops.into_iter().enumerate() {
            let braid = self.braid_of(index, &op)?;
            match parts.iter_mut().find(|(existing, _)| *existing == braid) {
                Some((_, part)) => part.push(op),
                None => parts.push((braid, vec![op])),
            }
        }
        let mut outcomes = Vec::with_capacity(parts.len());
        for (braid, part) in parts {
            match self.submit(braid, part)? {
                Resolved::Accepted {
                    braid,
                    generation,
                    durability,
                } => outcomes.push(BraidOutcome::Accepted {
                    braid,
                    generation,
                    durability,
                }),
                Resolved::Rejected(violations) => {
                    outcomes.push(BraidOutcome::Rejected { braid, violations });
                }
                Resolved::Failed(shared) => return Err(unwrap_shared(shared)),
            }
        }
        Ok((value, outcomes))
    }

    /// Read access to the engine's own surface — the store the writer
    /// serves reads from, current as of the last drained commit.
    pub fn with_db<R>(&self, f: impl FnOnce(&Db<T>) -> R) -> R {
        let core = lock(&self.inner.core);
        f(core.db())
    }

    /// The writer's vector: per-braid applied counts.
    #[must_use]
    pub fn vector(&self) -> Vector {
        lock(&self.inner.core).chain.vector()
    }

    /// Loser-algebra instrumentation since open.
    #[must_use]
    pub fn counters(&self) -> Counters {
        let cells = &self.inner.counters;
        Counters {
            re_judgments: cells.re_judgments.load(Ordering::Relaxed),
            republishes: cells.republishes.load(Ordering::Relaxed),
            subsumptions: cells.subsumptions.load(Ordering::Relaxed),
            disjoint_verdicts: cells.disjoint_verdicts.load(Ordering::Relaxed),
        }
    }

    /// The deposition signal, if a non-byte-equal `Exists` has proven a
    /// resident writer usurped.
    #[must_use]
    pub fn deposition(&self) -> Option<Deposition> {
        lock(&self.inner.core).deposition
    }

    /// The braid holding an applied-but-unpublished batch, if any —
    /// retained through `Contention`, republished before the next
    /// commit.
    #[must_use]
    pub fn backlog(&self) -> Option<BraidId> {
        lock(&self.inner.core)
            .chain
            .pending
            .as_ref()
            .map(|pending| pending.braid)
    }

    /// Braids wedged read-only by corruption-class verdicts.
    #[must_use]
    pub fn wedged_braids(&self) -> Vec<BraidId> {
        lock(&self.inner.core).wedged.keys().copied().collect()
    }

    /// Re-sizes the checkpoint cadence (both arms; 10 owns the
    /// defaults, the conformance pins re-size them).
    pub fn set_checkpoint_cadence(&self, sum: u64, bytes: u64) {
        let mut core = lock(&self.inner.core);
        core.cadence_sum = sum.max(1);
        core.cadence_bytes = bytes.max(1);
    }

    /// Joins the detached publisher and checkpoint-duty threads.
    pub fn quiesce(&self) {
        loop {
            let drained: Vec<JoinHandle<()>> = std::mem::take(&mut *lock(&self.inner.threads));
            if drained.is_empty() {
                return;
            }
            for handle in drained {
                let _ = handle.join();
            }
        }
    }

    fn record<R>(&self, body: impl FnOnce(&mut Batch<'_, S>) -> Result<R>) -> Result<(R, Vec<Op>)> {
        let mut batch = Batch {
            ops: Vec::new(),
            store: self.inner.store.as_ref(),
            prefix: &self.inner.prefix,
            leases: &self.inner.leases,
            maps: &self.inner.maps,
        };
        let value = body(&mut batch)?;
        Ok((value, batch.ops))
    }

    fn braid_of(&self, index: usize, op: &Op) -> Result<BraidId> {
        match self.inner.codec.braids().braid_of(op.relation) {
            Some(braid) => Ok(braid),
            None => match self.inner.codec.vocabulary().relation(op.relation) {
                Some(_) => Err(Error::Encode(EncodeError::ClosedRelation {
                    op: index,
                    relation: op.relation,
                })),
                None => Err(Error::Encode(EncodeError::UnknownRelation {
                    op: index,
                    relation: op.relation,
                })),
            },
        }
    }

    fn single_braid(&self, ops: &[Op]) -> Result<BraidId> {
        let mut braids: Vec<BraidId> = Vec::new();
        for (index, op) in ops.iter().enumerate() {
            let braid = self.braid_of(index, op)?;
            if !braids.contains(&braid) {
                braids.push(braid);
            }
        }
        match braids.as_slice() {
            [one] => Ok(*one),
            _ => Err(Error::SpanningCommit {
                braids: braids.into_boxed_slice(),
            }),
        }
    }

    fn submit(&self, braid: BraidId, ops: Vec<Op>) -> Result<Resolved> {
        let (rows, bytes) = self.inner.measure(&ops);
        let waiter = Arc::new(Waiter::new());
        lock(&self.inner.queues[&braid]).push_back(Request {
            ops,
            rows,
            bytes,
            waiter: Arc::clone(&waiter),
        });
        loop {
            if let Some(resolved) = waiter.get() {
                return Ok(resolved);
            }
            match self.inner.core.try_lock() {
                Ok(mut core) => {
                    if waiter.get().is_none() {
                        Inner::drain(&self.inner, &mut core, braid);
                    }
                }
                Err(TryLockError::WouldBlock) => waiter.wait_briefly(),
                Err(TryLockError::Poisoned(poisoned)) => {
                    let mut core = poisoned.into_inner();
                    if waiter.get().is_none() {
                        Inner::drain(&self.inner, &mut core, braid);
                    }
                }
            }
        }
    }
}

impl<T, S, H> Drop for Writer<T, S, H>
where
    T: Theory + Clone,
    S: ObjectStore,
    H: StepHook,
{
    fn drop(&mut self) {
        loop {
            let drained: Vec<JoinHandle<()>> = std::mem::take(&mut *lock(&self.inner.threads));
            if drained.is_empty() {
                return;
            }
            for handle in drained {
                let _ = handle.join();
            }
        }
    }
}

fn unwrap_shared(shared: Arc<Error>) -> Error {
    Arc::try_unwrap(shared).unwrap_or_else(Error::Drain)
}

impl<T, S, H> Inner<T, S, H>
where
    T: Theory + Clone + Send + Sync + 'static,
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    fn step(&self, step: WriterStep) -> Result<()> {
        match self.hook.observe(step) {
            StepControl::Continue => Ok(()),
            StepControl::Crash => Err(Error::InjectedCrash { step }),
        }
    }

    fn measure(&self, ops: &[Op]) -> (u64, u64) {
        let mut rows = 0u64;
        let mut sink = CountSink(0);
        for op in ops {
            sink.0 += 9;
            let layout = self.maps.layouts.get(&op.relation);
            for row in &op.rows {
                rows += 1;
                if let Some(layout) = layout {
                    for (value, ty) in row.iter().zip(layout.iter()) {
                        let _ = append_value(&mut sink, value, *ty);
                    }
                }
            }
        }
        (rows, sink.0)
    }

    /// The per-braid drain: snapshot the queue up to the packing caps,
    /// resolve any retained backlog first, then run one composite
    /// discipline — one batch, one generation, one transaction by law.
    /// A composite rejection falls back one-by-one in queue order so an
    /// innocent write never fails for a neighbor's violation.
    fn drain(self: &Arc<Self>, core: &mut Core<T>, braid: BraidId) {
        if self.linger > Duration::ZERO {
            std::thread::sleep(self.linger);
        }
        let mut picked: Vec<Request> = Vec::new();
        {
            let mut queue = lock(&self.queues[&braid]);
            let mut rows = 0u64;
            let mut bytes = 0u64;
            while let Some(front) = queue.front() {
                if !picked.is_empty()
                    && (rows + front.rows > DRAIN_MAX_WRITES
                        || bytes + front.bytes > DRAIN_MAX_BYTES)
                {
                    break;
                }
                rows += front.rows;
                bytes += front.bytes;
                picked.push(queue.pop_front().expect("front just peeked"));
            }
        }
        if picked.is_empty() {
            return;
        }
        let fail_all = |requests: &[Request], error: Error| {
            let shared = Arc::new(error);
            for request in requests {
                request
                    .waiter
                    .resolve(Resolved::Failed(Arc::clone(&shared)));
            }
        };
        if core.wedged.contains_key(&braid) {
            fail_all(&picked, Error::Wedged { braid });
            return;
        }
        if core.chain.pending.is_some()
            && let Err(error) = self.resolve_backlog(core, None, &mut Live { losses: 0 })
        {
            fail_all(&picked, error);
            return;
        }

        let composite: Vec<Op> = picked
            .iter()
            .flat_map(|request| request.ops.iter().cloned())
            .collect();
        let waiters: Vec<Arc<Waiter>> = picked
            .iter()
            .map(|request| Arc::clone(&request.waiter))
            .collect();
        let fp = match footprint(self.codec.vocabulary(), &composite) {
            Ok(fp) => fp,
            Err(error) => {
                fail_all(&picked, Error::Footprint(error));
                return;
            }
        };
        match self.discipline(
            core,
            braid,
            &composite,
            &fp,
            &mut Live { losses: 0 },
            Some(&waiters),
        ) {
            Ok(Settled::Accepted { generation }) => {
                for waiter in &waiters {
                    waiter.resolve(Resolved::Accepted {
                        braid,
                        generation,
                        durability: Durability::Published,
                    });
                }
            }
            Ok(Settled::Rejected(violations)) => {
                if picked.len() == 1 {
                    picked[0].waiter.resolve(Resolved::Rejected(violations));
                } else {
                    self.fallback(core, braid, &picked);
                }
            }
            Ok(Settled::Detached { bytes }) => {
                let segments: Vec<Vec<Op>> =
                    picked.iter().map(|request| request.ops.clone()).collect();
                self.spawn_publisher(braid, bytes, segments);
            }
            Err(error) => fail_all(&picked, error),
        }
    }

    /// One-by-one fallback for a rejected composite, each caller as its
    /// own transaction in queue order. Waiters a local ack already
    /// resolved keep their `LocalPending` answer — the honest arm.
    fn fallback(self: &Arc<Self>, core: &mut Core<T>, braid: BraidId, requests: &[Request]) {
        for request in requests {
            let fp = match footprint(self.codec.vocabulary(), &request.ops) {
                Ok(fp) => fp,
                Err(error) => {
                    request
                        .waiter
                        .resolve(Resolved::Failed(Arc::new(Error::Footprint(error))));
                    continue;
                }
            };
            match self.discipline(
                core,
                braid,
                &request.ops,
                &fp,
                &mut Live { losses: 0 },
                None,
            ) {
                Ok(Settled::Accepted { generation }) => {
                    request.waiter.resolve(Resolved::Accepted {
                        braid,
                        generation,
                        durability: Durability::Published,
                    });
                }
                Ok(Settled::Rejected(violations)) => {
                    request.waiter.resolve(Resolved::Rejected(violations));
                }
                Ok(Settled::Detached { .. }) => {
                    unreachable!("fallback passes no waiters, so no ack can detach")
                }
                Err(error) => {
                    request.waiter.resolve(Resolved::Failed(Arc::new(error)));
                }
            }
        }
    }

    /// The commit discipline, one shape for every pass: encode at the
    /// current head with the monotone ts clamp, fsync the pending slot
    /// BEFORE first judgment, apply in one `db.write`, then publish.
    /// A conflict loss loops back here — the re-judgment is the same
    /// `db.write` of the same recorded ops, never a body re-run.
    fn discipline(
        self: &Arc<Self>,
        core: &mut Core<T>,
        braid: BraidId,
        ops: &[Op],
        fp: &[Entry],
        live: &mut Live,
        mut waiters: Option<&[Arc<Waiter>]>,
    ) -> Result<Settled> {
        let mut pass = 0u64;
        loop {
            if core.wedged.contains_key(&braid) {
                return Err(Error::Wedged { braid });
            }
            let head = core.chain.position(braid);
            let slot = head.g + 1;
            let header = BatchHeader {
                fingerprint: self.fingerprint,
                braid,
                braid_gen: slot,
                prev: head.prev,
                writer: self.writer_id,
                timestamp: now_ms().max(head.ts),
            };
            let bytes = self.codec.encode(&header, ops).map_err(Error::Encode)?;
            self.step(WriterStep::Encode)?;

            core.chain.pending = Some(Pending {
                braid,
                slot,
                bytes: bytes.clone(),
            });
            core.chain
                .write_atomic(&self.dir)
                .map_err(|err| Error::Fault(Fault::Io(err)))?;
            self.step(WriterStep::PendingWrite)?;

            let before = core.generation()?;
            let admission = core
                .db()
                .write(|tx| {
                    for op in ops {
                        match op.kind {
                            OpKind::Insert => {
                                tx.insert_dyn(op.relation, op.rows.iter())?;
                            }
                            OpKind::Delete => {
                                tx.delete_dyn(op.relation, op.rows.iter())?;
                            }
                        }
                    }
                    Ok(())
                })
                .map_err(|err| Error::Fault(Fault::Engine(err)))?;
            self.step(WriterStep::ApplyLocal)?;

            match admission {
                Admission::Rejected(violations) => {
                    self.clear_pending(core)?;
                    return Ok(Settled::Rejected(violations));
                }
                Admission::Accepted(committed) => {
                    if committed.generation.value() == before {
                        // The publish law: the empty commit is not a
                        // commit, and the log never gains a no-op slot.
                        self.clear_pending(core)?;
                        return Ok(Settled::Accepted { generation: head.g });
                    }
                }
            }

            if let Some(acked) = waiters.take()
                && let AckMode::Local {
                    max_pending_batches,
                    max_pending_bytes,
                } = core.ack
                && max_pending_batches >= 1
                && bytes.len() as u64 <= max_pending_bytes
            {
                for waiter in acked {
                    waiter.resolve(Resolved::Accepted {
                        braid,
                        generation: slot,
                        durability: Durability::LocalPending,
                    });
                }
                self.step(WriterStep::AckLocal)?;
                return Ok(Settled::Detached { bytes });
            }

            if pass > 0 {
                self.counters.republishes.fetch_add(1, Ordering::Relaxed);
            }
            pass += 1;

            match self.publish(
                core,
                braid,
                slot,
                header.timestamp,
                &bytes,
                ops,
                fp,
                live,
                true,
            )? {
                PublishEnd::Done(settled) => return Ok(settled),
                PublishEnd::ReJudge => {}
            }
        }
    }

    /// One publication attempt: `put_create`, then `Created`, the
    /// byte-equal absorption of our own ambiguous PUT, or the loser
    /// algebra. A `put_create` failure is an ambiguous outcome — the
    /// request may have landed — so it is never retried blindly: the
    /// follow-up is a GET of the target key comparing content, and only
    /// a proven-absent create is reissued.
    #[allow(clippy::too_many_arguments)]
    fn publish(
        self: &Arc<Self>,
        core: &mut Core<T>,
        braid: BraidId,
        slot: u64,
        ts: u64,
        bytes: &[u8],
        ops: &[Op],
        fp: &[Entry],
        live: &mut Live,
        counts_live: bool,
    ) -> Result<PublishEnd> {
        const CREATE_ATTEMPTS: u32 = 6;
        let key = log_key(&self.prefix, braid, slot);
        let mut attempt: u32 = 0;
        let created = loop {
            match self.store.put_create(&key, bytes) {
                Ok(created) => break created,
                Err(err) => {
                    match resolve_ambiguous_create(self.store.as_ref(), &key, bytes)
                        .map_err(|probe_err| Error::Fault(Fault::Store(probe_err)))?
                    {
                        CreateProbe::Landed(etag) => break Create::Created(etag),
                        CreateProbe::Lost(_) => break Create::Exists,
                        CreateProbe::Absent => {
                            attempt += 1;
                            if attempt == CREATE_ATTEMPTS {
                                return Err(Error::Fault(Fault::Store(err)));
                            }
                        }
                    }
                }
            }
        };
        self.step(WriterStep::PutLog)?;
        match created {
            Create::Created(_) => {
                self.advance_and_clear(core, braid, slot, ts, bytes)?;
                Ok(PublishEnd::Done(Settled::Accepted { generation: slot }))
            }
            Create::Exists => {
                let winner = retry_read(|| self.store.get(&key))
                    .map_err(|err| Error::Fault(Fault::Store(err)))?
                    .ok_or_else(|| {
                        Error::Fault(Fault::Io(io::Error::other(
                            "log slot existed and then vanished",
                        )))
                    })?;
                if winner.bytes == bytes {
                    self.advance_and_clear(core, braid, slot, ts, bytes)?;
                    return Ok(PublishEnd::Done(Settled::Accepted { generation: slot }));
                }
                self.lose(core, braid, slot, &winner.bytes, ops, fp, live, counts_live)
            }
        }
    }

    /// The loser algebra at a lost slot. Subsumed applies the winner
    /// and lets the engine decide survive-or-discard through the
    /// wholeness identity; full key disjointness takes the republish
    /// fast path (L7 keeps the carried verdict and the publish-law
    /// standing true at the moved base); everything else discards and
    /// re-judges.
    #[allow(clippy::too_many_arguments)]
    fn lose(
        self: &Arc<Self>,
        core: &mut Core<T>,
        braid: BraidId,
        slot: u64,
        winner_bytes: &[u8],
        ops: &[Op],
        fp: &[Entry],
        live: &mut Live,
        counts_live: bool,
    ) -> Result<PublishEnd> {
        let winner = match self.codec.decode(winner_bytes) {
            Ok(batch) => batch,
            Err(error) => {
                core.wedged.insert(
                    braid,
                    Corruption::Refused(crate::apply::ApplyRefusal::Decode(error)),
                );
                return Err(Error::Wedged { braid });
            }
        };
        if let AckMode::Local { .. } = core.ack
            && core.deposition.is_none()
        {
            core.deposition = Some(Deposition {
                braid,
                slot,
                resident: self.writer_id,
                usurper: winner.header.writer,
            });
            core.ack = AckMode::Published;
        }
        let decision = intersect(
            self.codec.vocabulary(),
            fp,
            ops,
            &winner.ops,
            &BTreeMap::new(),
        )
        .map_err(Error::Footprint)?;
        match decision {
            LoserDecision::Subsumed => self.subsume(core, braid, slot, winner_bytes),
            LoserDecision::Disjoint => {
                self.counters
                    .disjoint_verdicts
                    .fetch_add(1, Ordering::Relaxed);
                if counts_live {
                    live.losses += 1;
                    if live.losses >= LOSS_BOUND {
                        return Err(Error::Contention {
                            braid,
                            cause: ContentionCause::SlotRace { tip: slot },
                        });
                    }
                }
                self.republish_disjoint(core, braid, slot, winner_bytes, ops, fp, live)
            }
            LoserDecision::Conflict(cause) => {
                if counts_live {
                    live.losses += 1;
                    if live.losses >= LOSS_BOUND {
                        return Err(Error::Contention {
                            braid,
                            cause: self.hot_key(ops, cause),
                        });
                    }
                }
                self.counters.re_judgments.fetch_add(1, Ordering::Relaxed);
                self.re_establish(core)?;
                Ok(PublishEnd::ReJudge)
            }
        }
    }

    /// The subsumed arm: the winner already performed our net effect,
    /// so nothing publishes and the engine decides the survival arm at
    /// the apply — a no-op means the winner's slot is accounted by our
    /// own earlier apply and the store survives in place; anything else
    /// means one slot now covers two local advances, the store forked,
    /// and forks are the disposable law's business, never
    /// bookkeeping's.
    fn subsume(
        self: &Arc<Self>,
        core: &mut Core<T>,
        braid: BraidId,
        slot: u64,
        winner_bytes: &[u8],
    ) -> Result<PublishEnd> {
        self.counters.subsumptions.fetch_add(1, Ordering::Relaxed);
        let applied = apply(
            core.db
                .as_ref()
                .expect("an established writer holds a store"),
            &mut core.chain,
            &self.codec,
            braid,
            slot,
            winner_bytes,
            0,
        )
        .map_err(|err| Error::Fault(Fault::Engine(err)))?;
        match applied {
            Applied::Absorbed { .. } => {
                self.clear_pending(core)?;
                Ok(PublishEnd::Done(Settled::Accepted { generation: slot }))
            }
            Applied::Advanced { .. } | Applied::Rejected(_) | Applied::Refused(_) => {
                self.re_establish(core)?;
                Ok(PublishEnd::Done(Settled::Accepted { generation: slot }))
            }
        }
    }

    /// The disjoint fast path (L7 licenses carrying the verdict and the
    /// footprint; L8 makes winner-over-ours equal ours-over-winner):
    /// apply the lost slot's winner in place — under full key
    /// disjointness that apply is provably state-changing-accepted —
    /// then replay any further occupied slots under the same pairwise
    /// tests (losses to history never count toward the live bound), and
    /// republish the recorded ops under a re-addressed header at the
    /// tip. The tip attempt races live, so its losses count.
    #[allow(clippy::too_many_arguments)]
    fn republish_disjoint(
        self: &Arc<Self>,
        core: &mut Core<T>,
        braid: BraidId,
        slot: u64,
        winner_bytes: &[u8],
        ops: &[Op],
        fp: &[Entry],
        live: &mut Live,
    ) -> Result<PublishEnd> {
        let mut at = slot;
        let mut pending_apply: Vec<u8> = winner_bytes.to_vec();
        loop {
            let applied = apply(
                core.db
                    .as_ref()
                    .expect("an established writer holds a store"),
                &mut core.chain,
                &self.codec,
                braid,
                at,
                &pending_apply,
                1,
            )
            .map_err(|err| Error::Fault(Fault::Engine(err)))?;
            match applied {
                Applied::Advanced { .. } => {}
                Applied::Absorbed { .. } | Applied::Rejected(_) => {
                    // Not the state-changing accept full disjointness
                    // promises: the conflict arm's rebuild is always
                    // sound.
                    self.re_establish(core)?;
                    return Ok(PublishEnd::ReJudge);
                }
                Applied::Refused(refusal) => {
                    core.wedged.insert(braid, Corruption::Refused(refusal));
                    return Err(Error::Wedged { braid });
                }
            }
            let key = log_key(&self.prefix, braid, at + 1);
            let occupant = retry_read(|| self.store.get(&key))
                .map_err(|err| Error::Fault(Fault::Store(err)))?;
            let Some(fetched) = occupant else {
                break;
            };
            let Ok(batch) = self.codec.decode(&fetched.bytes) else {
                self.re_establish(core)?;
                return Ok(PublishEnd::ReJudge);
            };
            match intersect(
                self.codec.vocabulary(),
                fp,
                ops,
                &batch.ops,
                &BTreeMap::new(),
            )
            .map_err(Error::Footprint)?
            {
                LoserDecision::Subsumed => {
                    return self.subsume(core, braid, at + 1, &fetched.bytes);
                }
                LoserDecision::Disjoint => {
                    self.counters
                        .disjoint_verdicts
                        .fetch_add(1, Ordering::Relaxed);
                    at += 1;
                    pending_apply = fetched.bytes;
                }
                LoserDecision::Conflict(_) => {
                    self.re_establish(core)?;
                    return Ok(PublishEnd::ReJudge);
                }
            }
        }
        let head = core.chain.position(braid);
        let next = head.g + 1;
        let header = BatchHeader {
            fingerprint: self.fingerprint,
            braid,
            braid_gen: next,
            prev: head.prev,
            writer: self.writer_id,
            timestamp: now_ms().max(head.ts),
        };
        let bytes = self.codec.encode(&header, ops).map_err(Error::Encode)?;
        self.step(WriterStep::Encode)?;
        core.chain.pending = Some(Pending {
            braid,
            slot: next,
            bytes: bytes.clone(),
        });
        core.chain
            .write_atomic(&self.dir)
            .map_err(|err| Error::Fault(Fault::Io(err)))?;
        self.step(WriterStep::PendingWrite)?;
        self.counters.republishes.fetch_add(1, Ordering::Relaxed);
        self.publish(
            core,
            braid,
            next,
            header.timestamp,
            &bytes,
            ops,
            fp,
            live,
            true,
        )
    }

    /// Maps the terminal conflict onto the loser's own raw determinant
    /// values — the operable handle the host examines.
    fn hot_key(&self, ops: &[Op], cause: ConflictCause) -> ContentionCause {
        match cause {
            ConflictCause::Fact { fid } => {
                for op in ops {
                    let Some(layout) = self.maps.layouts.get(&op.relation) else {
                        continue;
                    };
                    for row in &op.rows {
                        if hash_values(&op.relation.0.to_le_bytes(), layout, None, row) == Some(fid)
                        {
                            return ContentionCause::HotKey {
                                statement: None,
                                values: row.clone(),
                            };
                        }
                    }
                }
                ContentionCause::HotKey {
                    statement: None,
                    values: Box::from([]),
                }
            }
            ConflictCause::Key { statement, key }
            | ConflictCause::Containment { statement, key }
            | ConflictCause::CapacityInterval { statement, key }
            | ConflictCause::CapacityParent { statement, key }
            | ConflictCause::CapacityMeasureMissing { statement, key } => {
                if let Some(sides) = self.maps.dets.get(&statement) {
                    for (relation, projection) in sides {
                        let Some(layout) = self.maps.layouts.get(relation) else {
                            continue;
                        };
                        for op in ops.iter().filter(|op| op.relation == *relation) {
                            for row in &op.rows {
                                if hash_values(
                                    &statement.0.to_le_bytes(),
                                    layout,
                                    Some(projection),
                                    row,
                                ) == Some(key)
                                {
                                    let values: Box<[Value]> = projection
                                        .iter()
                                        .map(|field| row[usize::from(*field)].clone())
                                        .collect();
                                    return ContentionCause::HotKey {
                                        statement: Some(statement),
                                        values,
                                    };
                                }
                            }
                        }
                    }
                }
                ContentionCause::HotKey {
                    statement: Some(statement),
                    values: Box::from([]),
                }
            }
        }
    }

    fn advance_and_clear(
        self: &Arc<Self>,
        core: &mut Core<T>,
        braid: BraidId,
        slot: u64,
        ts: u64,
        bytes: &[u8],
    ) -> Result<()> {
        core.chain.entries.insert(
            braid,
            ChainEntry {
                g: slot,
                prev: *blake3::hash(bytes).as_bytes(),
                ts,
            },
        );
        self.step(WriterStep::ChainAdvance)?;
        core.log_bytes += bytes.len() as u64;
        self.clear_pending(core)?;
        self.maybe_duty(core);
        Ok(())
    }

    fn clear_pending(&self, core: &mut Core<T>) -> Result<()> {
        core.chain.pending = None;
        core.chain
            .write_atomic(&self.dir)
            .map_err(|err| Error::Fault(Fault::Io(err)))?;
        self.step(WriterStep::PendingClear)?;
        Ok(())
    }

    /// Pending resolution — 60's three forced arms, idempotent by L10:
    /// apply the pending batch; `Rejected` clears and publishes nothing
    /// (a resurrected never-judged batch); an accepted no-op at the
    /// exact vector sum was born a no-op and clears; otherwise the
    /// commit is real and unpublished — catch up with the loser tests
    /// and publish, create-or-compare.
    #[allow(clippy::too_many_lines)]
    fn resolve_backlog(
        self: &Arc<Self>,
        core: &mut Core<T>,
        segments: Option<&[Vec<Op>]>,
        live: &mut Live,
    ) -> Result<()> {
        let Some(pending) = core.chain.pending.clone() else {
            return Ok(());
        };
        let braid = pending.braid;
        if core.wedged.contains_key(&braid) {
            return Err(Error::Wedged { braid });
        }
        let Ok(batch) = self.codec.decode(&pending.bytes) else {
            // Our own pending bytes refusing to decode is a torn local
            // state; the disposable law answers.
            self.re_establish(core)?;
            return Ok(());
        };
        let ops = batch.ops;
        let fp = footprint(self.codec.vocabulary(), &ops).map_err(Error::Footprint)?;

        let before = core.generation()?;
        let admission = core
            .db()
            .write(|tx| {
                for op in &ops {
                    match op.kind {
                        OpKind::Insert => {
                            tx.insert_dyn(op.relation, op.rows.iter())?;
                        }
                        OpKind::Delete => {
                            tx.delete_dyn(op.relation, op.rows.iter())?;
                        }
                    }
                }
                Ok(())
            })
            .map_err(|err| Error::Fault(Fault::Engine(err)))?;
        self.step(WriterStep::ApplyLocal)?;
        let after = match admission {
            Admission::Rejected(_) => {
                // Nothing was acked; nothing is owed; a born-rejected
                // batch reaching the log is the publish law's cardinal
                // sin, structurally impossible here.
                self.clear_pending(core)?;
                return Ok(());
            }
            Admission::Accepted(committed) => committed.generation.value(),
        };
        let sum = core.chain.sum();
        if after == sum {
            // Born a net no-op: the crash landed between the no-op
            // verdict and its pending clear.
            self.clear_pending(core)?;
            return Ok(());
        }
        if after != sum + 1 || before > after {
            // A generation no pending term accounts for: phantom or
            // torn store.
            self.re_establish(core)?;
            return Ok(());
        }

        // Real and unpublished. The slot is head+1 by construction;
        // whatever occupies it is history, and losses to history never
        // count toward the live bound.
        let key = log_key(&self.prefix, braid, pending.slot);
        let occupant = self
            .store
            .get(&key)
            .map_err(|err| Error::Fault(Fault::Store(err)))?;
        let end = match occupant {
            Some(winner) if winner.bytes == pending.bytes => {
                // Already published: the crash was mid-publish, after
                // the create landed.
                let applied = apply(
                    core.db
                        .as_ref()
                        .expect("an established writer holds a store"),
                    &mut core.chain,
                    &self.codec,
                    braid,
                    pending.slot,
                    &winner.bytes,
                    0,
                )
                .map_err(|err| Error::Fault(Fault::Engine(err)))?;
                match applied {
                    Applied::Absorbed { .. } | Applied::Advanced { .. } => {
                        self.clear_pending(core)?;
                        return Ok(());
                    }
                    Applied::Rejected(_) | Applied::Refused(_) => {
                        self.re_establish(core)?;
                        return Ok(());
                    }
                }
            }
            Some(winner) => self.lose(
                core,
                braid,
                pending.slot,
                &winner.bytes,
                &ops,
                &fp,
                live,
                false,
            )?,
            None => {
                let hole = core
                    .floor
                    .as_ref()
                    .and_then(|(_, doc)| doc.braids.get(&braid))
                    .is_some_and(|head| pending.slot <= head.g);
                if hole {
                    // Retention passed our slot: the world moved beyond
                    // reach of the pairwise tests; rebuild and re-judge.
                    self.re_establish(core)?;
                    PublishEnd::ReJudge
                } else {
                    self.publish(
                        core,
                        braid,
                        pending.slot,
                        batch.header.timestamp,
                        &pending.bytes,
                        &ops,
                        &fp,
                        live,
                        true,
                    )?
                }
            }
        };
        match end {
            PublishEnd::Done(_) => Ok(()),
            PublishEnd::ReJudge => {
                let settled = self.discipline(core, braid, &ops, &fp, live, None)?;
                if let Settled::Rejected(_) = settled
                    && let Some(segments) = segments
                    && segments.len() > 1
                {
                    // The composite rejected at the conflict-loss
                    // re-judgment: one-by-one fallback, each caller as
                    // its own transaction in queue order.
                    for segment in segments {
                        let fp = footprint(self.codec.vocabulary(), segment)
                            .map_err(Error::Footprint)?;
                        let _ = self.discipline(
                            core,
                            braid,
                            segment,
                            &fp,
                            &mut Live { losses: 0 },
                            None,
                        )?;
                    }
                }
                Ok(())
            }
        }
    }

    /// The detached publisher: acks moved to the end of the local
    /// apply, so publication continues off the caller. Keyed by the
    /// pending bytes — if another drain resolved the backlog first, the
    /// publisher finds different bytes and stands down.
    fn spawn_publisher(self: &Arc<Self>, _braid: BraidId, bytes: Vec<u8>, segments: Vec<Vec<Op>>) {
        let inner = Arc::clone(self);
        let handle = std::thread::spawn(move || {
            let mut core = lock(&inner.core);
            let matches = core
                .chain
                .pending
                .as_ref()
                .is_some_and(|pending| pending.bytes == bytes);
            if matches {
                let _ = inner.resolve_backlog(&mut core, Some(&segments), &mut Live { losses: 0 });
            }
        });
        lock(&self.threads).push(handle);
    }

    /// Checkpoint duty: on a cadence crossing, compact and snapshot
    /// under the frozen core, then upload both objects and run the
    /// manifest CAS off the commit loop. Races resolve by the
    /// checkpoint order; losing objects are gc fodder.
    fn maybe_duty(self: &Arc<Self>, core: &mut Core<T>) {
        let sum = core.chain.sum();
        if core.duty_busy
            || (sum.saturating_sub(core.ckpt_sum) < core.cadence_sum
                && core.log_bytes < core.cadence_bytes)
        {
            return;
        }
        core.duty_busy = true;
        let seq = self.scratch_seq.fetch_add(1, Ordering::Relaxed);
        let scratch = PathBuf::from(format!("{}.ckpt{seq}", self.dir.display()));
        let _ = fs::remove_dir_all(&scratch);
        let compacted: std::result::Result<(Vec<u8>, [u8; 32]), ()> = (|| {
            core.db().compact(&scratch).map_err(|_| ())?;
            let bytes = fs::read(scratch.join(DATA_FILE)).map_err(|_| ())?;
            let catalog = core.db().catalog_digest().map_err(|_| ())?;
            Ok((bytes, catalog))
        })();
        let _ = fs::remove_dir_all(&scratch);
        let Ok((bytes, catalog)) = compacted else {
            core.duty_busy = false;
            return;
        };
        let heads: BTreeMap<BraidId, Head> = core
            .chain
            .entries
            .iter()
            .map(|(braid, entry)| {
                (
                    *braid,
                    Head {
                        g: entry.g,
                        hash: entry.prev,
                        ts: entry.ts,
                    },
                )
            })
            .collect();
        let inner = Arc::clone(self);
        let writer_id = self.writer_id;
        let handle = std::thread::spawn(move || {
            let digest = *blake3::hash(&bytes).as_bytes();
            let sum: u64 = heads.values().map(|head| head.g).sum();
            let outcome = (|| -> std::result::Result<bool, ()> {
                let prev = inner
                    .store
                    .get(&manifest_key(&inner.prefix))
                    .map_err(|_| ())?
                    .and_then(|fetched| Manifest::parse(&fetched.bytes).ok())
                    .and_then(|manifest| manifest.checkpoint);
                let doc = Checkpoint {
                    braids: heads,
                    catalog,
                    writer: writer_id,
                    prev,
                };
                inner
                    .store
                    .put_create(&ckpt_mdb_key(&inner.prefix, &digest), &bytes)
                    .map_err(|_| ())?;
                inner
                    .store
                    .put_create(&ckpt_json_key(&inner.prefix, &digest), &doc.render())
                    .map_err(|_| ())?;
                match publish_checkpoint(
                    inner.store.as_ref(),
                    &inner.prefix,
                    inner.codec.braids(),
                    digest,
                    sum,
                )
                .map_err(|_| ())?
                {
                    Published::Replaced | Published::Kept { .. } => Ok(true),
                    Published::Refused(_) => Ok(false),
                }
            })();
            let mut core = lock(&inner.core);
            core.duty_busy = false;
            if outcome == Ok(true) {
                core.ckpt_sum = core.ckpt_sum.max(sum);
                core.log_bytes = 0;
            }
        });
        lock(&self.threads).push(handle);
    }

    /// Establish at open: manifest gauntlet, mount, pending arms 1 and
    /// 2 inline (arm 3 marks the backlog), catch-up skipping the
    /// backlog braid, and the wholeness identity. `Some` is a refusal;
    /// `None` leaves the core established.
    fn open_establish(self: &Arc<Self>, core: &mut Core<T>) -> Result<Option<OpenRefusal>> {
        for _ in 0..8 {
            match self.read_floor()? {
                Ok(floor) => core.floor = floor,
                Err(refusal) => return Ok(Some(refusal)),
            }
            let mounted = self.mount(core)?;
            let (db, chain, pre_existing) = match mounted {
                MountEnd::Mounted {
                    db,
                    chain,
                    pre_existing,
                } => (db, chain, pre_existing),
                MountEnd::Discard => {
                    self.discard_dir()?;
                    continue;
                }
                MountEnd::Refused(refusal) => return Ok(Some(refusal)),
            };
            core.db = Some(*db);
            core.chain = chain;
            core.wedged.clear();

            let mut applied = 0u64;
            let mut skip: Option<BraidId> = None;
            if core.chain.pending.is_some() {
                match self.pending_arm(core)? {
                    PendingArm::Clear => {}
                    PendingArm::Backlog(braid) => {
                        applied = 1;
                        skip = Some(braid);
                    }
                    PendingArm::Discard => {
                        core.db = None;
                        self.discard_dir()?;
                        continue;
                    }
                }
            }
            match self.catch_up(core, skip, applied, pre_existing)? {
                CatchUp::Tips => {}
                CatchUp::Gap | CatchUp::RejectedInOpen => {
                    core.db = None;
                    self.discard_dir()?;
                    continue;
                }
            }
            if core.generation()? == core.chain.sum() + applied {
                core.ckpt_sum = core.floor.as_ref().map_or(0, |(_, doc)| doc.sum());
                return Ok(None);
            }
            core.db = None;
            self.discard_dir()?;
        }
        Err(Error::Fault(Fault::Io(io::Error::other(
            "discard-and-re-pull did not converge",
        ))))
    }

    /// The disposable law mid-commit: drop the store, delete the
    /// directory, and rebuild winner-current from the bucket.
    fn re_establish(&self, core: &mut Core<T>) -> Result<()> {
        core.db = None;
        self.discard_dir()?;
        for _ in 0..8 {
            match self.read_floor()? {
                Ok(floor) => core.floor = floor,
                Err(refusal) => return Err(Error::Refused(refusal)),
            }
            let mounted = self.mount(core)?;
            let (db, chain, pre_existing) = match mounted {
                MountEnd::Mounted {
                    db,
                    chain,
                    pre_existing,
                } => (db, chain, pre_existing),
                MountEnd::Discard => {
                    self.discard_dir()?;
                    continue;
                }
                MountEnd::Refused(refusal) => return Err(Error::Refused(refusal)),
            };
            core.db = Some(*db);
            core.chain = chain;
            core.wedged.clear();
            match self.catch_up(core, None, 0, pre_existing)? {
                CatchUp::Tips => {}
                CatchUp::Gap | CatchUp::RejectedInOpen => {
                    core.db = None;
                    self.discard_dir()?;
                    continue;
                }
            }
            if core.generation()? == core.chain.sum() {
                return Ok(());
            }
            core.db = None;
            self.discard_dir()?;
        }
        Err(Error::Fault(Fault::Io(io::Error::other(
            "discard-and-re-pull did not converge",
        ))))
    }

    /// Applies the pending batch and reads the arm: the verdict plus
    /// the wholeness instrument decide everything.
    fn pending_arm(&self, core: &mut Core<T>) -> Result<PendingArm> {
        let pending = core
            .chain
            .pending
            .clone()
            .expect("caller checked pending presence");
        let Ok(batch) = self.codec.decode(&pending.bytes) else {
            return Ok(PendingArm::Discard);
        };
        let admission = core
            .db()
            .write(|tx| {
                for op in &batch.ops {
                    match op.kind {
                        OpKind::Insert => {
                            tx.insert_dyn(op.relation, op.rows.iter())?;
                        }
                        OpKind::Delete => {
                            tx.delete_dyn(op.relation, op.rows.iter())?;
                        }
                    }
                }
                Ok(())
            })
            .map_err(|err| Error::Fault(Fault::Engine(err)))?;
        self.step(WriterStep::ApplyLocal)?;
        let after = match admission {
            Admission::Rejected(_) => {
                self.clear_pending(core)?;
                return Ok(PendingArm::Clear);
            }
            Admission::Accepted(committed) => committed.generation.value(),
        };
        let sum = core.chain.sum();
        if after == sum {
            self.clear_pending(core)?;
            Ok(PendingArm::Clear)
        } else if after == sum + 1 {
            Ok(PendingArm::Backlog(pending.braid))
        } else {
            Ok(PendingArm::Discard)
        }
    }

    fn read_floor(&self) -> Result<std::result::Result<Floor, OpenRefusal>> {
        let fetched = self
            .store
            .get(&manifest_key(&self.prefix))
            .map_err(|err| Error::Fault(Fault::Store(err)))?;
        let Some(fetched) = fetched else {
            return Ok(Err(OpenRefusal::ManifestMissing));
        };
        let manifest = match Manifest::parse(&fetched.bytes) {
            Ok(manifest) => manifest,
            Err(error) => return Ok(Err(OpenRefusal::Manifest(error))),
        };
        if manifest.fingerprint != self.fingerprint {
            return Ok(Err(OpenRefusal::FingerprintMismatch {
                manifest: manifest.fingerprint,
                derived: self.fingerprint,
            }));
        }
        let Some(digest) = manifest.checkpoint else {
            return Ok(Ok(None));
        };
        let doc = self
            .store
            .get(&ckpt_json_key(&self.prefix, &digest))
            .map_err(|err| Error::Fault(Fault::Store(err)))?;
        let Some(doc) = doc else {
            return Ok(Err(OpenRefusal::CheckpointDocMissing { digest }));
        };
        match Checkpoint::parse(&doc.bytes, self.codec.braids()) {
            Ok(doc) => Ok(Ok(Some((digest, doc)))),
            Err(error) => Ok(Err(OpenRefusal::Checkpoint { digest, error })),
        }
    }

    fn mount(&self, core: &Core<T>) -> Result<MountEnd<T>> {
        if self.dir.exists() {
            return match Db::open(&self.dir, self.theory.clone()) {
                Ok(db) => {
                    let Some(Ok(chain)) = Chain::read(&self.dir, self.codec.braids())
                        .map_err(|err| Error::Fault(Fault::Io(err)))?
                    else {
                        drop(db);
                        return Ok(MountEnd::Discard);
                    };
                    Ok(MountEnd::Mounted {
                        db: Box::new(db),
                        chain,
                        pre_existing: true,
                    })
                }
                Err(error @ bumbledb::Error::EnvironmentLocked) => {
                    Err(Error::Fault(Fault::Engine(error)))
                }
                Err(_) => Ok(MountEnd::Discard),
            };
        }
        if let Some((digest, doc)) = core.floor.clone() {
            return self.seed(digest, &doc);
        }
        match Db::create(&self.dir, self.theory.clone())
            .map_err(|err| Error::Fault(Fault::Engine(err)))?
        {
            Admission::Accepted(db) => {
                let chain = Chain::genesis(self.codec.braids());
                chain
                    .write_atomic(&self.dir)
                    .map_err(|err| Error::Fault(Fault::Io(err)))?;
                Ok(MountEnd::Mounted {
                    db: Box::new(db),
                    chain,
                    pre_existing: false,
                })
            }
            Admission::Rejected(violations) => {
                Ok(MountEnd::Refused(OpenRefusal::TheoryRejected(violations)))
            }
        }
    }

    fn seed(&self, digest: [u8; 32], doc: &Checkpoint) -> Result<MountEnd<T>> {
        let bytes = match fetch_checkpoint_bytes(self.store.as_ref(), &self.prefix, digest)
            .map_err(Error::Fault)?
        {
            Ok(bytes) => bytes,
            Err(refusal) => return Ok(MountEnd::Refused(refusal)),
        };
        write_checkpoint_bytes(&self.dir, &bytes).map_err(|err| Error::Fault(Fault::Io(err)))?;
        let db = match Db::open(&self.dir, self.theory.clone()) {
            Ok(db) => db,
            Err(error @ (bumbledb::Error::Io(_) | bumbledb::Error::EnvironmentLocked)) => {
                return Err(Error::Fault(Fault::Engine(error)));
            }
            Err(error) => {
                return Ok(MountEnd::Refused(OpenRefusal::CheckpointOpen {
                    digest,
                    error,
                }));
            }
        };
        let opened = db
            .generation()
            .map_err(|err| Error::Fault(Fault::Engine(err)))?
            .value();
        if opened != doc.sum() {
            return Ok(MountEnd::Refused(OpenRefusal::CheckpointState {
                digest,
                opened,
                sum: doc.sum(),
            }));
        }
        let computed = db
            .catalog_digest()
            .map_err(|err| Error::Fault(Fault::Engine(err)))?;
        if computed != doc.catalog {
            return Ok(MountEnd::Refused(OpenRefusal::CatalogMismatch {
                digest,
                writer: doc.writer,
                carried: doc.catalog,
                computed,
            }));
        }
        let chain = Chain {
            entries: doc
                .braids
                .iter()
                .map(|(braid, head)| {
                    (
                        *braid,
                        ChainEntry {
                            g: head.g,
                            prev: head.hash,
                            ts: head.ts,
                        },
                    )
                })
                .collect(),
            pending: None,
        };
        chain
            .write_atomic(&self.dir)
            .map_err(|err| Error::Fault(Fault::Io(err)))?;
        Ok(MountEnd::Mounted {
            db: Box::new(db),
            chain,
            pre_existing: false,
        })
    }

    /// Round-robin catch-up over all braids but the backlog's own —
    /// that braid's replay runs under the loser tests instead. A
    /// rejected replay on a pre-existing directory is the open-phase
    /// discard; on a seeded or bootstrapped store it wedges.
    fn catch_up(
        &self,
        core: &mut Core<T>,
        skip: Option<BraidId>,
        applied: u64,
        open_phase: bool,
    ) -> Result<CatchUp> {
        let braids: Vec<BraidId> = self
            .codec
            .braids()
            .components()
            .keys()
            .copied()
            .filter(|braid| Some(*braid) != skip)
            .collect();
        let mut at_tip: std::collections::BTreeSet<BraidId> = std::collections::BTreeSet::new();
        loop {
            let mut progressed = false;
            for braid in &braids {
                if at_tip.contains(braid) || core.wedged.contains_key(braid) {
                    continue;
                }
                let position = core.chain.position(*braid);
                let slot = position.g + 1;
                let key = log_key(&self.prefix, *braid, slot);
                let fetched = self
                    .store
                    .get(&key)
                    .map_err(|err| Error::Fault(Fault::Store(err)))?;
                let Some(fetched) = fetched else {
                    let hole = core
                        .floor
                        .as_ref()
                        .and_then(|(_, doc)| doc.braids.get(braid))
                        .is_some_and(|head| slot <= head.g);
                    if hole {
                        return Ok(CatchUp::Gap);
                    }
                    at_tip.insert(*braid);
                    continue;
                };
                let outcome = apply(
                    core.db.as_ref().expect("mounted"),
                    &mut core.chain,
                    &self.codec,
                    *braid,
                    slot,
                    &fetched.bytes,
                    applied,
                )
                .map_err(|err| Error::Fault(Fault::Engine(err)))?;
                match outcome {
                    Applied::Advanced { .. } | Applied::Absorbed { .. } => {
                        core.chain
                            .write_atomic(&self.dir)
                            .map_err(|err| Error::Fault(Fault::Io(err)))?;
                        progressed = true;
                    }
                    Applied::Rejected(violations) => {
                        if open_phase {
                            return Ok(CatchUp::RejectedInOpen);
                        }
                        core.wedged.insert(
                            *braid,
                            Corruption::ReplayDiverged {
                                braid: *braid,
                                slot,
                                violations,
                            },
                        );
                    }
                    Applied::Refused(refusal) => {
                        core.wedged.insert(*braid, Corruption::Refused(refusal));
                    }
                }
            }
            if !progressed {
                return Ok(CatchUp::Tips);
            }
        }
    }

    fn discard_dir(&self) -> Result<()> {
        match fs::remove_dir_all(&self.dir) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(Error::Fault(Fault::Io(err))),
        }
    }
}

/// Recomputes a raw-value key: blake3 over the prefix and the tagged
/// raw values of `projection` (or the whole row when absent) — the same
/// bytes the wire's one value encoder writes, so the recomputation can
/// never drift from the footprint's own keys.
fn hash_values(
    prefix: &[u8],
    layout: &[ValueType],
    projection: Option<&[u16]>,
    row: &[Value],
) -> Option<[u8; 32]> {
    if row.len() != layout.len() {
        return None;
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(prefix);
    let fields: Vec<u16> = match projection {
        Some(projection) => projection.to_vec(),
        None => (0..u16::try_from(layout.len()).ok()?).collect(),
    };
    for field in fields {
        let index = usize::from(field);
        if index >= row.len() {
            return None;
        }
        append_value(&mut hasher, &row[index], layout[index]).ok()?;
    }
    Some(*hasher.finalize().as_bytes())
}
