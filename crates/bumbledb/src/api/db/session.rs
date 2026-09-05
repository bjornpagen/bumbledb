//! Narrow native-host integration over the successor candidate protocol:
//! the writer session outlives rejected candidates, empty receipt
//! transactions, and a sealed publication attempt. This is the log/native
//! bridge's seam (consumed as `bumbledb::integration`); it is not a general
//! key/value or callback API.
//!
//! Everything here is a thin, typed facade over the store's own
//! capabilities (`storage::store::{WriteOwner, PreparedWrite, SealedWrite}`):
//! prepare applies the sealed `ChangeSet` as a private candidate and judges
//! it incrementally under a lawful parent; seal writes only
//! opaque host records/attachment into the same transaction; commit is the
//! one durability point for facts + generation + host rows + attachment.
//! A rejected or aborted candidate retains the exclusive session, so the
//! log can prepare its receipt-only transaction against the unchanged
//! parent with no gap for another local writer.

use std::marker::PhantomData;

use super::{Db, ReadFrame};
use crate::storage::GenerationId;
use crate::schema::judge::LawfulParent;
use crate::storage::store::{
    self, HostChanges, HostSealError, SchemaJudge, StoreError, UnindexedRows,
};
use crate::{Admission, ChangeError, ChangeSet, Error, WorkContext, WorkError};

/// Net application-fact changes of one candidate (C04's `AppliedChanges`,
/// exported under the integration seam's historical name).
pub type ApplicationChanges = store::AppliedChanges;

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

fn integration_error(error: StoreError) -> IntegrationError {
    match error {
        StoreError::Work(work) => IntegrationError::Work(work),
        StoreError::Changes(changes) => IntegrationError::Changes(changes),
        StoreError::ForeignSchema => IntegrationError::ForeignSchema,
        StoreError::ReentrantWriter => IntegrationError::ReentrantWriter,
        StoreError::HostKey(fault) => {
            IntegrationError::Host(crate::storage::store::host::seal_error_of(fault))
        }
        StoreError::GenerationExhausted => {
            IntegrationError::Host(HostSealError::GenerationExhausted)
        }
        other => IntegrationError::Core(Error::from_store(other)),
    }
}

pub struct WriterSession<'db, S> {
    db: &'db Db<S>,
    owner: store::WriteOwner<'db>,
    work: WorkContext,
}

pub struct PreparedWrite<'owner, 'db, S> {
    inner: store::PreparedWrite<'owner, 'db>,
    marker: PhantomData<fn() -> S>,
}

pub struct SealedWrite<'owner, 'db, S> {
    inner: store::SealedWrite<'owner, 'db>,
    marker: PhantomData<fn() -> S>,
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
        let owner = self.store.writer(work).map_err(integration_error)?;
        Ok(WriterSession {
            db: self,
            owner,
            work: work.clone(),
        })
    }
}

impl<'db, S> WriterSession<'db, S> {
    /// The committed parent generation, read while exclusivity is held.
    /// # Errors
    /// Storage failure or stopped work.
    pub fn generation(&self) -> Result<GenerationId, IntegrationError> {
        self.owner.parent_generation().map_err(integration_error)
    }

    /// Prepare and judge one sealed canonical delta as a private candidate.
    /// # Errors
    /// Refuses a foreign schema, malformed/storage failure or stopped work.
    /// A domain rejection drops its candidate, but this session stays owned.
    pub fn prepare<'owner>(
        &'owner mut self,
        changes: &ChangeSet,
    ) -> Result<Admission<PreparedWrite<'owner, 'db, S>>, IntegrationError> {
        self.work.checkpoint()?;
        let schema = self.db.schema_arc();
        let judge = SchemaJudge::new(schema.as_ref());
        match self
            .owner
            .prepare_incremental(LawfulParent::established(), changes, &UnindexedRows, &judge)
            .map_err(integration_error)?
        {
            store::Prepared::Rejected(judged) => {
                let violations =
                    super::violations::violations_from_judged(schema.as_ref(), judged, &self.work)
                        .map_err(IntegrationError::Core)?;
                Ok(Admission::Rejected(violations))
            }
            store::Prepared::Admitted(inner) => Ok(Admission::Accepted(PreparedWrite {
                inner,
                marker: PhantomData,
            })),
        }
    }
}

impl<'owner, 'db, S> PreparedWrite<'owner, 'db, S> {
    #[must_use]
    pub fn application_changes(&self) -> ApplicationChanges {
        self.inner.application_changes()
    }

    #[must_use]
    pub fn proposed_generation(&self) -> GenerationId {
        self.inner.proposed_generation()
    }

    /// Seal opaque host records/attachment into the same transaction. Only
    /// host bytes can change — never judged application facts. Any failure
    /// aborts the private facts and all staged host records; the returned
    /// capability exposes commit/drop only, never fact amendment.
    /// # Errors
    /// Host-key grammar violations, growth exhaustion, storage failure or
    /// stopped work.
    pub fn seal(
        self,
        host: HostChanges<'_>,
    ) -> Result<SealedWrite<'owner, 'db, S>, IntegrationError> {
        let inner = self.inner.seal(host).map_err(integration_error)?;
        Ok(SealedWrite {
            inner,
            marker: PhantomData,
        })
    }

    /// Drop the candidate; committed state untouched, session retained.
    pub fn abort(self) {
        self.inner.abort();
    }
}

impl<S> SealedWrite<'_, '_, S> {
    /// # Errors
    /// Reports local durability failure without making claims about any
    /// remote publication. A hosted caller must preserve its already-known
    /// receipt.
    pub fn commit(self) -> Result<CoreCommit, IntegrationError> {
        let commit = self.inner.commit().map_err(integration_error)?;
        Ok(CoreCommit {
            generation: commit.generation,
            application: commit.application,
            changed: commit.changed,
        })
    }

    /// Drop the sealed candidate whole; nothing was dispatched.
    pub fn abort(self) {
        self.inner.abort();
    }
}

impl<S> ReadFrame<'_, S> {
    /// # Errors
    /// The bytes and application rows borrow the same committed snapshot.
    #[doc(hidden)]
    pub fn integration_host_record(&self, key: &[u8]) -> Result<Option<&[u8]>, HostSealError> {
        self.owner.snapshot.host_record(key).map_err(|error| match error {
            StoreError::HostKey(fault) => crate::storage::store::host::seal_error_of(fault),
            other => HostSealError::Storage(Error::from_store(other)),
        })
    }

    /// Visit every committed host record whose key starts with `prefix`, in
    /// ascending key order, within this read's one committed transaction
    /// (the P02R host enumeration seam). Key and value bytes borrow the
    /// snapshot only for the duration of each visit — copy before
    /// returning; charged against this lease's work allowance per record.
    /// # Errors
    /// Host-key grammar or storage failure, stopped work, or the visitor's
    /// own refusal.
    #[doc(hidden)]
    #[expect(
        clippy::type_complexity,
        reason = "the P02R-requested visitor signature, spelled exactly as \
                  implementation/packets/P05.md records it"
    )]
    pub fn integration_host_scan(
        &self,
        prefix: &[u8],
        visit: &mut dyn FnMut(&[u8], &[u8]) -> Result<(), HostSealError>,
    ) -> Result<(), HostSealError> {
        self.owner.snapshot.host_scan(prefix, self.work, visit)
    }

    /// # Errors
    /// Returns opaque host attachment bytes from this exact read snapshot.
    #[doc(hidden)]
    pub fn integration_host_attachment(&self) -> crate::Result<Option<&[u8]>> {
        self.owner.snapshot.attachment().map_err(Error::from_store)
    }
}
