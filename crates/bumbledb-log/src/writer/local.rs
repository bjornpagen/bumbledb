//! `LocalHistory`: one LMDB transaction is the authority.
//!
//! Facts, the retained terminal receipt and the head-authority attachment
//! commit atomically in the core's exclusive writer session. There is no
//! remote HEAD, object epoch, tail envelope or second command-body log — LMDB
//! already contains complete authoritative state, so reopening needs no replay
//! checkpoint. The linearization point is the durable LMDB commit.

use std::sync::Arc;

use bumbledb::integration::{AttachmentChange, HostChanges};
use bumbledb::{ChangeSet, Db, WorkContext};

use crate::history::admission::Submission;
use crate::history::authority::{Activation, ActivationCause, HeadAuthority, encode_control};
use crate::history::command::{Command, Limits};
use crate::history::decision::{self, GenesisProvenance, GenesisRecord};
use crate::history::receipt::{decode_receipt_row, receipt_key};
use crate::history::{
    CommandId, DatabaseId, DatabaseIdentity, IncarnationId, OperationId, SchemaId, TerminalReceipt,
};

use super::decide::{self, Judged, Plan, RealPrepared};
use super::{LocalHealth, LogError, ResolveOutcome, SubmitOutcome};

/// A `LocalHistory` authority over one owned core database.
pub struct LocalHistory<S> {
    db: Arc<Db<S>>,
    identity: DatabaseIdentity,
    limits: Limits,
}

impl<S> LocalHistory<S> {
    /// Create a new local incarnation: write the genesis authority attachment
    /// in one transaction over an empty database. Refuses a database that
    /// already carries a control attachment (open-or-create is not a thing).
    ///
    /// # Errors
    /// Refuses an already-initialized database, storage failures, exhausted
    /// work and frame limits.
    pub fn create(
        db: Arc<Db<S>>,
        database_id: DatabaseId,
        incarnation_id: IncarnationId,
        operation: OperationId,
        limits: Limits,
        work: &WorkContext,
    ) -> Result<Self, LogError> {
        let schema_id = fingerprint(&db);
        let identity = DatabaseIdentity {
            database_id,
            incarnation_id,
            schema_id,
        };
        if read_attachment(&db)?.is_some() {
            return Err(LogError::Corruption);
        }
        let (application, system) = decision::blank_initial_digests();
        let genesis_record = GenesisRecord {
            identity,
            initial_application_digest: application,
            initial_system_digest: system,
            provenance: GenesisProvenance::Create,
        };
        let genesis = decision::genesis_stamp(&genesis_record, limits.envelope_bytes)?;
        let authority = HeadAuthority::genesis(
            identity,
            genesis,
            Activation::Activated {
                operation,
                target_genesis: genesis.hash,
                cause: ActivationCause::Create,
            },
        )?;
        let control = encode_control(&authority, limits.envelope_bytes)?;
        let mut session = db.integration_writer(work)?;
        let empty = ChangeSet::builder(db.schema(), work.clone())
            .finish()
            .map_err(|error| LogError::Core(error.into()))?;
        let prepared = match session.prepare(&empty)? {
            bumbledb::Admission::Accepted(prepared) => prepared,
            bumbledb::Admission::Rejected(_) => return Err(LogError::Corruption),
        };
        let sealed = prepared.seal(HostChanges {
            records: &[],
            attachment: AttachmentChange::Put(&control),
        })?;
        sealed.commit()?;
        // The session's borrow of `db` must end before `db` moves into Self.
        drop(session);
        Ok(Self {
            db,
            identity,
            limits,
        })
    }

    /// Open an existing local incarnation by reading its committed authority.
    /// Never initializes a missing/uninitialized database.
    ///
    /// # Errors
    /// Refuses an uninitialized database, a foreign schema and corruption.
    pub fn open(db: Arc<Db<S>>, limits: Limits) -> Result<Self, LogError> {
        let control = read_attachment(&db)?.ok_or(LogError::NotInitialized)?;
        let authority = crate::history::authority::decode_control(&control, limits.envelope_bytes)?;
        if authority.identity.schema_id != fingerprint(&db) {
            return Err(LogError::Identity);
        }
        Ok(Self {
            db,
            identity: authority.identity,
            limits,
        })
    }

    #[must_use]
    pub const fn identity(&self) -> DatabaseIdentity {
        self.identity
    }

    #[must_use]
    pub fn db(&self) -> &Db<S> {
        &self.db
    }

    /// The current committed authority projection.
    ///
    /// # Errors
    /// Refuses corruption/uninitialized state.
    pub fn authority(&self) -> Result<HeadAuthority, LogError> {
        let control = read_attachment(&self.db)?.ok_or(LogError::NotInitialized)?;
        Ok(crate::history::authority::decode_control(
            &control,
            self.limits.envelope_bytes,
        )?)
    }

    /// Submit one owned sealed command. All four terminal outcomes commit in
    /// one LMDB transaction beside their receipt. A retry of an already
    /// decided command returns the retained receipt without re-executing.
    #[must_use]
    pub fn submit(&self, command: &Command, work: &WorkContext) -> SubmitOutcome {
        let reference = command.command_ref();
        match self.try_submit(command, work) {
            Ok(outcome) => outcome,
            // A returned error means this invocation dispatched no durable
            // decision: the LMDB transaction is definite, so there is no
            // hosted-style unknown outcome for local commit failure.
            Err(error) => SubmitOutcome::NotSubmitted {
                command: reference,
                error,
            },
        }
    }

    fn try_submit(&self, command: &Command, work: &WorkContext) -> Result<SubmitOutcome, LogError> {
        work.checkpoint()?;
        let reference = command.command_ref();
        if reference.identity != self.identity {
            return Err(LogError::Identity);
        }
        // Serialize through the exclusive writer, then read the exact parent
        // state it will build on. The write mutex prevents any intervening
        // commit, so this snapshot is the candidate's true predecessor.
        let mut session = self.db.integration_writer(work)?;
        let authority = self.authority()?;
        let retained = self.retained(reference.id, work)?;
        let view = authority
            .admission_view()
            .ok_or(LogError::DatabaseDeleted)?;
        let plan = match view.submit(reference, command.metadata().condition, retained.as_ref())? {
            Submission::AlreadyDecided(receipt) => {
                let at = authority.position().map(|position| position.decision);
                let local_health = match at {
                    Some(at) => LocalHealth::Ready { at },
                    None => LocalHealth::Unavailable {
                        error: LogError::DatabaseDeleted,
                    },
                };
                return Ok(SubmitOutcome::Decided {
                    receipt: receipt.clone(),
                    local_health,
                });
            }
            Submission::PreconditionFailed { expected, observed } => {
                Plan::PreconditionFailed { expected, observed }
            }
            Submission::Evaluate => Plan::Evaluate,
        };
        // The two-call decide dance: the session is a local here, so the
        // rejected arm's second prepare stays a fresh existential borrow
        // (see writer::decide's module docs).
        let schema = self.db.schema();
        let candidate = match plan {
            Plan::PreconditionFailed { expected, observed } => {
                let prepared = decide::prepare_empty(&mut session, schema, work)?;
                decide::seal_candidate(
                    prepared,
                    &authority,
                    command,
                    Judged::PreconditionFailed { expected, observed },
                    self.limits,
                )?
            }
            Plan::Evaluate => {
                match decide::prepare_real(&mut session, schema, command, self.limits, work)? {
                    RealPrepared::Admitted { prepared, judged } => {
                        decide::seal_candidate(prepared, &authority, command, judged, self.limits)?
                    }
                    RealPrepared::Rejected { evidence } => {
                        let prepared = decide::prepare_empty(&mut session, schema, work)?;
                        decide::seal_candidate(
                            prepared,
                            &authority,
                            command,
                            Judged::InvariantRejected { evidence },
                            self.limits,
                        )?
                    }
                }
            }
        };
        let receipt = candidate.receipt.clone();
        let at = receipt.decision_at;
        candidate.sealed.commit()?;
        Ok(SubmitOutcome::Decided {
            receipt,
            local_health: LocalHealth::Ready { at },
        })
    }

    /// Resolve a retained command reference against the current committed
    /// state. Receipt lookup precedes admission guards, so a Frozen/closed
    /// epoch still resolves a known receipt; a retired epoch refuses.
    ///
    /// # Errors
    /// Refuses foreign identity, deletion, digest conflict and corruption.
    pub fn resolve(
        &self,
        command: crate::history::CommandRef,
        work: &WorkContext,
    ) -> Result<ResolveOutcome, LogError> {
        work.checkpoint()?;
        if command.identity != self.identity {
            return Err(LogError::Identity);
        }
        let authority = self.authority()?;
        let retained = self.retained(command.id, work)?;
        let view = authority
            .admission_view()
            .ok_or(LogError::DatabaseDeleted)?;
        match view.resolve(command, retained.as_ref()) {
            Ok(crate::history::admission::Resolution::Found(receipt)) => {
                Ok(ResolveOutcome::Found(receipt.clone()))
            }
            Ok(crate::history::admission::Resolution::NotRecordedAt { decision_at }) => {
                Ok(ResolveOutcome::NotRecordedAt { decision_at })
            }
            Err(crate::history::admission::Refusal::CommandEpochClosed) => {
                Ok(ResolveOutcome::CommandEpochClosed)
            }
            Err(crate::history::admission::Refusal::ReceiptExpiredUnknown) => {
                Ok(ResolveOutcome::ReceiptExpiredUnknown)
            }
            Err(refusal) => Err(refusal.into()),
        }
    }

    fn retained(
        &self,
        id: CommandId,
        work: &WorkContext,
    ) -> Result<Option<TerminalReceipt>, LogError> {
        work.checkpoint()?;
        let reference = crate::history::CommandRef {
            identity: self.identity,
            id,
            // Digest is checked by the admission guard against the caller's
            // ref; the stored row carries its own recorded digest.
            digest: crate::history::CommandDigest::from_bytes([0; 32]),
        };
        let key = receipt_key(id);
        let row = read_host_record(&self.db, &key)?;
        match row {
            Some(bytes) => {
                let receipt = decode_receipt_row(reference, &bytes, self.limits)?;
                Ok(Some(receipt))
            }
            None => Ok(None),
        }
    }
}

fn fingerprint<S>(db: &Db<S>) -> SchemaId {
    bumbledb::schema::fingerprint::fingerprint(db.schema())
}

fn read_attachment<S>(db: &Db<S>) -> Result<Option<Vec<u8>>, LogError> {
    let mut owned = None;
    db.read(|read| {
        owned = read.integration_host_attachment()?.map(<[u8]>::to_vec);
        Ok(())
    })?;
    Ok(owned)
}

fn read_host_record<S>(db: &Db<S>, key: &[u8]) -> Result<Option<Vec<u8>>, LogError> {
    let mut owned = None;
    let mut host_error = None;
    db.read(|read| {
        match read.integration_host_record(key) {
            Ok(record) => owned = record.map(<[u8]>::to_vec),
            Err(error) => host_error = Some(error),
        }
        Ok(())
    })?;
    if let Some(error) = host_error {
        return Err(error.into());
    }
    Ok(owned)
}
