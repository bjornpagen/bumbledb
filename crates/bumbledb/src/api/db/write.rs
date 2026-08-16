use std::sync::PoisonError;

use super::{CommitSeq, Db, Snapshot, WriteTx, WriterThreadReset};
use crate::error::{Error, Result};
use crate::storage::commit::{
    commit, crashpoint, flush_escaped_fresh_ids, flush_pending_escaped_fresh_ids,
};
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;

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

/// Burns the escaped fresh high-water when the write region terminates
/// without reaching `commit()` — [`WriterThreadReset`]'s sibling drop
/// guard, and the closure of the one panic gap: `reserve` hands ids to the
/// host before the commit's fate is known, so an `Err`-returning closure
/// and a PANICKING closure alike must burn what escaped
/// (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable` — one
/// `Reachable.txn` transition, the fate irrelevant). Armed while the
/// delta is alive-but-uncommitted; disarmed (the taken-`Option`) once
/// the delta moves into `commit()`, which owns the flush for every path
/// that reaches it — one conceptual owner per region, no path flushing
/// twice.
struct EscapedIdBurn<'a, S> {
    env: &'a Environment,
    /// `Some` from arming until [`EscapedIdBurn::disarm`]; a disarmed
    /// guard drops inert.
    tx: Option<WriteTx<'a, S>>,
}

impl<'a, S> EscapedIdBurn<'a, S> {
    /// Arms the guard around the live transaction.
    fn arm(env: &'a Environment, tx: WriteTx<'a, S>) -> Self {
        Self { env, tx: Some(tx) }
    }

    /// The armed transaction, for the closure region. The slot is `Some`
    /// for the guard's whole life — only [`EscapedIdBurn::disarm`] takes
    /// it, by consuming the guard.
    fn tx(&mut self) -> &mut WriteTx<'a, S> {
        self.tx.as_mut().expect("armed from construction to disarm")
    }

    /// Disarms the guard and releases the transaction toward
    /// `commit()`, which owns the flush from here on.
    fn disarm(mut self) -> WriteTx<'a, S> {
        self.tx.take().expect("armed from construction to disarm")
    }
}

impl<S> Drop for EscapedIdBurn<'_, S> {
    fn drop(&mut self) {
        let Some(tx) = self.tx.take() else {
            // Disarmed: the delta reached `commit()`, which owns the
            // flush from there.
            return;
        };
        let WriteTx { view, delta, .. } = tx;
        // The read view closes before the burn's own write transaction —
        // the same transaction discipline as every other flush site.
        drop(view);
        // Panic-safe: the result is discarded so an unwind never becomes
        // a double-panic. The in-process high-water is raised inside the
        // flush; a disk failure is parked and retried at the next write
        // begin (`never_reissue_observable`).
        let _ = flush_escaped_fresh_ids(self.env, &delta);
    }
}

/// The generation witness, reified (`docs/architecture/70-api.md`
/// § conditional writes): the environment identity and generation one
/// [`Snapshot`] observed, as a plain value. Fields are private and the one
/// construction site is [`Snapshot::witness`], so a witness stays evidence
/// — never an integer a caller could fabricate (the recorded refusal); a
/// stale-cached witness is exactly what the commit-time compare convicts
/// as `GenerationMoved`. The value carries the snapshot's schema typestate
/// (the binding-proof discipline every witness follows) and crosses
/// threads by MOVE, so no `&Snapshot` ever outlives its snapshot to reach
/// the writer — the dangling witness of a closed snapshot is
/// unrepresentable, not defended (findings 018/021).
#[derive(Clone, Copy)]
#[must_use]
pub struct Witness<S> {
    instance: u64,
    generation: crate::GenerationId,
    marker: std::marker::PhantomData<fn() -> S>,
}

impl<S> Snapshot<'_, S> {
    /// Mints this snapshot's [`Witness`]: the generation is read inside
    /// the snapshot's own transaction (snapshot-constant; the existing
    /// race-closer of `50-storage.md`), never through a separate read.
    /// The generation itself stays unreadable — the witness consumes it
    /// internally; the diagnostics surface remains [`Db::generation`].
    ///
    /// # Errors
    ///
    /// `Corruption(MetaMissing)` if the tx-id meta key is absent or
    /// malformed.
    pub fn witness(&self) -> Result<Witness<S>> {
        Ok(Witness {
            instance: self.txn().env_instance(),
            generation: self.txn().generation()?,
            marker: std::marker::PhantomData,
        })
    }
}

impl<S> Db<S> {
    /// Runs `f` as the single writer: takes the writer mutex, hands `f` a
    /// delta transaction, and commits on `Ok`. `Err` or panic drops the
    /// delta — LMDB never saw a fact — but fresh ids the closure already
    /// minted burn either way: the `EscapedIdBurn` drop guard flushes the
    /// escaped high-water on the `Err` exit AND on an unwinding panic,
    /// exactly once, so the never-reissue law holds on every termination
    /// (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`).
    /// Dependency statements are judged at commit against the final
    /// state; a violation aborts the whole transaction.
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
    /// re-run the query, re-compute, `write_from` again. [`Snapshot::generation`]
    /// is the diagnostics read of the same clock the witness consumes
    /// internally (its fields stay private); [`Db::generation`] is the
    /// handle-level sibling.
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
        self.write_from_witness(witness.witness()?, f)
    }

    /// [`Db::write_from`] over the reified [`Witness`] — the lane for a
    /// caller that cannot hold `&Snapshot` across its own boundary (the
    /// FFI bridge parks each snapshot on a worker thread): the snapshot
    /// mints the value where it lives and the value moves here. Same
    /// checks in the same order, same errors, same one-compare cost.
    ///
    /// # Errors
    ///
    /// As [`Db::write_from`].
    ///
    /// # Panics
    ///
    /// As [`Db::write`] (non-reentrant).
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a witness is spent by the write it justifies — the move is the API"
    )]
    pub fn write_from_witness<R>(
        &self,
        witness: Witness<S>,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<R> {
        if witness.instance != self.env.instance() {
            return Err(Error::ForeignSnapshot);
        }
        self.write_witnessed(Some(witness.generation), f)
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
        // A panicking closure poisons nothing real: the unwind burned the
        // delta's escaped fresh ids (the `EscapedIdBurn` guard, under this
        // same lock) and dropped everything else — no fact ever touched
        // LMDB — so the flag is cleared rather than propagated.
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
        flush_pending_escaped_fresh_ids(&self.env)?;
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
        // The burn region: from here until `disarm` hands the delta to
        // `commit()`, EVERY termination — an `Err`-returning closure AND
        // a PANICKING one — burns the escaped fresh high-water through
        // the guard's drop, exactly once. `reserve` may already have handed
        // the host fresh ids, and the never-reissue law binds every id
        // issued, the transaction's fate irrelevant
        // (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`).
        // Declared after `_writer_lock` (locals drop in reverse order),
        // so the burn's counters-only commit runs while the writer lock
        // is still held.
        let mut burn = EscapedIdBurn::arm(
            &self.env,
            WriteTx {
                view,
                delta: WriteDelta::new(&self.schema),
                schema: &self.schema,
                scratch: Vec::new(),
                refs: Vec::new(),
                phase: super::apply::WritePhase::Clean,
                marker: std::marker::PhantomData,
            },
        );
        let out = match f(burn.tx()) {
            Ok(value) => {
                if let super::apply::WritePhase::Poisoned(source) = &burn.tx().phase {
                    let error = Error::TransactionPoisoned {
                        source: source.clone(),
                    };
                    let WriteTx { view, delta, .. } = burn.disarm();
                    drop(view);
                    return match flush_escaped_fresh_ids(&self.env, &delta) {
                        Ok(()) => Err(error),
                        Err(flush_err) => Err(flush_err),
                    };
                }
                value
            }
            Err(error) => {
                // Non-unwind abort: disarm so Drop does not flush twice,
                // then surface a flush failure (the identity burn is
                // not silent). A sealed CommitRejected is not possible
                // here — commit has not run.
                let WriteTx { view, delta, .. } = burn.disarm();
                drop(view);
                return match flush_escaped_fresh_ids(&self.env, &delta) {
                    Ok(()) => Err(error),
                    Err(flush_err) => Err(flush_err),
                };
            }
        };
        // Disarmed: the delta moves into `commit()`, which owns the flush
        // for every path that reaches it — success flushes the marks
        // inside the commit transaction; reject/infra aborts burn on
        // their own exit.
        let WriteTx { view, delta, .. } = burn.disarm();
        drop(view);
        // The per-relation delete classification, read off the delta's
        // net dispositions before `commit` consumes it: which relations
        // does this commit delete from? (Cancelled delete-then-reinsert
        // pairs net to nothing, so the answer is exact.) The cache hook
        // below needs it — a deleted-from relation's ordinals shifted;
        // a delete-free relation's image survives as an append base.
        let dirty = delta.dirty_relations();
        let floors = delta.inserted_floors();
        let report = commit(delta, &self.env)?;
        txn_span.set_args(1, 0);
        txn_span.end();
        if let crate::storage::commit::CommitReport::Changed { new_generation } = report {
            // The one commit → cache wiring point (`50-storage.md`):
            // entries of relations this commit deleted from — or
            // inserted into below a retained base's boundary (the one
            // id allocator's non-tail arm, R16) — are stale the moment
            // the new generation exists; every other entry is retained
            // as an append base (`ImageCache::advance`).
            self.cache.advance(new_generation, &dirty, &floors);
            // Invalidate any snapshot parked mid-write by a concurrent
            // reader: the next read must begin fresh.
            CommitSeq::advance(&self.commit_seq, Ordering::Release);
            crashpoint!("after-memo-update");
        }
        Ok(out)
    }
}
