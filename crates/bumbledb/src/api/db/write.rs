use std::sync::PoisonError;

use super::{BULK_CHUNK, BulkLoadError, CommitSeq, Db, Fact, Snapshot, WriteTx, WriterThreadReset};
use crate::error::{Error, Result};
use crate::ir::Value;
use crate::storage::commit::{commit, crashpoint};
use crate::storage::delta::WriteDelta;
use bumbledb_theory::schema::RelationId;

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

impl<S> Db<S> {
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
    /// `f`'s error, or commit-time `CommitRejected` (the complete
    /// violation set, in materialized statement order) /
    /// `FreshExhausted` / `Lmdb` / `Io`.
    ///
    /// # Panics
    ///
    /// On a nested call from within a write closure on the same thread —
    /// `write` is non-reentrant, and a loud panic beats the silent
    /// forever-deadlock the writer mutex would otherwise become.
    pub fn write<R>(&self, f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>) -> Result<R> {
        self.write_witnessed(None, f)
    }

    /// [`Db::write`], conditional on a witness: the read-compute-write
    /// sequence as a value (`docs/architecture/70-api.md` § conditional
    /// writes). The witness is the [`Snapshot`] the host read its
    /// premises on — evidence, never a raw integer a caller could
    /// fabricate or stale-cache (the recorded refusal). Inside the
    /// writer's critical section, before any page is touched, the
    /// current state-changing generation is compared against the
    /// witness's: on mismatch the whole transaction aborts with
    /// [`Error::GenerationMoved`] and the delta drops exactly as any
    /// abort does — `f` never runs. The compare targets the same
    /// generation the image cache keys on, so a counters-only/no-op
    /// commit does not trip it.
    ///
    /// The engine ships the error, never a loop — retry is host policy:
    /// re-run the query, re-compute, `write_from` again. `Snapshot`
    /// exposes no `generation()` accessor: the witness consumes the
    /// generation internally, and the diagnostics surface is
    /// [`Db::generation`] (decided — nothing new ships until the stats
    /// surface wants it).
    ///
    /// # Errors
    ///
    /// [`Error::ForeignSnapshot`] on a witness from another database
    /// (the environment-identity check prepared queries run);
    /// [`Error::GenerationMoved`] when a state-changing commit landed
    /// after the witness; otherwise as [`Db::write`].
    ///
    /// # Panics
    ///
    /// As [`Db::write`] (non-reentrant).
    pub fn write_from<R>(
        &self,
        witness: &Snapshot<'_, S>,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<R> {
        if witness.txn().env_instance() != self.env.instance() {
            return Err(Error::ForeignSnapshot);
        }
        // Read inside the witness's own transaction (snapshot-constant;
        // the existing race-closer) — holding no lock across any read
        // phase: the writer mutex is taken only below.
        let witnessed = witness.txn().generation()?;
        self.write_witnessed(Some(witnessed), f)
    }

    /// The one write body. `witnessed` is the only difference between
    /// [`Db::write`] and [`Db::write_from`]: one integer compare inside
    /// the critical section, cold on the success path.
    fn write_witnessed<R>(
        &self,
        witnessed: Option<crate::GenerationId>,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<R> {
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
        let _writer_lock = self.writer.lock().unwrap_or_else(PoisonError::into_inner);
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
        let view = self.env.read_txn()?;
        // The generation witness (`Db::write_from`): current state-changing
        // generation, read inside the critical section, against the
        // witness's. Mismatch aborts before any page is touched.
        if let Some(witnessed) = witnessed {
            let current = view.generation()?;
            if current != witnessed {
                return Err(Error::GenerationMoved { witnessed, current });
            }
        }
        let mut txn_span =
            crate::obs::span(crate::obs::names::WRITE_TXN, crate::obs::Category::Commit);
        let mut tx = WriteTx {
            view,
            delta: WriteDelta::new(&self.schema),
            schema: &self.schema,
            scratch: Vec::new(),
            refs: Vec::new(),
            marker: std::marker::PhantomData,
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
            CommitSeq::advance(&self.commit_seq, Ordering::Release);
            crashpoint!("after-memo-update");
        }
        Ok(out)
    }

    /// Imports typed facts in chunks of 4096 per write transaction — the
    /// same delta mechanism at scale, over the generated fact structs
    /// (the typed lane is the bulk surface too — the unified-surface
    /// ruling, `docs/architecture/70-api.md` § ETL). Explicit fresh
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
    /// import sizable and resumable. Per fact as [`WriteTx::insert`].
    ///
    /// # Panics
    ///
    /// Inside a [`Db::write`] closure on the same thread: `bulk_load`
    /// chunks through `Db::write` internally, so it inherits the
    /// non-reentrancy panic (the assert fires before the delta or LMDB
    /// is touched — the outer transaction aborts cleanly by unwind).
    /// Run the import outside the transaction.
    pub fn bulk_load<'f, F, I>(&self, facts: I) -> std::result::Result<u64, BulkLoadError>
    where
        F: Fact<'f, Schema = S>,
        I: IntoIterator<Item = F>,
    {
        self.bulk_chunks(facts.into_iter(), |tx, fact| tx.insert(&fact))
    }

    /// [`Db::bulk_load`]'s dynamic sibling (the ETL/FFI lane, pairing
    /// with [`Snapshot::scan`]'s dynamic export): one [`Value`] row per
    /// fact, in declaration order. Same chunking, same partial-import
    /// contract.
    ///
    /// # Errors
    ///
    /// As [`Db::bulk_load`]; shape problems are typed `FactShape` errors,
    /// as [`WriteTx::insert_dyn`].
    ///
    /// # Panics
    ///
    /// As [`Db::bulk_load`] — the same chunking loop runs through
    /// [`Db::write`], so the same non-reentrancy panic applies.
    pub fn bulk_load_dyn<I>(
        &self,
        rel: RelationId,
        facts: I,
    ) -> std::result::Result<u64, BulkLoadError>
    where
        I: IntoIterator<Item = Vec<Value>>,
    {
        self.bulk_chunks(facts.into_iter(), |tx, values| tx.insert_dyn(rel, &values))
    }

    /// The one chunking loop under both bulk lanes: 4096 facts per write
    /// transaction, each chunk atomic, the committed count carried on
    /// failure.
    fn bulk_chunks<T>(
        &self,
        mut facts: impl Iterator<Item = T>,
        mut apply: impl FnMut(&mut WriteTx<'_, S>, T) -> Result<bool>,
    ) -> std::result::Result<u64, BulkLoadError> {
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
                    let Some(fact) = facts.next() else {
                        exhausted = true;
                        break;
                    };
                    submitted += 1;
                    if apply(tx, fact)? {
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

/// `?` in `crate::Result` contexts keeps the count: it carries into
/// [`crate::error::Error::BulkLoad`] — the count is the whole reason
/// [`BulkLoadError`] exists (resumable partial imports).
impl From<BulkLoadError> for crate::error::Error {
    fn from(err: BulkLoadError) -> Self {
        Self::BulkLoad {
            committed: err.committed,
            error: Box::new(err.error),
        }
    }
}
