//! The one `logAdmin` verb family: maintenance, retention,
//! backup/restore/erase over the native admin, checkpointer, GC and recovery
//! operations. Every request derives its ref-able identity BEFORE dispatch on
//! the TS side; the native side classifies its own refusals into the
//! certainty union: refusals that PROVABLY dispatched no mutation return
//! `not-started`, ambiguous hosted outcomes return `outcome-unknown`, and
//! successes return `completed`/`report` values. Nothing here manufactures
//! a receipt or resolves uncertainty by guessing.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use bumbledb::SchemaDescriptor;
use bumbledb::work::WorkContext;
use bumbledb_log::certainty::{AdminCertainty, PublicationPhase};
use bumbledb_log::checkpointer::{
    CheckpointError, CheckpointKind, CheckpointOutcome, CheckpointPolicy,
};
use bumbledb_log::gc::GcPolicy;
use bumbledb_log::history::authority::Lifecycle;
use bumbledb_log::history::receive_limits_for_object;
use bumbledb_log::history::{
    DatabaseIdentity, DecisionStamp, OperationId, ReceiptEpoch, StateStamp,
};
use bumbledb_log::manifest::RootPolicy;
use bumbledb_log::recovery::{self, RecoveryError};
use bumbledb_log::store::fence::acquire_directory;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::s3::S3Store;
use bumbledb_log::store::{TransportContext, get_verified};
use napi::bindgen_prelude::{BigInt, Env, External, Function, Object};

use crate::marshal;
use crate::runtime::owners::DbLease;
use crate::runtime::{Output, Runtime, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, RuntimeHandle, notification, operation_handle, owner as runtime_owner, thrown,
};

use super::{
    BackendSpec, CredentialsSpec, LIMITS, LogFail, MachineOutput, MachineResult, binding_spec_in,
    frame_object, hex32, identity_wire, optional_object, optional_string, protocol, s3_store,
    stamp_wire, state_wire, uuid_text,
};

// ---------------------------------------------------------------------------
// Owned admin values (Send) and the certainty envelope.
// ---------------------------------------------------------------------------

pub enum AdminOwned {
    Completed(AdminValueOwned),
    Report(AdminValueOwned),
    /// A refusal that provably dispatched no mutation (`not-started`), or a
    /// dispatched-but-unproven hosted outcome (`outcome-unknown`).
    Failed {
        fail: LogFail,
        phase: PublicationPhase,
    },
}

impl AdminOwned {
    pub(crate) fn mutation_evidence(&self) -> bool {
        matches!(self, Self::Completed(_))
    }
}

pub enum AdminValueOwned {
    Checkpoint {
        at: DecisionStamp,
        state: StateStamp,
        root: String,
    },
    PinRoot {
        root: String,
        at: DecisionStamp,
        state: StateStamp,
    },
    ReleaseRoot {
        root: String,
        was_current_recovery_base: bool,
    },
    RotateEpoch {
        open_epoch: u64,
    },
    RetireReceipts {
        retired_through: u64,
    },
    CollectGarbage {
        object_epoch: u64,
        swept: u64,
        orphans_observed: u64,
    },
    Backup {
        manifest_digest: [u8; 32],
        objects: u64,
        bytes: u64,
        at: DecisionStamp,
    },
    VerifyBackup {
        identity: DatabaseIdentity,
        at: DecisionStamp,
        state: StateStamp,
        objects: u64,
        bytes: u64,
        manifest_digest: [u8; 32],
    },
    Restore {
        identity: DatabaseIdentity,
        genesis: [u8; 32],
        directory: String,
    },
    Erase {
        tombstoned: bool,
        retained_roots: Vec<String>,
        residual: Vec<(String, String)>,
    },
}

// ---------------------------------------------------------------------------
// Request parsing (JS thread) into one owned Send request.
// ---------------------------------------------------------------------------

enum DestinationSpec {
    Filesystem {
        directory: String,
    },
    S3 {
        bucket: String,
        prefix: String,
        region: Option<String>,
        credentials: CredentialsSpec,
    },
}

fn destination_in(obj: &Object, ctx: &str) -> napi::Result<DestinationSpec> {
    let kind: String = marshal::req(obj, "kind", ctx)?;
    match kind.as_str() {
        "filesystem" => Ok(DestinationSpec::Filesystem {
            directory: marshal::req(obj, "directory", ctx)?,
        }),
        "s3" => {
            let credentials: Object = marshal::req(obj, "credentials", ctx)?;
            let credentials_kind: String = marshal::req(&credentials, "kind", ctx)?;
            let credentials = match credentials_kind.as_str() {
                "provider-chain" => CredentialsSpec::ProviderChain,
                "static" => CredentialsSpec::Static {
                    access_key_id: marshal::req(&credentials, "accessKeyId", ctx)?,
                    secret_access_key: marshal::req(&credentials, "secretAccessKey", ctx)?,
                    session_token: optional_string(&credentials, "sessionToken")?,
                },
                other => {
                    return Err(marshal::err(format!(
                        "bumbledb-log marshal: {ctx}: unknown credentials kind `{other}`"
                    )));
                }
            };
            Ok(DestinationSpec::S3 {
                bucket: marshal::req(obj, "bucket", ctx)?,
                prefix: marshal::req(obj, "prefix", ctx)?,
                region: optional_string(obj, "region")?,
                credentials,
            })
        }
        other => Err(marshal::err(format!(
            "bumbledb-log marshal: {ctx}: unknown destination kind `{other}`"
        ))),
    }
}

struct BindingSpec {
    directory: String,
    identity: DatabaseIdentity,
    backend: BackendSpec,
    /// The lowered `SchemaSpec`, parsed when present (verbs that must open
    /// the local materialization require it unless the tenant is already
    /// open in this runtime's registry).
    descriptor: Option<(SchemaDescriptor, crate::FieldAttrsTable)>,
}

fn binding_with_schema_in(env: Env, request: &Object, ctx: &str) -> napi::Result<BindingSpec> {
    let binding: Object = marshal::req(request, "binding", ctx)?;
    let (directory, identity, backend) = binding_spec_in(&binding, ctx)?;
    let descriptor = match optional_object(request, "schema")? {
        None => None,
        Some(spec) => match crate::descriptor_of(&spec)? {
            Ok(parsed) => Some(parsed),
            Err(
                crate::OpenOutcome::SchemaError(message)
                | crate::OpenOutcome::NewtypeMismatch(message),
            ) => {
                return Err(marshal::throw_kind_message(
                    env,
                    crate::tags::error_family::SCHEMA,
                    message,
                ));
            }
        },
    };
    Ok(BindingSpec {
        directory,
        identity,
        backend,
        descriptor,
    })
}

fn operation_in(request: &Object, ctx: &str) -> napi::Result<OperationId> {
    Ok(OperationId::from_core(marshal::uuid_in(
        &marshal::req::<String>(request, "operationId", ctx)?,
        ctx,
    )?))
}

enum AdminVerb {
    Checkpoint {
        binding: BindingSpec,
        operation: OperationId,
    },
    PinRoot {
        binding: BindingSpec,
        operation: OperationId,
        label: String,
    },
    ReleaseRoot {
        binding: BindingSpec,
        operation: OperationId,
        root: OperationId,
    },
    RotateEpoch {
        binding: BindingSpec,
        operation: OperationId,
    },
    RetireReceipts {
        binding: BindingSpec,
        operation: OperationId,
        through: u64,
    },
    CollectGarbage {
        binding: BindingSpec,
        operation: OperationId,
    },
    Backup {
        binding: BindingSpec,
        operation: OperationId,
        destination: DestinationSpec,
    },
    VerifyBackup {
        destination: DestinationSpec,
        backup: Option<OperationId>,
    },
    Restore {
        source: DestinationSpec,
        target: BindingSpec,
        operation: OperationId,
        backup: Option<OperationId>,
    },
    Erase {
        binding: BindingSpec,
        operation: OperationId,
        retain_roots: Vec<OperationId>,
    },
}

#[allow(clippy::too_many_lines)]
fn admin_verb_in(env: Env, request: &Object) -> napi::Result<AdminVerb> {
    let ctx = "admin request";
    let verb: String = marshal::req(request, "verb", ctx)?;
    Ok(match verb.as_str() {
        "checkpoint" => AdminVerb::Checkpoint {
            binding: binding_with_schema_in(env, request, ctx)?,
            operation: operation_in(request, ctx)?,
        },
        "pin-root" => AdminVerb::PinRoot {
            binding: binding_with_schema_in(env, request, ctx)?,
            operation: operation_in(request, ctx)?,
            label: marshal::req(request, "label", ctx)?,
        },
        "release-root" => AdminVerb::ReleaseRoot {
            binding: binding_with_schema_in(env, request, ctx)?,
            operation: operation_in(request, ctx)?,
            root: OperationId::from_core(marshal::uuid_in(
                &marshal::req::<String>(request, "root", ctx)?,
                ctx,
            )?),
        },
        "rotate-receipt-epoch" => AdminVerb::RotateEpoch {
            binding: binding_with_schema_in(env, request, ctx)?,
            operation: operation_in(request, ctx)?,
        },
        "retire-receipts" => AdminVerb::RetireReceipts {
            binding: binding_with_schema_in(env, request, ctx)?,
            operation: operation_in(request, ctx)?,
            through: marshal::u64_in(&marshal::req::<BigInt>(request, "through", ctx)?, ctx)?,
        },
        "collect-garbage" => AdminVerb::CollectGarbage {
            binding: binding_with_schema_in(env, request, ctx)?,
            operation: operation_in(request, ctx)?,
        },
        "backup" => AdminVerb::Backup {
            binding: binding_with_schema_in(env, request, ctx)?,
            operation: operation_in(request, ctx)?,
            destination: destination_in(
                &marshal::req::<Object>(request, "destination", ctx)?,
                ctx,
            )?,
        },
        "verify-backup" => AdminVerb::VerifyBackup {
            destination: destination_in(
                &marshal::req::<Object>(request, "destination", ctx)?,
                ctx,
            )?,
            backup: optional_string(request, "backup")?
                .map(|hex| marshal::uuid_in(&hex, ctx).map(OperationId::from_core))
                .transpose()?,
        },
        "restore" => AdminVerb::Restore {
            source: destination_in(&marshal::req::<Object>(request, "source", ctx)?, ctx)?,
            target: {
                let target: Object = marshal::req(request, "target", ctx)?;
                let (directory, identity, backend) = binding_spec_in(&target, ctx)?;
                let descriptor = match optional_object(request, "schema")? {
                    None => None,
                    Some(spec) => match crate::descriptor_of(&spec)? {
                        Ok(parsed) => Some(parsed),
                        Err(
                            crate::OpenOutcome::SchemaError(message)
                            | crate::OpenOutcome::NewtypeMismatch(message),
                        ) => {
                            return Err(marshal::throw_kind_message(
                                env,
                                crate::tags::error_family::SCHEMA,
                                message,
                            ));
                        }
                    },
                };
                BindingSpec {
                    directory,
                    identity,
                    backend,
                    descriptor,
                }
            },
            operation: operation_in(request, ctx)?,
            backup: optional_string(request, "backup")?
                .map(|hex| marshal::uuid_in(&hex, ctx).map(OperationId::from_core))
                .transpose()?,
        },
        "erase" => {
            let retain: napi::bindgen_prelude::Array = marshal::req(request, "retainRoots", ctx)?;
            let mut retain_roots = Vec::with_capacity(retain.len() as usize);
            for index in 0..retain.len() {
                retain_roots.push(OperationId::from_core(marshal::uuid_in(
                    &marshal::req_at::<String>(&retain, index, ctx)?,
                    ctx,
                )?));
            }
            AdminVerb::Erase {
                binding: binding_with_schema_in(env, request, ctx)?,
                operation: operation_in(request, ctx)?,
                retain_roots,
            }
        }
        other => {
            return Err(marshal::err(format!(
                "bumbledb-log marshal: unknown admin verb `{other}`"
            )));
        }
    })
}

// ---------------------------------------------------------------------------
// The registered admin operation.
// ---------------------------------------------------------------------------

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn admin_verb(
    env: Env,
    handle: &External<RuntimeHandle>,
    request: &Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = runtime_owner(handle).map_err(|error| thrown(env, error))?;
    let verb = admin_verb_in(env, request)?;
    let shared = Arc::clone(runtime);
    let operation = runtime
        .submit(WorkContext::new(), notification(callback)?, move |_| {
            Ok(Box::new(move |context| {
                let owned = match run_admin(&shared, verb, context) {
                    Ok(owned) => owned,
                    Err(LogFail::Core(core)) => return Err(core),
                    Err(fail) => AdminOwned::Failed {
                        fail,
                        phase: PublicationPhase::Prepared,
                    },
                };
                Ok(Output::Machine(MachineOutput::Admin(owned)))
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

/// One transiently opened local materialization for an admin verb — reuses
/// an already-open registry tenant when one holds the directory; otherwise
/// takes the kernel fence and opens the engine for exactly this job.
enum AdminDb {
    Leased(DbLease),
    Transient {
        db: Arc<crate::Engine>,
        _lock: bumbledb_log::store::fence::DirectoryLock,
    },
}

impl AdminDb {
    fn db(&self) -> &crate::Engine {
        match self {
            Self::Leased(lease) => lease.db(),
            Self::Transient { db, .. } => db,
        }
    }
}

fn open_admin_db(
    runtime: &Arc<Runtime>,
    binding: &BindingSpec,
    context: &WorkContext,
) -> MachineResult<AdminDb> {
    let directory = Path::new(&binding.directory);
    if let Some(lease) = runtime.lease_database_at(directory)? {
        // Warm reuse: the registry matched by DIRECTORY alone — the identity
        // gate below is what proves the request names THIS tenant.
        let db = AdminDb::Leased(lease);
        verify_admin_identity(&db, binding, context)?;
        return Ok(db);
    }
    let Some((descriptor, _attrs)) = binding.descriptor.clone() else {
        return Err(protocol(
            "Misuse",
            "this admin verb needs the tenant open in this runtime, or the request's \
             `schema` field (the lowered SchemaSpec)",
        ));
    };
    let held = acquire_directory(directory).map_err(|error| {
        if error.kind() == std::io::ErrorKind::WouldBlock {
            LogFail::Core(RuntimeError::DirectoryBusy)
        } else {
            LogFail::Core(crate::runtime::owners::io_error(error))
        }
    })?;
    let ready = recovery::materialization_path(directory);
    if !ready.exists() {
        return Err(protocol("DatabaseMissing", "no materialization"));
    }
    let db = crate::Engine::open(&ready, descriptor, context.clone())
        .map_err(|error| LogFail::Core(crate::runtime::session::engine_error(&error)))?;
    let db = AdminDb::Transient {
        db: Arc::new(db),
        _lock: held,
    };
    verify_admin_identity(&db, binding, context)?;
    Ok(db)
}

/// The origin binding the request claims, in the recorded grammar: local
/// bindings are `local` + the tenant directory; hosted bindings are
/// `s3:{bucket}` + the object prefix.
fn expected_binding(binding: &BindingSpec) -> recovery::OriginBinding {
    match &binding.backend {
        BackendSpec::Local => recovery::OriginBinding {
            origin: "local".into(),
            prefix: binding.directory.as_str().into(),
            identity: binding.identity,
        },
        BackendSpec::Hosted { bucket, prefix, .. } => recovery::OriginBinding {
            origin: format!("s3:{bucket}").into(),
            prefix: prefix.as_str().into(),
            identity: binding.identity,
        },
    }
}

/// The materialization's recorded origin binding, when one exists.
fn recorded_binding(
    db: &crate::Engine,
    context: &WorkContext,
) -> MachineResult<Option<recovery::OriginBinding>> {
    let mut owned: Option<Vec<u8>> = None;
    let mut host_error = None;
    db.read(context.clone(), |read| {
        match read.integration_host_record(recovery::BINDING_KEY) {
            Ok(record) => owned = record.map(<[u8]>::to_vec),
            Err(error) => host_error = Some(error),
        }
        Ok(())
    })
    .map_err(|error| LogFail::Core(crate::runtime::session::engine_error(&error)))?;
    if let Some(error) = host_error {
        return Err(protocol("Corruption", format!("{error:?}")));
    }
    match owned {
        None => Ok(None),
        Some(bytes) => {
            Ok(Some(recovery::decode_binding(&bytes).map_err(|error| {
                protocol("Corruption", format!("{error:?}"))
            })?))
        }
    }
}

/// The local admin identity+origin gate (REP-011/SDK-016/ARCH-004), run on
/// BOTH the warm (registry-reused) and cold (transiently opened) paths
/// BEFORE any admin verb mutates or cleans anything:
///
/// 1. the committed authority attachment must record exactly the requested
///    database/incarnation/schema identity, and
/// 2. a recorded origin binding, when present, must agree with the requested
///    backend origin (a hosted cache without a binding record is never
///    adopted; a transition-installed local materialization may legitimately
///    carry none — its authority identity is the dispositive gate).
fn verify_admin_identity(
    db: &AdminDb,
    binding: &BindingSpec,
    context: &WorkContext,
) -> MachineResult<()> {
    bumbledb_log::admin::verify_local_identity(db.db(), binding.identity, LIMITS.envelope_bytes)
        .map_err(fail_of_admin)?;
    let expected = expected_binding(binding);
    match recorded_binding(db.db(), context)? {
        Some(recorded) => {
            if recorded != expected {
                return Err(protocol(
                    "CacheIdentityMismatch",
                    "the materialization's recorded origin binding disagrees with the \
                     requested binding",
                ));
            }
        }
        None => {
            if matches!(binding.backend, BackendSpec::Hosted { .. }) {
                return Err(protocol(
                    "CacheIdentityMismatch",
                    "the cache carries no binding record",
                ));
            }
        }
    }
    Ok(())
}

/// The hosted authority selected by the binding, identity-validated: reads
/// the live head at the prefix and refuses unless it records exactly the
/// requested identity — BEFORE any hosted mutation (selecting a prefix is
/// not validation). `None` for local bindings.
fn validated_backend(
    binding: &BindingSpec,
    context: &WorkContext,
) -> MachineResult<Option<(Arc<S3Store>, String, bumbledb_log::manifest::HeadRecord)>> {
    let Some((backend, prefix)) = store_of_backend(&binding.backend)? else {
        return Ok(None);
    };
    let head = bumbledb_log::admin::verify_hosted_identity(
        backend.as_ref(),
        &prefix,
        binding.identity,
        LIMITS.envelope_bytes,
        context,
    )
    .map_err(fail_of_admin)?;
    Ok(Some((backend, prefix, head)))
}

/// Runs one bounded closure against a destination/backend store.
enum StoreSpec {
    Fs(FsStore),
    S3(Arc<S3Store>),
}

fn store_of_destination(destination: &DestinationSpec) -> MachineResult<(StoreSpec, String)> {
    match destination {
        DestinationSpec::Filesystem { directory } => Ok((
            StoreSpec::Fs(FsStore::new(directory.clone())),
            String::new(),
        )),
        DestinationSpec::S3 {
            bucket,
            prefix,
            region,
            credentials,
        } => Ok((
            StoreSpec::S3(s3_store(bucket, region.as_deref(), credentials)?),
            prefix.clone(),
        )),
    }
}

fn store_of_backend(backend: &BackendSpec) -> MachineResult<Option<(Arc<S3Store>, String)>> {
    match backend {
        BackendSpec::Local => Ok(None),
        BackendSpec::Hosted {
            bucket,
            prefix,
            region,
            credentials,
        } => Ok(Some((
            s3_store(bucket, region.as_deref(), credentials)?,
            prefix.clone(),
        ))),
    }
}

macro_rules! with_store {
    ($spec:expr, $prefix:ident, $store:ident => $body:expr) => {
        match $spec {
            (StoreSpec::Fs(inner), $prefix) => {
                let $store = &inner;
                $body
            }
            (StoreSpec::S3(inner), $prefix) => {
                let $store = inner.as_ref();
                $body
            }
        }
    };
}

#[allow(clippy::too_many_lines)]
fn run_admin(
    runtime: &Arc<Runtime>,
    verb: AdminVerb,
    context: &WorkContext,
) -> MachineResult<AdminOwned> {
    context.checkpoint().map_err(RuntimeError::from)?;
    match verb {
        AdminVerb::Checkpoint { binding, operation } => {
            let _ = operation;
            let Some((backend, prefix, _head)) = validated_backend(&binding, context)? else {
                return Ok(AdminOwned::Failed {
                    fail: protocol(
                        "Misuse",
                        "LocalHistory needs no checkpoint: LMDB is complete; named restore \
                         points (pin-root) are the local specialization",
                    ),
                    phase: PublicationPhase::Prepared,
                });
            };
            let db = open_admin_db(runtime, &binding, context)?;
            let outcome = bumbledb_log::checkpointer::publish_checkpoint(
                db.db(),
                &backend,
                &prefix,
                LIMITS,
                CheckpointKind::Ordinary,
                &CheckpointPolicy::DEFAULT,
                context,
            )
            .map_err(|error| protocol("Backend", format!("{error:?}")))?;
            match outcome {
                CheckpointOutcome::Published {
                    manifest,
                    base,
                    tip,
                    ..
                } => {
                    let state = bumbledb_log::admin::local_state(db.db(), LIMITS.envelope_bytes)
                        .map_err(|error| protocol("Corruption", format!("{error:?}")))?;
                    let _ = tip;
                    Ok(AdminOwned::Completed(AdminValueOwned::Checkpoint {
                        at: base,
                        state,
                        root: hex32(&manifest.digest),
                    }))
                }
                CheckpointOutcome::Discarded { current_base_seq } => Ok(AdminOwned::Failed {
                    fail: protocol(
                        "OperationConflict",
                        format!("another checkpoint already passed seq {current_base_seq}"),
                    ),
                    phase: PublicationPhase::ProvedNonpublication,
                }),
            }
        }
        AdminVerb::PinRoot {
            binding,
            operation,
            label,
        } => match &binding.backend {
            BackendSpec::Local => {
                let db = open_admin_db(runtime, &binding, context)?;
                let root = bumbledb_log::local_roots::create_restore_point(
                    db.db(),
                    Path::new(&binding.directory),
                    operation,
                    &label,
                    &CheckpointPolicy::DEFAULT,
                    &RootPolicy::DEFAULT,
                    context,
                )
                .map_err(fail_of_local_root)?;
                Ok(AdminOwned::Completed(AdminValueOwned::PinRoot {
                    root: uuid_text(root.id.as_core()),
                    at: root.decision,
                    state: root.state,
                }))
            }
            BackendSpec::Hosted { .. } => {
                let (backend, prefix, _head) =
                    validated_backend(&binding, context)?.expect("hosted backend");
                map_admin_certainty(
                    bumbledb_log::admin::add_named_root_hosted(
                        &backend,
                        &prefix,
                        operation,
                        bumbledb_log::manifest::RootKind::RestorePoint,
                        &label,
                        operation,
                        &RootPolicy::DEFAULT,
                        LIMITS.envelope_bytes,
                        context,
                    ),
                    |root| AdminValueOwned::PinRoot {
                        root: uuid_text(root.id.as_core()),
                        at: root.recovery.base,
                        state: root.state,
                    },
                )
            }
        },
        AdminVerb::ReleaseRoot {
            binding,
            operation,
            root,
        } => {
            let _ = operation;
            match &binding.backend {
                BackendSpec::Local => {
                    let db = open_admin_db(runtime, &binding, context)?;
                    let report = bumbledb_log::local_roots::release_restore_point(
                        db.db(),
                        Path::new(&binding.directory),
                        root,
                        context,
                    )
                    .map_err(fail_of_local_root)?;
                    let _ = report;
                    Ok(AdminOwned::Completed(AdminValueOwned::ReleaseRoot {
                        root: uuid_text(root.as_core()),
                        was_current_recovery_base: false,
                    }))
                }
                BackendSpec::Hosted { .. } => {
                    let (backend, prefix, _head) =
                        validated_backend(&binding, context)?.expect("hosted backend");
                    map_admin_certainty(
                        bumbledb_log::admin::release_named_root_hosted(
                            &backend,
                            &prefix,
                            root,
                            true,
                            LIMITS.envelope_bytes,
                            context,
                        ),
                        |released| AdminValueOwned::ReleaseRoot {
                            root: uuid_text(root.as_core()),
                            was_current_recovery_base: released.is_some(),
                        },
                    )
                }
            }
        }
        AdminVerb::RotateEpoch { binding, operation } => {
            let _ = operation;
            let db = open_admin_db(runtime, &binding, context)?;
            let authority = bumbledb_log::admin::local_authority(db.db(), LIMITS.envelope_bytes)
                .map_err(|error| protocol("Corruption", format!("{error:?}")))?;
            let current = match &authority.lifecycle {
                Lifecycle::Live(live) => live.receipts.open_epoch().get(),
                Lifecycle::Deleted { .. } => {
                    return Ok(AdminOwned::Failed {
                        fail: protocol("DatabaseDeleted", "terminal tombstone"),
                        phase: PublicationPhase::Prepared,
                    });
                }
            };
            let next = ReceiptEpoch::new(current + 1)
                .ok_or_else(|| protocol("Corruption", "epoch overflow"))?;
            match &binding.backend {
                BackendSpec::Local => {
                    bumbledb_log::admin::rotate_receipts_local(
                        db.db(),
                        next,
                        LIMITS.envelope_bytes,
                        context,
                    )
                    .map_err(fail_of_admin)?;
                    Ok(AdminOwned::Completed(AdminValueOwned::RotateEpoch {
                        open_epoch: next.get(),
                    }))
                }
                BackendSpec::Hosted { .. } => {
                    let (backend, prefix, _head) =
                        validated_backend(&binding, context)?.expect("hosted backend");
                    map_admin_certainty(
                        bumbledb_log::admin::rotate_receipts_hosted(
                            &backend,
                            &prefix,
                            next,
                            LIMITS.envelope_bytes,
                            context,
                        ),
                        |_| AdminValueOwned::RotateEpoch {
                            open_epoch: next.get(),
                        },
                    )
                }
            }
        }
        AdminVerb::RetireReceipts {
            binding,
            operation,
            through,
        } => {
            let _ = operation;
            match &binding.backend {
                BackendSpec::Local => {
                    let db = open_admin_db(runtime, &binding, context)?;
                    bumbledb_log::admin::retire_receipts_local(
                        db.db(),
                        through,
                        LIMITS.envelope_bytes,
                        context,
                    )
                    .map_err(fail_of_admin)?;
                    Ok(AdminOwned::Completed(AdminValueOwned::RetireReceipts {
                        retired_through: through,
                    }))
                }
                BackendSpec::Hosted { .. } => {
                    // Hosted retirement rides the checkpoint that stops
                    // promising the rows, then applies locally.
                    let (backend, prefix, _head) =
                        validated_backend(&binding, context)?.expect("hosted backend");
                    let db = open_admin_db(runtime, &binding, context)?;
                    let captured = bumbledb_log::admin::capture_local_parent(
                        &bumbledb_log::admin::local_authority(db.db(), LIMITS.envelope_bytes)
                            .map_err(fail_of_admin)?,
                    )
                    .map_err(fail_of_admin)?;
                    let outcome = bumbledb_log::checkpointer::publish_checkpoint(
                        db.db(),
                        &backend,
                        &prefix,
                        LIMITS,
                        CheckpointKind::RetireReceipts { through },
                        &CheckpointPolicy::DEFAULT,
                        context,
                    )
                    .map_err(|error| protocol("Backend", format!("{error:?}")))?;
                    match outcome {
                        CheckpointOutcome::Published { .. } => {
                            let (head, _) = bumbledb_log::checkpointer::read_live_head(
                                &backend,
                                &prefix,
                                LIMITS.envelope_bytes,
                                context,
                            )
                            .map_err(|error| protocol("Backend", format!("{error:?}")))?;
                            bumbledb_log::admin::apply_hosted_retirement_locally(
                                db.db(),
                                &head.control,
                                captured,
                                through,
                                LIMITS.envelope_bytes,
                                context,
                            )
                            .map_err(fail_of_admin)?;
                            Ok(AdminOwned::Completed(AdminValueOwned::RetireReceipts {
                                retired_through: through,
                            }))
                        }
                        CheckpointOutcome::Discarded { .. } => Ok(AdminOwned::Failed {
                            fail: protocol(
                                "OperationConflict",
                                "another checkpoint superseded the retirement capture",
                            ),
                            phase: PublicationPhase::ProvedNonpublication,
                        }),
                    }
                }
            }
        }
        AdminVerb::CollectGarbage { binding, operation } => {
            let Some((backend, prefix, _head)) = validated_backend(&binding, context)? else {
                return Ok(AdminOwned::Failed {
                    fail: protocol("Misuse", "LocalHistory holds no object store to collect"),
                    phase: PublicationPhase::Prepared,
                });
            };
            let report = match bumbledb_log::gc::run_collection(
                &backend,
                &prefix,
                operation,
                LIMITS,
                &GcPolicy::DEFAULT,
                context,
            ) {
                Ok(report) => report,
                Err(error) => {
                    // A stopped collection retains durable progress; the
                    // certainty answer is outcome-unknown, never a claim.
                    return Ok(AdminOwned::Failed {
                        fail: protocol("Backend", format!("{error:?}")),
                        phase: PublicationPhase::DispatchedUnresolved,
                    });
                }
            };
            let object_epoch = bumbledb_log::checkpointer::read_live_head(
                &backend,
                &prefix,
                LIMITS.envelope_bytes,
                context,
            )
            .map_or(0, |(head, _)| head.object_epoch);
            Ok(AdminOwned::Completed(AdminValueOwned::CollectGarbage {
                object_epoch,
                swept: report.deleted,
                orphans_observed: report
                    .retained_marked
                    .saturating_add(report.retained_newer)
                    .saturating_add(report.retained_unparsed),
            }))
        }
        AdminVerb::Backup {
            binding,
            operation,
            destination,
        } => {
            let Some((backend, prefix, head)) = validated_backend(&binding, context)? else {
                let db = open_admin_db(runtime, &binding, context)?;
                let destination = store_of_destination(&destination)?;
                let report = with_store!(destination, dest_prefix, store => {
                    bumbledb_log::backup::backup_local(
                        db.db(), store, &dest_prefix, binding.identity, operation,
                        LIMITS, &CheckpointPolicy::DEFAULT, context,
                    )
                });
                let report = match report {
                    Ok(report) => report,
                    Err(error) => {
                        return Ok(AdminOwned::failed_after_dispatch(format!("{error:?}")));
                    }
                };
                return Ok(AdminOwned::Completed(AdminValueOwned::Backup {
                    manifest_digest: report.manifest_digest,
                    objects: report.objects_copied,
                    bytes: report.bytes_copied,
                    at: report.manifest.tip,
                }));
            };
            let live = head
                .control
                .live()
                .map_err(|_| protocol("DatabaseDeleted", "terminal tombstone"))?;
            let root = head
                .recovery
                .as_ref()
                .ok_or_else(|| protocol("Corruption", "live head without a recovery root"))?;
            let destination = store_of_destination(&destination)?;
            let report = with_store!(destination, dest_prefix, store => {
                bumbledb_log::backup::backup_root(
                    &backend,
                    &prefix,
                    store,
                    &dest_prefix,
                    head.control.identity,
                    live.state,
                    root,
                    operation,
                    LIMITS,
                    bumbledb_log::codec::StreamLimits::DEFAULT,
                    context,
                )
            });
            let report = match report {
                Ok(report) => report,
                Err(error) => return Ok(AdminOwned::failed_after_dispatch(format!("{error:?}"))),
            };
            Ok(AdminOwned::Completed(AdminValueOwned::Backup {
                manifest_digest: report.manifest_digest,
                objects: report.objects_copied,
                bytes: report.bytes_copied,
                at: report.manifest.tip,
            }))
        }
        AdminVerb::VerifyBackup {
            destination,
            backup,
        } => {
            let Some(backup) = backup else {
                return Ok(AdminOwned::Failed {
                    fail: protocol(
                        "Misuse",
                        "verify-backup needs the backup operation id (`backup`)",
                    ),
                    phase: PublicationPhase::Prepared,
                });
            };
            let destination = store_of_destination(&destination)?;
            let (manifest, digest, report) = with_store!(destination, dest_prefix, store => {
                let (manifest, digest) =
                    bumbledb_log::backup::read_backup_manifest(store, &dest_prefix, backup, context)
                        .map_err(|error| LogFail::Protocol {
                            code: "Corruption",
                            detail: format!("{error:?}"),
                        })?;
                let report = bumbledb_log::backup::verify_backup(
                    store,
                    &dest_prefix,
                    backup,
                    LIMITS,
                    bumbledb_log::codec::StreamLimits::DEFAULT,
                    context,
                )
                .map_err(|error| LogFail::Protocol {
                    code: "Corruption",
                    detail: format!("{error:?}"),
                })?;
                (manifest, digest, report)
            });
            Ok(AdminOwned::Report(AdminValueOwned::VerifyBackup {
                identity: manifest.identity,
                at: manifest.tip,
                state: manifest.state,
                objects: report.objects_verified,
                bytes: report.bytes_verified,
                manifest_digest: digest,
            }))
        }
        AdminVerb::Restore {
            source,
            target,
            operation,
            backup,
        } => run_restore(runtime, &source, &target, operation, backup, context),
        AdminVerb::Erase {
            binding,
            operation,
            retain_roots,
        } => match validated_backend(&binding, context)? {
            None => {
                let db = open_admin_db(runtime, &binding, context)?;
                let _ = bumbledb_log::erase::erase_local(
                    db.db(),
                    operation,
                    LIMITS.envelope_bytes,
                    context,
                )
                .map_err(|error| protocol("Corruption", format!("{error:?}")))?;
                Ok(AdminOwned::Completed(AdminValueOwned::Erase {
                    tombstoned: true,
                    retained_roots: retain_roots
                        .iter()
                        .map(|root| uuid_text(root.as_core()))
                        .collect(),
                    residual: vec![("local-directory".to_string(), binding.directory.clone())],
                }))
            }
            Some((backend, prefix, head)) => {
                // Release exactly the roots NOT retained.
                let release: Vec<OperationId> = head
                    .roots
                    .iter()
                    .filter(|root| !retain_roots.contains(&root.id))
                    .map(|root| root.id)
                    .collect();
                let report = match bumbledb_log::erase::erase_hosted(
                    &backend,
                    &prefix,
                    operation,
                    &release,
                    LIMITS,
                    &GcPolicy::DEFAULT,
                    context,
                ) {
                    Ok(report) => report,
                    Err(error) => {
                        // Durable erase progress is retained; a stopped pass
                        // is outcome-unknown, resumable under the same id.
                        return Ok(AdminOwned::failed_after_dispatch(format!("{error:?}")));
                    }
                };
                let mut residual = Vec::new();
                if report.residual.head_tombstone_retained {
                    residual.push(("head-tombstone".to_string(), prefix.clone()));
                }
                if report.residual.remaining_objects > 0 {
                    residual.push((
                        "objects".to_string(),
                        format!("{} extant", report.residual.remaining_objects),
                    ));
                }
                if report.residual.backups_exports_blobs_keys_untouched {
                    residual.push((
                        "backups-exports-blobs-keys".to_string(),
                        "separately governed; untouched".to_string(),
                    ));
                }
                Ok(AdminOwned::Completed(AdminValueOwned::Erase {
                    tombstoned: true,
                    retained_roots: report
                        .residual
                        .retained_roots
                        .iter()
                        .map(|(id, _)| uuid_text(id.as_core()))
                        .collect(),
                    residual,
                }))
            }
        },
    }
}

impl AdminOwned {
    pub(crate) fn failed_after_dispatch(detail: String) -> Self {
        // A maintenance failure may have dispatched mutations before
        // stopping: durable progress is retained, and the certainty answer
        // is outcome-unknown, never a fabricated completion.
        Self::Failed {
            fail: LogFail::Protocol {
                code: "Backend",
                detail,
            },
            phase: PublicationPhase::DispatchedUnresolved,
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn fail_of_admin(error: bumbledb_log::admin::AdminError) -> LogFail {
    use bumbledb_log::admin::AdminError;
    match &error {
        AdminError::Identity(mismatch) => fail_of_identity(mismatch),
        AdminError::NotInitialized => protocol("NotInitialized", "open never initializes"),
        AdminError::Checkpoint(checkpoint) => fail_of_checkpoint(checkpoint),
        AdminError::Work(work) => LogFail::Core(RuntimeError::Work(*work)),
        AdminError::Storage(error) => LogFail::Core(crate::runtime::session::engine_error(error)),
        AdminError::Corruption(detail) => protocol("Corruption", *detail),
        AdminError::CasExhausted => LogFail::Structured(super::StructuredReason::Contention {
            attempts: 0,
            detail: "bounded CAS attempts exhausted; nothing is claimed".into(),
        }),
        AdminError::Object(_)
        | AdminError::Frame(_)
        | AdminError::Head(_)
        | AdminError::Authority(_)
        | AdminError::Host(_) => protocol("Backend", format!("{error:?}")),
    }
}

// ---------------------------------------------------------------------------
// Per-verb dispatch fences: an error after ANY dispatched
// mutation/CAS carries `DispatchedUnresolved` (outcome-unknown), never a
// fabricated `not-started`; genuinely pre-dispatch refusals stay
// `Prepared` by flowing out as `Err`.
// ---------------------------------------------------------------------------

fn admin_failed_certainty(phase: PublicationPhase) -> &'static str {
    match phase {
        PublicationPhase::Prepared | PublicationPhase::ProvedNonpublication => "not-started",
        PublicationPhase::DispatchedUnresolved | PublicationPhase::Confirmed => "outcome-unknown",
    }
}

fn map_admin_certainty<T>(
    certainty: AdminCertainty<T>,
    completed: impl FnOnce(T) -> AdminValueOwned,
) -> MachineResult<AdminOwned> {
    let phase = certainty.publication_phase();
    match certainty {
        AdminCertainty::Completed { value } => Ok(AdminOwned::Completed(completed(value))),
        AdminCertainty::NotStarted { error } => Err(fail_of_admin(error)),
        AdminCertainty::OutcomeUnknown { error } => Ok(AdminOwned::Failed {
            fail: fail_of_admin(error),
            phase,
        }),
    }
}

/// The typed checkpoint failure mapping (shared by the checkpoint and
/// hosted-retirement arms).
fn fail_of_checkpoint(error: &CheckpointError) -> LogFail {
    match error {
        CheckpointError::NotInitialized => {
            protocol("NotInitialized", "no head exists under this prefix")
        }
        CheckpointError::Deleted => protocol("DatabaseDeleted", "terminal tombstone"),
        CheckpointError::Work(work) => LogFail::Core(RuntimeError::Work(*work)),
        CheckpointError::Corruption(_) | CheckpointError::Frame(_) => {
            protocol("Corruption", format!("{error:?}"))
        }
        _ => protocol("Backend", format!("{error:?}")),
    }
}

/// The typed identity refusal (REP-011/SDK-016/ARCH-004): the request's
/// binding named a database/incarnation/schema the selected resource does
/// not hold. Spelled exactly like the open path's refusals — a wrong
/// incarnation of the SAME database is `WrongLineage`; everything else is
/// `ForeignIdentity`.
fn fail_of_identity(mismatch: &bumbledb_log::admin::IdentityMismatch) -> LogFail {
    match mismatch.dimension() {
        "incarnation" => protocol(
            "WrongLineage",
            format!(
                "the resource holds a different incarnation of this database (requested {}, \
                 found {})",
                uuid_text(mismatch.expected.incarnation_id.as_core()),
                uuid_text(mismatch.actual.incarnation_id.as_core()),
            ),
        ),
        "schema" => protocol(
            "ForeignIdentity",
            format!(
                "the resource records a different schema for this database (requested {}, \
                 found {})",
                hex32(&mismatch.expected.schema_id.0),
                hex32(&mismatch.actual.schema_id.0),
            ),
        ),
        _ => protocol(
            "ForeignIdentity",
            format!(
                "the resource belongs to a different database (requested {}, found {})",
                uuid_text(mismatch.expected.database_id.as_core()),
                uuid_text(mismatch.actual.database_id.as_core()),
            ),
        ),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn fail_of_local_root(error: bumbledb_log::local_roots::LocalRootError) -> LogFail {
    use bumbledb_log::local_roots::LocalRootError;
    match &error {
        LocalRootError::RootCapacityExceeded => {
            protocol("RootCapacityExceeded", "named-root capacity reached")
        }
        LocalRootError::DuplicateRoot => {
            protocol("OperationConflict", "root id already registered")
        }
        LocalRootError::UnknownRoot => protocol("Misuse", "unknown root id"),
        _ => protocol("Corruption", format!("{error:?}")),
    }
}

#[allow(clippy::needless_pass_by_value)]
/// One owned checkpoint chunk at a time. Restore borrows via `AsRef<[u8]>`
/// and drops the owner as it consumes the iterator.
pub(crate) fn verified_checkpoint_chunks<'a, B>(
    store: &'a B,
    prefix: &'a str,
    checkpoint: &'a bumbledb_log::codec::CheckpointManifest,
    context: &'a WorkContext,
) -> impl Iterator<Item = Result<bumbledb_log::store::ReceivedBody, RecoveryError>> + 'a
where
    B: bumbledb_log::store::ReceivingStore,
    B::Error: bumbledb_log::store::BackendError + bumbledb_log::store::ObservedError,
{
    checkpoint.chunks.iter().map(move |chunk_ref| {
        context.checkpoint().map_err(RecoveryError::Work)?;
        get_verified(
            store,
            prefix,
            chunk_ref,
            TransportContext::new(
                context,
                receive_limits_for_object(chunk_ref, LIMITS.envelope_bytes),
            ),
        )
        .map_err(RecoveryError::Object)
    })
}

#[allow(clippy::too_many_lines)]
fn run_restore(
    runtime: &Arc<Runtime>,
    source: &DestinationSpec,
    target: &BindingSpec,
    operation: OperationId,
    backup: Option<OperationId>,
    context: &WorkContext,
) -> MachineResult<AdminOwned> {
    let _ = runtime;
    match target.backend {
        BackendSpec::Local => {}
        BackendSpec::Hosted { .. } => {
            return Ok(AdminOwned::Failed {
                fail: protocol(
                    "UnsupportedArtifact",
                    "restore targets a local binding; hosted re-publication is the \
                     recorded C08 boundary",
                ),
                phase: PublicationPhase::Prepared,
            });
        }
    }
    let Some(backup) = backup else {
        return Ok(AdminOwned::Failed {
            fail: protocol("Misuse", "restore needs the backup operation id (`backup`)"),
            phase: PublicationPhase::Prepared,
        });
    };
    let Some((descriptor, _attrs)) = target.descriptor.clone() else {
        return Ok(AdminOwned::Failed {
            fail: protocol(
                "Misuse",
                "restore needs the target `schema` (lowered SchemaSpec)",
            ),
            phase: PublicationPhase::Prepared,
        });
    };
    // The restore target must be OURS to write: hold the tenant directory's
    // kernel fence for the whole restore, and never overwrite an existing
    // materialization. A completed retry resolves its native evidence; a
    // foreign operation/source remains a no-clobber refusal.
    std::fs::create_dir_all(&target.directory)
        .map_err(|error| LogFail::Core(crate::runtime::owners::io_error(error)))?;
    let target_dir = Path::new(&target.directory);
    let _target_fence = acquire_directory(target_dir).map_err(|error| {
        if error.kind() == std::io::ErrorKind::WouldBlock {
            LogFail::Core(RuntimeError::DirectoryBusy)
        } else {
            LogFail::Core(crate::runtime::owners::io_error(error))
        }
    })?;
    let destination = store_of_destination(source)?;
    let restored = with_store!(destination, dest_prefix, store => {
        let (manifest, manifest_digest) =
            bumbledb_log::backup::read_backup_manifest(store, &dest_prefix, backup, context).map_err(
                |error| LogFail::Protocol {
                    code: "Corruption",
                    detail: format!("{error:?}"),
                },
            )?;
        // The backup must be an artifact of exactly the database/schema the
        // target binding names (the incarnation is NEW by design) — refused
        // typed BEFORE any byte lands in the target directory.
        let normalized = DatabaseIdentity {
            incarnation_id: target.identity.incarnation_id,
            ..manifest.identity
        };
        if normalized != target.identity {
            return Err(fail_of_identity(&bumbledb_log::admin::IdentityMismatch {
                expected: target.identity,
                actual: normalized,
            }));
        }
        if recovery::materialization_path(target_dir).exists() {
            let db = bumbledb::Db::open(&recovery::materialization_path(target_dir), descriptor.clone(), context.clone())
                .map_err(|error| LogFail::Core(crate::runtime::session::engine_error(&error)))?;
            let genesis = bumbledb_log::restore::resolve_writable(&db, &expected_binding(target), operation, manifest_digest, LIMITS.envelope_bytes, context)
                .map_err(|error| protocol("Corruption", format!("{error:?}")))?;
            return match genesis {
                Some(genesis) => Ok(AdminOwned::Completed(AdminValueOwned::Restore {
                    identity: target.identity,
                    genesis: *genesis.hash.as_bytes(),
                    directory: target.directory.clone(),
                })),
                None => Ok(AdminOwned::Failed {
                    fail: protocol("AuthorityExists", "restore target belongs to another operation or source artifact"),
                    phase: PublicationPhase::Prepared,
                }),
            };
        }
        let Some(checkpoint_ref) = manifest.checkpoint else {
            return Err(protocol(
                "UnsupportedArtifact",
                "genesis-root backups (whole-chain tails) restore through the recorded \
                 C08 boundary",
            ));
        };
        let checkpoint_bytes = get_verified(
            store,
            &dest_prefix,
            &checkpoint_ref,
            TransportContext::new(
                context,
                receive_limits_for_object(&checkpoint_ref, LIMITS.envelope_bytes),
            ),
        )
        .map_err(|error| LogFail::Protocol {
            code: "Corruption",
            detail: format!("{error:?}"),
        })?;
        let checkpoint =
            bumbledb_log::codec::decode_manifest(&checkpoint_bytes, bumbledb_log::codec::StreamLimits::DEFAULT)
                .map_err(|error| LogFail::Protocol {
                    code: "Corruption",
                    detail: format!("{error:?}"),
                })?;
        drop(checkpoint_bytes);
        let chunks = verified_checkpoint_chunks(store, &dest_prefix, &checkpoint, context);
        let tail = bumbledb_log::backup::relocated_tail(
            store,
            &dest_prefix,
            &manifest,
            LIMITS,
            context,
        )
        .map(|item| item.map_err(RecoveryError::from));
        let ready = recovery::materialization_path(Path::new(&target.directory));
        bumbledb_log::restore::restore_writable_with_tail(
            &ready,
            descriptor,
            &checkpoint,
            chunks,
            tail,
            manifest.tip,
            target.identity.incarnation_id,
            operation,
            manifest_digest,
            "local",
            &target.directory,
            LIMITS,
            &CheckpointPolicy::DEFAULT,
            LIMITS.envelope_bytes,
            context,
        )
        .map_err(|error| LogFail::Protocol {
            code: "Corruption",
            detail: format!("{error:?}"),
        })?
    });
    let genesis = match &restored.authority.lifecycle {
        Lifecycle::Live(live) => *live.decision.hash.as_bytes(),
        Lifecycle::Deleted { .. } => [0u8; 32],
    };
    Ok(AdminOwned::Completed(AdminValueOwned::Restore {
        identity: restored.identity,
        genesis,
        directory: target.directory.clone(),
    }))
}

// ---------------------------------------------------------------------------
// Rendering the certainty envelope.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
// The returned object's brand lifetime is deliberately NOT tied to the
// `&Env` borrow: napi3's `Object::new(&Env)` leaves the brand free, and the
// take verbs pass a borrow of their own by-value `Env` (tying would be
// E0515 — returning a value referencing a local).
pub(crate) fn admin_wire<'e>(env: Env, owned: AdminOwned) -> napi::Result<Object<'e>> {
    let mut wire = Object::new(&env)?;
    let value = match owned {
        AdminOwned::Completed(value) => {
            wire.set("certainty", "completed")?;
            value
        }
        AdminOwned::Report(value) => {
            wire.set("certainty", "report")?;
            value
        }
        AdminOwned::Failed { fail, phase } => {
            wire.set("certainty", admin_failed_certainty(phase))?;
            wire.set("error", frame_object(&env, &fail)?)?;
            return Ok(wire);
        }
    };
    let mut body = Object::new(&env)?;
    match value {
        AdminValueOwned::Checkpoint { at, state, root } => {
            body.set("verb", "checkpoint")?;
            body.set("at", stamp_wire(&env, at)?)?;
            body.set("state", state_wire(&env, state)?)?;
            body.set("root", root)?;
        }
        AdminValueOwned::PinRoot { root, at, state } => {
            body.set("verb", "pin-root")?;
            body.set("root", root)?;
            body.set("at", stamp_wire(&env, at)?)?;
            body.set("state", state_wire(&env, state)?)?;
        }
        AdminValueOwned::ReleaseRoot {
            root,
            was_current_recovery_base,
        } => {
            body.set("verb", "release-root")?;
            body.set("root", root)?;
            body.set("wasCurrentRecoveryBase", was_current_recovery_base)?;
        }
        AdminValueOwned::RotateEpoch { open_epoch } => {
            body.set("verb", "rotate-receipt-epoch")?;
            body.set("openEpoch", BigInt::from(open_epoch))?;
        }
        AdminValueOwned::RetireReceipts { retired_through } => {
            body.set("verb", "retire-receipts")?;
            body.set("retiredThrough", BigInt::from(retired_through))?;
        }
        AdminValueOwned::CollectGarbage {
            object_epoch,
            swept,
            orphans_observed,
        } => {
            body.set("verb", "collect-garbage")?;
            body.set("objectEpoch", BigInt::from(object_epoch))?;
            body.set("swept", BigInt::from(swept))?;
            body.set("orphansObserved", BigInt::from(orphans_observed))?;
        }
        AdminValueOwned::Backup {
            manifest_digest,
            objects,
            bytes,
            at,
        } => {
            body.set("verb", "backup")?;
            body.set("manifestDigest", hex32(&manifest_digest))?;
            body.set("objects", BigInt::from(objects))?;
            body.set("bytes", BigInt::from(bytes))?;
            body.set("at", stamp_wire(&env, at)?)?;
        }
        AdminValueOwned::VerifyBackup {
            identity,
            at,
            state,
            objects,
            bytes,
            manifest_digest,
        } => {
            body.set("verb", "verify-backup")?;
            body.set("identity", identity_wire(&env, identity)?)?;
            body.set("at", stamp_wire(&env, at)?)?;
            body.set("state", state_wire(&env, state)?)?;
            body.set("objects", BigInt::from(objects))?;
            body.set("bytes", BigInt::from(bytes))?;
            body.set("manifestDigest", hex32(&manifest_digest))?;
        }
        AdminValueOwned::Restore {
            identity,
            genesis,
            directory,
        } => {
            body.set("verb", "restore")?;
            body.set("identity", identity_wire(&env, identity)?)?;
            body.set("genesis", hex32(&genesis))?;
            let mut binding = Object::new(&env)?;
            binding.set("kind", "local")?;
            binding.set("directory", directory)?;
            binding.set("identity", identity_wire(&env, identity)?)?;
            body.set("binding", binding)?;
        }
        AdminValueOwned::Erase {
            tombstoned,
            retained_roots,
            residual,
        } => {
            body.set("verb", "erase")?;
            body.set("tombstoned", tombstoned)?;
            body.set("retainedRoots", retained_roots)?;
            let mut rows = Vec::with_capacity(residual.len());
            for (kind, location) in residual {
                let mut row = Object::new(&env)?;
                row.set("kind", kind)?;
                row.set("location", location)?;
                rows.push(row);
            }
            body.set("residual", rows)?;
        }
    }
    wire.set("value", body)?;
    Ok(wire)
}

const _: fn() = || {
    // Compile locks: the admin machinery's owned values must cross threads.
    fn assert_send<T: Send>() {}
    assert_send::<AdminOwned>();
    let _ = AtomicBool::new(false);
};

#[cfg(test)]
#[path = "admin_identity_tests.rs"]
mod admin_identity_tests;
