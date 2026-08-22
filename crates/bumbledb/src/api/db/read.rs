use super::{Db, ParkedReader, ReadInstance, ScratchPool};
use crate::error::Result;
use crate::image::LmdbSource;
use crate::storage::env::GenerationId;

impl<S> Db<S> {
    /// Runs `f` over one LMDB read lease: a consistent generation for
    /// # Errors
    pub fn read<R>(&self, f: impl FnOnce(&ReadInstance<'_, S>) -> Result<R>) -> Result<R> {
        use std::sync::atomic::Ordering;
        let generation = GenerationId::from_storage(self.generation.load(Ordering::Acquire));
        let parked = self
            .read_cache
            .try_lock()
            .ok()
            .and_then(|mut slot| slot.take())
            .and_then(|parked| (parked.generation == generation).then_some(parked.txn));
        let txn = match parked {
            Some(raw) => self.env.resume_read_txn(raw),
            None => self.env.read_txn()?,
        };
        let scratch = self
            .scratch
            .try_lock()
            .ok()
            .and_then(|mut slot| slot.take())
            .unwrap_or_else(ScratchPool::new);
        let instance = ReadInstance {
            core: super::instance::InstanceCore::assemble(
                std::sync::Arc::clone(&self.schema),
                self.env.identity().clone(),
                LmdbSource::new(txn, &self.cache),
                scratch,
            ),
            thread_bound: std::marker::PhantomData,
        };
        let result = f(&instance);

        let ReadInstance { core, .. } = instance;
        let (source, scratch) = core.into_parts();
        let txn = source.into_txn();
        if GenerationId::from_storage(self.generation.load(Ordering::Acquire)) == generation
            && let Ok(mut slot) = self.read_cache.try_lock()
            && slot.is_none()
        {
            *slot = Some(ParkedReader {
                txn: txn.into_raw_txn(),
                generation,
            });
        }
        if let Ok(mut slot) = self.scratch.try_lock()
            && slot.is_none()
        {
            *slot = Some(scratch);
        }
        result
    }
}
