//! Checkpoint duty: cadence detection on the commit path, compact and
//! CAS off the lock. Commits never wait on the duty. Compact's input
//! is a Settled chain; the result is a total sum whose only success
//! is Replaced.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use bumbledb::Db;

use crate::braids::BraidId;
use crate::manifest::{
    ckpt_mdb_key, manifest_key, publish_checkpoint, Checkpoint, Head, Manifest, PublishRefusal,
    Published,
};
use crate::replica::Scream;
use crate::sidecar::{Chain, ChainEntry};
use crate::store::ObjectStore;

use super::{lock, Core, Inner, StepHook, Theory, DATA_FILE};

/// One detached checkpoint duty. `Kept` and `Refused` are not
/// success: they do not move `ckpt_sum` and do not subtract the meter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ran {
    Replaced {
        sum: u64,
    },
    Kept {
        incumbent: [u8; 32],
    },
    Refused(PublishRefusal),
    /// Pending, torn, or unreadable: the next cadence crossing re-arms.
    Deferred,
}

/// Releases `duty_busy` on every exit, including unwind.
struct DutyBusy<'a, T: Theory + Clone> {
    core: &'a Mutex<Core<T>>,
}

impl<T: Theory + Clone> Drop for DutyBusy<'_, T> {
    fn drop(&mut self) {
        lock(self.core).duty_busy = false;
    }
}

fn settled_view<T: Theory + Clone>(
    core: &Core<T>,
) -> Option<(Arc<Db<T>>, BTreeMap<BraidId, ChainEntry>)> {
    match (&core.db, &core.chain) {
        (Some(db), Chain::Settled { entries }) => Some((Arc::clone(db), entries.clone())),
        (Some(_), Chain::Pending { .. }) | (None, _) => None,
    }
}

impl<T, S, H> Inner<T, S, H>
where
    T: Theory + Clone + Send + Sync + 'static,
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    /// Cadence detection is the commit path's whole share of the
    /// checkpoint duty: compact, digest, uploads, and the manifest CAS
    /// all run on the detached thread, off the commit lock. A Pending
    /// chain cannot compact, so it does not arm.
    pub(crate) fn maybe_duty(self: &Arc<Self>, core: &mut Core<T>) {
        let sum = core.chain.sum();
        if core.duty_busy
            || matches!(core.chain, Chain::Pending { .. })
            || (sum.saturating_sub(core.ckpt_sum) < core.cadence_sum
                && core.log_bytes < core.cadence_bytes)
        {
            return;
        }
        core.duty_busy = true;
        let inner = Arc::clone(self);
        let handle = std::thread::spawn(move || {
            let _ran = inner.run_duty();
        });
        lock(&self.threads).push(handle);
    }

    /// The checkpoint duty, entirely off the commit lock. The
    /// consistent view is proven rather than scheduled: snapshot a
    /// Settled chain and the store handle under a short lock, compact
    /// off the lock, then re-take the lock and require that the chain
    /// is still that Settled value — the same handle, the same heads,
    /// the generation still the snapshot sum — which proves the
    /// compacted copy holds exactly the snapshot's content. A torn
    /// view retries with the legible scream; commits never wait on
    /// the duty.
    pub(crate) fn run_duty(self: &Arc<Self>) -> Ran {
        let _busy = DutyBusy { core: &self.core };
        let mut scream = Scream::new("checkpoint duty");
        let seq = self.scratch_seq.fetch_add(1, Ordering::Relaxed);
        let scratch = PathBuf::from(format!("{}.ckpt{seq}", self.dir.display()));
        let view = loop {
            let snapshot = {
                let core = lock(&self.core);
                settled_view(&core)
            };
            let Some((db, entries)) = snapshot else {
                // Pending, or the store is mid re-open: the next
                // cadence crossing re-arms the duty.
                return Ran::Deferred;
            };
            let sum = entries
                .values()
                .fold(0u64, |acc, entry| acc.saturating_add(entry.g));
            let _ = fs::remove_dir_all(&scratch);
            let compacted: std::result::Result<Vec<u8>, ()> = (|| {
                db.compact(&scratch).map_err(|_| ())?;
                fs::read(scratch.join(DATA_FILE)).map_err(|_| ())
            })();
            let _ = fs::remove_dir_all(&scratch);
            let Ok(bytes) = compacted else {
                return Ran::Deferred;
            };
            let Ok(catalog) = db.catalog_digest() else {
                return Ran::Deferred;
            };
            let consistent = {
                let core = lock(&self.core);
                match settled_view(&core) {
                    Some((current, now))
                        if Arc::ptr_eq(&current, &db)
                            && now == entries
                            && db.generation().map(bumbledb::GenerationId::value) == Ok(sum) =>
                    {
                        Some(core.log_bytes)
                    }
                    _ => None,
                }
            };
            if let Some(snap_bytes) = consistent {
                break (bytes, catalog, entries, sum, snap_bytes);
            }
            scream.attempt("a commit landed inside the snapshot window");
        };
        let (bytes, catalog, entries, sum, snap_bytes) = view;
        let heads: BTreeMap<BraidId, Head> = entries
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
        let prev = match self.store.get(&manifest_key(&self.prefix)) {
            Ok(Some(fetched)) => match Manifest::parse(&fetched.bytes) {
                Ok(manifest) => manifest.checkpoint,
                Err(_) => return Ran::Deferred,
            },
            Ok(None) => None,
            Err(_) => return Ran::Deferred,
        };
        let doc = Checkpoint {
            braids: heads,
            catalog,
            writer: self.writer_id,
            prev,
        };
        let digest = doc.digest();
        let ran = match (|| -> std::result::Result<Published, ()> {
            self.store
                .put_create(&ckpt_mdb_key(&self.prefix, &digest), &bytes)
                .map_err(|_| ())?;
            publish_checkpoint(self.store.as_ref(), &self.prefix, self.codec.braids(), &doc)
                .map_err(|_| ())
        })() {
            Ok(Published::Replaced) => Ran::Replaced { sum },
            Ok(Published::Kept { incumbent }) => Ran::Kept { incumbent },
            Ok(Published::Refused(refusal)) => Ran::Refused(refusal),
            Err(()) => Ran::Deferred,
        };
        if let Ran::Replaced { sum } = ran {
            let mut core = lock(&self.core);
            core.ckpt_sum = core.ckpt_sum.max(sum);
            core.log_bytes = core.log_bytes.saturating_sub(snap_bytes);
        }
        ran
    }
}
