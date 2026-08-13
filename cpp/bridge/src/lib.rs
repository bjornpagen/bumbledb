//! The C-ABI bridge for the C++26 SDK.
//!
//! The dumb-bridge law (the ts/crate precedent): no logic beyond marshaling
//! will EVER live in this crate. No schema knowledge beyond schema-DIRECTED
//! rendering of rejections, no validation, no name resolution, no retries,
//! no logging — anything smart belongs in the C++ SDK or in the bumbledb
//! engine itself. This crate exists only to carry values across the C
//! boundary.
//!
//! # The boundary protocol
//!
//! Every fallible export returns a [`bdb_status`] and takes a trailing
//! `*mut *mut bdb_error` out-param:
//!
//! - `BDB_STATUS_OK`: success; no error is written.
//! - `BDB_STATUS_ABORTED`: the caller's own callback returned
//!   `BDB_CALLBACK_CONTROL_ABORT` — domain abandonment, not a failure; no
//!   error is written and nothing committed.
//! - `BDB_STATUS_ERROR`: an engine or marshal failure; a [`bdb_error`] is
//!   written to the out-param (when it is non-null) and the caller owns it
//!   (`bdb_error_destroy`).
//! - `BDB_STATUS_MISUSE`: a contract violation the bridge could detect —
//!   a null required pointer, a stale snapshot/tx ref used after its
//!   callback returned, an out-of-range answers index, an unknown enum tag.
//!   No error is allocated: misuse is a programming error, not data.
//!
//! # The lexical model
//!
//! Snapshots and write transactions are LEXICAL borrowed capabilities:
//! [`db::bdb_snapshot_ref`] / [`db::bdb_tx_ref`] are stack values passed by
//! pointer into the caller's synchronous callback and invalidated the
//! moment the callback returns. They are never owned by C, never destroyed
//! by C, and every use re-checks the alive flag (returning
//! `BDB_STATUS_MISUSE` on a stale ref). `bdb_db_write_from` may be called
//! from inside a read callback with that callback's still-live snapshot
//! ref — the one sanctioned nesting (§18).
//!
//! # Panic policy
//!
//! A Rust panic unwinding across the C boundary into `-fno-exceptions` C++
//! is undefined behavior, so EVERY extern entry point routes through
//! [`guard`]: `std::panic::catch_unwind` maps a caught panic to
//! `BDB_ERROR_KIND_PANIC` (the caller treats the store as poisoned). Unwinding
//! stays inside Rust, so the engine's own drop guards (the escaped-fresh-id
//! burn on write failure) run as designed. Re-entrant
//! `write`/`write_from`/`bulk_load` are refused bridge-side with a typed
//! `BDB_ERROR_KIND_ENVIRONMENT_LOCKED` error BEFORE the engine's non-reentrancy
//! assertion can fire.
//!
//! # Safety shape
//!
//! Raw-pointer handling is concentrated in the small helper set below
//! ([`ref_in`], [`mut_in`], [`slice_in`], [`out`], [`box_in`]) plus the
//! per-module view readers; each helper's SAFETY argument is the generated
//! header's contract (pointers come from the constructors this header
//! names, views outlive the call they are passed to). The exported
//! functions themselves are spelled as safe `extern "C" fn`s whose whole
//! body rides those audited helpers — the ts/crate carve-out regime.

// C ABI type names ARE the header contract: the Rust spelling and the C
// spelling are one identifier, so grep works across the boundary and
// cbindgen needs no rename table.
#![allow(non_camel_case_types)]

pub mod answers;
pub mod db;
pub mod error;
pub mod query;
pub mod schema;
pub mod value;

#[cfg(test)]
mod tests;

use std::panic::AssertUnwindSafe;

use error::bdb_error;

/// The status every fallible export returns (module doc: the boundary
/// protocol).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_status {
    Ok = 0,
    Error = 1,
    Aborted = 2,
    Misuse = 3,
}

/// A callback's control return: `Ok` commits (write) / completes (read);
/// `Abort` abandons — the write delta drops, LMDB untouched, and the outer
/// call returns `BDB_STATUS_ABORTED` (the ts bridge's abort sentinel,
/// spelled as control flow).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_callback_control {
    Ok = 0,
    Abort = 1,
}

/// How a bridge operation failed, before it is rendered to the boundary
/// protocol: a contract violation (no error allocated), or a typed error
/// the caller will own.
pub(crate) enum Fail {
    Misuse,
    Error(Box<bdb_error>),
}

pub(crate) type BridgeResult<T> = Result<T, Fail>;

/// The one panic wall (module doc: panic policy): every extern entry's
/// body runs under `catch_unwind`; a caught panic becomes
/// `BDB_ERROR_KIND_PANIC`.
pub(crate) fn guard(
    out_error: *mut *mut bdb_error,
    body: impl FnOnce() -> BridgeResult<bdb_status>,
) -> bdb_status {
    match std::panic::catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(status)) => status,
        Ok(Err(Fail::Misuse)) => bdb_status::Misuse,
        Ok(Err(Fail::Error(error))) => {
            store_error(out_error, error);
            bdb_status::Error
        }
        Err(payload) => {
            store_error(out_error, Box::new(bdb_error::from_panic(&payload)));
            bdb_status::Error
        }
    }
}

/// Writes the boxed error to the caller's out-param; a null out-param
/// drops the error (the caller declined the payload, keeping only the
/// status).
fn store_error(out_error: *mut *mut bdb_error, error: Box<bdb_error>) {
    let raw = Box::into_raw(error);
    #[expect(
        unsafe_code,
        reason = "the one error-out write; the header contract makes a non-null \
                  out_error a writable *mut bdb_error location"
    )]
    // SAFETY: `out_error` was null-checked; per the header contract a
    // non-null value points at a writable `bdb_error*` slot owned by the
    // caller for the duration of this call.
    unsafe {
        if out_error.is_null() {
            drop(Box::from_raw(raw));
        } else {
            *out_error = raw;
        }
    }
}

/// A required borrowed handle/view argument.
pub(crate) fn ref_in<'a, T>(ptr: *const T) -> BridgeResult<&'a T> {
    #[expect(
        unsafe_code,
        reason = "the shared read-borrow of every pointer argument; the header \
                  contract keeps the pointee alive and unaliased-for-writes for \
                  the duration of the call"
    )]
    // SAFETY: null was refused above the deref; per the header contract the
    // pointer names a live, properly aligned T (a handle minted by this
    // bridge or a caller view) that outlives this call.
    unsafe {
        ptr.as_ref().ok_or(Fail::Misuse)
    }
}

/// A required mutable handle argument.
pub(crate) fn mut_in<'a, T>(ptr: *mut T) -> BridgeResult<&'a mut T> {
    #[expect(
        unsafe_code,
        reason = "the shared write-borrow of every mutable handle argument; the \
                  header contract makes the handle exclusively the caller's for \
                  the duration of the call (no aliasing, single thread)"
    )]
    // SAFETY: null was refused above the deref; per the header contract the
    // pointer names a live T owned by the caller with no other live
    // reference during this call.
    unsafe {
        ptr.as_mut().ok_or(Fail::Misuse)
    }
}

/// A borrowed `(pointer, count)` view argument. `count == 0` admits a null
/// pointer (the empty view); a null pointer under a nonzero count is
/// misuse.
pub(crate) fn slice_in<'a, T>(ptr: *const T, count: usize) -> BridgeResult<&'a [T]> {
    if count == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(Fail::Misuse);
    }
    #[expect(
        unsafe_code,
        reason = "the shared slice-borrow of every (pointer, count) view; the \
                  header contract sizes the allocation at exactly count elements \
                  alive for the duration of the call"
    )]
    // SAFETY: non-null was just checked; per the header contract the caller
    // passes `count` contiguous, initialized T values that outlive this
    // call and are not written to during it.
    unsafe {
        Ok(std::slice::from_raw_parts(ptr, count))
    }
}

/// Writes a value through a required out-param.
pub(crate) fn out<T>(ptr: *mut T, value: T) -> BridgeResult<()> {
    if ptr.is_null() {
        return Err(Fail::Misuse);
    }
    #[expect(
        unsafe_code,
        reason = "the shared out-param write; the header contract makes every \
                  non-null out pointer a writable, properly aligned T location"
    )]
    // SAFETY: non-null was just checked; per the header contract the caller
    // passes a writable T slot. `write` (not `*ptr =`) because the slot may
    // be uninitialized C memory — nothing is dropped in place.
    unsafe {
        ptr.write(value);
    }
    Ok(())
}

/// Mints an owned handle for the boundary (paired with [`box_in`] at the
/// matching destroy).
pub(crate) fn box_out<T>(value: T) -> *mut T {
    Box::into_raw(Box::new(value))
}

/// Reclaims a [`box_out`]-minted handle at its destroy entry.
pub(crate) fn box_in<T>(ptr: *mut T) -> BridgeResult<Box<T>> {
    if ptr.is_null() {
        return Err(Fail::Misuse);
    }
    #[expect(
        unsafe_code,
        reason = "the shared handle reclaim; the header contract says destroy \
                  receives a pointer minted by the matching constructor, exactly \
                  once"
    )]
    // SAFETY: non-null was just checked; per the header contract the
    // pointer came from `box_out` (Box::into_raw) for this same T and is
    // never used again after this call.
    unsafe {
        Ok(Box::from_raw(ptr))
    }
}
