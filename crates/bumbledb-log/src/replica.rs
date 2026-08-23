//! The replica: a local store that is a materialized view of the
//! braids' prefixes, plus the loop that keeps it current. Replicas are
//! disposable by construction — the sidecar is a floor cache with one
//! wholeness check, and recovery is the catch-up loop itself (L10),
//! never a procedure. A corruption-class refusal wedges one braid
//! read-only at its last good slot while the other braids keep serving
//! (L9 makes partial service sound); a phantom — a generation the log
//! never assigned and no pending accounts for — discards the directory
//! and re-pulls.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::Duration;

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::{Admission, Db, SchemaDescriptor, SchemaError, Theory, Violations};

use crate::apply::{Applied, ApplyRefusal, apply};
use crate::braids::BraidId;
use crate::codec::Codec;
use crate::footprint::VocabularyError;
use crate::manifest::{
    Checkpoint, CheckpointError, Manifest, ManifestError, ckpt_json_key, ckpt_mdb_key, log_key,
    manifest_key,
};
use crate::sidecar::Chain;
use crate::store::{Etag, ObjectStore, Poll, StoreError};

/// The gc-safety heartbeat cadence: every N-th `refresh` pass begins
/// with a conditional manifest poll, bounding hole-detection staleness
/// by law rather than by luck. A chosen bounded-staleness knob,
/// re-sized per deployment via [`Replica::set_heartbeat_every`].
pub const HEARTBEAT_EVERY: u64 = 16;

const DATA_FILE: &str = "data.mdb";

/// A braid vector: applied counts keyed by braid id. Any vector is a
/// legal restore point, and pointwise dominance is the read-your-writes
/// order.
pub type Vector = BTreeMap<BraidId, u64>;

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
    /// The theory admitted no codec vocabulary.
    Vocabulary(VocabularyError),
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

enum AttemptEnd {
    Whole,
    Discard,
    Refused(OpenRefusal),
}

enum CatchUpEnd {
    Tips,
    Gap,
    RejectedInOpen,
}

enum Step {
    Applied,
    Tip,
    Gap,
    Wedged,
    Rejected { slot: u64, violations: Violations },
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
    db: Option<Db<T>>,
    chain: Chain,
    provenance: Provenance,
    manifest_etag: Etag,
    floor: Option<([u8; 32], Checkpoint)>,
    ckpt_cache: BTreeMap<[u8; 32], Checkpoint>,
    passes: u64,
    heartbeat_every: u64,
    wedged: BTreeMap<BraidId, Corruption>,
    applied_pending: u64,
}

impl<T: Theory + Clone, S: ObjectStore> Replica<T, S> {
    /// Opens a replica against `prefix` in `store`, materialized at
    /// `dir`: manifest gauntlet, then local-dir open, checkpoint seed,
    /// or bootstrap; pending resolution; catch-up with tip-vs-hole
    /// decided from the current checkpoint vector before probing; and
    /// the wholeness identity before anything serves from a
    /// pre-existing directory.
    pub fn open(store: S, prefix: &str, dir: &Path, theory: T) -> Result<Opened<T, S>, Fault> {
        let (codec, fingerprint) = match derive_codec(&theory) {
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
            db: None,
            chain: Chain {
                entries: BTreeMap::new(),
                pending: None,
            },
            provenance: Provenance::Bootstrap,
            manifest_etag: Etag(String::new()),
            floor: None,
            ckpt_cache: BTreeMap::new(),
            passes: 0,
            heartbeat_every: HEARTBEAT_EVERY,
            wedged: BTreeMap::new(),
            applied_pending: 0,
        };
        match replica.establish()? {
            None => Ok(Opened::Ready(Box::new(replica))),
            Some(refusal) => Ok(Opened::Refused(refusal)),
        }
    }

    /// The engine's own surface — no wrapper query API exists.
    #[must_use]
    pub fn db(&self) -> &Db<T> {
        self.db
            .as_ref()
            .expect("an established replica holds a store")
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
    pub fn refresh(&mut self) -> Result<Refreshed, Fault> {
        self.passes += 1;
        if self.passes.is_multiple_of(self.heartbeat_every)
            && let Some(refusal) = self.heartbeat()?
        {
            return Ok(Refreshed::Refused(refusal));
        }
        // A pending slot may have landed since the last pass; resolving
        // it first keeps the identity's last term from double-counting
        // the commit once catch-up replays the published slot.
        if self.chain.pending.is_some() && self.resolve_pending()?.is_some() {
            self.discard()?;
            return match self.establish()? {
                None => Ok(Refreshed::Vector(self.chain.vector())),
                Some(refusal) => Ok(Refreshed::Refused(refusal)),
            };
        }
        match self.catch_up(Phase::Steady)? {
            CatchUpEnd::Tips => {
                let generation = self.db().generation()?.value();
                if generation == self.chain.sum() + self.applied_pending {
                    return Ok(Refreshed::Vector(self.chain.vector()));
                }
            }
            CatchUpEnd::Gap => {}
            CatchUpEnd::RejectedInOpen => {
                unreachable!("steady-state catch-up never reports the open-phase arm")
            }
        }
        self.discard()?;
        match self.establish()? {
            None => Ok(Refreshed::Vector(self.chain.vector())),
            Some(refusal) => Ok(Refreshed::Refused(refusal)),
        }
    }

    /// One braid's catch-up — cheap point freshness for a known-hot
    /// flow. No heartbeat, no wholeness ceremony: the braid either
    /// reaches its tip, wedges, or exposes a gap that heals through the
    /// full loop.
    pub fn refresh_braid(&mut self, braid: BraidId) -> Result<Refreshed, Fault> {
        loop {
            if self.wedged.contains_key(&braid) {
                return Ok(Refreshed::Vector(self.chain.vector()));
            }
            match self.step(braid)? {
                Step::Applied => {}
                Step::Tip | Step::Wedged => {
                    return Ok(Refreshed::Vector(self.chain.vector()));
                }
                Step::Gap => {
                    self.discard()?;
                    return match self.establish()? {
                        None => Ok(Refreshed::Vector(self.chain.vector())),
                        Some(refusal) => Ok(Refreshed::Refused(refusal)),
                    };
                }
                Step::Rejected { slot, violations } => {
                    self.wedged.insert(
                        braid,
                        Corruption::ReplayDiverged {
                            braid,
                            slot,
                            violations,
                        },
                    );
                    return Ok(Refreshed::Vector(self.chain.vector()));
                }
            }
        }
    }

    /// Refreshes until the replica's vector dominates `target`
    /// pointwise. Commits return `(braid, generation)` pairs; the
    /// pointwise max of every pair a flow has seen is its session
    /// token, and this is the one verb that waits on it.
    pub fn wait_for(&mut self, target: &Vector) -> Result<Waited, Fault> {
        loop {
            if let Some(braid) = target
                .iter()
                .find(|(braid, g)| {
                    self.wedged.contains_key(braid) && self.chain.position(**braid).g < **g
                })
                .map(|(braid, _)| *braid)
            {
                return Ok(Waited::Wedged { braid });
            }
            if target
                .iter()
                .all(|(braid, g)| self.chain.position(*braid).g >= *g)
            {
                return Ok(Waited::Reached(self.chain.vector()));
            }
            match self.refresh()? {
                Refreshed::Vector(_) => {}
                Refreshed::Refused(refusal) => return Ok(Waited::Refused(refusal)),
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Closes the replica and deletes its directory — the disposable
    /// law's verb, used by tenant eviction.
    pub fn dispose(mut self) -> io::Result<()> {
        self.db = None;
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

    fn establish(&mut self) -> Result<Option<OpenRefusal>, Fault> {
        for _ in 0..8 {
            if let Some(refusal) = self.read_manifest()? {
                return Ok(Some(refusal));
            }
            match self.attempt()? {
                AttemptEnd::Whole => return Ok(None),
                AttemptEnd::Discard => self.discard()?,
                AttemptEnd::Refused(refusal) => return Ok(Some(refusal)),
            }
        }
        Err(Fault::Io(io::Error::other(
            "discard-and-re-pull did not converge",
        )))
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
        let Some(doc) = self.store.get(&ckpt_json_key(&self.prefix, &digest))? else {
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

    fn attempt(&mut self) -> Result<AttemptEnd, Fault> {
        match self.mount()? {
            None => {}
            Some(end) => return Ok(end),
        }
        match self.resolve_pending()? {
            None => {}
            Some(end) => return Ok(end),
        }
        // Only a pre-existing directory is in the unproven open phase:
        // a checkpoint-seeded or bootstrapped store is whole by
        // construction, so its rejected replay is the corruption-class
        // verdict (the wedge), never a discard that could loop forever
        // on a poisoned slot.
        let phase = match self.provenance {
            Provenance::LocalDir => Phase::Open,
            Provenance::Bootstrap | Provenance::Checkpoint => Phase::Steady,
        };
        match self.catch_up(phase)? {
            CatchUpEnd::Tips => {}
            CatchUpEnd::Gap | CatchUpEnd::RejectedInOpen => return Ok(AttemptEnd::Discard),
        }
        let generation = self.db().generation()?.value();
        if generation == self.chain.sum() + self.applied_pending {
            Ok(AttemptEnd::Whole)
        } else {
            Ok(AttemptEnd::Discard)
        }
    }

    /// Mounts the local state: pre-existing directory, checkpoint seed,
    /// or bootstrap. `None` means mounted; `Some` short-circuits.
    fn mount(&mut self) -> Result<Option<AttemptEnd>, Fault> {
        self.wedged.clear();
        self.applied_pending = 0;
        if self.dir.exists() {
            match Db::open(&self.dir, self.theory.clone()) {
                Ok(db) => {
                    let Some(Ok(chain)) = Chain::read(&self.dir, self.codec.braids())? else {
                        drop(db);
                        return Ok(Some(AttemptEnd::Discard));
                    };
                    self.db = Some(db);
                    self.chain = chain;
                    self.provenance = Provenance::LocalDir;
                    return Ok(None);
                }
                Err(error @ bumbledb::Error::EnvironmentLocked) => {
                    return Err(Fault::Engine(error));
                }
                Err(_) => return Ok(Some(AttemptEnd::Discard)),
            }
        }
        if let Some((digest, doc)) = self.floor.clone() {
            return self.seed(digest, &doc);
        }
        match Db::create(&self.dir, self.theory.clone())? {
            Admission::Accepted(db) => {
                self.db = Some(db);
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
        self.db = Some(db);
        self.chain = Chain {
            entries: doc
                .braids
                .iter()
                .map(|(braid, head)| {
                    (
                        *braid,
                        crate::sidecar::ChainEntry {
                            g: head.g,
                            prev: head.hash,
                            ts: head.ts,
                        },
                    )
                })
                .collect(),
            pending: None,
        };
        self.chain.write_atomic(&self.dir)?;
        self.provenance = Provenance::Checkpoint;
        Ok(None)
    }

    /// Resolves the pending slot before replay: published pending
    /// clears; a lost slot over an applied pending is a fork (discard);
    /// an unpublished pending fixes the identity's last term.
    fn resolve_pending(&mut self) -> Result<Option<AttemptEnd>, Fault> {
        let Some(pending) = self.chain.pending.clone() else {
            self.applied_pending = 0;
            return Ok(None);
        };
        let key = log_key(&self.prefix, pending.braid, pending.slot);
        let generation = self.db().generation()?.value();
        let sum = self.chain.sum();
        match self.store.get(&key)? {
            Some(fetched) if fetched.bytes == pending.bytes => {
                self.chain.pending = None;
                self.chain.write_atomic(&self.dir)?;
                self.applied_pending = 0;
                Ok(None)
            }
            Some(_) => {
                if generation == sum {
                    self.chain.pending = None;
                    self.chain.write_atomic(&self.dir)?;
                    self.applied_pending = 0;
                    Ok(None)
                } else {
                    Ok(Some(AttemptEnd::Discard))
                }
            }
            None => {
                if generation == sum {
                    self.applied_pending = 0;
                    Ok(None)
                } else if generation == sum + 1 {
                    self.applied_pending = 1;
                    Ok(None)
                } else {
                    Ok(Some(AttemptEnd::Discard))
                }
            }
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
        let db = self
            .db
            .as_ref()
            .expect("an established replica holds a store");
        match apply(
            db,
            &mut self.chain,
            &self.codec,
            braid,
            slot,
            &fetched.bytes,
            self.applied_pending,
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
        self.db = None;
        match fs::remove_dir_all(&self.dir) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(Fault::Io(err)),
        }
    }
}

/// Derives the codec and fingerprint from the theory — the pure prelude
/// of `open` shared with the restore verbs in `crate::gc`.
pub(crate) fn derive_codec<T: Theory + Clone>(
    theory: &T,
) -> Result<(Codec, [u8; 32]), OpenRefusal> {
    let descriptor: SchemaDescriptor = theory.clone().descriptor();
    let schema = descriptor.clone().validate().map_err(OpenRefusal::Theory)?;
    let fingerprint = schema_fingerprint(&schema).0;
    let codec = Codec::new(&descriptor, fingerprint).map_err(OpenRefusal::Vocabulary)?;
    Ok((codec, fingerprint))
}

/// Fetches `ckpt/{digest}.mdb` and verifies the digest, retrying the
/// transfer once — the retry distinguishes a torn transfer from a
/// corrupt object; a second mismatch refuses.
pub(crate) fn fetch_checkpoint_bytes<S: ObjectStore>(
    store: &S,
    prefix: &str,
    digest: [u8; 32],
) -> Result<Result<Vec<u8>, OpenRefusal>, Fault> {
    let key = ckpt_mdb_key(prefix, &digest);
    for attempt in 0..2 {
        let Some(fetched) = store.get(&key)? else {
            return Ok(Err(OpenRefusal::CheckpointObjectMissing { digest }));
        };
        let got = *blake3::hash(&fetched.bytes).as_bytes();
        if got == digest {
            return Ok(Ok(fetched.bytes));
        }
        if attempt == 1 {
            return Ok(Err(OpenRefusal::CheckpointDigestMismatch { digest, got }));
        }
    }
    unreachable!("two attempts always return")
}

/// Writes digest-verified checkpoint bytes into a fresh directory as
/// the store file, fsynced — the shared materialization step under both
/// the replica's checkpoint seed and the restore verbs in `crate::gc`.
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
