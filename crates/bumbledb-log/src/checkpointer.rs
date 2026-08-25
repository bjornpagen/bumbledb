//! A checkpointer: a replica plus the right to compact, publish under
//! the checkpoint order, and run the retention sweep. No commits, no
//! leases. The duty binary is one cadence check of this type.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use bumbledb::{Db, Theory};

use crate::braids::{BraidId, Braids};
use crate::gc::{CHECKPOINT_RETAIN_MS, Gc, PublishClock, gc_at};
use crate::manifest::{
    Checkpoint, Head, Manifest, PublishRefusal, Published, ckpt_mdb_key, log_key, manifest_key,
    publish_checkpoint,
};
use crate::replica::{
    Fault, OpenRefusal, Opened, Refreshed, Replica, clear_ckpt_scratch, reclaim_orphan,
    record_ckpt_scratch,
};
use crate::sidecar::{Chain, ChainEntry};
use crate::store::{Create, ObjectStore, prove_create};
use crate::writer::{CHECKPOINT_EVERY_BYTES, CHECKPOINT_EVERY_SUM, DATA_FILE};

/// Outcome of `Checkpointer::open`.
pub enum CheckpointerOpened<T: Theory + Clone, S: ObjectStore> {
    Ready(Box<Checkpointer<T, S>>),
    Refused(OpenRefusal),
}

/// What the cadence check did about a checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compact {
    Quiet,
    Published(Published),
}

/// One duty body: refresh, one cadence check, the retention sweep.
#[derive(Debug)]
pub enum Ran {
    Ready { compact: Compact, gc: Gc },
    RefreshRefused(OpenRefusal),
}

/// Releases a `duty_busy` flag on unwind so a panic cannot freeze the
/// cadence (50: every resource has an owner).
pub struct DutyBusy<'a> {
    flag: &'a mut bool,
}

impl<'a> DutyBusy<'a> {
    /// Sets the flag and returns a guard that clears it on drop.
    pub fn hold(flag: &'a mut bool) -> Self {
        *flag = true;
        Self { flag }
    }
}

impl Drop for DutyBusy<'_> {
    fn drop(&mut self) {
        *self.flag = false;
    }
}

/// Replica plus checkpoint and gc rights.
pub struct Checkpointer<T: Theory + Clone, S: ObjectStore> {
    replica: Replica<T, S>,
    writer_id: u64,
    cadence_sum: u64,
    cadence_bytes: u64,
    /// The trusted instant the current checkpoint entered reachable
    /// history — the checkpointer's stamp, never a writer-claimed
    /// batch header (50 §3).
    publish_ms: Option<u64>,
    duty_busy: Arc<AtomicBool>,
}

impl<T, S> Checkpointer<T, S>
where
    T: Theory + Clone,
    S: ObjectStore,
{
    /// Opens the prefix as a checkpointer — the replica gauntlet, then
    /// the protocol cadence knobs.
    pub fn open(
        store: S,
        prefix: &str,
        dir: &Path,
        theory: T,
        writer_id: u64,
    ) -> Result<CheckpointerOpened<T, S>, Fault> {
        match Replica::open(store, prefix, dir, theory)? {
            Opened::Ready(replica) => Ok(CheckpointerOpened::Ready(Box::new(Self {
                replica: *replica,
                writer_id,
                cadence_sum: CHECKPOINT_EVERY_SUM,
                cadence_bytes: CHECKPOINT_EVERY_BYTES,
                publish_ms: None,
                duty_busy: Arc::new(AtomicBool::new(false)),
            }))),
            Opened::Refused(refusal) => Ok(CheckpointerOpened::Refused(refusal)),
        }
    }

    #[must_use]
    pub const fn replica(&self) -> &Replica<T, S> {
        &self.replica
    }

    /// Re-sizes the cadence (both arms) so a test can cross without
    /// writing 256 batches.
    pub fn set_checkpoint_cadence(&mut self, sum: u64, bytes: u64) {
        self.cadence_sum = sum.max(1);
        self.cadence_bytes = bytes.max(1);
    }

    /// The one body: refresh, compact and publish if the cadence is
    /// crossed, then the retention law against the trusted publish
    /// clock.
    pub fn run(&mut self) -> Result<Ran, Fault> {
        let busy = Arc::clone(&self.duty_busy);
        busy.store(true, Ordering::Release);
        let _busy = DutyBusyFlag(busy);
        match self.replica.refresh()? {
            Refreshed::Vector(_) => {}
            Refreshed::Refused(refusal) => return Ok(Ran::RefreshRefused(refusal)),
        }
        let settled = match self.replica.chain() {
            Chain::Settled { entries } => Some(entries.clone()),
            Chain::Pending { .. } => None,
        };
        let compact = match settled {
            Some(entries) if self.cadence_crossed()? => {
                let db = self.replica.db().map_err(|_| {
                    Fault::Io(io::Error::new(
                        io::ErrorKind::NotFound,
                        "replica is unmounted",
                    ))
                })?;
                let scratch = PathBuf::from(format!("{}.duty-ckpt", self.replica.dir().display()));
                // The detached binary is exclusive: compact cannot tear.
                let Some(published) = compact_and_publish(
                    self.replica.store(),
                    self.replica.prefix(),
                    self.replica.dir(),
                    self.replica.codec().braids(),
                    self.writer_id,
                    db,
                    &entries,
                    &scratch,
                    || true,
                )?
                else {
                    unreachable!("the detached hold is exclusive");
                };
                if matches!(published, Published::Replaced) {
                    // The instant the candidate entered reachable history —
                    // the CAS is the linearization point (50 §3).
                    self.publish_ms = Some(now_ms());
                }
                if !matches!(published, Published::Refused(_))
                    && let Some(refusal) = self.replica.pull_manifest()?
                {
                    return Ok(Ran::RefreshRefused(refusal));
                }
                Compact::Published(published)
            }
            Some(_) | None => Compact::Quiet,
        };
        let now = now_ms();
        let publish_ms = self.publish_ms.unwrap_or(now);
        let gc = gc_at(
            self.replica.store(),
            self.replica.prefix(),
            self.replica.codec(),
            CHECKPOINT_RETAIN_MS,
            PublishClock {
                now_ms: now,
                publish_ms,
            },
        )?;
        Ok(Ran::Ready { compact, gc })
    }

    fn cadence_crossed(&self) -> Result<bool, Fault> {
        let delta = self
            .replica
            .chain()
            .sum()
            .saturating_sub(self.replica.checkpoint_sum());
        Ok(delta >= self.cadence_sum || self.log_volume()? >= self.cadence_bytes)
    }

    fn log_volume(&self) -> Result<u64, Fault> {
        let mut bytes = 0u64;
        for braid in self.replica.codec().braids().components().keys() {
            let start = self.replica.checkpoint_g(*braid);
            let end = self.replica.chain().position(*braid).g;
            for slot in (start + 1)..=end {
                let key = log_key(self.replica.prefix(), *braid, slot);
                let Some(object) = self.replica.store().get(&key)? else {
                    break;
                };
                bytes = bytes.saturating_add(u64::try_from(object.bytes.len()).unwrap_or(u64::MAX));
            }
        }
        Ok(bytes)
    }
}

/// The compact→publish transition. Compaction's input is `Settled`.
/// The loser deletes its own digest on Kept and every refused publish.
/// The scratch lease names the candidate before the
/// upload-before-decision window; the successor GETs it at open.
///
/// `hold` is the snapshot still that Settled value after compact. The
/// resident entry proves it; the detached entry is exclusive. `None`
/// is a torn view — the entry retries.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compact_and_publish<T, S>(
    store: &S,
    prefix: &str,
    dir: &Path,
    braids: &Braids,
    writer_id: u64,
    db: &Db<T>,
    entries: &BTreeMap<BraidId, ChainEntry>,
    scratch: &Path,
    hold: impl FnOnce() -> bool,
) -> Result<Option<Published>, Fault>
where
    T: Theory + Clone,
    S: ObjectStore,
{
    let bytes = {
        let scratch = Scratch::new(scratch.to_path_buf());
        db.compact(&scratch.path).map_err(Fault::Engine)?;
        fs::read(scratch.path.join(DATA_FILE)).map_err(Fault::Io)?
    };
    let catalog = db.catalog_digest().map_err(Fault::Engine)?;
    if !hold() {
        return Ok(None);
    }
    let heads: BTreeMap<_, _> = entries
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
    let prev = match store.get(&manifest_key(prefix))? {
        Some(fetched) => match Manifest::parse(&fetched.bytes) {
            Ok(manifest) => manifest.checkpoint,
            Err(error) => {
                return Ok(Some(Published::Refused(PublishRefusal::Manifest(error))));
            }
        },
        None => None,
    };
    let doc = Checkpoint {
        braids: heads,
        catalog,
        writer: writer_id,
        prev,
    };
    let digest = doc.digest();
    // The scratch lease names this candidate before the
    // upload-before-decision window. The successor GETs it at open.
    record_ckpt_scratch(dir, &digest).map_err(Fault::Io)?;
    let key = ckpt_mdb_key(prefix, &digest);
    loop {
        match prove_create(store, &key, &bytes, store.put_create(&key, &bytes)?)? {
            Create::Created(_) | Create::Exists => break,
            Create::Ambiguous => {}
        }
    }
    let published = publish_checkpoint(store, prefix, braids, &doc)?;
    match &published {
        Published::Replaced => {
            let _ = clear_ckpt_scratch(dir);
        }
        Published::Kept { .. } | Published::Refused(_) => {
            // The loser deletes its own digest on Kept and every
            // refused publish.
            let _ = reclaim_orphan(store, prefix, &digest);
            let _ = clear_ckpt_scratch(dir);
        }
    }
    Ok(Some(published))
}

/// Clears `duty_busy` on unwind so a panic cannot freeze the cadence.
struct DutyBusyFlag(Arc<AtomicBool>);

impl Drop for DutyBusyFlag {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Scratch dir as a lease: any drop, including panic, reclaims it.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(path: PathBuf) -> Self {
        let _ = fs::remove_dir_all(&path);
        Self { path }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |span| {
            u64::try_from(span.as_millis()).unwrap_or(u64::MAX)
        })
}
