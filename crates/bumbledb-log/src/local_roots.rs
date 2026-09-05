//! LocalHistory named restore points (chapter 21 local specialization;
//! LOCAL-01..03).
//!
//! A local point is one streamed canonical export in a unique self-contained
//! root directory — no cross-root chunk sharing, no remote tail envelope, no
//! epoch GC. Complete and fsync the export first, then atomically register
//! its bounded metadata in local LMDB. A crash before registration leaves
//! owned scratch, not a published point; a crash after registration leaves a
//! durable complete root. Release removes the registry entry transactionally
//! and performs owner-scoped directory cleanup afterwards; root IDs and
//! directories are never reused. The next process cleans abandoned
//! unregistered directories only under the kernel directory lock — a merely
//! paused owner keeps its lock and its in-flight export.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write as _};
use std::path::{Path, PathBuf};

use bumbledb::integration::{AttachmentChange, HostChanges, HostRecordChange};
use bumbledb::{ChangeSet, Db, WorkContext};

use crate::checkpointer::{Captured, CheckpointError, CheckpointPolicy, capture_into};
use crate::codec::{self, CheckpointManifest, ChunkSink, StreamLimits};
use crate::history::{
    DecisionDigest, DecisionStamp, FrameError, IncarnationId, OperationId, StateStamp,
};
use crate::manifest::RootPolicy;
use crate::manifest::wire::{self, Reader};
use crate::recovery::RecoveryError;
use crate::store::{ObjectKind, ObjectRef, hex32};

/// The one bounded registry host record. Outside the `m`/`r` projection
/// namespaces: local points are host-local artifacts, never checkpoint
/// content.
pub const REGISTRY_KEY: &[u8] = b"localroots";
pub const REGISTRY_FAMILY: &[u8] = b"bumbledb.localroots.v1\0";
pub const REGISTRY_LAYOUT: u16 = 1;
const REGISTRY_KIND: u8 = 1;
const REGISTRY_CAP: usize = 256 * 1024;

/// One registered local restore point: bounded metadata naming a complete
/// self-contained export directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRoot {
    /// Unique root/operation identity; never reused.
    pub id: OperationId,
    pub decision: DecisionStamp,
    pub state: StateStamp,
    /// Digest/length of the root's manifest file (checkpoint-manifest bytes).
    pub manifest_digest: [u8; 32],
    pub manifest_length: u64,
    pub chunk_count: u32,
    pub label: Box<str>,
}

#[derive(Debug)]
pub enum LocalRootError {
    /// The registry is full; no other root is discarded.
    RootCapacityExceeded,
    /// A root with this ID already exists; IDs are never reused.
    DuplicateRoot,
    /// The named root is not registered (a stale release cannot remove a
    /// different root).
    UnknownRoot,
    Frame(FrameError),
    Io(io::Error),
    Checkpoint(CheckpointError),
    Recovery(RecoveryError),
    Storage(bumbledb::Error),
    Corrupt(&'static str),
}

impl From<FrameError> for LocalRootError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}
impl From<io::Error> for LocalRootError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<CheckpointError> for LocalRootError {
    fn from(error: CheckpointError) -> Self {
        Self::Checkpoint(error)
    }
}
impl From<RecoveryError> for LocalRootError {
    fn from(error: RecoveryError) -> Self {
        Self::Recovery(error)
    }
}
impl From<bumbledb::Error> for LocalRootError {
    fn from(error: bumbledb::Error) -> Self {
        Self::Storage(error)
    }
}
impl From<bumbledb::integration::IntegrationError> for LocalRootError {
    fn from(error: bumbledb::integration::IntegrationError) -> Self {
        Self::Recovery(error.into())
    }
}

fn encode_registry(roots: &[LocalRoot]) -> Result<Vec<u8>, FrameError> {
    let mut out = wire::frame_header(REGISTRY_FAMILY, REGISTRY_LAYOUT, REGISTRY_KIND);
    wire::put_u32(
        &mut out,
        u32::try_from(roots.len()).map_err(|_| FrameError::LengthOverflow)?,
    );
    for root in roots {
        out.extend_from_slice(root.id.as_core().as_bytes());
        wire::put_u64(&mut out, root.decision.seq);
        out.extend_from_slice(root.decision.hash.as_bytes());
        out.extend_from_slice(root.state.incarnation.as_core().as_bytes());
        wire::put_u64(&mut out, root.state.data_revision);
        out.extend_from_slice(&root.manifest_digest);
        wire::put_u64(&mut out, root.manifest_length);
        wire::put_u32(&mut out, root.chunk_count);
        wire::put_span(&mut out, root.label.as_bytes())?;
    }
    wire::check_limit(out.len(), REGISTRY_CAP)?;
    Ok(out)
}

fn decode_registry(bytes: &[u8]) -> Result<Vec<LocalRoot>, FrameError> {
    let mut input = Reader::begin(
        bytes,
        REGISTRY_FAMILY,
        REGISTRY_LAYOUT,
        REGISTRY_KIND,
        REGISTRY_CAP,
    )?;
    let count = input.u32()? as usize;
    if count > 4 * RootPolicy::DEFAULT.max_roots {
        return Err(FrameError::InvalidCount);
    }
    let mut roots = Vec::with_capacity(count);
    for _ in 0..count {
        let id = OperationId::from_core(bumbledb::Id128::from_bytes(input.array()?));
        let decision = DecisionStamp {
            seq: input.u64()?,
            hash: DecisionDigest::from_bytes(input.array()?),
        };
        let state = StateStamp {
            incarnation: IncarnationId::from_core(bumbledb::Id128::from_bytes(input.array()?)),
            data_revision: input.u64()?,
        };
        let manifest_digest = input.array()?;
        let manifest_length = input.u64()?;
        let chunk_count = input.u32()?;
        let label = std::str::from_utf8(input.span(1_024)?)
            .map_err(|_| FrameError::Truncated { at: 0 })?
            .into();
        roots.push(LocalRoot {
            id,
            decision,
            state,
            manifest_digest,
            manifest_length,
            chunk_count,
            label,
        });
    }
    input.end()?;
    Ok(roots)
}

/// Read the committed registry.
///
/// # Errors
/// Storage/frame refusals.
pub fn registered_roots<S>(db: &Db<S>) -> Result<Vec<LocalRoot>, LocalRootError> {
    let mut owned = None;
    db.read(|read| {
        if let Ok(record) = read.integration_host_record(REGISTRY_KEY) {
            owned = record.map(<[u8]>::to_vec);
        }
        Ok(())
    })?;
    match owned {
        Some(bytes) => Ok(decode_registry(&bytes)?),
        None => Ok(Vec::new()),
    }
}

#[must_use]
pub fn roots_base(directory: &Path) -> PathBuf {
    directory.join("roots")
}

#[must_use]
pub fn root_directory(directory: &Path, id: OperationId) -> PathBuf {
    roots_base(directory).join(hex_id(id))
}

fn hex_id(id: OperationId) -> String {
    let bytes = id.as_core().as_bytes().to_owned();
    let mut digest = [0u8; 32];
    digest[..16].copy_from_slice(&bytes);
    hex32(&digest)[..32].to_string()
}

fn staging_base(directory: &Path) -> PathBuf {
    roots_base(directory).join(".staging")
}

fn chunk_name(index: u32) -> String {
    format!("chunk-{index:08}")
}

struct FileSink {
    directory: PathBuf,
    index: u32,
}

impl ChunkSink for FileSink {
    type Error = CheckpointError;

    fn chunk(&mut self, bytes: &[u8]) -> Result<ObjectRef, CheckpointError> {
        let path = self.directory.join(chunk_name(self.index));
        self.index += 1;
        write_synced(&path, bytes).map_err(|error| {
            CheckpointError::Object(crate::store::ObjectError::Backend(Box::new(error)))
        })?;
        Ok(ObjectRef::of(0, ObjectKind::Chunk, bytes))
    }
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn sync_dir(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

/// Create one named local restore point: streamed complete export into a
/// unique self-contained directory, fully durable BEFORE the atomic registry
/// commit. Repeated complete snapshots cost disk, explicitly, rather than a
/// second shared-object collector.
///
/// # Errors
/// Capacity/duplicate refusals never discard another root; a failure before
/// registration leaves only owned scratch.
pub fn create_restore_point<S>(
    db: &Db<S>,
    directory: &Path,
    id: OperationId,
    label: &str,
    policy: &CheckpointPolicy,
    root_policy: &RootPolicy,
    work: &WorkContext,
) -> Result<LocalRoot, LocalRootError> {
    let roots = registered_roots(db)?;
    if roots.len() >= root_policy.max_roots {
        return Err(LocalRootError::RootCapacityExceeded);
    }
    if roots.iter().any(|root| root.id == id) {
        return Err(LocalRootError::DuplicateRoot);
    }
    if label.len() > root_policy.max_label_bytes {
        return Err(LocalRootError::Frame(FrameError::LimitExceeded));
    }
    // Export into owned staging under this process's exclusive ownership.
    let staging = staging_base(directory).join(hex_id(id));
    fs::create_dir_all(&staging)?;
    let mut sink = FileSink {
        directory: staging.clone(),
        index: 0,
    };
    // Locally retired rows are already deleted in the same transaction as
    // their frontier, so the export needs no extra filter.
    let Captured { authority, summary } =
        capture_into(db, &mut sink, 0, policy, work).map_err(LocalRootError::Checkpoint)?;
    let position = authority
        .position()
        .ok_or(LocalRootError::Corrupt("captured a tombstone"))?;
    let manifest = CheckpointManifest {
        identity: authority.identity,
        decision: position.decision,
        state: position.state,
        control_at_capture: authority,
        application_digest: summary.application_digest,
        system_digest: summary.system_digest,
        stream_digest: summary.stream_digest,
        total_bytes: summary.total_bytes,
        rows: summary.rows,
        system_records: summary.system_records,
        chunks: summary.chunks.clone(),
    };
    let manifest_bytes = codec::encode_manifest(&manifest, policy.stream)?;
    write_synced(&staging.join("manifest"), &manifest_bytes)?;
    sync_dir(&staging)?;
    // Complete directory first: rename into its final never-reused name.
    let final_dir = root_directory(directory, id);
    fs::rename(&staging, &final_dir)?;
    sync_dir(&roots_base(directory))?;
    // Then the atomic registry commit. A crash between rename and commit
    // leaves an unregistered complete directory — owned scratch the next
    // owner cleans; never a restore point.
    let root = LocalRoot {
        id,
        decision: position.decision,
        state: position.state,
        manifest_digest: *blake3::hash(&manifest_bytes).as_bytes(),
        manifest_length: manifest_bytes.len() as u64,
        chunk_count: u32::try_from(summary.chunks.len())
            .map_err(|_| LocalRootError::Frame(FrameError::LengthOverflow))?,
        label: label.into(),
    };
    let mut next = roots;
    next.push(root.clone());
    commit_registry(db, &next, work)?;
    Ok(root)
}

fn commit_registry<S>(
    db: &Db<S>,
    roots: &[LocalRoot],
    work: &WorkContext,
) -> Result<(), LocalRootError> {
    let bytes = encode_registry(roots)?;
    let mut session = db.integration_writer(work)?;
    let empty = ChangeSet::builder(db.schema(), work.clone())
        .finish()
        .map_err(|_| LocalRootError::Corrupt("empty delta refused"))?;
    let prepared = match session.prepare(&empty)? {
        bumbledb::Admission::Accepted(prepared) => prepared,
        bumbledb::Admission::Rejected(_) => {
            return Err(LocalRootError::Corrupt("empty delta rejected"));
        }
    };
    prepared
        .seal(HostChanges {
            records: &[HostRecordChange::Put {
                key: REGISTRY_KEY,
                value: &bytes,
            }],
            attachment: AttachmentChange::Keep,
        })?
        .commit()?;
    Ok(())
}

/// What a release actually accomplished. The registry entry is gone either
/// way; directory removal may need the reopen-time owner-scoped cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseReport {
    pub directory_removed: bool,
}

/// Release one named point: transactional deregistration, then owner-scoped
/// directory cleanup. A failed deletion is retried by the next owned open;
/// it never blocks the durable release or touches another root.
///
/// # Errors
/// Unknown roots refuse; a stale release cannot remove a different root.
pub fn release_restore_point<S>(
    db: &Db<S>,
    directory: &Path,
    id: OperationId,
    work: &WorkContext,
) -> Result<ReleaseReport, LocalRootError> {
    let roots = registered_roots(db)?;
    let Some(index) = roots.iter().position(|root| root.id == id) else {
        return Err(LocalRootError::UnknownRoot);
    };
    let mut next = roots;
    next.remove(index);
    commit_registry(db, &next, work)?;
    let removed = fs::remove_dir_all(root_directory(directory, id)).is_ok();
    Ok(ReleaseReport {
        directory_removed: removed,
    })
}

/// Owner-scoped cleanup at open, under the held directory lock: staging
/// scratch and released-but-undeleted root directories go; every registered
/// root and everything else stays. Distinct root directories share no
/// collectible files, so cleanup of one cannot corrupt another.
///
/// # Errors
/// Storage/IO refusals.
pub fn clean_roots<S>(db: &Db<S>, directory: &Path) -> Result<(), LocalRootError> {
    let registered: std::collections::BTreeSet<String> = registered_roots(db)?
        .into_iter()
        .map(|root| hex_id(root.id))
        .collect();
    let base = roots_base(directory);
    let staging = staging_base(directory);
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    let listing = match fs::read_dir(&base) {
        Ok(listing) => listing,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(LocalRootError::Io(error)),
    };
    for entry in listing {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with('.') {
            continue;
        }
        if !registered.contains(name) {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

/// Read one registered point's manifest and verify its recorded digest.
///
/// # Errors
/// Missing/corrupt manifests refuse with evidence; the point stays intact.
pub fn read_point_manifest(
    directory: &Path,
    root: &LocalRoot,
    limits: StreamLimits,
) -> Result<CheckpointManifest, LocalRootError> {
    let path = root_directory(directory, root.id).join("manifest");
    let bytes = read_file(&path)?;
    if bytes.len() as u64 != root.manifest_length
        || *blake3::hash(&bytes).as_bytes() != root.manifest_digest
    {
        return Err(LocalRootError::Corrupt("restore-point manifest mismatch"));
    }
    Ok(codec::decode_manifest(&bytes, limits)?)
}

/// The point's chunk bytes in order, each verified against the manifest's
/// chunk references.
///
/// # Errors
/// Missing/corrupt chunks refuse.
pub fn read_point_chunks<'m>(
    directory: &Path,
    root: &LocalRoot,
    manifest: &'m CheckpointManifest,
) -> impl Iterator<Item = Result<Vec<u8>, LocalRootError>> + 'm {
    let dir = root_directory(directory, root.id);
    manifest
        .chunks
        .iter()
        .enumerate()
        .map(move |(index, reference)| {
            let path = dir.join(chunk_name(
                u32::try_from(index).map_err(|_| LocalRootError::Corrupt("chunk index"))?,
            ));
            let bytes = read_file(&path)?;
            if bytes.len() as u64 != reference.length
                || crate::store::object_digest(ObjectKind::Chunk, &bytes) != reference.digest
            {
                return Err(LocalRootError::Corrupt("restore-point chunk mismatch"));
            }
            Ok(bytes)
        })
}

fn read_file(path: &Path) -> Result<Vec<u8>, LocalRootError> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}
