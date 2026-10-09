//! Apply and judge of a sealed change set, and bounded inspection. Both run
//! as a payload job over the change set's capability, so JavaScript never
//! holds the change-set bytes as authority.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use bumbledb::ChangeSet;
use bumbledb::work::WorkContext;

use crate::runtime::{Output, RuntimeError};

use super::{
    ApplyOutcome, ChangeCounts, DbInspection, Expected, JudgeOutcome, WitnessOut, change_error,
    engine_error,
};

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

fn counts(application: &bumbledb::integration::ApplicationChanges) -> ChangeCounts {
    ChangeCounts {
        added: application.added,
        removed: application.removed,
    }
}

/// Judge `changes` as a private candidate against the current state, then
/// commit it (`Apply`) or abort it (`Judge`). A second concurrent writer
/// refuses `WriterBusy`; an `expected` state that moved is a `Moved` outcome.
pub(crate) fn decide_change_set(
    lease: &crate::runtime::owners::DbLease,
    changes: &ChangeSet,
    expected: Option<&Expected>,
    context: &WorkContext,
    mode: WriteMode,
) -> Result<Output, RuntimeError> {
    context.checkpoint()?;
    let store = lease.db().integration_store().identity().store.to_string();
    let witness = |generation| WitnessOut {
        store: store.clone(),
        generation,
    };
    if lease.writing.swap(true, Ordering::AcqRel) {
        return Err(RuntimeError::WriterBusy);
    }
    let _flag = WriterFlag(lease.inner_arc());
    let mut session = lease
        .db()
        .integration_writer(context)
        .map_err(integration_error)?;
    let base = session.generation().map_err(integration_error)?.value();
    if let Some(expected) = expected {
        if expected.store != store {
            return Err(RuntimeError::Engine {
                kind: crate::tags::error_family::FOREIGN_WITNESS.into(),
                message: "expected-state witness names a different store".into(),
            });
        }
        if base != expected.generation {
            let (witnessed, current) = (witness(expected.generation), witness(base));
            return Ok(match mode {
                WriteMode::Apply => Output::Apply(ApplyOutcome::Moved { witnessed, current }),
                WriteMode::Judge => Output::Judge(JudgeOutcome::Moved { witnessed, current }),
            });
        }
    }
    match session.prepare(changes).map_err(integration_error)? {
        bumbledb::integration::Preparation::Rejected {
            violations,
            application,
        } => {
            let violations = crate::violations_out(&lease.schema.descriptor, &violations);
            Ok(match mode {
                WriteMode::Apply => Output::Apply(ApplyOutcome::Rejected { violations }),
                WriteMode::Judge => Output::Judge(JudgeOutcome::Rejected {
                    base: witness(base),
                    changes: counts(&application),
                    violations,
                }),
            })
        }
        bumbledb::integration::Preparation::Accepted(prepared) => {
            if matches!(mode, WriteMode::Judge) {
                let application = prepared.application_changes();
                prepared.abort();
                context.checkpoint()?;
                return Ok(Output::Judge(JudgeOutcome::Admitted {
                    base: witness(base),
                    changes: counts(&application),
                }));
            }
            let sealed = prepared
                .seal(bumbledb::integration::HostChanges {
                    records: &[],
                    attachment: bumbledb::integration::AttachmentChange::Keep,
                })
                .map_err(integration_error)?;
            let commit = sealed.commit().map_err(integration_error)?;
            let witness = witness(commit.generation.value());
            Ok(Output::Apply(if commit.changed {
                ApplyOutcome::Committed { witness }
            } else {
                ApplyOutcome::NoChange { witness }
            }))
        }
    }
}

pub(crate) fn integration_error(error: bumbledb::integration::IntegrationError) -> RuntimeError {
    use bumbledb::integration::IntegrationError;
    match error {
        IntegrationError::Core(error) => engine_error(&error),
        IntegrationError::Changes(error) => change_error(&error),
        IntegrationError::Host(error) => RuntimeError::Engine {
            kind: "hostSeal".into(),
            message: format!("{error:?}"),
        },
        IntegrationError::Work(error) => error.into(),
        IntegrationError::ForeignSchema => RuntimeError::Engine {
            kind: crate::tags::error_family::SCHEMA_MISMATCH.into(),
            message: "the ChangeSet's schema is not this database's schema".into(),
        },
        IntegrationError::ReentrantWriter => RuntimeError::WriterBusy,
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
