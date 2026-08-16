//! Database ownership and the lexical read/write boundary: the opaque
//! [`bdb_db`] handle, the three store constructors,
//! the fingerprint readback, synchronous callback-scoped snapshots and
//! write transactions, the dynamic fact surface, fresh reservation, and
//! the owned [`bdb_row_set`] carrier for scans and point reads.
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

use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{
    Db, Error, FieldId, RelationId, SchemaDescriptor, Snapshot, StatementId, Value, WriteTx,
};

use crate::error::{bdb_error, bdb_error_kind, fail_engine, fail_locked};
use crate::schema::{bdb_schema_spec, schema_spec_in};
use crate::value::{bdb_string_view, bdb_value, row_in, rows_in, value_out};
use crate::{
    BridgeResult, Fail, bdb_callback_control, bdb_status, box_in, box_out_to, guard, guard_value,
    out, ref_in, require_out, tag_in,
};

/// The engine typestate every handle shares: runtime-built schemas all
/// live at `Db<SchemaDescriptor>` (the descriptor implements `Theory` as
/// itself) — the Node bridge's `Engine` alias.
pub(crate) type Engine = Db<SchemaDescriptor>;

/// The opaque database handle: the engine behind an `Arc` (prepared
/// queries co-own it below the boundary — never visible to C), the
/// admitted descriptor (violation rendering, fingerprint readback), the
/// bridge-level writer/reader flags, and the heap slots that give
/// snapshot/tx refs a stable address for as long as this `Box` lives.
pub struct bdb_db {
    pub(crate) db: Arc<Engine>,
    pub(crate) descriptor: SchemaDescriptor,
    in_write: AtomicBool,
    in_read: AtomicBool,
    snapshot_slot: bdb_snapshot_ref,
    tx_slot: bdb_tx_ref,
}

/// The 64 lowercase hex chars of the store's schema fingerprint — the
/// cross-host identity (NOT NUL-terminated; the width is the type).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_fingerprint {
    pub hex: [u8; 64],
}

/// A borrowed snapshot capability, valid ONLY inside the read callback it
/// was passed to (§16). The struct lives in a heap slot on [`bdb_db`] so
/// a stashed C pointer remains a real object after the callback: every
/// use re-checks `alive`, and a stale ref answers `BDB_STATUS_MISUSE`
/// instead of being replayed or use-after-freeing a stack frame.
pub struct bdb_snapshot_ref {
    /// `*const Snapshot<'_, SchemaDescriptor>`, lifetime erased — valid
    /// exactly while `alive` holds. Nulled on invalidate.
    snap: AtomicPtr<c_void>,
    alive: AtomicBool,
}

/// A borrowed write-transaction capability, valid ONLY inside the write
/// callback (§17) — the [`bdb_snapshot_ref`] discipline, mutably. Carries
/// its engine pointer so `bdb_tx_reserve` can resolve fresh fields without
/// a second handle argument. `in_op` makes `transaction()` exclusive
/// across threads for the duration of one `bdb_tx_*` entry.
pub struct bdb_tx_ref {
    /// `*mut WriteTx<'_, SchemaDescriptor>`, lifetime erased — valid
    /// exactly while `alive` holds. Nulled on invalidate.
    tx: AtomicPtr<c_void>,
    /// `*const Engine` — the same handle's engine, for `fresh_field`.
    db: AtomicPtr<c_void>,
    alive: AtomicBool,
    in_op: AtomicBool,
}

/// The owned row carrier for scans and point reads: engine values copied
/// out whole (one crossing), decoded cell by cell on the host. Views handed
/// out by [`bdb_row_set_get`] borrow this carrier and die with it.
/// Arity is a property of the set (the relation's width), not of a row
/// index — inbound collections are already rectangular.
pub struct bdb_row_set {
    rows: Vec<Vec<Value>>,
    arity: usize,
}

impl bdb_row_set {
    fn from_rows(rows: Vec<Vec<Value>>) -> Self {
        let arity = rows.first().map_or(0, Vec::len);
        Self { rows, arity }
    }
}

/// Facts consumed vs facts that changed the in-memory final-state view.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_mutation_report {
    pub submitted: u64,
    pub changed: u64,
}

/// Wire encoding of a fresh-id range from one `bdb_tx_reserve`.
/// Empty is `{ start: 0, end_exclusive: 0 }` **at this boundary only** —
/// `start` is not a minted id when `start == end_exclusive`. Hosts must
/// not treat 0 as minted on empty (0 is also the first legal minted id).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_fresh_range {
    pub start: u64,
    pub end_exclusive: u64,
}

/// Engine [`bumbledb::FreshRange`] → C wire. Empty stays `{0, 0}` here;
/// that encoding is not the engine type. `start` is a minted id only on
/// the nonempty arm.
fn fresh_range_wire(range: bumbledb::FreshRange<u64>) -> bdb_fresh_range {
    match range {
        bumbledb::FreshRange::Empty => bdb_fresh_range {
            start: 0,
            end_exclusive: 0,
        },
        bumbledb::FreshRange::NonEmpty { start, count } => bdb_fresh_range {
            start,
            end_exclusive: start + count.get(),
        },
    }
}

/// The read callback: synchronous, on the calling thread, with a
/// snapshot ref valid only until it returns. The return is an integer
/// tag (`bdb_callback_control`); unknown values are `BDB_STATUS_MISUSE`.
/// Invoked directly from Rust. A C++ exception through this function is
/// unsupported.
pub type bdb_read_callback =
    Option<unsafe extern "C" fn(context: *mut c_void, snapshot: *const bdb_snapshot_ref) -> u32>;

/// The write callback: synchronous, on the calling thread, with a tx ref
/// valid only until it returns. `Ok` commits the delta (the engine judges
/// dependencies against the final state); `Abort` drops it — LMDB never
/// saw a fact. Direct invoke as [`bdb_read_callback`].
pub type bdb_write_callback =
    Option<unsafe extern "C" fn(context: *mut c_void, transaction: *mut bdb_tx_ref) -> u32>;

// ---------------------------------------------------------------------------
// Ref plumbing
// ---------------------------------------------------------------------------

impl bdb_snapshot_ref {
    fn empty() -> Self {
        Self {
            snap: AtomicPtr::new(std::ptr::null_mut()),
            alive: AtomicBool::new(false),
        }
    }

    fn mint(&self, snap: &Snapshot<'_, SchemaDescriptor>) {
        self.snap.store(
            (&raw const *snap).cast::<c_void>().cast_mut(),
            Ordering::Relaxed,
        );
        self.alive.store(true, Ordering::Release);
    }

    /// The borrowed snapshot, alive-checked. The returned lifetime is the
    /// caller's borrow of the ref — legal because a live ref's snapshot
    /// outlives the enclosing callback frame (module doc), and the ref
    /// itself lives in the `bdb_db` heap slot until destroy.
    pub(crate) fn snapshot(&self) -> BridgeResult<&Snapshot<'_, SchemaDescriptor>> {
        if !self.alive.load(Ordering::Acquire) {
            return Err(Fail::Misuse);
        }
        let ptr = self.snap.load(Ordering::Acquire);
        if ptr.is_null() {
            return Err(Fail::Misuse);
        }
        #[expect(
            unsafe_code,
            reason = "reborrowing the engine snapshot behind the lifetime-erased \
                      ref pointer; the alive flag plus the lexical-callback \
                      contract carry the argument"
        )]
        // SAFETY: `snap` was minted from a live `&Snapshot` in
        // `bdb_db_read`'s closure frame into this heap slot; `alive` is
        // cleared and the pointer nulled before that frame returns, so a
        // true flag proves the closure — and therefore the snapshot — is
        // still on the stack of this same thread. The slot itself outlives
        // the callback (it lives in `bdb_db`).
        unsafe {
            Ok(&*ptr.cast::<Snapshot<'_, SchemaDescriptor>>())
        }
    }

    fn invalidate(&self) {
        self.alive.store(false, Ordering::Release);
        self.snap.store(std::ptr::null_mut(), Ordering::Release);
    }
}

/// Clears the snapshot slot on every exit from the read closure,
/// including a panic inside the C callback.
struct InvalidateSnapshot<'a>(&'a bdb_snapshot_ref);

impl Drop for InvalidateSnapshot<'_> {
    fn drop(&mut self) {
        self.0.invalidate();
    }
}

struct InOpReset<'a>(&'a AtomicBool);

impl Drop for InOpReset<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl bdb_tx_ref {
    fn empty() -> Self {
        Self {
            tx: AtomicPtr::new(std::ptr::null_mut()),
            db: AtomicPtr::new(std::ptr::null_mut()),
            alive: AtomicBool::new(false),
            in_op: AtomicBool::new(false),
        }
    }

    fn mint(&self, tx: &mut WriteTx<'_, SchemaDescriptor>, db: &Engine) {
        self.tx
            .store((&raw mut *tx).cast::<c_void>(), Ordering::Relaxed);
        self.db.store(
            (&raw const *db).cast::<c_void>().cast_mut(),
            Ordering::Relaxed,
        );
        self.alive.store(true, Ordering::Release);
    }

    /// Exclusive claim for one `bdb_tx_*` entry: two threads cannot both
    /// `transaction()` at once. Sequential same-thread calls are fine
    /// (the previous entry's `_op` drops before the next).
    fn enter_op(&self) -> BridgeResult<InOpReset<'_>> {
        if !self.alive.load(Ordering::Acquire) {
            return Err(Fail::Misuse);
        }
        if self.in_op.swap(true, Ordering::AcqRel) {
            return Err(Fail::Misuse);
        }
        Ok(InOpReset(&self.in_op))
    }

    /// The borrowed transaction, alive-checked. Caller must hold
    /// [`Self::enter_op`]'s guard so this `&mut` is exclusive.
    #[expect(
        clippy::mut_from_ref,
        reason = "the FFI reborrow: the mutability is the pointee's (the write \
                  transaction the ref erases), not the ref struct's — `in_op` \
                  makes it exclusive for the duration of the entry"
    )]
    fn transaction(&self) -> BridgeResult<&mut WriteTx<'_, SchemaDescriptor>> {
        if !self.alive.load(Ordering::Acquire) {
            return Err(Fail::Misuse);
        }
        let ptr = self.tx.load(Ordering::Acquire);
        if ptr.is_null() {
            return Err(Fail::Misuse);
        }
        #[expect(
            unsafe_code,
            reason = "reborrowing the engine write transaction behind the \
                      lifetime-erased ref pointer; the alive flag plus `in_op` \
                      exclusive carry the argument"
        )]
        // SAFETY: `tx` was minted from the live `&mut WriteTx` in the
        // write closure frame into this heap slot; `alive` is cleared and
        // the pointer nulled before that frame returns. `enter_op` won
        // the exclusive, so this is the only `&mut WriteTx` for the
        // duration of the entry (including across threads that captured
        // the C pointer).
        unsafe {
            Ok(&mut *ptr.cast::<WriteTx<'_, SchemaDescriptor>>())
        }
    }

    fn engine(&self) -> BridgeResult<&Engine> {
        if !self.alive.load(Ordering::Acquire) {
            return Err(Fail::Misuse);
        }
        let ptr = self.db.load(Ordering::Acquire);
        if ptr.is_null() {
            return Err(Fail::Misuse);
        }
        #[expect(
            unsafe_code,
            reason = "reborrowing the engine handle behind the lifetime-erased ref \
                      pointer; same lexical argument as `transaction`"
        )]
        // SAFETY: `db` points at the `Engine` owned by the `bdb_db` handle
        // that spawned this write; destroy of that handle during its own
        // callback is refused (typed), so the engine outlives the write
        // call.
        unsafe {
            Ok(&*ptr.cast::<Engine>())
        }
    }

    fn invalidate(&self) {
        self.alive.store(false, Ordering::Release);
        self.tx.store(std::ptr::null_mut(), Ordering::Release);
        self.db.store(std::ptr::null_mut(), Ordering::Release);
    }
}

/// Clears the tx slot on every exit from the write closure.
struct InvalidateTx<'a>(&'a bdb_tx_ref);

impl Drop for InvalidateTx<'_> {
    fn drop(&mut self) {
        self.0.invalidate();
    }
}

/// The one abort sentinel: the error the bridge returns from the engine
/// closure to make it drop the delta when the C callback said `Abort`.
/// Never crosses the boundary — the `aborted` flag beside it decides the
/// status.
fn abort_sentinel() -> Error {
    Error::Io(std::io::Error::other("bumbledb-c callback abort"))
}

/// The bridge-level writer guard (§17): set for the duration of
/// `write`/`write_from`, cleared on every exit by drop.
struct InWriteReset<'a>(&'a AtomicBool);

impl Drop for InWriteReset<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn enter_write(handle: &bdb_db) -> BridgeResult<InWriteReset<'_>> {
    if handle.in_write.swap(true, Ordering::AcqRel) {
        return Err(fail_locked(
            "re-entrant write on this db handle (the engine is \
             single-writer and non-reentrant; finish the enclosing write first)",
        ));
    }
    Ok(InWriteReset(&handle.in_write))
}

struct InReadReset<'a>(&'a AtomicBool);

impl Drop for InReadReset<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn enter_read(handle: &bdb_db) -> BridgeResult<InReadReset<'_>> {
    if handle.in_write.load(Ordering::Acquire) {
        return Err(fail_locked(
            "re-entrant read on this db handle (a write callback is live; \
             finish it first)",
        ));
    }
    if handle.in_read.swap(true, Ordering::AcqRel) {
        return Err(fail_locked(
            "re-entrant read on this db handle (one live read callback per \
             handle — the snapshot slot is exclusive)",
        ));
    }
    Ok(InReadReset(&handle.in_read))
}

fn handle_in_callback(handle: &bdb_db) -> bool {
    handle.in_read.load(Ordering::Acquire) || handle.in_write.load(Ordering::Acquire)
}

/// Invokes a caller callback directly, then range-checks the integer
/// control tag. No C++ trampoline: the library links with no C++ runtime.
fn call_read_callback(
    callback: bdb_read_callback,
    context: *mut c_void,
    snapshot: &bdb_snapshot_ref,
) -> BridgeResult<bdb_callback_control> {
    let callback = callback.ok_or(Fail::Misuse)?;
    #[expect(
        unsafe_code,
        reason = "invoking the caller's extern C function; the header contract \
                  makes a non-null callback a valid function of this exact \
                  signature"
    )]
    // SAFETY: non-null was just checked; the ref argument is the db's
    // heap slot, live for the call. A C++ throw through `callback` is
    // unsupported (76-c-abi.md).
    let raw = unsafe { callback(context, snapshot) };
    tag_in(raw)
}

/// [`call_read_callback`]'s write twin.
fn call_write_callback(
    callback: bdb_write_callback,
    context: *mut c_void,
    transaction: &bdb_tx_ref,
) -> BridgeResult<bdb_callback_control> {
    let callback = callback.ok_or(Fail::Misuse)?;
    #[expect(
        unsafe_code,
        reason = "invoking the caller's extern C function; the header contract \
                  makes a non-null callback a valid function of this exact \
                  signature"
    )]
    // SAFETY: as `call_read_callback`.
    let raw = unsafe { callback(context, (&raw const *transaction).cast_mut()) };
    tag_in(raw)
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
    require_out(out_db)?;
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
    box_out_to(
        out_db,
        bdb_db {
            db: Arc::new(db),
            descriptor,
            in_write: AtomicBool::new(false),
            in_read: AtomicBool::new(false),
            snapshot_slot: bdb_snapshot_ref::empty(),
            tx_slot: bdb_tx_ref::empty(),
        },
    )?;
    Ok(bdb_status::Ok)
}

/// Creates a fresh DURABLE store at `path` from a schema spec. Schema
/// resolution/validation failures are `BDB_ERROR_KIND_SCHEMA`.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
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
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
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
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
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
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_db_destroy(db: *mut bdb_db) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        let handle = ref_in(db)?;
        if handle_in_callback(handle) {
            return Err(Fail::Misuse);
        }
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
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
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
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_db_read(
    db: *const bdb_db,
    callback: bdb_read_callback,
    context: *mut c_void,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let handle = ref_in(db)?;
        let _reader = enter_read(handle)?;
        let mut aborted = false;
        let mut misuse = false;
        let result = handle.db.read(|snap| {
            handle.snapshot_slot.mint(snap);
            let _slot = InvalidateSnapshot(&handle.snapshot_slot);
            let control = call_read_callback(callback, context, &handle.snapshot_slot);
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
        handle.tx_slot.mint(tx, engine);
        let _slot = InvalidateTx(&handle.tx_slot);
        let control = call_write_callback(callback, context, &handle.tx_slot);
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
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_db_write(
    db: *const bdb_db,
    callback: bdb_write_callback,
    context: *mut c_void,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let handle = ref_in(db)?;
        write_with(handle, callback, context, |engine, body| engine.write(body))
    })
}

/// `bdb_db_write` conditional on a still-live snapshot (§18): the
/// engine's `Db::write_from`. Callable from inside the read callback that
/// owns `snapshot` (the sanctioned nesting — module doc). A
/// state-changing commit since the snapshot returns
/// `BDB_ERROR_KIND_GENERATION_MOVED` (payload: witnessed/current); retry is
/// host policy.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
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

/// Records a collection of inserts into the delta. `values` is
/// `row_count × value_count` cells in row-major order (`value_count` is
/// the relation arity). `row_count == 0` is lawful and does not read
/// `values`. Shape violations are typed `BDB_ERROR_KIND_FACT_SHAPE` —
/// the whole collection is checked before any row enters the delta.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_tx_insert(
    transaction: *const bdb_tx_ref,
    relation: u32,
    values: *const bdb_value,
    value_count: usize,
    row_count: usize,
    out_report: *mut bdb_mutation_report,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let tx_ref = ref_in(transaction)?;
        require_out(out_report)?;
        let _op = tx_ref.enter_op()?;
        let rows = rows_in(values, value_count, row_count)?;
        let report = tx_ref
            .transaction()?
            .insert_dyn(RelationId(relation), rows)
            .map_err(|error| fail_engine(error, None))?;
        out(
            out_report,
            bdb_mutation_report {
                submitted: report.submitted,
                changed: report.changed,
            },
        )?;
        Ok(bdb_status::Ok)
    })
}

/// Records a collection of deletes into the delta. Layout as
/// [`bdb_tx_insert`].
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_tx_delete(
    transaction: *const bdb_tx_ref,
    relation: u32,
    values: *const bdb_value,
    value_count: usize,
    row_count: usize,
    out_report: *mut bdb_mutation_report,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let tx_ref = ref_in(transaction)?;
        require_out(out_report)?;
        let _op = tx_ref.enter_op()?;
        let rows = rows_in(values, value_count, row_count)?;
        let report = tx_ref
            .transaction()?
            .delete_dyn(RelationId(relation), rows)
            .map_err(|error| fail_engine(error, None))?;
        out(
            out_report,
            bdb_mutation_report {
                submitted: report.submitted,
                changed: report.changed,
            },
        )?;
        Ok(bdb_status::Ok)
    })
}

/// Final-state membership (base + pending delta — the view the commit
/// judgment judges, which is what makes check-then-act race-free).
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
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
        require_out(out_contains)?;
        let _op = tx_ref.enter_op()?;
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
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
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
        require_out(out_row)?;
        let _op = tx_ref.enter_op()?;
        let keys = row_in(key_values, key_value_count)?;
        let found = tx_ref
            .transaction()?
            .get_dyn(RelationId(relation), StatementId(key_statement), &keys)
            .map_err(|error| fail_engine(error, None))?;
        match found {
            Some(values) => box_out_to(out_row, bdb_row_set::from_rows(vec![values]))?,
            None => out(out_row, std::ptr::null_mut())?,
        }
        Ok(bdb_status::Ok)
    })
}

/// Mints `count` consecutive fresh values for `(relation, field)`.
/// `count == 0` is the empty wire range `{0, 0}` and does not read or
/// advance the sequence — `start` is not a minted id on empty. The bridge
/// re-resolves the field per call because the C surface carries no witness
/// type (ids at this surface are data; a mis-aimed pair is typed
/// `BDB_ERROR_KIND_FACT_SHAPE`).
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_tx_reserve(
    transaction: *const bdb_tx_ref,
    relation: u32,
    field: u16,
    count: u64,
    out_range: *mut bdb_fresh_range,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let tx_ref = ref_in(transaction)?;
        require_out(out_range)?;
        let _op = tx_ref.enter_op()?;
        let fresh = tx_ref
            .engine()?
            .fresh_field(RelationId(relation), FieldId(field))
            .map_err(|error| fail_engine(Error::FactShape(error), None))?;
        let range = tx_ref
            .transaction()?
            .reserve_at(fresh, count)
            .map_err(|error| fail_engine(error, None))?;
        out(out_range, fresh_range_wire(range))?;
        Ok(bdb_status::Ok)
    })
}

// ---------------------------------------------------------------------------
// Snapshot operations (read-callback scope)
// ---------------------------------------------------------------------------

/// Committed-state membership of one dynamic fact (sealed field order).
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
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
        require_out(out_contains)?;
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
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
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
        require_out(out_row)?;
        let keys = row_in(key_values, key_value_count)?;
        let found = snap
            .get_dyn(RelationId(relation), StatementId(key_statement), &keys)
            .map_err(|error| fail_engine(error, None))?;
        match found {
            Some(values) => box_out_to(out_row, bdb_row_set::from_rows(vec![values]))?,
            None => out(out_row, std::ptr::null_mut())?,
        }
        Ok(bdb_status::Ok)
    })
}

/// Full-relation export in `row_id` order (the ETL/derivation read):
/// one owned [`bdb_row_set`] crossing, iterated host-side — never one FFI
/// call per cell (§37).
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_snapshot_scan(
    snapshot: *const bdb_snapshot_ref,
    relation: u32,
    out_rows: *mut *mut bdb_row_set,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let snap = ref_in(snapshot)?.snapshot()?;
        require_out(out_rows)?;
        let rows = (|| -> bumbledb::Result<Vec<Vec<Value>>> {
            let iter = snap.scan(RelationId(relation))?;
            let mut rows = Vec::new();
            for row in iter {
                rows.push(row?);
            }
            Ok(rows)
        })()
        .map_err(|error| fail_engine(error, None))?;
        box_out_to(out_rows, bdb_row_set::from_rows(rows))?;
        Ok(bdb_status::Ok)
    })
}

/// Number of rows.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_row_set_len(rows: *const bdb_row_set) -> usize {
    guard_value(0, || match ref_in(rows) {
        Ok(rows) => rows.rows.len(),
        Err(_) => 0,
    })
}

/// The set's cell count (sealed field order — one width for the
/// relation, not per row). Empty sets answer 0.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_row_set_arity(rows: *const bdb_row_set) -> usize {
    guard_value(0, || match ref_in(rows) {
        Ok(rows) => rows.arity,
        Err(_) => 0,
    })
}

/// One cell, viewed — string/bytes payloads BORROW the row set and die
/// with it. Bounds-checked: `BDB_STATUS_MISUSE` out of range.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
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
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_row_set_destroy(rows: *mut bdb_row_set) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        drop(box_in(rows)?);
        Ok(bdb_status::Ok)
    })
}

#[cfg(test)]
pub(crate) fn test_only_trigger_panic(out_error: *mut *mut bdb_error) -> bdb_status {
    guard(out_error, || panic!("bumbledb-c test panic hook"))
}
