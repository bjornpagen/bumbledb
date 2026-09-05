//! [`Db::snapshot`]: one owned coherent snapshot the caller pins.
//! [`Db::read`]: one ephemeral frame over that pin for a scoped operation.
//!
//! Deliberately no parked-reader cache: a permanently parked LMDB read
//! transaction would block the elastic map's exclusive resize forever. The
//! store's gate reports long-held snapshots by age instead of invalidating
//! them. A caller's own pinned [`super::OwnedRead`] blocks resize as a
//! typed refusal; live mapped pages are never invalidated.

use std::marker::PhantomData;
use std::sync::Arc;

use super::{Db, OwnedRead, ReadFrame};
use crate::error::{Error, Result};
use crate::work::{ExecutionPolicy, WorkContext};

impl<S> Db<S> {
    /// Pin one owned coherent snapshot. Work is charged only for admitting
    /// the pin; each later operation takes a fresh frame and its own work.
    /// [`ImageCache::acquire`](crate::image::cache::ImageCache::acquire) is
    /// the generation pin — there is no `pin_generation`.
    ///
    /// # Errors
    /// Storage failure opening the snapshot, or stopped work.
    pub fn snapshot(&self, work: &WorkContext) -> Result<OwnedRead<S>> {
        let snapshot = self.store.snapshot(work).map_err(Error::from_store)?;
        Ok(OwnedRead {
            schema: Arc::clone(&self.schema),
            closed: Arc::clone(&self.closed),
            snapshot,
            cache: Arc::clone(&self.cache),
            pin: self.cache.acquire(),
            marker: PhantomData,
        })
    }

    /// Native worker-table pin. Admits the snapshot under a dedicated
    /// budget; later frames take their own work. Prefer [`Self::snapshot`]
    /// when the caller already holds an operation context.
    ///
    /// # Errors
    /// As [`Self::snapshot`].
    pub fn owned_read(&self) -> Result<OwnedRead<S>> {
        let work = crate::api::db::start_operation(ExecutionPolicy {
            input_bytes: 1 << 20,
            working_bytes: 1 << 20,
            scratch_bytes: 1 << 20,
            result_bytes: 1 << 20,
            rows: 1 << 20,
            work_units: 1 << 20,
            timeout: std::time::Duration::from_secs(60),
        })?;
        self.snapshot(&work)
    }

    /// Prepare one query against a scoped read frame. The prepared plan is
    /// owned and outlives the frame; execution binds a frame again.
    /// # Errors
    /// Prepare-time validation or storage failure.
    pub fn prepare(&self, query: &crate::ir::Query, work: WorkContext) -> Result<crate::PreparedQuery<S>> {
        self.read(work, |frame| frame.prepare(query))
    }

    /// Runs `f` over one operation frame with an explicit work budget.
    /// Prefer [`Self::snapshot`] when the pin must outlive a single call.
    /// # Errors
    /// Storage failure opening the snapshot, or the closure's own error.
    pub fn read<R>(
        &self,
        work: WorkContext,
        f: impl FnOnce(&ReadFrame<'_, S>) -> Result<R>,
    ) -> Result<R> {
        let owned = self.snapshot(&work)?;
        f(&owned.frame(&work))
    }
}
