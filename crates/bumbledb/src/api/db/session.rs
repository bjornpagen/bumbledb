//! Narrow native-host integration: the writer guard outlives rejected
//! candidates, empty receipt transactions, and a sealed publication attempt.
//! This transitional facade uses the existing admission engine; its legacy
//! planning/index allocations still require the M2/M3 budgeted rewrite.
use std::sync::{Arc, MutexGuard, PoisonError, TryLockError, atomic::Ordering};
use std::time::Duration;

use super::{Db, ReadInstance, ThreadKey, WriteTx, WriterThreadReset};
use crate::changes::ChangeKind;
use crate::storage::commit::{ApplicationChanges, CommitReport, PreparedCommit, SealedCommit};
use crate::storage::env::host::{HostChanges, HostSealError};
use crate::{
    Admission, ChangeError, ChangeSet, Error, GenerationId, RelationId, WorkContext, WorkError,
};

#[derive(Debug)]
pub enum IntegrationError {
    Core(Error),
    Changes(ChangeError),
    Host(HostSealError),
    Work(WorkError),
    ForeignSchema,
    ReentrantWriter,
}
impl From<Error> for IntegrationError {
    fn from(error: Error) -> Self {
        Self::Core(error)
    }
}
impl From<ChangeError> for IntegrationError {
    fn from(error: ChangeError) -> Self {
        Self::Changes(error)
    }
}
impl From<HostSealError> for IntegrationError {
    fn from(error: HostSealError) -> Self {
        Self::Host(error)
    }
}
impl From<WorkError> for IntegrationError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}
impl std::fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native write integration: {self:?}")
    }
}
impl std::error::Error for IntegrationError {}

pub struct WriterSession<'db, S> {
    db: &'db Db<S>,
    work: WorkContext,
    // Field order is Drop order: clear the owner before releasing the mutex.
    _owner: WriterThreadReset<'db>,
    _guard: MutexGuard<'db, ()>,
}

pub struct PreparedWrite<'owner, 'db, S> {
    session: &'owner mut WriterSession<'db, S>,
    prepared: PreparedCommit<'db>,
    dirty: Vec<RelationId>,
    floors: Vec<(RelationId, u64)>,
}

pub struct SealedWrite<'owner, 'db, S> {
    session: &'owner mut WriterSession<'db, S>,
    sealed: SealedCommit<'db>,
    changes: ApplicationChanges,
    dirty: Vec<RelationId>,
    floors: Vec<(RelationId, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreCommit {
    pub generation: GenerationId,
    pub application: ApplicationChanges,
    pub changed: bool,
}

impl<S> Db<S> {
    /// Native integration only, not a general key/value or callback API.
    /// # Errors
    /// Refuses reentrancy, cancellation/deadline or storage failure. Waiting
    /// for a writer consumes the same deadline as preparing and sealing it.
    #[doc(hidden)]
    pub fn integration_writer(
        &self,
        work: &WorkContext,
    ) -> Result<WriterSession<'_, S>, IntegrationError> {
        let caller = ThreadKey::mint();
        if ThreadKey::load(&self.writer_thread, Ordering::Acquire) == Some(caller) {
            return Err(IntegrationError::ReentrantWriter);
        }
        let guard = loop {
            work.checkpoint()?;
            match self.writer.try_lock() {
                Ok(guard) => break guard,
                Err(TryLockError::Poisoned(error)) => break error.into_inner(),
                Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(1)),
            }
        };
        work.checkpoint()?;
        ThreadKey::store(&self.writer_thread, Some(caller), Ordering::Release);
        let owner = WriterThreadReset(&self.writer_thread);
        drop(
            self.read_cache
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take(),
        );
        Ok(WriterSession {
            db: self,
            work: work.clone(),
            _owner: owner,
            _guard: guard,
        })
    }
}

impl<'db, S> WriterSession<'db, S> {
    /// # Errors
    /// Reads the parent generation while retaining exclusive writer ownership.
    pub fn generation(&self) -> Result<GenerationId, IntegrationError> {
        Ok(self.db.env.read_txn()?.generation()?)
    }

    /// # Errors
    /// Refuses a foreign schema, malformed/storage failure or stopped work.
    /// A domain rejection drops its candidate, but this session stays owned.
    pub fn prepare<'owner>(
        &'owner mut self,
        changes: &ChangeSet,
    ) -> Result<Admission<PreparedWrite<'owner, 'db, S>>, IntegrationError> {
        self.work.checkpoint()?;
        if changes.schema() != crate::schema::fingerprint::fingerprint(&self.db.schema) {
            return Err(IntegrationError::ForeignSchema);
        }
        let view = self.db.env.read_txn()?;
        let mut transaction: WriteTx<'_, S> = WriteTx {
            mutation: super::mutation_core::MutationCore::store(
                Arc::clone(&self.db.schema),
                self.db.schema.as_ref(),
                view,
            ),
        };
        // Delete all old tuples before adding replacements. No caller callback
        // or iterator runs in this interval; the command is already sealed.
        for kind in [ChangeKind::Remove, ChangeKind::Add] {
            for record in changes.records().filter(|record| record.kind == kind) {
                self.work.step(1)?;
                let row = crate::canonical::decode(
                    self.db.schema.relation(record.relation).fields(),
                    record.row,
                    &self.work,
                )
                .map_err(ChangeError::from)?;
                match kind {
                    ChangeKind::Remove => {
                        transaction.delete_dyn(record.relation, [&row.values])?;
                    }
                    ChangeKind::Add => {
                        transaction.insert_dyn(record.relation, [&row.values])?;
                    }
                }
            }
        }
        let (view, delta) = transaction.into_store();
        drop(view);
        self.work.checkpoint()?;
        let dirty = delta.dirty_relations();
        let floors = delta.inserted_floors();
        let prepared = match crate::storage::commit::prepare(&delta, &self.db.env)? {
            Admission::Rejected(violations) => return Ok(Admission::Rejected(violations)),
            Admission::Accepted(prepared) => prepared,
        };
        self.work.checkpoint()?;
        Ok(Admission::Accepted(PreparedWrite {
            session: self,
            prepared,
            dirty,
            floors,
        }))
    }
}

impl<'owner, 'db, S> PreparedWrite<'owner, 'db, S> {
    #[must_use]
    pub fn application_changes(&self) -> ApplicationChanges {
        self.prepared.application_changes()
    }
    #[must_use]
    pub fn proposed_generation(&self) -> GenerationId {
        self.prepared.report().generation()
    }

    /// # Errors
    /// Any failure aborts the private facts and all staged host records. The
    /// returned capability exposes commit/drop only, never fact amendment.
    pub fn seal(
        self,
        host: HostChanges<'_>,
    ) -> Result<SealedWrite<'owner, 'db, S>, IntegrationError> {
        let changes = self.prepared.application_changes();
        let sealed = self.prepared.seal(host, &self.session.work)?;
        Ok(SealedWrite {
            session: self.session,
            sealed,
            changes,
            dirty: self.dirty,
            floors: self.floors,
        })
    }
}

impl<S> SealedWrite<'_, '_, S> {
    /// # Errors
    /// Reports local durability failure without making claims about any remote
    /// publication. A hosted caller must preserve its already-known receipt.
    pub fn commit(self) -> Result<CoreCommit, IntegrationError> {
        let report = self.sealed.commit()?;
        if let CommitReport::Changed { new_generation } = report {
            self.session
                .db
                .cache
                .advance(new_generation, &self.dirty, &self.floors);
            self.session
                .db
                .generation
                .store(new_generation.storage_word(), Ordering::Release);
        }
        Ok(CoreCommit {
            generation: report.generation(),
            application: self.changes,
            changed: matches!(report, CommitReport::Changed { .. }),
        })
    }
}

impl<S> ReadInstance<'_, S> {
    /// # Errors
    /// The bytes and application rows borrow the same committed read snapshot.
    #[doc(hidden)]
    pub fn integration_host_record(&self, key: &[u8]) -> Result<Option<&[u8]>, HostSealError> {
        self.txn().host_record(key)
    }
    /// # Errors
    /// Returns opaque host attachment bytes from this exact read snapshot.
    #[doc(hidden)]
    pub fn integration_host_attachment(&self) -> crate::Result<Option<&[u8]>> {
        self.txn().host_attachment()
    }
}

#[cfg(test)]
mod tests;
