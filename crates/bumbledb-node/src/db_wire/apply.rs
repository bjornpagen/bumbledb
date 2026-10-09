//! One immutable final-state apply and bounded inspection.
//! Apply runs as a payload job over the sealed `ChangeSet` capability so the
//! JS thread never holds the change-set bytes as authority.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use bumbledb::ChangeSet;
use bumbledb::work::WorkContext;

use crate::runtime::{Output, RuntimeError};

use super::{
    ApplyOutcomeOwned, DbInspectionOwned, ExpectedOwned, JudgeOutcomeOwned, change_error,
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

/// Exclusive-writer admission by refusal (`WriterBusy`), witness comparison
/// as a domain outcome (`moved`), complete final-state judgment, one
/// durable commit. A foreign `ChangeSet` refuses typed.
pub(crate) fn apply_change_set(
    lease: &crate::runtime::owners::DbLease,
    changes: &ChangeSet,
    expected: &ExpectedOwned,
    context: &WorkContext,
) -> Result<Output, RuntimeError> {
    decide_change_set(lease, changes, expected, context, WriteMode::Apply)
}

/// Judge the same private candidate as apply, then abort it on this worker.
/// Neither the exclusive session nor a commit capability escapes to JS.
pub(crate) fn judge_change_set(
    lease: &crate::runtime::owners::DbLease,
    changes: &ChangeSet,
    expected: &ExpectedOwned,
    context: &WorkContext,
) -> Result<Output, RuntimeError> {
    decide_change_set(lease, changes, expected, context, WriteMode::Judge)
}

fn decide_change_set(
    lease: &crate::runtime::owners::DbLease,
    changes: &ChangeSet,
    expected: &ExpectedOwned,
    context: &WorkContext,
    mode: WriteMode,
) -> Result<Output, RuntimeError> {
    context.checkpoint()?;
    let store_hex = lease.db().integration_store().identity().store.to_string();
    if lease.writing.swap(true, Ordering::AcqRel) {
        return Err(RuntimeError::WriterBusy);
    }
    let _flag = WriterFlag(lease.inner_arc());
    let mut session = lease
        .db()
        .integration_writer(context)
        .map_err(integration_error)?;
    let base = session.generation().map_err(integration_error)?.value();
    if let ExpectedOwned::Exact { store, generation } = expected {
        if *store != store_hex {
            return Err(RuntimeError::Engine {
                diagnostic: None,
                kind: crate::tags::error_family::FOREIGN_WITNESS,
                message: "expected-state witness names a different store".into(),
            });
        }
        if base != *generation {
            return Ok(match mode {
                WriteMode::Apply => Output::Apply(ApplyOutcomeOwned::Moved {
                    store: store_hex,
                    witnessed: *generation,
                    current: base,
                }),
                WriteMode::Judge => Output::Judge(JudgeOutcomeOwned::Moved {
                    store: store_hex,
                    witnessed: *generation,
                    current: base,
                }),
            });
        }
    }
    match session.prepare(changes).map_err(integration_error)? {
        bumbledb::integration::Preparation::Rejected {
            violations,
            application,
        } => {
            let violations = crate::violations_wire(&lease.sealed.descriptor, &violations);
            Ok(match mode {
                WriteMode::Apply => Output::Apply(ApplyOutcomeOwned::Rejected(violations)),
                WriteMode::Judge => Output::Judge(JudgeOutcomeOwned::Rejected {
                    store: store_hex,
                    generation: base,
                    application,
                    violations,
                }),
            })
        }
        bumbledb::integration::Preparation::Accepted(prepared) => {
            if matches!(mode, WriteMode::Judge) {
                let application = prepared.application_changes();
                prepared.abort();
                context.checkpoint()?;
                return Ok(Output::Judge(JudgeOutcomeOwned::Admitted {
                    store: store_hex,
                    generation: base,
                    application,
                }));
            }
            let sealed = prepared
                .seal(bumbledb::integration::HostChanges {
                    records: &[],
                    attachment: bumbledb::integration::AttachmentChange::Keep,
                })
                .map_err(integration_error)?;
            let commit = sealed.commit().map_err(integration_error)?;
            let outcome = if commit.changed {
                ApplyOutcomeOwned::Accepted {
                    store: store_hex,
                    generation: commit.generation.value(),
                }
            } else {
                ApplyOutcomeOwned::NoChange {
                    store: store_hex,
                    generation: commit.generation.value(),
                }
            };
            Ok(Output::Apply(outcome))
        }
    }
}

pub(crate) fn integration_error(error: bumbledb::integration::IntegrationError) -> RuntimeError {
    use bumbledb::integration::IntegrationError;
    match error {
        IntegrationError::Core(error) => engine_error(&error),
        IntegrationError::Changes(error) => change_error(&error),
        IntegrationError::Host(error) => RuntimeError::Engine {
            diagnostic: None,
            kind: "hostSeal",
            message: format!("{error:?}"),
        },
        IntegrationError::Work(error) => RuntimeError::Work(error),
        IntegrationError::ForeignSchema => RuntimeError::Engine {
            diagnostic: None,
            kind: crate::tags::error_family::SCHEMA_MISMATCH,
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
    let report = lease
        .db()
        .integration_store()
        .map_report(context)
        .map_err(|error| engine_error(&bumbledb::Error::Store(Box::new(error))))?;
    let retained = lease.runtime().database_operations(owner_id, database_id);
    Ok(Output::DbReport(DbInspectionOwned {
        generation: generation.value(),
        storage: report,
        retained_operations: retained,
    }))
}

/// Worker-side copy of a sealed change set. L14 should submit a payload
/// job and call this instead of a JS-thread payload lock.
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
