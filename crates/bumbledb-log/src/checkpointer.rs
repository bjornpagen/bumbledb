//! A checkpointer: a replica plus the right to compact, publish under
//! the checkpoint order, and run the retention sweep. No commits, no
//! leases. The duty binary is one cadence check of this type.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bumbledb::Theory;

use crate::gc::{CHECKPOINT_RETAIN_MS, Gc, gc};
use crate::manifest::{Head, Published, ckpt_mdb_key, log_key, publish_checkpoint};
use crate::replica::{Fault, OpenRefusal, Opened, Refreshed, Replica};
use crate::store::{Create, ObjectStore};
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

/// Replica plus checkpoint and gc rights.
pub struct Checkpointer<T: Theory + Clone, S: ObjectStore> {
    replica: Replica<T, S>,
    writer_id: u64,
    cadence_sum: u64,
    cadence_bytes: u64,
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
    /// crossed, then the retention law.
    pub fn run(&mut self) -> Result<Ran, Fault> {
        match self.replica.refresh()? {
            Refreshed::Vector(_) => {}
            Refreshed::Refused(refusal) => return Ok(Ran::RefreshRefused(refusal)),
        }
        let compact = if self.cadence_crossed()? {
            let published = self.compact_and_publish()?;
            if !matches!(published, Published::Refused(_))
                && let Some(refusal) = self.replica.pull_manifest()?
            {
                return Ok(Ran::RefreshRefused(refusal));
            }
            Compact::Published(published)
        } else {
            Compact::Quiet
        };
        let gc = gc(
            self.replica.store(),
            self.replica.prefix(),
            self.replica.codec(),
            CHECKPOINT_RETAIN_MS,
            now_ms(),
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

    fn compact_and_publish(&self) -> Result<Published, Fault> {
        let scratch = PathBuf::from(format!("{}.duty-ckpt", self.replica.dir().display()));
        let _ = fs::remove_dir_all(&scratch);
        let compacted = (|| -> Result<Vec<u8>, Fault> {
            self.replica.db().compact(&scratch).map_err(Fault::Engine)?;
            fs::read(scratch.join(DATA_FILE)).map_err(Fault::Io)
        })();
        let _ = fs::remove_dir_all(&scratch);
        let bytes = compacted?;
        let catalog = self.replica.db().catalog_digest().map_err(Fault::Engine)?;
        let digest = *blake3::hash(&bytes).as_bytes();
        let heads: BTreeMap<_, _> = self
            .replica
            .chain()
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
        match self
            .replica
            .store()
            .put_create(&ckpt_mdb_key(self.replica.prefix(), &digest), &bytes)?
        {
            Create::Created(_) | Create::Exists | Create::Ambiguous => {}
        }
        Ok(publish_checkpoint(
            self.replica.store(),
            self.replica.prefix(),
            self.replica.codec().braids(),
            digest,
            &heads,
            catalog,
            self.writer_id,
        )?)
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |span| {
            u64::try_from(span.as_millis()).unwrap_or(u64::MAX)
        })
}
