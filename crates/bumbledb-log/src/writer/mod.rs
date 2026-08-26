//! The writer: a replica plus the right to create log objects. One
//! commit discipline for both ack modes over the shared pending slot,
//! and ONE loss path between the local store and the bucket, governed
//! by one law — a local commit survives in place iff it maps to its own
//! slot. The publish law holds at the only place it could break: a
//! batch reaches the network only if its local application advanced the
//! generation, so every log slot is a state-changing commit and the
//! wholeness identity stays one integer compare.
//!
//! On a lost slot the writer fetches the winner and byte-compares:
//! byte-equal means the object is ours (an ambiguous PUT absorbed);
//! anything else discards the local directory, re-opens through the
//! replica to the current tip, re-persists the carried pending, and
//! re-judges the recorded ops in one `db.write` — publish on
//! accepted-and-state-changing, `Accepted` at the current generation on
//! accepted-net-no-op, the serial `Rejected` otherwise. Each loop
//! iteration re-opens to tip and races once, so a loss and a
//! re-judgment are the same event, counted once.

mod batch;
mod discipline;
mod drain;
mod duty;
mod loss;
mod open;
mod pending;

pub use batch::Batch;
pub(crate) use batch::{SchemaMaps, schema_maps};
pub(crate) use discipline::{PublishEnd, Settled};
pub(crate) use drain::{Request, Resolved, Waiter};
pub(crate) use loss::Live;
pub(crate) use pending::PendingArm;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};
use std::thread::JoinHandle;

pub(crate) use bumbledb::Theory;
use bumbledb::schema::{Schema, SchemaDescriptor, StatementId};
use bumbledb::{Db, Value, Violations};

use crate::braids::BraidId;
use crate::codec::{Codec, EncodeError, Op};
use crate::lease::{LeaseRefusal, Leases};
use crate::manifest::{Checkpoint, Manifest, create_manifest, manifest_key};
use crate::replica::{Corruption, Fault, OpenRefusal, Vector, derive_codec};
use crate::sidecar::Chain;
pub(crate) use crate::store::ObjectStore;

/// Consecutive losses — each one race at the then-tip — before
/// `Err::Contention`.
pub const LOSS_BOUND: u32 = 16;

/// One drain packs at most this many host writes into one batch.
pub const DRAIN_MAX_WRITES: u64 = 512;

/// One drain packs at most this many estimated batch bytes.
pub const DRAIN_MAX_BYTES: u64 = 4 * 1024 * 1024;

/// Checkpoint cadence, vector-sum arm: a checkpoint after this many
/// applied batches since the current one. 10-protocol owns the value;
/// consumer: the writer's commit-path duty and the duty binary's one
/// cadence check.
pub const CHECKPOINT_EVERY_SUM: u64 = 256;

/// Checkpoint cadence, log-volume arm. 10-protocol owns the value;
/// consumer: the writer's commit-path duty and the duty binary's one
/// cadence check.
pub const CHECKPOINT_EVERY_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) const DATA_FILE: &str = "data.mdb";

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
/// the ack to the end of the local apply — the ack may precede the
/// publish of the one pending batch, so the loss window is exactly that
/// batch, at most one drain's worth, by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckMode {
    Published,
    Local,
}

/// Who holds the prefix. Role is a field on the handle, not an
/// accident of which `open` was called: only a writer births a
/// store; a replica that finds no manifest refuses `ManifestMissing`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Writer,
    Replica,
}

/// Writer construction knobs: the identity and the ack mode.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub writer_id: u64,
    pub ack: AckMode,
}

impl Options {
    #[must_use]
    pub const fn new(writer_id: u64) -> Self {
        Self {
            writer_id,
            ack: AckMode::Published,
        }
    }
}

/// The commit discipline's observable steps, in execution order — the
/// fault-injection seam the conformance crash matrices drive. The
/// re-persist of a carried pending after a loss's re-open reports as
/// `PendingWrite` too: the sidecar write is the same durable act.
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

/// What exhausted the contention bound. `HotKey` carries the statement
/// and the offending fact's raw values from the terminal re-judgment's
/// own rejection — engine-produced, an operable handle. `SlotRace`
/// carries the tip when the terminal re-judgments were
/// accepted-but-outraced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentionCause {
    HotKey {
        statement: StatementId,
        values: Box<[Value]>,
    },
    SlotRace {
        tip: u64,
    },
}

/// The operational signal a resident writer surfaces when a
/// non-byte-equal `Exists` proves it was deposed: both writer ids, from
/// the batch headers. The response already happened — the loss finished
/// through the one path and acks dropped to `Published`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deposition {
    pub braid: BraidId,
    pub slot: u64,
    pub resident: u64,
    pub usurper: u64,
}

/// The writer's error sum. Infrastructure failures ride `Fault`; the
/// named arms are the protocol's own verbs.
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
    /// The loss bound: publication retries on the next commit; an
    /// applied terminal re-judgment stays in `pending`.
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

/// The current checkpoint pointer and its parsed document.
pub(crate) type Floor = Option<([u8; 32], Checkpoint)>;

/// Presence of the local store. Access matches this sum;
/// `Unmounted` refuses — a missing store is not a pointer.
pub(crate) enum WriterState<T: Theory + Clone> {
    Mounted { db: Arc<Db<T>> },
    Unmounted,
}

pub(crate) struct Core<T: Theory + Clone> {
    /// Mounted holds the engine store; Unmounted is the arm access
    /// refuses.
    pub(crate) db: WriterState<T>,
    pub(crate) chain: Chain,
    pub(crate) floor: Floor,
    pub(crate) wedged: BTreeMap<BraidId, Corruption>,
    pub(crate) ack: AckMode,
    pub(crate) deposition: Option<Deposition>,
    pub(crate) ckpt_sum: u64,
    pub(crate) log_bytes: u64,
    pub(crate) cadence_sum: u64,
    pub(crate) cadence_bytes: u64,
    pub(crate) duty_busy: bool,
}

impl<T: Theory + Clone> Core<T> {
    /// Unmounted refuses; access never dereferences a missing store.
    pub(crate) fn db(&self) -> Result<&Db<T>> {
        match &self.db {
            WriterState::Mounted { db } => Ok(db),
            WriterState::Unmounted => Err(Error::Refused(OpenRefusal::Unmounted)),
        }
    }

    pub(crate) fn generation(&self) -> Result<u64> {
        Ok(self.db()?.generation().map_err(Fault::Engine)?.value())
    }
}

pub(crate) struct Inner<T: Theory + Clone, S: ObjectStore, H: StepHook> {
    pub(crate) store: Arc<S>,
    pub(crate) prefix: String,
    pub(crate) dir: PathBuf,
    pub(crate) theory: T,
    pub(crate) codec: Codec,
    pub(crate) schema: Schema,
    pub(crate) fingerprint: [u8; 32],
    pub(crate) writer_id: u64,
    pub(crate) role: Role,
    pub(crate) hook: H,
    pub(crate) losses: AtomicU64,
    /// The set of recent repair signatures. A recurrence of either
    /// trips the alarm, so an A,B,A,B loop is audible on the first
    /// return of A or of B.
    pub(crate) scream: Mutex<Scream>,
    pub(crate) leases: Mutex<Leases>,
    pub(crate) maps: SchemaMaps,
    pub(crate) queues: BTreeMap<BraidId, Mutex<VecDeque<Request>>>,
    pub(crate) core: Mutex<Core<T>>,
    pub(crate) threads: Mutex<Vec<JoinHandle<()>>>,
}

/// Outcome of `Writer::open`.
pub enum WriterOpened<T: Theory + Clone, S: ObjectStore, H: StepHook = NoFaults> {
    Ready(Writer<T, S, H>),
    Refused(OpenRefusal),
}

/// The writer handle. Role is `Role::Writer` — this handle births
/// the store when the manifest is absent. Clones of the inner state
/// ride the detached publisher and checkpoint-duty threads; `quiesce`
/// joins them.
pub struct Writer<T: Theory + Clone, S: ObjectStore, H: StepHook = NoFaults> {
    inner: Arc<Inner<T, S, H>>,
}

/// The legible scream of an unbounded repair loop: a warning every
/// eighth attempt naming the current signature, and an alarm the
/// moment a seen signature recurs. The scream tracks the
/// *set* of recent signatures, not the last one, so an A,B,A,B loop
/// trips on the first recurrence of either.
pub(crate) struct Scream {
    context: &'static str,
    seen: BTreeSet<&'static str>,
    alarmed: BTreeSet<&'static str>,
    attempts: u64,
}

impl Scream {
    const WARN_EVERY: u64 = 8;

    pub(crate) fn new(context: &'static str) -> Self {
        Self {
            context,
            seen: BTreeSet::new(),
            alarmed: BTreeSet::new(),
            attempts: 0,
        }
    }

    /// Records one repair attempt. Returns whether this attempt
    /// tripped the alarm for a recurring signature.
    pub(crate) fn attempt(&mut self, signature: &'static str) -> bool {
        self.attempts += 1;
        let recurred = !self.seen.insert(signature);
        let alarmed = recurred && self.alarmed.insert(signature);
        if alarmed {
            eprintln!(
                "bumbledb-log alarm: {} repair signature recurs: {signature}",
                self.context
            );
        }
        if self.attempts.is_multiple_of(Self::WARN_EVERY) {
            eprintln!(
                "bumbledb-log warning: {} repair attempt {}: {signature}",
                self.context, self.attempts
            );
        }
        alarmed
    }
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl<T: Theory + Clone, S: ObjectStore, H: StepHook> Inner<T, S, H> {
    /// Records one repair signature on the handle's scream. A
    /// signature already in the set trips the alarm.
    pub(crate) fn scream(&self, signature: &'static str) -> bool {
        lock(&self.scream).attempt(signature)
    }
}

impl<T, S> Writer<T, S, NoFaults>
where
    T: Theory + Clone + Send + Sync + 'static,
    S: ObjectStore + 'static,
{
    /// Opens a writer with the production hook.
    ///
    /// # Errors
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
    ///
    /// # Errors
    pub fn open_hooked(
        store: S,
        prefix: &str,
        dir: &Path,
        theory: T,
        options: Options,
        hook: H,
    ) -> Result<WriterOpened<T, S, H>> {
        let (codec, fingerprint, schema) = match derive_codec(&theory) {
            Ok(derived) => derived,
            Err(refusal) => return Ok(WriterOpened::Refused(refusal)),
        };
        let descriptor: SchemaDescriptor = theory.clone().descriptor();
        let maps = schema_maps(&descriptor);
        let store = Arc::new(store);
        let role = Role::Writer;
        match role {
            Role::Writer => Self::birth_store(store.as_ref(), prefix, fingerprint)?,
            Role::Replica => {}
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
            schema,
            fingerprint,
            writer_id: options.writer_id,
            role,
            hook,
            losses: AtomicU64::new(0),
            scream: Mutex::new(Scream::new("writer discard-and-re-pull")),
            leases: Mutex::new(Leases::new(options.writer_id)),
            maps,
            queues,
            core: Mutex::new(Core {
                db: WriterState::Unmounted,
                chain: Chain::Settled {
                    entries: BTreeMap::new(),
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
        });

        {
            let mut core = lock(&inner.core);
            if let Some(refusal) = inner.open_establish(&mut core)? {
                return Ok(WriterOpened::Refused(refusal));
            }
        }
        Ok(WriterOpened::Ready(Self { inner }))
    }

    /// Births the prefix when the manifest is absent. Only a writer
    /// calls this; a replica refuses `ManifestMissing` instead.
    fn birth_store(store: &S, prefix: &str, fingerprint: [u8; 32]) -> Result<()> {
        if store
            .get(&manifest_key(prefix))
            .map_err(|err| Error::Fault(Fault::Store(err)))?
            .is_none()
        {
            let manifest = Manifest {
                fingerprint,
                checkpoint: None,
            };
            let _ = create_manifest(store, prefix, &manifest)
                .map_err(|err| Error::Fault(Fault::Store(err)))?;
        }
        Ok(())
    }

    /// Runs `body` against a fresh batch and commits the recorded ops
    /// under one braid; a spanning batch refuses with the braids named.
    /// The driver never re-invokes `body` — retry is host policy.
    ///
    /// # Errors
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
    ///
    /// # Errors
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
    /// Unmounted refuses; access never dereferences a missing store.
    ///
    /// # Errors
    pub fn with_db<R>(&self, f: impl FnOnce(&Db<T>) -> R) -> Result<R> {
        let core = lock(&self.inner.core);
        Ok(f(core.db()?))
    }

    /// The handle's role. A writer births the store; a replica
    /// refuses an absent manifest.
    #[must_use]
    pub fn role(&self) -> Role {
        self.inner.role
    }

    /// The writer's vector: per-braid applied counts.
    #[must_use]
    pub fn vector(&self) -> Vector {
        lock(&self.inner.core).chain.vector()
    }

    /// The chain the store must match: `generation ≡ generation(chain)`.
    #[must_use]
    pub fn chain(&self) -> Chain {
        lock(&self.inner.core).chain.clone()
    }

    /// Losses since open. A loss and a re-judgment are the same event
    /// under the one path, so this is also the re-judgment count.
    #[must_use]
    pub fn losses(&self) -> u64 {
        self.inner.losses.load(Ordering::Relaxed)
    }

    /// The deposition signal, if a non-byte-equal `Exists` has proven a
    /// resident writer usurped.
    #[must_use]
    pub fn deposition(&self) -> Option<Deposition> {
        lock(&self.inner.core).deposition
    }

    /// The braid holding an applied-but-unpublished batch, if any —
    /// retained through `Contention`, published before the next
    /// commit.
    #[must_use]
    pub fn backlog(&self) -> Option<BraidId> {
        match &lock(&self.inner.core).chain {
            Chain::Pending { batch, .. } => Some(batch.braid),
            Chain::Settled { .. } => None,
        }
    }

    /// Braids wedged read-only by corruption-class verdicts.
    #[must_use]
    pub fn wedged_braids(&self) -> Vec<BraidId> {
        lock(&self.inner.core).wedged.keys().copied().collect()
    }

    /// Re-sizes the checkpoint cadence (both arms; the conformance pins
    /// re-size them).
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
    pub(crate) fn step(&self, step: WriterStep) -> Result<()> {
        match self.hook.observe(step) {
            StepControl::Continue => Ok(()),
            StepControl::Crash => Err(Error::InjectedCrash { step }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Scream;

    #[test]
    fn scream_alarms_on_first_recurrence_in_the_set() {
        let mut scream = Scream::new("test");
        assert!(!scream.attempt("A"));
        assert!(!scream.attempt("B"));
        assert!(scream.attempt("A"));
        assert!(scream.attempt("B"));
        assert!(!scream.attempt("A"));
        assert!(!scream.attempt("C"));
    }

    #[test]
    fn scream_alarms_on_consecutive_repeat() {
        let mut scream = Scream::new("test");
        assert!(!scream.attempt("A"));
        assert!(scream.attempt("A"));
        assert!(!scream.attempt("A"));
    }
}
