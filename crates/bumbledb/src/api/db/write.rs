//! [`Db::write`] / [`Db::write_from`]: the embedded durable write path over
//! the successor candidate protocol.
//!
//! Flow: acquire the store's one writer capability (exclusive, reentrancy
//! refused as a typed error) → witness check against the true parent →
//! open the parent snapshot → run the closure over an in-memory net delta
//! → drop the parent reader → prepare the sealed delta as a private
//! candidate, judged as a complete final state by the production judge →
//! seal (no host adjunct on the embedded path) → one durable LMDB commit
//! for facts and generation together. An abort — closure error, judge
//! rejection, or panic — never wrote a fact: the pending delta is plain
//! memory and the candidate transaction drops whole.

use super::tx::change_set_of_pending;
use super::{Db, ReadInstance, WriteTx, embedded_work};
use crate::error::{Committed, ConditionalWrite, Error, Result};
use crate::storage::GenerationId;
use crate::storage::store::{
    AttachmentChange, EnvironmentId, HostChanges, Prepared, SchemaJudge, UnindexedRows,
};

/// The generation witness, reified: the environment identity and
/// generation one [`ReadInstance`] observed. Fields are private and the
/// one construction site is [`ReadInstance::witness`], so a witness stays
/// evidence — never an integer a caller could fabricate. A stale witness
/// is exactly what the commit-time compare convicts as
/// [`ConditionalWrite::Moved`]. Evidence does not wear out: `Clone`, not
/// `Copy` (cloning is a decision at the call site).
/// ```compile_fail
/// fn require_copy<T: Copy>() {}
/// require_copy::<bumbledb::Witness<()>>();
/// ```
#[derive(Clone)]
#[must_use]
pub struct Witness<S> {
    environment: EnvironmentId,
    generation: GenerationId,
    marker: std::marker::PhantomData<fn() -> S>,
}

impl<S> ReadInstance<'_, S> {
    /// # Errors
    /// None today; kept fallible with the lease shape.
    pub fn witness(&self) -> Result<Witness<S>> {
        Ok(Witness {
            environment: self.snapshot.identity().environment,
            generation: self.snapshot.generation(),
            marker: std::marker::PhantomData,
        })
    }
}

impl<S> Db<S> {
    /// One durable write: in-memory set arithmetic inside the closure, then
    /// complete final-state judgment, then one LMDB commit. A rejection is
    /// an [`crate::Admission::Rejected`] carrying the complete violated
    /// statement set; the closure's minted values are gone with the delta.
    /// # Errors
    /// Storage failure, exhausted work, reentrant write, or the closure's
    /// own error (which aborts: LMDB never saw a fact).
    pub fn write<R>(
        &self,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<crate::Admission<Committed<R>>> {
        match self.write_witnessed(None, f)? {
            ConditionalWrite::Accepted(committed) => Ok(crate::Admission::Accepted(committed)),
            ConditionalWrite::Rejected(violations) => Ok(crate::Admission::Rejected(violations)),
            ConditionalWrite::Moved { .. } => {
                unreachable!("Db::write has no witness, so Moved is unrepresentable")
            }
        }
    }

    /// Conditional write: the engine ships the outcome, never a loop —
    /// retry is host policy. A moved generation is an answer, not an error.
    /// # Errors
    /// `ForeignWitness` for a witness from another environment; otherwise
    /// as [`Db::write`].
    pub fn write_from<R>(
        &self,
        witness: &Witness<S>,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<ConditionalWrite<R>> {
        if witness.environment != self.store.environment_id() {
            return Err(Error::ForeignWitness);
        }
        self.write_witnessed(Some(witness.generation), f)
    }

    fn write_witnessed<R>(
        &self,
        witnessed: Option<GenerationId>,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<ConditionalWrite<R>> {
        let work = embedded_work()?;
        // The exclusive writer first: nothing can commit between the
        // witness check / parent snapshot and this candidate's commit.
        let mut owner = self.store.writer(&work).map_err(Error::from_store)?;
        if let Some(witnessed) = witnessed {
            let current = owner.parent_generation().map_err(Error::from_store)?;
            if current != witnessed {
                return Ok(ConditionalWrite::Moved { witnessed, current });
            }
        }
        let mut txn_span = crate::obs::span(crate::obs::names::WRITE_TXN);
        let parent = self.store.snapshot(&work).map_err(Error::from_store)?;
        let mut tx = WriteTx::new(&self.schema, self.closed.as_ref(), &parent, &work);
        let value = f(&mut tx)?;
        if let Some(source) = tx.poisoned() {
            return Err(Error::TransactionPoisoned {
                source: Box::new(source.clone()),
            });
        }
        let pending = tx.into_pending();
        // Release the parent reader before the candidate's map-full →
        // grow → reapply loop: a held snapshot would block growth.
        drop(parent);
        let changes = change_set_of_pending(self.schema.as_ref(), &pending, &work)?;
        let judge = SchemaJudge::new(self.schema.as_ref());
        match owner
            .prepare(&changes, &UnindexedRows, &judge)
            .map_err(Error::from_store)?
        {
            Prepared::Rejected(judged) => Ok(ConditionalWrite::Rejected(
                super::violations::violations_from_judged(self.schema.as_ref(), judged, &work)?,
            )),
            Prepared::Admitted(prepared) => {
                let sealed = prepared
                    .seal(HostChanges {
                        records: &[],
                        attachment: AttachmentChange::Keep,
                    })
                    .map_err(Error::from_store)?;
                let commit = sealed.commit().map_err(Error::from_store)?;
                txn_span.set_flag(true);
                txn_span.end();
                Ok(ConditionalWrite::Accepted(Committed {
                    value,
                    generation: commit.generation,
                }))
            }
        }
    }
}
