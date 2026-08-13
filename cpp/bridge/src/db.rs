//! Database ownership and the lexical read/write boundary: the opaque
//! [`bdb_db`] handle, the three store constructors,
//! the fingerprint readback, synchronous callback-scoped snapshots and
//! write transactions, the dynamic fact surface, fresh allocation, bulk
//! import, and the owned [`bdb_row_set`] carrier for scans and point
//! reads.
//!
//! # The nesting SAFETY argument (§18)
//!
//! `bdb_db_write_from` re-enters the engine from inside a read callback.
//! This is sound for exactly one reason: the C callback executes
//! synchronously inside the Rust `Db::read` closure frame on the same
//! thread, so the `&Snapshot` behind [`bdb_snapshot_ref`] is alive for the
//! entire nested call. Both `read` and `write_from` take `&self`, and this
//! nesting is proven in-tree (`bumbledb-bench` witness tests;
//! `bumbledb-query` cookbook). The bridge never STORES the snapshot
//! reference — it only forwards the still-live callback argument — which
//! is what avoids the Node bridge's audited `&'static Snapshot`
//! fabrication (findings 018/021) entirely.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{
    Db, Error, FieldId, RelationId, SchemaDescriptor, Snapshot, StatementId, Value, WriteTx,
};

use crate::error::{bdb_error, bdb_error_kind, fail_engine};
use crate::schema::{bdb_schema_spec, schema_spec_in};
use crate::value::{bdb_string_view, bdb_value, row_in, value_out};
use crate::{
    BridgeResult, Fail, bdb_callback_control, bdb_status, box_in, box_out, guard, out, ref_in,
    slice_in,
};

/// The engine typestate every handle shares: runtime-built schemas all
/// live at `Db<SchemaDescriptor>` (the descriptor implements `Theory` as
/// itself) — the Node bridge's `Engine` alias.
pub(crate) type Engine = Db<SchemaDescriptor>;

/// The opaque database handle: the engine behind an `Arc` (prepared
/// queries co-own it below the boundary — never visible to C++), the
/// admitted descriptor (violation rendering, fingerprint readback), and
/// the bridge-level writer flag (§17: re-entrant writes are refused typed
/// BEFORE the engine's assertion).
pub struct bdb_db {
    pub(crate) db: Arc<Engine>,
    pub(crate) descriptor: SchemaDescriptor,
    in_write: AtomicBool,
}

/// The 64 lowercase hex chars of the store's schema fingerprint — the
/// cross-host identity (NOT NUL-terminated; the width is the type).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_fingerprint {
    pub hex: [u8; 64],
}

/// A borrowed snapshot capability, valid ONLY inside the read callback it
/// was passed to (§16). Never owned by C++, never destroyed by C++; every
/// use re-checks the alive flag the bridge clears when the callback
/// returns, so a stashed ref answers `BDB_STATUS_MISUSE` instead of being
/// replayed.
pub struct bdb_snapshot_ref {
    /// `*const Snapshot<'_, SchemaDescriptor>`, lifetime erased — valid
    /// exactly while `alive` holds (the ref lives on the enclosing
    /// closure's stack frame, so the pointer outlives every legal use).
    snap: *const c_void,
    alive: Cell<bool>,
}

/// A borrowed write-transaction capability, valid ONLY inside the write
/// callback (§17) — the [`bdb_snapshot_ref`] discipline, mutably. Carries
/// its engine pointer so `bdb_tx_alloc` can resolve fresh fields without
/// a second handle argument.
pub struct bdb_tx_ref {
    /// `*mut WriteTx<'_, SchemaDescriptor>`, lifetime erased — valid
    /// exactly while `alive` holds.
    tx: *mut c_void,
    /// `*const Engine` — the same handle's engine, for `fresh_field`.
    db: *const c_void,
    alive: Cell<bool>,
}

/// The owned row carrier for scans and point reads: engine values copied
/// out whole (one crossing), decoded cell by cell C++-side. Views handed
/// out by [`bdb_row_set_get`] borrow this carrier and die with it.
pub struct bdb_row_set {
    rows: Vec<Vec<Value>>,
}

/// One borrowed bulk-import row: `value_count` tagged values in
/// declaration order.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_row_view {
    pub values: *const bdb_value,
    pub value_count: usize,
}

/// The read callback: synchronous, on the calling thread, with a
/// snapshot ref valid only until it returns.
pub type bdb_read_callback = Option<
    unsafe extern "C" fn(context: *mut c_void, snapshot: *const bdb_snapshot_ref)
        -> bdb_callback_control,
>;

/// The write callback: synchronous, on the calling thread, with a tx ref
/// valid only until it returns. `Ok` commits the delta (the engine judges
/// dependencies against the final state); `Abort` drops it — LMDB never
/// saw a fact.
pub type bdb_write_callback = Option<
    unsafe extern "C" fn(context: *mut c_void, transaction: *mut bdb_tx_ref)
        -> bdb_callback_control,
>;

// ---------------------------------------------------------------------------
// Ref plumbing
// ---------------------------------------------------------------------------

impl bdb_snapshot_ref {
    fn new(snap: &Snapshot<'_, SchemaDescriptor>) -> Self {
        Self {
            snap: (&raw const *snap).cast::<c_void>(),
            alive: Cell::new(true),
        }
    }

    /// The borrowed snapshot, alive-checked. The returned lifetime is the
    /// caller's borrow of the ref — legal because a live ref's snapshot
    /// outlives the enclosing callback frame (module doc).
    pub(crate) fn snapshot(&self) -> BridgeResult<&Snapshot<'_, SchemaDescriptor>> {
        if !self.alive.get() {
            return Err(Fail::Misuse);
        }
        #[expect(
            unsafe_code,
            reason = "reborrowing the engine snapshot behind the lifetime-erased \
                      ref pointer; the alive flag plus the lexical-callback \
                      contract carry the argument"
        )]
        // SAFETY: `snap` was minted from a live `&Snapshot` in
        // `bdb_db_read`'s closure frame; `alive` is cleared before that
        // frame returns, so a true flag proves the closure — and therefore
        // the snapshot — is still on the stack of this same thread.
        unsafe {
            Ok(&*self.snap.cast::<Snapshot<'_, SchemaDescriptor>>())
        }
    }

    fn invalidate(&self) {
        self.alive.set(false);
    }
}

impl bdb_tx_ref {
    fn new(tx: &mut WriteTx<'_, SchemaDescriptor>, db: &Engine) -> Self {
        Self {
            tx: (&raw mut *tx).cast::<c_void>(),
            db: (&raw const *db).cast::<c_void>(),
            alive: Cell::new(true),
        }
    }

    /// The borrowed transaction, alive-checked and exclusive: bridge
    /// entries run one at a time on the callback's thread, so no second
    /// `&mut` can exist during the borrow.
    #[expect(
        clippy::mut_from_ref,
        reason = "the FFI reborrow: the mutability is the pointee's (the write \
                  transaction the ref erases), not the ref struct's — the \
                  single-threaded callback protocol makes it exclusive"
    )]
    fn transaction(&self) -> BridgeResult<&mut WriteTx<'_, SchemaDescriptor>> {
        if !self.alive.get() {
            return Err(Fail::Misuse);
        }
        #[expect(
            unsafe_code,
            reason = "reborrowing the engine write transaction behind the \
                      lifetime-erased ref pointer; the alive flag plus the \
                      single-threaded lexical-callback contract carry the \
                      argument"
        )]
        // SAFETY: `tx` was minted from the live `&mut WriteTx` in the
        // write closure frame; `alive` is cleared before that frame
        // returns, and the callback protocol is synchronous single-thread,
        // so this is the only reference for the duration of the entry.
        unsafe {
            Ok(&mut *self.tx.cast::<WriteTx<'_, SchemaDescriptor>>())
        }
    }

    fn engine(&self) -> BridgeResult<&Engine> {
        if !self.alive.get() {
            return Err(Fail::Misuse);
        }
        #[expect(
            unsafe_code,
            reason = "reborrowing the engine handle behind the lifetime-erased ref \
                      pointer; same lexical argument as `transaction`"
        )]
        // SAFETY: `db` points at the `Engine` owned by the `bdb_db` handle
        // that spawned this write; the handle outlives the write call by
        // the header contract (destroying a db during its own callback is
        // caller UB the alive flag cannot see).
        unsafe {
            Ok(&*self.db.cast::<Engine>())
        }
    }

    fn invalidate(&self) {
        self.alive.set(false);
    }
}

/// The one abort sentinel: the error the bridge returns from the engine
/// closure to make it drop the delta when the C callback said `Abort`.
/// Never crosses the boundary — the `aborted` flag beside it decides the
/// status.
fn abort_sentinel() -> Error {
    Error::Io(std::io::Error::other("bumbledb-cpp callback abort"))
}

/// The bridge-level writer guard (§17): set for the duration of
/// `write`/`write_from`/`bulk_load`, cleared on every exit by drop.
struct InWriteReset<'a>(&'a AtomicBool);

impl Drop for InWriteReset<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn enter_write(handle: &bdb_db) -> BridgeResult<InWriteReset<'_>> {
    if handle.in_write.swap(true, Ordering::AcqRel) {
        return Err(Fail::Error(Box::new(bdb_error::synthesized(
            bdb_error_kind::EnvironmentLocked,
            "bumbledb-cpp: re-entrant write on this db handle (the engine is \
             single-writer and non-reentrant; finish the enclosing write first)"
                .to_string(),
        ))));
    }
    Ok(InWriteReset(&handle.in_write))
}

/// Invokes a caller callback — the one C-call site shape.
fn call_read_callback(
    callback: bdb_read_callback,
    context: *mut c_void,
    snapshot: &bdb_snapshot_ref,
) -> BridgeResult<bdb_callback_control> {
    let callback = callback.ok_or(Fail::Misuse)?;
    #[expect(
        unsafe_code,
        reason = "invoking the caller's C function pointer; the header contract \
                  makes a non-null callback a valid function of this exact \
                  signature"
    )]
    // SAFETY: non-null was just checked; the pointer types match the
    // declared ABI signature, and the ref argument is a live stack value.
    unsafe {
        Ok(callback(context, &raw const *snapshot))
    }
}

/// [`call_read_callback`]'s write twin.
fn call_write_callback(
    callback: bdb_write_callback,
    context: *mut c_void,
    transaction: &mut bdb_tx_ref,
) -> BridgeResult<bdb_callback_control> {
    let callback = callback.ok_or(Fail::Misuse)?;
    #[expect(
        unsafe_code,
        reason = "invoking the caller's C function pointer; the header contract \
                  makes a non-null callback a valid function of this exact \
                  signature"
    )]
    // SAFETY: as `call_read_callback`.
    unsafe {
        Ok(callback(context, &raw mut *transaction))
    }
}

// ---------------------------------------------------------------------------
// Store lifecycle
// ---------------------------------------------------------------------------

fn open_with(
    path: bdb_string_view,
    spec: *const bdb_schema_spec,
    out_db: *mut *mut bdb_db,
    open: impl FnOnce(&std::path::Path, SchemaDescriptor) -> bumbledb::Result<Engine>,
) -> BridgeResult<bdb_status> {
    let path = path.as_str("store path")?;
    let spec = schema_spec_in(ref_in(spec)?)?;
    // The canonical lowering: name resolution, canonical-utterance rules,
    // coherence — all the engine's (§13). Every issue rides the message.
    let descriptor = spec.descriptor().map_err(|error| {
        Fail::Error(Box::new(bdb_error::synthesized(
            bdb_error_kind::Schema,
            format!("bumbledb: {error}"),
        )))
    })?;
    let db = open(std::path::Path::new(path), descriptor.clone())
        .map_err(|error| fail_engine(error, Some(&descriptor)))?;
    out(
        out_db,
        box_out(bdb_db {
            db: Arc::new(db),
            descriptor,
            in_write: AtomicBool::new(false),
        }),
    )?;
    Ok(bdb_status::Ok)
}

/// Creates a fresh DURABLE store at `path` from a schema spec. Schema
/// resolution/validation failures are `BDB_ERROR_KIND_SCHEMA`.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_db_create(
    path: bdb_string_view,
    spec: *const bdb_schema_spec,
    out_db: *mut *mut bdb_db,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || open_with(path, spec, out_db, Db::create))
}

/// Opens an existing durable store, verifying format version, store
/// kind, and schema fingerprint (`BDB_ERROR_KIND_SCHEMA_MISMATCH` on drift).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_db_open(
    path: bdb_string_view,
    spec: *const bdb_schema_spec,
    out_db: *mut *mut bdb_db,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || open_with(path, spec, out_db, Db::open))
}

/// Opens or initializes an EPHEMERAL store at `path` (`MDB_NOSYNC`; a
/// machine crash loses the store by the kind's own claim — every other
/// semantic is identical to a durable store).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_db_ephemeral(
    path: bdb_string_view,
    spec: *const bdb_schema_spec,
    out_db: *mut *mut bdb_db,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || open_with(path, spec, out_db, Db::ephemeral))
}

/// Destroys the handle: prepared queries keep their own engine reference
/// (the `Arc` below the boundary), so the environment — and its exclusive
/// lock — releases when the last of them is destroyed.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_db_destroy(db: *mut bdb_db) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        drop(box_in(db)?);
        Ok(bdb_status::Ok)
    })
}

/// The open store's schema fingerprint, 64 lowercase hex chars — the
/// cross-host identity readback (the Node bridge's
/// `dbFingerprint`, verbatim): `create` stored this exact value and
/// `open` verified it, so the descriptor's fingerprint IS the store's.
/// Dumb-bridge legal: validation and blake3 are the ENGINE's own
/// functions re-run on the already-admitted descriptor; the bridge only
/// hex-encodes the 32 bytes.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_db_fingerprint(
    db: *const bdb_db,
    out_fingerprint: *mut bdb_fingerprint,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let handle = ref_in(db)?;
        let schema = handle
            .descriptor
            .clone()
            .validate()
            .map_err(|error| fail_engine(Error::Schema(error), Some(&handle.descriptor)))?;
        let fingerprint = bumbledb::schema::fingerprint::fingerprint(&schema);
        let mut hex = [0u8; 64];
        for (pair, byte) in hex.as_chunks_mut::<2>().0.iter_mut().zip(fingerprint.0) {
            const DIGITS: &[u8; 16] = b"0123456789abcdef";
            pair[0] = DIGITS[usize::from(byte >> 4)];
            pair[1] = DIGITS[usize::from(byte & 0x0f)];
        }
        out(out_fingerprint, bdb_fingerprint { hex })?;
        Ok(bdb_status::Ok)
    })
}

// ---------------------------------------------------------------------------
// Lexical reads and writes
// ---------------------------------------------------------------------------

/// Runs `callback` over one consistent read snapshot (§16): the engine's
/// `Db::read` closure model, synchronous on the calling thread. The
/// snapshot ref is invalidated when the callback returns.
/// `BDB_STATUS_ABORTED` when the callback returned `Abort`.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_db_read(
    db: *const bdb_db,
    callback: bdb_read_callback,
    context: *mut c_void,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let handle = ref_in(db)?;
        let mut aborted = false;
        let mut misuse = false;
        let result = handle.db.read(|snap| {
            let snapshot_ref = bdb_snapshot_ref::new(snap);
            let control = call_read_callback(callback, context, &snapshot_ref);
            snapshot_ref.invalidate();
            match control {
                Ok(bdb_callback_control::Ok) => Ok(()),
                Ok(bdb_callback_control::Abort) => {
                    aborted = true;
                    Err(abort_sentinel())
                }
                Err(_) => {
                    misuse = true;
                    Err(abort_sentinel())
                }
            }
        });
        match result {
            _ if misuse => Err(Fail::Misuse),
            Ok(()) => Ok(bdb_status::Ok),
            Err(_) if aborted => Ok(bdb_status::Aborted),
            Err(error) => Err(fail_engine(error, Some(&handle.descriptor))),
        }
    })
}

/// The one write body under `bdb_db_write` / `bdb_db_write_from`.
fn write_with(
    handle: &bdb_db,
    callback: bdb_write_callback,
    context: *mut c_void,
    run: impl FnOnce(
        &Engine,
        &mut dyn FnMut(&mut WriteTx<'_, SchemaDescriptor>) -> bumbledb::Result<()>,
    ) -> bumbledb::Result<()>,
) -> BridgeResult<bdb_status> {
    let _writer = enter_write(handle)?;
    let mut aborted = false;
    let mut misuse = false;
    let engine = &*handle.db;
    let mut body = |tx: &mut WriteTx<'_, SchemaDescriptor>| -> bumbledb::Result<()> {
        let mut tx_ref = bdb_tx_ref::new(tx, engine);
        let control = call_write_callback(callback, context, &mut tx_ref);
        tx_ref.invalidate();
        match control {
            Ok(bdb_callback_control::Ok) => Ok(()),
            Ok(bdb_callback_control::Abort) => {
                aborted = true;
                Err(abort_sentinel())
            }
            Err(_) => {
                misuse = true;
                Err(abort_sentinel())
            }
        }
    };
    let result = run(engine, &mut body);
    match result {
        _ if misuse => Err(Fail::Misuse),
        Ok(()) => Ok(bdb_status::Ok),
        Err(_) if aborted => Ok(bdb_status::Aborted),
        Err(error) => Err(fail_engine(error, Some(&handle.descriptor))),
    }
}

/// Runs `callback` as the single writer (§17): the engine's `Db::write`
/// closure model. `Ok` from the callback commits — the dependency
/// judgment runs against the final state, and a rejection is
/// `BDB_ERROR_KIND_COMMIT_REJECTED` carrying the complete violation set.
/// `Abort` drops the delta (`BDB_STATUS_ABORTED`; LMDB untouched).
/// Re-entrant writes on this handle are refused with
/// `BDB_ERROR_KIND_ENVIRONMENT_LOCKED` before the engine's assertion.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_db_write(
    db: *const bdb_db,
    callback: bdb_write_callback,
    context: *mut c_void,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let handle = ref_in(db)?;
        write_with(handle, callback, context, |engine, body| {
            engine.write(body)
        })
    })
}

/// `bdb_db_write` conditional on a still-live snapshot (§18): the
/// engine's `Db::write_from`. Callable from inside the read callback that
/// owns `snapshot` (the sanctioned nesting — module doc). A
/// state-changing commit since the snapshot returns
/// `BDB_ERROR_KIND_GENERATION_MOVED` (payload: witnessed/current); retry is
/// host policy.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_db_write_from(
    db: *const bdb_db,
    snapshot: *const bdb_snapshot_ref,
    callback: bdb_write_callback,
    context: *mut c_void,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let handle = ref_in(db)?;
        let snap = ref_in(snapshot)?.snapshot()?;
        write_with(handle, callback, context, |engine, body| {
            engine.write_from(snap, body)
        })
    })
}

// ---------------------------------------------------------------------------
// Transaction operations (write-callback scope)
// ---------------------------------------------------------------------------

/// Records an insert into the delta; `out_changed` = whether the final
/// state changed. Values are the relation's sealed fields in declaration
/// order; shape violations are typed `BDB_ERROR_KIND_FACT_SHAPE` — nothing is
/// judged until commit.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_tx_insert(
    transaction: *const bdb_tx_ref,
    relation: u32,
    values: *const bdb_value,
    value_count: usize,
    out_changed: *mut bool,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let tx_ref = ref_in(transaction)?;
        let row = row_in(values, value_count)?;
        let changed = tx_ref
            .transaction()?
            .insert_dyn(RelationId(relation), &row)
            .map_err(|error| fail_engine(error, None))?;
        out(out_changed, changed)?;
        Ok(bdb_status::Ok)
    })
}

/// Records a delete into the delta; `out_changed` = whether the final
/// state changed.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_tx_delete(
    transaction: *const bdb_tx_ref,
    relation: u32,
    values: *const bdb_value,
    value_count: usize,
    out_changed: *mut bool,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let tx_ref = ref_in(transaction)?;
        let row = row_in(values, value_count)?;
        let changed = tx_ref
            .transaction()?
            .delete_dyn(RelationId(relation), &row)
            .map_err(|error| fail_engine(error, None))?;
        out(out_changed, changed)?;
        Ok(bdb_status::Ok)
    })
}

/// Final-state membership (base + pending delta — the view the commit
/// judgment judges, which is what makes check-then-act race-free).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_tx_contains(
    transaction: *const bdb_tx_ref,
    relation: u32,
    values: *const bdb_value,
    value_count: usize,
    out_contains: *mut bool,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let tx_ref = ref_in(transaction)?;
        let row = row_in(values, value_count)?;
        let contains = tx_ref
            .transaction()?
            .contains_dyn(RelationId(relation), &row)
            .map_err(|error| fail_engine(error, None))?;
        out(out_contains, contains)?;
        Ok(bdb_status::Ok)
    })
}

/// Final-state point lookup through a key statement (`key_values` in the
/// statement's projection order). A hit writes a one-row
/// [`bdb_row_set`] the caller owns; a miss writes null.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_tx_get(
    transaction: *const bdb_tx_ref,
    relation: u32,
    key_statement: u16,
    key_values: *const bdb_value,
    key_value_count: usize,
    out_row: *mut *mut bdb_row_set,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let tx_ref = ref_in(transaction)?;
        let keys = row_in(key_values, key_value_count)?;
        let found = tx_ref
            .transaction()?
            .get_dyn(RelationId(relation), StatementId(key_statement), &keys)
            .map_err(|error| fail_engine(error, None))?;
        out(
            out_row,
            match found {
                Some(values) => box_out(bdb_row_set { rows: vec![values] }),
                None => std::ptr::null_mut(),
            },
        )?;
        Ok(bdb_status::Ok)
    })
}

/// Mints the next fresh value for `(relation, field)` — resolve-once,
/// mint-per-row is the engine's own split (`Db::fresh_field` +
/// `WriteTx::alloc_at`); the bridge re-resolves per call because the C
/// surface carries no witness type (ids at this surface are data; a
/// mis-aimed pair is typed `BDB_ERROR_KIND_FACT_SHAPE`).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_tx_alloc(
    transaction: *const bdb_tx_ref,
    relation: u32,
    field: u16,
    out_id: *mut u64,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let tx_ref = ref_in(transaction)?;
        let fresh = tx_ref
            .engine()?
            .fresh_field(RelationId(relation), FieldId(field))
            .map_err(|error| fail_engine(Error::FactShape(error), None))?;
        let minted = tx_ref
            .transaction()?
            .alloc_at(fresh)
            .map_err(|error| fail_engine(error, None))?;
        out(out_id, minted)?;
        Ok(bdb_status::Ok)
    })
}

// ---------------------------------------------------------------------------
// Snapshot operations (read-callback scope)
// ---------------------------------------------------------------------------

/// Committed-state membership of one dynamic fact (sealed field order).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_snapshot_contains(
    snapshot: *const bdb_snapshot_ref,
    relation: u32,
    values: *const bdb_value,
    value_count: usize,
    out_contains: *mut bool,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let snap = ref_in(snapshot)?.snapshot()?;
        let row = row_in(values, value_count)?;
        let contains = snap
            .contains_dyn(RelationId(relation), &row)
            .map_err(|error| fail_engine(error, None))?;
        out(out_contains, contains)?;
        Ok(bdb_status::Ok)
    })
}

/// Committed-state point lookup of the full fact through a key statement
/// (`key_values` in the statement's projection order). A hit writes a
/// one-row [`bdb_row_set`] the caller owns; a miss writes null.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_snapshot_get(
    snapshot: *const bdb_snapshot_ref,
    relation: u32,
    key_statement: u16,
    key_values: *const bdb_value,
    key_value_count: usize,
    out_row: *mut *mut bdb_row_set,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let snap = ref_in(snapshot)?.snapshot()?;
        let keys = row_in(key_values, key_value_count)?;
        let found = snap
            .get_dyn(RelationId(relation), StatementId(key_statement), &keys)
            .map_err(|error| fail_engine(error, None))?;
        out(
            out_row,
            match found {
                Some(values) => box_out(bdb_row_set { rows: vec![values] }),
                None => std::ptr::null_mut(),
            },
        )?;
        Ok(bdb_status::Ok)
    })
}

/// Full-relation export in `row_id` order (the ETL/derivation read):
/// one owned [`bdb_row_set`] crossing, iterated C++-side — never one FFI
/// call per cell (§37).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_snapshot_scan(
    snapshot: *const bdb_snapshot_ref,
    relation: u32,
    out_rows: *mut *mut bdb_row_set,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let snap = ref_in(snapshot)?.snapshot()?;
        let rows = (|| -> bumbledb::Result<Vec<Vec<Value>>> {
            let iter = snap.scan(RelationId(relation))?;
            let mut rows = Vec::new();
            for row in iter {
                rows.push(row?);
            }
            Ok(rows)
        })()
        .map_err(|error| fail_engine(error, None))?;
        out(out_rows, box_out(bdb_row_set { rows }))?;
        Ok(bdb_status::Ok)
    })
}

// ---------------------------------------------------------------------------
// Bulk import
// ---------------------------------------------------------------------------

/// Bulk import (`Db::bulk_load_dyn`): atomic 4096-row chunks; prior
/// chunks stay committed on failure — `out_committed` always carries the
/// durable count (§24), and a failure is `BDB_ERROR_KIND_BULK_LOAD` (the same
/// count readable via `bdb_error_get_bulk_committed`, the underlying
/// cause in the message). The importer owns dependency ordering: a
/// bidirectional statement cluster must land within one chunk.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_db_bulk_load(
    db: *const bdb_db,
    relation: u32,
    rows: *const bdb_row_view,
    row_count: usize,
    out_committed: *mut u64,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let handle = ref_in(db)?;
        // Copied whole before the engine runs: a marshal refusal is a
        // clean typed error with ZERO facts durable, never a mid-import
        // surprise.
        let facts = slice_in(rows, row_count)?
            .iter()
            .map(|row| row_in(row.values, row.value_count))
            .collect::<BridgeResult<Vec<_>>>()?;
        let _writer = enter_write(handle)?;
        match handle.db.bulk_load_dyn(RelationId(relation), facts) {
            Ok(total) => {
                out(out_committed, total)?;
                Ok(bdb_status::Ok)
            }
            Err(bulk) => {
                out(out_committed, bulk.committed)?;
                Err(fail_engine(
                    Error::BulkLoad {
                        committed: bulk.committed,
                        error: Box::new(bulk.error),
                    },
                    Some(&handle.descriptor),
                ))
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Row sets
// ---------------------------------------------------------------------------

/// Number of rows.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_row_set_len(rows: *const bdb_row_set) -> usize {
    match ref_in(rows) {
        Ok(rows) => rows.rows.len(),
        Err(_) => 0,
    }
}

/// The row's cell count (sealed field order — every row of one scan has
/// the relation's arity).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_row_set_arity(rows: *const bdb_row_set, row: usize) -> usize {
    match ref_in(rows) {
        Ok(rows) => rows.rows.get(row).map_or(0, Vec::len),
        Err(_) => 0,
    }
}

/// One cell, viewed — string/bytes payloads BORROW the row set and die
/// with it. Bounds-checked: `BDB_STATUS_MISUSE` out of range.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_row_set_get(
    rows: *const bdb_row_set,
    row: usize,
    column: usize,
    out_value: *mut bdb_value,
) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        let rows = ref_in(rows)?;
        let value = rows
            .rows
            .get(row)
            .and_then(|row| row.get(column))
            .ok_or(Fail::Misuse)?;
        out(out_value, value_out(value))?;
        Ok(bdb_status::Ok)
    })
}

/// Frees a row set (invalidating every view borrowed from it).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_row_set_destroy(rows: *mut bdb_row_set) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        drop(box_in(rows)?);
        Ok(bdb_status::Ok)
    })
}

#[cfg(test)]
pub(crate) fn test_only_trigger_panic(out_error: *mut *mut bdb_error) -> bdb_status {
    guard(out_error, || panic!("bumbledb-cpp test panic hook"))
}
