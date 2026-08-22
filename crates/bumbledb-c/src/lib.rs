//! The C ABI: `bdb_*` symbols, dumb marshal only.
//! The dumb-bridge law (the ts/crate precedent): no logic beyond marshaling
//! will EVER live in this crate. This crate exists only to carry values across the C
//! boundary.
//! # Safety shape
#![allow(non_camel_case_types)]
pub mod answers;
pub mod db;
pub mod error;
pub mod query;
pub mod schema;
pub mod value;

#[cfg(test)]
mod tests;

use std::ffi::c_char;
use std::mem::size_of;
use std::panic::AssertUnwindSafe;

use error::bdb_error;

/// Crate version, NUL-terminated, program lifetime. Mirrors the Node
/// bridge's `engine_version` as a C string the host can print.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_version() -> *const c_char {
    concat!("bumbledb-c ", env!("CARGO_PKG_VERSION"), "\0")
        .as_ptr()
        .cast::<c_char>()
}

/// C ABI generation. `4` is the 0.17.0 purge: the measure/duration
/// family left the query surface, so `bdb_error_kind` and
/// `bdb_find_term_kind` renumbered — a host compiled against the
/// generation-3 header misreads those tags and must recompile. (`3` was
/// instance-lifetime: admission unions, the builder/owned/witness
/// handles, and the retirement of snapshot-named functions.)
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_abi_version() -> u32 {
    4
}

macro_rules! c_tag {
    ($ty:ty { $($variant:ident),* $(,)? }) => {
        impl TryFrom<u32> for $ty {
            type Error = ();
            fn try_from(tag: u32) -> Result<Self, ()> {
                $(if tag == Self::$variant as u32 {
                    return Ok(Self::$variant);
                })*
                Err(())
            }
        }
        impl From<$ty> for u32 {
            fn from(value: $ty) -> u32 {
                value as u32
            }
        }
    };
}
pub(crate) use c_tag;

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

c_tag!(bdb_callback_control { Ok, Abort });

/// How a bridge operation failed, before it is rendered to the boundary
/// protocol: a contract violation (no error allocated), or a typed error the
/// caller will own.
pub(crate) enum Fail {
    Misuse,
    Error(Box<bdb_error>),
}

pub(crate) type BridgeResult<T> = Result<T, Fail>;

pub(crate) fn guard(
    out_error: *mut *mut bdb_error,
    body: impl FnOnce() -> BridgeResult<bdb_status>,
) -> bdb_status {
    match std::panic::catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(status)) => {
            debug_assert!(
                status != bdb_status::Error,
                "guard body returned Error without Fail::Error"
            );
            status
        }
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

pub(crate) fn guard_statusless(body: impl FnOnce() -> BridgeResult<bdb_status>) -> bdb_status {
    match std::panic::catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(status)) => {
            debug_assert!(
                status != bdb_status::Error,
                "statusless body returned Error"
            );
            status
        }
        Ok(Err(Fail::Misuse | Fail::Error(_))) => bdb_status::Misuse,
        Err(_) => bdb_status::Error,
    }
}

fn store_error(out_error: *mut *mut bdb_error, error: Box<bdb_error>) {
    let raw = Box::into_raw(error);
    #[expect(
        unsafe_code,
        reason = "the one error-out write; the header contract makes a non-null \
                  out_error a writable *mut bdb_error location"
    )]
    // SAFETY: `out_error` was null-checked; per the header contract a

    unsafe {
        if out_error.is_null() {
            drop(Box::from_raw(raw));
        } else {
            let previous = *out_error;
            if !previous.is_null() {
                drop(Box::from_raw(previous));
            }
            *out_error = raw;
        }
    }
}

pub(crate) fn guard_value<T>(fallback: T, body: impl FnOnce() -> T) -> T {
    std::panic::catch_unwind(AssertUnwindSafe(body)).unwrap_or(fallback)
}

pub(crate) fn tag_in<T: TryFrom<u32>>(raw: u32) -> BridgeResult<T> {
    T::try_from(raw).map_err(|_| Fail::Misuse)
}

pub(crate) fn bool_in(raw: u8) -> BridgeResult<bool> {
    match raw {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Fail::Misuse),
    }
}

pub(crate) fn ref_in<'a, T>(ptr: *const T) -> BridgeResult<&'a T> {
    #[expect(
        unsafe_code,
        reason = "the shared read-borrow of every pointer argument; the header \
                  contract keeps the pointee alive and unaliased-for-writes for \
                  the duration of the call"
    )]
    // SAFETY: null was refused above the deref; per the header contract the

    unsafe {
        ptr.as_ref().ok_or(Fail::Misuse)
    }
}

pub(crate) fn mut_in<'a, T>(ptr: *mut T) -> BridgeResult<&'a mut T> {
    #[expect(
        unsafe_code,
        reason = "the shared write-borrow of every mutable handle argument; the \
                  header contract makes the handle exclusively the caller's for \
                  the duration of the call (no aliasing, single thread)"
    )]
    // SAFETY: null was refused above the deref; per the header contract the

    unsafe {
        ptr.as_mut().ok_or(Fail::Misuse)
    }
}

pub(crate) fn slice_in<'a, T>(ptr: *const T, count: usize) -> BridgeResult<&'a [T]> {
    if count == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(Fail::Misuse);
    }
    let bytes = count.checked_mul(size_of::<T>()).ok_or(Fail::Misuse)?;
    if bytes > isize::MAX as usize {
        return Err(Fail::Misuse);
    }
    if !ptr.is_aligned() {
        return Err(Fail::Misuse);
    }
    #[expect(
        unsafe_code,
        reason = "the shared slice-borrow of every (pointer, count) view; the \
                  header contract sizes the allocation at exactly count elements \
                  alive for the duration of the call"
    )]
    // SAFETY: non-null, alignment, and `count * size_of::<T>()` ≤

    unsafe {
        Ok(std::slice::from_raw_parts(ptr, count))
    }
}

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

    unsafe {
        ptr.write(value);
    }
    Ok(())
}

pub(crate) fn box_out<T>(value: T) -> *mut T {
    Box::into_raw(Box::new(value))
}

/// Required out-param: null is misuse, before any `into_raw`.
pub(crate) fn require_out<T>(ptr: *mut T) -> BridgeResult<*mut T> {
    if ptr.is_null() {
        Err(Fail::Misuse)
    } else {
        Ok(ptr)
    }
}

/// The `Box` is only `into_raw`'d after the slot is proven non-null, so a null
/// out-param drops `value` instead of leaking it.
pub(crate) fn box_out_to<T>(ptr: *mut *mut T, value: T) -> BridgeResult<()> {
    let ptr = require_out(ptr)?;
    out(ptr, box_out(value))
}

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

    // never used again after this call.
    unsafe {
        Ok(Box::from_raw(ptr))
    }
}
