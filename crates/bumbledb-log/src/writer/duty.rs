//! Checkpoint duty: cadence detection on the commit path, compact and
//! CAS off the lock. Commits never wait on the duty.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::braids::BraidId;
use crate::manifest::{Head, Published, ckpt_mdb_key, publish_checkpoint};
use crate::replica::Scream;
use crate::store::ObjectStore;

use super::{Core, DATA_FILE, Inner, StepHook, Theory, lock};

impl<T, S, H> Inner<T, S, H>
where
    T: Theory + Clone + Send + Sync + 'static,
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    /// Cadence detection is the commit path's whole share of the
    /// checkpoint duty: compact, digest, uploads, and the manifest CAS
    /// all run on the detached thread, off the commit lock.
    pub(crate) fn maybe_duty(self: &Arc<Self>, core: &mut Core<T>) {
        let sum = core.chain.sum();
        if core.duty_busy
            || (sum.saturating_sub(core.ckpt_sum) < core.cadence_sum
                && core.log_bytes < core.cadence_bytes)
        {
            return;
        }
        core.duty_busy = true;
        let inner = Arc::clone(self);
        let handle = std::thread::spawn(move || inner.run_duty());
        lock(&self.threads).push(handle);
    }

    /// The checkpoint duty, entirely off the commit lock. The
    /// consistent view is proven rather than scheduled: snapshot the
    /// store handle and the heads under a short lock, compact off the
    /// lock, then re-take the lock and require that nothing moved — the
    /// same handle, the same heads, no pending term, the generation
    /// still the snapshot sum — which proves the compacted copy holds
    /// exactly the snapshot's content. A torn view retries with the
    /// legible scream; commits never wait on the duty.
    pub(crate) fn run_duty(self: &Arc<Self>) {
        let mut scream = Scream::new("checkpoint duty");
        let seq = self.scratch_seq.fetch_add(1, Ordering::Relaxed);
        let scratch = PathBuf::from(format!("{}.ckpt{seq}", self.dir.display()));
        let view = loop {
            let snapshot = {
                let core = lock(&self.core);
                match (&core.db, core.chain.pending.is_some()) {
                    (Some(db), false) => Some((Arc::clone(db), core.chain.entries.clone())),
                    _ => None,
                }
            };
            let Some((db, entries)) = snapshot else {
                // A pending slot is occupied or the store is mid
                // re-open: the next cadence crossing re-arms the duty.
                break None;
            };
            let sum: u64 = entries.values().map(|entry| entry.g).sum();
            let _ = fs::remove_dir_all(&scratch);
            let compacted: std::result::Result<Vec<u8>, ()> = (|| {
                db.compact(&scratch).map_err(|_| ())?;
                fs::read(scratch.join(DATA_FILE)).map_err(|_| ())
            })();
            let _ = fs::remove_dir_all(&scratch);
            let Ok(bytes) = compacted else { break None };
            let Ok(catalog) = db.catalog_digest() else {
                break None;
            };
            let consistent = {
                let core = lock(&self.core);
                core.db
                    .as_ref()
                    .is_some_and(|current| Arc::ptr_eq(current, &db))
                    && core.chain.pending.is_none()
                    && core.chain.entries == entries
                    && db.generation().map(bumbledb::GenerationId::value) == Ok(sum)
            };
            if consistent {
                break Some((bytes, catalog, entries, sum));
            }
            scream.attempt("a commit landed inside the snapshot window");
        };
        let Some((bytes, catalog, entries, sum)) = view else {
            lock(&self.core).duty_busy = false;
            return;
        };
        let digest = *blake3::hash(&bytes).as_bytes();
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
        let outcome = (|| -> std::result::Result<bool, ()> {
            self.store
                .put_create(&ckpt_mdb_key(&self.prefix, &digest), &bytes)
                .map_err(|_| ())?;
            match publish_checkpoint(
                self.store.as_ref(),
                &self.prefix,
                self.codec.braids(),
                digest,
                &heads,
                catalog,
                self.writer_id,
            )
            .map_err(|_| ())?
            {
                Published::Replaced | Published::Kept { .. } => Ok(true),
                Published::Refused(_) => Ok(false),
            }
        })();
        let mut core = lock(&self.core);
        core.duty_busy = false;
        if outcome == Ok(true) {
            core.ckpt_sum = core.ckpt_sum.max(sum);
            core.log_bytes = 0;
        }
    }
}
