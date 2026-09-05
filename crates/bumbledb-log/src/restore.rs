//! Restore: read-only inspection or a new writable lineage — never a default
//! "rewind the old head" (chapter 22; RESTORE-01..03).
//!
//! A writable restore creates a new `IncarnationId` in a fresh directory:
//! its genesis binds the restored application digest and provenance; new
//! decision/state counters start at zero; the executable command-receipt
//! table starts EMPTY under the new incarnation (old receipts are archival
//! recovery evidence, and old-scoped requests permanently refuse), while
//! migration-history evidence records are copied forward. Application
//! values, including ordinary application-owned 128-bit entity IDs, are
//! preserved byte-for-byte: entity bytes encode no lineage and are not
//! command/witness credentials. Reusing a live incarnation refuses.
//!
//! Checkpoint state streams into the fresh store in bounded batches
//! ([`crate::recovery::import_stream`]); tip agreement, new-incarnation
//! genesis/binding, and bounded receipt cleanup happen on the private
//! unready owner. Receipt cleanup is L07 `delete_host_batch` / `HostResume`.
//! [`crate::recovery::StagedPopulation::complete_install`] is the one
//! no-clobber publication of that finished incarnation. A valid checksum
//! preserving a semantically invalid export still refuses activation with
//! evidence — never a whole-database RAM materialization (audit-log #5).
//!
//! Genesis-root backups (`checkpoint: None`, the whole chain as tail)
//! restore through [`restore_writable_genesis`]: the blank-genesis base
//! stays unready while the backed-up chain replays to the exact tip
//! (audit-log #4 — a tenant that never checkpointed has usable backups).
//!
//! Read-only inspection retains original provenance and stamps and grants no
//! publication authority: no history machine is constructed and the wrapper
//! exposes reads only.

use std::path::Path;
use std::sync::Arc;

use bumbledb::integration::HostRecordChange;
use bumbledb::schema::Theory;
use bumbledb::store::{HostResume, HostWindow};
use bumbledb::work::ChargedBytes;
use bumbledb::{Db, WorkContext};

use crate::checkpointer::{CheckpointPolicy, HISTORY_KEY_PREFIX};
use crate::codec::{self, CheckpointManifest};
use crate::history::authority::{Activation, ActivationCause, HeadAuthority, encode_control};
use crate::history::command::Limits;
use crate::history::decision::{GenesisProvenance, GenesisRecord, genesis_stamp};
use crate::history::receipt::RECEIPT_KEY_PREFIX;
use crate::history::{DatabaseIdentity, DecisionStamp, IncarnationId, OperationId, StateStamp};
use crate::recovery::{
    BINDING_KEY, OriginBinding, RecoveryError, StagedPopulation, apply_unready_decision,
    begin_staged, encode_binding, import_stream, open_published, settlement_failed,
};

#[derive(Debug)]
pub enum RestoreError {
    /// The requested incarnation equals the source's: a writable restore is
    /// a new lineage, never a rewind of a possibly-live authority.
    RewindRefused,
    /// The manifest's schema disagrees with the supplied theory.
    ForeignSchema,
    Recovery(RecoveryError),
}

impl From<RecoveryError> for RestoreError {
    fn from(error: RecoveryError) -> Self {
        Self::Recovery(error)
    }
}

impl From<crate::backup::BackupError> for RecoveryError {
    fn from(error: crate::backup::BackupError) -> Self {
        match error {
            crate::backup::BackupError::Object(error) => Self::Object(error),
            crate::backup::BackupError::Frame(error) => Self::Frame(error),
            crate::backup::BackupError::Work(error) => Self::Work(error),
            crate::backup::BackupError::Corrupt(msg) => Self::Corrupt(msg),
            crate::backup::BackupError::ConflictingOperation
            | crate::backup::BackupError::CompletionUnresolved
            | crate::backup::BackupError::Incomplete { .. } => {
                Self::Corrupt("relocated backup tail refused")
            }
        }
    }
}

/// A completed writable restore: the new incarnation's database, identity
/// and genesis authority. The caller wraps it in `LocalHistory`/hosted
/// creation under its own explicit operation.
pub struct RestoredIncarnation<S> {
    pub db: Arc<Db<S>>,
    pub identity: DatabaseIdentity,
    pub authority: HeadAuthority,
    /// The source's captured stamps, retained as provenance.
    pub source_decision: DecisionStamp,
    pub source_state: StateStamp,
}

/// The incremental system-record projection: exactly the stream writer's
/// keyed-record framing under the shared system domain, so a digest computed
/// while records stream equals the one an export of the same records yields.
struct SystemProjection {
    hasher: blake3::Hasher,
}

impl SystemProjection {
    fn new() -> Self {
        Self {
            hasher: blake3::Hasher::new_derive_key(codec::SYSTEM_DOMAIN),
        }
    }

    fn record(&mut self, key: &[u8], value: &[u8]) {
        let mut head = Vec::with_capacity(11 + key.len());
        head.push(2); // TAG_SYSTEM
        head.extend_from_slice(&(u16::try_from(key.len()).unwrap_or(u16::MAX)).to_be_bytes());
        head.extend_from_slice(key);
        head.extend_from_slice(&(value.len() as u64).to_be_bytes());
        self.hasher.update(&head);
        self.hasher.update(value);
    }

    fn finish(self) -> [u8; 32] {
        *self.hasher.finalize().as_bytes()
    }
}

/// Application-digest projection matching [`codec::StreamWriter::fact`]
/// framing (tag 1, relation, length, row) under the shared domain.
struct ApplicationProjection {
    hasher: blake3::Hasher,
}

impl ApplicationProjection {
    fn new() -> Self {
        Self {
            hasher: blake3::Hasher::new_derive_key(codec::APPLICATION_DOMAIN),
        }
    }

    fn fact(&mut self, relation: u32, row: &[u8]) {
        let mut head = [0u8; 5];
        head[0] = 1;
        head[1..5].copy_from_slice(&relation.to_be_bytes());
        self.hasher.update(&head);
        self.hasher.update(&(row.len() as u64).to_be_bytes());
        self.hasher.update(row);
    }

    fn finish(self) -> [u8; 32] {
        *self.hasher.finalize().as_bytes()
    }
}

/// One charged receipt-delete batch. Reused across flushes; peak working
/// storage does not grow with total receipt count.
const RECEIPT_CLEANUP_BATCH_BYTES: usize = 16 * 1024;

/// Restore a verified checkpoint stream into a NEW writable incarnation at a
/// fresh directory. `source_evidence` binds the backup/root manifest digest
/// into the genesis provenance.
///
/// # Errors
/// Rewind attempts, schema disagreement and every collection/admission
/// refusal; a failure leaves only owned scratch, never a half-restore.
#[expect(clippy::too_many_arguments, reason = "one bounded restore pipeline")]
pub fn restore_writable<S, E, B>(
    directory: &Path,
    schema: S,
    manifest: &CheckpointManifest,
    chunks: impl IntoIterator<Item = Result<B, E>>,
    new_incarnation: IncarnationId,
    operation: OperationId,
    source_evidence: [u8; 32],
    binding_origin: &str,
    binding_prefix: &str,
    stream: codec::StreamLimits,
    head_cap: usize,
    work: &WorkContext,
) -> Result<RestoredIncarnation<S>, RestoreError>
where
    S: Theory + Clone,
    E: Into<RecoveryError>,
    B: AsRef<[u8]>,
{
    if new_incarnation == manifest.identity.incarnation_id {
        return Err(RestoreError::RewindRefused);
    }
    // New incarnation: preserved migration-history evidence, EMPTY executable
    // receipt table, preserved application bytes. Receipt rows are filtered
    // out as the stream imports; the kept records are hashed incrementally in
    // exactly the stream's own framing.
    let staged = begin_staged(directory, schema.clone(), work).map_err(RestoreError::Recovery)?;
    let mut projection = SystemProjection::new();
    let mut keep = |key: &[u8], value: &[u8]| {
        if key.first() == Some(&RECEIPT_KEY_PREFIX) {
            return false;
        }
        projection.record(key, value);
        true
    };
    import_stream(&staged, manifest, chunks, &mut keep, None, stream, work)
        .map_err(RestoreError::Recovery)?;
    let identity = DatabaseIdentity {
        database_id: manifest.identity.database_id,
        incarnation_id: new_incarnation,
        schema_id: manifest.identity.schema_id,
    };
    // The genesis binds the restored application digest and the system
    // digest of exactly the carried-forward records (acyclic: it excludes
    // its own stamp and the manifest hash that will carry it).
    let initial_system_digest = projection.finish();
    let genesis_record = GenesisRecord {
        identity,
        initial_application_digest: manifest.application_digest,
        initial_system_digest,
        provenance: GenesisProvenance::Restore { source_evidence },
    };
    let genesis = genesis_stamp(&genesis_record, head_cap)
        .map_err(|error| RestoreError::Recovery(RecoveryError::Frame(error)))?;
    let authority = HeadAuthority::genesis(
        identity,
        genesis,
        Activation::Activated {
            operation,
            target_genesis: genesis.hash,
            cause: ActivationCause::Restore,
        },
    )
    .map_err(|_| RestoreError::Recovery(RecoveryError::Corrupt("genesis stamp invalid")))?;
    let control = encode_control(&authority, head_cap)
        .map_err(|error| RestoreError::Recovery(RecoveryError::Frame(error)))?;
    let binding = OriginBinding {
        origin: binding_origin.into(),
        prefix: binding_prefix.into(),
        identity,
    };
    let binding_bytes = encode_binding(&binding)
        .map_err(|error| RestoreError::Recovery(RecoveryError::Frame(error)))?;
    // Host records then the one complete judged install — a valid checksum
    // preserving a semantically invalid export still refuses activation.
    staged
        .write_host(
            &[HostRecordChange::Put {
                key: BINDING_KEY,
                value: &binding_bytes,
            }],
            Some(&control),
            work,
        )
        .map_err(RestoreError::Recovery)?;
    let dest = staged.destination().to_path_buf();
    staged
        .complete_install(work)
        .map_err(RestoreError::Recovery)?;
    let db = open_published(&dest, schema, work).map_err(RestoreError::Recovery)?;
    Ok(RestoredIncarnation {
        db,
        identity,
        authority,
        source_decision: manifest.decision,
        source_state: manifest.state,
    })
}

/// Restore a verified backup — checkpoint stream PLUS its exact bounded tail
/// — into a NEW writable incarnation at a fresh directory, faithful to the
/// backed-up tip. The tail applies as unjudged batches on the unready owner
/// (current admission guards do not apply; no `LawfulParent` from a prefix).
/// Tip agreement, new-incarnation genesis/binding, and bounded receipt
/// cleanup run on that unpublished owner; one
/// [`crate::recovery::StagedPopulation::complete_install`]
/// is the publication. Replayed receipt rows are archival evidence and
/// are dropped from the new incarnation's executable table in that genesis
/// write, while migration-history evidence carries forward. The genesis
/// digests are recomputed from the reached state's unready export, never
/// copied from the base checkpoint. `tail` yields [`ChargedBytes`]; replay
/// decodes under that owner and does not copy the decision into a detached
/// `Vec`.
///
/// # Errors
/// Rewind attempts, tip disagreement, replay verification and admission
/// refusals; a failure before publication leaves only owned scratch.
#[expect(clippy::too_many_arguments, reason = "one bounded restore pipeline")]
pub fn restore_writable_with_tail<S, E, T, B>(
    directory: &Path,
    schema: S,
    manifest: &CheckpointManifest,
    chunks: impl IntoIterator<Item = Result<B, E>>,
    tail: T,
    expected_tip: DecisionStamp,
    new_incarnation: IncarnationId,
    operation: OperationId,
    source_evidence: [u8; 32],
    binding_origin: &str,
    binding_prefix: &str,
    command_limits: Limits,
    policy: &CheckpointPolicy,
    head_cap: usize,
    work: &WorkContext,
) -> Result<RestoredIncarnation<S>, RestoreError>
where
    S: Theory + Clone,
    E: Into<RecoveryError>,
    T: IntoIterator<Item = Result<ChargedBytes, E>>,
    B: AsRef<[u8]>,
{
    if new_incarnation == manifest.identity.incarnation_id {
        return Err(RestoreError::RewindRefused);
    }
    // Populate the faithful old-lineage state on the unready owner, apply
    // the tail, then seal the new incarnation before the one publication.
    let staged = begin_staged(directory, schema.clone(), work).map_err(RestoreError::Recovery)?;
    let mut keep = |_key: &[u8], _value: &[u8]| true;
    import_stream(
        &staged,
        manifest,
        chunks,
        &mut keep,
        None,
        policy.stream,
        work,
    )
    .map_err(RestoreError::Recovery)?;
    let mut authority = manifest.control_at_capture;
    for decision in tail {
        let charged = decision.map_err(Into::into)?;
        authority = apply_unready_decision(
            &staged,
            &authority,
            charged.as_bytes(),
            command_limits,
            work,
        )
        .map_err(RestoreError::Recovery)?;
        drop(charged.into_owner());
    }
    seal_new_incarnation(
        staged,
        schema,
        &authority,
        expected_tip,
        manifest.identity,
        new_incarnation,
        operation,
        source_evidence,
        binding_origin,
        binding_prefix,
        head_cap,
        work,
    )
}

/// Restore a verified GENESIS-ROOT backup (`checkpoint: None` — a tenant
/// that never checkpointed; the whole decision chain is the tail) into a NEW
/// writable incarnation at a fresh directory. The blank-genesis base stays
/// unready under the same identity/incarnation discipline as
/// [`restore_writable_with_tail`] while the backed-up chain replays to the
/// exact tip; publication is one complete install of the sealed incarnation
/// (audit-log #4; BACKUP-01/RESTORE-01).
///
/// `genesis_base` is the backup manifest's `base` stamp (sequence zero);
/// `expected_tip` its `tip`; `tail` streams the backed-up decisions
/// oldest-first ([`crate::backup::relocated_tail`]).
///
/// # Errors
/// Rewind attempts, a non-genesis base, tip disagreement, replay
/// verification and admission refusals; a failure before publication leaves
/// only owned scratch.
#[expect(clippy::too_many_arguments, reason = "one bounded restore pipeline")]
pub fn restore_writable_genesis<S, E, T>(
    directory: &Path,
    schema: S,
    source_identity: DatabaseIdentity,
    genesis_base: DecisionStamp,
    tail: T,
    expected_tip: DecisionStamp,
    new_incarnation: IncarnationId,
    operation: OperationId,
    source_evidence: [u8; 32],
    binding_origin: &str,
    binding_prefix: &str,
    command_limits: Limits,
    _policy: &CheckpointPolicy,
    head_cap: usize,
    work: &WorkContext,
) -> Result<RestoredIncarnation<S>, RestoreError>
where
    S: Theory + Clone,
    E: Into<RecoveryError>,
    T: IntoIterator<Item = Result<ChargedBytes, E>>,
{
    if new_incarnation == source_identity.incarnation_id {
        return Err(RestoreError::RewindRefused);
    }
    // The faithful old-lineage blank genesis base stays unready (the same
    // blank population recovery::hydrate uses for a genesis root) until the
    // backed-up chain has been applied and the sealed incarnation publishes.
    // An empty nonempty-required prefix never becomes Ready.
    let base_authority =
        HeadAuthority::genesis(source_identity, genesis_base, Activation::NotActivated).map_err(
            |_| RestoreError::Recovery(RecoveryError::Corrupt("backup base is not a genesis stamp")),
        )?;
    let staged = begin_staged(directory, schema.clone(), work).map_err(RestoreError::Recovery)?;
    let mut authority = base_authority;
    for decision in tail {
        let charged = decision.map_err(Into::into)?;
        authority = apply_unready_decision(
            &staged,
            &authority,
            charged.as_bytes(),
            command_limits,
            work,
        )
        .map_err(RestoreError::Recovery)?;
        drop(charged.into_owner());
    }
    seal_new_incarnation(
        staged,
        schema,
        &authority,
        expected_tip,
        source_identity,
        new_incarnation,
        operation,
        source_evidence,
        binding_origin,
        binding_prefix,
        head_cap,
        work,
    )
}

/// Reached-tip check, export-order digests, binding, receipt cleanup, and
/// new-incarnation genesis on the unpublished owner; then one complete
/// install. A tip or metadata refusal drops the sibling and never publishes.
/// A rename that settles poorly keeps the destination (`SettlementFailed`).
#[expect(clippy::too_many_arguments, reason = "one bounded restore seal")]
fn seal_new_incarnation<S>(
    staged: StagedPopulation,
    schema: S,
    reached: &HeadAuthority,
    expected_tip: DecisionStamp,
    source_identity: DatabaseIdentity,
    new_incarnation: IncarnationId,
    operation: OperationId,
    source_evidence: [u8; 32],
    binding_origin: &str,
    binding_prefix: &str,
    head_cap: usize,
    work: &WorkContext,
) -> Result<RestoredIncarnation<S>, RestoreError>
where
    S: Theory + Clone,
{
    let position = reached
        .position()
        .ok_or(RestoreError::Recovery(RecoveryError::Corrupt(
            "replayed into a tombstone",
        )))?;
    if position.decision != expected_tip {
        return Err(RestoreError::Recovery(RecoveryError::Corrupt(
            "replay did not reach the backed-up tip",
        )));
    }

    let (application_digest, system_digest) = project_unready(&staged, work)?;

    let identity = DatabaseIdentity {
        database_id: source_identity.database_id,
        incarnation_id: new_incarnation,
        schema_id: source_identity.schema_id,
    };
    let genesis_record = GenesisRecord {
        identity,
        initial_application_digest: application_digest,
        initial_system_digest: system_digest,
        provenance: GenesisProvenance::Restore { source_evidence },
    };
    let genesis = genesis_stamp(&genesis_record, head_cap)
        .map_err(|error| RestoreError::Recovery(RecoveryError::Frame(error)))?;
    let new_authority = HeadAuthority::genesis(
        identity,
        genesis,
        Activation::Activated {
            operation,
            target_genesis: genesis.hash,
            cause: ActivationCause::Restore,
        },
    )
    .map_err(|_| RestoreError::Recovery(RecoveryError::Corrupt("genesis stamp invalid")))?;
    let new_control = encode_control(&new_authority, head_cap)
        .map_err(|error| RestoreError::Recovery(RecoveryError::Frame(error)))?;
    let binding = OriginBinding {
        origin: binding_origin.into(),
        prefix: binding_prefix.into(),
        identity,
    };
    let binding_bytes = encode_binding(&binding)
        .map_err(|error| RestoreError::Recovery(RecoveryError::Frame(error)))?;

    // Binding then genesis control first (`put_host` only). Receipt
    // cleanup is `delete_host_batch`; `m` history rows stay.
    staged
        .write_host(
            &[HostRecordChange::Put {
                key: BINDING_KEY,
                value: &binding_bytes,
            }],
            Some(&new_control),
            work,
        )
        .map_err(RestoreError::Recovery)?;
    delete_receipts_batched(&staged, work)?;
    let dest = staged.destination().to_path_buf();
    staged
        .complete_install(work)
        .map_err(RestoreError::Recovery)?;
    let db = open_published(&dest, schema, work).map_err(RestoreError::Recovery)?;
    Ok(RestoredIncarnation {
        db,
        identity,
        authority: new_authority,
        source_decision: expected_tip,
        source_state: position.state,
    })
}

/// Export-order application digest and non-receipt system digest on the
/// unpublished owner. Receipt keys are not collected. Same snapshot grammar
/// as a ready capture; not `apply::materialize`.
fn project_unready(
    staged: &StagedPopulation,
    work: &WorkContext,
) -> Result<([u8; 32], [u8; 32]), RestoreError> {
    staged
        .inspect(work, |reader, work| {
            let mut application = ApplicationProjection::new();
            reader.snapshot().export(work, &mut |relation, row| {
                application.fact(relation.0, row);
                Ok(())
            })?;
            let mut system = SystemProjection::new();
            reader.host_scan(&[HISTORY_KEY_PREFIX], work, &mut |key, value| {
                system.record(key, value);
                Ok(())
            })?;
            Ok((application.finish(), system.finish()))
        })
        .map_err(RestoreError::Recovery)
}

/// Delete `r` host rows through L07 [`StagedPopulation::delete_host_batch`].
/// Peak is one charged window; resume is the last key, not the remaining set.
fn delete_receipts_batched(
    staged: &StagedPopulation,
    work: &WorkContext,
) -> Result<(), RestoreError> {
    let cap = RECEIPT_CLEANUP_BATCH_BYTES as u64;
    let mut after: Option<HostResume> = None;
    loop {
        work.checkpoint().map_err(RecoveryError::Work)?;
        match staged
            .delete_host_batch(
                &[RECEIPT_KEY_PREFIX],
                after.as_ref().map(HostResume::as_key),
                work,
                cap,
            )
            .map_err(RestoreError::Recovery)?
        {
            HostWindow::Done { .. } => break,
            HostWindow::More { resume, .. } => after = Some(resume),
        }
    }
    Ok(())
}

/// Read-only inspection of a checkpoint stream: original provenance and
/// stamps retained, no publication authority granted and no write surface
/// exposed. An explicit attempt to reuse the live incarnation is simply not
/// representable — no authority record is created at all.
pub struct Inspection<S> {
    db: Arc<Db<S>>,
    pub manifest: CheckpointManifest,
}

impl<S> Inspection<S> {
    /// Bounded read access to the inspected state.
    ///
    /// # Errors
    /// The closure's storage refusals.
    pub fn read<R>(
        &self,
        work: WorkContext,
        f: impl FnOnce(&bumbledb::ReadFrame<'_, S>) -> bumbledb::Result<R>,
    ) -> bumbledb::Result<R> {
        self.db.read(work, f)
    }

    #[must_use]
    pub fn provenance(&self) -> (DecisionStamp, StateStamp) {
        (self.manifest.decision, self.manifest.state)
    }
}

/// Materialize a checkpoint stream into an inspection-only scratch
/// directory. The caller owns the scratch path's lifetime.
///
/// # Errors
/// Every collection/admission refusal.
pub fn inspect<S, E, B>(
    scratch: &Path,
    schema: S,
    manifest: &CheckpointManifest,
    chunks: impl IntoIterator<Item = Result<B, E>>,
    stream: codec::StreamLimits,
    head_cap: usize,
    work: &WorkContext,
) -> Result<Inspection<S>, RestoreError>
where
    S: Theory + Clone,
    E: Into<RecoveryError>,
    B: AsRef<[u8]>,
{
    let staged = begin_staged(scratch, schema.clone(), work).map_err(RestoreError::Recovery)?;
    let mut keep = |_key: &[u8], _value: &[u8]| true;
    import_stream(&staged, manifest, chunks, &mut keep, None, stream, work)
        .map_err(RestoreError::Recovery)?;
    // Original provenance: the captured control at S, verbatim — evidence,
    // never new-admission authority (no history machine is built over it).
    let control = encode_control(&manifest.control_at_capture, head_cap)
        .map_err(|error| RestoreError::Recovery(RecoveryError::Frame(error)))?;
    staged
        .write_host(&[], Some(&control), work)
        .map_err(RestoreError::Recovery)?;
    let dest = staged.destination().to_path_buf();
    staged
        .complete_install(work)
        .map_err(RestoreError::Recovery)?;
    let db = match Db::open(&dest, schema, work.clone()) {
        Ok(db) => Arc::new(db),
        Err(error) => {
            return Err(RestoreError::Recovery(settlement_failed(
                dest,
                bumbledb::store::StoreError::from(std::io::Error::other(error)),
            )));
        }
    };
    Ok(Inspection {
        db,
        manifest: manifest.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_import_tail_is_charged_bytes_not_a_detached_vec() {
        fn consume_tail<T, E>(tail: T)
        where
            T: IntoIterator<Item = Result<ChargedBytes, E>>,
            E: Into<RecoveryError>,
        {
            drop(tail);
        }
        consume_tail::<core::iter::Empty<Result<ChargedBytes, RecoveryError>>, RecoveryError>(
            core::iter::empty(),
        );
    }
}
