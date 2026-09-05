//! The one `logAdmin` verb family (C08/C11): maintenance, retention,
//! backup/restore/erase and the migration workflow, over P05's admin/
//! checkpointer/gc/backup/restore/erase modules and P09's migration
//! executor. Every request derives its ref-able identity BEFORE dispatch on
//! the TS side; the native side classifies its own refusals into the
//! certainty union: refusals that PROVABLY dispatched no mutation return
//! `not-started`, ambiguous hosted outcomes return `outcome-unknown`, and
//! successes return `completed`/`report` values. Nothing here manufactures
//! a receipt or resolves uncertainty by guessing.

use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use bumbledb::SchemaDescriptor;
use bumbledb::work::WorkContext;
use bumbledb_log::checkpointer::{CheckpointKind, CheckpointOutcome, CheckpointPolicy};
use bumbledb_log::codec::StreamLimits;
use bumbledb_log::gc::GcPolicy;
use bumbledb_log::history::authority::{Access, Lifecycle};
use bumbledb_log::history::{
    DatabaseIdentity, DecisionStamp, OperationId, ReceiptEpoch, StateStamp,
};
use bumbledb_log::manifest::RootPolicy;
use bumbledb_log::migration::executor::{
    AbortRequest, ActivationRef, LocalMigration, MigrateOutcome, MigrationStatus, StepInput,
    SuffixRequest, activate_target, initialize,
};
use bumbledb_log::migration::manifest::{Manifest, parse_manifest, prefix_at};
use bumbledb_log::migration::plan::parse_plan;
use bumbledb_log::recovery::{self, RecoveryError};
use bumbledb_log::store::fence::acquire_directory;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::s3::S3Store;
use bumbledb_log::writer::LocalHistory;
use napi::bindgen_prelude::{BigInt, Env, External, Function, Object};

use crate::marshal;
use crate::runtime::owners::DbLease;
use crate::runtime::{Output, Runtime, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, PolicyWire, RuntimeHandle, notification, operation_handle,
    owner as runtime_owner, thrown,
};

use super::{
    BackendSpec, CredentialsSpec, LIMITS, LogFail, MachineOutput, MachineResult, binding_spec_in,
    fail_of_log, frame_object, hex16, hex32, identity_in, identity_wire, optional_object,
    optional_string, protocol, s3_store, stamp_wire, state_wire, targets_root,
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
        dispatched: bool,
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
    MigrationStatus(StatusOwned),
    MigrationInitialize {
        directory: String,
        identity: DatabaseIdentity,
        genesis: [u8; 32],
    },
    Migrate(MigrateOwned),
    MigrationActivate {
        target: DatabaseIdentity,
        access: &'static str,
        operation: OperationId,
        activated_now: bool,
    },
    MigrationAbort {
        target: DatabaseIdentity,
        target_fenced: bool,
        source_access: &'static str,
    },
}

pub enum StatusOwned {
    UpToDate {
        applied_prefix: [u8; 32],
    },
    Pending {
        pending: Vec<String>,
    },
    InProgress {
        source: DatabaseIdentity,
        operation: OperationId,
        plan_set: [u8; 32],
        target: DatabaseIdentity,
    },
    Aborted {
        source: DatabaseIdentity,
        operation: OperationId,
        plan_set: [u8; 32],
        target: DatabaseIdentity,
    },
}

pub enum MigrateOwned {
    UpToDate {
        directory: String,
        identity: DatabaseIdentity,
    },
    ReadyToSwitch {
        deployment_directory: String,
        target: DatabaseIdentity,
        activation: ActivationRef,
    },
    Paused {
        fail: LogFail,
        operation: Option<OperationId>,
    },
}

// ---------------------------------------------------------------------------
// Request parsing (JS thread) into one owned Send request.
// ---------------------------------------------------------------------------

pub(crate) struct PlansSpec {
    manifest_text: String,
    plan_texts: Vec<String>,
    /// Canonical schema snapshots (`schema_file::render` texts): the base
    /// schema first, then each entry's TARGET schema, order-matched —
    /// entries + 1 rows. Requested `PlansWire` extension (P08/P10); absent
    /// snapshots refuse the verbs that must compile steps.
    snapshots: Vec<String>,
}

fn plans_in(obj: &Object, ctx: &str) -> napi::Result<PlansSpec> {
    // The wire carries manifest FIELDS + plan bodies; the native side
    // re-renders the manifest from its fields through the one canonical
    // grammar by reconstructing the manifest JSON text.
    let manifest_version =
        marshal::ordinal(marshal::req::<f64>(obj, "manifestVersion", ctx)?, ctx)?;
    let plan_version = marshal::ordinal(marshal::req::<f64>(obj, "planVersion", ctx)?, ctx)?;
    let base_schema: String = marshal::req(obj, "baseSchemaId", ctx)?;
    let base_prefix: String = marshal::req(obj, "basePrefixDigest", ctx)?;
    let entries: napi::bindgen_prelude::Array = marshal::req(obj, "entries", ctx)?;
    let mut manifest_text = String::from("{\n");
    let _ = writeln!(manifest_text, "  \"manifestVersion\": {manifest_version},");
    let _ = writeln!(manifest_text, "  \"planVersion\": {plan_version},");
    let _ = writeln!(manifest_text, "  \"baseSchemaId\": \"{base_schema}\",");
    let _ = writeln!(manifest_text, "  \"basePrefixDigest\": \"{base_prefix}\",");
    manifest_text.push_str("  \"entries\": [");
    for index in 0..entries.len() {
        let entry = marshal::req_at::<Object>(&entries, index, ctx)?;
        let sequence: String = marshal::req(&entry, "sequence", ctx)?;
        let id: String = marshal::req(&entry, "id", ctx)?;
        let from: String = marshal::req(&entry, "fromSchemaId", ctx)?;
        let to: String = marshal::req(&entry, "toSchemaId", ctx)?;
        let plan_digest: String = marshal::req(&entry, "planDigest", ctx)?;
        let prefix_digest: String = marshal::req(&entry, "prefixDigest", ctx)?;
        if index > 0 {
            manifest_text.push(',');
        }
        let _ = write!(
            manifest_text,
            "\n    {{\n      \"sequence\": \"{sequence}\",\n      \"id\": \"{id}\",\n      \
             \"fromSchemaId\": \"{from}\",\n      \"toSchemaId\": \"{to}\",\n      \
             \"planDigest\": \"{plan_digest}\",\n      \"prefixDigest\": \"{prefix_digest}\"\n    }}"
        );
    }
    if entries.len() > 0 {
        manifest_text.push_str("\n  ");
    }
    manifest_text.push_str("]\n}\n");
    let plans_arr: napi::bindgen_prelude::Array = marshal::req(obj, "plans", ctx)?;
    let mut plan_texts = Vec::with_capacity(plans_arr.len() as usize);
    for index in 0..plans_arr.len() {
        plan_texts.push(marshal::req_at::<String>(&plans_arr, index, ctx)?);
    }
    let mut snapshots = Vec::new();
    if let Some(snapshot_obj) = optional_object(obj, "snapshots")? {
        // Arrays are objects; re-read the property as the typed array.
        let _ = snapshot_obj;
        if let Some(snapshot_arr) = obj.get::<napi::bindgen_prelude::Array>("snapshots")? {
            for index in 0..snapshot_arr.len() {
                snapshots.push(marshal::req_at::<String>(&snapshot_arr, index, ctx)?);
            }
        }
    }
    Ok(PlansSpec {
        manifest_text,
        plan_texts,
        snapshots,
    })
}

impl PlansSpec {
    fn manifest(&self) -> MachineResult<Manifest> {
        parse_manifest(&self.manifest_text, LIMITS.envelope_bytes)
            .map_err(|error| protocol("MigrationDrift", format!("{error:?}")))
    }

    fn plans(&self) -> MachineResult<Vec<bumbledb_log::migration::plan::Plan>> {
        self.plan_texts
            .iter()
            .map(|text| {
                parse_plan(text).map_err(|error| protocol("MigrationDrift", format!("{error:?}")))
            })
            .collect()
    }

    /// Parsed schema snapshots: base first, then one per entry.
    fn descriptors(&self) -> MachineResult<Vec<SchemaDescriptor>> {
        if self.snapshots.is_empty() {
            return Err(protocol(
                "UnsupportedArtifact",
                "migration execution requires the schema snapshots (PlansWire.snapshots — \
                 base first, then each entry's target)",
            ));
        }
        self.snapshots
            .iter()
            .map(|text| {
                bumbledb_log::schema_file::parse(text)
                    .map_err(|error| protocol("UnsupportedArtifact", format!("{error:?}")))
            })
            .collect()
    }
}

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
    Ok(OperationId::from_core(marshal::id128_in(
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
    MigrationStatus {
        binding: BindingSpec,
        plans: PlansSpec,
    },
    MigrationInitialize {
        binding: BindingSpec,
        operation: OperationId,
        plans: PlansSpec,
    },
    Migrate {
        binding: BindingSpec,
        operation: OperationId,
        plans: PlansSpec,
        to: Option<String>,
    },
    MigrationActivate {
        binding: Option<BindingSpec>,
        reference: ActivationRefSpec,
    },
    MigrationAbort {
        binding: Option<BindingSpec>,
        reference: MigrationRefSpec,
    },
}

struct ActivationRefSpec {
    operation: OperationId,
    plan_set_digest: [u8; 32],
    target: DatabaseIdentity,
    target_genesis: [u8; 32],
}

struct MigrationRefSpec {
    /// The ref's SOURCE identity: parsed (wire-shape validation) but the
    /// abort verb locates everything by operation/target — never read.
    _identity: DatabaseIdentity,
    operation: OperationId,
    plan_set_digest: [u8; 32],
    target: DatabaseIdentity,
}

fn optional_binding_in(env: Env, request: &Object, ctx: &str) -> napi::Result<Option<BindingSpec>> {
    if optional_object(request, "binding")?.is_none() {
        return Ok(None);
    }
    Ok(Some(binding_with_schema_in(env, request, ctx)?))
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
            root: OperationId::from_core(marshal::id128_in(
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
                .map(|hex| marshal::id128_in(&hex, ctx).map(OperationId::from_core))
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
                .map(|hex| marshal::id128_in(&hex, ctx).map(OperationId::from_core))
                .transpose()?,
        },
        "erase" => {
            let retain: napi::bindgen_prelude::Array = marshal::req(request, "retainRoots", ctx)?;
            let mut retain_roots = Vec::with_capacity(retain.len() as usize);
            for index in 0..retain.len() {
                retain_roots.push(OperationId::from_core(marshal::id128_in(
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
        "migration-status" => AdminVerb::MigrationStatus {
            binding: binding_with_schema_in(env, request, ctx)?,
            plans: plans_in(&marshal::req::<Object>(request, "plans", ctx)?, ctx)?,
        },
        "migration-initialize" => AdminVerb::MigrationInitialize {
            binding: binding_with_schema_in(env, request, ctx)?,
            operation: operation_in(request, ctx)?,
            plans: plans_in(&marshal::req::<Object>(request, "plans", ctx)?, ctx)?,
        },
        "migration-migrate" => AdminVerb::Migrate {
            binding: binding_with_schema_in(env, request, ctx)?,
            operation: operation_in(request, ctx)?,
            plans: plans_in(&marshal::req::<Object>(request, "plans", ctx)?, ctx)?,
            to: optional_string(request, "to")?,
        },
        "migration-activate" => {
            let reference: Object = marshal::req(request, "ref", ctx)?;
            AdminVerb::MigrationActivate {
                binding: optional_binding_in(env, request, ctx)?,
                reference: ActivationRefSpec {
                    operation: OperationId::from_core(marshal::id128_in(
                        &marshal::req::<String>(&reference, "operationId", ctx)?,
                        ctx,
                    )?),
                    plan_set_digest: super::fingerprint_of_hex(&marshal::req::<String>(
                        &reference,
                        "planSetDigest",
                        ctx,
                    )?)?
                    .0,
                    target: identity_in(&marshal::req::<Object>(&reference, "target", ctx)?, ctx)?,
                    target_genesis: super::fingerprint_of_hex(&marshal::req::<String>(
                        &reference,
                        "targetGenesis",
                        ctx,
                    )?)?
                    .0,
                },
            }
        }
        "migration-abort" => {
            let reference: Object = marshal::req(request, "ref", ctx)?;
            AdminVerb::MigrationAbort {
                binding: optional_binding_in(env, request, ctx)?,
                reference: MigrationRefSpec {
                    _identity: identity_in(
                        &marshal::req::<Object>(&reference, "identity", ctx)?,
                        ctx,
                    )?,
                    operation: OperationId::from_core(marshal::id128_in(
                        &marshal::req::<String>(&reference, "operationId", ctx)?,
                        ctx,
                    )?),
                    plan_set_digest: super::fingerprint_of_hex(&marshal::req::<String>(
                        &reference,
                        "planSetDigest",
                        ctx,
                    )?)?
                    .0,
                    target: identity_in(&marshal::req::<Object>(&reference, "target", ctx)?, ctx)?,
                },
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
    policy: PolicyWire,
    request: &Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = runtime_owner(handle).map_err(|error| thrown(env, error))?;
    let verb = admin_verb_in(env, request)?;
    let shared = Arc::clone(runtime);
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context| {
                    let owned = match run_admin(&shared, verb, context) {
                        Ok(owned) => owned,
                        Err(LogFail::Core(core)) => return Err(core),
                        Err(fail) => AdminOwned::Failed {
                            fail,
                            dispatched: false,
                        },
                    };
                    Ok(Output::Machine(MachineOutput::Admin(owned)))
                }))
            },
        )
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

fn open_admin_db(runtime: &Arc<Runtime>, binding: &BindingSpec) -> MachineResult<AdminDb> {
    let directory = Path::new(&binding.directory);
    if let Some(lease) = runtime.lease_database_at(directory)? {
        return Ok(AdminDb::Leased(lease));
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
    let db = crate::Engine::open(&ready, descriptor)
        .map_err(|error| LogFail::Core(crate::runtime::session::engine_error(&error)))?;
    Ok(AdminDb::Transient {
        db: Arc::new(db),
        _lock: held,
    })
}

fn local_history_of(db: &AdminDb) -> MachineResult<LocalHistory<SchemaDescriptor>> {
    let arc = match db {
        AdminDb::Leased(lease) => Arc::clone(&lease.inner_arc().db),
        AdminDb::Transient { db, .. } => Arc::clone(db),
    };
    LocalHistory::open(arc, LIMITS).map_err(fail_of_log)
}

fn access_of(db: &AdminDb) -> MachineResult<(&'static str, Option<OperationId>)> {
    let authority = bumbledb_log::admin::local_authority(db.db(), LIMITS.envelope_bytes)
        .map_err(|error| protocol("Corruption", format!("{error:?}")))?;
    Ok(match &authority.lifecycle {
        Lifecycle::Live(live) => match live.access {
            Access::Active => ("active", None),
            Access::Frozen { operation, .. } => ("frozen", Some(operation)),
        },
        Lifecycle::Deleted { .. } => ("deleted", None),
    })
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
            let Some((backend, prefix)) = store_of_backend(&binding.backend)? else {
                return Ok(AdminOwned::Failed {
                    fail: protocol(
                        "Misuse",
                        "LocalHistory needs no checkpoint: LMDB is complete; named restore \
                         points (pin-root) are the local specialization",
                    ),
                    dispatched: false,
                });
            };
            let db = open_admin_db(runtime, &binding)?;
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
                    dispatched: true,
                }),
            }
        }
        AdminVerb::PinRoot {
            binding,
            operation,
            label,
        } => match &binding.backend {
            BackendSpec::Local => {
                let db = open_admin_db(runtime, &binding)?;
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
                    root: hex16(root.id.as_core()),
                    at: root.decision,
                    state: root.state,
                }))
            }
            BackendSpec::Hosted { .. } => {
                let (backend, prefix) =
                    store_of_backend(&binding.backend)?.expect("hosted backend");
                let root = bumbledb_log::admin::add_named_root_hosted(
                    &backend,
                    &prefix,
                    operation,
                    bumbledb_log::manifest::RootKind::RestorePoint,
                    &label,
                    operation,
                    &RootPolicy::DEFAULT,
                    LIMITS.envelope_bytes,
                    context,
                )
                .map_err(fail_of_admin)?;
                Ok(AdminOwned::Completed(AdminValueOwned::PinRoot {
                    root: hex16(root.id.as_core()),
                    at: root.recovery.base,
                    state: root.state,
                }))
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
                    let db = open_admin_db(runtime, &binding)?;
                    let report = bumbledb_log::local_roots::release_restore_point(
                        db.db(),
                        Path::new(&binding.directory),
                        root,
                        context,
                    )
                    .map_err(fail_of_local_root)?;
                    let _ = report;
                    Ok(AdminOwned::Completed(AdminValueOwned::ReleaseRoot {
                        root: hex16(root.as_core()),
                        was_current_recovery_base: false,
                    }))
                }
                BackendSpec::Hosted { .. } => {
                    let (backend, prefix) =
                        store_of_backend(&binding.backend)?.expect("hosted backend");
                    let released = bumbledb_log::admin::release_named_root_hosted(
                        &backend,
                        &prefix,
                        root,
                        true,
                        LIMITS.envelope_bytes,
                        context,
                    )
                    .map_err(fail_of_admin)?;
                    Ok(AdminOwned::Completed(AdminValueOwned::ReleaseRoot {
                        root: hex16(root.as_core()),
                        was_current_recovery_base: released.is_some(),
                    }))
                }
            }
        }
        AdminVerb::RotateEpoch { binding, operation } => {
            let _ = operation;
            let db = open_admin_db(runtime, &binding)?;
            let authority = bumbledb_log::admin::local_authority(db.db(), LIMITS.envelope_bytes)
                .map_err(|error| protocol("Corruption", format!("{error:?}")))?;
            let current = match &authority.lifecycle {
                Lifecycle::Live(live) => live.receipts.open_epoch().get(),
                Lifecycle::Deleted { .. } => {
                    return Ok(AdminOwned::Failed {
                        fail: protocol("DatabaseDeleted", "terminal tombstone"),
                        dispatched: false,
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
                }
                BackendSpec::Hosted { .. } => {
                    let (backend, prefix) =
                        store_of_backend(&binding.backend)?.expect("hosted backend");
                    bumbledb_log::admin::rotate_receipts_hosted(
                        &backend,
                        &prefix,
                        next,
                        LIMITS.envelope_bytes,
                        context,
                    )
                    .map_err(fail_of_admin)?;
                }
            }
            Ok(AdminOwned::Completed(AdminValueOwned::RotateEpoch {
                open_epoch: next.get(),
            }))
        }
        AdminVerb::RetireReceipts {
            binding,
            operation,
            through,
        } => {
            let _ = operation;
            match &binding.backend {
                BackendSpec::Local => {
                    let db = open_admin_db(runtime, &binding)?;
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
                    // promising the rows (C08), then applies locally.
                    let (backend, prefix) =
                        store_of_backend(&binding.backend)?.expect("hosted backend");
                    let db = open_admin_db(runtime, &binding)?;
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
                            )
                            .map_err(|error| protocol("Backend", format!("{error:?}")))?;
                            bumbledb_log::admin::apply_hosted_retirement_locally(
                                db.db(),
                                &head.control,
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
                            dispatched: true,
                        }),
                    }
                }
            }
        }
        AdminVerb::CollectGarbage { binding, operation } => {
            let Some((backend, prefix)) = store_of_backend(&binding.backend)? else {
                return Ok(AdminOwned::Failed {
                    fail: protocol("Misuse", "LocalHistory holds no object store to collect"),
                    dispatched: false,
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
                        dispatched: true,
                    });
                }
            };
            let object_epoch = bumbledb_log::checkpointer::read_live_head(
                &backend,
                &prefix,
                LIMITS.envelope_bytes,
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
            let Some((backend, prefix)) = store_of_backend(&binding.backend)? else {
                return Ok(AdminOwned::Failed {
                    fail: protocol(
                        "Misuse",
                        "LocalHistory backup is a named restore point (pin-root): the \
                         self-contained root directory IS the backup artifact",
                    ),
                    dispatched: false,
                });
            };
            let (head, _) = bumbledb_log::checkpointer::read_live_head(
                &backend,
                &prefix,
                LIMITS.envelope_bytes,
            )
            .map_err(|error| protocol("Backend", format!("{error:?}")))?;
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
                    head.object_epoch,
                    operation,
                    LIMITS,
                    StreamLimits::DEFAULT,
                    context,
                )
                .map_err(|error| LogFail::Protocol {
                    code: "Backend",
                    detail: format!("{error:?}"),
                })?
            });
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
                    dispatched: false,
                });
            };
            let destination = store_of_destination(&destination)?;
            let (manifest, digest, report) = with_store!(destination, dest_prefix, store => {
                let (manifest, digest) =
                    bumbledb_log::backup::read_backup_manifest(store, &dest_prefix, backup)
                        .map_err(|error| LogFail::Protocol {
                            code: "Corruption",
                            detail: format!("{error:?}"),
                        })?;
                let report = bumbledb_log::backup::verify_backup(
                    store,
                    &dest_prefix,
                    backup,
                    LIMITS,
                    StreamLimits::DEFAULT,
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
        } => match store_of_backend(&binding.backend)? {
            None => {
                let db = open_admin_db(runtime, &binding)?;
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
                        .map(|root| hex16(root.as_core()))
                        .collect(),
                    residual: vec![("local-directory".to_string(), binding.directory.clone())],
                }))
            }
            Some((backend, prefix)) => {
                // Release exactly the roots NOT retained.
                let (head, _) = bumbledb_log::checkpointer::read_live_head(
                    &backend,
                    &prefix,
                    LIMITS.envelope_bytes,
                )
                .map_err(|error| protocol("Backend", format!("{error:?}")))?;
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
                        return Ok(AdminOwned::failed_hosted(format!("{error:?}")));
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
                        .map(|(id, _)| hex16(id.as_core()))
                        .collect(),
                    residual,
                }))
            }
        },
        AdminVerb::MigrationStatus { binding, plans } => {
            migration_status(runtime, &binding, &plans, context)
        }
        AdminVerb::MigrationInitialize {
            binding,
            operation,
            plans,
        } => migration_initialize(&binding, operation, &plans, context),
        AdminVerb::Migrate {
            binding,
            operation,
            plans,
            to,
        } => migrate(runtime, &binding, operation, &plans, to.as_deref(), context),
        AdminVerb::MigrationActivate { binding, reference } => {
            let Some(binding) = binding else {
                return Ok(AdminOwned::Failed {
                    fail: protocol(
                        "Misuse",
                        "migration-activate needs the source `binding` (and `schema` for \
                         the target descriptor)",
                    ),
                    dispatched: false,
                });
            };
            let Some((descriptor, _)) = binding.descriptor.clone() else {
                return Ok(AdminOwned::Failed {
                    fail: protocol(
                        "Misuse",
                        "migration-activate needs the target `schema` (lowered SchemaSpec)",
                    ),
                    dispatched: false,
                });
            };
            let reference = ActivationRef {
                operation: reference.operation,
                plan_set_digest: reference.plan_set_digest,
                target: reference.target,
                target_genesis: bumbledb_log::history::DecisionDigest::from_bytes(
                    reference.target_genesis,
                ),
            };
            let report = activate_target(
                &targets_root(&binding.directory),
                &reference,
                &descriptor,
                LIMITS,
                context,
            )
            .map_err(fail_of_migration)?;
            let access = match report.access {
                bumbledb_log::history::AccessMode::Active => "active",
                bumbledb_log::history::AccessMode::Frozen => "frozen",
                bumbledb_log::history::AccessMode::Deleted => "deleted",
            };
            let activated_now = matches!(
                report.activation,
                bumbledb_log::history::authority::Activation::Activated { .. }
            );
            Ok(AdminOwned::Completed(AdminValueOwned::MigrationActivate {
                target: reference.target,
                access,
                operation: reference.operation,
                activated_now,
            }))
        }
        AdminVerb::MigrationAbort { binding, reference } => {
            let Some(binding) = binding else {
                return Ok(AdminOwned::Failed {
                    fail: protocol(
                        "Misuse",
                        "migration-abort needs the source `binding` (and `schema` for the \
                         target descriptor)",
                    ),
                    dispatched: false,
                });
            };
            let Some((descriptor, _)) = binding.descriptor.clone() else {
                return Ok(AdminOwned::Failed {
                    fail: protocol(
                        "Misuse",
                        "migration-abort needs the target `schema` (lowered SchemaSpec)",
                    ),
                    dispatched: false,
                });
            };
            let db = open_admin_db(runtime, &binding)?;
            let history = local_history_of(&db)?;
            let runner = LocalMigration::new(&history, &targets_root(&binding.directory), LIMITS);
            let report = runner
                .abort(
                    &AbortRequest {
                        operation: reference.operation,
                        plan_set_digest: reference.plan_set_digest,
                        target_database: reference.target.database_id,
                        target_incarnation: reference.target.incarnation_id,
                        target_schema: reference.target.schema_id,
                        target_descriptor: &descriptor,
                    },
                    context,
                )
                .map_err(fail_of_migration)?;
            let (source_access, _) = access_of(&db)?;
            // Every `TargetFence` arm of a SUCCESSFUL abort means the target
            // is fenced (tombstoned, deleted, or matching evidence already
            // existed — the idempotent retry): the wire boolean is the state,
            // not this-call attribution (the `activatedNow` precedent).
            let _ = report.fence;
            Ok(AdminOwned::Completed(AdminValueOwned::MigrationAbort {
                target: reference.target,
                target_fenced: true,
                source_access,
            }))
        }
    }
}

impl AdminOwned {
    pub(crate) fn failed_hosted(detail: String) -> Self {
        // A hosted maintenance failure may have dispatched mutations before
        // stopping: durable progress is retained, and the certainty answer
        // is outcome-unknown, never a fabricated completion.
        Self::Failed {
            fail: LogFail::Protocol {
                code: "Backend",
                detail,
            },
            dispatched: true,
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn fail_of_admin(error: bumbledb_log::admin::AdminError) -> LogFail {
    protocol("Backend", format!("{error:?}"))
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
fn fail_of_migration(error: bumbledb_log::migration::executor::MigrationError) -> LogFail {
    use bumbledb_log::migration::executor::MigrationError;
    match &error {
        MigrationError::Aborted { .. }
        | MigrationError::SourceFrozenByOther { .. }
        | MigrationError::ActivationWon
        | MigrationError::StaleActivationRef => protocol("OperationConflict", format!("{error:?}")),
        MigrationError::TargetConflict | MigrationError::OutputMismatch => {
            protocol("MigrationOutputMismatch", format!("{error:?}"))
        }
        MigrationError::Log(log) => fail_of_log(log.clone()),
        // Bounded work/deadline/cancellation stays the exact core reason —
        // never respelled as drift.
        MigrationError::Work(work) => LogFail::Core(RuntimeError::Work(*work)),
        _ => protocol("MigrationDrift", format!("{error:?}")),
    }
}

// ---------------------------------------------------------------------------
// Migration verbs (local bindings; hosted staged data plane is a recorded
// C08 boundary).
// ---------------------------------------------------------------------------

/// Chain positions are u64 on the wire; a position past this host's address
/// space cannot index the in-memory manifest (32-bit hosts) — refuse typed
/// instead of truncating.
fn chain_index(value: u64) -> Result<usize, LogFail> {
    usize::try_from(value)
        .map_err(|_| protocol("Misuse", "chain position exceeds this host's address space"))
}

fn require_local(binding: &BindingSpec) -> MachineResult<()> {
    match binding.backend {
        BackendSpec::Local => Ok(()),
        BackendSpec::Hosted { .. } => Err(protocol(
            "MigrationUnsupported",
            "hosted migration execution awaits the staged hosted data plane (C08); run \
             the migration against the authoritative local materialization",
        )),
    }
}

fn migration_status(
    runtime: &Arc<Runtime>,
    binding: &BindingSpec,
    plans: &PlansSpec,
    context: &WorkContext,
) -> MachineResult<AdminOwned> {
    require_local(binding)?;
    let db = open_admin_db(runtime, binding)?;
    let history = local_history_of(&db)?;
    let manifest = plans.manifest()?;
    let runner = LocalMigration::new(&history, &targets_root(&binding.directory), LIMITS);
    let status = runner
        .status(&manifest, context)
        .map_err(fail_of_migration)?;
    let owned = match status {
        MigrationStatus::UpToDate { applied } => {
            let prefix = prefix_at(&manifest, chain_index(applied)?, LIMITS.envelope_bytes)
                .map_err(|error| protocol("MigrationDrift", format!("{error:?}")))?;
            StatusOwned::UpToDate {
                applied_prefix: prefix,
            }
        }
        MigrationStatus::Pending { applied, pending } => {
            let labels = manifest
                .entries
                .iter()
                .skip(chain_index(applied)?)
                .take(chain_index(pending)?)
                .map(|entry| entry.label.as_str().to_string())
                .collect();
            StatusOwned::Pending { pending: labels }
        }
        MigrationStatus::Frozen {
            operation,
            intent,
            target_cancelled,
            ..
        } => {
            let (target_incarnation, plan_set) = match intent {
                bumbledb_log::history::authority::FreezeIntent::Migration {
                    target,
                    plan_set_digest,
                } => (target, plan_set_digest),
                bumbledb_log::history::authority::FreezeIntent::Erasure => {
                    return Err(protocol(
                        "DatabaseFrozen",
                        "the source is frozen for erasure, not migration",
                    ));
                }
            };
            let target = DatabaseIdentity {
                database_id: history.identity().database_id,
                incarnation_id: target_incarnation,
                schema_id: history.identity().schema_id,
            };
            if target_cancelled {
                StatusOwned::Aborted {
                    source: history.identity(),
                    operation,
                    plan_set,
                    target,
                }
            } else {
                StatusOwned::InProgress {
                    source: history.identity(),
                    operation,
                    plan_set,
                    target,
                }
            }
        }
    };
    Ok(AdminOwned::Report(AdminValueOwned::MigrationStatus(owned)))
}

fn steps_of(
    plans: &PlansSpec,
    manifest: &Manifest,
    first: usize,
    count: usize,
) -> MachineResult<(SchemaDescriptor, Vec<StepInput>)> {
    let descriptors = plans.descriptors()?;
    if descriptors.len() != manifest.entries.len() + 1 {
        return Err(protocol(
            "UnsupportedArtifact",
            "snapshots must carry the base schema plus one target per entry",
        ));
    }
    let parsed = plans.plans()?;
    if parsed.len() != manifest.entries.len() {
        return Err(protocol(
            "MigrationDrift",
            "recorded plans and manifest entries disagree in count",
        ));
    }
    let source = descriptors[first].clone();
    let mut steps = Vec::with_capacity(count);
    for offset in 0..count {
        let index = first + offset;
        steps.push(StepInput {
            plan: parsed[index].clone(),
            to_descriptor: descriptors[index + 1].clone(),
        });
    }
    Ok((source, steps))
}

#[allow(clippy::too_many_lines)]
fn migrate(
    runtime: &Arc<Runtime>,
    binding: &BindingSpec,
    operation: OperationId,
    plans: &PlansSpec,
    to: Option<&str>,
    context: &WorkContext,
) -> MachineResult<AdminOwned> {
    require_local(binding)?;
    let db = open_admin_db(runtime, binding)?;
    let history = local_history_of(&db)?;
    let manifest = plans.manifest()?;
    let root = targets_root(&binding.directory);
    let runner = LocalMigration::new(&history, &root, LIMITS);
    let status = runner
        .status(&manifest, context)
        .map_err(fail_of_migration)?;
    let applied = match status {
        MigrationStatus::UpToDate { applied } => {
            let _ = applied;
            return Ok(AdminOwned::Completed(AdminValueOwned::Migrate(
                MigrateOwned::UpToDate {
                    directory: binding.directory.clone(),
                    identity: history.identity(),
                },
            )));
        }
        MigrationStatus::Pending { applied, .. } => chain_index(applied)?,
        MigrationStatus::Frozen {
            operation: held,
            applied,
            ..
        } => {
            if held != operation {
                return Err(protocol(
                    "OperationConflict",
                    "the source is frozen by a different operation",
                ));
            }
            // Resume under the held freeze: the applied prefix comes from
            // the verified chain; the executor re-verifies everything.
            chain_index(applied)?
        }
    };
    let end = match to {
        None => manifest.entries.len(),
        Some(to) => {
            let target = super::fingerprint_of_hex(to)
                .map_err(|_| protocol("MigrationDrift", "malformed `to` schema id"))?;
            let position = manifest
                .entries
                .iter()
                .position(|entry| entry.to_schema == target)
                .ok_or_else(|| protocol("MigrationDrift", "`to` names no entry's target schema"))?;
            position + 1
        }
    };
    if end <= applied {
        return Ok(AdminOwned::Completed(AdminValueOwned::Migrate(
            MigrateOwned::UpToDate {
                directory: binding.directory.clone(),
                identity: history.identity(),
            },
        )));
    }
    let (source_descriptor, steps) = steps_of(plans, &manifest, applied, end - applied)?;
    let target_incarnation = super::planned_target_incarnation(operation);
    let request = SuffixRequest {
        operation,
        manifest: &manifest,
        source_descriptor,
        steps: &steps,
        target_database: history.identity().database_id,
        target_incarnation,
    };
    match runner.migrate(&request, context) {
        // Already-activated retries and an up-to-date chain answer the same
        // wire value: nothing to run, the tenant is on the target.
        Ok(MigrateOutcome::UpToDate { .. } | MigrateOutcome::AlreadyActivated { .. }) => Ok(
            AdminOwned::Completed(AdminValueOwned::Migrate(MigrateOwned::UpToDate {
                directory: binding.directory.clone(),
                identity: history.identity(),
            })),
        ),
        Ok(MigrateOutcome::ReadyToSwitch { activation_ref, .. }) => {
            let deployment = root.join(hex16(activation_ref.target.incarnation_id.as_core()));
            Ok(AdminOwned::Completed(AdminValueOwned::Migrate(
                MigrateOwned::ReadyToSwitch {
                    deployment_directory: deployment.to_string_lossy().into_owned(),
                    target: activation_ref.target,
                    activation: activation_ref,
                },
            )))
        }
        Err(error) => {
            // A failure after the durable freeze leaves the source frozen —
            // reported HONESTLY as completed(paused), never a silent thaw.
            let (access, held) = access_of(&db)?;
            if access == "frozen" && held == Some(operation) {
                Ok(AdminOwned::Completed(AdminValueOwned::Migrate(
                    MigrateOwned::Paused {
                        fail: fail_of_migration(error),
                        operation: held,
                    },
                )))
            } else {
                Err(fail_of_migration(error))
            }
        }
    }
}

fn migration_initialize(
    binding: &BindingSpec,
    operation: OperationId,
    plans: &PlansSpec,
    context: &WorkContext,
) -> MachineResult<AdminOwned> {
    require_local(binding)?;
    let manifest = plans.manifest()?;
    if manifest.entries.is_empty() {
        return Err(protocol(
            "MigrationDrift",
            "an empty chain initializes nothing",
        ));
    }
    let (source_descriptor, steps) = steps_of(plans, &manifest, 0, manifest.entries.len())?;
    let target_incarnation = binding.identity.incarnation_id;
    let root = targets_root(&binding.directory);
    std::fs::create_dir_all(&root)
        .map_err(|error| LogFail::Core(crate::runtime::owners::io_error(error)))?;
    let request = SuffixRequest {
        operation,
        manifest: &manifest,
        source_descriptor,
        steps: &steps,
        target_database: binding.identity.database_id,
        target_incarnation,
    };
    let outcome = initialize(&root, &request, LIMITS, context).map_err(fail_of_migration)?;
    let activation_ref = match outcome {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => activation_ref,
        MigrateOutcome::AlreadyActivated { .. } | MigrateOutcome::UpToDate { .. } => {
            // Idempotent completion: adopt the recorded evidence below.
            return finish_initialize(binding, operation, &root, None, context);
        }
    };
    let target_descriptor = steps
        .last()
        .map(|step| step.to_descriptor.clone())
        .expect("nonempty steps");
    let report = activate_target(&root, &activation_ref, &target_descriptor, LIMITS, context)
        .map_err(fail_of_migration)?;
    let _ = report;
    finish_initialize(binding, operation, &root, Some(activation_ref), context)
}

/// Installs the activated initialization target as the tenant's ready
/// materialization (`<dir>/db`) and reports the genesis binding — the
/// explicit creation artifact flow (chapter 33's generated-plan
/// `initialize`): seeds ran exactly once inside the executor.
fn finish_initialize(
    binding: &BindingSpec,
    operation: OperationId,
    root: &Path,
    activation: Option<ActivationRef>,
    context: &WorkContext,
) -> MachineResult<AdminOwned> {
    let _ = operation;
    let target_incarnation = binding.identity.incarnation_id;
    let target_dir = root.join(hex16(target_incarnation.as_core()));
    let ready = recovery::materialization_path(Path::new(&binding.directory));
    if !ready.exists() {
        if !target_dir.exists() {
            return Err(protocol(
                "MigrationDrift",
                "no published initialization target to install",
            ));
        }
        std::fs::rename(&target_dir, &ready)
            .map_err(|error| LogFail::Core(crate::runtime::owners::io_error(error)))?;
    }
    let genesis = match activation {
        Some(reference) => *reference.target_genesis.as_bytes(),
        None => [0u8; 32],
    };
    let _ = context;
    Ok(AdminOwned::Completed(
        AdminValueOwned::MigrationInitialize {
            directory: binding.directory.clone(),
            identity: DatabaseIdentity {
                database_id: binding.identity.database_id,
                incarnation_id: target_incarnation,
                schema_id: binding.identity.schema_id,
            },
            genesis,
        },
    ))
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
                    "MigrationUnsupported",
                    "restore targets a local binding; hosted re-publication is the \
                     recorded C08 boundary",
                ),
                dispatched: false,
            });
        }
    }
    let Some(backup) = backup else {
        return Ok(AdminOwned::Failed {
            fail: protocol("Misuse", "restore needs the backup operation id (`backup`)"),
            dispatched: false,
        });
    };
    let Some((descriptor, _attrs)) = target.descriptor.clone() else {
        return Ok(AdminOwned::Failed {
            fail: protocol(
                "Misuse",
                "restore needs the target `schema` (lowered SchemaSpec)",
            ),
            dispatched: false,
        });
    };
    let destination = store_of_destination(source)?;
    let restored = with_store!(destination, dest_prefix, store => {
        let (manifest, manifest_digest) =
            bumbledb_log::backup::read_backup_manifest(store, &dest_prefix, backup).map_err(
                |error| LogFail::Protocol {
                    code: "Corruption",
                    detail: format!("{error:?}"),
                },
            )?;
        let Some(checkpoint_ref) = manifest.checkpoint else {
            return Err(protocol(
                "UnsupportedArtifact",
                "genesis-root backups (whole-chain tails) restore through the recorded \
                 C08 boundary",
            ));
        };
        let checkpoint_bytes =
            bumbledb_log::store::get_verified(store, &dest_prefix, &checkpoint_ref).map_err(
                |error| LogFail::Protocol {
                    code: "Corruption",
                    detail: format!("{error:?}"),
                },
            )?;
        let checkpoint =
            bumbledb_log::codec::decode_manifest(&checkpoint_bytes, StreamLimits::DEFAULT)
                .map_err(|error| LogFail::Protocol {
                    code: "Corruption",
                    detail: format!("{error:?}"),
                })?;
        let mut chunks: Vec<Result<Vec<u8>, RecoveryError>> = Vec::new();
        for chunk_ref in &checkpoint.chunks {
            context.checkpoint().map_err(RuntimeError::from)?;
            match bumbledb_log::store::get_verified(store, &dest_prefix, chunk_ref) {
                Ok(bytes) => chunks.push(Ok(bytes)),
                Err(error) => chunks.push(Err(RecoveryError::Object(error))),
            }
        }
        let tail = bumbledb_log::backup::read_backup_tail(
            store,
            &dest_prefix,
            &manifest,
            LIMITS,
            context,
        )
        .map_err(|error| LogFail::Protocol {
            code: "Corruption",
            detail: format!("{error:?}"),
        })?;
        std::fs::create_dir_all(&target.directory)
            .map_err(|error| LogFail::Core(crate::runtime::owners::io_error(error)))?;
        let ready = recovery::materialization_path(Path::new(&target.directory));
        bumbledb_log::restore::restore_writable_with_tail(
            &ready,
            descriptor,
            &checkpoint,
            chunks,
            &tail,
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
        AdminOwned::Failed { fail, dispatched } => {
            wire.set(
                "certainty",
                if dispatched {
                    "outcome-unknown"
                } else {
                    "not-started"
                },
            )?;
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
        AdminValueOwned::MigrationStatus(status) => {
            body.set("verb", "migration-status")?;
            let mut wire_status = Object::new(&env)?;
            match status {
                StatusOwned::UpToDate { applied_prefix } => {
                    wire_status.set("kind", "up-to-date")?;
                    wire_status.set("appliedPrefixDigest", hex32(&applied_prefix))?;
                }
                StatusOwned::Pending { pending } => {
                    wire_status.set("kind", "pending")?;
                    wire_status.set("pending", pending)?;
                }
                StatusOwned::InProgress {
                    source,
                    operation,
                    plan_set,
                    target,
                } => {
                    wire_status.set("kind", "in-progress")?;
                    wire_status.set(
                        "operationRef",
                        migration_ref_wire(&env, source, operation, plan_set, target)?,
                    )?;
                }
                StatusOwned::Aborted {
                    source,
                    operation,
                    plan_set,
                    target,
                } => {
                    wire_status.set("kind", "aborted")?;
                    wire_status.set(
                        "operationRef",
                        migration_ref_wire(&env, source, operation, plan_set, target)?,
                    )?;
                }
            }
            body.set("status", wire_status)?;
        }
        AdminValueOwned::MigrationInitialize {
            directory,
            identity,
            genesis,
        } => {
            body.set("verb", "migration-initialize")?;
            let mut binding = Object::new(&env)?;
            binding.set("kind", "local")?;
            binding.set("directory", directory)?;
            binding.set("identity", identity_wire(&env, identity)?)?;
            body.set("binding", binding)?;
            body.set("genesis", hex32(&genesis))?;
        }
        AdminValueOwned::Migrate(outcome) => {
            body.set("verb", "migration-migrate")?;
            let mut value = Object::new(&env)?;
            match outcome {
                MigrateOwned::UpToDate {
                    directory,
                    identity,
                } => {
                    value.set("kind", "up-to-date")?;
                    let mut binding = Object::new(&env)?;
                    binding.set("kind", "local")?;
                    binding.set("directory", directory)?;
                    binding.set("identity", identity_wire(&env, identity)?)?;
                    value.set("binding", binding)?;
                }
                MigrateOwned::ReadyToSwitch {
                    deployment_directory,
                    target,
                    activation,
                } => {
                    value.set("kind", "ready-to-switch")?;
                    let mut binding = Object::new(&env)?;
                    binding.set("kind", "local")?;
                    binding.set("directory", deployment_directory)?;
                    binding.set("identity", identity_wire(&env, target)?)?;
                    value.set("deploymentBinding", binding)?;
                    let mut reference = Object::new(&env)?;
                    reference.set("operationId", hex16(activation.operation.as_core()))?;
                    reference.set("planSetDigest", hex32(&activation.plan_set_digest))?;
                    reference.set("target", identity_wire(&env, activation.target)?)?;
                    reference.set("targetGenesis", hex32(activation.target_genesis.as_bytes()))?;
                    value.set("activation", reference)?;
                }
                MigrateOwned::Paused { fail, operation } => {
                    value.set("kind", "paused")?;
                    value.set("error", frame_object(&env, &fail)?)?;
                    let mut source_state = Object::new(&env)?;
                    source_state.set("access", "frozen")?;
                    source_state.set(
                        "operationId",
                        operation.map(|operation| hex16(operation.as_core())),
                    )?;
                    value.set("sourceState", source_state)?;
                }
            }
            body.set("value", value)?;
        }
        AdminValueOwned::MigrationActivate {
            target,
            access,
            operation,
            activated_now,
        } => {
            body.set("verb", "migration-activate")?;
            body.set("target", identity_wire(&env, target)?)?;
            body.set("accessMode", access)?;
            body.set("operationId", hex16(operation.as_core()))?;
            body.set("activatedNow", activated_now)?;
        }
        AdminValueOwned::MigrationAbort {
            target,
            target_fenced,
            source_access,
        } => {
            body.set("verb", "migration-abort")?;
            body.set("target", identity_wire(&env, target)?)?;
            body.set("targetFenced", target_fenced)?;
            body.set("sourceAccess", source_access)?;
        }
    }
    wire.set("value", body)?;
    Ok(wire)
}

fn migration_ref_wire(
    env: &Env,
    source: DatabaseIdentity,
    operation: OperationId,
    plan_set: [u8; 32],
    target: DatabaseIdentity,
) -> napi::Result<Object<'_>> {
    let mut reference = Object::new(env)?;
    reference.set("identity", identity_wire(env, source)?)?;
    reference.set("operationId", hex16(operation.as_core()))?;
    reference.set("planSetDigest", hex32(&plan_set))?;
    reference.set("target", identity_wire(env, target)?)?;
    Ok(reference)
}

const _: fn() = || {
    // Compile locks: the admin machinery's owned values must cross threads.
    fn assert_send<T: Send>() {}
    assert_send::<AdminOwned>();
    let _ = AtomicBool::new(false);
};
