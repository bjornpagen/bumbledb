//! Hosted databases: the log's sans-IO `Machine` over its LMDB `Cache`,
//! driven from JavaScript. Each step runs on the machine's worker and returns
//! the object-store requests JavaScript performs plus the settled tickets;
//! JavaScript feeds every response back as another step. Reads pin
//! snapshots of the cache's current database on the same worker.
use std::sync::{Arc, Mutex};

use bumbledb::work::WorkContext;
use bumbledb_log::{
    Bundle, Cache, CacheDb, CheckpointPolicy, Command, Config, DatabaseId, Input, IoId, IoResponse,
    IoResult, Machine, MigrationHash, MigrationId, Millis, Population, Precondition, Replica as _,
    RequestId, Revision, Seq, Ticket,
};
use napi::bindgen_prelude::{BigInt, Env, External, Function, Unknown};
use napi_derive::napi;
use serde::Deserialize;

use crate::input::schema::SchemaSpecIn;
use crate::input::value::{HexBytes, U64Text};
use crate::marshal;
use crate::runtime::owners::{DirectoryReference, ExternalLease, ManagedDb};
use crate::runtime::registry::{NativeKind, Payload, RegistryAdmission};
use crate::runtime::{Output, RuntimeError};
use crate::runtime_wire::{
    CloseOut, DirectoryHandle, OperationHandle, notification, operation_handle, operation_runtime,
    reporter, take_output, thrown, unshared_input, wrong_output,
};
use crate::schema::{SchemaDiagnostic, SchemaHandle};

mod out;

pub use out::*;

/// One bundled migration: its name, content hash (64 hex digits) and the
/// schema spec it migrates to. The first step is the initial schema.
#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BundleStepIn {
    pub name: String,
    #[napi(ts_type = "string")]
    pub hash: HexBytes,
    pub schema: SchemaSpecIn,
}

/// Everything a hosted database opens with. `seed` (32 hex digits) is
/// random per process; `create` (32 hex digits) is the identity a new
/// database gets when its log is empty, absent to refuse creation.
#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostedOpenIn {
    pub bundle: Vec<BundleStepIn>,
    #[napi(ts_type = "string")]
    pub seed: HexBytes,
    #[napi(ts_type = "string")]
    pub create: Option<HexBytes>,
    pub probe_window: u32,
    #[napi(ts_type = "string")]
    pub checkpoint_every: U64Text,
    pub checkpoint_keep: u32,
    pub max_entry_bytes: u32,
}

fn id16(bytes: &HexBytes, what: &str) -> Result<[u8; 16], RuntimeError> {
    <[u8; 16]>::try_from(&*bytes.0)
        .map_err(|_| marshal::err(format!("{what}: expected 32 hex digits")))
}

fn id32(bytes: &[u8], what: &str) -> Result<[u8; 32], RuntimeError> {
    <[u8; 32]>::try_from(bytes).map_err(|_| marshal::err(format!("{what}: expected 64 hex digits")))
}

fn hex_id16(text: &str, what: &str) -> Result<[u8; 16], RuntimeError> {
    let bytes = HexBytes::try_from(text.to_owned()).map_err(marshal::err)?;
    id16(&bytes, what)
}

fn u64_in(value: &BigInt, what: &str) -> Result<u64, RuntimeError> {
    marshal::u64_in(value, what)
}

/// The registered database reads pin: the cache's current database, kept
/// current after every step; `None` until the first state exists.
type Reads = Arc<Mutex<Option<(Arc<CacheDb>, ManagedDb)>>>;

/// The machine, the compiled schema of every bundled step, and the shared
/// slot naming the database reads pin.
pub struct Hosted {
    machine: Machine<Cache>,
    schemas: Schemas,
    directory: DirectoryReference,
    reads: Reads,
    _lease: ExternalLease,
}

impl Hosted {
    fn head_schema(&self) -> Option<&Arc<SchemaHandle>> {
        let head = self.machine.replica().head()?;
        let index = self
            .machine
            .replica()
            .bundle()
            .steps()
            .iter()
            .position(|step| step.fingerprint == head.schema)?;
        self.schemas.get(index)
    }

    fn step(&mut self, input: Input) -> Result<StepOut, RuntimeError> {
        let step = self.machine.step(input);
        self.refresh_reads()?;
        Ok(out::step(step, self.machine.replica(), &self.schemas))
    }

    /// Attach the cache's database for reads when it swapped in a new one.
    fn refresh_reads(&self) -> Result<(), RuntimeError> {
        let Some(db) = self.machine.replica().db() else {
            return Ok(());
        };
        let mut reads = self.reads.lock().map_err(|_| RuntimeError::Internal)?;
        if reads
            .as_ref()
            .is_some_and(|(held, _)| Arc::ptr_eq(held, db))
        {
            return Ok(());
        }
        let Some(schema) = self.head_schema() else {
            return Ok(());
        };
        let managed = self.directory.attach_db(crate::DbInner {
            db: Arc::clone(db),
            schema: Arc::clone(schema),
            writing: std::sync::atomic::AtomicBool::new(false),
        })?;
        *reads = Some((Arc::clone(db), managed));
        Ok(())
    }
}

/// One open hosted database. Its step verbs must be serialized: a call while
/// a step is still running refuses `HandleBusy`.
pub struct HostedHandle {
    admission: RegistryAdmission,
    unchanged: Arc<[Vec<CopyPair>]>,
    reads: Reads,
}

/// One relation a migration copies unchanged: `target` at the new schema
/// from `source` at the old one.
#[napi(object)]
#[derive(Clone, Copy)]
pub struct CopyPair {
    pub target: u32,
    pub source: u32,
}

pub struct HostedOpened {
    hosted: Box<Hosted>,
    unchanged: Arc<[Vec<CopyPair>]>,
}

/// The compiled schema of every bundled step, in bundle order.
type Schemas = Arc<[Arc<SchemaHandle>]>;

fn compile_bundle(steps: Vec<BundleStepIn>) -> Result<(Bundle, Schemas), RuntimeError> {
    let mut schemas = Vec::with_capacity(steps.len());
    let mut bundle = Vec::with_capacity(steps.len());
    for (index, step) in steps.into_iter().enumerate() {
        let schema = SchemaHandle::compile(step.schema.into()).map_err(|diagnostic| {
            let message = match diagnostic {
                SchemaDiagnostic::Spec { issues } => issues
                    .into_iter()
                    .map(|issue| issue.message)
                    .collect::<Vec<_>>()
                    .join("; "),
                SchemaDiagnostic::Schema { message, .. } => message,
            };
            marshal::err(format!("bundle[{index}].schema: {message}"))
        })?;
        let id = MigrationId {
            name: step.name.into(),
            hash: MigrationHash(id32(&step.hash.0, "bundle hash")?),
        };
        bundle.push((id, schema.descriptor.clone()));
        schemas.push(Arc::new(schema));
    }
    let bundle = Bundle::new(bundle).map_err(|error| RuntimeError::Engine {
        kind: "Bundle".into(),
        message: format!("{error:?}"),
    })?;
    Ok((bundle, schemas.into()))
}

/// Open the hosted database whose cache lives in `childName` of an owned
/// directory. `openJson` is a `HostedOpenIn`.
#[napi]
pub fn hosted_open(
    env: Env,
    directory: &External<DirectoryHandle>,
    child_name: String,
    open_json: String,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let open: HostedOpenIn =
        crate::input::decode(&open_json).map_err(|malformed| thrown(env, malformed.into()))?;
    let owner = directory.owner();
    let reference = owner.reference();
    let lease = reference.lease().map_err(|error| thrown(env, error))?;
    let config = Config {
        seed: id16(&open.seed, "seed").map_err(|error| thrown(env, error))?,
        create: open
            .create
            .as_ref()
            .map(|id| id16(id, "create").map(DatabaseId))
            .transpose()
            .map_err(|error| thrown(env, error))?,
        probe_window: open.probe_window,
        checkpoint: CheckpointPolicy {
            every: open.checkpoint_every.0,
            keep: open.checkpoint_keep,
        },
        max_entry_bytes: open.max_entry_bytes as usize,
    };
    let steps = open.bundle;
    let operation = owner
        .runtime()
        .submit_owned(owner, WorkContext::new(), notification(callback)?, |_| {
            Ok(Box::new(move |context| {
                context.checkpoint()?;
                let root = reference.child_path(&child_name)?;
                open_hosted(reference, &root, steps, config, lease).map(Output::Hosted)
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(owner.runtime(), operation))
}

fn open_hosted(
    directory: DirectoryReference,
    root: &std::path::Path,
    steps: Vec<BundleStepIn>,
    config: Config,
    lease: ExternalLease,
) -> Result<HostedOpened, RuntimeError> {
    let (bundle, schemas) = compile_bundle(steps)?;
    let unchanged = (0..bundle.steps().len())
        .map(|index| {
            bundle
                .unchanged(index)
                .iter()
                .map(|(target, source)| CopyPair {
                    target: target.0,
                    source: source.0,
                })
                .collect()
        })
        .collect();
    let cache = Cache::open(root, bundle).map_err(|error| RuntimeError::Engine {
        kind: "Cache".into(),
        message: error.to_string(),
    })?;
    Ok(HostedOpened {
        hosted: Box::new(Hosted {
            machine: Machine::new(cache, config),
            schemas,
            directory,
            reads: Arc::default(),
            _lease: lease,
        }),
        unchanged,
    })
}

#[napi]
pub fn hosted_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<HostedHandle>> {
    let runtime = operation_runtime(handle);
    let Output::Hosted(opened) = take_output(env, handle)? else {
        return Err(wrong_output(env));
    };
    let directory = opened.hosted.directory.clone();
    let reads = Arc::clone(&opened.hosted.reads);
    let admission =
        RegistryAdmission::admit(runtime, NativeKind::Hosted, Payload::Hosted(opened.hosted))
            .map_err(|error| thrown(env, error))?;
    directory
        .adopt(admission.cap())
        .map_err(|error| thrown(env, error))?;
    Ok(External::new(HostedHandle {
        admission,
        unchanged: opened.unchanged,
        reads,
    }))
}

/// The relations bundled migration `step` copies unchanged by default.
#[napi]
pub fn hosted_unchanged(
    env: Env,
    hosted: &External<HostedHandle>,
    step: u32,
) -> napi::Result<Vec<CopyPair>> {
    hosted
        .unchanged
        .get(step as usize)
        .cloned()
        .ok_or_else(|| thrown(env, RuntimeError::InvalidArgument))
}

fn hosted_payload(payload: &mut Payload) -> Result<&mut Hosted, RuntimeError> {
    match payload {
        Payload::Hosted(hosted) => Ok(hosted),
        _ => Err(RuntimeError::Internal),
    }
}

fn submit_step(
    env: Env,
    hosted: &HostedHandle,
    callback: Function<(), ()>,
    input: Input,
) -> napi::Result<External<OperationHandle>> {
    let operation = step(&hosted.admission, notification(callback)?, input)
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(hosted.admission.runtime(), operation))
}

fn step(
    hosted: &RegistryAdmission,
    notify: crate::runtime::Notify,
    input: Input,
) -> Result<Arc<crate::runtime::Operation>, RuntimeError> {
    hosted
        .runtime()
        .submit_payload(hosted.cap(), WorkContext::new(), notify, move |_| {
            Ok(Box::new(move |context, payload, _| {
                context.checkpoint()?;
                Ok(Output::HostedStep(hosted_payload(payload)?.step(input)?))
            }))
        })
}

/// A machine input that carries no change set.
#[napi(discriminant = "_tag", object_to_js = false)]
pub enum HostedInput {
    /// Hydrate and catch up.
    Open { ticket: BigInt },
    /// Settle once every entry that existed now is applied.
    Sync { ticket: BigInt },
    /// Sync, then report the request's receipt; `request` is 32 hex digits.
    Resolve { ticket: BigInt, request: String },
    /// Freeze for the next pending migration for `leaseMillis` of store time.
    Freeze {
        ticket: BigInt,
        lease_millis: BigInt,
    },
    /// Settle everything pending; later inputs are refused.
    Close,
}

#[napi]
pub fn hosted_step(
    env: Env,
    hosted: &External<HostedHandle>,
    input: HostedInput,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let ticket = |value: &BigInt| u64_in(value, "ticket").map(Ticket);
    let input = (|| {
        Ok(match input {
            HostedInput::Open { ticket: value } => Input::Open(ticket(&value)?),
            HostedInput::Sync { ticket: value } => Input::Sync(ticket(&value)?),
            HostedInput::Resolve {
                ticket: value,
                request,
            } => Input::Resolve(ticket(&value)?, RequestId(hex_id16(&request, "request")?)),
            HostedInput::Freeze {
                ticket: value,
                lease_millis,
            } => Input::Freeze(ticket(&value)?, u64_in(&lease_millis, "leaseMillis")?),
            HostedInput::Close => Input::Close,
        })
    })()
    .map_err(|error: RuntimeError| thrown(env, error))?;
    submit_step(env, hosted, callback, input)
}

/// Submit one command: the request id (32 hex digits) is its idempotency
/// key; `revision` present requires exactly that committed revision.
#[napi]
pub fn hosted_submit(
    env: Env,
    hosted: &External<HostedHandle>,
    ticket: BigInt,
    request: String,
    revision: Option<BigInt>,
    changes: &External<crate::db_wire::ChangesHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let parsed = (|| {
        let precondition = match revision {
            None => Precondition::None,
            Some(value) => Precondition::ExactRevision(Revision(u64_in(&value, "revision")?)),
        };
        Ok::<_, RuntimeError>((
            Ticket(u64_in(&ticket, "ticket")?),
            RequestId(hex_id16(&request, "request")?),
            precondition,
        ))
    })()
    .map_err(|error| thrown(env, error))?;
    let (ticket, request, precondition) = parsed;
    let operation = step_with_changes(
        &hosted.admission,
        changes,
        notification(callback)?,
        move |changes, _| {
            Ok(Input::Submit(
                ticket,
                Command::seal(request, precondition, changes),
            ))
        },
    )
    .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(hosted.admission.runtime(), operation))
}

/// Apply bundled migration `step`, computed at `base` (the head the rows
/// were computed from): `copy` relations carried over unchanged plus the
/// computed `rows` at the new schema.
#[napi]
pub fn hosted_migrate(
    env: Env,
    hosted: &External<HostedHandle>,
    ticket: BigInt,
    step: u32,
    base: BigInt,
    copy: Vec<CopyPair>,
    rows: &External<crate::db_wire::ChangesHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let parsed = (|| {
        let base = Seq::new(u64_in(&base, "base")?)
            .ok_or_else(|| marshal::err("base: log positions start at 1".into()))?;
        Ok::<_, RuntimeError>((Ticket(u64_in(&ticket, "ticket")?), base))
    })()
    .map_err(|error| thrown(env, error))?;
    let (ticket, base) = parsed;
    let copy: Box<[_]> = copy
        .into_iter()
        .map(|pair| {
            (
                bumbledb::RelationId(pair.target),
                bumbledb::RelationId(pair.source),
            )
        })
        .collect();
    let operation = step_with_changes(
        &hosted.admission,
        rows,
        notification(callback)?,
        move |rows, hosted| {
            let step = hosted
                .machine
                .replica()
                .bundle()
                .steps()
                .get(step as usize)
                .map(|bundled| bundled.id.clone())
                .ok_or(RuntimeError::InvalidArgument)?;
            Ok(Input::Migrate(
                ticket,
                Population {
                    step,
                    base,
                    copy,
                    rows,
                },
            ))
        },
    )
    .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(hosted.admission.runtime(), operation))
}

/// Read a sealed change set on its own worker, then step the machine with
/// the input built from it on the machine's worker.
fn step_with_changes(
    hosted: &RegistryAdmission,
    changes: &crate::db_wire::ChangesHandle,
    notify: crate::runtime::Notify,
    input: impl FnOnce(bumbledb::ChangeSet, &Hosted) -> Result<Input, RuntimeError> + Send + 'static,
) -> Result<Arc<crate::runtime::Operation>, RuntimeError> {
    let runtime = hosted.runtime();
    if !Arc::ptr_eq(runtime, changes.runtime()) {
        return Err(RuntimeError::ForeignRuntime);
    }
    let target = hosted.cap();
    runtime.submit_payload(changes.cap(), WorkContext::new(), notify, move |_| {
        Ok(Box::new(move |context, payload, _| {
            context.checkpoint()?;
            let changes = crate::db_wire::changes_from_payload(payload)?.changes;
            Ok(Output::PayloadContinuation {
                cap: target,
                work: Box::new(move |context, payload, _| {
                    context.checkpoint()?;
                    let hosted = hosted_payload(payload)?;
                    let input = input(changes, hosted)?;
                    Ok(Output::HostedStep(hosted.step(input)?))
                }),
            })
        }))
    })
}

fn millis(value: Option<&BigInt>, what: &str) -> Result<Option<Millis>, RuntimeError> {
    value
        .map(|value| u64_in(value, what).map(Millis))
        .transpose()
}

/// The outcome of one store request other than a body read.
#[napi(discriminant = "_tag", object_to_js = false)]
pub enum IoOutcomeIn {
    /// A `GetFile` request wrote the object to its path.
    Saved {
        last_modified: BigInt,
    },
    /// A get found no object.
    Missing,
    /// A put-if-absent created the object.
    Created,
    /// A put-if-absent found the key taken (412).
    Occupied,
    /// A list's keys, in the requested form.
    Keys {
        keys: Vec<String>,
    },
    Deleted,
    /// Anything else after JavaScript's own retries of idempotent reads; a
    /// failed put is reported here, never retried.
    Failed,
}

/// Feed back one completed store request. `date` is the response's `Date`
/// header in Unix milliseconds.
#[napi]
pub fn hosted_respond(
    env: Env,
    hosted: &External<HostedHandle>,
    id: BigInt,
    date: Option<BigInt>,
    outcome: IoOutcomeIn,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let response = (|| {
        let result = match outcome {
            IoOutcomeIn::Saved { last_modified } => IoResult::Saved {
                last_modified: Millis(u64_in(&last_modified, "lastModified")?),
            },
            IoOutcomeIn::Missing => IoResult::Missing,
            IoOutcomeIn::Created => IoResult::Created,
            IoOutcomeIn::Occupied => IoResult::Occupied,
            IoOutcomeIn::Keys { keys } => IoResult::Keys(keys),
            IoOutcomeIn::Deleted => IoResult::Deleted,
            IoOutcomeIn::Failed => IoResult::Failed,
        };
        Ok::<_, RuntimeError>(IoResponse {
            id: IoId(u64_in(&id, "id")?),
            date: millis(date.as_ref(), "date")?,
            result,
        })
    })()
    .map_err(|error| thrown(env, error))?;
    submit_step(env, hosted, callback, Input::Response(response))
}

/// Feed back one `GetMemory` request that found its object.
#[napi]
pub fn hosted_respond_body(
    env: Env,
    hosted: &External<HostedHandle>,
    id: BigInt,
    date: Option<BigInt>,
    last_modified: BigInt,
    #[napi(ts_arg_type = "Uint8Array")] body: Unknown,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let bytes = unshared_input(env, body)?.to_vec();
    let response = (|| {
        Ok::<_, RuntimeError>(IoResponse {
            id: IoId(u64_in(&id, "id")?),
            date: millis(date.as_ref(), "date")?,
            result: IoResult::Body {
                bytes,
                last_modified: Millis(u64_in(&last_modified, "lastModified")?),
            },
        })
    })()
    .map_err(|error| thrown(env, error))?;
    submit_step(env, hosted, callback, Input::Response(response))
}

#[napi]
pub fn hosted_step_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<StepOut> {
    match take_output(env, handle)? {
        Output::HostedStep(step) => Ok(step),
        _ => Err(wrong_output(env)),
    }
}

/// Pin a snapshot of the cache's current state; take it with
/// `runtimeSnapshotTake`. Refuses `ClosedHandle` before the first state.
#[napi]
pub fn hosted_snapshot(
    env: Env,
    hosted: &External<HostedHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let operation =
        snapshot(&hosted.reads, notification(callback)?).map_err(|error| thrown(env, error))?;
    Ok(operation_handle(hosted.admission.runtime(), operation))
}

fn snapshot(
    reads: &Reads,
    notify: crate::runtime::Notify,
) -> Result<Arc<crate::runtime::Operation>, RuntimeError> {
    let reads = reads.lock().map_err(|_| RuntimeError::Internal)?;
    let (_, managed) = reads.as_ref().ok_or(RuntimeError::ClosedHandle)?;
    managed
        .runtime()
        .open_session(managed, WorkContext::new(), notify)
}

/// Close the machine and its cache. Step `Close` first to settle every
/// pending ticket.
#[napi]
pub fn hosted_close(
    hosted: &External<HostedHandle>,
    callback: Function<CloseOut, ()>,
) -> napi::Result<()> {
    crate::db_wire::close_admitted(
        hosted.admission.runtime(),
        hosted.admission.cap(),
        reporter(callback)?,
    );
    Ok(())
}

#[cfg(test)]
mod tests;
