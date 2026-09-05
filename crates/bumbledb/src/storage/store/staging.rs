//! Private staged storage: unready → admitted → installed (C04 / chapter 61).
//!
//! An [`UnreadyStore`] owns one population directory and exposes only
//! bounded population/index/opaque-adjunct operations, inspect of that
//! unready owner, and final admission. Genesis, origin binding, and
//! small host seals write through [`StageWriter::put_host`]. Receipt
//! cleanup uses [`UnreadyStore::delete_host_batch`] / [`StageWriter::delete_host_batch`]
//! in charged windows — never a full receipt-key vector and never a ready
//! [`crate::Db`]. There is no ordinary [`Store`] / [`crate::Db`] accessor
//! and no disarm-to-`(Store, PathBuf)` escape. Readiness is owning
//! [`AdmittedStore`] after [`UnreadyStore::admit`] (`judge_complete`), not
//! a destination file name. The one no-clobber publication is
//! [`AdmittedStore::install`] after that admit.
//!
//! ```compile_fail
//! fn require_store(unready: &bumbledb::store::UnreadyStore) {
//!     let _ = unready.store();
//! }
//! ```
//! ```compile_fail
//! fn require_disarm(unready: bumbledb::store::UnreadyStore) {
//!     let _ = unready.disarm();
//! }
//! ```
//! ```compile_fail
//! fn require_lawful_parent(unready: &bumbledb::store::UnreadyStore) {
//!     let _ = unready.lawful_parent();
//! }
//! ```
//! ```compile_fail
//! fn inspect_is_not_store(reader: &bumbledb::store::StageReader<'_>) {
//!     let _ = reader.store();
//! }
//! ```

use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use super::candidate::{Judgment, StoreCommit};
use super::error::{StoreError, StoreResult};
use super::host::{AttachmentChange, HostChanges, HostRecordChange, HostWindow};
use super::judge_bridge::UnindexedRows;
use super::map::MapPolicy;
use super::snapshot::OwnedSnapshot;
use super::store_env::{
    Store, init_staging_directory, publish_staging, staging_path, PublishOutcome,
};
use crate::schema::Schema;
use crate::work::{ByteKind, ByteReservation, WorkContext};
use crate::ChangeSet;

/// Exact staging identity owned for cleanup. Dropping an unpublished owner
/// removes only this sibling, never an unrelated destination or a path
/// another installer published.
#[derive(Debug)]
struct StagingIdentity {
    path: PathBuf,
}

impl StagingIdentity {
    fn disarm(&mut self) {
        self.path = PathBuf::new();
    }

    fn remove(&self) {
        if !self.path.as_os_str().is_empty() {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

impl Drop for StagingIdentity {
    fn drop(&mut self) {
        self.remove();
    }
}

/// Cleanup owner for one unpublished staging identity. Transferring this
/// value (not a bare path) is the only legal handoff.
#[derive(Debug)]
pub struct StagingCleanup {
    staging: StagingIdentity,
}

impl StagingCleanup {
    fn take(staging: StagingIdentity) -> Self {
        Self { staging }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.staging.path
    }

    /// Remove the owned unpublished sibling only.
    pub fn abandon(self) {
        self.staging.remove();
    }
}

/// Owned private unready store at a staging sibling of `dest`.
pub struct UnreadyStore {
    store: Store,
    staging: StagingIdentity,
    dest: PathBuf,
}

/// Complete final-state judgment succeeded; ready for no-clobber install.
pub struct AdmittedStore {
    store: Store,
    staging: StagingIdentity,
    dest: PathBuf,
}

/// Bounded population writer over an unready store. Not an ordinary store
/// escape and not a readiness or lawful-parent mint.
pub struct StageWriter<'store> {
    store: &'store Store,
}

/// Bounded inspect of one unready owner. Host records, attachment, and a
/// private [`OwnedSnapshot`] of the staging sibling — not a ready
/// [`crate::Db`], not [`Store`], and not a lawful-parent mint.
pub struct StageReader<'unready> {
    snapshot: OwnedSnapshot,
    _unready: PhantomData<&'unready UnreadyStore>,
}

/// Outcome of publishing an admitted staging directory to its destination.
/// Settlement is derived from this attempt's rename, not `dest.exists()`.
#[derive(Debug)]
pub enum InstallOutcome {
    /// The destination is complete and opened.
    Installed(Store),
    /// This attempt's rename reached `dest`; a later sync/open step failed.
    /// Cleanup must not remove `dest`.
    SettlementFailed {
        dest: PathBuf,
        detail: StoreError,
    },
    /// Nothing was published. `cleanup` owns this attempt's unpublished sibling.
    NotInstalled {
        cleanup: StagingCleanup,
        detail: StoreError,
    },
}

impl UnreadyStore {
    /// Begin population in a private staging directory for `dest`. Cleanup
    /// ownership exists before the first fallible setup step after the
    /// sibling directory is created. The destination must not exist.
    ///
    /// # Errors
    /// `DestinationExists`, lock/I/O/LMDB failures.
    pub fn begin(
        dest: &Path,
        schema: &Schema,
        policy: MapPolicy,
        work: &WorkContext,
    ) -> StoreResult<Self> {
        if dest.exists() {
            return Err(StoreError::DestinationExists {
                path: dest.to_path_buf(),
            });
        }
        if let Some(parent) = dest.parent()
            && !parent.as_os_str().is_empty()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)?;
        }
        let staging_path = staging_path(dest)?;
        let mut staging = StagingIdentity {
            path: staging_path.clone(),
        };
        if let Err(error) = init_staging_directory(&staging.path, dest, schema, policy) {
            staging.remove();
            staging.disarm();
            return Err(error);
        }
        let store = match Store::open(&staging.path, schema, policy) {
            Ok(store) => store,
            Err(error) => {
                staging.remove();
                staging.disarm();
                return Err(error);
            }
        };
        work.checkpoint()?;
        Ok(Self {
            store,
            staging,
            dest: dest.to_path_buf(),
        })
    }

    /// Bounded population: apply facts, maintain indexes, write adjuncts
    /// (genesis/binding via [`StageWriter::put_host`]; receipt cleanup via
    /// [`StageWriter::delete_host_batch`]). Intermediate invalidity is
    /// allowed; only [`Self::admit`] judges. Dest stays unpublished until
    /// install — that is not readiness.
    ///
    /// # Errors
    /// Population callback failure, storage failure, or stopped work.
    pub fn populate<R>(
        &self,
        work: &WorkContext,
        populate: impl FnOnce(&StageWriter<'_>, &WorkContext) -> StoreResult<R>,
    ) -> StoreResult<R> {
        populate(&StageWriter { store: &self.store }, work)
    }

    /// Bounded inspect of the unready owner. Dest stays unpublished (not
    /// a readiness name). Use [`StageReader::host_scan_batch`] for charged
    /// receipt windows; full [`StageReader::host_scan`] already streams
    /// but has no resume or byte cap. [`StageReader::snapshot`] is the
    /// same export / `visit_projection` grammar, without a ready [`crate::Db`].
    ///
    /// # Errors
    /// Snapshot acquisition, inspect callback failure, or stopped work.
    pub fn inspect<R>(
        &self,
        work: &WorkContext,
        inspect: impl FnOnce(&StageReader<'_>, &WorkContext) -> StoreResult<R>,
    ) -> StoreResult<R> {
        work.checkpoint()?;
        let snapshot = self.store.snapshot(work)?;
        inspect(
            &StageReader {
                snapshot,
                _unready: PhantomData,
            },
            work,
        )
    }

    /// Planned install path. Existence of this name is not readiness.
    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.dest
    }

    /// Delete one charged host window under `prefix`, exclusive after
    /// `after`. Peak holds this window's keys only. [`StageWriter::put_host`]
    /// still requires a complete [`HostChanges`] slice — do not pass every
    /// receipt delete there.
    ///
    /// # Errors
    /// Host-key grammar, growth refusals, storage failure, or stopped work.
    pub fn delete_host_batch(
        &self,
        prefix: &[u8],
        after: Option<&[u8]>,
        work: &WorkContext,
        byte_cap: u64,
    ) -> StoreResult<HostWindow> {
        self.populate(work, |stage, work| {
            stage.delete_host_batch(prefix, after, work, byte_cap)
        })
    }

    /// Complete final-state judgment over the populated store. Success is
    /// admitted ownership ([`AdmittedStore`]), not a dest file name. Never
    /// an empty-delta incremental prepare, and never a
    /// [`crate::schema::judge::LawfulParent`].
    ///
    /// # Errors
    /// Judgment refusals, storage failure, or stopped work.
    pub fn admit(self, schema: &Schema, work: &WorkContext) -> StoreResult<AdmittedStore> {
        work.checkpoint()?;
        let UnreadyStore {
            store,
            staging,
            dest,
        } = self;
        match store.judge_populated(schema, work)? {
            Judgment::Rejected(judged) => {
                let statement = judged
                    .first()
                    .map(|violation| violation.statement)
                    .unwrap_or(bumbledb_theory::schema::StatementId(0));
                drop(judged);
                return Err(StoreError::JudgeRefused {
                    statement,
                    detail: "staged store failed complete final-state judgment",
                });
            }
            Judgment::Admitted => {}
        }
        Ok(AdmittedStore {
            store,
            staging,
            dest,
        })
    }

    /// Abandon population: transfer the unpublished cleanup owner.
    #[must_use]
    pub fn abandon(self) -> StagingCleanup {
        drop(self.store);
        StagingCleanup::take(self.staging)
    }
}

impl StageWriter<'_> {
    /// Ingest a sealed delta without judgment.
    ///
    /// # Errors
    /// Foreign schema, growth refusals, storage failure, or stopped work.
    pub fn apply(&self, changes: &ChangeSet, work: &WorkContext) -> StoreResult<StoreCommit> {
        let _ = work;
        let mut owner = self.store.writer(work)?;
        owner.ingest(changes, &UnindexedRows)
    }

    /// Write opaque host records / attachment without judging facts.
    /// Requires a complete [`HostChanges`] slice for one seal — genesis
    /// attachment and origin binding. Do not pass every receipt delete
    /// here; use [`Self::delete_host_batch`]. Not a ready [`crate::Db`].
    ///
    /// # Errors
    /// Host-key grammar, growth refusals, storage failure, or stopped work.
    pub fn put_host(&self, host: HostChanges<'_>, work: &WorkContext) -> StoreResult<StoreCommit> {
        let mut owner = self.store.writer(work)?;
        owner.prepare_unchanged()?.seal(host)?.commit()
    }

    /// Delete one charged host window under `prefix`, exclusive after
    /// `after`. At least one record is taken if any remain, even when that
    /// record exceeds `byte_cap`. Peak holds this window's keys (Working
    /// reserved per key), never the full prefix.
    ///
    /// # Errors
    /// Host-key grammar, growth refusals, storage failure, or stopped work.
    pub fn delete_host_batch(
        &self,
        prefix: &[u8],
        after: Option<&[u8]>,
        work: &WorkContext,
        byte_cap: u64,
    ) -> StoreResult<HostWindow> {
        let snapshot = self.store.snapshot(work)?;
        let mut held: Vec<(Vec<u8>, ByteReservation)> = Vec::new();
        let window = snapshot.host_scan_batch(
            prefix,
            after,
            work,
            byte_cap,
            &mut |key, _value| {
                let charge = work.reserve(ByteKind::Working, key.len() as u64)?;
                held.push((key.to_vec(), charge));
                Ok(())
            },
        )?;
        drop(snapshot);
        if held.is_empty() {
            return Ok(window);
        }
        {
            let records: Vec<HostRecordChange<'_>> = held
                .iter()
                .map(|(key, _)| HostRecordChange::Delete { key })
                .collect();
            self.put_host(
                HostChanges {
                    records: &records,
                    attachment: AttachmentChange::Keep,
                },
                work,
            )?;
        }
        drop(held);
        Ok(window)
    }
}

impl StageReader<'_> {
    /// The private snapshot of the unready staging sibling. Same host and
    /// projection grammar as a published store; dest stays unpublished
    /// (publication, not readiness).
    #[must_use]
    pub fn snapshot(&self) -> &OwnedSnapshot {
        &self.snapshot
    }

    /// Opaque host attachment from this unready transaction.
    ///
    /// # Errors
    /// Storage failure.
    pub fn attachment(&self) -> StoreResult<Option<&[u8]>> {
        self.snapshot.attachment()
    }

    /// One opaque host record from this unready transaction.
    ///
    /// # Errors
    /// Host-key grammar or storage failure.
    pub fn host_record(&self, key: &[u8]) -> StoreResult<Option<&[u8]>> {
        self.snapshot.host_record(key)
    }

    /// Visit every committed host record under `prefix`. Already streams
    /// (one work step per record). No resume and no byte cap — do not
    /// accumulate every key. Use [`Self::host_scan_batch`] for windows.
    ///
    /// # Errors
    /// Host-key grammar, storage failure, stopped work, or the visitor.
    #[expect(
        clippy::type_complexity,
        reason = "same visitor as OwnedSnapshot::host_scan"
    )]
    pub fn host_scan<E: From<StoreError>>(
        &self,
        prefix: &[u8],
        work: &WorkContext,
        visit: &mut dyn FnMut(&[u8], &[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        self.snapshot.host_scan(prefix, work, visit)
    }

    /// One charged host window under `prefix`, exclusive after `after`.
    /// Peak is the visitor plus one resume key.
    ///
    /// # Errors
    /// Host-key grammar, storage failure, stopped work, or the visitor.
    #[expect(
        clippy::type_complexity,
        reason = "same visitor as OwnedSnapshot::host_scan_batch"
    )]
    pub fn host_scan_batch<E: From<StoreError>>(
        &self,
        prefix: &[u8],
        after: Option<&[u8]>,
        work: &WorkContext,
        byte_cap: u64,
        visit: &mut dyn FnMut(&[u8], &[u8]) -> Result<(), E>,
    ) -> Result<HostWindow, E> {
        self.snapshot
            .host_scan_batch(prefix, after, work, byte_cap, visit)
    }
}

impl AdmittedStore {
    /// Publish to the planned destination with no-clobber semantics.
    /// Rename success — not `dest.exists()` — decides whether this attempt
    /// installed the destination.
    #[must_use]
    pub fn install(self, schema: &Schema, policy: MapPolicy, work: &WorkContext) -> InstallOutcome {
        let AdmittedStore {
            store,
            mut staging,
            dest,
        } = self;
        let staging_path = staging.path.clone();
        drop(store);
        match publish_staging(&staging_path, &dest, schema, policy, work) {
            PublishOutcome::Installed(opened) => {
                staging.disarm();
                InstallOutcome::Installed(opened)
            }
            PublishOutcome::PublishedUnsettled { dest, detail } => {
                staging.disarm();
                InstallOutcome::SettlementFailed { dest, detail }
            }
            PublishOutcome::DestinationOccupied { path } => InstallOutcome::NotInstalled {
                cleanup: StagingCleanup::take(staging),
                detail: StoreError::DestinationExists { path },
            },
            PublishOutcome::NotPublished(detail) => InstallOutcome::NotInstalled {
                cleanup: StagingCleanup::take(staging),
                detail,
            },
        }
    }

    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.dest
    }
}

/// Populate a new store in staging, complete-judge, and install.
///
/// # Errors
/// Population/judgment failure before publish; settlement failures after
/// this attempt's rename become [`StoreError::InstallSettlementFailed`].
pub fn install_populated(
    dest: &Path,
    schema: &Schema,
    policy: MapPolicy,
    work: &WorkContext,
    populate: impl FnOnce(&StageWriter<'_>, &WorkContext) -> StoreResult<()>,
) -> StoreResult<Store> {
    let unready = UnreadyStore::begin(dest, schema, policy, work)?;
    unready.populate(work, populate)?;
    let admitted = unready.admit(schema, work)?;
    match admitted.install(schema, policy, work) {
        InstallOutcome::Installed(store) => Ok(store),
        InstallOutcome::SettlementFailed { dest, detail } => {
            Err(StoreError::InstallSettlementFailed {
                path: dest,
                detail: Box::new(detail),
            })
        }
        InstallOutcome::NotInstalled { cleanup, detail } => {
            cleanup.abandon();
            Err(detail)
        }
    }
}
