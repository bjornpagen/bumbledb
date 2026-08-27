//! The replica: a local store that is a materialized view of the
//! braids' prefixes, plus the loop that keeps it current. Replicas are
//! disposable by construction — the sidecar is a floor cache with one
//! wholeness check, and recovery is the catch-up loop itself (L10),
//! never a procedure. A corruption-class refusal wedges one braid
//! read-only at its last good slot while the other braids keep serving
//! (L9 makes partial service sound); a phantom — a generation the log
//! never assigned and no pending accounts for — discards the directory
//! and re-pulls.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::Duration;

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::{Admission, Db, SchemaDescriptor, SchemaError, Theory, Violations};

use crate::apply::{Applied, ApplyRefusal, PendingFold, apply, fold_pending};
use crate::braids::BraidId;
use crate::codec::Codec;
use crate::manifest::{
    Checkpoint, CheckpointError, Manifest, ManifestError, ckpt_doc_key, ckpt_mdb_key, log_key,
    manifest_key,
};
use crate::sidecar::{CHAIN_FILE, Chain, ChainEntry, SidecarRead};
use crate::store::{Etag, LEASE_NAMESPACE, ObjectStore, Poll, StoreError, TEMP_NAMESPACE};

pub use crate::vector::{CheckpointOrder, Overflow, Vector};

/// The gc-safety heartbeat cadence: every N-th `refresh` pass begins
/// with a conditional manifest poll, bounding hole-detection staleness
/// by law rather than by luck. A chosen bounded-staleness knob,
/// re-sized per deployment via [`Replica::set_heartbeat_every`].
pub const HEARTBEAT_EVERY: u64 = 16;

/// The re-poll cadence of [`Replica::wait_for`], its one consumer: the
/// read-your-writes waiter sleeps this long between refresh passes
/// that have not yet reached the target vector. The value is pinned in
/// `conformance/v3/machine-constants.json`; both machines assert it.
pub const WAIT_FOR_POLL_MS: u64 = 10;

const DATA_FILE: &str = "data.mdb";

/// Infrastructure failure — the transport, the filesystem, or the
/// engine's own environment. Never a protocol outcome.
#[derive(Debug)]
pub enum Fault {
    Store(StoreError),
    Engine(bumbledb::Error),
    Io(io::Error),
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(err) => write!(f, "object store: {err}"),
            Self::Engine(err) => write!(f, "engine: {err}"),
            Self::Io(err) => write!(f, "io: {err}"),
        }
    }
}

impl std::error::Error for Fault {}

impl From<StoreError> for Fault {
    fn from(err: StoreError) -> Self {
        Self::Store(err)
    }
}

impl From<bumbledb::Error> for Fault {
    fn from(err: bumbledb::Error) -> Self {
        Self::Engine(err)
    }
}

impl From<io::Error> for Fault {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Typed refusals at open (and at the heartbeat, which re-runs the same
/// manifest gauntlet). None of these retry: each names a disagreement
/// between declared truths.
#[derive(Debug)]
pub enum OpenRefusal {
    /// The theory itself failed engine validation.
    Theory(SchemaError),
    ManifestMissing,
    Manifest(ManifestError),
    /// The manifest pins a different schema fingerprint.
    FingerprintMismatch {
        manifest: [u8; 32],
        derived: [u8; 32],
    },
    /// The manifest points at a checkpoint document that does not exist.
    CheckpointDocMissing {
        digest: [u8; 32],
    },
    /// The checkpoint document refused to parse — including the braid
    /// set disagreeing with the locally derived decomposition.
    Checkpoint {
        digest: [u8; 32],
        error: CheckpointError,
    },
    /// The manifest points at a checkpoint object that does not exist.
    CheckpointObjectMissing {
        digest: [u8; 32],
    },
    /// Downloaded checkpoint bytes hash to the wrong digest twice — a
    /// torn transfer heals on the retry; this did not.
    CheckpointDigestMismatch {
        digest: [u8; 32],
        got: [u8; 32],
    },
    /// Digest-verified checkpoint bytes refused to open as a store.
    CheckpointOpen {
        digest: [u8; 32],
        error: bumbledb::Error,
    },
    /// The opened checkpoint's generation is not the vector sum it
    /// claims.
    CheckpointState {
        digest: [u8; 32],
        opened: u64,
        sum: u64,
    },
    /// The checkpoint's catalog content claim disagrees with the opened
    /// store — corruption-class, naming the publisher.
    CatalogMismatch {
        digest: [u8; 32],
        writer: u64,
        carried: [u8; 32],
        computed: [u8; 32],
    },
    /// Bootstrap refused: the theory's own admission rejected the empty
    /// store.
    TheoryRejected(Violations),
    /// The local store is unmounted. The stepper refuses this arm —
    /// there is no missing-db pointer to dereference.
    Unmounted,
}

/// The corruption-class verdict wedging one braid: an apply refusal, or
/// a rejected replay on a store that has proven itself whole. The
/// publish law makes both impossible for honest writers.
#[derive(Debug)]
pub enum Corruption {
    Refused(ApplyRefusal),
    ReplayDiverged {
        braid: BraidId,
        slot: u64,
        violations: Violations,
    },
}

/// Where the local directory came from — the provenance that decides
/// read legality: a checkpoint-seeded or bootstrapped store is whole by
/// construction; a pre-existing directory serves nothing until the
/// wholeness check passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    Bootstrap,
    Checkpoint,
    LocalDir,
}

/// Presence of the local store. The stepper matches this sum;
/// `Unmounted` refuses — a missing store is not a pointer.
#[allow(clippy::large_enum_variant)]
pub enum ReplicaState<T: Theory + Clone> {
    Mounted { db: Db<T> },
    Unmounted,
}

impl<T: Theory + Clone> ReplicaState<T> {
    fn db(&self) -> Result<&Db<T>, OpenRefusal> {
        match self {
            Self::Mounted { db } => Ok(db),
            Self::Unmounted => Err(OpenRefusal::Unmounted),
        }
    }
}

/// Role of the handle: a replica refuses a missing manifest; only a
/// writer births one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Replica,
    Writer,
}

/// Outcome of `Replica::open`.
pub enum Opened<T: Theory + Clone, S: ObjectStore> {
    Ready(Box<Replica<T, S>>),
    Refused(OpenRefusal),
}

/// Outcome of a refresh pass.
#[derive(Debug)]
pub enum Refreshed {
    Vector(Vector),
    /// The heartbeat's manifest gauntlet refused — the store's declared
    /// truths changed under us.
    Refused(OpenRefusal),
}

/// Outcome of `wait_for`.
#[derive(Debug)]
pub enum Waited {
    Reached(Vector),
    /// A braid the target needs is wedged below it; no refresh will
    /// ever reach the target.
    Wedged {
        braid: BraidId,
    },
    Refused(OpenRefusal),
}

/// The legible scream of an unbounded repair loop: a warning every
/// eighth attempt naming the current signature, and an alarm the moment
/// a signature is already in the recent set. The loop it serves never
/// caps and never fabricates a convergence error — it repairs forever
/// and says so.
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

    pub(crate) fn attempt(&mut self, signature: &'static str) {
        self.attempts += 1;
        if !self.seen.insert(signature) && self.alarmed.insert(signature) {
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
    }
}

enum AttemptEnd {
    Whole,
    Discard(&'static str),
    Refused(OpenRefusal),
}

enum CatchUpEnd {
    Tips,
    Gap,
    RejectedInOpen,
    Unmounted,
}

enum Step {
    Applied,
    Tip,
    Gap,
    Wedged,
    Rejected { slot: u64, violations: Violations },
    Unmounted,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Open,
    Steady,
}

pub struct Replica<T: Theory + Clone, S: ObjectStore> {
    store: S,
    prefix: String,
    dir: PathBuf,
    theory: T,
    codec: Codec,
    fingerprint: [u8; 32],
    /// Mounted holds the engine store; Unmounted is the arm the
    /// stepper refuses.
    state: ReplicaState<T>,
    chain: Chain,
    provenance: Provenance,
    manifest_etag: Etag,
    floor: Option<([u8; 32], Checkpoint)>,
    audited_floor: Option<[u8; 32]>,
    ckpt_cache: BTreeMap<[u8; 32], Checkpoint>,
    passes: u64,
    heartbeat_every: u64,
    wedged: BTreeMap<BraidId, Corruption>,
}

impl<T: Theory + Clone, S: ObjectStore> Replica<T, S> {
    /// Opens a replica against `prefix` in `store`, materialized at
    /// `dir`: manifest gauntlet, then local-dir open, checkpoint seed,
    /// or bootstrap; pending resolution; catch-up with tip-vs-hole
    /// decided from the current checkpoint vector before probing; and
    /// the wholeness identity before anything serves from a
    /// pre-existing directory. A missing manifest is `ManifestMissing`.
    ///
    /// # Errors
    pub fn open(store: S, prefix: &str, dir: &Path, theory: T) -> Result<Opened<T, S>, Fault> {
        let (codec, fingerprint, _) = match derive_codec(&theory) {
            Ok(derived) => derived,
            Err(refusal) => return Ok(Opened::Refused(refusal)),
        };
        let mut replica = Self {
            store,
            prefix: prefix.to_string(),
            dir: dir.to_path_buf(),
            theory,
            codec,
            fingerprint,
            state: ReplicaState::Unmounted,
            chain: Chain::Settled {
                entries: BTreeMap::new(),
            },
            provenance: Provenance::Bootstrap,
            manifest_etag: Etag(String::new()),
            floor: None,
            audited_floor: None,
            ckpt_cache: BTreeMap::new(),
            passes: 0,
            heartbeat_every: HEARTBEAT_EVERY,
            wedged: BTreeMap::new(),
        };
        sweep_at_open(&replica.store, &replica.prefix, &replica.dir)?;
        match replica.establish()? {
            None => Ok(Opened::Ready(Box::new(replica))),
            Some(refusal) => Ok(Opened::Refused(refusal)),
        }
    }

    /// The engine's own surface — no wrapper query API exists.
    /// Unmounted refuses; the stepper never dereferences a missing store.
    ///
    /// # Errors
    pub fn db(&self) -> Result<&Db<T>, OpenRefusal> {
        self.state.db()
    }

    /// The replica's vector: per-braid applied counts.
    #[must_use]
    pub fn vector(&self) -> Vector {
        self.chain.vector()
    }

    /// Where the current directory came from.
    #[must_use]
    pub const fn provenance(&self) -> Provenance {
        self.provenance
    }

    /// Presence of the local store: `Mounted` or `Unmounted`.
    #[must_use]
    pub const fn state(&self) -> &ReplicaState<T> {
        &self.state
    }

    /// A replica handle: it refuses `ManifestMissing` and never births.
    #[must_use]
    pub const fn role(&self) -> Role {
        Role::Replica
    }

    /// Braids wedged read-only by corruption-class verdicts, with the
    /// verdicts. The other braids keep serving.
    #[must_use]
    pub const fn wedged(&self) -> &BTreeMap<BraidId, Corruption> {
        &self.wedged
    }

    /// Re-sizes the heartbeat cadence (floor 1).
    pub fn set_heartbeat_every(&mut self, passes: u64) {
        self.heartbeat_every = passes.max(1);
    }

    /// One catch-up pass over all braids; returns the vector. Every
    /// N-th pass begins with the conditional manifest poll that keeps
    /// tip-vs-hole honest for long-lived replicas.
    ///
    /// # Errors
    pub fn refresh(&mut self) -> Result<Refreshed, Fault> {
        self.step_pass(Phase::Steady)
    }

    /// `refresh` until this vector dominates `target`.
    ///
    /// # Errors
    pub fn wait_for(&mut self, target: &Vector) -> Result<Waited, Fault> {
        loop {
            let have = self.chain.vector();
            if let Some(braid) = target.braids().find(|&braid| {
                self.wedged.contains_key(&braid) && have.at(braid) < target.at(braid)
            }) {
                return Ok(Waited::Wedged { braid });
            }
            if have.dominates(target) {
                return Ok(Waited::Reached(have));
            }
            match self.refresh()? {
                Refreshed::Vector(_) => {}
                Refreshed::Refused(refusal) => return Ok(Waited::Refused(refusal)),
            }
            std::thread::sleep(Duration::from_millis(WAIT_FOR_POLL_MS));
        }
    }

    /// Closes the replica and deletes its directory — the disposable
    /// law's verb, used by tenant eviction.
    ///
    /// # Errors
    pub fn dispose(mut self) -> io::Result<()> {
        self.state = ReplicaState::Unmounted;
        match fs::remove_dir_all(&self.dir) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }

    /// The local directory this replica materializes into.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    #[must_use]
    pub fn store(&self) -> &S {
        &self.store
    }

    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    #[must_use]
    pub const fn codec(&self) -> &Codec {
        &self.codec
    }

    #[must_use]
    pub const fn chain(&self) -> &Chain {
        &self.chain
    }

    /// The current checkpoint's vector sum, or zero when the manifest
    /// still says `checkpoint: null`.
    #[must_use]
    pub fn checkpoint_sum(&self) -> u64 {
        self.floor.as_ref().map_or(0, |(_, doc)| doc.sum())
    }

    /// The current checkpoint's applied count for `braid`, or zero.
    #[must_use]
    pub fn checkpoint_g(&self, braid: BraidId) -> u64 {
        self.floor
            .as_ref()
            .and_then(|(_, doc)| doc.braids.get(&braid).map(|head| head.g))
            .unwrap_or(0)
    }

    /// Re-read the manifest pointer. A checkpointer that just published
    /// adopts the floor it installed so the next cadence check is
    /// against the new sum, not the pre-publish one.
    ///
    /// # Errors
    pub fn pull_manifest(&mut self) -> Result<Option<OpenRefusal>, Fault> {
        self.read_manifest()
    }

    fn establish(&mut self) -> Result<Option<OpenRefusal>, Fault> {
        let mut scream = Scream::new("replica discard-and-re-pull");
        loop {
            if let Some(refusal) = self.read_manifest()? {
                return Ok(Some(refusal));
            }
            match self.attempt()? {
                AttemptEnd::Whole => return Ok(None),
                AttemptEnd::Discard(signature) => {
                    self.discard()?;
                    scream.attempt(signature);
                }
                AttemptEnd::Refused(refusal) => return Ok(Some(refusal)),
            }
        }
    }

    fn read_manifest(&mut self) -> Result<Option<OpenRefusal>, Fault> {
        let Some(fetched) = self.store.get(&manifest_key(&self.prefix))? else {
            return Ok(Some(OpenRefusal::ManifestMissing));
        };
        let refusal = self.adopt_manifest(&fetched.bytes)?;
        if refusal.is_none() {
            self.manifest_etag = fetched.etag;
        }
        Ok(refusal)
    }

    fn adopt_manifest(&mut self, bytes: &[u8]) -> Result<Option<OpenRefusal>, Fault> {
        let manifest = match Manifest::parse(bytes) {
            Ok(manifest) => manifest,
            Err(error) => return Ok(Some(OpenRefusal::Manifest(error))),
        };
        if manifest.fingerprint != self.fingerprint {
            return Ok(Some(OpenRefusal::FingerprintMismatch {
                manifest: manifest.fingerprint,
                derived: self.fingerprint,
            }));
        }
        let Some(digest) = manifest.checkpoint else {
            self.floor = None;
            return Ok(None);
        };
        if let Some(doc) = self.ckpt_cache.get(&digest) {
            self.floor = Some((digest, doc.clone()));
            return Ok(None);
        }
        let Some(doc) = self.store.get(&ckpt_doc_key(&self.prefix, &digest))? else {
            return Ok(Some(OpenRefusal::CheckpointDocMissing { digest }));
        };
        let doc = match Checkpoint::parse(&doc.bytes, self.codec.braids()) {
            Ok(doc) => doc,
            Err(error) => return Ok(Some(OpenRefusal::Checkpoint { digest, error })),
        };
        self.ckpt_cache.insert(digest, doc.clone());
        self.floor = Some((digest, doc));
        Ok(None)
    }

    fn heartbeat(&mut self) -> Result<Option<OpenRefusal>, Fault> {
        match self
            .store
            .get_if_changed(&manifest_key(&self.prefix), &self.manifest_etag)?
        {
            Poll::Unchanged => Ok(None),
            Poll::Changed(fetched) => {
                let refusal = self.adopt_manifest(&fetched.bytes)?;
                if refusal.is_none() {
                    self.manifest_etag = fetched.etag;
                }
                Ok(refusal)
            }
        }
    }

    /// The one stepper: heartbeat, pending fold, one slot per braid,
    /// wholeness, then serve or reseed. Unmounted refuses — the
    /// stepper never applies without a store.
    fn step_pass(&mut self, phase: Phase) -> Result<Refreshed, Fault> {
        match &self.state {
            ReplicaState::Unmounted => return Ok(Refreshed::Refused(OpenRefusal::Unmounted)),
            ReplicaState::Mounted { .. } => {}
        }
        self.passes += 1;
        if self.passes.is_multiple_of(self.heartbeat_every)
            && let Some(refusal) = self.heartbeat()?
        {
            return Ok(Refreshed::Refused(refusal));
        }
        if matches!(self.chain, Chain::Pending { .. }) && self.resolve_pending()?.is_some() {
            return self.reseed();
        }
        match self.catch_up(phase)? {
            CatchUpEnd::Tips => match self.whole()? {
                Ok(true) => {
                    if let Some(refusal) = self.audit_reached_floor()? {
                        return Ok(Refreshed::Refused(refusal));
                    }
                    return Ok(Refreshed::Vector(self.chain.vector()));
                }
                Ok(false) => {}
                Err(refusal) => return Ok(Refreshed::Refused(refusal)),
            },
            CatchUpEnd::Gap => {}
            CatchUpEnd::RejectedInOpen => {
                unreachable!("steady-state catch-up never reports the open-phase arm")
            }
            CatchUpEnd::Unmounted => return Ok(Refreshed::Refused(OpenRefusal::Unmounted)),
        }
        self.reseed()
    }

    fn reseed(&mut self) -> Result<Refreshed, Fault> {
        self.discard()?;
        match self.establish()? {
            None => Ok(Refreshed::Vector(self.chain.vector())),
            Some(refusal) => Ok(Refreshed::Refused(refusal)),
        }
    }

    fn whole(&self) -> Result<Result<bool, OpenRefusal>, Fault> {
        let generation = match self.state.db() {
            Ok(db) => db.generation()?.value(),
            Err(refusal) => return Ok(Err(refusal)),
        };
        Ok(Ok(
            generation == self.chain.sum() || generation == self.chain.generation()
        ))
    }

    fn attempt(&mut self) -> Result<AttemptEnd, Fault> {
        match self.mount()? {
            None => {}
            Some(end) => return Ok(end),
        }
        match self.resolve_pending()? {
            None => {}
            Some(end) => return Ok(end),
        }
        let phase = match self.provenance {
            Provenance::LocalDir => Phase::Open,
            Provenance::Bootstrap | Provenance::Checkpoint => Phase::Steady,
        };
        match self.catch_up(phase)? {
            CatchUpEnd::Tips => {}
            CatchUpEnd::Gap => {
                return Ok(AttemptEnd::Discard("catch-up hit a hole below the floor"));
            }
            CatchUpEnd::RejectedInOpen => {
                return Ok(AttemptEnd::Discard("replay rejected in the open phase"));
            }
            CatchUpEnd::Unmounted => {
                return Ok(AttemptEnd::Refused(OpenRefusal::Unmounted));
            }
        }
        match self.whole()? {
            Ok(true) => {}
            Ok(false) => {
                return Ok(AttemptEnd::Discard(
                    "the wholeness identity failed after catch-up",
                ));
            }
            Err(refusal) => return Ok(AttemptEnd::Refused(refusal)),
        }
        if let Some(refusal) = self.audit_reached_floor()? {
            return Ok(AttemptEnd::Refused(refusal));
        }
        Ok(AttemptEnd::Whole)
    }

    /// The replay-reaching half of the checkpoint content claim: a
    /// store standing at exactly the current checkpoint's vector by its
    /// own count compares its computed catalog digest against the
    /// carried claim and refuses a mismatch as corruption-class, naming
    /// the publisher. The seed path verifies the same claim over the
    /// downloaded bytes, so between the two paths a checkpoint is
    /// audited from independent directions while its history is still
    /// replayable. A passing digest is remembered — the claim is
    /// immutable, so one comparison per checkpoint is the whole cost.
    fn audit_reached_floor(&mut self) -> Result<Option<OpenRefusal>, Fault> {
        let Some((digest, doc)) = &self.floor else {
            return Ok(None);
        };
        if self.audited_floor == Some(*digest)
            || matches!(self.chain, Chain::Pending { .. })
            || doc.vector() != self.chain.vector()
        {
            return Ok(None);
        }
        let computed = match self.state.db() {
            Ok(db) => db.catalog_digest()?,
            Err(refusal) => return Ok(Some(refusal)),
        };
        if computed == doc.catalog {
            self.audited_floor = Some(*digest);
            return Ok(None);
        }
        Ok(Some(OpenRefusal::CatalogMismatch {
            digest: *digest,
            writer: doc.writer,
            carried: doc.catalog,
            computed,
        }))
    }

    /// Mounts the local state: pre-existing directory, checkpoint seed,
    /// or bootstrap. `None` means mounted; `Some` short-circuits.
    fn mount(&mut self) -> Result<Option<AttemptEnd>, Fault> {
        self.wedged.clear();
        if self.dir.exists() {
            match Db::open(&self.dir, self.theory.clone()) {
                Ok(db) => match Chain::read(&self.dir, self.codec.braids()) {
                    SidecarRead::Read(chain) => {
                        self.state = ReplicaState::Mounted { db };
                        self.chain = chain;
                        self.provenance = Provenance::LocalDir;
                        Ok(None)
                    }
                    SidecarRead::Fault(err) => {
                        drop(db);
                        Err(Fault::Io(err))
                    }
                    SidecarRead::Absent | SidecarRead::Corrupt(_) => {
                        drop(db);
                        Ok(Some(AttemptEnd::Discard("the sidecar refused to read")))
                    }
                },
                Err(error @ bumbledb::Error::EnvironmentLocked) => Err(Fault::Engine(error)),
                Err(_) => Ok(Some(AttemptEnd::Discard(
                    "the local directory refused to open",
                ))),
            }
        } else if let Some((digest, doc)) = self.floor.clone() {
            self.seed(digest, &doc)
        } else {
            match Db::create(&self.dir, self.theory.clone())? {
                Admission::Accepted(db) => {
                    self.state = ReplicaState::Mounted { db };
                    self.chain = Chain::genesis(self.codec.braids());
                    self.chain.write_atomic(&self.dir)?;
                    self.provenance = Provenance::Bootstrap;
                    Ok(None)
                }
                Admission::Rejected(violations) => Ok(Some(AttemptEnd::Refused(
                    OpenRefusal::TheoryRejected(violations),
                ))),
            }
        }
    }

    fn seed(&mut self, digest: [u8; 32], doc: &Checkpoint) -> Result<Option<AttemptEnd>, Fault> {
        let bytes = match fetch_checkpoint_bytes(&self.store, &self.prefix, digest)? {
            Ok(bytes) => bytes,
            Err(refusal) => return Ok(Some(AttemptEnd::Refused(refusal))),
        };
        write_checkpoint_bytes(&self.dir, &bytes)?;
        let db = match Db::open(&self.dir, self.theory.clone()) {
            Ok(db) => db,
            Err(error @ (bumbledb::Error::Io(_) | bumbledb::Error::EnvironmentLocked)) => {
                return Err(Fault::Engine(error));
            }
            Err(error) => {
                return Ok(Some(AttemptEnd::Refused(OpenRefusal::CheckpointOpen {
                    digest,
                    error,
                })));
            }
        };
        let opened = db.generation()?.value();
        if opened != doc.sum() {
            return Ok(Some(AttemptEnd::Refused(OpenRefusal::CheckpointState {
                digest,
                opened,
                sum: doc.sum(),
            })));
        }
        let computed = db.catalog_digest()?;
        if computed != doc.catalog {
            return Ok(Some(AttemptEnd::Refused(OpenRefusal::CatalogMismatch {
                digest,
                writer: doc.writer,
                carried: doc.catalog,
                computed,
            })));
        }
        self.audited_floor = Some(digest);
        self.state = ReplicaState::Mounted { db };
        self.chain = Chain::Settled {
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
        };
        self.chain.write_atomic(&self.dir)?;
        self.provenance = Provenance::Checkpoint;
        Ok(None)
    }

    /// Inherited pending: one fold against occupant, generation, and
    /// floor. Settled is written after the fold, never ahead of it.
    fn resolve_pending(&mut self) -> Result<Option<AttemptEnd>, Fault> {
        let Chain::Pending { batch, .. } = &self.chain else {
            return Ok(None);
        };
        let pending = batch.clone();
        let key = log_key(&self.prefix, pending.braid, pending.slot);
        let occupant = self.store.get(&key)?.map(|fetched| fetched.bytes);
        let below_floor = self
            .floor
            .as_ref()
            .and_then(|(_, doc)| doc.braids.get(&pending.braid))
            .is_some_and(|head| pending.slot <= head.g);
        let generation = match self.state.db() {
            Ok(db) => db.generation()?.value(),
            Err(refusal) => return Ok(Some(AttemptEnd::Refused(refusal))),
        };
        match fold_pending(
            self.chain.sum(),
            generation,
            occupant.as_deref(),
            &pending.bytes,
            below_floor,
        ) {
            PendingFold::Ours | PendingFold::TheirsUnapplied | PendingFold::BelowFloor => {
                let entries = self.chain.entries().clone();
                self.chain = Chain::Settled { entries };
                self.chain.write_atomic(&self.dir)?;
                Ok(None)
            }
            PendingFold::AbsentUnapplied | PendingFold::AbsentApplied => Ok(None),
            PendingFold::TheirsApplied => Ok(Some(AttemptEnd::Discard(
                "a lost slot over an applied pending is a fork",
            ))),
            PendingFold::Phantom => Ok(Some(AttemptEnd::Discard(
                "a generation no pending term accounts for",
            ))),
        }
    }

    /// Round-robin catch-up: one slot per braid per round, so a hot
    /// braid cannot starve the others' freshness.
    fn catch_up(&mut self, phase: Phase) -> Result<CatchUpEnd, Fault> {
        let braids: Vec<BraidId> = self.codec.braids().components().keys().copied().collect();
        let mut at_tip: std::collections::BTreeSet<BraidId> = std::collections::BTreeSet::new();
        loop {
            let mut progressed = false;
            for braid in &braids {
                if at_tip.contains(braid) || self.wedged.contains_key(braid) {
                    continue;
                }
                match self.step(*braid)? {
                    Step::Applied => progressed = true,
                    Step::Tip => {
                        at_tip.insert(*braid);
                    }
                    Step::Wedged => {}
                    Step::Gap => return Ok(CatchUpEnd::Gap),
                    Step::Unmounted => return Ok(CatchUpEnd::Unmounted),
                    Step::Rejected { slot, violations } => match phase {
                        Phase::Open => return Ok(CatchUpEnd::RejectedInOpen),
                        Phase::Steady => {
                            self.wedged.insert(
                                *braid,
                                Corruption::ReplayDiverged {
                                    braid: *braid,
                                    slot,
                                    violations,
                                },
                            );
                        }
                    },
                }
            }
            if !progressed {
                return Ok(CatchUpEnd::Tips);
            }
        }
    }

    fn step(&mut self, braid: BraidId) -> Result<Step, Fault> {
        let position = self.chain.position(braid);
        let slot = position.g + 1;
        let key = log_key(&self.prefix, braid, slot);
        let Some(fetched) = self.store.get(&key)? else {
            let hole = self
                .floor
                .as_ref()
                .and_then(|(_, doc)| doc.braids.get(&braid))
                .is_some_and(|head| slot <= head.g);
            return Ok(if hole { Step::Gap } else { Step::Tip });
        };
        match apply(
            match &self.state {
                ReplicaState::Mounted { db } => db,
                ReplicaState::Unmounted => return Ok(Step::Unmounted),
            },
            &mut self.chain,
            &self.codec,
            braid,
            slot,
            &fetched.bytes,
        )? {
            Applied::Advanced { .. } | Applied::Absorbed { .. } => {
                self.chain.write_atomic(&self.dir)?;
                Ok(Step::Applied)
            }
            Applied::Rejected(violations) => Ok(Step::Rejected { slot, violations }),
            Applied::Refused(refusal) => {
                self.wedged.insert(braid, Corruption::Refused(refusal));
                Ok(Step::Wedged)
            }
        }
    }

    fn discard(&mut self) -> Result<(), Fault> {
        self.state = ReplicaState::Unmounted;
        match fs::remove_dir_all(&self.dir) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(Fault::Io(err)),
        }
    }
}

/// Derives the codec, fingerprint, and validated schema from the theory
/// — the pure prelude of `open` shared with the restore verbs in
/// `crate::gc` and with the writer, whose contention cause reads
/// statement identities off the schema.
pub(crate) fn derive_codec<T: Theory + Clone>(
    theory: &T,
) -> Result<(Codec, [u8; 32], bumbledb::Schema), OpenRefusal> {
    let descriptor: SchemaDescriptor = theory.clone().descriptor();
    let schema = descriptor.clone().validate().map_err(OpenRefusal::Theory)?;
    let fingerprint = schema_fingerprint(&schema).0;
    let codec = Codec::new(&descriptor, fingerprint);
    Ok((codec, fingerprint, schema))
}

/// Fetches the store snapshot paired with the checkpoint document
/// `ckpt/{digest}`. The pair shares the document digest as its name;
/// the catalog claim is audited after open, not against this object's
/// bytes.
pub(crate) fn fetch_checkpoint_bytes<S: ObjectStore>(
    store: &S,
    prefix: &str,
    digest: [u8; 32],
) -> Result<Result<Vec<u8>, OpenRefusal>, Fault> {
    let key = ckpt_mdb_key(prefix, &digest);
    match store.get(&key)? {
        Some(fetched) => Ok(Ok(fetched.bytes)),
        None => Ok(Err(OpenRefusal::CheckpointObjectMissing { digest })),
    }
}

/// Writes checkpoint bytes into a fresh directory as the store file,
/// fsynced — the shared materialization step under both the replica's
/// checkpoint seed and the restore verbs in `crate::gc`.
pub(crate) fn write_checkpoint_bytes(dir: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let data = dir.join(DATA_FILE);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&data)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    File::open(dir)?.sync_all()
}

/// The reserved-namespace scratch lease that names an in-flight
/// checkpoint candidate. The successor GETs this document at open.
pub const CKPT_SCRATCH_LEASE: &str = "ckpt-scratch";

/// `{dir}/~lease/ckpt-scratch` — known path, no LIST.
#[must_use]
pub fn ckpt_scratch_path(dir: &Path) -> PathBuf {
    dir.join(LEASE_NAMESPACE).join(CKPT_SCRATCH_LEASE)
}

const SCRATCH_VERSION: u8 = 3;

/// The scratch-lease body: version byte 3, then the 32-byte digest.
#[must_use]
pub fn encode_ckpt_scratch(digest: &[u8; 32]) -> [u8; 33] {
    let mut body = [0u8; 33];
    body[0] = SCRATCH_VERSION;
    body[1..].copy_from_slice(digest);
    body
}

/// The digest a scratch-lease body names, or none.
#[must_use]
pub fn parse_ckpt_scratch(bytes: &[u8]) -> Option<[u8; 32]> {
    if bytes.len() != 33 || bytes.first().copied()? != SCRATCH_VERSION {
        return None;
    }
    bytes[1..].try_into().ok()
}

/// Records `digest` in the scratch lease before the upload-before-decision
/// window. The successor GETs this document at open.
///
/// # Errors
pub fn record_ckpt_scratch(dir: &Path, digest: &[u8; 32]) -> io::Result<()> {
    let path = ckpt_scratch_path(dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)?;
    file.write_all(&encode_ckpt_scratch(digest))?;
    file.sync_all()?;
    drop(file);
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}

/// Drops the scratch lease after the candidate is live, reclaimed, or
/// never uploaded.
///
/// # Errors
pub fn clear_ckpt_scratch(dir: &Path) -> io::Result<()> {
    match fs::remove_file(ckpt_scratch_path(dir)) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

/// Deletes `ckpt/{digest}.mdb` and `ckpt/{digest}` as one unit. A missing object
/// is already gone.
///
/// # Errors
pub fn reclaim_orphan<S: ObjectStore>(
    store: &S,
    prefix: &str,
    digest: &[u8; 32],
) -> Result<(), StoreError> {
    store.delete(&ckpt_mdb_key(prefix, digest))?;
    store.delete(&ckpt_doc_key(prefix, digest))?;
    Ok(())
}

/// Any successor reclaims the predecessor's reserved temps, sidecar
/// temps, sibling compact scratch, and the crash-strand scratch lease.
///
/// # Errors
pub fn sweep_at_open<S: ObjectStore>(store: &S, prefix: &str, dir: &Path) -> Result<(), Fault> {
    sweep_ckpt_scratch(store, prefix, dir)?;
    sweep_local_litter(dir)?;
    Ok(())
}

fn sweep_ckpt_scratch<S: ObjectStore>(store: &S, prefix: &str, dir: &Path) -> Result<(), Fault> {
    let path = ckpt_scratch_path(dir);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(Fault::Io(err)),
    };
    let Some(digest) = parse_ckpt_scratch(&bytes) else {
        let _ = fs::remove_file(&path);
        return Ok(());
    };
    if live_head(store, prefix)? != Some(digest) {
        reclaim_orphan(store, prefix, &digest)?;
    }
    let _ = fs::remove_file(&path);
    Ok(())
}

fn live_head<S: ObjectStore>(store: &S, prefix: &str) -> Result<Option<[u8; 32]>, Fault> {
    let Some(fetched) = store.get(&manifest_key(prefix))? else {
        return Ok(None);
    };
    Ok(Manifest::parse(&fetched.bytes)
        .ok()
        .and_then(|manifest| manifest.checkpoint))
}

fn sweep_local_litter(dir: &Path) -> io::Result<()> {
    let tmp = dir.join(TEMP_NAMESPACE);
    match fs::remove_dir_all(&tmp) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    sweep_sidecar_temp(dir)?;
    sweep_sibling_scratch(dir)
}

/// `{dir}/.chain.tmp` — known path.
fn sidecar_temp_path(dir: &Path) -> PathBuf {
    dir.join(format!(".{CHAIN_FILE}.tmp"))
}

/// `{dir}.ckpt` — known path. The resident duty writes here.
pub(crate) fn compact_scratch_path(dir: &Path) -> PathBuf {
    PathBuf::from(format!("{}.ckpt", dir.display()))
}

/// `{dir}.duty-ckpt` — known path. The detached checkpointer writes here.
pub(crate) fn duty_scratch_path(dir: &Path) -> PathBuf {
    PathBuf::from(format!("{}.duty-ckpt", dir.display()))
}

fn sweep_sidecar_temp(dir: &Path) -> io::Result<()> {
    match fs::remove_file(sidecar_temp_path(dir)) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn sweep_sibling_scratch(dir: &Path) -> io::Result<()> {
    for path in [compact_scratch_path(dir), duty_scratch_path(dir)] {
        match fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

#[cfg(test)]
mod machine_constants {
    use super::{HEARTBEAT_EVERY, WAIT_FOR_POLL_MS};

    /// The shared-constants table: one value per fact, both machines
    /// assert against it, so a unilateral edit is red here.
    const TABLE: &str = include_str!("../conformance/v3/machine-constants.json");

    fn pinned(name: &str) -> u64 {
        let key = format!("\"{name}\": \"");
        let start = TABLE
            .find(&key)
            .unwrap_or_else(|| panic!("{name} is absent from the machine-constants table"))
            + key.len();
        let rest = &TABLE[start..];
        let end = rest.find('"').expect("a quoted decimal value");
        rest[..end].parse().expect("a decimal u64")
    }

    #[test]
    fn shared_constants_match_the_conformance_table() {
        assert_eq!(WAIT_FOR_POLL_MS, pinned("wait_for_poll_ms"));
        assert_eq!(HEARTBEAT_EVERY, pinned("heartbeat_every"));
        assert_eq!(u64::from(crate::writer::LOSS_BOUND), pinned("loss_bound"));
        assert_eq!(crate::lease::LEASE_WIDTH, pinned("lease_width"));
    }
}

#[cfg(test)]
mod sweep_tests {
    use super::*;
    use crate::store::mem::MemStore;
    use crate::store::{Create, ObjectStore};

    fn temp_dir(tag: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "bdb-log-ckpt-{tag}-{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("scratch dir");
        path
    }

    #[test]
    fn kept_loser_reclaims_its_own_digest() {
        let store = MemStore::new();
        let digest = [0x11u8; 32];
        assert!(matches!(
            store
                .put_create(&ckpt_doc_key("", &digest), b"doc")
                .expect("doc"),
            Create::Created(_)
        ));
        assert!(matches!(
            store
                .put_create(&ckpt_mdb_key("", &digest), b"mdb")
                .expect("mdb"),
            Create::Created(_)
        ));
        reclaim_orphan(&store, "", &digest).expect("reclaim");
        assert!(
            store
                .get(&ckpt_doc_key("", &digest))
                .expect("get")
                .is_none()
        );
        assert!(
            store
                .get(&ckpt_mdb_key("", &digest))
                .expect("get")
                .is_none()
        );
    }

    #[test]
    fn successor_sweeps_scratch_lease_and_local_litter() {
        let store = MemStore::new();
        let dir = temp_dir("open");
        let digest = [0x22u8; 32];
        record_ckpt_scratch(&dir, &digest).expect("lease");
        store
            .put_create(&ckpt_doc_key("", &digest), b"doc")
            .expect("doc");
        store
            .put_create(&ckpt_mdb_key("", &digest), b"mdb")
            .expect("mdb");

        let tmp = dir.join(TEMP_NAMESPACE).join("litter");
        fs::create_dir_all(tmp.parent().expect("parent")).expect("tmp");
        fs::write(&tmp, b"tmp").expect("write tmp");
        fs::write(sidecar_temp_path(&dir), b"sidecar").expect("sidecar");
        let sibling = compact_scratch_path(&dir);
        fs::create_dir_all(&sibling).expect("sibling");
        fs::write(sibling.join("data.mdb"), b"x").expect("scratch bytes");
        let duty = duty_scratch_path(&dir);
        fs::create_dir_all(&duty).expect("duty scratch");

        sweep_at_open(&store, "", &dir).expect("sweep");

        assert!(
            store
                .get(&ckpt_doc_key("", &digest))
                .expect("get")
                .is_none()
        );
        assert!(
            store
                .get(&ckpt_mdb_key("", &digest))
                .expect("get")
                .is_none()
        );
        assert!(!ckpt_scratch_path(&dir).exists());
        assert!(!dir.join(TEMP_NAMESPACE).exists());
        assert!(!sidecar_temp_path(&dir).exists());
        assert!(!sibling.exists());
        assert!(!duty.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sweep_spares_the_live_head() {
        let store = MemStore::new();
        let dir = temp_dir("live");
        let digest = [0x33u8; 32];
        let manifest = Manifest {
            fingerprint: [0x44; 32],
            checkpoint: Some(digest),
        };
        store
            .put_create(&manifest_key(""), &manifest.render())
            .expect("manifest");
        store
            .put_create(&ckpt_doc_key("", &digest), b"doc")
            .expect("doc");
        store
            .put_create(&ckpt_mdb_key("", &digest), b"mdb")
            .expect("mdb");
        record_ckpt_scratch(&dir, &digest).expect("lease");

        sweep_at_open(&store, "", &dir).expect("sweep");

        assert!(
            store
                .get(&ckpt_doc_key("", &digest))
                .expect("get")
                .is_some()
        );
        assert!(
            store
                .get(&ckpt_mdb_key("", &digest))
                .expect("get")
                .is_some()
        );
        assert!(!ckpt_scratch_path(&dir).exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
