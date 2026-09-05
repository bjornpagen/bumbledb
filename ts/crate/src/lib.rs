//! The dumb-bridge law: no logic beyond marshaling will EVER live in this
//! crate. Anything smart belongs in the TypeScript SDK or the engine.
//!
//! Every database resource is owned by the ONE runtime registry
//! (`runtime_wire.rs`): databases live behind kernel-held directory
//! owners, `!Send` engine transactions and prepared queries live inside
//! worker-affine sessions (`runtime/session.rs`), and every operation is
//! registered, charged and drainable. The historical raw-pointer
//! `InstanceHandle`/`TxHandle` scoped-borrow surface — a JavaScript
//! callback executing inside a native transaction frame — is deleted, as
//! are the libuv `AsyncTask` entrypoints and the fresh/reserve issuance
//! verbs (the successor has application-owned `Id128` identity only).
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bumbledb::schema::{SpecIssue, StatementDescriptor};
use bumbledb::{
    BindValue, Db, Error, OwnedInstance, ParamArg, SchemaDescriptor, Theory as _, Value,
    Violations, Witness, render_rejection,
};
use napi::bindgen_prelude::{Buffer, Env, External, Object, ToNapiValue};
use napi::sys;
use napi_derive::napi;

pub mod db_wire;
#[cfg(test)]
mod fingerprint_lock;
pub mod log;
pub mod log_wire;
mod marshal;
mod migration_wire;
mod runtime;
pub mod runtime_wire;
mod tags;

use marshal::{DescriptorWire, ManifestWire, OwnedParam, ViolationWire};

/// Per-relation, sealed-order spec attribute rows (host newtype names) —
/// the spec-only half of the field vocabulary the descriptor drops.
type FieldAttrsTable = Vec<Vec<marshal::FieldAttrs>>;

#[napi]
#[must_use]
pub fn engine_version() -> String {
    format!(
        "bumbledb-node {} (bumbledb storage format v{})",
        env!("CARGO_PKG_VERSION"),
        bumbledb::STORAGE_FORMAT_VERSION
    )
}

/// The engine's own blake3 (`bumbledb::digest::Digest`), lent to the
/// replication driver so the SDK ships exactly one hash implementation.
/// Internal surface: not part of the SDK's documented API. Bounded bulk
/// hashing belongs on the executor (`runtime_hash`); this synchronous verb
/// is for small identity-sized inputs only.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn blake3_hash(data: Buffer) -> Buffer {
    let mut digest = bumbledb::digest::Digest::new();
    digest.update(&data);
    Buffer::from(digest.finalize().to_vec())
}

/// The engine's own sealed descriptor as data, lent to the
/// replication driver so one authority seals the theory.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn descriptor(env: Env, spec: Object) -> napi::Result<DescriptorWire> {
    use bumbledb::schema::ValidateDescriptor as _;
    let (descriptor, attrs) = match descriptor_of(&spec)? {
        Ok(parsed) => parsed,
        Err(OpenOutcome::SchemaError(message) | OpenOutcome::NewtypeMismatch(message)) => {
            return Err(marshal::throw_kind_message(
                env,
                tags::error_family::SCHEMA,
                message,
            ));
        }
    };
    let sealed = seal(descriptor, attrs);
    let schema = sealed.descriptor.clone().validate().map_err(|error| {
        marshal::throw_kind_message(env, tags::error_family::SCHEMA, error.to_string())
    })?;
    let fingerprint = bumbledb::schema::fingerprint::fingerprint(&schema);
    Ok(DescriptorWire {
        manifest: sealed.descriptor.manifest(),
        statements: sealed.statements,
        fingerprint: hex_fingerprint(&fingerprint.0),
        attrs: sealed.attrs,
    })
}

pub struct Sealed {
    pub(crate) descriptor: SchemaDescriptor,
    pub(crate) statements: Vec<StatementDescriptor>,
    /// The resident sealed field rosters, index = `RelationId` ordinal —
    /// computed once here, borrowed by every fact-lane call; the bridge
    /// re-derives nothing.
    pub(crate) rosters: Vec<marshal::SealedRoster>,
    /// The spec-only field attributes in the same sealed order — carried
    /// so the manifest wire speaks the spec's whole field vocabulary.
    pub(crate) attrs: FieldAttrsTable,
}

pub(crate) fn seal(descriptor: SchemaDescriptor, attrs: FieldAttrsTable) -> Sealed {
    let statements = descriptor.materialized_statements();
    let rosters = marshal::sealed_rosters(&descriptor);
    Sealed {
        descriptor,
        statements,
        rosters,
        attrs,
    }
}

pub(crate) type Engine = Db<SchemaDescriptor>;

pub(crate) fn abort_sentinel() -> Error {
    Error::from(std::io::Error::other("bumbledb-node transaction abort"))
}

fn closed_handle(what: &str) -> napi::Error {
    marshal::err(format!("bumbledb: use of a closed {what} handle"))
}

fn leased_handle(what: &str) -> napi::Error {
    marshal::err(format!("bumbledb: {what} is leased for a native operation"))
}

fn reentrant_use(what: &str) -> napi::Error {
    marshal::err(format!("bumbledb: re-entrant use of a {what} handle"))
}

fn throw_engine(env: Env, error: &Error) -> napi::Error {
    marshal::throw_engine(env, error)
}

fn take_handle<T>(cell: &RefCell<Option<T>>, what: &str) -> napi::Result<T> {
    let mut borrowed = cell.try_borrow_mut().map_err(|_| reentrant_use(what))?;
    borrowed.take().ok_or_else(|| closed_handle(what))
}

macro_rules! outcome_to_napi {
    ($ty:ty { $( $variant:ident $(( $($tuple:ident),+ ))? $({ $($field:ident),+ })? => { $($key:literal : $value:expr),+ $(,)? } ),+ $(,)? }) => {
        impl ToNapiValue for $ty {
            #[expect(
                unsafe_code,
                reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                          builds a plain object and delegates to napi's own impls"
            )]
            unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
                let env_handle = napi::Env::from_raw(env);
                let mut obj = Object::new(&env_handle)?;
                match val {
                    $(Self::$variant $(( $($tuple),+ ))? $({ $($field),+ })? => {
                        $(obj.set($key, $value)?;)+
                    })+
                }
                unsafe { Object::to_napi_value(env, obj) }
            }
        }
    };
}

fn live<'a, T>(cell: &'a RefCell<Option<T>>, what: &str) -> napi::Result<Ref<'a, T>> {
    let borrowed = cell.try_borrow().map_err(|_| reentrant_use(what))?;
    Ref::filter_map(borrowed, Option::as_ref).map_err(|_| closed_handle(what))
}

fn live_mut<'a, T>(cell: &'a RefCell<Option<T>>, what: &str) -> napi::Result<RefMut<'a, T>> {
    let borrowed = cell.try_borrow_mut().map_err(|_| reentrant_use(what))?;
    RefMut::filter_map(borrowed, Option::as_mut).map_err(|_| closed_handle(what))
}

pub(crate) fn hex_fingerprint(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

pub(crate) fn bind_value(value: &Value) -> BindValue<'_> {
    match value {
        Value::Bool(v) => BindValue::Bool(*v),
        Value::U64(v) => BindValue::U64(*v),
        Value::I64(v) => BindValue::I64(*v),
        Value::F64(v) => BindValue::F64(*v),
        Value::Id128(v) => BindValue::Id128(*v),
        Value::String(text) => BindValue::Str(text),
        Value::FixedBytes(bytes) => BindValue::FixedBytes(bytes),
        Value::IntervalU64(interval) => BindValue::IntervalU64(interval.start(), interval.end()),
        Value::IntervalI64(interval) => BindValue::IntervalI64(interval.start(), interval.end()),
        Value::IntervalF64(interval) => BindValue::IntervalF64(*interval),
    }
}

pub(crate) fn param_args(params: &[OwnedParam]) -> Vec<ParamArg<'_>> {
    params
        .iter()
        .map(|param| match param {
            OwnedParam::Set(values) => ParamArg::Set(values),
            OwnedParam::Scalar(value) => ParamArg::Scalar(bind_value(value)),
        })
        .collect()
}

pub(crate) fn violations_wire(
    descriptor: &SchemaDescriptor,
    violations: &Violations,
) -> Vec<ViolationWire> {
    render_rejection(descriptor, violations)
        .into_iter()
        .map(ViolationWire::from_rendered)
        .collect()
}

pub(crate) fn assemble_inner(
    db: Engine,
    descriptor: SchemaDescriptor,
    attrs: FieldAttrsTable,
) -> DbInner {
    DbInner {
        db: Arc::new(db),
        sealed: Arc::new(seal(descriptor, attrs)),
        writing: AtomicBool::new(false),
    }
}

/// The one database owner: a registry-held [`runtime::owners::ManagedDb`].
/// Every native DB lives in the one runtime registry behind a kernel-held
/// directory lock, so a retained JS wrapper can never keep an engine,
/// mapping, FD or directory lock alive after a completed close, and the
/// directory lock always belongs to the same native owner as its
/// environment and active operations.
pub struct DbHandle {
    inner: runtime::owners::ManagedDb,
}

impl DbHandle {
    pub(crate) fn managed(owner: runtime::owners::ManagedDb) -> Self {
        Self { inner: owner }
    }

    /// A short-lived registered access lease. The lease holds one native
    /// operation, so a concurrent close cannot free the resource under an
    /// in-flight call; the lease drops the transient Engine `Arc` before
    /// the operation leaves the registry and teardown can run.
    fn access(&self, env: Env) -> napi::Result<runtime::owners::DbLease> {
        self.inner
            .access()
            .map_err(|error| runtime_wire::thrown(env, error))
    }

    pub(crate) fn owner(&self) -> &runtime::owners::ManagedDb {
        &self.inner
    }
}

pub(crate) struct DbInner {
    pub(crate) db: Arc<Engine>,
    pub(crate) sealed: Arc<Sealed>,
    /// The single-writer admission flag: a live write session owns the
    /// engine writer; a second open refuses (`WriterBusy`) instead of
    /// parking a session thread on the writer mutex.
    pub(crate) writing: AtomicBool,
}

/// The two spec-resolution refusals `descriptor_of` can surface. This is
/// an internal error carrier only: database creation/open is the managed
/// runtime path (`runtime_directory_db_open` → `runtime_db_take`), which
/// renders these as its own `refused` wire arms.
pub enum OpenOutcome {
    SchemaError(String),
    NewtypeMismatch(String),
}

pub(crate) fn descriptor_of(
    spec: &Object,
) -> napi::Result<std::result::Result<(SchemaDescriptor, FieldAttrsTable), OpenOutcome>> {
    let spec = marshal::schema_spec(spec)?;
    let attrs = marshal::field_attrs(&spec);
    match spec.descriptor() {
        Ok(descriptor) => Ok(Ok((descriptor, attrs))),
        Err(error) => {
            let mismatched = error
                .issues()
                .iter()
                .any(|issue| matches!(issue, SpecIssue::StatementNewtypeMismatch { .. }));
            Ok(Err(if mismatched {
                OpenOutcome::NewtypeMismatch(error.to_string())
            } else {
                OpenOutcome::SchemaError(error.to_string())
            }))
        }
    }
}

#[napi]
pub fn db_manifest(env: Env, db: &External<DbHandle>) -> napi::Result<ManifestWire> {
    let inner = db.access(env)?;
    Ok(ManifestWire {
        manifest: inner.sealed.descriptor.clone().manifest(),
        attrs: inner.sealed.attrs.clone(),
    })
}

#[napi]
pub fn db_fingerprint(env: Env, db: &External<DbHandle>) -> napi::Result<String> {
    use bumbledb::schema::ValidateDescriptor as _;
    let inner = db.access(env)?;
    let schema = inner
        .sealed
        .descriptor
        .clone()
        .validate()
        .map_err(|error| marshal::err(error.to_string()))?;
    let fingerprint = bumbledb::schema::fingerprint::fingerprint(&schema);
    Ok(hex_fingerprint(&fingerprint.0))
}

#[napi]
pub fn db_generation(env: Env, db: &External<DbHandle>) -> napi::Result<u64> {
    let inner = db.access(env)?;
    match inner.db.generation() {
        Ok(generation) => Ok(generation.value()),
        Err(error) => Err(throw_engine(env, &error)),
    }
}

/// blake3 over the canonical catalog enumeration — the replication
/// equality oracle: equal digests imply identical judged content
/// regardless of page layout or allocation history.
#[napi]
pub fn db_catalog_digest(env: Env, db: &External<DbHandle>) -> napi::Result<Buffer> {
    let inner = db.access(env)?;
    match inner.db.catalog_digest() {
        Ok(digest) => Ok(Buffer::from(digest.to_vec())),
        Err(error) => Err(throw_engine(env, &error)),
    }
}

// ---------------------------------------------------------------------------
// Witness: owned generation evidence. A witness is a value (Clone), never a
// borrow into an engine frame, so the External is an ordinary owned handle.
// ---------------------------------------------------------------------------

pub struct WitnessHandle {
    inner: RefCell<Option<Witness<SchemaDescriptor>>>,
}

impl WitnessHandle {
    pub(crate) fn mint(witness: Witness<SchemaDescriptor>) -> Self {
        Self {
            inner: RefCell::new(Some(witness)),
        }
    }
}

/// A clone of the owned witness evidence, for a conditional write session.
pub(crate) fn witness_of(
    witness: &External<WitnessHandle>,
) -> napi::Result<Witness<SchemaDescriptor>> {
    Ok(live(&witness.inner, "witness")?.clone())
}

#[napi]
pub fn witness_close(witness: &External<WitnessHandle>) -> napi::Result<()> {
    let _ = take_handle(&witness.inner, "witness")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Wire shells shared by the executor verbs.
// ---------------------------------------------------------------------------

pub enum MutationReportWire {
    Report { submitted: u64, changed: u64 },
}

outcome_to_napi!(MutationReportWire {
    Report { submitted, changed } => { "submitted": submitted, "changed": changed },
});

// ---------------------------------------------------------------------------
// Instance builder: a database-free admitted-state draft. `InstanceBuilder`
// is `Send` (not `Sync`), so admission moves the whole draft onto the one
// executor (`runtime_builder_admit`) — the libuv `AsyncTask` path is gone.
// ---------------------------------------------------------------------------

pub struct BuilderHandle {
    inner: RefCell<Option<bumbledb::InstanceBuilder<SchemaDescriptor>>>,
    sealed: Arc<Sealed>,
}

/// The owned admission outcome crossing back from the executor.
pub enum AdmitOwned {
    Accepted {
        instance: OwnedInstance<SchemaDescriptor>,
        sealed: Arc<Sealed>,
    },
    Rejected(Vec<ViolationWire>),
}

/// Spends the builder handle and hands the owned draft plus its sealed
/// datum to the executor admission job.
pub(crate) fn builder_take(
    builder: &External<BuilderHandle>,
) -> napi::Result<(bumbledb::InstanceBuilder<SchemaDescriptor>, Arc<Sealed>)> {
    let taken = take_handle(&builder.inner, "builder")?;
    Ok((taken, Arc::clone(&builder.sealed)))
}

/// Puts an untouched draft back after a refused admission submission, so
/// a queue-full refusal does not spend the builder.
pub(crate) fn builder_restore(
    builder: &External<BuilderHandle>,
    draft: bumbledb::InstanceBuilder<SchemaDescriptor>,
) {
    if let Ok(mut slot) = builder.inner.try_borrow_mut()
        && slot.is_none()
    {
        *slot = Some(draft);
    }
}

#[napi]
pub fn instance_builder_new(env: Env, spec: Object) -> napi::Result<External<BuilderHandle>> {
    let (descriptor, attrs) = match descriptor_of(&spec)? {
        Ok(parsed) => parsed,
        Err(OpenOutcome::SchemaError(message) | OpenOutcome::NewtypeMismatch(message)) => {
            return Err(marshal::throw_kind_message(
                env,
                tags::error_family::SCHEMA,
                message,
            ));
        }
    };
    let builder = bumbledb::InstanceBuilder::new(descriptor.clone())
        .map_err(|error| throw_engine(env, &error))?;
    Ok(External::new(BuilderHandle {
        inner: RefCell::new(Some(builder)),
        sealed: Arc::new(seal(descriptor, attrs)),
    }))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn instance_builder_load(
    env: Env,
    builder: &External<BuilderHandle>,
    relation: u32,
    rows: napi::bindgen_prelude::BigInt,
    cells: napi::bindgen_prelude::Array,
) -> napi::Result<MutationReportWire> {
    let rows = marshal::u64_in(&rows, "collection rows")?;
    let collection =
        marshal::accepted_collection(env, &builder.sealed.rosters, relation, rows, &cells)?;
    let report = live_mut(&builder.inner, "builder")?
        .load_accepted(&collection)
        .map_err(|error| throw_engine(env, &error))?;
    Ok(mutation_report_wire(report))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn instance_builder_delete(
    env: Env,
    builder: &External<BuilderHandle>,
    relation: u32,
    rows: napi::bindgen_prelude::BigInt,
    cells: napi::bindgen_prelude::Array,
) -> napi::Result<MutationReportWire> {
    let rows = marshal::u64_in(&rows, "collection rows")?;
    let collection =
        marshal::accepted_collection(env, &builder.sealed.rosters, relation, rows, &cells)?;
    let report = live_mut(&builder.inner, "builder")?
        .delete_accepted(&collection)
        .map_err(|error| throw_engine(env, &error))?;
    Ok(mutation_report_wire(report))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn instance_builder_contains(
    env: Env,
    builder: &External<BuilderHandle>,
    relation: u32,
    values: napi::bindgen_prelude::Array,
) -> napi::Result<bool> {
    let row = marshal::fact_row(&builder.sealed.rosters, relation, &values)?;
    live_mut(&builder.inner, "builder")?
        .contains_dyn(row.0, &row.1)
        .map_err(|error| throw_engine(env, &error))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn instance_builder_get(
    env: Env,
    builder: &External<BuilderHandle>,
    relation: u32,
    key_statement: u32,
    key_values: napi::bindgen_prelude::Array,
) -> napi::Result<Option<Vec<marshal::ValueOut>>> {
    let (rel, key, row) = marshal::key_row(
        &builder.sealed.rosters,
        &builder.sealed.statements,
        relation,
        key_statement,
        &key_values,
    )?;
    let found = live_mut(&builder.inner, "builder")?
        .get_dyn(rel, key, &row)
        .map_err(|error| throw_engine(env, &error))?;
    Ok(found.map(|values| {
        values
            .into_iter()
            .map(marshal::ValueOut::from_value)
            .collect()
    }))
}

#[napi]
pub fn instance_builder_close(builder: &External<BuilderHandle>) -> napi::Result<()> {
    take_handle(&builder.inner, "builder")?;
    Ok(())
}

pub(crate) fn mutation_report_wire(report: bumbledb::MutationReport) -> MutationReportWire {
    MutationReportWire::Report {
        submitted: report.submitted(),
        changed: report.changed(),
    }
}

// ---------------------------------------------------------------------------
// Owned instances: heap-resident admitted state (`Send + Sync`), the log's
// materialization input. Point verbs stay synchronous (bounded owned point
// work); scans, queries and publish run on the executor.
// ---------------------------------------------------------------------------

pub struct OwnedHandle {
    inner: RefCell<Option<OwnedSlot>>,
}

pub(crate) struct OwnedSlot {
    pub(crate) instance: Arc<OwnedInstance<SchemaDescriptor>>,
    pub(crate) sealed: Arc<Sealed>,
    accounted: Cell<i64>,
    pub(crate) leased: Arc<AtomicBool>,
}

fn account_bytes(env: Env, bytes: usize) -> napi::Result<i64> {
    let delta = i64::try_from(bytes).unwrap_or(i64::MAX);
    if delta != 0 {
        env.adjust_external_memory(delta)?;
    }
    Ok(delta)
}

fn release_accounted(env: Env, accounted: i64) -> napi::Result<()> {
    if accounted != 0 {
        env.adjust_external_memory(-accounted)?;
    }
    Ok(())
}

fn sync_owned_accounted(env: Env, slot: &OwnedSlot) -> napi::Result<()> {
    let want = i64::try_from(slot.instance.retained_bytes()).unwrap_or(i64::MAX);
    let have = slot.accounted.get();
    let delta = want.saturating_sub(have);
    if delta != 0 {
        env.adjust_external_memory(delta)?;
        slot.accounted.set(want);
    }
    Ok(())
}

/// Wraps an executor-admitted instance into a JS-held owned handle with
/// external-memory accounting. Runs at take time on the JS thread.
pub(crate) fn owned_wrap(
    env: Env,
    instance: OwnedInstance<SchemaDescriptor>,
    sealed: Arc<Sealed>,
) -> napi::Result<External<OwnedHandle>> {
    let accounted = account_bytes(env, instance.retained_bytes())?;
    Ok(External::new(OwnedHandle {
        inner: RefCell::new(Some(OwnedSlot {
            instance: Arc::new(instance),
            sealed,
            accounted: Cell::new(accounted),
            leased: Arc::new(AtomicBool::new(false)),
        })),
    }))
}

/// Shares the owned instance for an executor job (scan/query/publish) and
/// marks it leased so close refuses while native work retains it. The
/// caller clears the flag with [`OwnedLeaseFlag`]'s drop.
pub(crate) fn owned_lease(
    instance: &External<OwnedHandle>,
) -> napi::Result<(
    Arc<OwnedInstance<SchemaDescriptor>>,
    Arc<Sealed>,
    OwnedLeaseFlag,
)> {
    let slot = live(&instance.inner, "owned instance")?;
    if slot.leased.swap(true, Ordering::AcqRel) {
        return Err(leased_handle("owned instance"));
    }
    Ok((
        Arc::clone(&slot.instance),
        Arc::clone(&slot.sealed),
        OwnedLeaseFlag(Arc::clone(&slot.leased)),
    ))
}

/// Clears the owned-instance lease flag when the executor job settles,
/// however it settles.
pub(crate) struct OwnedLeaseFlag(Arc<AtomicBool>);

impl Drop for OwnedLeaseFlag {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[napi]
pub fn owned_instance_close(env: Env, instance: &External<OwnedHandle>) -> napi::Result<()> {
    {
        let slot = live(&instance.inner, "owned instance")?;
        if slot.leased.load(Ordering::Acquire) {
            return Err(leased_handle("owned instance"));
        }
    }
    let slot = take_handle(&instance.inner, "owned instance")?;
    release_accounted(env, slot.accounted.get())
}

#[napi]
pub fn owned_count(env: Env, instance: &External<OwnedHandle>, relation: u32) -> napi::Result<u64> {
    let slot = live(&instance.inner, "owned instance")?;
    let count = slot
        .instance
        .count(bumbledb::RelationId(relation))
        .map_err(|error| throw_engine(env, &error))?;
    sync_owned_accounted(env, &slot)?;
    Ok(count)
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn owned_contains(
    env: Env,
    instance: &External<OwnedHandle>,
    relation: u32,
    values: napi::bindgen_prelude::Array,
) -> napi::Result<bool> {
    let slot = live(&instance.inner, "owned instance")?;
    let row = marshal::fact_row(&slot.sealed.rosters, relation, &values)?;
    let found = slot
        .instance
        .contains_dyn(row.0, &row.1)
        .map_err(|error| throw_engine(env, &error))?;
    sync_owned_accounted(env, &slot)?;
    Ok(found)
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn owned_get(
    env: Env,
    instance: &External<OwnedHandle>,
    relation: u32,
    key_statement: u32,
    key_values: napi::bindgen_prelude::Array,
) -> napi::Result<Option<Vec<marshal::ValueOut>>> {
    let slot = live(&instance.inner, "owned instance")?;
    let (rel, key, row) = marshal::key_row(
        &slot.sealed.rosters,
        &slot.sealed.statements,
        relation,
        key_statement,
        &key_values,
    )?;
    let found = slot
        .instance
        .get_dyn(rel, key, &row)
        .map_err(|error| throw_engine(env, &error))?;
    sync_owned_accounted(env, &slot)?;
    Ok(found.map(|values| {
        values
            .into_iter()
            .map(marshal::ValueOut::from_value)
            .collect()
    }))
}

/// Collects a scan iterator into owned rows on the executor.
pub(crate) fn scan_rows(
    iter: impl Iterator<Item = bumbledb::Result<Vec<Value>>>,
) -> bumbledb::Result<Vec<Vec<Value>>> {
    let mut rows = Vec::new();
    for row in iter {
        rows.push(row?);
    }
    Ok(rows)
}

#[cfg(test)]
mod handle_lifecycle {
    use super::*;

    #[test]
    fn take_handle_reentrant_borrow_is_typed_error() {
        let cell = RefCell::new(Some(1_u8));
        let error = {
            let _guard = cell.borrow();
            take_handle(&cell, "db").expect_err("re-entrant take must not panic")
        };
        assert!(
            error.reason.contains("re-entrant use of a db handle"),
            "{}",
            error.reason
        );
        assert!(
            cell.borrow().is_some(),
            "re-entrant take must not spend the handle"
        );
    }

    #[test]
    fn take_handle_closed_is_typed_error() {
        let cell = RefCell::new(None::<u8>);
        let error = take_handle(&cell, "instance").expect_err("empty handle");
        assert!(
            error.reason.contains("closed instance handle"),
            "{}",
            error.reason
        );
    }

    #[test]
    fn take_handle_spends_the_value() {
        let cell = RefCell::new(Some(7_u8));
        assert_eq!(take_handle(&cell, "db").expect("live handle"), 7);
        assert!(cell.borrow().is_none());
    }

    #[test]
    fn owned_lease_flag_is_one_at_a_time_and_clears_on_drop() {
        let leased = Arc::new(AtomicBool::new(false));
        assert!(!leased.swap(true, Ordering::AcqRel));
        let flag = OwnedLeaseFlag(Arc::clone(&leased));
        assert!(leased.load(Ordering::Acquire), "lease flag held");
        drop(flag);
        assert!(
            !leased.load(Ordering::Acquire),
            "lease flag cleared on drop, however the job settles"
        );
    }
}
