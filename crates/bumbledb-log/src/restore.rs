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
//! Read-only inspection retains original provenance and stamps and grants no
//! publication authority: no history machine is constructed and the wrapper
//! exposes reads only.

use std::path::Path;
use std::sync::Arc;

use bumbledb::integration::{AttachmentChange, HostChanges, HostRecordChange};
use bumbledb::schema::Theory;
use bumbledb::{ChangeSet, Db, WorkContext};

use crate::apply;
use crate::checkpointer::{CheckpointPolicy, capture_into};
use crate::codec::{self, CheckpointManifest, ChunkSink};
use crate::history::authority::{Activation, ActivationCause, HeadAuthority, encode_control};
use crate::history::command::Limits;
use crate::history::decision::{GenesisProvenance, GenesisRecord, genesis_stamp};
use crate::history::receipt::RECEIPT_KEY_PREFIX;
use crate::history::{DatabaseIdentity, DecisionStamp, IncarnationId, OperationId, StateStamp};
use crate::recovery::{
    BINDING_KEY, OriginBinding, RecoveryError, admit_collected, collect_checkpoint, encode_binding,
};
use crate::store::{ObjectKind, ObjectRef};

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

/// Restore a verified checkpoint stream into a NEW writable incarnation at a
/// fresh directory. `source_evidence` binds the backup/root manifest digest
/// into the genesis provenance.
///
/// # Errors
/// Rewind attempts, schema disagreement and every collection/admission
/// refusal; a failure leaves only owned scratch, never a half-restore.
#[expect(clippy::too_many_arguments, reason = "one bounded restore pipeline")]
pub fn restore_writable<S, E>(
    directory: &Path,
    schema: S,
    manifest: &CheckpointManifest,
    chunks: impl IntoIterator<Item = Result<Vec<u8>, E>>,
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
    S: Theory,
    E: Into<RecoveryError>,
{
    if new_incarnation == manifest.identity.incarnation_id {
        return Err(RestoreError::RewindRefused);
    }
    let state = collect_checkpoint(manifest, chunks, stream)?;
    // New incarnation: preserved migration-history evidence, EMPTY executable
    // receipt table, preserved application bytes.
    let mut kept: Vec<(&[u8], &[u8])> = Vec::new();
    for (key, value) in &state.system {
        if key.first() == Some(&RECEIPT_KEY_PREFIX) {
            continue;
        }
        kept.push((key, value));
    }
    let identity = DatabaseIdentity {
        database_id: manifest.identity.database_id,
        incarnation_id: new_incarnation,
        schema_id: manifest.identity.schema_id,
    };
    // The genesis binds the restored application digest and the system
    // digest of exactly the carried-forward records (acyclic: it excludes
    // its own stamp and the manifest hash that will carry it).
    let initial_system_digest = system_projection(&kept);
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
    let mut records: Vec<HostRecordChange<'_>> = Vec::new();
    records.push(HostRecordChange::Put {
        key: BINDING_KEY,
        value: &binding_bytes,
    });
    for (key, value) in &kept {
        records.push(HostRecordChange::Put { key, value });
    }
    // Restore validates the complete canonical theory and state through the
    // one judged admission — a valid checksum preserving a semantically
    // invalid export still refuses activation with evidence.
    let db = admit_collected(directory, schema, &state, &records, &control, work)?;
    let report = db
        .verify_store()
        .map_err(|error| RestoreError::Recovery(RecoveryError::Storage(error)))?;
    if !report.findings().is_empty() {
        return Err(RestoreError::Recovery(RecoveryError::Corrupt(
            "restored store failed verification",
        )));
    }
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
/// backed-up tip. The tail decisions replay through the historical evaluator
/// (current admission guards do not apply); the replayed receipt rows are
/// archival evidence and are dropped from the new incarnation's executable
/// table in the final genesis transaction, while migration-history evidence
/// carries forward. The genesis digests are recomputed from the reached
/// state's canonical projection, never copied from the base checkpoint.
///
/// # Errors
/// Rewind attempts, tip disagreement, replay verification and admission
/// refusals; a failure leaves only owned scratch, never a half-restore.
/// A capture sink that keeps digests only; restore recomputes projection
/// digests without persisting chunks.
struct Discard;
impl ChunkSink for Discard {
    type Error = crate::checkpointer::CheckpointError;
    fn chunk(&mut self, bytes: &[u8]) -> Result<ObjectRef, Self::Error> {
        Ok(ObjectRef::of(0, ObjectKind::Chunk, bytes))
    }
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one bounded restore pipeline"
)]
pub fn restore_writable_with_tail<S, E>(
    directory: &Path,
    schema: S,
    manifest: &CheckpointManifest,
    chunks: impl IntoIterator<Item = Result<Vec<u8>, E>>,
    tail: &[Vec<u8>],
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
    S: Theory,
    E: Into<RecoveryError>,
{
    if new_incarnation == manifest.identity.incarnation_id {
        return Err(RestoreError::RewindRefused);
    }
    // 1. Materialize the faithful old-lineage state at the base, with the
    //    captured control and every system row verbatim, then replay the tail.
    let state = collect_checkpoint(manifest, chunks, policy.stream)?;
    let control_at_base = encode_control(&manifest.control_at_capture, head_cap)
        .map_err(|error| RestoreError::Recovery(RecoveryError::Frame(error)))?;
    let mut records: Vec<HostRecordChange<'_>> = Vec::new();
    for (key, value) in &state.system {
        records.push(HostRecordChange::Put { key, value });
    }
    let db = admit_collected(directory, schema, &state, &records, &control_at_base, work)?;
    let mut authority = manifest.control_at_capture;
    for decision_bytes in tail {
        authority = apply::materialize(&db, &authority, decision_bytes, command_limits, work)
            .map_err(|error| RestoreError::Recovery(RecoveryError::Apply(error)))?;
    }
    let position = authority
        .position()
        .ok_or(RestoreError::Recovery(RecoveryError::Corrupt(
            "replayed into a tombstone",
        )))?;
    if position.decision != expected_tip {
        return Err(RestoreError::Recovery(RecoveryError::Corrupt(
            "replay did not reach the backed-up tip",
        )));
    }

    // 2. Recompute the reached state's canonical projection digests: the
    //    application digest at the tip, and the system digest over exactly
    //    the carried-forward (non-receipt) records. `u64::MAX` filters every
    //    receipt row out of the projection.
    let mut discard = Discard;
    let captured = capture_into(&db, &mut discard, u64::MAX, policy, work)
        .map_err(|_| RestoreError::Recovery(RecoveryError::Corrupt("projection capture failed")))?;

    // 3. The new incarnation's genesis over the reached state.
    let identity = DatabaseIdentity {
        database_id: manifest.identity.database_id,
        incarnation_id: new_incarnation,
        schema_id: manifest.identity.schema_id,
    };
    let genesis_record = GenesisRecord {
        identity,
        initial_application_digest: captured.summary.application_digest,
        initial_system_digest: captured.summary.system_digest,
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

    // 4. One final transaction: drop every archival receipt row (the new
    //    incarnation's executable table starts empty; old-scoped requests
    //    permanently refuse), install the binding and the genesis control.
    let receipt_keys = crate::admin::retired_row_keys(&db, u64::MAX)
        .map_err(|_| RestoreError::Recovery(RecoveryError::Corrupt("receipt scan failed")))?;
    let mut final_records: Vec<HostRecordChange<'_>> = receipt_keys
        .iter()
        .map(|key| HostRecordChange::Delete { key })
        .collect();
    final_records.push(HostRecordChange::Put {
        key: BINDING_KEY,
        value: &binding_bytes,
    });
    let empty = ChangeSet::builder(db.schema(), work.clone())
        .finish()
        .map_err(RecoveryError::Changes)
        .map_err(RestoreError::Recovery)?;
    let mut session = db
        .integration_writer(work)
        .map_err(RecoveryError::from)
        .map_err(RestoreError::Recovery)?;
    let prepared = match session
        .prepare(&empty)
        .map_err(RecoveryError::from)
        .map_err(RestoreError::Recovery)?
    {
        bumbledb::Admission::Accepted(prepared) => prepared,
        bumbledb::Admission::Rejected(_) => {
            return Err(RestoreError::Recovery(RecoveryError::InvariantViolation));
        }
    };
    prepared
        .seal(HostChanges {
            records: &final_records,
            attachment: AttachmentChange::Put(&new_control),
        })
        .map_err(RecoveryError::from)
        .map_err(RestoreError::Recovery)?
        .commit()
        .map_err(RecoveryError::from)
        .map_err(RestoreError::Recovery)?;
    drop(session);

    let report = db
        .verify_store()
        .map_err(|error| RestoreError::Recovery(RecoveryError::Storage(error)))?;
    if !report.findings().is_empty() {
        return Err(RestoreError::Recovery(RecoveryError::Corrupt(
            "restored store failed verification",
        )));
    }
    Ok(RestoredIncarnation {
        db,
        identity,
        authority: new_authority,
        source_decision: expected_tip,
        source_state: position.state,
    })
}

/// The system digest of exactly these carried-forward records, under the
/// shared projection domain — the same framing the stream writer uses.
fn system_projection(records: &[(&[u8], &[u8])]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(codec::SYSTEM_DOMAIN);
    for (key, value) in records {
        let mut head = Vec::with_capacity(11 + key.len());
        head.push(2); // TAG_SYSTEM
        head.extend_from_slice(&(u16::try_from(key.len()).unwrap_or(u16::MAX)).to_be_bytes());
        head.extend_from_slice(key);
        head.extend_from_slice(&(value.len() as u64).to_be_bytes());
        hasher.update(&head);
        hasher.update(value);
    }
    *hasher.finalize().as_bytes()
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
        f: impl FnOnce(&bumbledb::ReadInstance<'_, S>) -> bumbledb::Result<R>,
    ) -> bumbledb::Result<R> {
        self.db.read(f)
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
pub fn inspect<S, E>(
    scratch: &Path,
    schema: S,
    manifest: &CheckpointManifest,
    chunks: impl IntoIterator<Item = Result<Vec<u8>, E>>,
    stream: codec::StreamLimits,
    head_cap: usize,
    work: &WorkContext,
) -> Result<Inspection<S>, RestoreError>
where
    S: Theory,
    E: Into<RecoveryError>,
{
    let state = collect_checkpoint(manifest, chunks, stream)?;
    // Original provenance: the captured control at S, verbatim — evidence,
    // never new-admission authority (no history machine is built over it).
    let control = encode_control(&manifest.control_at_capture, head_cap)
        .map_err(|error| RestoreError::Recovery(RecoveryError::Frame(error)))?;
    let mut records: Vec<HostRecordChange<'_>> = Vec::new();
    for (key, value) in &state.system {
        records.push(HostRecordChange::Put { key, value });
    }
    let db = admit_collected(scratch, schema, &state, &records, &control, work)?;
    Ok(Inspection {
        db,
        manifest: manifest.clone(),
    })
}
