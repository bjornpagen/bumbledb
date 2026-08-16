//! The dumb-bridge law: no logic beyond marshaling will EVER live in this
//! crate. No schema knowledge beyond schema-DIRECTED marshaling, no
//! validation, no name resolution, no retries, no logging — anything smart
//! belongs in the TypeScript SDK or in the bumbledb engine itself
//! (docs/graph-builder-rebirth/prd-03, prd-04). This crate exists only to
//! carry values across the Node-API boundary.
//!
//! # Threading model (the closure inversion)
//!
//! The engine's snapshot and write-transaction surfaces are closure-scoped
//! (`Db::read(|snap| …)`, `Db::write(|tx| …)`) and closures cannot cross the
//! FFI, so each live snapshot or write transaction is a dedicated worker
//! thread PARKED inside the engine closure, serving one request at a time
//! over an mpsc channel pair. Every request is a synchronous round trip: the
//! JS thread sends, blocks on the reply, and returns. `PreparedQuery` is
//! `!Sync` (interior scratch); the one raw pointer that crosses to the
//! snapshot worker during `preparedExecute` / `preparedExplain` /
//! `preparedStaleness` is taken under an exclusive `in_flight` lease: the JS
//! thread briefly borrows the handle only to refuse re-entry, set the lease,
//! and extract the address from `UnsafeCell`, then DROPS that `Ref`/`RefMut`
//! before `SnapWorker::call`. Close/`take_handle` refuse the same lease with
//! a typed re-entrant error. The worker dereferences the pointer only while
//! the lease is held and no typed Rust reference exists on the JS thread —
//! that lease, not "JS is blocked so a live `RefMut` is fine," is the
//! soundness argument. The `dbWriteFrom` witness is NOT a pointer: the
//! snapshot worker mints the engine's own `Witness` value inside its closure
//! and the value moves — snapshot close order cannot dangle anything.
//! `WriteTx` point reads are the engine's own final-state view, live, never
//! simulated.
//!
//! # Handle lifecycle
//!
//! Every handle owns its Rust value through a napi `External`; double-close
//! and use-after-close throw typed programming errors. The `Db` handle keeps
//! the LMDB exclusive advisory lock: ONE process owns one store, and the
//! environment closes when the last dependent handle (snapshot, transaction,
//! prepared query — each holds an `Arc` on the engine) is closed or
//! collected. One write transaction may be open per Db handle at a time: the
//! engine is single-writer and the JS thread is the only caller, so a second
//! concurrent `dbWriteBegin` would deadlock the process against its own
//! writer mutex — the bridge refuses it with a typed error instead.
//!
//! # Error taxonomy
//!
//! Domain outcomes are DATA (`{ ok: … }` results): schema errors,
//! fingerprint mismatches, commit rejections with their full violation
//! rendering, generation moves, IR validation errors. Language and shape
//! errors THROW: marshaling mismatches (naming relation/field/expected/got),
//! use-after-close, engine `FactShape`/storage errors.

use std::cell::{Ref, RefCell, RefMut, UnsafeCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

use bumbledb::schema::{SpecIssue, StatementDescriptor};
use bumbledb::{
    Answers, BindValue, Db, Error, Exhumed, FieldId, FreshRange, ParamArg, PreparedQuery,
    RelationId, SchemaDescriptor, Snapshot, StatementId, Theory, Value, Violations, Witness,
    WriteTx, exhume, render_rejection,
};
use napi::bindgen_prelude::{Array, BigInt, External, Object, ToNapiValue};
use napi::sys;
use napi_derive::napi;

#[cfg(test)]
mod fingerprint_lock;
mod marshal;
mod tags;

use marshal::{ExplainWire, ManifestWire, OwnedParam, StalenessWire, ValueOut, ViolationWire};

/// Proof-of-life export for the package scaffold (PRD-03): evidence that the
/// path dependency on the sibling bumbledb engine compiles, links, and loads
/// through Node-API. The engine crate exposes no runtime crate-version
/// accessor, so this reports the bridge crate's own version alongside the
/// engine's `STORAGE_FORMAT_VERSION` — a genuine engine export, which is what
/// makes the string proof rather than decoration. The crate version rides
/// the release lockstep (Cargo.toml == npm manifests, finding 139), so the
/// string identifies the SHIPPED release, not a scaffold-frozen number.
#[napi]
#[must_use]
pub fn engine_version() -> String {
    format!(
        "bumbledb-node {} (bumbledb storage format v{})",
        env!("CARGO_PKG_VERSION"),
        bumbledb::STORAGE_FORMAT_VERSION
    )
}

/// The sealed schema data every handle carries for schema-directed
/// marshaling and violation rendering: the descriptor plus its materialized
/// statement roster (computed once per open — key point reads resolve their
/// statement's projection here).
struct Sealed {
    descriptor: SchemaDescriptor,
    statements: Vec<StatementDescriptor>,
}

/// The engine typestate every handle shares: runtime-built schemas all live
/// at `Db<SchemaDescriptor>` (the descriptor implements `Theory` as itself).
type Engine = Db<SchemaDescriptor>;

/// A closed-over engine error rendered for the reply channel.
struct WireError(String);

fn wire(error: &Error) -> WireError {
    WireError(marshal::engine_err(error))
}

fn thrown(error: WireError) -> napi::Error {
    marshal::err(error.0)
}

/// The one sentinel error the transaction worker returns from the write
/// closure to make the engine ABORT (an `Err` return drops the delta; LMDB
/// was never touched). Never surfaces to JS: the worker's ending state
/// decides what crosses back.
fn abort_sentinel() -> Error {
    Error::Io(std::io::Error::other("bumbledb-node transaction abort"))
}

fn closed_handle(what: &str) -> napi::Error {
    marshal::err(format!("bumbledb: use of a closed {what} handle"))
}

fn reentrant_use(what: &str) -> napi::Error {
    marshal::err(format!("bumbledb: re-entrant use of a {what} handle"))
}

fn worker_died(what: &str) -> napi::Error {
    marshal::err(format!("bumbledb: the {what} worker thread died"))
}

/// An engine error thrown across the boundary — the one spelling of the
/// render-then-throw chain (five call sites; domain outcomes never ride it).
fn throw_engine(error: &Error) -> napi::Error {
    marshal::err(marshal::engine_err(error))
}

/// Takes a handle's inner value, spending it — the shared close/commit/abort
/// seam: `None` (already spent) is the typed use-after-close refusal.
/// Re-entrant `RefCell` use throws (the same error as [`live`]/[`live_mut`])
/// rather than panicking across napi.
fn take_handle<T>(cell: &RefCell<Option<T>>, what: &str) -> napi::Result<T> {
    let mut borrowed = cell.try_borrow_mut().map_err(|_| reentrant_use(what))?;
    borrowed.take().ok_or_else(|| closed_handle(what))
}

/// The reply-unwrap triplet, ONE spelling (cleanup-0.5.0 U3 kill 12): a
/// worker call's reply is the expected variant carrying `Ok` (the value),
/// the expected variant carrying `Err` (an engine error, thrown), or the
/// wrong variant (the worker died mid-protocol). Ten call sites ride this
/// macro; the commit/begin verdicts keep their own richer matches.
macro_rules! reply {
    ($call:expr, $variant:path, $what:literal) => {
        match $call {
            $variant(Ok(value)) => Ok(value),
            $variant(Err(error)) => Err(thrown(error)),
            _ => Err(worker_died($what)),
        }
    };
}

/// One domain-outcome `ToNapiValue` impl per line of shape (cleanup-0.5.0
/// U3 kill 12): every outcome crosses as a plain object built key-by-key
/// from its variant's own fields — the five near-clone impls are one
/// declaration each.
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
                // SAFETY: `env` is the live environment napi handed this very
                // call, and `obj` was created against it two lines up.
                unsafe { Object::to_napi_value(env, obj) }
            }
        }
    };
}

/// Borrows a handle's live inner value or throws the typed
/// use-after-close error.
fn live<'a, T>(cell: &'a RefCell<Option<T>>, what: &str) -> napi::Result<Ref<'a, T>> {
    let borrowed = cell.try_borrow().map_err(|_| reentrant_use(what))?;
    Ref::filter_map(borrowed, Option::as_ref).map_err(|_| closed_handle(what))
}

/// [`live`], mutably.
fn live_mut<'a, T>(cell: &'a RefCell<Option<T>>, what: &str) -> napi::Result<RefMut<'a, T>> {
    let borrowed = cell.try_borrow_mut().map_err(|_| reentrant_use(what))?;
    RefMut::filter_map(borrowed, Option::as_mut).map_err(|_| closed_handle(what))
}

// ---------------------------------------------------------------------------
// Db handle
// ---------------------------------------------------------------------------

/// The opaque database handle JS holds through an `External`.
pub struct DbHandle {
    inner: RefCell<Option<DbInner>>,
}

struct DbInner {
    db: Arc<Engine>,
    sealed: Arc<Sealed>,
    /// The one-open-write-transaction guard (module doc: a second begin
    /// would self-deadlock the JS thread on the engine's writer mutex).
    tx_open: Arc<AtomicBool>,
}

/// `dbCreate`/`dbOpen`'s domain outcome. `NewtypeMismatch` is the
/// coherence wall's own kind — a spec whose statement pairs faces with
/// disagreeing newtype labels (the engine twin of the SDK's schema-level
/// class wall; unreachable through the typed builder, provable through a
/// raw spec) — carved out of `SchemaError` so the SDK's rejection is
/// typed. The carve-out is a match on the lowering's OWN issue list, no
/// judgment here (the dumb-bridge law holds).
pub enum OpenOutcome {
    Ok(External<DbHandle>),
    SchemaError(String),
    NewtypeMismatch(String),
    FingerprintMismatch(String),
}

outcome_to_napi!(OpenOutcome {
    Ok(handle) => { "ok": true, "db": handle },
    SchemaError(message) => { "ok": false, "kind": "schemaError", "message": message },
    NewtypeMismatch(message) => { "ok": false, "kind": "newtypeMismatch", "message": message },
    FingerprintMismatch(message) => { "ok": false, "kind": "fingerprintMismatch", "message": message },
});

fn open_with(
    path: &str,
    spec: &Object,
    open: impl FnOnce(&std::path::Path, SchemaDescriptor) -> bumbledb::Result<Engine>,
) -> napi::Result<OpenOutcome> {
    let spec = marshal::schema_spec(spec)?;
    let descriptor = match spec.descriptor() {
        Ok(descriptor) => descriptor,
        Err(error) => {
            // The coherence wall's kind rides its own arm; the message
            // still carries the COMPLETE issue list either way.
            let mismatched = error
                .issues()
                .iter()
                .any(|issue| matches!(issue, SpecIssue::StatementNewtypeMismatch { .. }));
            return Ok(if mismatched {
                OpenOutcome::NewtypeMismatch(error.to_string())
            } else {
                OpenOutcome::SchemaError(error.to_string())
            });
        }
    };
    let statements = descriptor.materialized_statements();
    match open(std::path::Path::new(path), descriptor.clone()) {
        Ok(db) => Ok(OpenOutcome::Ok(External::new(DbHandle {
            inner: RefCell::new(Some(DbInner {
                db: Arc::new(db),
                sealed: Arc::new(Sealed {
                    descriptor,
                    statements,
                }),
                tx_open: Arc::new(AtomicBool::new(false)),
            })),
        }))),
        Err(Error::Schema(error)) => Ok(OpenOutcome::SchemaError(error.to_string())),
        Err(error @ Error::SchemaMismatch { .. }) => Ok(OpenOutcome::FingerprintMismatch(
            marshal::engine_err(&error),
        )),
        Err(error) => Err(throw_engine(&error)),
    }
}

/// Creates a fresh DURABLE store at `path` from a `SchemaSpec` (frozen
/// ruling 3: the bridge exposes no ephemeral kind). Schema resolution and
/// validation failures return as data; environment failures throw.
#[napi]
#[allow(
    clippy::needless_pass_by_value,
    reason = "napi export signature: the FFI hands the bridge an owned String \\
              (an `expect` is untrackable through the #[napi] expansion)"
)]
pub fn db_create(path: String, spec: Object) -> napi::Result<OpenOutcome> {
    open_with(&path, &spec, |path, descriptor| {
        Db::create(path, descriptor)
    })
}

/// Opens an existing durable store, verifying format version, store kind,
/// and schema fingerprint (`fingerprintMismatch` as data).
#[napi]
#[allow(
    clippy::needless_pass_by_value,
    reason = "napi export signature: the FFI hands the bridge an owned String \\
              (an `expect` is untrackable through the #[napi] expansion)"
)]
pub fn db_open(path: String, spec: Object) -> napi::Result<OpenOutcome> {
    open_with(&path, &spec, Db::open)
}

/// Closes the handle. Dependent handles (snapshots, transactions, prepared
/// queries) each hold their own reference on the engine; the environment —
/// and its exclusive lock — releases when the last of them closes.
#[napi]
pub fn db_close(db: &External<DbHandle>) -> napi::Result<()> {
    take_handle(&db.inner, "db")?;
    Ok(())
}

/// The PRD-02 manifest: every name → id table of the theory, one plain JS
/// object, rendered off the descriptor (called once per open by the SDK).
#[napi]
pub fn db_manifest(db: &External<DbHandle>) -> napi::Result<ManifestWire> {
    let inner = live(&db.inner, "db")?;
    Ok(ManifestWire(inner.sealed.descriptor.clone().manifest()))
}

/// The open store's schema fingerprint, 64 lowercase hex chars — the
/// cross-host identity readback (the fingerprint lock, `fingerprint_lock`
/// module + `test/fingerprint.test.ts`). `dbCreate` stored this exact value
/// and `dbOpen` verified it, so the descriptor's fingerprint IS the store's.
/// Dumb-bridge legal: validation and blake3 are the ENGINE's own functions
/// re-run on the already-admitted descriptor (they cannot fail here); the
/// bridge only hex-encodes the 32 bytes for the wire.
#[napi]
pub fn db_fingerprint(db: &External<DbHandle>) -> napi::Result<String> {
    use bumbledb::schema::ValidateDescriptor as _;
    let inner = live(&db.inner, "db")?;
    let schema = inner
        .sealed
        .descriptor
        .clone()
        .validate()
        .map_err(|error| marshal::err(error.to_string()))?;
    let fingerprint = bumbledb::schema::fingerprint::fingerprint(&schema);
    Ok(hex_fingerprint(&fingerprint.0))
}

/// 32 fingerprint bytes as their 64 lowercase hex chars — the ONE wire
/// spelling of the cross-host identity (the `fingerprint_lock` test renders
/// its pin through this same function).
fn hex_fingerprint(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

/// The current committed generation (diagnostics; the write-side witness is
/// always the snapshot handle, never this integer — the engine's recorded
/// refusal of fabricable witnesses).
#[napi]
pub fn db_generation(db: &External<DbHandle>) -> napi::Result<u64> {
    let inner = live(&db.inner, "db")?;
    match inner.db.generation() {
        Ok(generation) => Ok(generation.value()),
        Err(error) => Err(throw_engine(&error)),
    }
}

// ---------------------------------------------------------------------------
// Exhume handle (the read-only, theory-less open — engine 70-api.md § exhume)
// ---------------------------------------------------------------------------

/// The opaque exhume handle: the engine's [`Exhumed`] — a store opened FROM
/// ITS OWN PERSISTED DESCRIPTOR, no caller schema anywhere. Lifetimes are
/// disposables (ruled 2026-07-23, R12): `exhumeClose` is the deterministic
/// teardown the SDK's `Symbol.dispose` rides — releasing the environment
/// scope-shaped, never a GC race; the `External`'s drop remains the
/// reclamation-only backstop for a collected-but-undisposed handle (the
/// store is never written through this type, so there is nothing to
/// flush). No worker thread parks here: every read is one self-contained
/// `Exhumed::read` round trip on the calling JS thread, so no snapshot
/// ever lives across the FFI boundary.
pub struct ExhumeHandle {
    inner: RefCell<Option<Exhumed>>,
}

/// `dbExhume`'s domain outcome: the live handle, or one of the three
/// adoption-era refusals as data (`descriptorMissing`, `formatMismatch`,
/// `corruption`). Everything else — a missing path, an unreadable
/// environment — throws, exactly as `dbOpen`'s environment failures do.
/// A writer's held lock is on neither list: the lock law is a writer law
/// (ruled 2026-07-23, R17), so exhume opens lockless read-only — another
/// process's live writer never turns the archival read away, and the
/// lane works on read-only media outright.
#[expect(
    clippy::large_enum_variant,
    reason = "the outcome is built once, marshaled to JS, and dropped — it \
              never sits in a collection, so boxing would buy one allocation \
              per crossing and nothing else"
)]
pub enum ExhumeOutcome {
    Ok(External<ExhumeHandle>),
    Refused { kind: &'static str, message: String },
}

outcome_to_napi!(ExhumeOutcome {
    Ok(handle) => { "ok": true, "exhume": handle },
    Refused { kind, message } => { "ok": false, "kind": kind, "message": message },
});

/// Opens a store from its persisted descriptor (`bumbledb::exhume` — the
/// crate-root entry; `Db<S>`'s typestate is a theory and this entry's whole
/// point is having none). The typed adoption refusals cross as DATA: a
/// store not yet adopted (`DescriptorMissing` — the remedy is one
/// fingerprint-matching open under the creating schema), a format-version
/// mismatch, and the descriptor integrity corruptions
/// (fingerprint/descriptor desync, undecodable bytes).
#[napi]
#[allow(
    clippy::needless_pass_by_value,
    reason = "napi export signature: the FFI hands the bridge an owned String \\
              (an `expect` is untrackable through the #[napi] expansion)"
)]
pub fn db_exhume(path: String) -> napi::Result<ExhumeOutcome> {
    match exhume(std::path::Path::new(&path)) {
        Ok(exhumed) => Ok(ExhumeOutcome::Ok(External::new(ExhumeHandle {
            inner: RefCell::new(Some(exhumed)),
        }))),
        Err(error @ Error::DescriptorMissing) => Ok(ExhumeOutcome::Refused {
            kind: "descriptorMissing",
            message: marshal::engine_err(&error),
        }),
        Err(error @ Error::FormatMismatch { .. }) => Ok(ExhumeOutcome::Refused {
            kind: "formatMismatch",
            message: marshal::engine_err(&error),
        }),
        Err(error @ Error::Corruption(_)) => Ok(ExhumeOutcome::Refused {
            kind: "corruption",
            message: marshal::engine_err(&error),
        }),
        Err(error) => Err(throw_engine(&error)),
    }
}

/// The exhumed store's persisted schema as manifest-shaped plain data — the
/// engine's own `SchemaDescriptor::manifest()` rendering, verbatim (the
/// dumb-bridge law: no second descriptor decoder exists here): relations in
/// engine-id order, sealed field lists (a closed relation opens with the
/// synthetic (`id`, u64) handle field) with structural value types, and
/// closed-relation rosters.
#[napi]
pub fn exhume_descriptor(exhume: &External<ExhumeHandle>) -> napi::Result<ManifestWire> {
    let exhumed = live(&exhume.inner, "exhume")?;
    Ok(ManifestWire(exhumed.descriptor().clone().manifest()))
}

/// Closes the exhume handle, releasing its read-only environment
/// deterministically — the native teardown under the SDK's
/// `Symbol.dispose` (ruled 2026-07-23, R12: lifetimes are disposables,
/// never `close()` methods to remember). No lock releases here because
/// none was taken: the lock law is a writer law (ruled 2026-07-23, R17),
/// and exhume reads lockless.
#[napi]
pub fn exhume_close(exhume: &External<ExhumeHandle>) -> napi::Result<()> {
    take_handle(&exhume.inner, "exhume")?;
    Ok(())
}

/// Full-relation export by NAME in row-id order, values decoded per the
/// STORED descriptor (str resolved through `_dict` inside the engine; a
/// closed relation scans its sealed roster). Each call is one
/// self-contained snapshot read. An unknown relation name throws — the
/// descriptor is the caller's roster, so a miss is a programming error,
/// never a domain outcome.
#[napi]
#[allow(
    clippy::needless_pass_by_value,
    reason = "napi export signature: the FFI hands the bridge an owned String \\
              (an `expect` is untrackable through the #[napi] expansion)"
)]
pub fn exhume_scan(
    exhume: &External<ExhumeHandle>,
    relation_name: String,
) -> napi::Result<Vec<Vec<ValueOut>>> {
    let exhumed = live(&exhume.inner, "exhume")?;
    let Some(relation) = exhumed.relation(&relation_name) else {
        return Err(marshal::err(format!(
            "bumbledb: the exhumed store's descriptor declares no relation `{relation_name}`"
        )));
    };
    let rows = exhumed.read(|snap| {
        let iter = snap.scan(relation)?;
        let mut rows = Vec::new();
        for row in iter {
            rows.push(row?);
        }
        Ok(rows)
    });
    match rows {
        Ok(rows) => marshal::rows_out(rows),
        Err(error) => Err(throw_engine(&error)),
    }
}

// ---------------------------------------------------------------------------
// Snapshot handle (worker thread parked inside `Db::read`)
// ---------------------------------------------------------------------------

/// A leased `PreparedQuery` pointer that may cross to the snapshot worker.
/// Raw `*mut` is `!Send` on this toolchain; the `in_flight` lease is the
/// exclusive-access proof (`PreparedQuery` is `!Sync`).
#[repr(transparent)]
struct LeasedPrepared(*mut PreparedQuery<'static, SchemaDescriptor>);

// SAFETY: only `lease_prepared` constructs this, after setting `in_flight`.
// The JS thread holds no `Ref`/`RefMut` to `PreparedInner` while the
// pointer is in flight; the snapshot worker is the only dereference;
// `take_prepared` refuses the lease so the pointee outlives the send.
#[expect(
    unsafe_code,
    reason = "the in_flight lease is exclusive access for a !Sync pointee \
              whose raw pointer is !Send on this toolchain"
)]
unsafe impl Send for LeasedPrepared {}

enum SnapReq {
    Scan(RelationId),
    Contains(RelationId, Vec<Value>),
    Get(RelationId, StatementId, Vec<Value>),
    Execute {
        /// `UnsafeCell::get()` pointer from a live `in_flight` lease —
        /// the JS thread holds no `Ref`/`RefMut` to `PreparedInner` while
        /// this request is in flight (module doc, threading model).
        prepared: LeasedPrepared,
        params: Vec<OwnedParam>,
    },
    Explain {
        /// Same exclusive lease as `Execute`.
        prepared: LeasedPrepared,
        params: Vec<OwnedParam>,
    },
    Staleness {
        /// Same exclusive lease as `Execute` (`PreparedQuery` is `!Sync`,
        /// so even a shared engine method rides the exclusive pointer).
        prepared: LeasedPrepared,
    },
    /// Replies with the parked `Snapshot`'s minted [`Witness`] value for
    /// `dbWriteFrom` — evidence minted where the snapshot lives, moved
    /// across the channel; no snapshot reference ever leaves this worker.
    Witness,
    Close,
}

enum SnapReply {
    /// Open verdict, carrying the snapshot's witnessed generation — read
    /// inside the snapshot's own transaction on the open round trip
    /// (finding 016: the host's second `dbGeneration` crossing and its
    /// fault-pairing close dance die; the race-closing rule of
    /// `50-storage.md` holds by construction).
    Ready(Result<u64, WireError>),
    Rows(Result<Vec<Vec<Value>>, WireError>),
    /// The executed [`Answers`] carrier, crossed WHOLE (owned, `Send`) —
    /// the engine's flat one-allocation buffer is already the right
    /// representation, so the worker ships it instead of rebuilding it
    /// as per-row value vectors (a full intermediate copy).
    Answers(Result<Answers, WireError>),
    Flag(Result<bool, WireError>),
    Row(Result<Option<Vec<Value>>, WireError>),
    Explain(Result<bumbledb::ExecutionStats, WireError>),
    Staleness(Result<StalenessWire, WireError>),
    Witness(Result<Witness<SchemaDescriptor>, WireError>),
}

/// The opaque snapshot handle: one worker thread parked inside `Db::read`,
/// holding one MVCC read snapshot across calls.
pub struct SnapshotHandle {
    inner: RefCell<Option<SnapWorker>>,
}

struct SnapWorker {
    requests: Sender<SnapReq>,
    replies: Receiver<SnapReply>,
    thread: Option<JoinHandle<()>>,
    sealed: Arc<Sealed>,
}

impl Drop for SnapWorker {
    fn drop(&mut self) {
        let _ = self.requests.send(SnapReq::Close);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl SnapWorker {
    fn call(&self, req: SnapReq) -> napi::Result<SnapReply> {
        self.requests
            .send(req)
            .map_err(|_| worker_died("snapshot"))?;
        self.replies.recv().map_err(|_| worker_died("snapshot"))
    }
}

/// One owned scalar to the engine's bind value. String payloads re-borrow
/// as `&str` (marshaling admitted only JS strings, so UTF-8 holds; a
/// corrupt payload is refused typed rather than unwrapped).
fn bind_value(value: &Value) -> Result<BindValue<'_>, WireError> {
    Ok(match value {
        Value::Bool(v) => BindValue::Bool(*v),
        Value::U64(v) => BindValue::U64(*v),
        Value::I64(v) => BindValue::I64(*v),
        Value::String(bytes) => BindValue::Str(
            std::str::from_utf8(bytes)
                .map_err(|_| WireError("bumbledb: non-UTF-8 string param".into()))?,
        ),
        Value::FixedBytes(bytes) => BindValue::FixedBytes(bytes),
        Value::IntervalU64(interval) => BindValue::IntervalU64(interval.start(), interval.end()),
        Value::IntervalI64(interval) => BindValue::IntervalI64(interval.start(), interval.end()),
    })
}

/// Owned params to the engine's positional bind arguments.
fn param_args(params: &[OwnedParam]) -> Result<Vec<ParamArg<'_>>, WireError> {
    params
        .iter()
        .map(|param| match param {
            OwnedParam::Set(values) => Ok(ParamArg::Set(values)),
            OwnedParam::Scalar(value) => Ok(ParamArg::Scalar(bind_value(value)?)),
        })
        .collect()
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "thread entry: the worker must OWN its engine reference and \
              channel ends for the 'static spawn"
)]
fn run_snapshot(db: Arc<Engine>, requests: Receiver<SnapReq>, replies: Sender<SnapReply>) {
    let outcome = db.read(|snap| {
        let generation = snap.generation()?;
        if replies
            .send(SnapReply::Ready(Ok(generation.value())))
            .is_err()
        {
            return Ok(());
        }
        while let Ok(req) = requests.recv() {
            let reply = match req {
                SnapReq::Scan(relation) => SnapReply::Rows(scan_rows(snap, relation)),
                SnapReq::Contains(relation, values) => {
                    SnapReply::Flag(snap.contains_dyn(relation, &values).map_err(|e| wire(&e)))
                }
                SnapReq::Get(relation, key, values) => {
                    SnapReply::Row(snap.get_dyn(relation, key, &values).map_err(|e| wire(&e)))
                }
                SnapReq::Execute { prepared, params } => {
                    // SAFETY: `prepared` is `lease_prepared`'s
                    // `UnsafeCell::get()` pointer. The JS thread dropped
                    // every `Ref`/`RefMut` to `PreparedInner` before
                    // `SnapWorker::call` and holds the `in_flight` lease
                    // until after `recv`; close refuses that lease. No
                    // second typed reference exists. `PreparedQuery` is
                    // `!Sync`; the lease is the exclusive-access proof.
                    #[expect(
                        unsafe_code,
                        reason = "a prepared query cannot cross the FFI as a closure \
                                  capture; its leased address rides the request"
                    )]
                    let prepared = unsafe { &mut *prepared.0 };
                    SnapReply::Answers(execute_answers(snap, prepared, &params))
                }
                SnapReq::Explain { prepared, params } => {
                    // SAFETY: same `in_flight` lease as `Execute`.
                    #[expect(
                        unsafe_code,
                        reason = "a prepared query cannot cross the FFI as a closure \
                                  capture; its leased address rides the request"
                    )]
                    let prepared = unsafe { &mut *prepared.0 };
                    SnapReply::Explain(explain_stats(snap, prepared, &params))
                }
                SnapReq::Staleness { prepared } => {
                    // SAFETY: same exclusive `in_flight` lease as `Execute`
                    // (`PreparedQuery` is `!Sync`).
                    #[expect(
                        unsafe_code,
                        reason = "a prepared query cannot cross the FFI as a closure \
                                  capture; its leased address rides the request"
                    )]
                    let prepared = unsafe { &*prepared.0 };
                    SnapReply::Staleness(staleness_wire(snap, prepared))
                }
                SnapReq::Witness => SnapReply::Witness(snap.witness().map_err(|e| wire(&e))),
                SnapReq::Close => break,
            };
            if replies.send(reply).is_err() {
                break;
            }
        }
        Ok(())
    });
    if let Err(error) = outcome {
        let _ = replies.send(SnapReply::Ready(Err(wire(&error))));
    }
}

fn scan_rows(
    snap: &Snapshot<'_, SchemaDescriptor>,
    relation: RelationId,
) -> Result<Vec<Vec<Value>>, WireError> {
    let iter = snap.scan(relation).map_err(|e| wire(&e))?;
    let mut rows = Vec::new();
    for row in iter {
        rows.push(row.map_err(|e| wire(&e))?);
    }
    Ok(rows)
}

fn execute_answers(
    snap: &Snapshot<'_, SchemaDescriptor>,
    prepared: &mut PreparedQuery<'static, SchemaDescriptor>,
    params: &[OwnedParam],
) -> Result<Answers, WireError> {
    let args = param_args(params)?;
    snap.execute_collect_args(prepared, &args)
        .map_err(|e| wire(&e))
}

/// The plan-as-data half of `Snapshot::profile` (ANALYZE semantics: the
/// query really executes, with counting instrumentation); the answers are
/// discarded — execute is the answer surface. Binds through the same
/// mixed scalar/set `ParamArg` marshaling as execute — the R13 symmetry:
/// whatever executes, explains.
fn explain_stats(
    snap: &Snapshot<'_, SchemaDescriptor>,
    prepared: &mut PreparedQuery<'static, SchemaDescriptor>,
    params: &[OwnedParam],
) -> Result<bumbledb::ExecutionStats, WireError> {
    let args = param_args(params)?;
    let (_, stats) = snap.profile(prepared, &args).map_err(|e| wire(&e))?;
    Ok(stats)
}

fn staleness_wire(
    snap: &Snapshot<'_, SchemaDescriptor>,
    prepared: &PreparedQuery<'static, SchemaDescriptor>,
) -> Result<StalenessWire, WireError> {
    let staleness = prepared.staleness(snap).map_err(|e| wire(&e))?;
    Ok(StalenessWire {
        per_occurrence: staleness
            .per_occurrence
            .iter()
            .map(|drift| (drift.relation.0, drift.pinned, drift.live, drift.ratio))
            .collect(),
        max_ratio: staleness.max_ratio,
    })
}

/// `dbSnapshot`'s reply: the live handle plus the snapshot's witnessed
/// generation — one crossing carries both, so no second `dbGeneration`
/// call (with its own transient read transaction and fault-pairing close
/// branch) exists to pay or defend (finding 016).
pub enum SnapshotOpened {
    Ok {
        handle: External<SnapshotHandle>,
        generation: u64,
    },
}

outcome_to_napi!(SnapshotOpened {
    Ok { handle, generation } => { "ok": true, "snapshot": handle, "generation": generation },
});

/// Opens one MVCC read snapshot as a live handle, returning it WITH its
/// witnessed generation (read inside the snapshot's own transaction —
/// the race-closing rule of `50-storage.md` holds by construction).
#[napi]
pub fn db_snapshot(db: &External<DbHandle>) -> napi::Result<SnapshotOpened> {
    let inner = live(&db.inner, "db")?;
    let (req_tx, req_rx) = channel::<SnapReq>();
    let (rep_tx, rep_rx) = channel::<SnapReply>();
    let engine = Arc::clone(&inner.db);
    let thread = std::thread::spawn(move || run_snapshot(engine, req_rx, rep_tx));
    match rep_rx.recv() {
        Ok(SnapReply::Ready(Ok(generation))) => Ok(SnapshotOpened::Ok {
            handle: External::new(SnapshotHandle {
                inner: RefCell::new(Some(SnapWorker {
                    requests: req_tx,
                    replies: rep_rx,
                    thread: Some(thread),
                    sealed: Arc::clone(&inner.sealed),
                })),
            }),
            generation,
        }),
        Ok(SnapReply::Ready(Err(error))) => {
            let _ = thread.join();
            Err(thrown(error))
        }
        _ => {
            let _ = thread.join();
            Err(worker_died("snapshot"))
        }
    }
}

/// Closes the snapshot, releasing its LMDB reader slot.
#[napi]
pub fn snapshot_close(snap: &External<SnapshotHandle>) -> napi::Result<()> {
    take_handle(&snap.inner, "snapshot")?;
    Ok(())
}

/// Full-relation export in `row_id` order: one natural-value row per fact
/// (a storage stream materialized whole — the ETL/derivation read).
#[napi]
pub fn snapshot_scan(
    snap: &External<SnapshotHandle>,
    relation: u32,
) -> napi::Result<Vec<Vec<ValueOut>>> {
    let worker = live(&snap.inner, "snapshot")?;
    let rows = reply!(
        worker.call(SnapReq::Scan(RelationId(relation)))?,
        SnapReply::Rows,
        "snapshot"
    )?;
    marshal::rows_out(rows)
}

/// Committed-state membership of one dynamic fact (sealed field order).
#[napi]
pub fn snapshot_contains(
    snap: &External<SnapshotHandle>,
    relation: u32,
    values: Array,
) -> napi::Result<bool> {
    let worker = live(&snap.inner, "snapshot")?;
    let (rel, row) = marshal::fact_row(&worker.sealed.descriptor, relation, &values)?;
    reply!(
        worker.call(SnapReq::Contains(rel, row))?,
        SnapReply::Flag,
        "snapshot"
    )
}

/// Committed-state point lookup of the full fact through a key statement
/// (`keyValues` in the statement's projection order); `null` on a miss.
#[napi]
pub fn snapshot_get(
    snap: &External<SnapshotHandle>,
    relation: u32,
    key_statement: u32,
    key_values: Array,
) -> napi::Result<Option<Vec<ValueOut>>> {
    let worker = live(&snap.inner, "snapshot")?;
    let (rel, key, row) = marshal::key_row(
        &worker.sealed.descriptor,
        &worker.sealed.statements,
        relation,
        key_statement,
        &key_values,
    )?;
    let found = reply!(
        worker.call(SnapReq::Get(rel, key, row))?,
        SnapReply::Row,
        "snapshot"
    )?;
    found
        .map(|values| values.into_iter().map(ValueOut::from_value).collect())
        .transpose()
}

// ---------------------------------------------------------------------------
// Write-transaction handle (worker thread parked inside `Db::write`)
// ---------------------------------------------------------------------------

enum TxReq {
    Insert(RelationId, Vec<Vec<Value>>),
    Delete(RelationId, Vec<Vec<Value>>),
    Contains(RelationId, Vec<Value>),
    Get(RelationId, StatementId, Vec<Value>),
    Reserve(RelationId, FieldId, u64),
    Commit,
    Abort,
}

/// Engine fresh range parsed at the worker: empty cannot yield a start.
enum RangeWire {
    Empty,
    NonEmpty { start: u64, end_exclusive: u64 },
}

enum TxReply {
    Ready,
    BeginMoved { witnessed: u64, current: u64 },
    BeginFailed(WireError),
    Flag(Result<bool, WireError>),
    Report(Result<(u64, u64), WireError>),
    Row(Result<Option<Vec<Value>>, WireError>),
    Range(Result<RangeWire, WireError>),
    Committed(Result<u64, WireError>),
    Rejected(Vec<ViolationWire>),
    Aborted,
}

/// How the serve loop left the write closure.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TxEnding {
    Commit,
    Abort,
    Abandoned,
}

/// The opaque write-transaction handle: one worker thread parked inside
/// `Db::write` / `Db::write_from`, holding the live delta across calls.
pub struct TxHandle {
    inner: RefCell<Option<TxWorker>>,
}

struct TxWorker {
    requests: Sender<TxReq>,
    replies: Receiver<TxReply>,
    thread: Option<JoinHandle<()>>,
    sealed: Arc<Sealed>,
    tx_open: Arc<AtomicBool>,
}

impl TxWorker {
    fn call(&self, req: TxReq) -> napi::Result<TxReply> {
        self.requests
            .send(req)
            .map_err(|_| worker_died("transaction"))?;
        self.replies.recv().map_err(|_| worker_died("transaction"))
    }

    fn finish(&mut self) {
        self.tx_open.store(false, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for TxWorker {
    /// A collected-but-uncommitted transaction aborts: dropping the request
    /// sender ends the serve loop's `recv`, the closure returns the abort
    /// sentinel, and the engine drops the delta (nothing ever touched LMDB).
    fn drop(&mut self) {
        let (dead, _) = channel::<TxReq>();
        self.requests = dead;
        self.finish();
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "thread entry: the worker must OWN its engine reference and \
              channel ends for the 'static spawn"
)]
fn run_tx(
    db: Arc<Engine>,
    sealed: Arc<Sealed>,
    witness: Option<Witness<SchemaDescriptor>>,
    requests: Receiver<TxReq>,
    replies: Sender<TxReply>,
) {
    let mut entered = false;
    let mut ending = TxEnding::Abandoned;
    let serve = |tx: &mut WriteTx<'_, SchemaDescriptor>| -> bumbledb::Result<()> {
        entered = true;
        if replies.send(TxReply::Ready).is_err() {
            return Err(abort_sentinel());
        }
        loop {
            let Ok(req) = requests.recv() else {
                return Err(abort_sentinel());
            };
            let reply = match req {
                TxReq::Insert(relation, rows) => TxReply::Report(
                    tx.insert_dyn(relation, rows)
                        .map(|r| (r.submitted, r.changed))
                        .map_err(|e| wire(&e)),
                ),
                TxReq::Delete(relation, rows) => TxReply::Report(
                    tx.delete_dyn(relation, rows)
                        .map(|r| (r.submitted, r.changed))
                        .map_err(|e| wire(&e)),
                ),
                TxReq::Contains(relation, values) => {
                    TxReply::Flag(tx.contains_dyn(relation, &values).map_err(|e| wire(&e)))
                }
                TxReq::Get(relation, key, values) => {
                    TxReply::Row(tx.get_dyn(relation, key, &values).map_err(|e| wire(&e)))
                }
                TxReq::Reserve(relation, field, count) => {
                    let range = db
                        .fresh_field(relation, field)
                        .map_err(Error::FactShape)
                        .and_then(|witness| {
                            tx.reserve_at(witness, count).map(|range| match range {
                                FreshRange::Empty => RangeWire::Empty,
                                FreshRange::NonEmpty { start, count } => RangeWire::NonEmpty {
                                    start,
                                    end_exclusive: start + count.get(),
                                },
                            })
                        });
                    TxReply::Range(range.map_err(|e| wire(&e)))
                }
                TxReq::Commit => {
                    ending = TxEnding::Commit;
                    return Ok(());
                }
                TxReq::Abort => {
                    ending = TxEnding::Abort;
                    return Err(abort_sentinel());
                }
            };
            if replies.send(reply).is_err() {
                return Err(abort_sentinel());
            }
        }
    };
    let result = match witness {
        // The witness is the engine's own minted value (`Snapshot::witness`
        // on the snapshot worker), moved here — snapshot close order is
        // irrelevant to soundness by representation.
        Some(witness) => db.write_from_witness(witness, serve),
        None => db.write(serve),
    };
    let reply = match result {
        Ok(()) => Some(TxReply::Committed(
            db.generation()
                .map(bumbledb::GenerationId::value)
                .map_err(|e| wire(&e)),
        )),
        Err(error) => match (entered, ending) {
            (_, TxEnding::Abort) => Some(TxReply::Aborted),
            (true, TxEnding::Abandoned) => None,
            (false, _) => match error {
                Error::GenerationMoved { witnessed, current } => Some(TxReply::BeginMoved {
                    witnessed: witnessed.value(),
                    current: current.value(),
                }),
                other => Some(TxReply::BeginFailed(wire(&other))),
            },
            (true, TxEnding::Commit) => match error {
                Error::CommitRejected { violations } => {
                    Some(TxReply::Rejected(violations_wire(&sealed, &violations)))
                }
                other => Some(TxReply::Committed(Err(wire(&other)))),
            },
        },
    };
    if let Some(reply) = reply {
        let _ = replies.send(reply);
    }
}

fn violations_wire(sealed: &Sealed, violations: &Violations) -> Vec<ViolationWire> {
    render_rejection(&sealed.descriptor, violations)
        .into_iter()
        .map(ViolationWire::from_rendered)
        .collect()
}

fn spawn_tx(inner: &DbInner, witness: Option<Witness<SchemaDescriptor>>) -> napi::Result<TxWorker> {
    if inner.tx_open.swap(true, Ordering::AcqRel) {
        return Err(marshal::err(
            "bumbledb: a write transaction is already open on this db handle \
             (single-writer engine; commit or abort it first)"
                .into(),
        ));
    }
    let (req_tx, req_rx) = channel::<TxReq>();
    let (rep_tx, rep_rx) = channel::<TxReply>();
    let engine = Arc::clone(&inner.db);
    let sealed = Arc::clone(&inner.sealed);
    let thread = match std::thread::Builder::new().spawn(move || {
        run_tx(engine, sealed, witness, req_rx, rep_tx);
    }) {
        Ok(thread) => thread,
        Err(error) => {
            inner.tx_open.store(false, Ordering::Release);
            return Err(marshal::err(format!(
                "bumbledb: failed to spawn the write-transaction worker: {error}"
            )));
        }
    };
    Ok(TxWorker {
        requests: req_tx,
        replies: rep_rx,
        thread: Some(thread),
        sealed: Arc::clone(&inner.sealed),
        tx_open: Arc::clone(&inner.tx_open),
    })
}

/// `dbWriteFrom`'s domain outcome.
pub enum WriteFromOutcome {
    Ok(External<TxHandle>),
    Moved { witnessed: u64, current: u64 },
}

outcome_to_napi!(WriteFromOutcome {
    Ok(handle) => { "ok": true, "tx": handle },
    Moved { witnessed, current } => {
        "ok": false,
        "kind": "generationMoved",
        "witnessed": witnessed,
        "current": current,
    },
});

/// Awaits the worker's begin verdict and wraps the live worker as a handle.
fn begin_outcome(mut worker: TxWorker) -> napi::Result<WriteFromOutcome> {
    match worker.replies.recv() {
        Ok(TxReply::Ready) => Ok(WriteFromOutcome::Ok(External::new(TxHandle {
            inner: RefCell::new(Some(worker)),
        }))),
        Ok(TxReply::BeginMoved { witnessed, current }) => {
            worker.finish();
            Ok(WriteFromOutcome::Moved { witnessed, current })
        }
        Ok(TxReply::BeginFailed(error)) => {
            worker.finish();
            Err(thrown(error))
        }
        _ => {
            worker.finish();
            Err(worker_died("transaction"))
        }
    }
}

/// Begins an unwitnessed write transaction: the submitted-delta lane —
/// operations accumulate through `tx*` calls, one `txCommit` judges.
#[napi]
pub fn db_write_begin(db: &External<DbHandle>) -> napi::Result<External<TxHandle>> {
    let inner = live(&db.inner, "db")?;
    let worker = spawn_tx(&inner, None)?;
    match begin_outcome(worker)? {
        WriteFromOutcome::Ok(handle) => Ok(handle),
        WriteFromOutcome::Moved { .. } => Err(marshal::err(
            "bumbledb: unreachable generationMoved on an unwitnessed write".into(),
        )),
    }
}

/// Begins a WITNESSED write transaction (`Db::write_from_witness`): the
/// witness is the snapshot handle's own minted [`Witness`] value —
/// evidence the snapshot worker constructs where the snapshot lives,
/// never a caller-supplied integer (the engine's recorded refusal). A
/// state-changing commit since the witness returns `generationMoved` as
/// data; retry policy stays host-side.
#[napi]
pub fn db_write_from(
    db: &External<DbHandle>,
    snap: &External<SnapshotHandle>,
) -> napi::Result<WriteFromOutcome> {
    let inner = live(&db.inner, "db")?;
    let witness = reply!(
        {
            let snap_worker = live(&snap.inner, "snapshot")?;
            snap_worker.call(SnapReq::Witness)?
        },
        SnapReply::Witness,
        "snapshot"
    )?;
    let worker = spawn_tx(&inner, Some(witness))?;
    begin_outcome(worker)
}

fn tx_flag(tx: &External<TxHandle>, req: TxReq) -> napi::Result<bool> {
    let worker = live(&tx.inner, "transaction")?;
    reply!(worker.call(req)?, TxReply::Flag, "transaction")
}

fn tx_report(
    tx: &External<TxHandle>,
    relation: u32,
    rows: &Array,
    req: fn(RelationId, Vec<Vec<Value>>) -> TxReq,
) -> napi::Result<MutationReportWire> {
    let facts = {
        let worker = live(&tx.inner, "transaction")?;
        marshal::fact_rows(&worker.sealed.descriptor, relation, rows)?
    };
    let worker = live(&tx.inner, "transaction")?;
    let (submitted, changed) = reply!(
        worker.call(req(facts.0, facts.1))?,
        TxReply::Report,
        "transaction"
    )?;
    Ok(MutationReportWire::Report {
        submitted,
        changed,
    })
}

/// The one row-verb body the flag verbs share: marshal the row
/// against the sealed descriptor, send the caller's request constructor,
/// unwrap the flag reply.
fn tx_row_flag(
    tx: &External<TxHandle>,
    relation: u32,
    values: &Array,
    req: fn(RelationId, Vec<Value>) -> TxReq,
) -> napi::Result<bool> {
    let row = {
        let worker = live(&tx.inner, "transaction")?;
        marshal::fact_row(&worker.sealed.descriptor, relation, values)?
    };
    tx_flag(tx, req(row.0, row.1))
}

/// Records a collection of inserts into the delta; returns the engine
/// `{ submitted, changed }` report. `rows` is an array of value-arrays
/// in sealed field order. Empty is lawful and still enters the
/// transaction (poison is observed). Shape violations throw typed;
/// nothing is judged until commit.
#[napi]
pub fn tx_insert(
    tx: &External<TxHandle>,
    relation: u32,
    rows: Array,
) -> napi::Result<MutationReportWire> {
    tx_report(tx, relation, &rows, TxReq::Insert)
}

/// Records a collection of deletes into the delta; returns the engine
/// `{ submitted, changed }` report.
#[napi]
pub fn tx_delete(
    tx: &External<TxHandle>,
    relation: u32,
    rows: Array,
) -> napi::Result<MutationReportWire> {
    tx_report(tx, relation, &rows, TxReq::Delete)
}

/// Final-state membership (base + pending delta — the view the commit
/// judgment judges, which is what makes check-then-act race-free).
#[napi]
pub fn tx_contains(tx: &External<TxHandle>, relation: u32, values: Array) -> napi::Result<bool> {
    tx_row_flag(tx, relation, &values, TxReq::Contains)
}

/// Final-state point lookup through a key statement; `null` on a miss.
#[napi]
pub fn tx_get(
    tx: &External<TxHandle>,
    relation: u32,
    key_statement: u32,
    key_values: Array,
) -> napi::Result<Option<Vec<ValueOut>>> {
    let worker = live(&tx.inner, "transaction")?;
    let (rel, key, row) = marshal::key_row(
        &worker.sealed.descriptor,
        &worker.sealed.statements,
        relation,
        key_statement,
        &key_values,
    )?;
    let found = reply!(
        worker.call(TxReq::Get(rel, key, row))?,
        TxReply::Row,
        "transaction"
    )?;
    found
        .map(|values| values.into_iter().map(ValueOut::from_value).collect())
        .transpose()
}

/// Facts consumed vs facts that changed the in-memory final-state view.
pub enum MutationReportWire {
    Report { submitted: u64, changed: u64 },
}

outcome_to_napi!(MutationReportWire {
    Report { submitted, changed } => { "submitted": submitted, "changed": changed },
});

/// Mints `count` consecutive fresh values for `(relation, field)`.
/// Empty cannot yield a start — that encoding lives at the C `{0, 0}`
/// wire, not on this object.
pub enum FreshRangeWire {
    Empty,
    NonEmpty { start: u64, end_exclusive: u64 },
}

outcome_to_napi!(FreshRangeWire {
    Empty => { "empty": true },
    NonEmpty { start, end_exclusive } => { "empty": false, "start": start, "endExclusive": end_exclusive },
});

/// Mints `count` consecutive fresh values for `(relation, field)`.
/// `count == 0` is empty and does not read or advance the sequence.
#[napi]
#[allow(clippy::needless_pass_by_value)] // napi BigInt is owned at the JS boundary
pub fn tx_reserve(
    tx: &External<TxHandle>,
    relation: u32,
    field: u32,
    count: BigInt,
) -> napi::Result<FreshRangeWire> {
    let worker = live(&tx.inner, "transaction")?;
    let field = u16::try_from(field)
        .map_err(|_| marshal::err(format!("bumbledb marshal: field id {field} exceeds u16")))?;
    let count = marshal::u64_in(&count, "reserve count")?;
    let range = reply!(
        worker.call(TxReq::Reserve(RelationId(relation), FieldId(field), count))?,
        TxReply::Range,
        "transaction"
    )?;
    Ok(match range {
        RangeWire::Empty => FreshRangeWire::Empty,
        RangeWire::NonEmpty {
            start,
            end_exclusive,
        } => FreshRangeWire::NonEmpty {
            start,
            end_exclusive,
        },
    })
}

/// `txCommit`'s domain outcome: the committed generation, or the COMPLETE
/// violation set in materialized statement order, rendered to plain data.
pub enum CommitOutcome {
    Committed(u64),
    Rejected(Vec<ViolationWire>),
}

outcome_to_napi!(CommitOutcome {
    Committed(generation) => { "ok": true, "generation": generation },
    Rejected(violations) => { "ok": false, "violations": violations },
});

/// Commits the delta: the engine judges every dependency statement against
/// the final state; a rejection carries the complete violation set as data.
/// Either way the handle is spent.
#[napi]
pub fn tx_commit(tx: &External<TxHandle>) -> napi::Result<CommitOutcome> {
    let mut taken = take_handle(&tx.inner, "transaction")?;
    let outcome = match taken.call(TxReq::Commit) {
        Ok(TxReply::Committed(Ok(generation))) => Ok(CommitOutcome::Committed(generation)),
        Ok(TxReply::Committed(Err(error))) => Err(thrown(error)),
        Ok(TxReply::Rejected(violations)) => Ok(CommitOutcome::Rejected(violations)),
        Ok(_) => Err(worker_died("transaction")),
        Err(error) => Err(error),
    };
    taken.finish();
    outcome
}

/// Aborts the delta — nothing ever touched LMDB. The handle is spent.
#[napi]
pub fn tx_abort(tx: &External<TxHandle>) -> napi::Result<()> {
    let mut taken = take_handle(&tx.inner, "transaction")?;
    let outcome = match taken.call(TxReq::Abort) {
        Ok(TxReply::Aborted) => Ok(()),
        Ok(_) => Err(worker_died("transaction")),
        Err(error) => Err(error),
    };
    taken.finish();
    outcome
}

// ---------------------------------------------------------------------------
// Prepared queries
// ---------------------------------------------------------------------------

/// The opaque prepared-query handle. Field order is load-bearing: the
/// prepared value borrows the engine through the `Arc` and must drop first.
pub struct PreparedHandle {
    inner: RefCell<Option<PreparedInner>>,
}

struct PreparedInner {
    prepared: UnsafeCell<PreparedQuery<'static, SchemaDescriptor>>,
    /// Exclusive lease for the snapshot-worker pointer. Set only around
    /// `SnapWorker::call`; close refuses while set.
    in_flight: AtomicBool,
    _db: Arc<Engine>,
}

/// Clears `in_flight` after the worker round trip. Does not hold a typed
/// reference to `PreparedQuery` — that is the whole point of the lease.
#[must_use]
struct PreparedLease<'a> {
    cell: &'a RefCell<Option<PreparedInner>>,
}

impl Drop for PreparedLease<'_> {
    fn drop(&mut self) {
        if let Ok(inner) = self.cell.try_borrow()
            && let Some(inner) = inner.as_ref()
        {
            inner.in_flight.store(false, Ordering::Release);
        }
    }
}

/// Briefly borrows the handle to refuse re-entry, set `in_flight`, and
/// extract the `UnsafeCell` pointer — then drops the `RefMut` before the
/// caller invokes `SnapWorker::call`.
fn lease_prepared(
    cell: &RefCell<Option<PreparedInner>>,
) -> napi::Result<(LeasedPrepared, PreparedLease<'_>)> {
    let ptr = {
        let inner = live_mut(cell, "prepared query")?;
        if inner.in_flight.swap(true, Ordering::AcqRel) {
            return Err(reentrant_use("prepared query"));
        }
        inner.prepared.get()
    };
    Ok((LeasedPrepared(ptr), PreparedLease { cell }))
}

/// [`take_handle`] for a prepared query: also refuses while `in_flight`.
fn take_prepared(cell: &RefCell<Option<PreparedInner>>) -> napi::Result<PreparedInner> {
    let mut borrowed = cell
        .try_borrow_mut()
        .map_err(|_| reentrant_use("prepared query"))?;
    if borrowed
        .as_ref()
        .is_some_and(|inner| inner.in_flight.load(Ordering::Acquire))
    {
        return Err(reentrant_use("prepared query"));
    }
    borrowed
        .take()
        .ok_or_else(|| closed_handle("prepared query"))
}

/// `dbPrepare`'s domain outcome.
#[expect(
    clippy::large_enum_variant,
    reason = "the outcome is built once, marshaled to JS, and dropped — it \
              never sits in a collection, so boxing would buy one allocation \
              per crossing and nothing else"
)]
pub enum PrepareOutcome {
    Ok(External<PreparedHandle>),
    IrError(String),
}

outcome_to_napi!(PrepareOutcome {
    Ok(handle) => { "ok": true, "prepared": handle },
    IrError(message) => { "ok": false, "kind": "irError", "message": message },
});

/// Prepares a query (IR as plain data, ids only). Roster
/// (validation) errors return as data; statistics-read failures throw.
#[napi]
pub fn db_prepare(db: &External<DbHandle>, query: Object) -> napi::Result<PrepareOutcome> {
    let inner = live(&db.inner, "db")?;
    let query = marshal::query_in(&query)?;
    let engine = Arc::clone(&inner.db);
    let prepared = match engine.prepare(&query) {
        Ok(prepared) => prepared,
        Err(Error::Validation(error)) => return Ok(PrepareOutcome::IrError(error.to_string())),
        Err(error) => return Err(throw_engine(&error)),
    };
    // SAFETY of the lifetime erasure: the prepared query borrows schema and
    // cache data owned by the engine behind `engine` (an `Arc` whose heap
    // address is stable); `PreparedInner` carries that `Arc` and declares
    // `prepared` first, so the borrow always drops before its owner.
    #[expect(
        unsafe_code,
        reason = "the self-referential handle (prepared query + its owning Arc) \
                  needs a lifetime erasure; the SAFETY comment above carries the \
                  drop-order argument"
    )]
    let prepared = unsafe {
        std::mem::transmute::<
            PreparedQuery<'_, SchemaDescriptor>,
            PreparedQuery<'static, SchemaDescriptor>,
        >(prepared)
    };
    Ok(PrepareOutcome::Ok(External::new(PreparedHandle {
        inner: RefCell::new(Some(PreparedInner {
            prepared: UnsafeCell::new(prepared),
            in_flight: AtomicBool::new(false),
            _db: engine,
        })),
    })))
}

/// Executes against a snapshot with positional params (tagged values;
/// `{ kind: "set", values }` binds a param set). The engine's flat
/// `Answers` carrier crosses the worker channel whole and each cell
/// decodes ONCE here (the one-copy crossing); column order = the
/// query's head order; answers are a set — the host sorts.
#[napi]
pub fn prepared_execute(
    prepared: &External<PreparedHandle>,
    snap: &External<SnapshotHandle>,
    params: Array,
) -> napi::Result<Vec<Vec<ValueOut>>> {
    let params = marshal::params_in(&params)?;
    let (prepared_ptr, _lease) = lease_prepared(&prepared.inner)?;
    let worker = live(&snap.inner, "snapshot")?;
    let answers = reply!(
        worker.call(SnapReq::Execute {
            prepared: prepared_ptr,
            params,
        })?,
        SnapReply::Answers,
        "snapshot"
    )?;
    Ok(marshal::answers_out(&answers))
}

/// Plan introspection as data (ruled 2026-07-23, R13): runs the prepared
/// query against the snapshot with counting instrumentation (the engine's
/// `Snapshot::profile`, ANALYZE semantics) and returns the structured
/// stats — plan sections and counters as plain values, the `FjPlan` shape
/// with its numbers. Diagnostic surface, EXPLICITLY UNFROZEN: the shape
/// follows the plan representation wherever it goes. Scalar params only
/// (the engine's profile entry has no param-set spelling).
#[napi]
pub fn prepared_explain(
    prepared: &External<PreparedHandle>,
    snap: &External<SnapshotHandle>,
    params: Array,
) -> napi::Result<ExplainWire> {
    let params = marshal::params_in(&params)?;
    let (prepared_ptr, _lease) = lease_prepared(&prepared.inner)?;
    let worker = live(&snap.inner, "snapshot")?;
    let stats = reply!(
        worker.call(SnapReq::Explain {
            prepared: prepared_ptr,
            params,
        })?,
        SnapReply::Explain,
        "snapshot"
    )?;
    Ok(ExplainWire(stats))
}

/// The pull-based plan-drift signal against a snapshot — engine-policy-free;
/// thresholds are the host's.
#[napi]
pub fn prepared_staleness(
    prepared: &External<PreparedHandle>,
    snap: &External<SnapshotHandle>,
) -> napi::Result<StalenessWire> {
    let (prepared_ptr, _lease) = lease_prepared(&prepared.inner)?;
    let worker = live(&snap.inner, "snapshot")?;
    reply!(
        worker.call(SnapReq::Staleness {
            prepared: prepared_ptr,
        })?,
        SnapReply::Staleness,
        "snapshot"
    )
}

/// Releases the prepared query (its plan, memo, and engine reference).
#[napi]
pub fn prepared_close(prepared: &External<PreparedHandle>) -> napi::Result<()> {
    take_prepared(&prepared.inner)?;
    Ok(())
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
        let error = take_handle(&cell, "snapshot").expect_err("empty handle");
        assert!(
            error.reason.contains("closed snapshot handle"),
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
}
