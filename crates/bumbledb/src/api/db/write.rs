use std::sync::PoisonError;

use super::{BulkLoadError, Db, WriteTx, WriterThreadReset, BULK_CHUNK};
use crate::error::Result;
use crate::ir::Value;
use crate::schema::RelationId;
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;

/// A per-thread key, distinct process-wide (never 0). `ThreadId`
/// itself has no stable integer form, so each thread mints one from a
/// shared counter on first use.
fn thread_key() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    thread_local! {
        static KEY: u64 = NEXT.fetch_add(1, Ordering::Relaxed);
    }
    KEY.with(|key| *key)
}

impl Drop for WriterThreadReset<'_> {
    fn drop(&mut self) {
        self.0.store(0, std::sync::atomic::Ordering::Release);
    }
}

impl Db<'_> {
    /// Runs `f` as the single writer: takes the writer mutex, hands `f` a
    /// delta transaction, and commits on `Ok`. `Err` or panic drops the
    /// delta — LMDB was never touched. Dependency statements are judged at
    /// commit against the final state; a violation aborts the whole
    /// transaction.
    ///
    /// Queries are not reachable from the write closure — [`WriteTx`]
    /// simply offers none (forbidden by representation, `70-api.md`).
    /// Read-modify-write is served by the point reads
    /// ([`WriteTx::contains`] / [`WriteTx::get`] / [`WriteTx::get_dyn`]),
    /// which observe the final-state view the judgment phase will judge —
    /// check-then-act is race-free by construction (single writer, one
    /// view).
    ///
    /// # Errors
    ///
    /// `f`'s error, or commit-time `FunctionalityViolation` /
    /// `ContainmentViolation` / `SerialExhausted` / `Lmdb` / `Io`.
    ///
    /// # Panics
    ///
    /// On a nested call from within a write closure on the same thread —
    /// `write` is non-reentrant, and a loud panic beats the silent
    /// forever-deadlock the writer mutex would otherwise become.
    pub fn write<R>(&self, f: impl FnOnce(&mut WriteTx<'_>) -> Result<R>) -> Result<R> {
        use std::sync::atomic::Ordering;
        let caller = thread_key();
        assert_ne!(
            self.writer_thread.load(Ordering::Acquire),
            caller,
            "nested Db::write — re-entrant write transactions are forbidden"
        );
        // A panicking closure poisons nothing real: the delta died in the
        // unwind and LMDB was never touched, so the flag is cleared rather
        // than propagated.
        let _guard = self.writer.lock().unwrap_or_else(PoisonError::into_inner);
        self.writer_thread.store(caller, Ordering::Release);
        let _owner = WriterThreadReset(&self.writer_thread);
        // Drop the parked reader before writing: a
        // pinned old snapshot blocks LMDB page reuse for the writer.
        drop(
            self.read_cache
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take(),
        );
        let mut txn_span =
            crate::obs::span(crate::obs::names::WRITE_TXN, crate::obs::Category::Commit);
        let mut tx = WriteTx {
            view: self.env.read_txn()?,
            delta: WriteDelta::new(self.schema),
            schema: self.schema,
            scratch: Vec::new(),
            refs: Vec::new(),
        };
        let out = f(&mut tx)?;
        let WriteTx { view, delta, .. } = tx;
        drop(view);
        let report = commit(delta, &self.env)?;
        txn_span.set_args(1, 0);
        txn_span.end();
        if report.changed {
            // The one commit → cache wiring point (`40-storage.md`):
            // images of older generations are stale the moment the new
            // generation exists.
            self.cache.evict_older_than(report.new_generation);
            // Invalidate any snapshot parked mid-write by a concurrent
            // reader: the next read must begin fresh.
            self.commit_seq.fetch_add(1, Ordering::Release);
        }
        Ok(out)
    }

    /// Imports dynamic facts in chunks of 4096 per write
    /// transaction — the same delta mechanism at scale. Explicit serial
    /// values preserve identity: the high-water mark advances past them.
    /// Returns the number of facts that changed state.
    ///
    /// A fresh-database append-order fast path is a documented possibility
    /// (`40-storage.md`) deliberately not taken: it saves only the
    /// membership probes on an empty database, and the normal insert path
    /// is the one with the invariants (decision: do not gold-plate).
    ///
    /// # Errors
    ///
    /// [`BulkLoadError`]: the underlying error plus how many facts had
    /// already committed — a failing chunk aborts that chunk whole,
    /// leaving prior chunks committed, and the count makes the partial
    /// import sizable and resumable. Shape problems are typed `FactShape`
    /// errors, as [`WriteTx::insert_dyn`].
    pub fn bulk_load<I>(&self, rel: RelationId, facts: I) -> std::result::Result<u64, BulkLoadError>
    where
        I: IntoIterator<Item = Vec<Value>>,
    {
        let mut iter = facts.into_iter();
        let mut total = 0u64;
        loop {
            let mut exhausted = false;
            // Folded into `total` only after the chunk commits: a failing
            // chunk aborts whole, so its partial successes never happened.
            let mut chunk = 0u64;
            let mut submitted = 0u64;
            let mut chunk_span =
                crate::obs::span(crate::obs::names::BULK_CHUNK, crate::obs::Category::Commit);
            self.write(|tx| {
                for _ in 0..BULK_CHUNK {
                    let Some(values) = iter.next() else {
                        exhausted = true;
                        break;
                    };
                    submitted += 1;
                    if tx.insert_dyn(rel, &values)? {
                        chunk += 1;
                    }
                }
                Ok(())
            })
            .map_err(|error| BulkLoadError {
                committed: total,
                error,
            })?;
            chunk_span.set_args(submitted, chunk);
            chunk_span.end();
            total += chunk;
            if exhausted {
                return Ok(total);
            }
        }
    }
}

impl std::fmt::Display for BulkLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "bulk load failed after {} committed facts: {}",
            self.committed, self.error
        )
    }
}

impl std::error::Error for BulkLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Dropping the count keeps `?` working in `crate::Result` contexts.
impl From<BulkLoadError> for crate::error::Error {
    fn from(err: BulkLoadError) -> Self {
        err.error
    }
}
