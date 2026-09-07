//! [`Db::write`] / [`Db::write_from`]: the embedded durable write path over
//! the successor candidate protocol.
//!
//! Flow: acquire the store's one writer capability (exclusive, reentrancy
//! refused as a typed error) → witness check against the true parent →
//! open the parent snapshot → run the closure over an in-memory net delta
//! → drop the parent reader → prepare the sealed delta as a private
//! candidate, judged incrementally under a [`crate::schema::judge::LawfulParent`] →
//! seal (no host adjunct on the embedded path) → one durable LMDB commit
//! for facts and generation together. An abort — closure error, judge
//! rejection, or panic — never wrote a fact: the pending delta is plain
//! memory and the candidate transaction drops whole.

use super::{Db, OwnedRead, ReadFrame, WriteTx};
use crate::error::{Committed, ConditionalWrite, Error, Result};
use crate::schema::judge::LawfulParent;
use crate::storage::GenerationId;
use crate::storage::store::{
    AttachmentChange, EnvironmentId, HostChanges, Prepared, SchemaJudge, UnindexedRows,
};
use crate::work::WorkContext;

fn admitted_parent() -> LawfulParent {
    LawfulParent::established()
}

/// The generation witness, reified: the environment identity and
/// generation one [`OwnedRead`] observed. Fields are private and the
/// construction sites are [`OwnedRead::witness`] / [`ReadFrame::witness`],
/// so a witness stays evidence — never an integer a caller could fabricate.
/// A stale witness is exactly what the commit-time compare convicts as
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

/// Expected parent for [`Db::apply`].
pub enum ApplyExpected<S> {
    Any,
    Exact(Witness<S>),
}

/// Typed public write outcome. Distinct from a JS callback.
///
/// ```compile_fail
/// fn require_rejected(outcome: bumbledb::ApplyOutcome) {
///     let _ = match outcome {
///         bumbledb::ApplyOutcome::Rejected(_) => {}
///         _ => {}
///     };
/// }
/// ```
pub enum ApplyOutcome {
    Accepted {
        generation: GenerationId,
    },
    NoChange {
        generation: GenerationId,
    },
    InvariantRejected {
        violations: crate::error::Violations,
    },
    Moved {
        witnessed: GenerationId,
        current: GenerationId,
    },
}

impl<S> OwnedRead<S> {
    pub fn witness(&self) -> Witness<S> {
        snapshot_witness(&self.snapshot)
    }
}

impl<S> ReadFrame<'_, S> {
    /// Capture the pinned generation for a later conditional write.
    /// # Errors
    /// No current failure; matches the fallible read-frame operations.
    pub fn witness(&self) -> Result<Witness<S>> {
        Ok(snapshot_witness(self.snapshot))
    }
}

fn snapshot_witness<S>(snapshot: &crate::storage::store::OwnedSnapshot) -> Witness<S> {
    Witness {
        environment: snapshot.identity().environment,
        generation: snapshot.generation(),
        marker: std::marker::PhantomData,
    }
}

impl<S> Db<S> {
    /// One durable write under an explicit operation allowance: in-memory
    /// set arithmetic inside the closure, then incremental judgment under
    /// the admitted lawful parent, then one LMDB commit.
    /// # Errors
    /// Storage failure, exhausted work, reentrant write, or the closure's
    /// own error (which aborts: LMDB never saw a fact).
    pub fn write<R>(
        &self,
        work: WorkContext,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<crate::Admission<Committed<R>>> {
        match self.write_witnessed(work, None, f)? {
            ConditionalWrite::Accepted(committed) => Ok(crate::Admission::Accepted(committed)),
            ConditionalWrite::Rejected(violations) => Ok(crate::Admission::Rejected(violations)),
            ConditionalWrite::Moved { .. } => {
                unreachable!("Db::write has no witness, so Moved is unrepresentable")
            }
        }
    }

    /// Conditional write under an explicit operation allowance: the engine
    /// ships the outcome, never a loop — retry is host policy.
    /// # Errors
    /// `ForeignWitness` for a witness from another environment; otherwise
    /// as [`Db::write`].
    pub fn write_from<R>(
        &self,
        work: WorkContext,
        witness: &Witness<S>,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<ConditionalWrite<R>> {
        if witness.environment != self.store.environment_id() {
            return Err(Error::ForeignWitness);
        }
        self.write_witnessed(work, Some(witness.generation), f)
    }

    /// Apply a sealed change set under an explicit work budget.
    ///
    /// # Errors
    /// `ForeignWitness` for a witness from another environment; otherwise
    /// storage failure, exhausted work, or reentrant write.
    pub fn apply(
        &self,
        changes: &crate::ChangeSet,
        expected: ApplyExpected<S>,
        work: &WorkContext,
    ) -> Result<ApplyOutcome> {
        let witnessed = match expected {
            ApplyExpected::Any => None,
            ApplyExpected::Exact(witness) => {
                if witness.environment != self.store.environment_id() {
                    return Err(Error::ForeignWitness);
                }
                Some(witness.generation)
            }
        };
        let mut owner = self.store.writer(work).map_err(Error::from_store)?;
        if let Some(witnessed) = witnessed {
            let current = owner.parent_generation().map_err(Error::from_store)?;
            if current != witnessed {
                return Ok(ApplyOutcome::Moved { witnessed, current });
            }
        }
        let judge = SchemaJudge::new(self.schema.as_ref());
        match owner
            .prepare_incremental(admitted_parent(), changes, &UnindexedRows, &judge)
            .map_err(Error::from_store)?
        {
            Prepared::Rejected(judged) => Ok(ApplyOutcome::InvariantRejected {
                violations: super::violations::violations_from_judged(
                    self.schema.as_ref(),
                    judged,
                    work,
                )?,
            }),
            Prepared::Admitted(prepared) => {
                let sealed = prepared
                    .seal(HostChanges {
                        records: &[],
                        attachment: AttachmentChange::Keep,
                    })
                    .map_err(Error::from_store)?;
                let commit = sealed.commit().map_err(Error::from_store)?;
                if commit.changed {
                    Ok(ApplyOutcome::Accepted {
                        generation: commit.generation,
                    })
                } else {
                    Ok(ApplyOutcome::NoChange {
                        generation: commit.generation,
                    })
                }
            }
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Database operations accept owned call-scoped work and key values consistently"
    )]
    fn write_witnessed<R>(
        &self,
        work: WorkContext,
        witnessed: Option<GenerationId>,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<ConditionalWrite<R>> {
        let mut owner = self.store.writer(&work).map_err(Error::from_store)?;
        if let Some(witnessed) = witnessed {
            let current = owner.parent_generation().map_err(Error::from_store)?;
            if current != witnessed {
                return Ok(ConditionalWrite::Moved { witnessed, current });
            }
        }
        let parent = self.store.snapshot(&work).map_err(Error::from_store)?;
        let mut tx = WriteTx::new(&self.schema, self.closed.as_ref(), &parent, &work);
        let value = f(&mut tx)?;
        if let Some(source) = tx.poisoned() {
            return Err(Error::TransactionPoisoned {
                source: Box::new(source.clone()),
            });
        }
        let pending = tx.into_pending();
        drop(parent);
        let changes = pending.seal(self.schema.as_ref(), &work)?;
        let judge = SchemaJudge::new(self.schema.as_ref());
        match owner
            .prepare_incremental(admitted_parent(), &changes, &UnindexedRows, &judge)
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
                Ok(ConditionalWrite::Accepted(Committed {
                    value,
                    generation: commit.generation,
                }))
            }
        }
    }
}
