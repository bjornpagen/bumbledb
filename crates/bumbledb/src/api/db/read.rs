use super::{Db, ParkedReader, Snapshot};
use crate::error::Result;

impl Db<'_> {
    /// Runs `f` over one LMDB read snapshot: a consistent generation for
    /// every query and scan inside. Reuses the parked reader when no
    /// commit intervened — same snapshot bits, no
    /// `mdb_txn_begin`.
    ///
    /// # Errors
    ///
    /// `Lmdb` on snapshot open; otherwise whatever `f` returns.
    pub fn read<R>(&self, f: impl FnOnce(&Snapshot<'_>) -> Result<R>) -> Result<R> {
        use std::sync::atomic::Ordering;
        let seq = self.commit_seq.load(Ordering::Acquire);
        let parked = self
            .read_cache
            .try_lock()
            .ok()
            .and_then(|mut slot| slot.take())
            .and_then(|parked| {
                // A stale parked snapshot drops here — freeing its
                // reader slot and unpinning its pages.
                (parked.commit_seq == seq).then_some(parked.txn)
            });
        let txn = match parked {
            Some(raw) => self.env.resume_read_txn(raw),
            None => self.env.read_txn()?,
        };
        let snap = Snapshot {
            txn,
            cache: &self.cache,
            schema: self.schema,
        };
        let result = f(&snap);
        // Park the snapshot for the next read — only if it is still
        // current (a concurrent commit may have landed while `f` ran)
        // and the slot is free. A snapshot that fails either check
        // drops here, freeing its reader slot.
        let Snapshot { txn, .. } = snap;
        if self.commit_seq.load(Ordering::Acquire) == seq {
            if let Ok(mut slot) = self.read_cache.try_lock() {
                if slot.is_none() {
                    *slot = Some(ParkedReader {
                        txn: txn.into_raw_txn(),
                        commit_seq: seq,
                    });
                }
            }
        }
        result
    }
}
