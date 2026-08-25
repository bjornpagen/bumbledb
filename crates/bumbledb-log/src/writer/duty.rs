//! Checkpoint duty: cadence detection on the commit path, compact and
//! CAS off the lock. Commits never wait on the duty. Compact's input
//! is a Settled chain; the result is a total sum whose only success
//! is Replaced.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use bumbledb::Db;

use crate::braids::BraidId;
use crate::checkpointer::compact_and_publish;
use crate::manifest::{PublishRefusal, Published};
use crate::sidecar::{Chain, ChainEntry};
use crate::store::ObjectStore;

use super::{Core, Inner, StepHook, Theory, lock};

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

    /// The resident cadence entry. Snapshot a Settled chain under a
    /// short lock, then the compact→publish transition; `hold` proves
    /// the chain is still that Settled value — the same handle, the
    /// same heads, the generation still the snapshot sum — so the
    /// compacted copy holds exactly the snapshot's content. A torn
    /// view retries with the legible scream; commits never wait on
    /// the duty.
    pub(crate) fn run_duty(self: &Arc<Self>) -> Ran {
        let _busy = DutyBusy { core: &self.core };
        let seq = self.scratch_seq.fetch_add(1, Ordering::Relaxed);
        let scratch = PathBuf::from(format!("{}.ckpt{seq}", self.dir.display()));
        loop {
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
            let mut snap_bytes = 0u64;
            let published = match compact_and_publish(
                self.store.as_ref(),
                &self.prefix,
                &self.dir,
                self.codec.braids(),
                self.writer_id,
                db.as_ref(),
                &entries,
                &scratch,
                || {
                    let core = lock(&self.core);
                    match settled_view(&core) {
                        Some((current, now))
                            if Arc::ptr_eq(&current, &db)
                                && now == entries
                                && db.generation().map(bumbledb::GenerationId::value)
                                    == Ok(sum) =>
                        {
                            snap_bytes = core.log_bytes;
                            true
                        }
                        _ => false,
                    }
                },
            ) {
                Ok(Some(published)) => published,
                Ok(None) => {
                    self.scream("a commit landed inside the snapshot window");
                    continue;
                }
                Err(_) => return Ran::Deferred,
            };
            let ran = match published {
                Published::Replaced => Ran::Replaced { sum },
                Published::Kept { incumbent } => Ran::Kept { incumbent },
                Published::Refused(refusal) => Ran::Refused(refusal),
            };
            if let Ran::Replaced { sum } = ran {
                let mut core = lock(&self.core);
                core.ckpt_sum = core.ckpt_sum.max(sum);
                core.log_bytes = core.log_bytes.saturating_sub(snap_bytes);
            }
            return ran;
        }
    }
}
