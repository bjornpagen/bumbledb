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

use crate::certainty::AdminCertainty;
use crate::codec::{self, StreamLimits};
use crate::history::command::Limits;
use crate::history::decision;
use crate::history::locator::ChainVisitor;
use crate::history::{
    DatabaseId, DatabaseIdentity, DecisionDigest, DecisionStamp, FrameError, IncarnationId,
    OperationId, SchemaId, StateStamp,
};
use crate::manifest::RecoveryRoot;
use crate::manifest::wire::{self, Reader};
use crate::store::{
    BackendError, ChargedBytes, ConditionalOutcome, ObjectError, ObjectKind, ObjectRef,
    ObservedError, ReceiveLimits, ReceivedHead, ReceivingStore, TransportContext,
    backend as backend_error, get_verified, hex32, read_head_bounded,
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
    Src: ReceivingStore,
    Src::Error: BackendError + ObservedError,
    Dst: ReceivingStore,
    Dst::Error: BackendError + ObservedError,
{
    work.checkpoint()?;
    let transport = TransportContext::new(work, ReceiveLimits::exact(reference.length));
    let bytes = get_verified(source, source_prefix, reference, transport)?;
    let key = reference.key(dest_prefix);
    // put_object is idempotent for identical bytes and refuses conflicts.
    destination
        .put_object(&key, bytes.as_bytes())
        .map_err(backend_error)
        .map_err(BackupError::Object)?;
    // Verified bytes: read back from the destination, not trust in the PUT.
    let copied = get_verified(
        destination,
        dest_prefix,
        reference,
        TransportContext::new(work, ReceiveLimits::exact(reference.length)),
    )
    .map_err(|_| BackupError::Incomplete { key: key.clone() })?;
    if copied.as_bytes() != bytes.as_bytes() {
        return Err(BackupError::Incomplete { key });
    }
    let length = bytes.len() as u64;
    drop(copied.into_owner());
    drop(bytes.into_owner());
    Ok(length)
}

/// Copy one decision object by authenticated locator and verify it from the
/// destination.
fn copy_decision_by_ref<Src, Dst>(
    source: &Src,
    source_prefix: &str,
    destination: &Dst,
    dest_prefix: &str,
    reference: &ObjectRef,
    work: &WorkContext,
) -> Result<CopiedDecision, BackupError>
where
    Src: ReceivingStore,
    Src::Error: BackendError + ObservedError,
    Dst: ReceivingStore,
    Dst::Error: BackendError + ObservedError,
{
    work.checkpoint()?;
    let transport = TransportContext::new(work, ReceiveLimits::exact(reference.length));
    let bytes = get_verified(source, source_prefix, reference, transport)?;
    let key = reference.key(dest_prefix);
    destination
        .put_object(&key, bytes.as_bytes())
        .map_err(backend_error)
        .map_err(BackupError::Object)?;
    let copied = get_verified(
        destination,
        dest_prefix,
        reference,
        TransportContext::new(work, ReceiveLimits::exact(reference.length)),
    )
    .map_err(|_| BackupError::Incomplete { key: key.clone() })?;
    if copied.as_bytes() != bytes.as_bytes() {
        return Err(BackupError::Incomplete { key });
    }
    drop(copied.into_owner());
    drop(bytes.into_owner());
    Ok(CopiedDecision {
        epoch: reference.epoch,
        digest: DecisionDigest::from_bytes(reference.digest),
        length: reference.length,
    })
}

/// Stream-copy one tail decision while walking. Newest-first; the caller
/// reverses metadata after the walk.
struct CopyTail<'a, Src, Dst> {
    source: std::marker::PhantomData<&'a Src>,
    destination: &'a Dst,
    dest_prefix: &'a str,
    work: &'a WorkContext,
    objects: &'a mut u64,
    bytes: &'a mut u64,
    decisions: &'a mut Vec<CopiedDecision>,
}

impl<Src, Dst> ChainVisitor for CopyTail<'_, Src, Dst>
where
    Src: ReceivingStore,
    Src::Error: BackendError + ObservedError,
    Dst: ReceivingStore,
    Dst::Error: BackendError + ObservedError,
{
    type Error = ObjectError;

    fn visit(
        &mut self,
        _stamp: crate::history::DecisionStamp,
        bytes: &[u8],
        reference: ObjectRef,
    ) -> Result<bool, ObjectError> {
        let key = reference.key(self.dest_prefix);
        self.destination
            .put_object(&key, bytes)
            .map_err(crate::store::backend)?;
        let copied = get_verified(
            self.destination,
            self.dest_prefix,
            &reference,
            TransportContext::new(self.work, ReceiveLimits::exact(reference.length)),
        )
        .map_err(|_| ObjectError::Missing { key: key.clone() })?;
        if copied.as_bytes() != bytes {
            return Err(ObjectError::WrongDigest { key });
        }
        drop(copied.into_owner());
        *self.bytes += reference.length;
        *self.objects += 1;
        self.decisions.push(CopiedDecision {
            epoch: reference.epoch,
            digest: crate::history::DecisionDigest::from_bytes(reference.digest),
            length: reference.length,
        });
        Ok(true)
    }
}

/// Stream one recovery root's complete declared dependency closure into the
/// destination, verify every copied object, then install the complete
/// manifest LAST by no-overwrite conditional create. Bounded: one object in
/// memory at a time, never a full-RAM materialization.
///
/// The caller must already hold this root against GC — a named restore point
/// for a hosted source ([`crate::admin::add_named_root_hosted`]); local
/// points are self-contained directories that need no pin. The public
/// hosted operation is [`backup_pinned_hosted`], which acquires and
/// releases the pin around this copy.
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
    Src: ReceivingStore,
    Src::Error: BackendError + ObservedError,
    Dst: ReceivingStore,
    Dst::Error: BackendError + ObservedError,
{
    let mut objects = 0u64;
    let mut bytes = 0u64;
    // 1. The checkpoint manifest and every chunk, each verified from the
    //    destination after the copy.
    let (checkpoint, application_digest, system_digest) = match root.checkpoint {
        Some(reference) => {
            let charged = get_verified(
                source,
                source_prefix,
                &reference,
                TransportContext::new(work, ReceiveLimits::exact(reference.length)),
            )?;
            let checkpoint_manifest = codec::decode_manifest(charged.as_bytes(), stream)?;
            drop(charged.into_owner());
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
    // 2. The exact bounded tail (base, tip], copied newest-first then
    //    reversed so the manifest records oldest-first. One object at a
    //    time — no whole-tail body Vec.
    let mut decisions = Vec::new();
    if root.tip != root.base {
        let tip_object = root.tip_object.ok_or(BackupError::Corrupt(
            "suffix root missing tip ObjectRef",
        ))?;
        let mut walk_budget = root.tail_count().saturating_add(8);
        let mut copier = CopyTail {
            source: std::marker::PhantomData,
            destination,
            dest_prefix,
            work,
            objects: &mut objects,
            bytes: &mut bytes,
            decisions: &mut decisions,
        };
        crate::history::locator::walk_decision_chain(
            source,
            source_prefix,
            root.tip,
            root.base,
            Some(tip_object),
            limits,
            &mut walk_budget,
            work,
            &mut copier,
        )
        .map_err(BackupError::Object)?;
        decisions.reverse();
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
            match read_head_bounded(
                destination,
                &key,
                TransportContext {
                    work: Some(work),
                    receive: ReceiveLimits::capped(MANIFEST_CAP as u64),
                },
            )
            .map_err(BackupError::Object)?
            {
                ReceivedHead::Present { body, .. } if body.as_bytes() == encoded => false,
                ReceivedHead::Present { .. } => return Err(BackupError::ConflictingOperation),
                ReceivedHead::Absent => return Err(BackupError::CompletionUnresolved),
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

/// A pin-scoped backup refusal: the pin (named-root) machinery's or the
/// copy/verify pipeline's.
#[derive(Debug)]
pub enum PinnedBackupError {
    Admin(crate::admin::AdminError),
    Backup(BackupError),
}

impl From<crate::admin::AdminError> for PinnedBackupError {
    fn from(error: crate::admin::AdminError) -> Self {
        Self::Admin(error)
    }
}
impl From<BackupError> for PinnedBackupError {
    fn from(error: BackupError) -> Self {
        Self::Backup(error)
    }
}

/// What one completed pin-scoped backup established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedBackupReport {
    pub report: BackupReport,
    /// The full destination-only verification pass that gated release.
    pub objects_verified: u64,
    pub bytes_verified: u64,
    /// The operation-scoped named restore point the copy ran under.
    pub root: crate::manifest::NamedRoot,
    /// Whether the pin was released after verification (the caller's
    /// policy); `false` leaves the named restore point held.
    pub released: bool,
}

/// The complete hosted backup operation (chapter 22 backup steps 1–4;
/// BACKUP-01, audit-log #3): acquire an OPERATION-SCOPED named restore point
/// against the exact current head — the durable pin that keeps the captured
/// closure protected while checkpoints advance the recovery base and GC
/// barriers/sweeps run — copy the PINNED closure into the destination,
/// verify the completed backup entirely from the destination, then release
/// the pin per `release_pin`.
///
/// Evidence-idempotent under `operation`: a retry recognizes the held pin
/// (same root ID) and resolves an already-installed manifest by digest. A
/// failed copy or verification returns with the pin still held, so the
/// retry copies a still-protected closure; the pin is released only after
/// the destination-only verification passes.
///
/// Local (`LocalHistory`) sources need no head pin: a local named restore
/// point ([`crate::local_roots::create_restore_point`]) is already a
/// complete self-contained directory.
///
/// # Errors
/// Pin registration (capacity refusals discard nothing), copy/verify and
/// release refusals; an incomplete operation is never listed as a backup.
#[expect(
    clippy::too_many_arguments,
    reason = "one bounded pin-copy-verify-release pipeline"
)]
pub fn backup_pinned_hosted<Src, Dst>(
    source: &Src,
    source_prefix: &str,
    destination: &Dst,
    dest_prefix: &str,
    operation: OperationId,
    label: &str,
    release_pin: bool,
    limits: Limits,
    stream: StreamLimits,
    root_policy: &crate::manifest::RootPolicy,
    head_cap: usize,
    work: &WorkContext,
) -> Result<PinnedBackupReport, PinnedBackupError>
where
    Src: ReceivingStore,
    Src::Error: BackendError + ObservedError,
    Dst: ReceivingStore,
    Dst::Error: BackendError + ObservedError,
{
    // 1. The operation-scoped pin: a named restore point registered against
    //    the exact current head (retries recognize the recorded root).
    let root = match crate::admin::add_named_root_hosted(
        source,
        source_prefix,
        operation,
        crate::manifest::RootKind::RestorePoint,
        label,
        operation,
        root_policy,
        head_cap,
        work,
    ) {
        AdminCertainty::Completed { value } => value,
        AdminCertainty::NotStarted { error } | AdminCertainty::OutcomeUnknown { error } => {
            return Err(PinnedBackupError::Admin(error));
        }
    };
    // The head names the identity and the epoch ceiling for tail-decision
    // probes; every object of the pinned closure lives at an epoch at or
    // below the current one, and the pin keeps it protected from here on.
    let (head, _) = crate::checkpointer::read_live_head(source, source_prefix, head_cap, work)
        .map_err(crate::admin::AdminError::from)?;
    // 2. Copy the PINNED closure — not the moving live recovery root.
    let report = backup_root(
        source,
        source_prefix,
        destination,
        dest_prefix,
        head.control.identity,
        root.state,
        &root.recovery,
        head.object_epoch,
        operation,
        limits,
        stream,
        work,
    )?;
    // 3. Verify the completed backup from the destination only.
    let verified = verify_backup(destination, dest_prefix, operation, limits, stream, work)?;
    // 4. Release the pin per policy — only after verification passed.
    if release_pin {
        match crate::admin::release_named_root_hosted(
            source,
            source_prefix,
            operation,
            true,
            head_cap,
            work,
        ) {
            AdminCertainty::Completed { .. } => {}
            AdminCertainty::NotStarted { error } | AdminCertainty::OutcomeUnknown { error } => {
                return Err(PinnedBackupError::Admin(error));
            }
        }
    }
    Ok(PinnedBackupReport {
        report,
        objects_verified: verified.objects_verified,
        bytes_verified: verified.bytes_verified,
        root,
        released: release_pin,
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
    work: &WorkContext,
) -> Result<(BackupManifest, [u8; 32]), BackupError>
where
    Dst: ReceivingStore,
    Dst::Error: BackendError + ObservedError,
{
    work.checkpoint()?;
    let key = backup_manifest_key(dest_prefix, operation);
    let charged = match read_head_bounded(
        destination,
        &key,
        TransportContext::new(work, ReceiveLimits::capped(MANIFEST_CAP as u64)),
    )
    .map_err(BackupError::Object)?
    {
        ReceivedHead::Present { body, .. } => body,
        ReceivedHead::Absent => return Err(BackupError::Incomplete { key }),
    };
    let manifest = decode_backup_manifest(charged.as_bytes())?;
    let digest = *blake3::hash(charged.as_bytes()).as_bytes();
    drop(charged);
    if manifest.operation != operation {
        return Err(BackupError::ConflictingOperation);
    }
    Ok((manifest, digest))
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
    Dst: ReceivingStore,
    Dst::Error: BackendError + ObservedError,
{
    let (manifest, _) = read_backup_manifest(destination, dest_prefix, operation, work)?;
    let mut objects = 0u64;
    let mut bytes = 0u64;
    if let Some(reference) = manifest.checkpoint {
        let charged = get_verified(
            destination,
            dest_prefix,
            &reference,
            TransportContext::new(work, ReceiveLimits::exact(reference.length)),
        )?;
        let checkpoint_manifest = codec::decode_manifest(charged.as_bytes(), stream)?;
        let manifest_len = charged.len() as u64;
        drop(charged.into_owner());
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
        bytes += manifest_len;
        for chunk in &checkpoint_manifest.chunks {
            work.checkpoint()?;
            let chunk_bytes = get_verified(
                destination,
                dest_prefix,
                chunk,
                TransportContext::new(work, ReceiveLimits::exact(chunk.length)),
            )?;
            objects += 1;
            bytes += chunk_bytes.len() as u64;
            drop(chunk_bytes.into_owner());
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
        let reference = ObjectRef {
            epoch: copied.epoch,
            kind: ObjectKind::Decision,
            digest: *copied.digest.as_bytes(),
            length: copied.length,
        };
        let body = get_verified(
            destination,
            dest_prefix,
            &reference,
            TransportContext::new(work, ReceiveLimits::exact(copied.length)),
        )
        .map_err(|_| BackupError::Incomplete {
            key: reference.key(dest_prefix),
        })?;
        let envelope = decision::decode_decision(body.as_bytes(), limits)
            .map_err(|_| BackupError::Corrupt("backed-up decision malformed"))?;
        if envelope.stamp() != expected {
            return Err(BackupError::Corrupt("backed-up decision stamp mismatch"));
        }
        objects += 1;
        bytes += body.len() as u64;
        expected = envelope.parent;
        drop(body.into_owner());
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

/// Streaming iterator over a backup's relocated decision bodies, oldest
/// first. Consumes the manifest's ordered relocated refs and verifies
/// unchanged historical decision commitments. Does not rewrite parent
/// bytes or follow source-location refs (C6).
pub struct RelocatedTail<'a, Dst> {
    destination: &'a Dst,
    dest_prefix: &'a str,
    remaining: std::slice::Iter<'a, CopiedDecision>,
    expected: crate::history::DecisionStamp,
    limits: Limits,
    work: &'a WorkContext,
    done: bool,
}

/// Walk the destination-only relocated tail. One decision body is live
/// at a time.
#[must_use]
pub fn relocated_tail<'a, Dst>(
    destination: &'a Dst,
    dest_prefix: &'a str,
    manifest: &'a BackupManifest,
    limits: Limits,
    work: &'a WorkContext,
) -> RelocatedTail<'a, Dst> {
    RelocatedTail {
        destination,
        dest_prefix,
        remaining: manifest.decisions.iter(),
        expected: manifest.base,
        limits,
        work,
        done: false,
    }
}

impl<Dst> Iterator for RelocatedTail<'_, Dst>
where
    Dst: ReceivingStore,
    Dst::Error: BackendError + ObservedError,
{
    type Item = Result<ChargedBytes, BackupError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let Some(copied) = self.remaining.next() else {
            self.done = true;
            return None;
        };
        if let Err(error) = self.work.checkpoint() {
            self.done = true;
            return Some(Err(BackupError::Work(error)));
        }
        let reference = ObjectRef {
            epoch: copied.epoch,
            kind: ObjectKind::Decision,
            digest: *copied.digest.as_bytes(),
            length: copied.length,
        };
        let body = match get_verified(
            self.destination,
            self.dest_prefix,
            &reference,
            TransportContext::new(self.work, ReceiveLimits::exact(copied.length)),
        ) {
            Ok(body) => body,
            Err(error) => {
                self.done = true;
                return Some(Err(BackupError::Object(error)));
            }
        };
        let envelope = match decision::decode_decision(body.as_bytes(), self.limits) {
            Ok(envelope) => envelope,
            Err(_) => {
                self.done = true;
                return Some(Err(BackupError::Corrupt("backed-up decision malformed")));
            }
        };
        if envelope.parent != self.expected {
            self.done = true;
            return Some(Err(BackupError::Corrupt(
                "relocated decision parent commitment mismatch",
            )));
        }
        if envelope.stamp().hash != copied.digest {
            self.done = true;
            return Some(Err(BackupError::Corrupt(
                "backed-up decision stamp mismatch",
            )));
        }
        // Advance along the recorded parent-stamp chain — never follow a
        // source-location ObjectRef. The charged owner is the item.
        self.expected = envelope.stamp();
        Some(Ok(body))
    }
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
