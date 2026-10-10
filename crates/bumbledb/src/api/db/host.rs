//! The log host's view of a database: identity, batch decisions, unjudged
//! application of decided change sets sealed with host records and the
//! head, host reads, content digests, images, and migration populations.
//!
//! Nothing here interprets host bytes; the log owns their meaning.

use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

use bumbledb_theory::schema::RelationId;

use super::{Db, ReadFrame};
use crate::error::{Admission, Error, FactShapeError, Mismatch, Result, Violations};
use crate::schema::judge::Judgment;
use crate::schema::{Schema, Theory, ValidateDescriptor as _};
use crate::storage::GenerationId;
use crate::storage::store::candidate::{PreparedWrite, SealedWrite, WriteOwner};
use crate::storage::store::staging::Staging;
use crate::storage::store::{self, Store};
use crate::{ChangeSet, WorkContext};

pub use crate::digest::Digest;
pub use crate::storage::store::{
    Applied, Commit, DatabaseId, Head, HostChanges, HostRecord, VerifyCorruption,
};
pub use crate::verify_store::StoreReport;

/// The on-disk layout of stores and images this build reads and writes.
pub const LAYOUT: u32 = store::format::LAYOUT;

/// The longest host record key.
pub const MAX_KEY: usize = store::keys::HOST_KEY_MAX;

/// A visitor of host records: key, then value.
pub type HostVisitor<'v> = dyn FnMut(&[u8], &[u8]) -> Result<()> + 'v;

/// One change set's verdict in a batch decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judged {
    Accepted(Applied),
    Rejected(Violations),
}

/// The exclusive writer, held across a decision and its application.
pub struct WriterSession<'db, S> {
    db: &'db Db<S>,
    owner: WriteOwner<'db>,
    work: WorkContext,
}

/// Applied change sets in an open transaction: seal or abort.
pub struct Prepared<'session, 'db, S> {
    inner: PreparedWrite<'session, 'db>,
    marker: PhantomData<fn() -> S>,
}

/// Sealed: commit or abort.
pub struct Sealed<'session, 'db, S> {
    inner: SealedWrite<'session, 'db>,
    marker: PhantomData<fn() -> S>,
}

impl<S> Db<S> {
    /// The database identity.
    #[must_use]
    pub fn database_id(&self) -> DatabaseId {
        self.store.identity().database
    }

    /// Take the single writer.
    /// # Errors
    /// Reentrancy from the owning thread, a closing store, or cancellation.
    pub fn host_writer(&self, work: &WorkContext) -> Result<WriterSession<'_, S>> {
        Ok(WriterSession {
            db: self,
            owner: self.store.writer(work)?,
            work: work.clone(),
        })
    }
}

impl<S: Theory> Db<S> {
    /// Create a database whose identity is `database`, the log's Genesis id.
    /// # Errors
    /// As [`Db::create`].
    pub fn create_identified(
        path: &Path,
        schema: S,
        database: DatabaseId,
        options: store::Options,
        work: WorkContext,
    ) -> Result<Admission<Self>> {
        let schema = schema.descriptor().validate()?;
        super::open::create_validated(path, schema, database, options, work)
    }

    /// Install the image file at `image` (moved, not copied when possible)
    /// as the database at `dest`, which must not exist. Format and schema are
    /// checked before the image is published.
    /// # Errors
    /// `NotABumbleDb`, a foreign schema, `DestinationExists`, or I/O failure.
    pub fn install_image(
        image: &Path,
        dest: &Path,
        schema: S,
        options: store::Options,
        work: WorkContext,
    ) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        let store = Store::install_image(image, dest, &schema, options)?;
        Self::assemble(store, schema, work)
    }
}

impl<'db, S> WriterSession<'db, S> {
    /// The committed generation under this session's exclusivity.
    /// # Errors
    /// Storage failure or cancellation.
    pub fn generation(&self) -> Result<GenerationId> {
        self.owner.parent_generation()
    }

    /// Judge each change set in order against the committed state plus the
    /// earlier accepted sets; a rejected set rolls back alone. Nothing
    /// commits.
    /// # Errors
    /// A foreign-schema change set, storage failure or cancellation.
    pub fn decide_all(&mut self, changes: &[ChangeSet]) -> Result<Vec<Judged>> {
        let schema = self.db.schema.as_ref();
        self.owner
            .decide_all(schema, changes)?
            .into_iter()
            .map(|decided| match decided.judgment {
                Judgment::Admitted => Ok(Judged::Accepted(decided.applied)),
                Judgment::Rejected(judged) => Ok(Judged::Rejected(
                    super::violations::violations_from_judged(schema, judged, &self.work)?,
                )),
            })
            .collect()
    }

    /// Apply already-decided change sets in order, without judgment.
    /// # Errors
    /// A foreign-schema change set, `Full`, storage failure or cancellation.
    pub fn apply_decided<'session>(
        &'session mut self,
        changes: &[ChangeSet],
    ) -> Result<Prepared<'session, 'db, S>> {
        Ok(Prepared {
            inner: self.owner.prepare_decided(changes)?,
            marker: PhantomData,
        })
    }

    /// A transaction for host records and the head only.
    /// # Errors
    /// Storage failure or cancellation.
    pub fn unchanged<'session>(&'session mut self) -> Result<Prepared<'session, 'db, S>> {
        Ok(Prepared {
            inner: self.owner.prepare_unchanged()?,
            marker: PhantomData,
        })
    }
}

impl<'session, 'db, S> Prepared<'session, 'db, S> {
    /// Each applied change set's own net changes, in order.
    #[must_use]
    pub fn applied_each(&self) -> &[Applied] {
        self.inner.applied_each()
    }

    /// Seal host records and the head into the same transaction.
    /// # Errors
    /// Host key grammar, `Full`, storage failure or cancellation; any error
    /// drops the whole transaction.
    pub fn seal(self, host: HostChanges<'_>) -> Result<Sealed<'session, 'db, S>> {
        Ok(Sealed {
            inner: self.inner.seal(host)?,
            marker: PhantomData,
        })
    }

    pub fn abort(self) {
        self.inner.abort();
    }
}

impl<S> Sealed<'_, '_, S> {
    /// The one durability point for facts, generation and host bytes.
    /// # Errors
    /// `Full` or storage failure; nothing committed.
    pub fn commit(self) -> Result<Commit> {
        self.inner.commit()
    }

    pub fn abort(self) {
        self.inner.abort();
    }
}

impl<S> ReadFrame<'_, S> {
    /// The head from this snapshot.
    /// # Errors
    /// Storage failure.
    pub fn head(&self) -> Result<Option<&[u8]>> {
        self.snapshot.head()
    }

    /// One host record from this snapshot.
    /// # Errors
    /// A key longer than [`MAX_KEY`], or storage failure.
    pub fn host_record(&self, key: &[u8]) -> Result<Option<&[u8]>> {
        self.snapshot.host_record(key)
    }

    /// Every host record under `prefix`, in key order. Keys and values
    /// borrow the snapshot for one visit.
    /// # Errors
    /// A prefix longer than [`MAX_KEY`], storage failure, cancellation, or
    /// the visitor's own error.
    pub fn host_scan(&self, prefix: &[u8], visit: &mut HostVisitor<'_>) -> Result<()> {
        self.snapshot.host_scan(prefix, self.work, visit)
    }

    /// Every stored row in the canonical export order: relations in
    /// declaration order, then home, then canonical bytes.
    /// # Errors
    /// Storage failure, cancellation, or the visitor's own error.
    pub fn export(&self, visit: &mut dyn FnMut(RelationId, &[u8]) -> Result<()>) -> Result<()> {
        let mut failed = None;
        let exported = self.snapshot.export(self.work, &mut |relation, row| {
            visit(relation, row).map_err(|error| {
                failed = Some(error);
                Error::Cancelled
            })
        });
        match failed {
            Some(error) => Err(error),
            None => exported,
        }
    }

    /// BLAKE3 over the canonical export: equal digests mean equal content on
    /// any platform, whatever the allocation history.
    /// # Errors
    /// Storage failure or cancellation.
    pub fn content_digest(&self) -> Result<[u8; 32]> {
        self.snapshot.content_digest(self.work)
    }
}

/// A new database being populated for a migration: rows are copied and
/// applied unjudged, then the whole state is judged once at
/// [`Population::admit`]. Nothing is visible at the destination before then.
pub struct Population<S> {
    store: Store,
    staging: Staging,
    schema: Arc<Schema>,
    work: WorkContext,
    marker: PhantomData<fn() -> S>,
}

impl<S: Theory> Population<S> {
    /// Begin a population for `dest`, which must not exist.
    /// # Errors
    /// Schema validation, `DestinationExists`, or I/O failure.
    pub fn begin(
        dest: &Path,
        schema: S,
        database: DatabaseId,
        options: store::Options,
        work: WorkContext,
    ) -> Result<Self> {
        let schema = schema.descriptor().validate()?;
        let staging = Staging::begin(dest)?;
        store::store_env::init_directory(staging.path(), &schema, database, options)?;
        let store = Store::open(staging.path(), &schema, options)?;
        Ok(Self {
            store,
            staging,
            schema: Arc::new(schema),
            work,
            marker: PhantomData,
        })
    }
}

impl<S> Population<S> {
    /// Copy every row of relation `old` from another database's snapshot
    /// into relation `new`; both relations must have the same field types.
    /// Returns the rows added.
    /// # Errors
    /// Mismatched relations, `Full`, storage failure or cancellation.
    pub fn copy_relation<T>(
        &mut self,
        from: &ReadFrame<'_, T>,
        old: RelationId,
        new: RelationId,
    ) -> Result<u64> {
        let source = from
            .schema()
            .relation_checked(old)
            .ok_or(crate::error::DynIdError::UnknownRelation { relation: old })?;
        let target = self
            .schema
            .relation_checked(new)
            .ok_or(crate::error::DynIdError::UnknownRelation { relation: new })?;
        if source.fields().len() != target.fields().len() {
            return Err(FactShapeError::ArityMismatch {
                relation: new,
                mismatch: Mismatch {
                    witnessed: source.fields().len(),
                    required: target.fields().len(),
                },
            }
            .into());
        }
        if let Some(field) = source
            .fields()
            .iter()
            .zip(target.fields())
            .position(|(a, b)| a.value_type != b.value_type)
        {
            return Err(FactShapeError::TypeMismatch {
                relation: new,
                field: bumbledb_theory::schema::FieldId(
                    u16::try_from(field).expect("field ids fit u16"),
                ),
            }
            .into());
        }
        let rows = from.snapshot.rows(old)?;
        let mut owner = self.store.writer(&self.work)?;
        let prepared =
            owner.prepare_rows(new, &mut rows.map(|entry| entry.map(|(_, bytes)| bytes)))?;
        let added = prepared.applied().added;
        prepared
            .seal(HostChanges::NONE)
            .and_then(SealedWrite::commit)?;
        Ok(added)
    }

    /// Apply a change set sealed for the new schema, unjudged.
    /// # Errors
    /// A foreign-schema change set, `Full`, storage failure or cancellation.
    pub fn apply(&mut self, changes: &ChangeSet) -> Result<Applied> {
        let mut owner = self.store.writer(&self.work)?;
        let prepared = owner.prepare_decided(std::slice::from_ref(changes))?;
        let applied = prepared.applied();
        prepared
            .seal(HostChanges::NONE)
            .and_then(SealedWrite::commit)?;
        Ok(applied)
    }

    /// Judge the complete populated state. Accepted: seal `host`, publish,
    /// and open the database. Rejected: the staging directory is removed.
    /// # Errors
    /// Storage failure or cancellation.
    pub fn admit(self, host: HostChanges<'_>) -> Result<Admission<Db<S>>> {
        let Self {
            store,
            staging,
            schema,
            work,
            ..
        } = self;
        let snapshot = store.snapshot(&work)?;
        let judgment = store::judge_bridge::judge_snapshot(&schema, &snapshot, &work)?;
        drop(snapshot);
        if let Judgment::Rejected(judged) = judgment {
            return Ok(Admission::Rejected(
                super::violations::violations_from_judged(&schema, judged, &work)?,
            ));
        }
        {
            let mut owner = store.writer(&work)?;
            owner
                .prepare_unchanged()
                .and_then(|prepared| prepared.seal(host))
                .and_then(SealedWrite::commit)?;
        }
        let options = store.options();
        drop(store);
        let published = staging.publish(&schema, options)?;
        let schema = Arc::try_unwrap(schema).unwrap_or_else(|shared| (*shared).clone());
        Ok(Admission::Accepted(Db::assemble(published, schema, work)?))
    }
}
