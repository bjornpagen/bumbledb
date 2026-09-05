//! Recovery: Closed → OwnedDirectory → IdentifiedOrigin → SelectingPublishedRoot
//! → BuildingOrCatchingUp → Verifying → Ready (chapter 22; REC-01..07).
//!
//! The local process-lifetime lock is acquired FIRST — before reading
//! recovery scratch, adopting a capsule or deleting anything. Identity and
//! configured origin are verified before any row is served or any cleanup
//! runs; equal row counts/schema/generations never establish identity.
//! Failed hydration is never an empty database; a missing `HEAD` at a
//! configured existing origin is `DatabaseMissing` — creation is a separate
//! explicit operation, never open-or-create after an ambiguous GET.
//!
//! Cold hydration builds a new owned staging directory from the retained
//! recovery root: stream + verify every chunk against the manifest's shared
//! logical digests, apply the exact tail `(S, T]` as unjudged batches on
//! that unready owner (no `LawfulParent` from an incomplete prefix), then
//! one complete admission of the finished identified state — including the
//! captured target head's CURRENT control projection for new admission
//! (never the old checkpoint's authority). Interrupted candidates stay
//! invisible staging.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bumbledb::integration::{AttachmentChange, HostChanges, HostRecordChange, HostSealError};
use bumbledb::schema::Schema;
use bumbledb::schema::Theory;
use bumbledb::schema::RelationId;
use bumbledb::store::{
    HostWindow, InstallOutcome as StoreInstall, MapPolicy, StageReader, StageWriter,
    StagingCleanup, Store, UnreadyStore,
};
use bumbledb::{ChangeSet, Db, ScratchRelation, WorkContext, WorkError};
use bumbledb::work::{ChargedBytes, DEFAULT_RAM_BYTES};

use crate::apply::{self, ApplyError};
use crate::checkpointer::read_live_head;
use crate::codec::{self, StreamLimits, StreamSink};
use crate::history::authority::{Activation, HeadAuthority, Lifecycle, LiveAuthority, encode_control};
use crate::history::command::{
    Command, Limits, ReceiptMetadata, UnverifiedOutcome, UnverifiedReceiptEnvelope, encode_receipt,
};
use crate::history::decision;
use crate::history::locator::ChainVisitor;
use crate::history::receipt::receipt_key;
use crate::history::{DatabaseIdentity, DecisionStamp, FrameError, HeadRevision};
use crate::writer::LogError;
use crate::store::ObjectRef;
use crate::manifest::{GcPhase, HeadRecord};
use crate::store::fence::{DirectoryLock, acquire_directory};
use crate::store::{
    BackendError, ObjectError, ObservedError, ReceiveLimits, ReceivingStore, TransportContext,
    get_verified,
};

/// The host-record key of the origin/identity binding. Outside the `m`/`r`
/// system namespaces: bindings are host-local and never part of the logical
/// projection or a checkpoint.
pub const BINDING_KEY: &[u8] = b"binding";
pub const BINDING_FAMILY: &[u8] = b"bumbledb.binding.v1\0";
pub const BINDING_LAYOUT: u16 = 1;
const BINDING_KIND: u8 = 1;
const BINDING_CAP: usize = 8_192;

/// The complete canonical binding a local cache stores and verifies before
/// adoption: configured origin/prefix plus full database identity. Hashing a
/// directory name alone is not an origin check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginBinding {
    /// Backend/account/bucket description as configured ("local" for
    /// `LocalHistory`).
    pub origin: Box<str>,
    /// The object-key prefix of this database under the origin.
    pub prefix: Box<str>,
    pub identity: DatabaseIdentity,
}

/// # Errors
/// Oversized bindings refuse.
pub fn encode_binding(binding: &OriginBinding) -> Result<Vec<u8>, FrameError> {
    let mut out = crate::manifest::wire::frame_header(BINDING_FAMILY, BINDING_LAYOUT, BINDING_KIND);
    crate::manifest::wire::put_span(&mut out, binding.origin.as_bytes())?;
    crate::manifest::wire::put_span(&mut out, binding.prefix.as_bytes())?;
    out.extend_from_slice(binding.identity.database_id.as_core().as_bytes());
    out.extend_from_slice(binding.identity.incarnation_id.as_core().as_bytes());
    out.extend_from_slice(&binding.identity.schema_id.0);
    crate::manifest::wire::check_limit(out.len(), BINDING_CAP)?;
    Ok(out)
}

/// # Errors
/// Malformed bindings refuse.
pub fn decode_binding(bytes: &[u8]) -> Result<OriginBinding, FrameError> {
    let mut input = crate::manifest::wire::Reader::begin(
        bytes,
        BINDING_FAMILY,
        BINDING_LAYOUT,
        BINDING_KIND,
        BINDING_CAP,
    )?;
    let origin = std::str::from_utf8(input.span(BINDING_CAP)?)
        .map_err(|_| FrameError::Truncated { at: 0 })?
        .into();
    let prefix = std::str::from_utf8(input.span(BINDING_CAP)?)
        .map_err(|_| FrameError::Truncated { at: 0 })?
        .into();
    let identity = DatabaseIdentity {
        database_id: crate::history::DatabaseId::from_core(bumbledb::Id128::from_bytes(
            input.array()?,
        )),
        incarnation_id: crate::history::IncarnationId::from_core(bumbledb::Id128::from_bytes(
            input.array()?,
        )),
        schema_id: crate::history::SchemaId(input.array()?),
    };
    input.end()?;
    Ok(OriginBinding {
        origin,
        prefix,
        identity,
    })
}

/// Definite refusals: the caller's configuration or explicit operation is
/// wrong, and retrying without change cannot help.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryRefusal {
    /// Another live process owns this directory.
    AlreadyOwned,
    /// The configured origin has no HEAD; creation is a separate explicit
    /// operation.
    DatabaseMissing,
    /// The authority is a terminal tombstone; ordinary open refuses before
    /// hydration.
    DatabaseDeleted,
    /// The cache's recorded binding disagrees with the configured origin /
    /// current head identity. No read, pending publication or cleanup
    /// crosses this boundary.
    ForeignCache {
        cached: Box<OriginBinding>,
        expected: Box<OriginBinding>,
    },
    /// The cache carries no binding record — an unidentified directory is
    /// never adopted.
    UnidentifiedCache,
}

#[derive(Debug)]
pub enum RecoveryError {
    Refused(RecoveryRefusal),
    /// Authoritative objects are malformed/missing: a stopped tenant with
    /// evidence, never endless delete/reseed and never an empty fallback.
    Corrupt(&'static str),
    Object(ObjectError),
    Frame(FrameError),
    Storage(bumbledb::Error),
    Host(HostSealError),
    Work(WorkError),
    Io(io::Error),
    Apply(ApplyError),
    Changes(bumbledb::ChangeError),
    /// The imported final state violates the schema's laws: refuse
    /// activation, preserve evidence.
    InvariantViolation,
}

impl From<ObjectError> for RecoveryError {
    fn from(error: ObjectError) -> Self {
        Self::Object(error)
    }
}
impl From<FrameError> for RecoveryError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}
impl From<bumbledb::Error> for RecoveryError {
    fn from(error: bumbledb::Error) -> Self {
        Self::Storage(error)
    }
}
impl From<WorkError> for RecoveryError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}
impl From<io::Error> for RecoveryError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<HostSealError> for RecoveryError {
    fn from(error: HostSealError) -> Self {
        Self::Host(error)
    }
}
impl From<ApplyError> for RecoveryError {
    fn from(error: ApplyError) -> Self {
        Self::Apply(error)
    }
}
impl From<bumbledb::ChangeError> for RecoveryError {
    fn from(error: bumbledb::ChangeError) -> Self {
        Self::Changes(error)
    }
}
impl From<bumbledb::integration::IntegrationError> for RecoveryError {
    fn from(error: bumbledb::integration::IntegrationError) -> Self {
        match error {
            bumbledb::integration::IntegrationError::Core(error) => Self::Storage(error),
            bumbledb::integration::IntegrationError::Changes(error) => Self::Changes(error),
            bumbledb::integration::IntegrationError::Host(error) => Self::Host(error),
            bumbledb::integration::IntegrationError::Work(error) => Self::Work(error),
            bumbledb::integration::IntegrationError::ForeignSchema
            | bumbledb::integration::IntegrationError::ReentrantWriter => {
                Self::Corrupt("integration misuse during recovery")
            }
        }
    }
}

/// A `Ready` recovery: kernel ownership, the verified materialization, and
/// the head observed during identification. The lock is declared LAST so it
/// releases after the database handle drops.
pub struct Recovered<S> {
    pub db: Arc<Db<S>>,
    pub head: HeadRecord,
    pub lock: DirectoryLock,
}

/// The tenant-directory layout recovery owns: `<dir>/db` is the ready
/// materialization. Unpublished siblings are L07 staging identities, not a
/// filename-readiness test.
#[must_use]
pub fn materialization_path(directory: &Path) -> PathBuf {
    directory.join("db")
}

/// Write the binding record in one transaction (used at create/hydration).
///
/// # Errors
/// Storage refusals.
pub fn write_binding<S>(
    db: &Db<S>,
    binding: &OriginBinding,
    work: &WorkContext,
) -> Result<(), RecoveryError> {
    let bytes = encode_binding(binding)?;
    let mut session = db.integration_writer(work)?;
    let empty = ChangeSet::builder(db.schema(), work.clone())
        .finish()
        .map_err(RecoveryError::Changes)?;
    let prepared = match session.prepare(&empty)? {
        bumbledb::Admission::Accepted(prepared) => prepared,
        bumbledb::Admission::Rejected(_) => {
            return Err(RecoveryError::Corrupt("empty delta rejected"));
        }
    };
    prepared
        .seal(HostChanges {
            records: &[HostRecordChange::Put {
                key: BINDING_KEY,
                value: &bytes,
            }],
            attachment: AttachmentChange::Keep,
        })?
        .commit()?;
    Ok(())
}

/// Verify the cache's recorded binding against the expected configuration.
///
/// # Errors
/// `UnidentifiedCache` / `ForeignCache` refusals precede every read.
pub fn verify_binding<S>(db: &Db<S>, expected: &OriginBinding) -> Result<(), RecoveryError> {
    let mut owned = None;
    db.read(|read| {
        match read.integration_host_record(BINDING_KEY) {
            Ok(record) => owned = record.map(<[u8]>::to_vec),
            Err(_) => owned = None,
        }
        Ok(())
    })?;
    let Some(bytes) = owned else {
        return Err(RecoveryError::Refused(RecoveryRefusal::UnidentifiedCache));
    };
    let cached = decode_binding(&bytes)?;
    if cached != *expected {
        return Err(RecoveryError::Refused(RecoveryRefusal::ForeignCache {
            cached: Box::new(cached),
            expected: Box::new(expected.clone()),
        }));
    }
    Ok(())
}

fn lock_directory(directory: &Path) -> Result<DirectoryLock, RecoveryError> {
    match acquire_directory(directory) {
        Ok(lock) => Ok(lock),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
            Err(RecoveryError::Refused(RecoveryRefusal::AlreadyOwned))
        }
        Err(error) => Err(RecoveryError::Io(error)),
    }
}

/// Owned-scratch cleanup under the held lock: abandoned staging directories
/// of dead processes are removed; the ready materialization, quarantine
/// evidence and everything else stay. Never reached without ownership.
fn clean_owned_staging(directory: &Path) -> Result<(), RecoveryError> {
    let listing = match fs::read_dir(directory) {
        Ok(listing) => listing,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(RecoveryError::Io(error)),
    };
    for entry in listing {
        let entry = entry.map_err(RecoveryError::Io)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with("staging-") {
            fs::remove_dir_all(entry.path()).map_err(RecoveryError::Io)?;
        }
    }
    Ok(())
}

/// Open the hosted tenant directory: ownership, identification, verification
/// and (for an absent cache) complete hydration. Never initializes a missing
/// head and never serves an unverified directory.
///
/// # Errors
/// Typed refusals; `Corrupt` stops the tenant with evidence.
#[expect(clippy::too_many_arguments, reason = "one bounded hosted open")]
pub fn open_hosted<S, B>(
    directory: &Path,
    schema: S,
    backend: &B,
    origin: &str,
    prefix: &str,
    command_limits: Limits,
    stream: StreamLimits,
    head_cap: usize,
    work: &WorkContext,
) -> Result<Recovered<S>, RecoveryError>
where
    S: Theory + Clone,
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    // OwnedDirectory: the kernel lock precedes every read and every cleanup.
    let lock = lock_directory(directory)?;
    // IdentifiedOrigin: the authoritative head, before touching the cache.
    let head = identify(backend, prefix, head_cap, work)?;
    let expected = OriginBinding {
        origin: origin.into(),
        prefix: prefix.into(),
        identity: head.control.identity,
    };
    clean_owned_staging(directory)?;
    let ready = materialization_path(directory);
    let db = if ready.exists() {
        let db = Arc::new(Db::open(&ready, schema, work.clone())?);
        verify_binding(&db, &expected)?;
        db
    } else {
        // BuildingOrCatchingUp: hydrate the unpublished sibling of `<dir>/db`.
        hydrate(
            directory,
            schema,
            backend,
            &expected,
            &head,
            command_limits,
            stream,
            head_cap,
            work,
        )?
    };
    Ok(Recovered { db, head, lock })
}

fn identify<B>(
    backend: &B,
    prefix: &str,
    head_cap: usize,
    work: &WorkContext,
) -> Result<HeadRecord, RecoveryError>
where
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    match read_live_head(backend, prefix, head_cap, work) {
        Ok((head, _)) => {
            if head.control.live().is_err() {
                return Err(RecoveryError::Refused(RecoveryRefusal::DatabaseDeleted));
            }
            Ok(head)
        }
        Err(crate::checkpointer::CheckpointError::NotInitialized) => {
            Err(RecoveryError::Refused(RecoveryRefusal::DatabaseMissing))
        }
        Err(crate::checkpointer::CheckpointError::Object(error)) => Err(error.into()),
        Err(crate::checkpointer::CheckpointError::Frame(error)) => Err(error.into()),
        Err(_) => Err(RecoveryError::Corrupt("head identification failed")),
    }
}

// ---------------------------------------------------------------------------
// Bounded streaming import (C4 / C6).
//
// Hydration and restore never materialize `Vec<entire_database>`: facts and
// system records stream from the verified chunks into the private unready
// owner in bounded batches. Intermediate prefixes are unjudged. The ONE
// complete production judgment is [`StagedPopulation::complete_install`]
// (`UnreadyStore::admit` → L07 `judge_complete`). No empty-delta incremental
// path, no `store()` / `disarm` escape, no handmade ChangeSet header.
// ---------------------------------------------------------------------------

/// Upper bound for one import batch's buffered bytes. The effective budget
/// is `min(stream.record_bytes, this)` — the import must buffer at least one
/// record, so its RAM bound rides the stream's own record bound and never
/// exceeds one batch plus one record. Small tenants stay on the in-RAM fast
/// path: one batch, one commit.
const IMPORT_BATCH_BYTES: usize = 16 * 1024 * 1024;

fn import_batch_budget(stream: StreamLimits) -> usize {
    stream.record_bytes.clamp(1, IMPORT_BATCH_BYTES)
}

fn store_recovery_error(error: bumbledb::store::StoreError) -> RecoveryError {
    RecoveryError::Host(error.into())
}

fn map_store_install(outcome: StoreInstall) -> Result<Store, RecoveryError> {
    match outcome {
        StoreInstall::Installed(store) => Ok(store),
        StoreInstall::SettlementFailed { dest, detail } => {
            Err(RecoveryError::Storage(bumbledb::Error::from_store(
                match detail {
                    bumbledb::store::StoreError::DestinationExists { .. } => detail,
                    other => bumbledb::store::StoreError::InstallSettlementFailed {
                        path: dest,
                        detail: Box::new(other),
                    },
                },
            )))
        }
        StoreInstall::NotInstalled { cleanup, detail } => {
            cleanup.abandon();
            Err(store_recovery_error(detail))
        }
    }
}

/// Post-rename settlement: the destination exists. Callers must not treat
/// this as `NotInstalled` or delete `dest`.
pub(crate) fn settlement_failed(
    dest: PathBuf,
    detail: bumbledb::store::StoreError,
) -> RecoveryError {
    RecoveryError::Storage(bumbledb::Error::from_store(
        bumbledb::store::StoreError::InstallSettlementFailed {
            path: dest,
            detail: Box::new(detail),
        },
    ))
}

/// Open and verify a destination this attempt already published.
///
/// # Errors
/// [`StoreError::InstallSettlementFailed`]: dest stays.
pub(crate) fn open_published<S>(
    dest: &Path,
    schema: S,
    work: &WorkContext,
) -> Result<Arc<Db<S>>, RecoveryError>
where
    S: Theory,
{
    let db = match Db::open(dest, schema, work.clone()) {
        Ok(db) => Arc::new(db),
        Err(error) => {
            return Err(settlement_failed(
                dest.to_path_buf(),
                bumbledb::store::StoreError::from(io::Error::other(error)),
            ));
        }
    };
    let report = match db.verify_store() {
        Ok(report) => report,
        Err(error) => {
            return Err(settlement_failed(
                dest.to_path_buf(),
                bumbledb::store::StoreError::from(io::Error::other(error)),
            ));
        }
    };
    if !report.findings().is_empty() {
        return Err(settlement_failed(
            dest.to_path_buf(),
            bumbledb::store::StoreError::from(io::Error::other(
                "published store failed verification",
            )),
        ));
    }
    Ok(db)
}

/// Owned unready population: private [`UnreadyStore`] only — no ordinary
/// Store/Db accessor, no `store()` / `disarm`, no bare cleanup path (C4).
/// Population goes through [`StageWriter`]; terminal admit is complete
/// judgment; unpublished cleanup is [`StagingCleanup`].
pub(crate) struct StagedPopulation {
    unready: UnreadyStore,
    schema: Schema,
}

impl StagedPopulation {
    #[must_use]
    pub(crate) fn schema(&self) -> &Schema {
        &self.schema
    }

    #[must_use]
    pub(crate) fn destination(&self) -> &Path {
        self.unready.destination()
    }

    /// Bounded population through the private unready owner (C4).
    pub(crate) fn populate<R>(
        &self,
        work: &WorkContext,
        populate: impl FnOnce(&StageWriter<'_>, &WorkContext) -> bumbledb::store::StoreResult<R>,
    ) -> Result<R, RecoveryError> {
        self.unready
            .populate(work, populate)
            .map_err(store_recovery_error)
    }

    /// Apply one unjudged fact batch (intermediate prefixes are not final).
    pub(crate) fn apply_unjudged(
        &self,
        changes: &ChangeSet,
        work: &WorkContext,
    ) -> Result<(), RecoveryError> {
        self.populate(work, |writer, work| {
            writer.apply(changes, work)?;
            Ok(())
        })
    }

    /// Bounded inspect of the unpublished owner: export / host_scan of the
    /// staging sibling, not a ready [`Db`] and not a lawful-parent mint.
    pub(crate) fn inspect<R>(
        &self,
        work: &WorkContext,
        inspect: impl FnOnce(&StageReader<'_>, &WorkContext) -> bumbledb::store::StoreResult<R>,
    ) -> Result<R, RecoveryError> {
        self.unready
            .inspect(work, inspect)
            .map_err(store_recovery_error)
    }

    /// Binding and genesis attachment only. Receipt cleanup is
    /// [`Self::delete_host_batch`], not a `put_host` Delete slice.
    pub(crate) fn write_host(
        &self,
        records: &[HostRecordChange<'_>],
        control: Option<&[u8]>,
        work: &WorkContext,
    ) -> Result<(), RecoveryError> {
        self.populate(work, |writer, work| {
            let attachment = match control {
                Some(bytes) => AttachmentChange::Put(bytes),
                None => AttachmentChange::Keep,
            };
            writer.put_host(
                HostChanges {
                    records,
                    attachment,
                },
                work,
            )?;
            Ok(())
        })
    }

    /// One charged host-delete window under `prefix`, exclusive after
    /// `after`. Peak is this window, not every matching key.
    pub(crate) fn delete_host_batch(
        &self,
        prefix: &[u8],
        after: Option<&[u8]>,
        work: &WorkContext,
        byte_cap: u64,
    ) -> Result<HostWindow, RecoveryError> {
        self.unready
            .delete_host_batch(prefix, after, work, byte_cap)
            .map_err(store_recovery_error)
    }

    /// Complete final-state admission (`judge_complete` via L07 admit) and
    /// no-clobber publish. The one admission of a streamed import.
    pub(crate) fn complete_install(self, work: &WorkContext) -> Result<Store, RecoveryError> {
        let schema = self.schema.clone();
        let admitted = self
            .unready
            .admit(&schema, work)
            .map_err(store_recovery_error)?;
        map_store_install(admitted.install(&schema, MapPolicy::default(), work))
    }

    /// Abandon an unpublished stage: transfer the cleanup owner, never a path.
    #[must_use]
    pub(crate) fn abandon(self) -> StagingCleanup {
        self.unready.abandon()
    }
}

/// Begin population in a private staging directory for `dest`. Only the
/// store writer is available until judgment and install complete.
///
/// # Errors
/// Storage refusals.
pub(crate) fn begin_staged<S>(
    dest: &Path,
    theory: S,
    work: &WorkContext,
) -> Result<StagedPopulation, RecoveryError>
where
    S: Theory,
{
    let schema = theory.descriptor().validate().map_err(RecoveryError::Storage)?;
    let unready = UnreadyStore::begin(dest, &schema, MapPolicy::default(), work)
        .map_err(store_recovery_error)?;
    Ok(StagedPopulation { unready, schema })
}

/// Create a fresh judged database at `path` (a blank state that violates the
/// schema's laws refuses with evidence). Use [`begin_staged`] for hydration
/// and restore paths that import a complete final state.
///
/// # Errors
/// Storage refusals; judged violations refuse creation.
pub(crate) fn create_judged<S>(path: &Path, schema: S, work: &WorkContext) -> Result<Arc<Db<S>>, RecoveryError>
where
    S: Theory,
{
    match Db::create(path, schema, work.clone())? {
        bumbledb::Admission::Accepted(db) => Ok(Arc::new(db)),
        bumbledb::Admission::Rejected(_) => {
            if path.exists() {
                let _ = fs::remove_dir_all(path);
            }
            Err(RecoveryError::InvariantViolation)
        }
    }
}

/// Host-record / control update on an already-admitted [`Db`]. The store
/// has a lawful parent; this is not the unready complete-admission path.
///
/// # Errors
/// Judged invariant violations refuse installation with evidence.
pub(crate) fn install_judged<S>(
    db: &Db<S>,
    records: &[HostRecordChange<'_>],
    control: &[u8],
    work: &WorkContext,
) -> Result<(), RecoveryError> {
    let empty = ChangeSet::builder(db.schema(), work.clone())
        .finish()
        .map_err(RecoveryError::Changes)?;
    let mut session = db.integration_writer(work)?;
    let prepared = match session.prepare(&empty)? {
        bumbledb::Admission::Accepted(prepared) => prepared,
        bumbledb::Admission::Rejected(_) => return Err(RecoveryError::InvariantViolation),
    };
    prepared
        .seal(HostChanges {
            records,
            attachment: AttachmentChange::Put(control),
        })?
        .commit()?;
    Ok(())
}

struct BatchImportSink<'a> {
    staged: &'a StagedPopulation,
    work: &'a WorkContext,
    budget: usize,
    facts: Vec<(u32, Vec<u8>)>,
    fact_bytes: usize,
    rows: u64,
    keep: &'a mut dyn FnMut(&[u8], &[u8]) -> bool,
    on_fact: Option<&'a mut dyn FnMut(u32, &[u8])>,
    system: Vec<(Vec<u8>, Vec<u8>)>,
    system_bytes: usize,
}

impl BatchImportSink<'_> {
    /// Sort one bounded fact batch, decode through the core codec, and apply
    /// via [`ChangeSet::builder`] as an unjudged candidate. No handmade
    /// ChangeSet header.
    fn flush_facts(&mut self) -> Result<(), RecoveryError> {
        if self.facts.is_empty() {
            return Ok(());
        }
        self.facts.sort_unstable();
        let schema = self.staged.schema();
        let mut builder = ChangeSet::builder(schema, self.work.clone());
        for (relation, row) in &self.facts {
            let id = RelationId(*relation);
            let fields = schema
                .relation_checked(id)
                .ok_or(RecoveryError::Corrupt("import names an unknown relation"))?
                .fields();
            let decoded = bumbledb::canonical::decode(fields, row, self.work)
                .map_err(|_| RecoveryError::Corrupt("canonical row refused during import"))?;
            builder
                .insert(id, decoded.values())
                .map_err(RecoveryError::Changes)?;
        }
        let changes = builder.finish().map_err(RecoveryError::Changes)?;
        self.staged.apply_unjudged(&changes, self.work)?;
        self.facts.clear();
        self.fact_bytes = 0;
        Ok(())
    }

    /// Write one bounded batch of kept system records (a contiguous slice of
    /// the stream's ascending key order, so the host-key grammar holds).
    fn flush_system(&mut self) -> Result<(), RecoveryError> {
        if self.system.is_empty() {
            return Ok(());
        }
        let records: Vec<HostRecordChange<'_>> = self
            .system
            .iter()
            .map(|(key, value)| HostRecordChange::Put { key, value })
            .collect();
        self.staged.write_host(&records, None, self.work)?;
        self.system.clear();
        self.system_bytes = 0;
        Ok(())
    }

    fn finish(&mut self) -> Result<u64, RecoveryError> {
        self.flush_facts()?;
        self.flush_system()?;
        Ok(self.rows)
    }
}

impl StreamSink for BatchImportSink<'_> {
    type Error = RecoveryError;

    fn fact(&mut self, relation: u32, row: &[u8]) -> Result<(), RecoveryError> {
        if let Some(on_fact) = self.on_fact.as_mut() {
            on_fact(relation, row);
        }
        self.fact_bytes += row.len() + 16;
        self.facts.push((relation, row.to_vec()));
        self.rows += 1;
        if self.fact_bytes >= self.budget {
            self.flush_facts()?;
        }
        Ok(())
    }

    fn system(&mut self, key: &[u8], value: &[u8]) -> Result<(), RecoveryError> {
        // The stream's fact section is over; release the last fact batch
        // before buffering system records.
        self.flush_facts()?;
        if !(self.keep)(key, value) {
            return Ok(());
        }
        self.system_bytes += key.len() + value.len() + 32;
        self.system.push((key.to_vec(), value.to_vec()));
        if self.system_bytes >= self.budget {
            self.flush_system()?;
        }
        Ok(())
    }
}

/// Stream one checkpoint's verified chunks into the private unready owner
/// in bounded batches. The caller finishes with
/// [`StagedPopulation::complete_install`] — the one complete judgment.
///
/// Chunks are [`AsRef<[u8]>`] views. Fetch sites hold [`ChargedBytes`] and
/// pass `as_bytes()` (`B = &[u8]`) for the decode; `into_owner` runs after.
/// There is no Vec-only twin.
///
/// # Errors
/// Digest/counter disagreement is corruption-class; the target stays an
/// unactivated scratch on every failure.
pub(crate) fn import_stream<E, B>(
    staged: &StagedPopulation,
    manifest: &codec::CheckpointManifest,
    chunks: impl IntoIterator<Item = Result<B, E>>,
    keep: &mut dyn FnMut(&[u8], &[u8]) -> bool,
    on_fact: Option<&mut dyn FnMut(u32, &[u8])>,
    stream: StreamLimits,
    work: &WorkContext,
) -> Result<(), RecoveryError>
where
    E: Into<RecoveryError>,
    B: AsRef<[u8]>,
{
    let mut sink = BatchImportSink {
        staged,
        work,
        budget: import_batch_budget(stream),
        facts: Vec::new(),
        fact_bytes: 0,
        rows: 0,
        keep,
        on_fact,
        system: Vec::new(),
        system_bytes: 0,
    };
    let summary = codec::read_stream(chunks, &mut sink, stream).map_err(|error| match error {
        codec::ReadError::Chunk(error) => error.into(),
        codec::ReadError::Frame(error) => RecoveryError::Frame(error),
        codec::ReadError::Sink(error) => error,
    })?;
    let rows = sink.finish()?;
    codec::verify_summary(manifest, &summary)
        .map_err(|_| RecoveryError::Corrupt("stream disagrees with its certificate"))?;
    if rows != manifest.rows {
        return Err(RecoveryError::Corrupt("row counter mismatch"));
    }
    Ok(())
}

/// Fetch every checkpoint chunk under charge. Owners stay live across
/// [`import_stream`]; the caller releases them with [`ChargedBytes::into_owner`].
pub(crate) fn fetch_charged_chunks<B>(
    backend: &B,
    prefix: &str,
    chunks: &[ObjectRef],
    work: &WorkContext,
) -> Result<Vec<ChargedBytes>, RecoveryError>
where
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    chunks
        .iter()
        .map(|chunk| {
            get_verified(
                backend,
                prefix,
                chunk,
                TransportContext::new(work, ReceiveLimits::exact(chunk.length)),
            )
            .map_err(RecoveryError::from)
        })
        .collect()
}

/// Spill-backed reverse walk of `(base, tip]`. Replay stays on the unready
/// owner; [`StagedPopulation::complete_install`] is the one judgment.
struct ScratchTail {
    scratch: ScratchRelation,
}

impl ScratchTail {
    fn new(work: &WorkContext) -> Self {
        Self {
            scratch: ScratchRelation::new(work, DEFAULT_RAM_BYTES),
        }
    }
}

impl ChainVisitor for ScratchTail {
    type Error = RecoveryError;

    fn visit(
        &mut self,
        stamp: DecisionStamp,
        bytes: &[u8],
        _reference: ObjectRef,
    ) -> Result<bool, RecoveryError> {
        self.scratch
            .put(&stamp.seq.to_be_bytes(), bytes)
            .map_err(RecoveryError::Storage)?;
        Ok(true)
    }
}

/// Historical replay onto an unready owner. `apply::materialize` still needs
/// a ready [`Db`] and would mint a `LawfulParent` from an incomplete prefix
/// (empty nonempty-required genesis, or a capacity-floor checkpoint). The
/// recorded outcome is installed as a position — [`HeadAuthority::decided`]
/// refuses frozen historical heads and is not this path.
pub(crate) fn apply_unready_decision(
    staged: &StagedPopulation,
    authority_before: &HeadAuthority,
    decision_bytes: &[u8],
    limits: Limits,
    work: &WorkContext,
) -> Result<HeadAuthority, RecoveryError> {
    work.checkpoint().map_err(RecoveryError::Work)?;
    let envelope = decision::decode_decision(decision_bytes, limits)?;
    if envelope.identity != authority_before.identity {
        return Err(RecoveryError::Apply(ApplyError::Command(LogError::Identity)));
    }
    if apply::already_at(authority_before, envelope.stamp()) {
        return Ok(*authority_before);
    }
    let live = authority_before
        .live()
        .map_err(|_| RecoveryError::Corrupt("replayed into a tombstone"))?;
    decision::verify_step(live.decision, &envelope)
        .map_err(|error| RecoveryError::Apply(ApplyError::from(error)))?;
    let command = Command::parse(staged.schema(), envelope.canonical_command, limits, work)
        .map_err(|error| RecoveryError::Apply(ApplyError::Command(error.into())))?;
    if command.command_ref() != envelope.command {
        return Err(RecoveryError::Apply(ApplyError::OutcomeMismatch));
    }
    if matches!(envelope.outcome, UnverifiedOutcome::Committed { .. }) {
        staged.apply_unjudged(command.changes(), work)?;
    }
    let key = receipt_key(envelope.command.id);
    let row = encode_receipt(
        UnverifiedReceiptEnvelope {
            metadata: ReceiptMetadata {
                command: envelope.command,
                decision_at: envelope.stamp(),
                state_at: envelope.after_state,
            },
            outcome: envelope.outcome,
        },
        limits,
    )?;
    staged.write_host(
        &[HostRecordChange::Put {
            key: &key,
            value: &row,
        }],
        None,
        work,
    )?;
    replayed_authority(authority_before, &envelope)
}

fn replayed_authority(
    previous: &HeadAuthority,
    envelope: &decision::UnverifiedDecisionEnvelope<'_>,
) -> Result<HeadAuthority, RecoveryError> {
    let live = previous
        .live()
        .map_err(|_| RecoveryError::Corrupt("replayed into a tombstone"))?;
    let revision = previous
        .revision
        .0
        .checked_add(1)
        .ok_or(RecoveryError::Corrupt("head revision exhausted"))?;
    Ok(HeadAuthority {
        identity: previous.identity,
        revision: HeadRevision(revision),
        lifecycle: Lifecycle::Live(LiveAuthority {
            access: live.access,
            decision: envelope.stamp(),
            state: envelope.after_state,
            receipts: live.receipts,
        }),
        activation: previous.activation,
    })
}

fn replay_scratch_tail(
    staged: &StagedPopulation,
    mut authority: HeadAuthority,
    tail: &mut ScratchTail,
    command_limits: Limits,
    work: &WorkContext,
) -> Result<HeadAuthority, RecoveryError> {
    let mut apply_error = None;
    tail.scratch
        .for_each(&mut |_, bytes| match apply_unready_decision(
            staged,
            &authority,
            bytes,
            command_limits,
            work,
        ) {
            Ok(next) => {
                authority = next;
                Ok(true)
            }
            Err(error) => {
                apply_error = Some(error);
                Ok(false)
            }
        })
        .map_err(RecoveryError::Storage)?;
    if let Some(error) = apply_error {
        return Err(error);
    }
    Ok(authority)
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one bounded recovery pipeline"
)]
fn hydrate<S, B>(
    directory: &Path,
    schema: S,
    backend: &B,
    binding: &OriginBinding,
    head: &HeadRecord,
    command_limits: Limits,
    stream: StreamLimits,
    head_cap: usize,
    work: &WorkContext,
) -> Result<Arc<Db<S>>, RecoveryError>
where
    S: Theory + Clone,
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    let live = head
        .control
        .live()
        .map_err(|_| RecoveryError::Refused(RecoveryRefusal::DatabaseDeleted))?;
    let recovery = head
        .recovery
        .ok_or(RecoveryError::Corrupt("live head without recovery root"))?;
    let dest = materialization_path(directory);

    // SelectingPublishedRoot + Building: base state from the checkpoint (or
    // blank genesis), then the exact tail. The checkpoint streams into the
    // private unready sibling of `<dir>/db` in bounded batches (never a
    // whole-database RAM materialization). One complete_install publishes
    // that dest; an interrupted import stays an unpublished sibling.
    let binding_bytes = encode_binding(binding)?;
    let staged = begin_staged(&dest, schema.clone(), work)?;
    let base_authority = if let Some(manifest_ref) = recovery.checkpoint {
        let charged = get_verified(
            backend,
            binding.prefix.as_ref(),
            &manifest_ref,
            TransportContext::new(work, ReceiveLimits::exact(manifest_ref.length)),
        )?;
        let manifest = codec::decode_manifest(charged.as_bytes(), stream)?;
        drop(charged.into_owner());
        if manifest.identity != head.control.identity {
            return Err(RecoveryError::Corrupt(
                "checkpoint names a foreign identity",
            ));
        }
        if manifest.decision != recovery.base {
            return Err(RecoveryError::Corrupt(
                "checkpoint disagrees with recovery base",
            ));
        }
        let owners = fetch_charged_chunks(
            backend,
            binding.prefix.as_ref(),
            &manifest.chunks,
            work,
        )?;
        // Filter receipt rows by the CAPTURED TARGET head's retirement
        // policy — the stream digests still cover the UNFILTERED records.
        let retired_through = live.receipts.retired_through();
        let mut keep = |key: &[u8], _value: &[u8]| {
            !(key.first() == Some(&crate::history::receipt::RECEIPT_KEY_PREFIX)
                && key.len() >= 9
                && u64::from_be_bytes(key[1..9].try_into().expect("width")) <= retired_through)
        };
        import_stream(
            &staged,
            &manifest,
            owners.iter().map(|charged| Ok::<_, RecoveryError>(charged.as_bytes())),
            &mut keep,
            None,
            stream,
            work,
        )?;
        for charged in owners {
            drop(charged.into_owner());
        }
        // Captured control AT S is a host attachment on the unready owner:
        // the tail verifies each decision against this predecessor.
        let control_at_base = manifest.control_at_capture;
        let control_bytes = encode_control(&control_at_base, head_cap)?;
        staged.write_host(
            &[HostRecordChange::Put {
                key: BINDING_KEY,
                value: &binding_bytes,
            }],
            Some(&control_bytes),
            work,
        )?;
        control_at_base
    } else {
        // Genesis root: blank unready owner; the whole chain is the tail.
        // Do not admit the empty prefix — a nonempty-required schema is
        // still invalid here.
        let genesis = HeadAuthority::genesis(
            head.control.identity,
            recovery.base,
            Activation::NotActivated,
        )
        .map_err(|_| RecoveryError::Corrupt("recovery base is not a genesis stamp"))?;
        let control_bytes = encode_control(&genesis, head_cap)?;
        staged.write_host(
            &[HostRecordChange::Put {
                key: BINDING_KEY,
                value: &binding_bytes,
            }],
            Some(&control_bytes),
            work,
        )?;
        genesis
    };

    // The exact tail (S, T]: stream into spill-backed scratch, apply
    // forwards on the same unready owner. No whole-tail Vec.
    let mut budget = 1_048_576u64;
    let mut tail = ScratchTail::new(work);
    crate::history::locator::walk_decision_chain(
        backend,
        binding.prefix.as_ref(),
        live.decision,
        recovery.base,
        recovery.tip_object,
        command_limits,
        &mut budget,
        work,
        &mut tail,
    )?;
    let authority = replay_scratch_tail(&staged, base_authority, &mut tail, command_limits, work)?;

    // Verifying: the local position must equal the captured target tip.
    // NEW admission gets the captured target head's current control
    // projection — never the checkpoint's old authority — written before
    // the one complete judgment.
    let position = authority
        .position()
        .ok_or(RecoveryError::Corrupt("replayed into a tombstone"))?;
    if position.decision != live.decision {
        return Err(RecoveryError::Corrupt(
            "replay did not reach the captured tip",
        ));
    }
    let target_control = encode_control(&head.control, head_cap)?;
    staged.write_host(&[], Some(&target_control), work)?;
    staged.complete_install(work)?;
    open_published(&dest, schema, work)
}

/// Open a `LocalHistory` tenant directory: ownership, then the committed LMDB
/// state IS the authority — no remote tail envelope or replay checkpoint is
/// required merely to reopen it. The stored [`OriginBinding`] is verified
/// against the caller's configured origin before the directory is adopted
/// (STORE-08/REC-03 on the local path, audit-log #8): a byte-copied or moved
/// directory refuses instead of opening as a live authority at a new
/// location while the original may still accept writes. A deliberate
/// relocation is an explicit operation that rewrites the binding
/// ([`write_binding`]), never an implicit adoption at open.
///
/// # Errors
/// Ownership, binding (`UnidentifiedCache`/`ForeignCache`) and storage
/// refusals.
pub fn open_local<S>(
    directory: &Path,
    schema: S,
    expected: &OriginBinding,
    work: &WorkContext,
) -> Result<(DirectoryLock, Arc<Db<S>>), RecoveryError>
where
    S: Theory,
{
    let lock = lock_directory(directory)?;
    clean_owned_staging(directory)?;
    let ready = materialization_path(directory);
    if !ready.exists() {
        return Err(RecoveryError::Refused(RecoveryRefusal::DatabaseMissing));
    }
    let db = Arc::new(Db::open(&ready, schema, work.clone())?);
    verify_binding(&db, expected)?;
    let _ = work;
    Ok((lock, db))
}

/// Create a `LocalHistory` tenant directory: explicit creation, never
/// open-or-create. The binding commits before the lock releases.
///
/// # Errors
/// An existing materialization refuses.
pub fn create_local<S>(
    directory: &Path,
    schema: S,
    binding: &OriginBinding,
    work: &WorkContext,
) -> Result<(DirectoryLock, Arc<Db<S>>), RecoveryError>
where
    S: Theory,
{
    let lock = lock_directory(directory)?;
    let ready = materialization_path(directory);
    if ready.exists() {
        return Err(RecoveryError::Corrupt("materialization already exists"));
    }
    fs::create_dir_all(directory).map_err(RecoveryError::Io)?;
    let db = match Db::create(&ready, schema, work.clone())? {
        bumbledb::Admission::Accepted(db) => Arc::new(db),
        bumbledb::Admission::Rejected(_) => {
            if ready.exists() {
                let _ = fs::remove_dir_all(&ready);
            }
            return Err(RecoveryError::InvariantViolation);
        }
    };
    write_binding(&db, binding, work)?;
    Ok((lock, db))
}

/// What one materialization install established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOutcome {
    /// This call renamed the staged directory into place and synced the
    /// parent.
    Installed,
    /// A ready materialization already exists; nothing was moved or
    /// overwritten (the idempotent-retry arm).
    AlreadyInstalled,
}

/// Install a completed staged materialization as `<directory>/db` under the
/// tenant directory fence — the install fence the migration-initialize
/// bridge path calls (REC-01/MIG-14/FS-02; audit-log #12). The kernel
/// directory lock is acquired FIRST, so the existence check and the rename
/// are one critical section (no TOCTOU with a racing initialize or open);
/// the rename is followed by a parent-directory fsync so a power failure
/// cannot lose the completed install.
///
/// # Errors
/// `AlreadyOwned` when another live holder owns the directory; IO refusals.
pub fn install_materialization(
    directory: &Path,
    staged: &Path,
) -> Result<InstallOutcome, RecoveryError> {
    let _lock = lock_directory(directory)?;
    let ready = materialization_path(directory);
    if ready.exists() {
        return Ok(InstallOutcome::AlreadyInstalled);
    }
    fs::rename(staged, &ready).map_err(RecoveryError::Io)?;
    crate::store::fence::sync_parent(&ready).map_err(RecoveryError::Io)?;
    Ok(InstallOutcome::Installed)
}

/// The crash/ambiguity table's constructive arm for GC state: reopening
/// scratch after a checkpoint CAS may find the candidate is now a retained
/// ancestor/root — never delete objects because another checkpoint is
/// current. Exposed for the duty/recovery drivers.
#[must_use]
pub fn gc_in_progress(head: &HeadRecord) -> bool {
    !matches!(head.gc, GcPhase::Idle)
}
