//! Open and the disposable law: mount, seed, catch-up, the wholeness
//! identity, and discard-and-re-pull with the legible scream.

use std::fs;
use std::io;
use std::sync::Arc;

use bumbledb::{Admission, Db};

use crate::apply::{Applied, apply};
use crate::braids::BraidId;
use crate::manifest::{Checkpoint, Manifest, log_key, manifest_key};
use crate::replica::{
    Corruption, Fault, OpenRefusal, Scream, fetch_checkpoint_bytes, write_checkpoint_bytes,
};
use crate::sidecar::{Chain, ChainEntry, Pending};

use super::{
    Core, Error, Floor, Inner, ObjectStore, PendingArm, Result, StepHook, Theory, WriterStep,
};

pub(crate) enum MountEnd<T: Theory + Clone> {
    Mounted {
        db: Arc<Db<T>>,
        chain: Chain,
        /// A pre-existing directory is in the unproven open phase until
        /// the wholeness identity passes; a seeded or bootstrapped
        /// store is whole by construction.
        pre_existing: bool,
    },
    Discard(&'static str),
    Refused(OpenRefusal),
}

pub(crate) enum CatchUp {
    Tips,
    Gap,
    RejectedInOpen,
}

impl<T, S, H> Inner<T, S, H>
where
    T: Theory + Clone + Send + Sync + 'static,
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    /// Establish at open: manifest gauntlet, mount, pending arms 1 and
    /// 2 inline (arm 3 marks the backlog), catch-up skipping the
    /// backlog braid, and the wholeness identity. `Some` is a refusal;
    /// `None` leaves the core established. The loop repairs forever
    /// with the legible scream — a healthy remote converges, and a
    /// remote that keeps tearing keeps saying so.
    pub(crate) fn open_establish(
        self: &Arc<Self>,
        core: &mut Core<T>,
    ) -> Result<Option<OpenRefusal>> {
        let mut scream = Scream::new("writer open discard-and-re-pull");
        loop {
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
                MountEnd::Discard(signature) => {
                    self.discard_dir()?;
                    scream.attempt(signature);
                    continue;
                }
                MountEnd::Refused(refusal) => return Ok(Some(refusal)),
            };
            core.db = Some(db);
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
                        scream.attempt("the pending arm convicted a torn store");
                        continue;
                    }
                }
            }
            match self.catch_up(core, skip, applied, pre_existing)? {
                CatchUp::Tips => {}
                CatchUp::Gap => {
                    core.db = None;
                    self.discard_dir()?;
                    scream.attempt("catch-up hit a hole below the floor");
                    continue;
                }
                CatchUp::RejectedInOpen => {
                    core.db = None;
                    self.discard_dir()?;
                    scream.attempt("replay rejected in the open phase");
                    continue;
                }
            }
            if core.generation()? == core.chain.sum() + applied {
                core.ckpt_sum = core.floor.as_ref().map_or(0, |(_, doc)| doc.sum());
                return Ok(None);
            }
            core.db = None;
            self.discard_dir()?;
            scream.attempt("the wholeness identity failed after catch-up");
        }
    }

    /// The disposable law mid-commit: drop the store, delete the
    /// directory, rebuild winner-current from the bucket, and
    /// re-persist any carried pending into the fresh sidecar before the
    /// caller re-judges — so recovery stays crash-idempotent at every
    /// prefix. The loop repairs forever with the legible scream.
    pub(crate) fn re_establish(&self, core: &mut Core<T>, carry: Option<Pending>) -> Result<()> {
        core.db = None;
        self.discard_dir()?;
        let mut scream = Scream::new("writer discard-and-re-pull");
        loop {
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
                MountEnd::Discard(signature) => {
                    self.discard_dir()?;
                    scream.attempt(signature);
                    continue;
                }
                MountEnd::Refused(refusal) => return Err(Error::Refused(refusal)),
            };
            core.db = Some(db);
            core.chain = chain;
            core.wedged.clear();
            match self.catch_up(core, None, 0, pre_existing)? {
                CatchUp::Tips => {}
                CatchUp::Gap => {
                    core.db = None;
                    self.discard_dir()?;
                    scream.attempt("catch-up hit a hole below the floor");
                    continue;
                }
                CatchUp::RejectedInOpen => {
                    core.db = None;
                    self.discard_dir()?;
                    scream.attempt("replay rejected in the open phase");
                    continue;
                }
            }
            if core.generation()? == core.chain.sum() {
                break;
            }
            core.db = None;
            self.discard_dir()?;
            scream.attempt("the wholeness identity failed after catch-up");
        }
        if let Some(pending) = carry {
            core.chain.pending = Some(pending);
            core.chain
                .write_atomic(&self.dir)
                .map_err(|err| Error::Fault(Fault::Io(err)))?;
            self.step(WriterStep::PendingWrite)?;
        }
        Ok(())
    }
    pub(crate) fn read_floor(&self) -> Result<std::result::Result<Floor, OpenRefusal>> {
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
            .get(&crate::manifest::ckpt_json_key(&self.prefix, &digest))
            .map_err(|err| Error::Fault(Fault::Store(err)))?;
        let Some(doc) = doc else {
            return Ok(Err(OpenRefusal::CheckpointDocMissing { digest }));
        };
        match Checkpoint::parse(&doc.bytes, self.codec.braids()) {
            Ok(doc) => Ok(Ok(Some((digest, doc)))),
            Err(error) => Ok(Err(OpenRefusal::Checkpoint { digest, error })),
        }
    }

    pub(crate) fn mount(&self, core: &Core<T>) -> Result<MountEnd<T>> {
        if self.dir.exists() {
            return match Db::open(&self.dir, self.theory.clone()) {
                Ok(db) => {
                    let Some(Ok(chain)) = Chain::read(&self.dir, self.codec.braids())
                        .map_err(|err| Error::Fault(Fault::Io(err)))?
                    else {
                        drop(db);
                        return Ok(MountEnd::Discard("the sidecar refused to read"));
                    };
                    Ok(MountEnd::Mounted {
                        db: Arc::new(db),
                        chain,
                        pre_existing: true,
                    })
                }
                Err(error @ bumbledb::Error::EnvironmentLocked) => {
                    Err(Error::Fault(Fault::Engine(error)))
                }
                Err(_) => Ok(MountEnd::Discard("the local directory refused to open")),
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
                    db: Arc::new(db),
                    chain,
                    pre_existing: false,
                })
            }
            Admission::Rejected(violations) => {
                Ok(MountEnd::Refused(OpenRefusal::TheoryRejected(violations)))
            }
        }
    }

    pub(crate) fn seed(&self, digest: [u8; 32], doc: &Checkpoint) -> Result<MountEnd<T>> {
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
            db: Arc::new(db),
            chain,
            pre_existing: false,
        })
    }

    /// Round-robin catch-up over all braids but the backlog's own —
    /// that braid's replay runs through backlog resolution instead. A
    /// rejected replay on a pre-existing directory is the open-phase
    /// discard; on a seeded or bootstrapped store it wedges.
    pub(crate) fn catch_up(
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
                    core.db.as_deref().expect("mounted"),
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

    pub(crate) fn discard_dir(&self) -> Result<()> {
        match fs::remove_dir_all(&self.dir) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(Error::Fault(Fault::Io(err))),
        }
    }
}
