//! Database ownership, heap construction, and the lexical read/write
//! boundary: opaque handles, tagged admissions, per-callback instance
//! refs, and cloneable witnesses.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::sync::{Mutex, PoisonError};

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{
    Admission, ConditionalWrite, Db, Error, FieldId, Instance, InstanceBuilder, OwnedInstance,
    ReadInstance, RelationId, SchemaDescriptor, StatementId, Value, Witness, WriteTx,
};

use crate::error::{
    bdb_error, bdb_violations, fail_busy, fail_engine, fail_schema_message,
};
use crate::query::{bdb_prepared, bdb_query, query_in};
use crate::schema::{bdb_schema_spec, schema_spec_in};
use crate::value::{bdb_string_view, bdb_value, row_in, rows_in, value_out};
use crate::{
    BridgeResult, Fail, bdb_callback_control, bdb_status, box_in, box_out, box_out_to, guard,
    guard_statusless, guard_value, mut_in, out, ref_in, require_out, tag_in,
};

pub(crate) type Engine = Db<SchemaDescriptor>;

const READERS_MASK: u32 = 0x0000_FFFF;
const WRITING: u32 = 1 << 16;
const WRITING_BUSY: u32 = 1 << 17;

const KIND_STORE: u8 = 1;
const KIND_HEAP: u8 = 2;

/// Bridge owner identity — compared before execute so a foreign prepared
/// never reaches the engine. Pointers are never dereferenced after mint.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerToken {
    Store(*const Engine),
    Heap(*const OwnedInstance<SchemaDescriptor>),
}

/// Opaque database handle.
pub struct bdb_db {
    pub(crate) db: Arc<Engine>,
    pub(crate) descriptor: SchemaDescriptor,
    phase: AtomicU32,
    retired: Mutex<Vec<Retired>>,
}

#[allow(dead_code, reason = "boxes are held so stashed C pointers stay allocated")]
enum Retired {
    Instance(Box<bdb_instance_ref>),
    Witness(Box<bdb_witness>),
    Tx(Box<bdb_tx_ref>),
}

/// Opaque heap builder. Spent by [`bdb_instance_builder_admit`].
pub struct bdb_instance_builder {
    builder: InstanceBuilder<SchemaDescriptor>,
    descriptor: SchemaDescriptor,
}

/// Opaque admitted heap instance.
pub struct bdb_owned_instance {
    instance: OwnedInstance<SchemaDescriptor>,
    descriptor: SchemaDescriptor,
}

/// Borrowed query surface, valid only during the callback that minted it.
pub struct bdb_instance_ref {
    kind: u8,
    ptr: AtomicPtr<c_void>,
    engine: Option<Arc<Engine>>,
    owner: OwnerToken,
    alive: AtomicBool,
}

/// Generation witness: cloneable evidence. A callback argument is borrowed
/// and invalidated when the callback returns. [`bdb_witness_retain`] clones
/// an owning handle.
pub struct bdb_witness {
    value: Witness<SchemaDescriptor>,
    owner: *const Engine,
    alive: AtomicBool,
    retained: bool,
}

/// Borrowed write-transaction capability, valid only inside the write
/// callback.
pub struct bdb_tx_ref {
    tx: AtomicPtr<c_void>,
    db: AtomicPtr<c_void>,
    alive: AtomicBool,
    in_op: AtomicBool,
}

/// 64 lowercase hex chars of the store's schema fingerprint.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_fingerprint {
    pub hex: [u8; 64],
}

/// Facts consumed vs facts that changed the in-memory final-state view.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_mutation_report {
    pub submitted: u64,
    pub changed: u64,
}

/// Tagged fresh-id range. Empty is the tag, never `{0, 0}`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_fresh_range_tag {
    Empty = 0,
    NonEmpty = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_fresh_range {
    pub tag: bdb_fresh_range_tag,
    pub start: u64,
    pub end_exclusive: u64,
}

/// Admission discriminant. Zero is the documented empty/uninitialized
/// state and is never returned with `BDB_STATUS_OK`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_admission_tag {
    Empty = 0,
    Accepted = 1,
    Rejected = 2,
    Moved = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_moved_generations {
    pub witnessed: u64,
    pub current: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union bdb_instance_admission_value {
    pub accepted: *mut bdb_owned_instance,
    pub rejected: *mut bdb_violations,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_instance_admission {
    pub tag: bdb_admission_tag,
    pub value: bdb_instance_admission_value,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union bdb_db_admission_value {
    pub accepted: *mut bdb_db,
    pub rejected: *mut bdb_violations,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_db_admission {
    pub tag: bdb_admission_tag,
    pub value: bdb_db_admission_value,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union bdb_write_admission_value {
    pub accepted_generation: u64,
    pub rejected: *mut bdb_violations,
    pub moved: bdb_moved_generations,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_write_admission {
    pub tag: bdb_admission_tag,
    pub value: bdb_write_admission_value,
}

/// Owned row carrier for scans and point reads.
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

/// Store-read callback: instance + borrowed witness.
pub type bdb_db_read_callback = Option<
    unsafe extern "C" fn(
        context: *mut c_void,
        instance: *const bdb_instance_ref,
        witness: *const bdb_witness,
    ) -> u32,
>;

/// Heap-instance callback: the same query surface, no witness.
pub type bdb_owned_instance_read_callback = Option<
    unsafe extern "C" fn(context: *mut c_void, instance: *const bdb_instance_ref) -> u32,
>;

pub type bdb_write_callback =
    Option<unsafe extern "C" fn(context: *mut c_void, transaction: *mut bdb_tx_ref) -> u32>;

fn fresh_range_wire(range: bumbledb::FreshRange<u64>) -> bdb_fresh_range {
    match range {
        bumbledb::FreshRange::Empty => bdb_fresh_range {
            tag: bdb_fresh_range_tag::Empty,
            start: 0,
            end_exclusive: 0,
        },
        bumbledb::FreshRange::NonEmpty { start, count } => bdb_fresh_range {
            tag: bdb_fresh_range_tag::NonEmpty,
            start,
            end_exclusive: start + count.get(),
        },
    }
}

fn empty_db_admission() -> bdb_db_admission {
    bdb_db_admission {
        tag: bdb_admission_tag::Empty,
        value: bdb_db_admission_value {
            accepted: std::ptr::null_mut(),
        },
    }
}

fn empty_instance_admission() -> bdb_instance_admission {
    bdb_instance_admission {
        tag: bdb_admission_tag::Empty,
        value: bdb_instance_admission_value {
            accepted: std::ptr::null_mut(),
        },
    }
}

fn empty_write_admission() -> bdb_write_admission {
    bdb_write_admission {
        tag: bdb_admission_tag::Empty,
        value: bdb_write_admission_value {
            accepted_generation: 0,
        },
    }
}

fn assemble(db: Engine, descriptor: SchemaDescriptor) -> bdb_db {
    bdb_db {
        db: Arc::new(db),
        descriptor,
        phase: AtomicU32::new(0),
        retired: Mutex::new(Vec::new()),
    }
}

fn descriptor_of(spec: *const bdb_schema_spec) -> BridgeResult<SchemaDescriptor> {
    let spec = schema_spec_in(ref_in(spec)?)?;
    spec.descriptor()
        .map_err(|error| fail_schema_message(&error.to_string()))
}

fn hex_fingerprint(descriptor: &SchemaDescriptor) -> BridgeResult<bdb_fingerprint> {
    let schema = descriptor
        .clone()
        .validate()
        .map_err(|error| fail_engine(Error::Schema(error)))?;
    let fingerprint = bumbledb::schema::fingerprint::fingerprint(&schema);
    let mut hex = [0u8; 64];
    for (pair, byte) in hex.as_chunks_mut::<2>().0.iter_mut().zip(fingerprint.0) {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        pair[0] = DIGITS[usize::from(byte >> 4)];
        pair[1] = DIGITS[usize::from(byte & 0x0f)];
    }
    Ok(bdb_fingerprint { hex })
}

fn retire(handle: &bdb_db, slot: Retired) {
    handle
        .retired
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(slot);
}

fn leak_retired(slot: Retired) {
    match slot {
        Retired::Instance(slot) => {
            let _ = Box::leak(slot);
        }
        Retired::Witness(slot) => {
            let _ = Box::leak(slot);
        }
        Retired::Tx(slot) => {
            let _ = Box::leak(slot);
        }
    }
}

fn phase_readers(phase: u32) -> u32 {
    phase & READERS_MASK
}

fn phase_writing(phase: u32) -> bool {
    phase & WRITING != 0
}

struct ReadGuard<'a>(&'a AtomicU32);

impl Drop for ReadGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Release);
    }
}

fn enter_read(phase: &AtomicU32) -> BridgeResult<ReadGuard<'_>> {
    loop {
        let cur = phase.load(Ordering::Acquire);
        if phase_writing(cur) {
            return Err(fail_busy(
                "read while a write callback is live on this db handle",
            ));
        }
        let readers = phase_readers(cur);
        let next = readers
            .checked_add(1)
            .filter(|n| *n <= READERS_MASK)
            .ok_or_else(|| fail_busy("reader count overflow on this db handle"))?;
        if phase
            .compare_exchange_weak(cur, next, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            return Ok(ReadGuard(phase));
        }
    }
}

struct WriteGuard<'a>(&'a AtomicU32);

impl Drop for WriteGuard<'_> {
    fn drop(&mut self) {
        self.0
            .fetch_and(!(WRITING | WRITING_BUSY), Ordering::Release);
    }
}

fn enter_write(phase: &AtomicU32) -> BridgeResult<WriteGuard<'_>> {
    loop {
        let cur = phase.load(Ordering::Acquire);
        if phase_writing(cur) {
            return Err(fail_busy(
                "re-entrant write on this db handle (the engine is \
                 single-writer and non-reentrant; finish the enclosing write first)",
            ));
        }
        if phase
            .compare_exchange_weak(cur, cur | WRITING, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            return Ok(WriteGuard(phase));
        }
    }
}

fn handle_busy(handle: &bdb_db) -> bool {
    let phase = handle.phase.load(Ordering::Acquire);
    phase_readers(phase) != 0 || phase_writing(phase)
}

fn callback_interrupt() -> Error {
    Error::from(std::io::Error::from(std::io::ErrorKind::Interrupted))
}

fn is_callback_interrupt(error: &Error) -> bool {
    matches!(error, Error::Io(failure) if failure.kind == std::io::ErrorKind::Interrupted)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Exit {
    Proceed,
    Abort,
    Misuse,
}

struct InOpReset<'a>(&'a AtomicBool);

impl Drop for InOpReset<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl bdb_instance_ref {
    pub(crate) fn owner(&self) -> OwnerToken {
        self.owner
    }

    fn store(engine: Arc<Engine>, snap: &ReadInstance<'_, SchemaDescriptor>) -> Self {
        let owner = OwnerToken::Store(Arc::as_ptr(&engine));
        Self {
            kind: KIND_STORE,
            ptr: AtomicPtr::new((&raw const *snap).cast::<c_void>().cast_mut()),
            engine: Some(engine),
            owner,
            alive: AtomicBool::new(true),
        }
    }

    fn heap(instance: &OwnedInstance<SchemaDescriptor>) -> Self {
        Self {
            kind: KIND_HEAP,
            ptr: AtomicPtr::new((&raw const *instance).cast::<c_void>().cast_mut()),
            engine: None,
            owner: OwnerToken::Heap(std::ptr::from_ref(instance)),
            alive: AtomicBool::new(true),
        }
    }

    fn invalidate(&self) {
        self.alive.store(false, Ordering::Release);
        self.ptr.store(std::ptr::null_mut(), Ordering::Release);
    }

    fn require_live(&self) -> BridgeResult<()> {
        if self.alive.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err(Fail::Misuse)
        }
    }

    fn with_store<R>(
        &self,
        body: impl FnOnce(&ReadInstance<'_, SchemaDescriptor>) -> BridgeResult<R>,
    ) -> BridgeResult<R> {
        self.require_live()?;
        if self.kind != KIND_STORE {
            return Err(Fail::Misuse);
        }
        let ptr = self.ptr.load(Ordering::Acquire);
        if ptr.is_null() {
            return Err(Fail::Misuse);
        }
        #[expect(
            unsafe_code,
            reason = "reborrowing the engine ReadInstance behind the lifetime-erased ref"
        )]
        // SAFETY: minted from a live `&ReadInstance` in the callback frame;
        // `alive` is cleared and the pointer nulled before that frame returns.
        unsafe {
            body(&*ptr.cast::<ReadInstance<'_, SchemaDescriptor>>())
        }
    }

    fn with_heap<R>(
        &self,
        body: impl FnOnce(&OwnedInstance<SchemaDescriptor>) -> BridgeResult<R>,
    ) -> BridgeResult<R> {
        self.require_live()?;
        if self.kind != KIND_HEAP {
            return Err(Fail::Misuse);
        }
        let ptr = self.ptr.load(Ordering::Acquire);
        if ptr.is_null() {
            return Err(Fail::Misuse);
        }
        #[expect(
            unsafe_code,
            reason = "reborrowing the OwnedInstance behind the lifetime-erased ref"
        )]
        // SAFETY: minted from a live `&OwnedInstance` in `bdb_owned_instance_read`;
        // `alive` is cleared before that call returns. The owned handle outlives
        // the callback (C cannot destroy it during the callback without
        // racing the same thread).
        unsafe {
            body(&*ptr.cast::<OwnedInstance<SchemaDescriptor>>())
        }
    }

    fn contains_dyn(&self, relation: RelationId, row: &[Value]) -> BridgeResult<bool> {
        match self.kind {
            KIND_STORE => self.with_store(|snap| snap.contains_dyn(relation, row).map_err(fail_engine)),
            KIND_HEAP => self.with_heap(|inst| inst.contains_dyn(relation, row).map_err(fail_engine)),
            _ => Err(Fail::Misuse),
        }
    }

    fn get_dyn(
        &self,
        relation: RelationId,
        key: StatementId,
        keys: &[Value],
    ) -> BridgeResult<Option<Vec<Value>>> {
        match self.kind {
            KIND_STORE => self.with_store(|snap| {
                snap.get_dyn(relation, key, keys).map_err(fail_engine)
            }),
            KIND_HEAP => self.with_heap(|inst| {
                inst.get_dyn(relation, key, keys).map_err(fail_engine)
            }),
            _ => Err(Fail::Misuse),
        }
    }

    fn scan(&self, relation: RelationId) -> BridgeResult<Vec<Vec<Value>>> {
        match self.kind {
            KIND_STORE => self.with_store(|snap| collect_scan(snap.scan(relation))),
            KIND_HEAP => self.with_heap(|inst| collect_scan(inst.scan(relation))),
            _ => Err(Fail::Misuse),
        }
    }

    pub(crate) fn execute(
        &self,
        prepared: &mut bumbledb::PreparedQuery<SchemaDescriptor>,
        params: &[bumbledb::ParamArg<'_>],
        answers: &mut bumbledb::Answers,
    ) -> BridgeResult<()> {
        match self.kind {
            KIND_STORE => self.with_store(|snap| {
                Instance::execute(snap, prepared, params, answers).map_err(fail_engine)
            }),
            KIND_HEAP => self.with_heap(|inst| {
                Instance::execute(inst, prepared, params, answers).map_err(fail_engine)
            }),
            _ => Err(Fail::Misuse),
        }
    }

    fn prepare(&self, query: &bumbledb::Query) -> BridgeResult<bumbledb::PreparedQuery<SchemaDescriptor>> {
        match self.kind {
            KIND_STORE => self.with_store(|snap| snap.prepare(query).map_err(fail_engine)),
            KIND_HEAP => self.with_heap(|inst| inst.prepare(query).map_err(fail_engine)),
            _ => Err(Fail::Misuse),
        }
    }

    fn row_count(&self, relation: RelationId) -> BridgeResult<u64> {
        match self.kind {
            KIND_STORE => self.with_store(|snap| snap.row_count(relation).map_err(fail_engine)),
            KIND_HEAP => self.with_heap(|inst| inst.row_count(relation).map_err(fail_engine)),
            _ => Err(Fail::Misuse),
        }
    }
}

fn collect_scan(
    iter: bumbledb::Result<impl Iterator<Item = bumbledb::Result<Vec<Value>>>>,
) -> BridgeResult<Vec<Vec<Value>>> {
    let iter = iter.map_err(fail_engine)?;
    let mut rows = Vec::new();
    for row in iter {
        rows.push(row.map_err(fail_engine)?);
    }
    Ok(rows)
}

impl bdb_witness {
    fn borrowed_from(db: &bdb_db, value: Witness<SchemaDescriptor>) -> Self {
        Self {
            value,
            owner: Arc::as_ptr(&db.db),
            alive: AtomicBool::new(true),
            retained: false,
        }
    }

    fn retained_from(src: &Self) -> Self {
        Self {
            value: src.value.clone(),
            owner: src.owner,
            alive: AtomicBool::new(true),
            retained: true,
        }
    }

    fn invalidate(&self) {
        self.alive.store(false, Ordering::Release);
    }

    fn live_value(&self) -> BridgeResult<&Witness<SchemaDescriptor>> {
        if self.alive.load(Ordering::Acquire) {
            Ok(&self.value)
        } else {
            Err(Fail::Misuse)
        }
    }
}

impl bdb_tx_ref {
    fn mint(tx: &mut WriteTx<'_, SchemaDescriptor>, db: &Engine) -> Self {
        Self {
            tx: AtomicPtr::new((&raw mut *tx).cast::<c_void>()),
            db: AtomicPtr::new((&raw const *db).cast::<c_void>().cast_mut()),
            alive: AtomicBool::new(true),
            in_op: AtomicBool::new(false),
        }
    }

    fn invalidate(&self) {
        self.alive.store(false, Ordering::Release);
        self.tx.store(std::ptr::null_mut(), Ordering::Release);
        self.db.store(std::ptr::null_mut(), Ordering::Release);
    }

    fn enter_op(&self) -> BridgeResult<InOpReset<'_>> {
        if !self.alive.load(Ordering::Acquire) {
            return Err(Fail::Misuse);
        }
        if self.in_op.swap(true, Ordering::AcqRel) {
            return Err(Fail::Misuse);
        }
        Ok(InOpReset(&self.in_op))
    }

    #[expect(
        clippy::mut_from_ref,
        reason = "the FFI reborrow: mutability is the pointee's; `in_op` is exclusive"
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
            reason = "reborrowing the engine write transaction behind the lifetime-erased ref"
        )]
        // SAFETY: minted from the live `&mut WriteTx` in the write closure;
        // `alive` is cleared before that frame returns. `enter_op` won exclusive.
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
            reason = "reborrowing the engine handle behind the lifetime-erased ref"
        )]
        // SAFETY: `db` points at the `Engine` owned by the `bdb_db` that spawned
        // this write; destroy during the callback is refused.
        unsafe {
            Ok(&*ptr.cast::<Engine>())
        }
    }
}

fn call_read_callback(
    callback: bdb_db_read_callback,
    context: *mut c_void,
    instance: &bdb_instance_ref,
    witness: &bdb_witness,
) -> BridgeResult<bdb_callback_control> {
    let callback = callback.ok_or(Fail::Misuse)?;
    #[expect(
        unsafe_code,
        reason = "invoking the caller's extern C function"
    )]
    // SAFETY: non-null was just checked; the ref arguments are heap slots
    // live for the call. A C++ throw through `callback` is unsupported.
    let raw = unsafe { callback(context, instance, witness) };
    tag_in(raw)
}

fn call_owned_callback(
    callback: bdb_owned_instance_read_callback,
    context: *mut c_void,
    instance: &bdb_instance_ref,
) -> BridgeResult<bdb_callback_control> {
    let callback = callback.ok_or(Fail::Misuse)?;
    #[expect(
        unsafe_code,
        reason = "invoking the caller's extern C function"
    )]
    let raw = unsafe { callback(context, instance) };
    tag_in(raw)
}

fn call_write_callback(
    callback: bdb_write_callback,
    context: *mut c_void,
    transaction: &bdb_tx_ref,
) -> BridgeResult<bdb_callback_control> {
    let callback = callback.ok_or(Fail::Misuse)?;
    #[expect(
        unsafe_code,
        reason = "invoking the caller's extern C function"
    )]
    let raw = unsafe { callback(context, (&raw const *transaction).cast_mut()) };
    tag_in(raw)
}

fn exit_of(control: &BridgeResult<bdb_callback_control>) -> Exit {
    match control {
        Ok(bdb_callback_control::Ok) => Exit::Proceed,
        Ok(bdb_callback_control::Abort) => Exit::Abort,
        Err(_) => Exit::Misuse,
    }
}

// ---------------------------------------------------------------------------
// Store lifecycle
// ---------------------------------------------------------------------------

/// Creates a fresh DURABLE store. Empty that does not hold is
/// `BDB_ADMISSION_REJECTED` with no directory. `BDB_STATUS_OK` always
/// fills `out_admission` (never the empty tag).
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_db_create(
    path: bdb_string_view,
    spec: *const bdb_schema_spec,
    out_admission: *mut bdb_db_admission,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_admission)?;
        out(out_admission, empty_db_admission())?;
        let path = path.as_str("store path")?;
        let descriptor = descriptor_of(spec)?;
        match Db::create(std::path::Path::new(path), descriptor.clone()).map_err(fail_engine)? {
            Admission::Accepted(db) => {
                out(
                    out_admission,
                    bdb_db_admission {
                        tag: bdb_admission_tag::Accepted,
                        value: bdb_db_admission_value {
                            accepted: box_out(assemble(db, descriptor)),
                        },
                    },
                )?;
            }
            Admission::Rejected(violations) => {
                out(
                    out_admission,
                    bdb_db_admission {
                        tag: bdb_admission_tag::Rejected,
                        value: bdb_db_admission_value {
                            rejected: box_out(bdb_violations::from_engine(
                                &violations,
                                &descriptor,
                            )),
                        },
                    },
                )?;
            }
        }
        Ok(bdb_status::Ok)
    })
}

/// Opens an existing durable store. No admission union — format-8 open
/// carries admission provenance.
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
    guard(out_error, || {
        require_out(out_db)?;
        let path = path.as_str("store path")?;
        let descriptor = descriptor_of(spec)?;
        let db = Db::open(std::path::Path::new(path), descriptor.clone()).map_err(fail_engine)?;
        box_out_to(out_db, assemble(db, descriptor))?;
        Ok(bdb_status::Ok)
    })
}

/// Opens or initializes an EPHEMERAL store. Fresh initialize and wipe
/// complete-admit empty; an existing admitted format-8 store reopens as
/// accepted.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_db_ephemeral(
    path: bdb_string_view,
    spec: *const bdb_schema_spec,
    out_admission: *mut bdb_db_admission,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_admission)?;
        out(out_admission, empty_db_admission())?;
        let path = path.as_str("store path")?;
        let descriptor = descriptor_of(spec)?;
        match Db::ephemeral(std::path::Path::new(path), descriptor.clone()).map_err(fail_engine)? {
            Admission::Accepted(db) => {
                out(
                    out_admission,
                    bdb_db_admission {
                        tag: bdb_admission_tag::Accepted,
                        value: bdb_db_admission_value {
                            accepted: box_out(assemble(db, descriptor)),
                        },
                    },
                )?;
            }
            Admission::Rejected(violations) => {
                out(
                    out_admission,
                    bdb_db_admission {
                        tag: bdb_admission_tag::Rejected,
                        value: bdb_db_admission_value {
                            rejected: box_out(bdb_violations::from_engine(
                                &violations,
                                &descriptor,
                            )),
                        },
                    },
                )?;
            }
        }
        Ok(bdb_status::Ok)
    })
}

/// Raw-copies an admitted heap instance into a new durable store.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_db_from_instance(
    path: bdb_string_view,
    instance: *const bdb_owned_instance,
    out_db: *mut *mut bdb_db,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_db)?;
        let path = path.as_str("store path")?;
        let instance = ref_in(instance)?;
        let db = Db::from_instance(std::path::Path::new(path), &instance.instance)
            .map_err(fail_engine)?;
        box_out_to(out_db, assemble(db, instance.descriptor.clone()))?;
        Ok(bdb_status::Ok)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_db_destroy(db: *mut bdb_db) -> bdb_status {
    guard_statusless(|| {
        let handle = ref_in(db)?;
        if handle_busy(handle) {
            return Err(Fail::Misuse);
        }
        let handle = box_in(db)?;
        let retired = std::mem::take(
            &mut *handle
                .retired
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        drop(handle);
        for slot in retired {
            leak_retired(slot);
        }
        Ok(bdb_status::Ok)
    })
}

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
        out(out_fingerprint, hex_fingerprint(&handle.descriptor)?)?;
        Ok(bdb_status::Ok)
    })
}

// ---------------------------------------------------------------------------
// Builder / owned instance
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_instance_builder_new(
    spec: *const bdb_schema_spec,
    out_builder: *mut *mut bdb_instance_builder,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_builder)?;
        let descriptor = descriptor_of(spec)?;
        let builder =
            InstanceBuilder::new(descriptor.clone()).map_err(fail_engine)?;
        box_out_to(
            out_builder,
            bdb_instance_builder {
                builder,
                descriptor,
            },
        )?;
        Ok(bdb_status::Ok)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_instance_builder_load(
    builder: *mut bdb_instance_builder,
    relation: u32,
    values: *const bdb_value,
    value_count: usize,
    row_count: usize,
    out_report: *mut bdb_mutation_report,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_report)?;
        let rows = rows_in(values, value_count, row_count)?;
        let report = mut_in(builder)?
            .builder
            .load_dyn(RelationId(relation), rows)
            .map_err(fail_engine)?;
        out(
            out_report,
            bdb_mutation_report {
                submitted: report.submitted(),
                changed: report.changed(),
            },
        )?;
        Ok(bdb_status::Ok)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_instance_builder_delete(
    builder: *mut bdb_instance_builder,
    relation: u32,
    values: *const bdb_value,
    value_count: usize,
    row_count: usize,
    out_report: *mut bdb_mutation_report,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_report)?;
        let rows = rows_in(values, value_count, row_count)?;
        let report = mut_in(builder)?
            .builder
            .delete_dyn(RelationId(relation), rows)
            .map_err(fail_engine)?;
        out(
            out_report,
            bdb_mutation_report {
                submitted: report.submitted(),
                changed: report.changed(),
            },
        )?;
        Ok(bdb_status::Ok)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_instance_builder_reserve(
    builder: *mut bdb_instance_builder,
    relation: u32,
    field: u16,
    count: u64,
    out_range: *mut bdb_fresh_range,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_range)?;
        let builder = &mut mut_in(builder)?.builder;
        let fresh = builder
            .fresh_field(RelationId(relation), FieldId(field))
            .map_err(|error| fail_engine(Error::FactShape(error)))?;
        let range = builder.reserve_at(fresh, count).map_err(fail_engine)?;
        out(out_range, fresh_range_wire(range))?;
        Ok(bdb_status::Ok)
    })
}

/// Consumes the builder on every outcome and nulls the caller's pointer.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_instance_builder_admit(
    builder: *mut *mut bdb_instance_builder,
    out_admission: *mut bdb_instance_admission,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_admission)?;
        out(out_admission, empty_instance_admission())?;
        let slot = mut_in(builder)?;
        let handle = box_in(*slot)?;
        *slot = std::ptr::null_mut();
        match handle.builder.admit().map_err(fail_engine)? {
            Admission::Accepted(instance) => {
                out(
                    out_admission,
                    bdb_instance_admission {
                        tag: bdb_admission_tag::Accepted,
                        value: bdb_instance_admission_value {
                            accepted: box_out(bdb_owned_instance {
                                instance,
                                descriptor: handle.descriptor,
                            }),
                        },
                    },
                )?;
            }
            Admission::Rejected(violations) => {
                out(
                    out_admission,
                    bdb_instance_admission {
                        tag: bdb_admission_tag::Rejected,
                        value: bdb_instance_admission_value {
                            rejected: box_out(bdb_violations::from_engine(
                                &violations,
                                &handle.descriptor,
                            )),
                        },
                    },
                )?;
            }
        }
        Ok(bdb_status::Ok)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_instance_builder_destroy(
    builder: *mut bdb_instance_builder,
) -> bdb_status {
    guard_statusless(|| {
        drop(box_in(builder)?);
        Ok(bdb_status::Ok)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_owned_instance_destroy(
    instance: *mut bdb_owned_instance,
) -> bdb_status {
    guard_statusless(|| {
        drop(box_in(instance)?);
        Ok(bdb_status::Ok)
    })
}

/// Borrows an owned instance through the common [`bdb_instance_ref`]
/// query surface.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_owned_instance_read(
    instance: *const bdb_owned_instance,
    callback: bdb_owned_instance_read_callback,
    context: *mut c_void,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let owned = ref_in(instance)?;
        let instance_ref = Box::new(bdb_instance_ref::heap(&owned.instance));
        let exit = exit_of(&call_owned_callback(callback, context, &instance_ref));
        instance_ref.invalidate();
        // The owned handle outlives this call; the ref is not stashed on a
        // db, so keep it alive for stashed-pointer MISUSE by leaking.
        let _ = Box::leak(instance_ref);
        match exit {
            Exit::Proceed => Ok(bdb_status::Ok),
            Exit::Abort => Ok(bdb_status::Aborted),
            Exit::Misuse => Err(Fail::Misuse),
        }
    })
}

// ---------------------------------------------------------------------------
// Lexical reads and writes
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_db_read(
    db: *const bdb_db,
    callback: bdb_db_read_callback,
    context: *mut c_void,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let handle = ref_in(db)?;
        let _reader = enter_read(&handle.phase)?;
        let exit = Cell::new(Exit::Proceed);
        let engine = Arc::clone(&handle.db);
        let result = engine.read(|snap| {
            let witness_value = snap.witness()?;
            let instance_ref = Box::new(bdb_instance_ref::store(Arc::clone(&handle.db), snap));
            let witness = Box::new(bdb_witness::borrowed_from(handle, witness_value));
            exit.set(exit_of(&call_read_callback(
                callback,
                context,
                &instance_ref,
                &witness,
            )));
            instance_ref.invalidate();
            witness.invalidate();
            retire(handle, Retired::Instance(instance_ref));
            retire(handle, Retired::Witness(witness));
            match exit.get() {
                Exit::Proceed => Ok(()),
                Exit::Abort | Exit::Misuse => Err(callback_interrupt()),
            }
        });
        match (exit.get(), result) {
            (Exit::Misuse, _) => Err(Fail::Misuse),
            (Exit::Abort, Ok(())) => Ok(bdb_status::Aborted),
            (Exit::Abort, Err(error)) if is_callback_interrupt(&error) => {
                Ok(bdb_status::Aborted)
            }
            (Exit::Abort | Exit::Proceed, Err(error)) => Err(fail_engine(error)),
            (Exit::Proceed, Ok(())) => Ok(bdb_status::Ok),
        }
    })
}

fn write_outcome(
    handle: &bdb_db,
    callback: bdb_write_callback,
    context: *mut c_void,
    run: impl FnOnce(
        &Engine,
        &mut dyn FnMut(&mut WriteTx<'_, SchemaDescriptor>) -> bumbledb::Result<()>,
    ) -> bumbledb::Result<ConditionalWrite<()>>,
) -> BridgeResult<(bdb_status, Option<bdb_write_admission>)> {
    let _writer = enter_write(&handle.phase)?;
    let exit = Cell::new(Exit::Proceed);
    let engine = Arc::clone(&handle.db);
    let mut body = |tx: &mut WriteTx<'_, SchemaDescriptor>| -> bumbledb::Result<()> {
        let tx_ref = Box::new(bdb_tx_ref::mint(tx, engine.as_ref()));
        exit.set(exit_of(&call_write_callback(callback, context, &tx_ref)));
        tx_ref.invalidate();
        retire(handle, Retired::Tx(tx_ref));
        match exit.get() {
            Exit::Proceed => Ok(()),
            Exit::Abort | Exit::Misuse => Err(callback_interrupt()),
        }
    };
    let result = run(engine.as_ref(), &mut body);
    match (exit.get(), result) {
        (Exit::Misuse, _) => Err(Fail::Misuse),
        (Exit::Abort, Ok(_)) => Ok((bdb_status::Aborted, None)),
        (Exit::Abort, Err(error)) if is_callback_interrupt(&error) => {
            Ok((bdb_status::Aborted, None))
        }
        (Exit::Abort | Exit::Proceed, Err(error)) => Err(fail_engine(error)),
        (Exit::Proceed, Ok(ConditionalWrite::Accepted(committed))) => Ok((
            bdb_status::Ok,
            Some(bdb_write_admission {
                tag: bdb_admission_tag::Accepted,
                value: bdb_write_admission_value {
                    accepted_generation: committed.generation.value(),
                },
            }),
        )),
        (Exit::Proceed, Ok(ConditionalWrite::Rejected(violations))) => Ok((
            bdb_status::Ok,
            Some(bdb_write_admission {
                tag: bdb_admission_tag::Rejected,
                value: bdb_write_admission_value {
                    rejected: box_out(bdb_violations::from_engine(
                        &violations,
                        &handle.descriptor,
                    )),
                },
            }),
        )),
        (Exit::Proceed, Ok(ConditionalWrite::Moved { witnessed, current })) => Ok((
            bdb_status::Ok,
            Some(bdb_write_admission {
                tag: bdb_admission_tag::Moved,
                value: bdb_write_admission_value {
                    moved: bdb_moved_generations {
                        witnessed: witnessed.value(),
                        current: current.value(),
                    },
                },
            }),
        )),
    }
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_db_write(
    db: *const bdb_db,
    callback: bdb_write_callback,
    context: *mut c_void,
    out_admission: *mut bdb_write_admission,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_admission)?;
        out(out_admission, empty_write_admission())?;
        let handle = ref_in(db)?;
        let (status, admission) = write_outcome(handle, callback, context, |engine, body| {
            Ok(match engine.write(body)? {
                Admission::Accepted(committed) => ConditionalWrite::Accepted(committed),
                Admission::Rejected(violations) => ConditionalWrite::Rejected(violations),
            })
        })?;
        if let Some(admission) = admission {
            debug_assert_ne!(admission.tag, bdb_admission_tag::Empty);
            out(out_admission, admission)?;
        }
        Ok(status)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_db_write_from(
    db: *const bdb_db,
    witness: *const bdb_witness,
    callback: bdb_write_callback,
    context: *mut c_void,
    out_admission: *mut bdb_write_admission,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_admission)?;
        out(out_admission, empty_write_admission())?;
        let handle = ref_in(db)?;
        let witness = ref_in(witness)?.live_value()?;
        let (status, admission) = write_outcome(handle, callback, context, |engine, body| {
            engine.write_from(witness, body)
        })?;
        if let Some(admission) = admission {
            debug_assert_ne!(admission.tag, bdb_admission_tag::Empty);
            out(out_admission, admission)?;
        }
        Ok(status)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_witness_retain(
    witness: *const bdb_witness,
    out_witness: *mut *mut bdb_witness,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_witness)?;
        let src = ref_in(witness)?;
        let _ = src.live_value()?;
        box_out_to(out_witness, bdb_witness::retained_from(src))?;
        Ok(bdb_status::Ok)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_witness_destroy(witness: *mut bdb_witness) -> bdb_status {
    guard_statusless(|| {
        let handle = ref_in(witness)?;
        if !handle.retained {
            return Err(Fail::Misuse);
        }
        drop(box_in(witness)?);
        Ok(bdb_status::Ok)
    })
}

// ---------------------------------------------------------------------------
// Transaction operations
// ---------------------------------------------------------------------------

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
            .map_err(fail_engine)?;
        out(
            out_report,
            bdb_mutation_report {
                submitted: report.submitted(),
                changed: report.changed(),
            },
        )?;
        Ok(bdb_status::Ok)
    })
}

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
            .map_err(fail_engine)?;
        out(
            out_report,
            bdb_mutation_report {
                submitted: report.submitted(),
                changed: report.changed(),
            },
        )?;
        Ok(bdb_status::Ok)
    })
}

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
    out_contains: *mut u8,
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
            .map_err(fail_engine)?;
        out(out_contains, u8::from(contains))?;
        Ok(bdb_status::Ok)
    })
}

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
            .map_err(fail_engine)?;
        match found {
            Some(values) => box_out_to(out_row, bdb_row_set::from_rows(vec![values]))?,
            None => out(out_row, std::ptr::null_mut())?,
        }
        Ok(bdb_status::Ok)
    })
}

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
            .map_err(|error| fail_engine(Error::FactShape(error)))?;
        let range = tx_ref
            .transaction()?
            .reserve_at(fresh, count)
            .map_err(fail_engine)?;
        out(out_range, fresh_range_wire(range))?;
        Ok(bdb_status::Ok)
    })
}

// ---------------------------------------------------------------------------
// Instance operations
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_instance_contains(
    instance: *const bdb_instance_ref,
    relation: u32,
    values: *const bdb_value,
    value_count: usize,
    out_contains: *mut u8,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_contains)?;
        let row = row_in(values, value_count)?;
        let contains = ref_in(instance)?.contains_dyn(RelationId(relation), &row)?;
        out(out_contains, u8::from(contains))?;
        Ok(bdb_status::Ok)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_instance_get(
    instance: *const bdb_instance_ref,
    relation: u32,
    key_statement: u16,
    key_values: *const bdb_value,
    key_value_count: usize,
    out_row: *mut *mut bdb_row_set,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_row)?;
        let keys = row_in(key_values, key_value_count)?;
        let found = ref_in(instance)?.get_dyn(
            RelationId(relation),
            StatementId(key_statement),
            &keys,
        )?;
        match found {
            Some(values) => box_out_to(out_row, bdb_row_set::from_rows(vec![values]))?,
            None => out(out_row, std::ptr::null_mut())?,
        }
        Ok(bdb_status::Ok)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_instance_scan(
    instance: *const bdb_instance_ref,
    relation: u32,
    out_rows: *mut *mut bdb_row_set,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_rows)?;
        let rows = ref_in(instance)?.scan(RelationId(relation))?;
        box_out_to(out_rows, bdb_row_set::from_rows(rows))?;
        Ok(bdb_status::Ok)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_instance_row_count(
    instance: *const bdb_instance_ref,
    relation: u32,
    out_count: *mut u64,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_count)?;
        let count = ref_in(instance)?.row_count(RelationId(relation))?;
        out(out_count, count)?;
        Ok(bdb_status::Ok)
    })
}

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_instance_prepare(
    instance: *const bdb_instance_ref,
    query: *const bdb_query,
    out_prepared: *mut *mut bdb_prepared,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_prepared)?;
        let instance = ref_in(instance)?;
        let query = query_in(ref_in(query)?)?;
        let prepared = instance.prepare(&query)?;
        box_out_to(
            out_prepared,
            bdb_prepared {
                prepared,
                owner: instance.owner(),
                _keep: instance.engine.clone(),
                in_execute: AtomicBool::new(false),
            },
        )?;
        Ok(bdb_status::Ok)
    })
}

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

#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_row_set_destroy(rows: *mut bdb_row_set) -> bdb_status {
    guard_statusless(|| {
        drop(box_in(rows)?);
        Ok(bdb_status::Ok)
    })
}

#[cfg(test)]
pub(crate) fn test_only_trigger_panic(out_error: *mut *mut bdb_error) -> bdb_status {
    guard(out_error, || panic!("bumbledb-c test panic hook"))
}
