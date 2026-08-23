//! Retention and point-in-time restore. The retention law: delete log
//! objects and checkpoints older than window R, always exempting the
//! current checkpoint and every log object at or above its vector per
//! braid; a store whose manifest still says `checkpoint: null` has
//! nothing gc-eligible, by the same rule. Restore points are vectors —
//! braids are independent (L9), so every pointwise combination of braid
//! prefixes is a real serial state — discovered by walking the
//! checkpoint backlink chain from the manifest with GETs alone (no LIST
//! exists). The gc window is also the audit window: history nobody
//! replays again is vouched for by its publisher alone.

use std::path::Path;

use bumbledb::{Db, Theory, Violations};

use crate::apply::{Applied, ApplyRefusal, apply};
use crate::braids::BraidId;
use crate::codec::Codec;
use crate::manifest::{
    Checkpoint, CheckpointError, Manifest, ManifestError, ckpt_json_key, ckpt_mdb_key, log_key,
    manifest_key,
};
use crate::replica::{
    Fault, OpenRefusal, Vector, derive_codec, fetch_checkpoint_bytes, write_checkpoint_bytes,
};
use crate::sidecar::{Chain, ChainEntry};
use crate::store::{ObjectStore, Result as StoreResult};

/// What one sweep deleted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sweep {
    pub log_deleted: Vec<String>,
    pub checkpoints_deleted: Vec<String>,
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

/// The gc verb: one sweep under window `window_ms` at `now_ms`.
/// Log objects strictly below the current checkpoint's vector whose
/// batch timestamp is older than the window die, walking each braid
/// downward from the floor so deletion stays a contiguous bottom
/// segment. Checkpoints behind the current one die by the same clock,
/// json before object, so an interrupted sweep truncates the backlink
/// walk instead of dangling it.
pub fn gc<S: ObjectStore>(
    store: &S,
    prefix: &str,
    codec: &Codec,
    window_ms: u64,
    now_ms: u64,
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

    for (braid, head) in &doc.braids {
        let mut old = false;
        let mut slot = head.g;
        while slot > 1 {
            slot -= 1;
            let key = log_key(prefix, *braid, slot);
            let Some(object) = store.get(&key)? else {
                break;
            };
            if !old {
                let Ok(batch) = codec.decode(&object.bytes) else {
                    // An undecodable object blocks its braid's sweep:
                    // deleting around evidence is worse than keeping it.
                    break;
                };
                if now_ms.saturating_sub(batch.header.timestamp) <= window_ms {
                    continue;
                }
                old = true;
            }
            store.delete(&key)?;
            sweep.log_deleted.push(key);
        }
    }

    let mut digest = doc.prev;
    while let Some(prior) = digest {
        let Some(object) = store.get(&ckpt_json_key(prefix, &prior))? else {
            break;
        };
        let Ok(prior_doc) = Checkpoint::parse(&object.bytes, codec.braids()) else {
            break;
        };
        digest = prior_doc.prev;
        let age = prior_doc
            .braids
            .values()
            .map(|head| head.ts)
            .max()
            .unwrap_or(0);
        if now_ms.saturating_sub(age) > window_ms {
            store.delete(&ckpt_json_key(prefix, &prior))?;
            store.delete(&ckpt_mdb_key(prefix, &prior))?;
            sweep
                .checkpoints_deleted
                .push(crate::manifest::hex32(&prior));
        }
    }

    Ok(Gc::Swept(sweep))
}

fn fetch_doc<S: ObjectStore>(
    store: &S,
    prefix: &str,
    digest: [u8; 32],
    codec: &Codec,
) -> StoreResult<Result<Checkpoint, GcRefusal>> {
    let Some(object) = store.get(&ckpt_json_key(prefix, &digest))? else {
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
}

/// Outcome of a restore: the opened store and the restored vector —
/// the vector, not any wall-clock instant, is the truth the restore
/// reports.
pub enum Restore<T> {
    Restored { db: Box<Db<T>>, vector: Vector },
    Refused(RestoreRefusal),
}

/// Restores to `target`: walks the checkpoint backlink chain from the
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

    let mut base: Option<([u8; 32], Checkpoint)> = None;
    let mut cursor = manifest.checkpoint;
    while let Some(digest) = cursor {
        let Some(object) = store.get(&ckpt_json_key(prefix, &digest))? else {
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
        let qualifies = doc
            .braids
            .iter()
            .all(|(braid, head)| head.g <= target.get(braid).copied().unwrap_or(0));
        if qualifies {
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
        let goal = target.get(&braid).copied().unwrap_or(0);
        while chain.position(braid).g < goal {
            let slot = chain.position(braid).g + 1;
            let key = log_key(prefix, braid, slot);
            let Some(object) = store.get(&key)? else {
                return Ok(Restore::Refused(RestoreRefusal::SlotMissing {
                    braid,
                    slot,
                }));
            };
            match apply(&db, &mut chain, &codec, braid, slot, &object.bytes, 0)? {
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
    let vector = chain.vector();
    Ok(Restore::Restored {
        db: Box::new(db),
        vector,
    })
}

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
        let Some(object) = store.get(&ckpt_json_key(prefix, &digest))? else {
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

    let mut target: Vector = Vector::new();
    for (braid, start) in &base_vector {
        let mut g = *start;
        loop {
            let key = log_key(prefix, *braid, g + 1);
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
        target.insert(*braid, g);
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
    Ok(Ok((db, chain)))
}
