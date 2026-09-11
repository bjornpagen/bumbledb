//! Ordinary change batches against one unpublished native owner. Retained
//! transition evidence is data; no application transformation runs here.
use std::path::PathBuf;
use std::sync::Arc;

use bumbledb::SchemaDescriptor;
use bumbledb_log::history::authority::{Access, Activation};
use bumbledb_log::history::decision::genesis_stamp;
use bumbledb_log::transition::local::{self, Begin, Population, Resolution};
use bumbledb_log::transition::namespace::{NamespaceError, TargetNamespace};
use bumbledb_log::transition::{Captured, Contract, Installed};
use napi::bindgen_prelude::{Buffer, Env, External, Function, Object};
use napi_derive::napi;

use super::{
    AdminOwned, FreshnessOwned, HistoryKind, HistoryResource, LIMITS, LogFail, LogHistoryHandle,
    MachineOutput, MachineResult, OperationId, RecoveryError, SnapshotHandle, SnapshotOwned,
    WorkContext, begin_cache_operation, begin_snapshot_teardown, byte_limit, fail_of_log,
    fail_of_recovery, fail_output, fingerprint_of_hex, hex32, history_handle, identity_in,
    identity_wire, marshal, pin_authority_frame, protocol, stamp_wire, state_wire, throw_frame,
    uuid_text,
};
use crate::db_wire::{ChangesHandle, changes_from_payload, changes_route};
use crate::runtime::registry::{NativeKind, Payload, RegistryAdmission};
use crate::runtime::{Capability, Output, Runtime, RuntimeError};
use crate::runtime_wire::{
    CloseWire, OperationHandle, notification, operation_handle, reporter, take_output, thrown,
};

pub enum TransitionOwned {
    Populating {
        population: Box<Population>,
        source: Box<SnapshotOwned>,
        directory: PathBuf,
    },
    Ready {
        installed: Installed,
        source: Option<Box<SnapshotOwned>>,
        directory: PathBuf,
    },
    Activated {
        contract: Contract,
        genesis: bumbledb_log::history::DecisionDigest,
        directory: PathBuf,
    },
    Uninstalled,
    Aborted,
    Applied,
}

pub struct PopulationHandle {
    identity: usize,
    admission: RegistryAdmission,
    sequence: Arc<crate::runtime::sequence::PayloadSequence>,
}

impl Drop for PopulationHandle {
    fn drop(&mut self) {
        let _ = self
            .admission
            .runtime
            .request_resource_close(self.admission.cap());
    }
}

fn population_route(handle: &PopulationHandle) -> Result<(Arc<Runtime>, Capability), RuntimeError> {
    if handle.identity != crate::runtime_wire::addon_identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    Ok((
        Arc::clone(&handle.admission.runtime),
        handle.admission.cap(),
    ))
}

fn contract_in(value: &Object) -> napi::Result<Contract> {
    let ctx = "transition contract";
    Ok(Contract {
        operation: OperationId::from_core(marshal::uuid_in(
            &marshal::req::<String>(value, "operationId", ctx)?,
            ctx,
        )?),
        source: identity_in(&marshal::req::<Object>(value, "source", ctx)?, ctx)?,
        target: identity_in(&marshal::req::<Object>(value, "target", ctx)?, ctx)?,
        commitment: fingerprint_of_hex(&marshal::req::<String>(value, "commitment", ctx)?)?.0,
    })
}

fn descriptor_in(env: Env, request: &Object) -> napi::Result<SchemaDescriptor> {
    let spec: Object = marshal::req(request, "schema", "transition schema")?;
    match crate::descriptor_of(&spec)? {
        Ok((descriptor, _)) => Ok(descriptor),
        Err(
            crate::OpenOutcome::SchemaError(message) | crate::OpenOutcome::NewtypeMismatch(message),
        ) => Err(marshal::throw_kind_message(
            env,
            crate::tags::error_family::SCHEMA,
            message,
        )),
    }
}

fn installed_in(env: Env, request: &Object) -> napi::Result<Installed> {
    let bytes = crate::runtime_wire::unshared_input(
        env,
        marshal::req::<napi::Unknown>(request, "evidence", "transition evidence")?,
    )?;
    Installed::decode(bytes.as_ref(), LIMITS.envelope_bytes).map_err(|error| {
        throw_frame(
            env,
            &protocol(
                "UnsupportedArtifact",
                format!("transition evidence: {error:?}"),
            ),
        )
    })
}

enum Call {
    Begin(Contract, SchemaDescriptor),
    Resolve(Contract, SchemaDescriptor),
    Activate(Installed, SchemaDescriptor),
    Abort(Contract, SchemaDescriptor),
    Inspect(Installed),
}

fn call_in(env: Env, request: &Object) -> napi::Result<Call> {
    let verb: String = marshal::req(request, "verb", "transition")?;
    Ok(match verb.as_str() {
        "begin" => Call::Begin(
            contract_in(&marshal::req(request, "contract", "transition")?)?,
            descriptor_in(env, request)?,
        ),
        "resolve" => Call::Resolve(
            contract_in(&marshal::req(request, "contract", "transition")?)?,
            descriptor_in(env, request)?,
        ),
        "activate" => Call::Activate(installed_in(env, request)?, descriptor_in(env, request)?),
        "abort" => Call::Abort(
            contract_in(&marshal::req(request, "contract", "transition")?)?,
            descriptor_in(env, request)?,
        ),
        "inspect" => Call::Inspect(installed_in(env, request)?),
        _ => return Err(marshal::err("unknown transition verb".into())),
    })
}

fn fail(error: local::Error) -> LogFail {
    use local::Error;
    match error {
        Error::Core(error) => LogFail::Core(crate::runtime::session::engine_error(&error)),
        Error::Work(error) => LogFail::Core(RuntimeError::Work(error)),
        Error::Log(error) => fail_of_log(error),
        Error::Recovery(RecoveryError::Changes(error)) => {
            LogFail::Core(crate::db_wire::change_error(&error))
        }
        Error::Recovery(error) => fail_of_recovery(error),
        Error::Namespace(NamespaceError::Busy) => LogFail::Core(RuntimeError::HandleBusy),
        Error::Namespace(NamespaceError::Io(error)) => {
            LogFail::Core(crate::runtime::owners::io_error(error))
        }
        Error::Frame(bumbledb_log::history::FrameError::LimitExceeded {
            section,
            required,
            limit,
        }) => byte_limit(section, required, limit),
        Error::Frame(bumbledb_log::history::FrameError::Allocation) => {
            LogFail::Core(RuntimeError::Work(bumbledb::WorkError::Allocation))
        }
        Error::Frame(error) => protocol("Corruption", format!("transition record: {error:?}")),
        Error::ContractMismatch => protocol(
            "OperationConflict",
            "transition contract differs from its native evidence",
        ),
        Error::UnsupportedArtifact => protocol("UnsupportedArtifact", "retired migration artifact"),
        Error::Aborted => protocol("OperationConflict", "transition was aborted"),
        Error::ActivationWon => protocol(
            "OperationConflict",
            "target was activated; source cannot thaw",
        ),
        Error::OutputMismatch => protocol(
            "Corruption",
            "target content differs from its installed evidence",
        ),
        Error::Namespace(error) => protocol("OperationConflict", error.to_string()),
    }
}

fn output(result: MachineResult<TransitionOwned>) -> Result<Output, RuntimeError> {
    match result {
        Ok(value) => Ok(Output::Machine(MachineOutput::Transition(value))),
        Err(LogFail::Core(error)) => Err(error),
        Err(error) => Ok(fail_output(error)),
    }
}

fn capture(
    resource: &Arc<HistoryResource>,
    captured: Captured,
    work: &WorkContext,
) -> MachineResult<Box<SnapshotOwned>> {
    let (_, lease) = resource.kind_and_lease()?;
    let frame = pin_authority_frame(resource, lease, work)?;
    let live = frame
        .authority
        .live()
        .map_err(|_| protocol("DatabaseDeleted", "source deleted"))?;
    if frame.identity != captured.contract.source
        || frame.decision != captured.decision
        || frame.state != captured.state
        || live.access
            != (Access::Frozen {
                operation: captured.contract.operation,
                intent: local::intent(&captured.contract).map_err(fail)?,
            })
    {
        begin_snapshot_teardown(&frame.opened.session);
        return Err(protocol(
            "OperationConflict",
            "source reader does not match the exact frozen capture",
        ));
    }
    Ok(Box::new(SnapshotOwned {
        session: Arc::new(frame.opened.session),
        sealed: frame.opened.sealed,
        identity: frame.identity,
        decision: frame.decision,
        state: frame.state,
        freshness: FreshnessOwned::Latest,
        transferred: false,
    }))
}

#[expect(
    clippy::large_types_passed_by_value,
    reason = "consumes a native result into its bridge output"
)]
fn resolved(value: Resolution, directory: PathBuf) -> TransitionOwned {
    match value {
        Resolution::Uninstalled => TransitionOwned::Uninstalled,
        Resolution::Ready(installed) => TransitionOwned::Ready {
            installed,
            source: None,
            directory,
        },
        Resolution::Activated { contract, genesis } => TransitionOwned::Activated {
            contract,
            genesis,
            directory,
        },
        Resolution::Aborted => TransitionOwned::Aborted,
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one transition dispatch and native ownership boundary"
)]
fn run(
    resource: &Arc<HistoryResource>,
    call: Call,
    work: &WorkContext,
) -> MachineResult<TransitionOwned> {
    let (kind, _lease) = resource.kind_and_lease()?;
    let HistoryKind::Local(history) = kind else {
        return Err(protocol(
            "UnsupportedArtifact",
            "transition calls currently require an authoritative local history",
        ));
    };
    if let Call::Inspect(installed) = call {
        let contract = installed.captured.contract;
        let (_, lease) = resource.kind_and_lease()?;
        let frame = pin_authority_frame(resource, lease, work)?;
        let stamp = genesis_stamp(
            &local::genesis(&installed, LIMITS.envelope_bytes).map_err(fail)?,
            LIMITS.envelope_bytes,
        )
        .map_err(|error| fail(error.into()))?;
        if frame.identity != contract.target
            || frame.decision != stamp
            || frame.authority.activation != Activation::NotActivated
        {
            begin_snapshot_teardown(&frame.opened.session);
            return Err(protocol(
                "OperationConflict",
                "reader is not the immutable ready target",
            ));
        }
        // Reuse the public snapshot take grammar, with exact native provenance.
        return Ok(TransitionOwned::Ready {
            installed,
            source: Some(Box::new(SnapshotOwned {
                session: Arc::new(frame.opened.session),
                sealed: frame.opened.sealed,
                identity: frame.identity,
                decision: frame.decision,
                state: frame.state,
                freshness: FreshnessOwned::Latest,
                transferred: false,
            })),
            directory: PathBuf::new(),
        });
    }
    let root = resource.owner.reference().child_path("targets")?;
    let contract = match &call {
        Call::Begin(c, _) | Call::Resolve(c, _) | Call::Abort(c, _) => *c,
        Call::Activate(i, _) => i.captured.contract,
        Call::Inspect(_) => unreachable!(),
    };
    if history.identity() != contract.source {
        return Err(protocol(
            "ForeignIdentity",
            "history is not the transition source",
        ));
    }
    let directory = TargetNamespace::new(&root, contract.target.incarnation_id)
        .map_err(|error| fail(error.into()))?
        .deployment_dir();
    match call {
        Call::Begin(contract, descriptor) => {
            match Population::begin(history.as_ref(), &root, contract, descriptor, LIMITS, work)
                .map_err(fail)?
            {
                Begin::Population(population) => {
                    let source = capture(resource, population.captured(), work)?;
                    Ok(TransitionOwned::Populating {
                        population: Box::new(population),
                        source,
                        directory,
                    })
                }
                Begin::Resolved(Resolution::Ready(installed)) => Ok(TransitionOwned::Ready {
                    source: Some(capture(resource, installed.captured, work)?),
                    installed,
                    directory,
                }),
                Begin::Resolved(value) => Ok(resolved(value, directory)),
            }
        }
        Call::Resolve(contract, descriptor) => Ok(resolved(
            local::resolve(
                history.as_ref(),
                &root,
                &contract,
                &descriptor,
                LIMITS,
                work,
            )
            .map_err(fail)?,
            directory,
        )),
        Call::Activate(installed, descriptor) => Ok(resolved(
            local::activate(
                history.as_ref(),
                &root,
                &installed,
                &descriptor,
                LIMITS,
                work,
            )
            .map_err(fail)?,
            directory,
        )),
        Call::Abort(contract, descriptor) => {
            local::abort(
                history.as_ref(),
                &root,
                &contract,
                &descriptor,
                LIMITS,
                work,
            )
            .map_err(fail)?;
            Ok(TransitionOwned::Aborted)
        }
        Call::Inspect(_) => unreachable!(),
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_transition_call(
    env: Env,
    handle: &External<LogHistoryHandle>,
    request: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let resource = Arc::clone(history_handle(handle).map_err(|error| thrown(env, error))?);
    let call = call_in(env, &request)?;
    let guard = handle
        .borrow
        .as_ref()
        .map(begin_cache_operation)
        .transpose()
        .map_err(|error| thrown(env, error))?;
    let runtime = Arc::clone(&resource.runtime);
    let job = Arc::clone(&resource);
    let operation = runtime
        .submit_db(
            &resource.managed,
            WorkContext::new(),
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |work| {
                    let _guard = guard;
                    output(run(&job, call, work))
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_population_apply(
    env: Env,
    handle: &External<PopulationHandle>,
    changes: &External<ChangesHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let (runtime, _cap) = population_route(handle).map_err(|error| thrown(env, error))?;
    let (change_runtime, change) = changes_route(changes).map_err(|error| thrown(env, error))?;
    if !Arc::ptr_eq(&runtime, &change_runtime) {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    let reservation = handle
        .sequence
        .reserve(false)
        .map_err(|error| thrown(env, error))?;
    let operation = runtime
        .submit_payload(change, WorkContext::new(), notification(callback)?, |_| {
            Ok(Box::new(move |_, payload, _| {
                let changes = changes_from_payload(payload)?.changes;
                Ok(Output::OrderedPayloadContinuation {
                    reservation,
                    work: Box::new(move |work, payload, _| {
                        let Payload::Population(slot) = payload else {
                            return Err(RuntimeError::InvalidArgument);
                        };
                        let population = slot.as_mut().ok_or(RuntimeError::ClosedHandle)?;
                        output(
                            population
                                .apply(&changes, work)
                                .map(|()| TransitionOwned::Applied)
                                .map_err(fail),
                        )
                    }),
                })
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_population_finish(
    env: Env,
    handle: &External<PopulationHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let (runtime, cap) = population_route(handle).map_err(|error| thrown(env, error))?;
    let notify = notification(callback)?;
    let reservation = handle
        .sequence
        .reserve(true)
        .map_err(|error| thrown(env, error))?;
    let operation = runtime
        .finalize_payload_ordered(cap, Some(reservation), WorkContext::new(), notify, |_| {
            Ok(Box::new(move |work, payload, _| {
                let Payload::Population(slot) = payload else {
                    return Err(RuntimeError::InvalidArgument);
                };
                let population = slot.take().ok_or(RuntimeError::ClosedHandle)?;
                let directory = population.deployment_dir();
                output(
                    population
                        .finish(work)
                        .map(|installed| TransitionOwned::Ready {
                            installed,
                            source: None,
                            directory,
                        })
                        .map_err(fail),
                )
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn log_population_close(
    env: Env,
    handle: &External<PopulationHandle>,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    let (runtime, cap) = population_route(handle).map_err(|error| thrown(env, error))?;
    crate::db_wire::close_admitted(&runtime, cap, reporter(callback)?);
    Ok(())
}

fn contract_wire<'e>(env: Env, contract: Contract) -> napi::Result<Object<'e>> {
    let mut value = Object::new(&env)?;
    value.set("operationId", uuid_text(contract.operation.as_core()))?;
    value.set("source", identity_wire(&env, contract.source)?)?;
    value.set("target", identity_wire(&env, contract.target)?)?;
    value.set("commitment", hex32(&contract.commitment))?;
    Ok(value)
}
fn captured_wire<'e>(env: Env, captured: Captured) -> napi::Result<Object<'e>> {
    let mut value = Object::new(&env)?;
    value.set("contract", contract_wire(env, captured.contract)?)?;
    value.set("decision", stamp_wire(&env, captured.decision)?)?;
    value.set("state", state_wire(&env, captured.state)?)?;
    Ok(value)
}
fn installed_wire<'e>(env: Env, installed: &Installed) -> napi::Result<Object<'e>> {
    let mut value = Object::new(&env)?;
    value.set("captured", captured_wire(env, installed.captured)?)?;
    value.set("applicationDigest", hex32(&installed.application_digest))?;
    value.set(
        "bytes",
        Buffer::from(
            installed
                .encode(LIMITS.envelope_bytes)
                .map_err(|error| throw_frame(env, &fail(error.into())))?,
        ),
    )?;
    Ok(value)
}
fn snapshot_wire<'e>(env: Env, mut owned: SnapshotOwned) -> napi::Result<Object<'e>> {
    let mut value = Object::new(&env)?;
    value.set(
        "snapshot",
        External::new(SnapshotHandle::assemble(
            Arc::clone(&owned.session),
            Arc::clone(&owned.sealed),
        )),
    )?;
    let mut provenance = Object::new(&env)?;
    provenance.set("identity", identity_wire(&env, owned.identity)?)?;
    provenance.set("decision", stamp_wire(&env, owned.decision)?)?;
    provenance.set("state", state_wire(&env, owned.state)?)?;
    let mut freshness = Object::new(&env)?;
    freshness.set("kind", "latest")?;
    provenance.set("freshness", freshness)?;
    value.set("provenance", provenance)?;
    owned.transferred = true;
    Ok(value)
}
#[napi]
pub fn log_transition_result(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Object<'_>> {
    let value = match take_output(env, handle)? {
        Output::Machine(MachineOutput::Transition(value)) => value,
        Output::Machine(MachineOutput::Admin(AdminOwned::Failed { fail, .. })) => {
            return Err(throw_frame(env, &fail));
        }
        _ => return Err(thrown(env, RuntimeError::InvalidArgument)),
    };
    let mut wire = Object::new(&env)?;
    match value {
        TransitionOwned::Populating {
            population,
            source,
            directory,
        } => {
            wire.set("kind", "populating")?;
            wire.set("captured", captured_wire(env, population.captured())?)?;
            let runtime = crate::runtime_wire::operation_runtime(handle);
            let admission = RegistryAdmission::admit(
                runtime,
                NativeKind::Population,
                Payload::Population(Some(population)),
            )
            .map_err(|error| thrown(env, error))?;
            let sequence = crate::runtime::sequence::PayloadSequence::new(
                Arc::clone(&admission.runtime),
                admission.cap(),
            );
            wire.set(
                "population",
                External::new(PopulationHandle {
                    identity: crate::runtime_wire::addon_identity(),
                    admission,
                    sequence,
                }),
            )?;
            wire.set("source", snapshot_wire(env, *source)?)?;
            wire.set("directory", directory.to_string_lossy().as_ref())?;
        }
        TransitionOwned::Ready {
            installed,
            source,
            directory,
        } => {
            wire.set("kind", "ready")?;
            wire.set("installed", installed_wire(env, &installed)?)?;
            wire.set("directory", directory.to_string_lossy().as_ref())?;
            if let Some(source) = source {
                wire.set("source", snapshot_wire(env, *source)?)?;
            }
        }
        TransitionOwned::Activated {
            contract,
            genesis,
            directory,
        } => {
            wire.set("kind", "activated")?;
            wire.set("contract", contract_wire(env, contract)?)?;
            wire.set("genesis", hex32(genesis.as_bytes()))?;
            wire.set("directory", directory.to_string_lossy().as_ref())?;
        }
        TransitionOwned::Uninstalled => wire.set("kind", "uninstalled")?,
        TransitionOwned::Aborted => wire.set("kind", "aborted")?,
        TransitionOwned::Applied => wire.set("kind", "applied")?,
    }
    Ok(wire)
}
