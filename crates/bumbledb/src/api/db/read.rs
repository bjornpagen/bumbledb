//! [`Db::read`]: one owned coherent snapshot per lease.
//!
//! Deliberately no parked-reader cache: a permanently parked LMDB read
//! transaction would block the elastic map's exclusive resize forever. The
//! store's gate reports long-held snapshots by age instead of invalidating
//! them; leases are scoped to the closure, so growth is never starved by
//! the embedding read path.

use std::marker::PhantomData;

use super::{Db, ReadInstance, embedded_work};
use crate::error::{Error, Result};

impl<S> Db<S> {
    /// Prepare one query against a scoped read lease. The prepared plan is
    /// owned and outlives the lease; execution binds a lease again.
    /// # Errors
    /// Prepare-time validation or storage failure.
    pub fn prepare(&self, query: &crate::ir::Query) -> Result<crate::PreparedQuery<S>> {
        self.read(|instance| instance.prepare(query))
    }

    /// Runs `f` over one read lease: a coherent generation for prepared
    /// queries, point reads and export.
    /// # Errors
    /// Storage failure opening the snapshot, or the closure's own error.
    pub fn read<R>(&self, f: impl FnOnce(&ReadInstance<'_, S>) -> Result<R>) -> Result<R> {
        let work = embedded_work()?;
        let snapshot = self.store.snapshot(&work).map_err(Error::from_store)?;
        let instance = ReadInstance {
            schema: &self.schema,
            closed: self.closed.as_ref(),
            snapshot,
            work,
            thread_bound: PhantomData,
            marker: PhantomData,
        };
        f(&instance)
    }
}
