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
//! logical digests, admit the complete state through the core's one strict
//! change decoder and judged admission, install receipt/history rows and the
//! captured control **at S**, replay exactly the tail `(S, T]`, then install
//! the captured target head's CURRENT control projection for new admission
//! (never the old checkpoint's authority), verify the store, and only then
//! atomically activate the completed directory. Interrupted candidates stay
//! invisible staging.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bumbledb::integration::{AttachmentChange, HostChanges, HostRecordChange, HostSealError};
use bumbledb::schema::Theory;
use bumbledb::{ChangeSet, Db, WorkContext, WorkError};

use crate::apply::{self, ApplyError};
use crate::checkpointer::read_live_head;
use crate::codec::{self, StreamLimits, StreamSink};
use crate::history::authority::{Activation, HeadAuthority, encode_control};
use crate::history::command::Limits;
use crate::history::{DatabaseIdentity, DecisionStamp, FrameError};
use crate::manifest::{GcPhase, HeadRecord};
use crate::store::fence::{DirectoryLock, acquire_directory};
use crate::store::{BackendError, ConditionalStore, ObjectError, fetch_decision, get_verified};

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
/// materialization; `<dir>/staging-*` are owned invisible candidates.
#[must_use]
pub fn materialization_path(directory: &Path) -> PathBuf {
    directory.join("db")
}

fn staging_path(directory: &Path, nonce: u64) -> PathBuf {
    directory.join(format!("staging-{}-{nonce}", std::process::id()))
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
    B: ConditionalStore,
    B::Error: BackendError,
{
    // OwnedDirectory: the kernel lock precedes every read and every cleanup.
    let lock = lock_directory(directory)?;
    // IdentifiedOrigin: the authoritative head, before touching the cache.
    let head = identify(backend, prefix, head_cap)?;
    let expected = OriginBinding {
        origin: origin.into(),
        prefix: prefix.into(),
        identity: head.control.identity,
    };
    clean_owned_staging(directory)?;
    let ready = materialization_path(directory);
    let db = if ready.exists() {
        let db = Arc::new(Db::open(&ready, schema)?);
        verify_binding(&db, &expected)?;
        db
    } else {
        // BuildingOrCatchingUp: hydrate a fresh owned staging directory.
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

fn identify<B>(backend: &B, prefix: &str, head_cap: usize) -> Result<HeadRecord, RecoveryError>
where
    B: ConditionalStore,
    B::Error: BackendError,
{
    match read_live_head(backend, prefix, head_cap) {
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

/// The core `ChangeSet` wire header/record widths. `ChangeSet::parse` is the
/// authority over these bytes: any drift refuses there, fail-closed, before
/// any fact is admitted.
const CSET_MAGIC: &[u8; 8] = b"BDBCSET\0";
const CSET_VERSION: u16 = 1;

/// Collected checkpoint state: synthesized core change bytes (header + one
/// Add record per fact, in the stream's canonical order — validated by the
/// core's own strict `ChangeSet::parse` afterwards, fail-closed) plus the
/// keyed system records. RAM-bounded by the work budget (recorded cost
/// boundary: the successor store's streaming builder replaces this).
pub(crate) struct CollectedState {
    pub(crate) changes: Vec<u8>,
    pub(crate) system: Vec<(Vec<u8>, Vec<u8>)>,
}

struct ImportSink {
    changes: Vec<u8>,
    rows: u64,
    system: Vec<(Vec<u8>, Vec<u8>)>,
}

impl StreamSink for ImportSink {
    type Error = FrameError;

    fn fact(&mut self, relation: u32, row: &[u8]) -> Result<(), FrameError> {
        self.changes.push(1); // Add
        self.changes.extend_from_slice(&relation.to_be_bytes());
        self.changes
            .extend_from_slice(&(row.len() as u64).to_be_bytes());
        self.changes.extend_from_slice(row);
        self.rows += 1;
        Ok(())
    }

    fn system(&mut self, key: &[u8], value: &[u8]) -> Result<(), FrameError> {
        self.system.push((key.to_vec(), value.to_vec()));
        Ok(())
    }
}

/// Stream one checkpoint's chunks, verify them against the manifest's shared
/// logical digests over the UNFILTERED records, and collect the state.
///
/// # Errors
/// Digest/counter disagreement is corruption-class; nothing partial returns.
pub(crate) fn collect_checkpoint<E>(
    manifest: &codec::CheckpointManifest,
    chunks: impl IntoIterator<Item = Result<Vec<u8>, E>>,
    stream: StreamLimits,
) -> Result<CollectedState, RecoveryError>
where
    E: Into<RecoveryError>,
{
    let mut sink = ImportSink {
        changes: Vec::new(),
        rows: 0,
        system: Vec::new(),
    };
    sink.changes.extend_from_slice(CSET_MAGIC);
    sink.changes.extend_from_slice(&CSET_VERSION.to_be_bytes());
    sink.changes
        .extend_from_slice(&manifest.identity.schema_id.0);
    sink.changes.extend_from_slice(&manifest.rows.to_be_bytes());
    let summary = codec::read_stream(chunks, &mut sink, stream).map_err(|error| match error {
        codec::ReadError::Chunk(error) => error.into(),
        codec::ReadError::Frame(error) | codec::ReadError::Sink(error) => {
            RecoveryError::Frame(error)
        }
    })?;
    codec::verify_summary(manifest, &summary)
        .map_err(|_| RecoveryError::Corrupt("stream disagrees with its certificate"))?;
    if sink.rows != manifest.rows {
        return Err(RecoveryError::Corrupt("row counter mismatch"));
    }
    Ok(CollectedState {
        changes: sink.changes,
        system: sink.system,
    })
}

/// Create a fresh judged database at `path` and admit the collected state,
/// the given host records and the control attachment in one transaction.
///
/// # Errors
/// Judged invariant violations refuse activation with evidence.
pub(crate) fn admit_collected<S>(
    path: &Path,
    schema: S,
    state: &CollectedState,
    records: &[HostRecordChange<'_>],
    control: &[u8],
    work: &WorkContext,
) -> Result<Arc<Db<S>>, RecoveryError>
where
    S: Theory,
{
    let db = match Db::create(path, schema)? {
        bumbledb::Admission::Accepted(db) => Arc::new(db),
        bumbledb::Admission::Rejected(_) => return Err(RecoveryError::InvariantViolation),
    };
    let changes = ChangeSet::parse(db.schema(), &state.changes, work)?;
    let mut session = db.integration_writer(work)?;
    let prepared = match session.prepare(&changes)? {
        bumbledb::Admission::Accepted(prepared) => prepared,
        bumbledb::Admission::Rejected(_) => return Err(RecoveryError::InvariantViolation),
    };
    prepared
        .seal(HostChanges {
            records,
            attachment: AttachmentChange::Put(control),
        })?
        .commit()?;
    drop(session);
    Ok(db)
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
    B: ConditionalStore,
    B::Error: BackendError,
{
    let live = head
        .control
        .live()
        .map_err(|_| RecoveryError::Refused(RecoveryRefusal::DatabaseDeleted))?;
    let recovery = head
        .recovery
        .ok_or(RecoveryError::Corrupt("live head without recovery root"))?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::from(d.subsec_nanos()));
    let staging = staging_path(directory, nonce);

    // SelectingPublishedRoot + Building: base state from the checkpoint (or
    // blank genesis), then the exact tail.
    let (db, base_authority) = if let Some(manifest_ref) = recovery.checkpoint {
        let manifest_bytes = get_verified(backend, binding.prefix.as_ref(), &manifest_ref)?;
        let manifest = codec::decode_manifest(&manifest_bytes, stream)?;
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
        let chunk_bytes = manifest
            .chunks
            .iter()
            .map(|chunk| get_verified(backend, binding.prefix.as_ref(), chunk));
        let state = collect_checkpoint(&manifest, chunk_bytes, stream)?;
        // Filter receipt rows by the CAPTURED TARGET head's retirement
        // policy — after digest verification, never before.
        let retired_through = live.receipts.retired_through();
        let binding_bytes = encode_binding(binding)?;
        let mut records: Vec<HostRecordChange<'_>> = Vec::new();
        records.push(HostRecordChange::Put {
            key: BINDING_KEY,
            value: &binding_bytes,
        });
        for (key, value) in &state.system {
            if key.first() == Some(&crate::history::receipt::RECEIPT_KEY_PREFIX)
                && key.len() >= 9
                && u64::from_be_bytes(key[1..9].try_into().expect("width")) <= retired_through
            {
                continue;
            }
            records.push(HostRecordChange::Put { key, value });
        }
        // Install the captured control AT S: the tail replay verifies
        // each decision against this exact predecessor.
        let control_at_base = manifest.control_at_capture;
        let control_bytes = encode_control(&control_at_base, head_cap)?;
        let db = admit_collected(
            &staging,
            schema.clone(),
            &state,
            &records,
            &control_bytes,
            work,
        )?;
        (db, control_at_base)
    } else {
        // Genesis root: blank creation, whole chain is the tail.
        let db = match Db::create(&staging, schema.clone())? {
            bumbledb::Admission::Accepted(db) => Arc::new(db),
            bumbledb::Admission::Rejected(_) => {
                return Err(RecoveryError::InvariantViolation);
            }
        };
        let genesis = HeadAuthority::genesis(
            head.control.identity,
            recovery.base,
            Activation::NotActivated,
        )
        .map_err(|_| RecoveryError::Corrupt("recovery base is not a genesis stamp"))?;
        let control_bytes = encode_control(&genesis, head_cap)?;
        let binding_bytes = encode_binding(binding)?;
        let empty = ChangeSet::builder(db.schema(), work.clone())
            .finish()
            .map_err(RecoveryError::Changes)?;
        let mut session = db.integration_writer(work)?;
        let prepared = match session.prepare(&empty)? {
            bumbledb::Admission::Accepted(prepared) => prepared,
            bumbledb::Admission::Rejected(_) => {
                return Err(RecoveryError::InvariantViolation);
            }
        };
        prepared
            .seal(HostChanges {
                records: &[HostRecordChange::Put {
                    key: BINDING_KEY,
                    value: &binding_bytes,
                }],
                attachment: AttachmentChange::Put(&control_bytes),
            })?
            .commit()?;
        drop(session);
        (db, genesis)
    };

    // The exact tail (S, T]: walk digests backwards, apply forwards through
    // historical replay (current admission guards do not apply).
    let mut stamps: Vec<DecisionStamp> = Vec::new();
    let mut cursor = live.decision;
    while cursor != recovery.base {
        work.checkpoint()?;
        if cursor.seq == 0 || cursor.seq <= recovery.base.seq {
            return Err(RecoveryError::Corrupt(
                "tail walk left the recovery boundary",
            ));
        }
        stamps.push(cursor);
        let (_, bytes) = fetch_decision(
            backend,
            binding.prefix.as_ref(),
            recovery.epoch_floor,
            head.object_epoch,
            &cursor.hash,
        )?;
        let envelope = crate::history::decision::decode_decision(&bytes, command_limits)?;
        if envelope.stamp() != cursor {
            return Err(RecoveryError::Corrupt("tail decision digest mismatch"));
        }
        cursor = envelope.parent;
    }
    let mut authority = base_authority;
    for stamp in stamps.into_iter().rev() {
        let (_, bytes) = fetch_decision(
            backend,
            binding.prefix.as_ref(),
            recovery.epoch_floor,
            head.object_epoch,
            &stamp.hash,
        )?;
        authority = apply::materialize(&db, &authority, &bytes, command_limits, work)?;
    }

    // Verifying: the local position must equal the captured target tip, the
    // store must verify, and NEW admission gets the captured target head's
    // current control projection — never the checkpoint's old authority.
    let position = authority
        .position()
        .ok_or(RecoveryError::Corrupt("replayed into a tombstone"))?;
    if position.decision != live.decision {
        return Err(RecoveryError::Corrupt(
            "replay did not reach the captured tip",
        ));
    }
    let target_control = encode_control(&head.control, head_cap)?;
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
            records: &[],
            attachment: AttachmentChange::Put(&target_control),
        })?
        .commit()?;
    drop(session);
    let report = db.verify_store()?;
    if !report.findings().is_empty() {
        return Err(RecoveryError::Corrupt("hydrated store failed verification"));
    }

    // Ready: atomic completed-directory activation. Close the staging
    // mappings first; reopen at the final path.
    drop(db);
    let ready = materialization_path(directory);
    fs::rename(&staging, &ready).map_err(RecoveryError::Io)?;
    crate::store::fence::sync_parent(&ready).map_err(RecoveryError::Io)?;
    Ok(Arc::new(Db::open(&ready, schema)?))
}

/// Open a `LocalHistory` tenant directory: ownership, then the committed LMDB
/// state IS the authority — no remote tail envelope or replay checkpoint is
/// required merely to reopen it.
///
/// # Errors
/// Ownership, binding and storage refusals.
pub fn open_local<S>(
    directory: &Path,
    schema: S,
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
    let db = Arc::new(Db::open(&ready, schema)?);
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
    let db = match Db::create(&ready, schema)? {
        bumbledb::Admission::Accepted(db) => Arc::new(db),
        bumbledb::Admission::Rejected(_) => return Err(RecoveryError::InvariantViolation),
    };
    write_binding(&db, binding, work)?;
    Ok((lock, db))
}

/// The crash/ambiguity table's constructive arm for GC state: reopening
/// scratch after a checkpoint CAS may find the candidate is now a retained
/// ancestor/root — never delete objects because another checkpoint is
/// current. Exposed for the duty/recovery drivers.
#[must_use]
pub fn gc_in_progress(head: &HeadRecord) -> bool {
    !matches!(head.gc, GcPhase::Idle)
}
