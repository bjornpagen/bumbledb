use std::sync::PoisonError;

use super::{Db, ReadInstance, ThreadKey, WriteTx, WriterThreadReset};
use crate::error::{Admission, Committed, ConditionalWrite, Error, Result};
use crate::storage::commit::{commit, flush_escaped_fresh_ids, flush_pending_escaped_fresh_ids};
use crate::storage::env::Environment;

impl Drop for WriterThreadReset<'_> {
    fn drop(&mut self) {
        ThreadKey::store(self.0, None, std::sync::atomic::Ordering::Release);
    }
}

/// Burns the escaped fresh high-water when the write region terminates without
/// reaching `commit` — [`WriterThreadReset`]'s sibling drop guard, and the
/// closure of the one panic gap: `reserve` hands ids to the host before the
/// commit's fate is known, so an `Err`-returning closure and a PANICKING
/// closure alike must burn what escaped (`lean/Bumbledb/Txn/Fresh.lean:
/// never_reissue_observable` — one `Reachable.txn` transition, the fate
/// irrelevant).
struct EscapedIdBurn<'a, S> {
    env: &'a Environment,

    tx: Option<WriteTx<'a, S>>,
}

impl<'a, S> EscapedIdBurn<'a, S> {
    fn arm(env: &'a Environment, tx: WriteTx<'a, S>) -> Self {
        Self { env, tx: Some(tx) }
    }

    fn tx(&mut self) -> &mut WriteTx<'a, S> {
        self.tx.as_mut().expect("armed from construction to disarm")
    }

    fn disarm(mut self) -> WriteTx<'a, S> {
        self.tx.take().expect("armed from construction to disarm")
    }
}

impl<S> Drop for EscapedIdBurn<'_, S> {
    fn drop(&mut self) {
        let Some(tx) = self.tx.take() else {
            return;
        };
        let (view, delta) = tx.into_store();
        // The read view closes before the burn's own write transaction —

        drop(view);

        // begin (`never_reissue_observable`).
        let _ = flush_escaped_fresh_ids(self.env, &delta);
    }
}

enum WriteEnd<R> {
    Committed(R),
    Poisoned(Error),
    Aborted(Error),
}

/// The generation witness, reified: the catalog identity and generation one
/// [`ReadInstance`] observed. Fields are private and the one
/// construction site is [`ReadInstance::witness`], so a witness stays
/// evidence — never an integer a caller could fabricate. A stale
/// witness is exactly what the commit-time compare convicts as
/// [`ConditionalWrite::Moved`].
/// A witness is evidence, and evidence does not wear out. The spent-move
/// ruling is overturned: this value is [`Clone`] and not [`Copy`] (the
/// ```compile_fail
/// fn require_copy<T: Copy>() {}
/// require_copy::<bumbledb::Witness<()>>();
/// ```
#[derive(Clone)]
#[must_use]
pub struct Witness<S> {
    identity: crate::storage::env::CatalogIdentity,
    generation: crate::GenerationId,
    marker: std::marker::PhantomData<fn() -> S>,
}

impl<S> ReadInstance<'_, S> {
    /// # Errors

    pub fn witness(&self) -> Result<Witness<S>> {
        Ok(Witness {
            identity: self.txn().identity().clone(),
            generation: self.txn().generation()?,
            marker: std::marker::PhantomData,
        })
    }
}

impl<S> Db<S> {
    /// delta — LMDB never saw a fact — but fresh ids the closure already
    /// minted burn either way: the `EscapedIdBurn` drop guard flushes the

    /// (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`).

    /// # Errors

    /// # Panics

    pub fn write<R>(
        &self,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<Admission<Committed<R>>> {
        match self.write_witnessed(None, f)? {
            ConditionalWrite::Accepted(committed) => Ok(Admission::Accepted(committed)),
            ConditionalWrite::Rejected(violations) => Ok(Admission::Rejected(violations)),
            ConditionalWrite::Moved { .. } => {
                unreachable!("Db::write has no witness, so Moved is unrepresentable")
            }
        }
    }

    /// The engine ships the outcome, never a loop — retry is host policy.

    /// # Errors

    /// # Panics

    pub fn write_from<R>(
        &self,
        witness: &Witness<S>,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<ConditionalWrite<R>> {
        if !witness.identity.same(self.env.identity()) {
            return Err(Error::ForeignWitness);
        }
        self.write_witnessed(Some(witness.generation), f)
    }

    fn write_witnessed<R>(
        &self,
        witnessed: Option<crate::GenerationId>,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<ConditionalWrite<R>> {
        use std::sync::atomic::Ordering;
        let caller = ThreadKey::mint();
        assert_ne!(
            ThreadKey::load(&self.writer_thread, Ordering::Acquire),
            Some(caller),
            "nested Db::write — re-entrant write transactions are forbidden"
        );

        // LMDB — so the flag is cleared rather than propagated.
        let _writer_lock = self.writer.lock().unwrap_or_else(PoisonError::into_inner);
        ThreadKey::store(&self.writer_thread, Some(caller), Ordering::Release);
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

        // witness's. Mismatch aborts before any page is touched.
        if let Some(witnessed) = witnessed {
            let current = view.generation()?;
            if current != witnessed {
                return Ok(ConditionalWrite::Moved { witnessed, current });
            }
        }
        let mut txn_span = crate::obs::span(crate::obs::names::WRITE_TXN);

        // the guard's drop, exactly once. `reserve` may already have handed

        // (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`).
        // Declared after `_writer_lock` (locals drop in reverse order),

        let mut burn = EscapedIdBurn::arm(
            &self.env,
            WriteTx {
                mutation: super::mutation_core::MutationCore::store(
                    std::sync::Arc::clone(&self.schema),
                    self.schema.as_ref(),
                    view,
                ),
            },
        );
        let end = match f(burn.tx()) {
            Ok(value) => {
                if let Some(source) = burn.tx().poisoned() {
                    WriteEnd::Poisoned(Error::TransactionPoisoned {
                        source: Box::new(source.clone()),
                    })
                } else {
                    WriteEnd::Committed(value)
                }
            }
            Err(error) => WriteEnd::Aborted(error),
        };
        let out = match end {
            WriteEnd::Committed(value) => value,
            WriteEnd::Poisoned(error) | WriteEnd::Aborted(error) => {
                // Non-unwind abort: disarm so Drop does not flush twice,

                let (view, delta) = burn.disarm().into_store();
                drop(view);
                return match flush_escaped_fresh_ids(&self.env, &delta) {
                    Ok(()) => Err(error),
                    Err(flush_err) => Err(flush_err),
                };
            }
        };

        let (view, delta) = burn.disarm().into_store();
        drop(view);

        let dirty = delta.dirty_relations();
        let floors = delta.inserted_floors();
        let report = match commit(delta, &self.env)? {
            Admission::Rejected(violations) => {
                return Ok(ConditionalWrite::Rejected(violations));
            }
            Admission::Accepted(report) => report,
        };
        txn_span.set_flag(true);
        txn_span.end();
        if let crate::storage::commit::CommitReport::Changed { new_generation } = report {
            self.cache.advance(new_generation, &dirty, &floors);

            self.generation
                .store(new_generation.storage_word(), Ordering::Release);
        }
        Ok(ConditionalWrite::Accepted(Committed {
            value: out,
            generation: report.generation(),
        }))
    }
}
