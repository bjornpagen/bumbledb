//! [`Db::snapshot`]: one owned coherent snapshot the caller pins.
//! [`Db::read`]: one frame over a scoped snapshot for the duration of a closure.

use std::marker::PhantomData;
use std::sync::Arc;

use super::{Db, OwnedRead, ReadFrame};
use crate::error::Result;
use crate::work::WorkContext;

impl<S> Db<S> {
    /// Pin one owned coherent snapshot. The context controls admission;
    /// each later operation takes a fresh frame and its own cancellation.
    /// Canonical rows belong to the LMDB snapshot. Query operations acquire
    /// their own current cache resolver only while interpreting text tokens.
    ///
    /// # Errors
    /// Storage failure opening the snapshot, or stopped work.
    pub fn snapshot(&self, work: &WorkContext) -> Result<OwnedRead<S>> {
        let snapshot = self.store.snapshot(work)?;
        Ok(OwnedRead {
            schema: Arc::clone(&self.schema),
            snapshot,
            cache: Arc::clone(&self.cache),
            marker: PhantomData,
        })
    }

    /// Native worker-table pin. Admits the snapshot with a fresh context;
    /// later frames take their own cancellation. Prefer [`Self::snapshot`]
    /// when the caller already holds an operation context.
    ///
    /// # Errors
    /// As [`Self::snapshot`].
    pub fn owned_read(&self) -> Result<OwnedRead<S>> {
        let work = WorkContext::new();
        self.snapshot(&work)
    }

    /// Prepare one query against a scoped read frame. The prepared plan is
    /// owned and outlives the frame; execution binds a frame again.
    /// # Errors
    /// Prepare-time validation or storage failure.
    pub fn prepare(
        &self,
        query: &crate::ir::Query,
        work: WorkContext,
    ) -> Result<crate::PreparedQuery<S>> {
        self.read(work, |frame| frame.prepare(query))
    }

    /// Runs `f` over one operation frame with cooperative cancellation.
    /// Prefer [`Self::snapshot`] when the pin must outlive a single call.
    /// A scoped frame cannot escape its read:
    /// ```compile_fail
    /// fn escape<'a>(
    ///     db: &'a bumbledb::Db<()>,
    ///     work: bumbledb::WorkContext,
    /// ) -> &'a bumbledb::ReadFrame<'a, ()> {
    ///     db.read(work, |frame| Ok(frame)).unwrap()
    /// }
    /// ```
    /// Nor can a fact borrowing that frame's mapped rows:
    /// ```compile_fail
    /// fn escape_fact<'a, S, K: bumbledb::Key<'a, Schema = S>>(
    ///     db: &'a bumbledb::Db<S>,
    ///     work: bumbledb::WorkContext,
    ///     key: K,
    /// ) -> Option<K::Fact> {
    ///     db.read(work, |frame| frame.get(key)).unwrap()
    /// }
    /// ```
    /// # Errors
    /// Storage failure opening the snapshot, or the closure's own error.
    pub fn read<R>(
        &self,
        work: WorkContext,
        f: impl FnOnce(&ReadFrame<'_, S>) -> Result<R>,
    ) -> Result<R> {
        let snapshot = self.store.snapshot(&work)?;
        f(&ReadFrame {
            schema: &self.schema,
            snapshot: &snapshot,
            cache: &self.cache,
            work: &work,
            marker: PhantomData,
        })
    }
}
