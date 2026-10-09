//! The write path: [`Db::write`], [`Db::write_from`], [`Db::apply`] and
//! [`Db::apply_from`] share one commit path. The writer is taken first, a witness
//! is checked against the true parent, and the change is applied to a private
//! candidate, judged incrementally and committed once.

use super::{Db, OwnedRead, ReadFrame, WriteTx};
use crate::error::{Error, Result, Violations};
use crate::storage::GenerationId;
use crate::storage::store::HostChanges;
use crate::storage::store::candidate::{Candidate, WriteOwner};
use crate::storage::store::format::EnvironmentId;
use crate::work::WorkContext;

/// The environment and generation one [`OwnedRead`] observed. Only
/// [`OwnedRead::witness`] and [`ReadFrame::witness`] mint one, so a witness
/// stays evidence. `Clone`, not `Copy`.
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

/// An admitted write: the closure's value and the generation after it.
/// `changed` is false when the change set matched the committed state; the
/// generation moved exactly when it is true.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed<R> {
    pub value: R,
    pub generation: GenerationId,
    pub changed: bool,
}

/// The outcome of one durable write.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteOutcome<R> {
    Committed(Committed<R>),
    /// The proposed state violates the theory; nothing was written.
    Rejected(Violations),
    /// The witnessed generation is not the current one; nothing was written.
    Moved {
        witnessed: GenerationId,
        current: GenerationId,
    },
}

impl<R> WriteOutcome<R> {
    /// # Panics
    /// If the write was rejected or moved.
    #[track_caller]
    pub fn unwrap(self) -> Committed<R> {
        self.expect("write outcome")
    }

    /// # Panics
    /// If the write was rejected or moved.
    #[track_caller]
    pub fn expect(self, msg: &str) -> Committed<R> {
        match self {
            Self::Committed(committed) => committed,
            Self::Rejected(violations) => panic!("{msg}: rejected: {violations}"),
            Self::Moved { witnessed, current } => {
                panic!("{msg}: moved ({witnessed} -> {current})")
            }
        }
    }
}

impl<S> OwnedRead<S> {
    pub fn witness(&self) -> Witness<S> {
        Witness {
            environment: self.snapshot.identity().environment,
            generation: self.snapshot.generation(),
            marker: std::marker::PhantomData,
        }
    }
}

impl<S> ReadFrame<'_, S> {
    pub fn witness(&self) -> Witness<S> {
        Witness {
            environment: self.snapshot.identity().environment,
            generation: self.snapshot.generation(),
            marker: std::marker::PhantomData,
        }
    }
}

impl<S> Db<S> {
    /// One durable write: set arithmetic inside the closure, incremental
    /// judgment, one commit.
    /// # Errors
    /// Storage failure, cancellation, a reentrant write, or the closure's
    /// own error (which aborts).
    pub fn write<R>(
        &self,
        work: WorkContext,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<WriteOutcome<R>> {
        let mut owner = self.store.writer(&work)?;
        self.write_owned(&mut owner, work, f)
    }

    /// A write conditional on `witness` still naming the current generation.
    /// The outcome is reported, never retried.
    /// # Errors
    /// `ForeignWitness` for a witness of another environment; otherwise as
    /// [`Db::write`].
    pub fn write_from<R>(
        &self,
        work: WorkContext,
        witness: &Witness<S>,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<WriteOutcome<R>> {
        let mut owner = self.store.writer(&work)?;
        if let Some(moved) = self.moved(&owner, witness)? {
            return Ok(moved);
        }
        self.write_owned(&mut owner, work, f)
    }

    /// Commit a sealed change set.
    /// # Errors
    /// A foreign-schema change set; otherwise as [`Db::write`].
    pub fn apply(
        &self,
        changes: &crate::ChangeSet,
        work: &WorkContext,
    ) -> Result<WriteOutcome<()>> {
        let mut owner = self.store.writer(work)?;
        self.commit(&mut owner, changes, (), work)
    }

    /// Commit a sealed change set conditional on `witness`.
    /// # Errors
    /// As [`Db::write_from`] and [`Db::apply`].
    pub fn apply_from(
        &self,
        changes: &crate::ChangeSet,
        witness: &Witness<S>,
        work: &WorkContext,
    ) -> Result<WriteOutcome<()>> {
        let mut owner = self.store.writer(work)?;
        if let Some(moved) = self.moved(&owner, witness)? {
            return Ok(moved);
        }
        self.commit(&mut owner, changes, (), work)
    }

    fn moved<R>(
        &self,
        owner: &WriteOwner<'_>,
        witness: &Witness<S>,
    ) -> Result<Option<WriteOutcome<R>>> {
        if witness.environment != self.store.identity().environment {
            return Err(Error::ForeignWitness);
        }
        let current = owner.parent_generation()?;
        Ok(
            (current != witness.generation).then_some(WriteOutcome::Moved {
                witnessed: witness.generation,
                current,
            }),
        )
    }

    fn write_owned<R>(
        &self,
        owner: &mut WriteOwner<'_>,
        work: WorkContext,
        f: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
    ) -> Result<WriteOutcome<R>> {
        let parent = self.store.snapshot(&work)?;
        let mut tx = WriteTx::new(&self.schema, &parent, &work);
        let value = f(&mut tx)?;
        if let Some(source) = tx.poisoned() {
            return Err(Error::TransactionPoisoned {
                source: Box::new(source.clone()),
            });
        }
        let pending = tx.into_pending();
        drop(parent);
        let changes = pending.seal(self.schema.as_ref(), &work)?;
        self.commit(owner, &changes, value, &work)
    }

    /// The one private commit path.
    fn commit<R>(
        &self,
        owner: &mut WriteOwner<'_>,
        changes: &crate::ChangeSet,
        value: R,
        work: &WorkContext,
    ) -> Result<WriteOutcome<R>> {
        match owner.prepare_judged(self.schema.as_ref(), changes)? {
            Candidate::Rejected(judged) => Ok(WriteOutcome::Rejected(
                super::violations::violations_from_judged(self.schema.as_ref(), judged, work)?,
            )),
            Candidate::Admitted(prepared) => {
                let commit = prepared
                    .seal(HostChanges::NONE)
                    .and_then(crate::storage::store::candidate::SealedWrite::commit)?;
                Ok(WriteOutcome::Committed(Committed {
                    value,
                    generation: commit.generation,
                    changed: commit.changed,
                }))
            }
        }
    }
}
