//! Apply and judge of a sealed change set, and bounded inspection. Both run
//! as a payload job over the change set's capability, so JavaScript never
//! holds the change-set bytes as authority.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use bumbledb::host::Judged;
use bumbledb::work::WorkContext;
use bumbledb::{ChangeSet, ErrorKind, WriteOutcome};

use crate::runtime::{Output, RuntimeError};

use super::{ApplyOutcome, ChangeCounts, DbInspection, Expected, JudgeOutcome, engine_error};

#[derive(Clone, Copy)]
pub(crate) enum WriteMode {
    Apply,
    Judge,
}

struct WriterFlag(Arc<crate::DbInner>);

impl Drop for WriterFlag {
    fn drop(&mut self) {
        self.0.writing.store(false, Ordering::Release);
    }
}

/// Apply (`Apply`) or only judge (`Judge`) `changes` against the current
/// state. A second concurrent writer refuses `WriterBusy`; an `expected`
/// state that moved is a `Moved` outcome, never an error.
pub(crate) fn decide_change_set(
    lease: &crate::runtime::owners::DbLease,
    changes: &ChangeSet,
    expected: Option<&Expected>,
    context: &WorkContext,
    mode: WriteMode,
) -> Result<Output, RuntimeError> {
    context.checkpoint()?;
    if let Some(expected) = expected
        && expected.database != lease.db().database_id()
    {
        return Err(RuntimeError::engine(
            ErrorKind::ForeignWitness,
            "the expected state belongs to another database",
        ));
    }
    if lease.writing.swap(true, Ordering::AcqRel) {
        return Err(RuntimeError::WriterBusy);
    }
    let _flag = WriterFlag(lease.inner_arc());
    let violations =
        |found: &bumbledb::Violations| crate::violations_out(&lease.schema.descriptor, found);
    match mode {
        WriteMode::Apply => {
            let outcome = match expected {
                Some(expected) => lease.db().apply_from(changes, &expected.witness, context),
                None => lease.db().apply(changes, context),
            }
            .map_err(|error| engine_error(&error))?;
            Ok(Output::Apply(match outcome {
                WriteOutcome::Committed(committed) => ApplyOutcome::Committed {
                    generation: committed.generation.value(),
                    changed: committed.changed,
                },
                WriteOutcome::Rejected(found) => ApplyOutcome::Rejected {
                    violations: violations(&found),
                },
                WriteOutcome::Moved { witnessed, current } => ApplyOutcome::Moved {
                    witnessed: witnessed.value(),
                    current: current.value(),
                },
            }))
        }
        WriteMode::Judge => {
            let mut session = lease
                .db()
                .host_writer(context)
                .map_err(|error| engine_error(&error))?;
            let generation = session
                .generation()
                .map_err(|error| engine_error(&error))?
                .value();
            if let Some(expected) = expected
                && expected.generation != generation
            {
                return Ok(Output::Judge(JudgeOutcome::Moved {
                    witnessed: expected.generation,
                    current: generation,
                }));
            }
            let judged = session
                .decide_all(std::slice::from_ref(changes))
                .map_err(|error| engine_error(&error))?
                .into_iter()
                .next()
                .ok_or(RuntimeError::Internal)?;
            Ok(Output::Judge(match judged {
                Judged::Accepted(applied) => JudgeOutcome::Admitted {
                    generation,
                    changes: ChangeCounts {
                        added: applied.added,
                        removed: applied.removed,
                    },
                },
                Judged::Rejected(found) => JudgeOutcome::Rejected {
                    generation,
                    violations: violations(&found),
                },
            }))
        }
    }
}

pub(crate) fn inspect_db(
    lease: &crate::runtime::owners::DbLease,
    owner_id: u64,
    database_id: u64,
    context: &WorkContext,
) -> Result<Output, RuntimeError> {
    context.checkpoint()?;
    let generation = lease
        .db()
        .generation(context.clone())
        .map_err(|error| engine_error(&error))?;
    let disk_bytes = lease
        .db()
        .disk_size()
        .map_err(|error| engine_error(&error))?;
    let retained = lease.runtime().database_operations(owner_id, database_id);
    Ok(Output::DbReport(DbInspection {
        generation: generation.value(),
        disk_bytes,
        retained_operations: retained,
    }))
}

/// The sealed change set held by a payload, read on its owning worker.
pub(crate) fn changes_from_payload(
    payload: &crate::runtime::registry::Payload,
) -> Result<super::ChangesOpened, RuntimeError> {
    let crate::runtime::registry::Payload::Changes {
        changes,
        schema,
        fingerprint,
    } = payload
    else {
        return Err(RuntimeError::Internal);
    };
    Ok(super::ChangesOpened {
        changes: changes.clone(),
        schema: Arc::clone(schema),
        fingerprint: fingerprint.clone(),
    })
}
