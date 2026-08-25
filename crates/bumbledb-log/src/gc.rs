//! Retention and point-in-time restore. The retention law: delete log
//! objects and checkpoints older than window R, always exempting the
//! current checkpoint and every log object at or above its vector per
//! braid; a store whose manifest still says `checkpoint: null` has
//! nothing gc-eligible, by the same rule. Restore points are vectors —
//! braids are independent (L9), so every pointwise combination of braid
//! prefixes is a real serial state — discovered by walking the
//! checkpoint backlink chain from the manifest with GETs alone (no LIST
//! exists). Crash-stranded candidates live in the reserved-namespace
//! scratch lease; the successor GETs that known document at open and
//! deletes the named objects. GET-only GC does not LIST-delete the
//! complement of the reachable spine. The gc window is also the audit
//! window: history nobody replays again is vouched for by its publisher
//! alone.

use std::collections::BTreeMap;
use std::path::Path;

use bumbledb::{Db, Theory, Violations};

use crate::apply::{Applied, ApplyRefusal, apply};
use crate::braids::BraidId;
use crate::codec::Codec;
use crate::manifest::{
    Checkpoint, CheckpointError, Manifest, ManifestError, ckpt_doc_key, ckpt_mdb_key, log_key,
    manifest_key,
};
use crate::replica::{
    Fault, OpenRefusal, Vector, derive_codec, fetch_checkpoint_bytes, write_checkpoint_bytes,
};
use crate::sidecar::{Chain, ChainEntry};
use crate::store::{ObjectStore, Result as StoreResult};

/// Retention window R in milliseconds. 10-protocol owns the ninety-day
/// value; consumer: the duty binary's one sweep after the cadence check.
pub const CHECKPOINT_RETAIN_MS: u64 = 90 * 24 * 60 * 60 * 1000;

/// What one sweep deleted. `checkpoints_deleted` is the dropped
/// checkpoint identities. `swept_below` is the exclusive end of the
/// contiguous deleted prefix per braid — the next sweep resumes there.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sweep {
    pub log_deleted: Vec<String>,
    pub checkpoints_deleted: Vec<[u8; 32]>,
    pub swept_below: BTreeMap<BraidId, u64>,
}

/// The checkpointer's trusted publish clock. Age is
/// `now_ms.saturating_sub(publish_ms)`; `publish_ms` is the instant the
/// current checkpoint entered reachable history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishClock {
    pub now_ms: u64,
    pub publish_ms: u64,
}

/// Why the sweep refused to run at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcRefusal {
    ManifestMissing,
    Manifest(ManifestError),
    CheckpointDocMissing {
        digest: [u8; 32],
    },
    Checkpoint {
        digest: [u8; 32],
        error: CheckpointError,
    },
}

/// Outcome of the gc verb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gc {
    Swept(Sweep),
    /// The manifest says `checkpoint: null` — nothing has ever been
    /// gc-eligible.
    NothingEligible,
    Refused(GcRefusal),
}

/// The gc verb: one sweep under window `window_ms`. Ages against
/// `now_ms` as the publish stamp — the checkpointer's clock when the
/// caller has not threaded a distinct stamp (see [`gc_at`]).
///
/// # Errors
pub fn gc<S: ObjectStore>(
    store: &S,
    prefix: &str,
    codec: &Codec,
    window_ms: u64,
    now_ms: u64,
) -> StoreResult<Gc> {
    gc_at(
        store,
        prefix,
        codec,
        window_ms,
        PublishClock {
            now_ms,
            publish_ms: now_ms,
        },
    )
}

/// One sweep under `window_ms` at `clock`. Log objects strictly below
/// the current checkpoint's vector whose publish age exceeds the window
/// die, walking each braid upward from the swept-below marker toward
/// the floor so the deleted region is the contiguous prefix `[0, marker)`.
/// A missing slot advances the marker; a young or undecodable slot
/// stops that braid. Checkpoints behind the current one die by the same
/// clock: the sweep walks the Merkle backlink, then deletes the mdb and
/// the document as one unit from the tail so an interrupted sweep leaves
/// a walkable document, never an orphan mdb. Crash-stranded candidates are
/// not this walk: they are named in the scratch lease and reclaimed at
/// open.
///
/// # Errors
pub fn gc_at<S: ObjectStore>(
    store: &S,
    prefix: &str,
    codec: &Codec,
    window_ms: u64,
    clock: PublishClock,
) -> StoreResult<Gc> {
    let Some(fetched) = store.get(&manifest_key(prefix))? else {
        return Ok(Gc::Refused(GcRefusal::ManifestMissing));
    };
    let manifest = match Manifest::parse(&fetched.bytes) {
        Ok(manifest) => manifest,
        Err(error) => return Ok(Gc::Refused(GcRefusal::Manifest(error))),
    };
    let Some(current) = manifest.checkpoint else {
        return Ok(Gc::NothingEligible);
    };
    let doc = match fetch_doc(store, prefix, current, codec)? {
        Ok(doc) => doc,
        Err(refusal) => return Ok(Gc::Refused(refusal)),
    };

    let mut sweep = Sweep::default();
    let old = clock.now_ms.saturating_sub(clock.publish_ms) > window_ms;

    for (braid, head) in &doc.braids {
        let marker = sweep_log_braid(store, prefix, codec, *braid, head.g, old, &mut sweep)?;
        sweep.swept_below.insert(*braid, marker);
    }

    sweep_checkpoints(store, prefix, codec, doc.prev, old, &mut sweep)?;

    Ok(Gc::Swept(sweep))
}

/// Walks slots `[1, floor)` upward. Missing objects are already in the
/// deleted prefix and advance the marker; an undecodable object blocks
/// the braid.
fn sweep_log_braid<S: ObjectStore>(
    store: &S,
    prefix: &str,
    codec: &Codec,
    braid: BraidId,
    floor: u64,
    old: bool,
    sweep: &mut Sweep,
) -> StoreResult<u64> {
    let mut marker = 0;
    let mut slot = 1;
    while slot < floor {
        let key = log_key(prefix, braid, slot);
        match store.get(&key)? {
            None => {
                marker = slot + 1;
                slot += 1;
            }
            Some(object) => {
                if !old {
                    break;
                }
                if codec.decode(&object.bytes).is_err() {
                    break;
                }
                store.delete(&key)?;
                sweep.log_deleted.push(key.to_string());
                marker = slot + 1;
                slot += 1;
            }
        }
    }
    Ok(marker)
}

/// Walks the Merkle backlink from `start`, then deletes old nodes from
/// the tail. A missing document still drops its mdb; a corrupt document
/// stops the walk.
fn sweep_checkpoints<S: ObjectStore>(
    store: &S,
    prefix: &str,
    codec: &Codec,
    start: Option<[u8; 32]>,
    old: bool,
    sweep: &mut Sweep,
) -> StoreResult<()> {
    let mut chain = Vec::new();
    let mut digest = start;
    while let Some(prior) = digest {
        let Some(object) = store.get(&ckpt_doc_key(prefix, &prior))? else {
            store.delete(&ckpt_mdb_key(prefix, &prior))?;
            break;
        };
        let Ok(prior_doc) = Checkpoint::parse(&object.bytes, codec.braids()) else {
            break;
        };
        digest = prior_doc.prev;
        chain.push(prior);
    }
    if !old {
        return Ok(());
    }
    for prior in chain.into_iter().rev() {
        delete_checkpoint_unit(store, prefix, &prior)?;
        sweep.checkpoints_deleted.push(prior);
    }
    Ok(())
}

fn delete_checkpoint_unit<S: ObjectStore>(
    store: &S,
    prefix: &str,
    digest: &[u8; 32],
) -> StoreResult<()> {
    store.delete(&ckpt_mdb_key(prefix, digest))?;
    store.delete(&ckpt_doc_key(prefix, digest))?;
    Ok(())
}

fn fetch_doc<S: ObjectStore>(
    store: &S,
    prefix: &str,
    digest: [u8; 32],
    codec: &Codec,
) -> StoreResult<Result<Checkpoint, GcRefusal>> {
    let Some(object) = store.get(&ckpt_doc_key(prefix, &digest))? else {
        return Ok(Err(GcRefusal::CheckpointDocMissing { digest }));
    };
    match Checkpoint::parse(&object.bytes, codec.braids()) {
        Ok(doc) => Ok(Ok(doc)),
        Err(error) => Ok(Err(GcRefusal::Checkpoint { digest, error })),
    }
}

/// Why a restore refused.
#[derive(Debug)]
pub enum RestoreRefusal {
    Open(OpenRefusal),
    /// The backlink walk hit a gc'd checkpoint document before finding
    /// a qualifying base — the target predates retention.
    BeyondRetention {
        digest: [u8; 32],
    },
    /// A slot the replay needs is gone (gc'd or never existed).
    SlotMissing {
        braid: BraidId,
        slot: u64,
    },
    /// A slot refused to apply — the log itself is corrupt there.
    Corrupt(ApplyRefusal),
    /// A slot the engine rejected — impossible for honest writers.
    Rejected {
        braid: BraidId,
        slot: u64,
        violations: Violations,
    },
    /// The restore destination already exists; restores materialize
    /// into fresh directories only.
    DirExists,
    /// A braid id the schema's own decomposition does not mint.
    UnknownBraid {
        got: u32,
    },
}

/// Outcome of a restore: the opened store and the restored vector —
/// the vector, not any wall-clock instant, is the truth the restore
/// reports.
pub enum Restore<T> {
    Restored { db: Box<Db<T>>, vector: Vector },
    Refused(RestoreRefusal),
}

/// Restores to `target`: walks the checkpoint backlink chain from the
///
/// # Errors
/// manifest to the first checkpoint whose vector is pointwise at or
/// below the target (bootstrapping from zero when the walk runs out at
/// the first checkpoint), opens it at `dir`, then replays each braid to
/// its target — braid order irrelevant (L8).
pub fn restore_to_vector<T: Theory + Clone, S: ObjectStore>(
    store: &S,
    prefix: &str,
    dir: &Path,
    theory: &T,
    target: &Vector,
) -> Result<Restore<T>, Fault> {
    let (codec, fingerprint, _) = match derive_codec(theory) {
        Ok(derived) => derived,
        Err(refusal) => return Ok(Restore::Refused(RestoreRefusal::Open(refusal))),
    };
    let manifest = match read_manifest(store, prefix, fingerprint)? {
        Ok(manifest) => manifest,
        Err(refusal) => return Ok(Restore::Refused(refusal)),
    };
    if dir.exists() {
        return Ok(Restore::Refused(RestoreRefusal::DirExists));
    }
    for braid in target.braids() {
        if codec.braids().parse(braid.raw()).is_none() {
            return Ok(Restore::Refused(RestoreRefusal::UnknownBraid {
                got: braid.raw(),
            }));
        }
    }

    let mut base: Option<([u8; 32], Checkpoint)> = None;
    let mut cursor = manifest.checkpoint;
    while let Some(digest) = cursor {
        let Some(object) = store.get(&ckpt_doc_key(prefix, &digest))? else {
            return Ok(Restore::Refused(RestoreRefusal::BeyondRetention { digest }));
        };
        let doc = match Checkpoint::parse(&object.bytes, codec.braids()) {
            Ok(doc) => doc,
            Err(error) => {
                return Ok(Restore::Refused(RestoreRefusal::Open(
                    OpenRefusal::Checkpoint { digest, error },
                )));
            }
        };
        if target.dominates(&doc.vector()) {
            base = Some((digest, doc));
            break;
        }
        cursor = doc.prev;
    }

    let (db, mut chain) = match seed_restore(store, prefix, dir, theory, base.as_ref())? {
        Ok(seeded) => seeded,
        Err(refusal) => return Ok(Restore::Refused(refusal)),
    };

    let braids: Vec<BraidId> = codec.braids().components().keys().copied().collect();
    for braid in braids {
        let goal = target.at(braid);
        while chain.position(braid).g < goal {
            let slot = chain.position(braid).g + 1;
            let key = log_key(prefix, braid, slot);
            let Some(object) = store.get(&key)? else {
                return Ok(Restore::Refused(RestoreRefusal::SlotMissing {
                    braid,
                    slot,
                }));
            };
            match apply(&db, &mut chain, &codec, braid, slot, &object.bytes)? {
                Applied::Advanced { .. } | Applied::Absorbed { .. } => {}
                Applied::Refused(refusal) => {
                    return Ok(Restore::Refused(RestoreRefusal::Corrupt(refusal)));
                }
                Applied::Rejected(violations) => {
                    return Ok(Restore::Refused(RestoreRefusal::Rejected {
                        braid,
                        slot,
                        violations,
                    }));
                }
            }
        }
    }
    chain.write_atomic(dir)?;
    Ok(Restore::Restored {
        db: Box::new(db),
        vector: chain.vector(),
    })
}

/// # Errors
/// Maps a wall-clock instant through the batch timestamps — per braid
/// the largest g with `ts ≤ T`; timestamps are clamped monotone per
/// braid at publish, so the mapped set is a prefix by construction —
/// then restores to the mapped vector. Cross-braid, wall clocks are
/// writer-local: the restored vector, not the instant, is the reported
/// truth.
pub fn restore_by_time<T: Theory + Clone, S: ObjectStore>(
    store: &S,
    prefix: &str,
    dir: &Path,
    theory: &T,
    t_ms: u64,
) -> Result<Restore<T>, Fault> {
    let (codec, fingerprint, _) = match derive_codec(theory) {
        Ok(derived) => derived,
        Err(refusal) => return Ok(Restore::Refused(RestoreRefusal::Open(refusal))),
    };
    let manifest = match read_manifest(store, prefix, fingerprint)? {
        Ok(manifest) => manifest,
        Err(refusal) => return Ok(Restore::Refused(refusal)),
    };

    let mut base_vector: Vector = codec
        .braids()
        .components()
        .keys()
        .map(|braid| (*braid, 0))
        .collect();
    let mut cursor = manifest.checkpoint;
    while let Some(digest) = cursor {
        let Some(object) = store.get(&ckpt_doc_key(prefix, &digest))? else {
            return Ok(Restore::Refused(RestoreRefusal::BeyondRetention { digest }));
        };
        let doc = match Checkpoint::parse(&object.bytes, codec.braids()) {
            Ok(doc) => doc,
            Err(error) => {
                return Ok(Restore::Refused(RestoreRefusal::Open(
                    OpenRefusal::Checkpoint { digest, error },
                )));
            }
        };
        if doc.braids.values().all(|head| head.ts <= t_ms) {
            base_vector = doc.vector();
            break;
        }
        cursor = doc.prev;
    }

    let mut target = Vector::new();
    for (braid, start) in base_vector.iter() {
        let mut g = start;
        loop {
            let key = log_key(prefix, braid, g + 1);
            let Some(object) = store.get(&key)? else {
                break;
            };
            let batch = match codec.decode(&object.bytes) {
                Ok(batch) => batch,
                Err(error) => {
                    return Ok(Restore::Refused(RestoreRefusal::Corrupt(
                        ApplyRefusal::Decode(error),
                    )));
                }
            };
            if batch.header.timestamp > t_ms {
                break;
            }
            g += 1;
        }
        target.set(braid, g);
    }

    restore_to_vector(store, prefix, dir, theory, &target)
}

fn read_manifest<S: ObjectStore>(
    store: &S,
    prefix: &str,
    fingerprint: [u8; 32],
) -> Result<Result<Manifest, RestoreRefusal>, Fault> {
    let Some(fetched) = store.get(&manifest_key(prefix))? else {
        return Ok(Err(RestoreRefusal::Open(OpenRefusal::ManifestMissing)));
    };
    let manifest = match Manifest::parse(&fetched.bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            return Ok(Err(RestoreRefusal::Open(OpenRefusal::Manifest(error))));
        }
    };
    if manifest.fingerprint != fingerprint {
        return Ok(Err(RestoreRefusal::Open(
            OpenRefusal::FingerprintMismatch {
                manifest: manifest.fingerprint,
                derived: fingerprint,
            },
        )));
    }
    Ok(Ok(manifest))
}

fn seed_restore<T: Theory + Clone, S: ObjectStore>(
    store: &S,
    prefix: &str,
    dir: &Path,
    theory: &T,
    base: Option<&([u8; 32], Checkpoint)>,
) -> Result<Result<(Db<T>, Chain), RestoreRefusal>, Fault> {
    let Some((digest, doc)) = base else {
        let db = match Db::create(dir, theory.clone())? {
            bumbledb::Admission::Accepted(db) => db,
            bumbledb::Admission::Rejected(violations) => {
                return Ok(Err(RestoreRefusal::Open(OpenRefusal::TheoryRejected(
                    violations,
                ))));
            }
        };
        let (codec, _, _) = derive_codec(theory).expect("derived once already");
        return Ok(Ok((db, Chain::genesis(codec.braids()))));
    };
    let bytes = match fetch_checkpoint_bytes(store, prefix, *digest)? {
        Ok(bytes) => bytes,
        Err(refusal) => return Ok(Err(RestoreRefusal::Open(refusal))),
    };
    write_checkpoint_bytes(dir, &bytes)?;
    let db = match Db::open(dir, theory.clone()) {
        Ok(db) => db,
        Err(error @ (bumbledb::Error::Io(_) | bumbledb::Error::EnvironmentLocked)) => {
            return Err(Fault::Engine(error));
        }
        Err(error) => {
            return Ok(Err(RestoreRefusal::Open(OpenRefusal::CheckpointOpen {
                digest: *digest,
                error,
            })));
        }
    };
    let opened = db.generation()?.value();
    if opened != doc.sum() {
        return Ok(Err(RestoreRefusal::Open(OpenRefusal::CheckpointState {
            digest: *digest,
            opened,
            sum: doc.sum(),
        })));
    }
    let computed = db.catalog_digest()?;
    if computed != doc.catalog {
        return Ok(Err(RestoreRefusal::Open(OpenRefusal::CatalogMismatch {
            digest: *digest,
            writer: doc.writer,
            carried: doc.catalog,
            computed,
        })));
    }
    let chain = Chain::Settled {
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
    Ok(Ok((db, chain)))
}
