//! Independent verified-bytes backup (chapter 22; BACKUP-01..05, OPS-002).
//!
//! A named restore point protects a root from ordinary GC INSIDE the active
//! store; a backup is independent complete bytes under separately authorized
//! credentials/destination. The procedure: copy the root's complete declared
//! dependency closure (checkpoint manifest, every chunk, the exact bounded
//! tail decisions) into the destination, verify every copied object by
//! reading it BACK from the destination and checking length + domain-separated
//! digest, then install one complete backup manifest LAST by no-overwrite
//! conditional create. Partial uploads are incomplete operations, never
//! usable backups; a lost completion response resolves by operation identity
//! and manifest digest. Normal writer/GC credentials never own the backup
//! destination — this module takes the destination as a distinct
//! [`ConditionalStore`] and never derives it from the source.
//!
//! The manifest declares external blobs explicitly: 1.0 backups are
//! database-only and say so, rather than implying arbitrary URLs in facts
//! were copied.

use crate::codec::{self, StreamLimits};
use crate::history::command::Limits;
use crate::history::decision;
use crate::history::{
    DatabaseId, DatabaseIdentity, DecisionDigest, DecisionStamp, FrameError, IncarnationId,
    OperationId, SchemaId, StateStamp,
};
use crate::manifest::RecoveryRoot;
use crate::manifest::wire::{self, Reader};
use crate::store::{
    BackendError, ConditionalOutcome, ConditionalStore, HeadRead, ObjectError, ObjectKind,
    ObjectRef, backend as backend_error, decision_key, fetch_decision, get_verified, hex32,
    object_digest,
};

use bumbledb::{WorkContext, WorkError};

pub const BACKUP_FAMILY: &[u8] = b"bumbledb.backup.v1\0";
pub const BACKUP_LAYOUT: u16 = 1;
const BACKUP_KIND: u8 = 1;
const MANIFEST_CAP: usize = 64 * 1024 * 1024;

/// The destination key of one backup operation's completion manifest. The
/// manifest is the ONLY completion marker: absent manifest means incomplete
/// backup, whatever objects were copied.
#[must_use]
pub fn backup_manifest_key(dest_prefix: &str, operation: OperationId) -> String {
    let mut digest = [0u8; 32];
    digest[..16].copy_from_slice(operation.as_core().as_bytes());
    format!("{dest_prefix}/backup/{}/manifest", &hex32(&digest)[..32])
}

/// Explicit external-blob declaration. 1.0 supports exactly `DatabaseOnly`:
/// a database-only backup remains clearly labeled database-only, and a text
/// field containing an S3 URL is never treated as reachability evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlobPolicy {
    DatabaseOnly,
}

/// One copied decision object: the epoch it was found under in the SOURCE
/// (it keeps that storage name in the destination), its digest and length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopiedDecision {
    pub epoch: u64,
    pub digest: DecisionDigest,
    pub length: u64,
}

/// The complete backup manifest: original identity/stamps, framing versions
/// (the frame header), the copied refs, the checkpoint's logical digests and
/// the explicit blob declaration. Installed last, no-overwrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupManifest {
    pub operation: OperationId,
    pub identity: DatabaseIdentity,
    /// The captured root: base (checkpoint) and tip stamps.
    pub base: DecisionStamp,
    pub tip: DecisionStamp,
    pub state: StateStamp,
    /// The copied checkpoint-manifest object, `None` for a genesis root
    /// whose whole chain is the tail.
    pub checkpoint: Option<ObjectRef>,
    /// The copied tail decisions, oldest first: exactly `(base, tip]`.
    pub decisions: Vec<CopiedDecision>,
    /// The checkpoint's logical digests (blank projection for genesis roots).
    pub application_digest: [u8; 32],
    pub system_digest: [u8; 32],
    pub blobs: BlobPolicy,
}

/// # Errors
/// Oversized manifests refuse.
pub fn encode_backup_manifest(manifest: &BackupManifest) -> Result<Vec<u8>, FrameError> {
    let mut out = wire::frame_header(BACKUP_FAMILY, BACKUP_LAYOUT, BACKUP_KIND);
    out.extend_from_slice(manifest.operation.as_core().as_bytes());
    out.extend_from_slice(manifest.identity.database_id.as_core().as_bytes());
    out.extend_from_slice(manifest.identity.incarnation_id.as_core().as_bytes());
    out.extend_from_slice(&manifest.identity.schema_id.0);
    wire::put_u64(&mut out, manifest.base.seq);
    out.extend_from_slice(manifest.base.hash.as_bytes());
    wire::put_u64(&mut out, manifest.tip.seq);
    out.extend_from_slice(manifest.tip.hash.as_bytes());
    out.extend_from_slice(manifest.state.incarnation.as_core().as_bytes());
    wire::put_u64(&mut out, manifest.state.data_revision);
    match manifest.checkpoint {
        Some(reference) => {
            out.push(1);
            crate::manifest::put_object_ref(&mut out, &reference);
        }
        None => out.push(0),
    }
    wire::put_u32(
        &mut out,
        u32::try_from(manifest.decisions.len()).map_err(|_| FrameError::LengthOverflow)?,
    );
    for copied in &manifest.decisions {
        wire::put_u64(&mut out, copied.epoch);
        out.extend_from_slice(copied.digest.as_bytes());
        wire::put_u64(&mut out, copied.length);
    }
    out.extend_from_slice(&manifest.application_digest);
    out.extend_from_slice(&manifest.system_digest);
    out.push(match manifest.blobs {
        BlobPolicy::DatabaseOnly => 0,
    });
    wire::check_limit(out.len(), MANIFEST_CAP)?;
    Ok(out)
}

/// # Errors
/// Malformed manifests refuse.
pub fn decode_backup_manifest(bytes: &[u8]) -> Result<BackupManifest, FrameError> {
    let mut input = Reader::begin(
        bytes,
        BACKUP_FAMILY,
        BACKUP_LAYOUT,
        BACKUP_KIND,
        MANIFEST_CAP,
    )?;
    let operation = OperationId::from_core(bumbledb::Id128::from_bytes(input.array()?));
    let identity = DatabaseIdentity {
        database_id: DatabaseId::from_core(bumbledb::Id128::from_bytes(input.array()?)),
        incarnation_id: IncarnationId::from_core(bumbledb::Id128::from_bytes(input.array()?)),
        schema_id: SchemaId(input.array()?),
    };
    let base = DecisionStamp {
        seq: input.u64()?,
        hash: DecisionDigest::from_bytes(input.array()?),
    };
    let tip = DecisionStamp {
        seq: input.u64()?,
        hash: DecisionDigest::from_bytes(input.array()?),
    };
    let state = StateStamp {
        incarnation: IncarnationId::from_core(bumbledb::Id128::from_bytes(input.array()?)),
        data_revision: input.u64()?,
    };
    let checkpoint = match input.tag()? {
        (_, 0) => None,
        (_, 1) => Some(crate::manifest::read_object_ref(&mut input)?),
        (at, got) => return Err(FrameError::Tag { at, got }),
    };
    let count = input.u32()? as usize;
    if count > bytes.len() / 48 {
        return Err(FrameError::InvalidCount);
    }
    let mut decisions = Vec::with_capacity(count);
    for _ in 0..count {
        decisions.push(CopiedDecision {
            epoch: input.u64()?,
            digest: DecisionDigest::from_bytes(input.array()?),
            length: input.u64()?,
        });
    }
    let application_digest = input.array()?;
    let system_digest = input.array()?;
    let blobs = match input.tag()? {
        (_, 0) => BlobPolicy::DatabaseOnly,
        (at, got) => return Err(FrameError::Tag { at, got }),
    };
    input.end()?;
    if state.incarnation != identity.incarnation_id || tip.seq < base.seq {
        return Err(FrameError::StateIdentityMismatch);
    }
    Ok(BackupManifest {
        operation,
        identity,
        base,
        tip,
        state,
        checkpoint,
        decisions,
        application_digest,
        system_digest,
        blobs,
    })
}

#[derive(Debug)]
pub enum BackupError {
    Object(ObjectError),
    Frame(FrameError),
    Work(WorkError),
    /// The destination's manifest key already holds a DIFFERENT completed
    /// backup — a foreign operation reused the key; nothing is overwritten.
    ConflictingOperation,
    /// The completion create's outcome could not be established and the read
    /// back found no manifest; the operation is incomplete, retry it.
    CompletionUnresolved,
    /// A verification pass found the named object absent/mismatched.
    Incomplete {
        key: String,
    },
    Corrupt(&'static str),
}

impl From<ObjectError> for BackupError {
    fn from(error: ObjectError) -> Self {
        Self::Object(error)
    }
}
impl From<FrameError> for BackupError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}
impl From<WorkError> for BackupError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

/// What one completed backup established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupReport {
    pub manifest: BackupManifest,
    pub manifest_digest: [u8; 32],
    /// Whether THIS call installed the manifest (false: an earlier identical
    /// operation had already completed; the retry is idempotent evidence).
    pub installed: bool,
    pub objects_copied: u64,
    pub bytes_copied: u64,
}

/// Copy one verified object from source to destination and prove the copy by
/// reading it back from the DESTINATION. An identical existing object is
/// idempotent; a conflicting one refuses.
fn copy_verified<Src, Dst>(
    source: &Src,
    source_prefix: &str,
    destination: &Dst,
    dest_prefix: &str,
    reference: &ObjectRef,
    work: &WorkContext,
) -> Result<u64, BackupError>
where
    Src: ConditionalStore,
    Src::Error: BackendError,
    Dst: ConditionalStore,
    Dst::Error: BackendError,
{
    work.checkpoint()?;
    let bytes = get_verified(source, source_prefix, reference)?;
    let key = reference.key(dest_prefix);
    // put_object is idempotent for identical bytes and refuses conflicts.
    destination
        .put_object(&key, &bytes)
        .map_err(backend_error)
        .map_err(BackupError::Object)?;
    // Verified bytes: read back from the destination, not trust in the PUT.
    let copied = get_verified(destination, dest_prefix, reference)
        .map_err(|_| BackupError::Incomplete { key: key.clone() })?;
    if copied != bytes {
        return Err(BackupError::Incomplete { key });
    }
    Ok(bytes.len() as u64)
}

/// Copy one decision object (content-addressed under the decision digest
/// domain, epoch-named) and verify it from the destination.
#[expect(clippy::too_many_arguments, reason = "one bounded decision copy")]
fn copy_decision<Src, Dst>(
    source: &Src,
    source_prefix: &str,
    destination: &Dst,
    dest_prefix: &str,
    epoch_floor: u64,
    epoch_ceiling: u64,
    digest: &DecisionDigest,
    work: &WorkContext,
) -> Result<CopiedDecision, BackupError>
where
    Src: ConditionalStore,
    Src::Error: BackendError,
    Dst: ConditionalStore,
    Dst::Error: BackendError,
{
    work.checkpoint()?;
    let (epoch, bytes) = fetch_decision(source, source_prefix, epoch_floor, epoch_ceiling, digest)?;
    let key = decision_key(dest_prefix, epoch, digest);
    destination
        .put_object(&key, &bytes)
        .map_err(backend_error)
        .map_err(BackupError::Object)?;
    let copied = match destination
        .get_object(&key)
        .map_err(backend_error)
        .map_err(BackupError::Object)?
    {
        crate::store::ObjectRead::Present { body } => body,
        crate::store::ObjectRead::Absent => return Err(BackupError::Incomplete { key }),
    };
    if object_digest(ObjectKind::Decision, &copied) != *digest.as_bytes() {
        return Err(BackupError::Incomplete { key });
    }
    Ok(CopiedDecision {
        epoch,
        digest: *digest,
        length: bytes.len() as u64,
    })
}

/// Stream one recovery root's complete declared dependency closure into the
/// destination, verify every copied object, then install the complete
/// manifest LAST by no-overwrite conditional create. Bounded: one object in
/// memory at a time, never a full-RAM materialization.
///
/// The caller must already hold this root against GC — a named restore point
/// for a hosted source ([`crate::admin::add_named_root_hosted`]); local
/// points are self-contained directories that need no pin.
///
/// # Errors
/// Copy/verify refusals leave an incomplete, unlisted operation. A lost
/// completion response resolves on retry by operation identity and manifest
/// digest; a foreign manifest at the operation's key refuses.
#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one bounded backup pipeline"
)]
pub fn backup_root<Src, Dst>(
    source: &Src,
    source_prefix: &str,
    destination: &Dst,
    dest_prefix: &str,
    identity: DatabaseIdentity,
    state: StateStamp,
    root: &RecoveryRoot,
    epoch_ceiling: u64,
    operation: OperationId,
    limits: Limits,
    stream: StreamLimits,
    work: &WorkContext,
) -> Result<BackupReport, BackupError>
where
    Src: ConditionalStore,
    Src::Error: BackendError,
    Dst: ConditionalStore,
    Dst::Error: BackendError,
{
    let mut objects = 0u64;
    let mut bytes = 0u64;
    // 1. The checkpoint manifest and every chunk, each verified from the
    //    destination after the copy.
    let (checkpoint, application_digest, system_digest) = match root.checkpoint {
        Some(reference) => {
            let manifest_bytes = get_verified(source, source_prefix, &reference)?;
            let checkpoint_manifest = codec::decode_manifest(&manifest_bytes, stream)?;
            if checkpoint_manifest.identity != identity {
                return Err(BackupError::Corrupt("checkpoint names a foreign identity"));
            }
            if checkpoint_manifest.decision != root.base {
                return Err(BackupError::Corrupt(
                    "checkpoint disagrees with the root base",
                ));
            }
            for chunk in &checkpoint_manifest.chunks {
                bytes +=
                    copy_verified(source, source_prefix, destination, dest_prefix, chunk, work)?;
                objects += 1;
            }
            bytes += copy_verified(
                source,
                source_prefix,
                destination,
                dest_prefix,
                &reference,
                work,
            )?;
            objects += 1;
            (
                Some(reference),
                checkpoint_manifest.application_digest,
                checkpoint_manifest.system_digest,
            )
        }
        None => (
            None,
            codec::empty_application_digest(),
            codec::empty_system_digest(),
        ),
    };
    // 2. The exact bounded tail (base, tip], copied oldest-first.
    let mut stamps: Vec<DecisionStamp> = Vec::new();
    let mut cursor = root.tip;
    while cursor != root.base {
        work.checkpoint()?;
        if cursor.seq == 0 || cursor.seq <= root.base.seq {
            return Err(BackupError::Corrupt("tail walk left the root boundary"));
        }
        stamps.push(cursor);
        let (_, decision_bytes) = fetch_decision(
            source,
            source_prefix,
            root.epoch_floor,
            epoch_ceiling,
            &cursor.hash,
        )?;
        let envelope = decision::decode_decision(&decision_bytes, limits)
            .map_err(|_| BackupError::Corrupt("tail decision malformed"))?;
        if envelope.stamp() != cursor {
            return Err(BackupError::Corrupt("tail decision digest mismatch"));
        }
        cursor = envelope.parent;
    }
    let mut decisions = Vec::with_capacity(stamps.len());
    for stamp in stamps.into_iter().rev() {
        let copied = copy_decision(
            source,
            source_prefix,
            destination,
            dest_prefix,
            root.epoch_floor,
            epoch_ceiling,
            &stamp.hash,
            work,
        )?;
        bytes += copied.length;
        objects += 1;
        decisions.push(copied);
    }
    // 3. The complete manifest, LAST, no-overwrite.
    let manifest = BackupManifest {
        operation,
        identity,
        base: root.base,
        tip: root.tip,
        state,
        checkpoint,
        decisions,
        application_digest,
        system_digest,
        blobs: BlobPolicy::DatabaseOnly,
    };
    let encoded = encode_backup_manifest(&manifest)?;
    let manifest_digest = *blake3::hash(&encoded).as_bytes();
    let key = backup_manifest_key(dest_prefix, operation);
    let installed = match destination
        .create_head(&key, &encoded)
        .map_err(backend_error)
        .map_err(BackupError::Object)?
    {
        ConditionalOutcome::Published { .. } => true,
        ConditionalOutcome::PreconditionFailed | ConditionalOutcome::Indeterminate => {
            // Resolve by operation identity and manifest digest.
            match destination
                .read_head(&key)
                .map_err(backend_error)
                .map_err(BackupError::Object)?
            {
                HeadRead::Present { body, .. } if *body == *encoded => false,
                HeadRead::Present { .. } => return Err(BackupError::ConflictingOperation),
                HeadRead::Absent => return Err(BackupError::CompletionUnresolved),
            }
        }
    };
    Ok(BackupReport {
        manifest,
        manifest_digest,
        installed,
        objects_copied: objects,
        bytes_copied: bytes,
    })
}

/// Read one completed backup's manifest from the destination ONLY.
///
/// # Errors
/// An absent manifest is an incomplete backup, never listed as complete.
pub fn read_backup_manifest<Dst>(
    destination: &Dst,
    dest_prefix: &str,
    operation: OperationId,
) -> Result<(BackupManifest, [u8; 32]), BackupError>
where
    Dst: ConditionalStore,
    Dst::Error: BackendError,
{
    let key = backup_manifest_key(dest_prefix, operation);
    let bytes = match destination
        .read_head(&key)
        .map_err(backend_error)
        .map_err(BackupError::Object)?
    {
        HeadRead::Present { body, .. } => body,
        HeadRead::Absent => return Err(BackupError::Incomplete { key }),
    };
    let manifest = decode_backup_manifest(&bytes)?;
    if manifest.operation != operation {
        return Err(BackupError::ConflictingOperation);
    }
    Ok((manifest, *blake3::hash(&bytes).as_bytes()))
}

/// What a full verification pass established, from the destination only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyReport {
    pub manifest: BackupManifest,
    pub objects_verified: u64,
    pub bytes_verified: u64,
}

/// Verify one completed backup entirely from the destination: manifest,
/// checkpoint manifest, every chunk, every tail decision (including the
/// parent chain connecting tip back to base). "Copied bytes", "verified
/// complete backup" and "restored application state" remain three different
/// facts; this is the second.
///
/// # Errors
/// The first missing/mismatched object refuses with its key.
pub fn verify_backup<Dst>(
    destination: &Dst,
    dest_prefix: &str,
    operation: OperationId,
    limits: Limits,
    stream: StreamLimits,
    work: &WorkContext,
) -> Result<VerifyReport, BackupError>
where
    Dst: ConditionalStore,
    Dst::Error: BackendError,
{
    let (manifest, _) = read_backup_manifest(destination, dest_prefix, operation)?;
    let mut objects = 0u64;
    let mut bytes = 0u64;
    if let Some(reference) = manifest.checkpoint {
        let manifest_bytes = get_verified(destination, dest_prefix, &reference)?;
        let checkpoint_manifest = codec::decode_manifest(&manifest_bytes, stream)?;
        if checkpoint_manifest.identity != manifest.identity
            || checkpoint_manifest.decision != manifest.base
            || checkpoint_manifest.application_digest != manifest.application_digest
            || checkpoint_manifest.system_digest != manifest.system_digest
        {
            return Err(BackupError::Corrupt(
                "checkpoint disagrees with the backup manifest",
            ));
        }
        objects += 1;
        bytes += manifest_bytes.len() as u64;
        for chunk in &checkpoint_manifest.chunks {
            work.checkpoint()?;
            let chunk_bytes = get_verified(destination, dest_prefix, chunk)?;
            objects += 1;
            bytes += chunk_bytes.len() as u64;
        }
    }
    // The tail chain must connect tip back to base through exactly the
    // recorded decisions.
    let mut expected = manifest.tip;
    for copied in manifest.decisions.iter().rev() {
        work.checkpoint()?;
        if copied.digest != expected.hash {
            return Err(BackupError::Corrupt(
                "tail order disagrees with the tip chain",
            ));
        }
        let key = decision_key(dest_prefix, copied.epoch, &copied.digest);
        let body = match destination
            .get_object(&key)
            .map_err(backend_error)
            .map_err(BackupError::Object)?
        {
            crate::store::ObjectRead::Present { body } => body,
            crate::store::ObjectRead::Absent => return Err(BackupError::Incomplete { key }),
        };
        if body.len() as u64 != copied.length
            || object_digest(ObjectKind::Decision, &body) != *copied.digest.as_bytes()
        {
            return Err(BackupError::Incomplete { key });
        }
        let envelope = decision::decode_decision(&body, limits)
            .map_err(|_| BackupError::Corrupt("backed-up decision malformed"))?;
        if envelope.stamp() != expected {
            return Err(BackupError::Corrupt("backed-up decision stamp mismatch"));
        }
        objects += 1;
        bytes += body.len() as u64;
        expected = envelope.parent;
    }
    if expected != manifest.base {
        return Err(BackupError::Corrupt("tail chain does not reach the base"));
    }
    Ok(VerifyReport {
        manifest,
        objects_verified: objects,
        bytes_verified: bytes,
    })
}

/// The backed-up tail's decision bytes, oldest first, each fetched and
/// verified from the destination — the restore pipeline's input.
///
/// # Errors
/// Missing/mismatched decisions refuse.
pub fn read_backup_tail<Dst>(
    destination: &Dst,
    dest_prefix: &str,
    manifest: &BackupManifest,
    limits: Limits,
    work: &WorkContext,
) -> Result<Vec<Vec<u8>>, BackupError>
where
    Dst: ConditionalStore,
    Dst::Error: BackendError,
{
    let mut tail = Vec::with_capacity(manifest.decisions.len());
    for copied in &manifest.decisions {
        work.checkpoint()?;
        let key = decision_key(dest_prefix, copied.epoch, &copied.digest);
        let body = match destination
            .get_object(&key)
            .map_err(backend_error)
            .map_err(BackupError::Object)?
        {
            crate::store::ObjectRead::Present { body } => body,
            crate::store::ObjectRead::Absent => return Err(BackupError::Incomplete { key }),
        };
        if object_digest(ObjectKind::Decision, &body) != *copied.digest.as_bytes() {
            return Err(BackupError::Incomplete { key });
        }
        decision::decode_decision(&body, limits)
            .map_err(|_| BackupError::Corrupt("backed-up decision malformed"))?;
        tail.push(body.into_vec());
    }
    Ok(tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> DatabaseIdentity {
        DatabaseIdentity {
            database_id: DatabaseId::from_core(bumbledb::Id128::from_bytes([1; 16])),
            incarnation_id: IncarnationId::from_core(bumbledb::Id128::from_bytes([2; 16])),
            schema_id: SchemaId([3; 32]),
        }
    }

    #[test]
    fn backup_manifests_roundtrip_and_truncations_refuse() {
        let manifest = BackupManifest {
            operation: OperationId::from_core(bumbledb::Id128::from_bytes([7; 16])),
            identity: identity(),
            base: DecisionStamp {
                seq: 4,
                hash: DecisionDigest::from_bytes([9; 32]),
            },
            tip: DecisionStamp {
                seq: 6,
                hash: DecisionDigest::from_bytes([8; 32]),
            },
            state: StateStamp {
                incarnation: identity().incarnation_id,
                data_revision: 5,
            },
            checkpoint: Some(ObjectRef::of(2, ObjectKind::Checkpoint, b"m")),
            decisions: vec![
                CopiedDecision {
                    epoch: 2,
                    digest: DecisionDigest::from_bytes([5; 32]),
                    length: 100,
                },
                CopiedDecision {
                    epoch: 3,
                    digest: DecisionDigest::from_bytes([8; 32]),
                    length: 90,
                },
            ],
            application_digest: [4; 32],
            system_digest: [5; 32],
            blobs: BlobPolicy::DatabaseOnly,
        };
        let bytes = encode_backup_manifest(&manifest).unwrap();
        assert_eq!(decode_backup_manifest(&bytes).unwrap(), manifest);
        for end in 0..bytes.len() {
            assert!(decode_backup_manifest(&bytes[..end]).is_err());
        }
    }

    #[test]
    fn manifest_keys_are_operation_scoped_protocol_names() {
        let a = backup_manifest_key(
            "dest",
            OperationId::from_core(bumbledb::Id128::from_bytes([1; 16])),
        );
        let b = backup_manifest_key(
            "dest",
            OperationId::from_core(bumbledb::Id128::from_bytes([2; 16])),
        );
        assert_ne!(a, b);
        assert!(a.starts_with("dest/backup/"));
        assert!(crate::store::key_ok(&a));
    }
}
