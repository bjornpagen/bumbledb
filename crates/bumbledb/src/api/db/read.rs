use super::{Db, ParkedReader, ReadInstance};
use crate::error::Result;
use crate::storage::env::GenerationId;

impl<S> Db<S> {
    /// Runs `f` over one LMDB read snapshot: a consistent generation for
    /// every query and scan inside. Reuses the parked reader when no
    /// commit intervened — same snapshot bits, no
    /// `mdb_txn_begin`.
    ///
    /// # Errors
    ///
    /// `Lmdb` on snapshot open; otherwise whatever `f` returns.
    pub fn read<R>(&self, f: impl FnOnce(&ReadInstance<'_, S>) -> Result<R>) -> Result<R> {
        use std::sync::atomic::Ordering;
        let generation = GenerationId::from_storage(self.generation.load(Ordering::Acquire));
        let parked = self
            .read_cache
            .try_lock()
            .ok()
            .and_then(|mut slot| slot.take())
            .and_then(|parked| {
                // A stale parked snapshot drops here — freeing its
                // reader slot and unpinning its pages.
                (parked.generation == generation).then_some(parked.txn)
            });
        let txn = match parked {
            Some(raw) => self.env.resume_read_txn(raw),
            None => self.env.read_txn()?,
        };
        let snap = ReadInstance {
            txn,
            cache: &self.cache,
            schema: std::sync::Arc::clone(&self.schema),
            scratch: &self.read_scratch,
            thread_bound: std::marker::PhantomData,
            marker: std::marker::PhantomData,
        };
        let result = f(&snap);
        // Park the snapshot for the next read — only if it is still
        // current (a concurrent commit may have landed while `f` ran)
        // and the slot is free. A snapshot that fails either check
        // drops here, freeing its reader slot.
        let ReadInstance { txn, .. } = snap;
        if GenerationId::from_storage(self.generation.load(Ordering::Acquire)) == generation
            && let Ok(mut slot) = self.read_cache.try_lock()
            && slot.is_none()
        {
            *slot = Some(ParkedReader {
                txn: txn.into_raw_txn(),
                generation,
            });
        }
        result
    }
}
