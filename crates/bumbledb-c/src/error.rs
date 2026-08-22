//! The typed error crossing: the opaque [`bdb_error`] owns a rendered
//! engine or bridge failure. Theory rejection is not an error — it lives
//! on [`bdb_violations`], the owning handle of an admission rejected arm.
//!
//! The kind table is the FOURTH spelling of the engine taxonomy (Rust
//! enum, TypeScript union, tags.json, this C header). The sync mechanism
//! is mechanical: the engine's [`Error::family`] table is exhaustive over
//! `Error`, and [`kind_of`] matches [`ErrorFamily`] 1:1 — no wildcard arm
//! anywhere — so a new engine variant breaks the engine crate, and a new
//! family arm breaks this crate. `BDB_ERROR_KIND_PARAM` covers the six
//! bind-time parameter variants; `BDB_ERROR_KIND_PANIC` is
//! bridge-synthesized, never engine-originated. Bridge refusals
//! (`BusyHandle`, `Marshal`) carry [`bdb_error_origin::Bridge`] and never
//! impersonate engine kinds.

use bumbledb::{
    Direction, Error, ErrorFamily, SchemaDescriptor, StatementKind, Violations, render_rejection,
};

use crate::value::bdb_string_view;
use crate::{Fail, bdb_status, box_in, guard, guard_statusless, guard_value, out, ref_in};

/// Origin of a [`bdb_error`]: engine taxonomy vs bridge marshal/busy.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_error_origin {
    Engine = 0,
    Bridge = 1,
}

/// The C error kind — one constant per engine error family, plus the
/// bridge-synthesized `Panic`, `BusyHandle`, and `Marshal`. Proved write
/// outcomes are admission-union arms, not kinds.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_error_kind {
    Schema,
    SchemaMismatch,
    FormatMismatch,
    AlreadyInitialized,
    DestinationExists,
    PublishedButUnsynced,
    EnvironmentLocked,
    ReadersFull,
    Validation,
    CommitSync,
    ForeignWitness,
    ForeignPrepared,
    FactShape,
    ClosedRelationWrite,
    FreshExhausted,
    TransactionPoisoned,
    Param,
    CapacityRayMeasure,
    DerivedBudgetExceeded,
    Overflow,
    ResultBytesOverflow,
    Corruption,
    Io,
    Lmdb,
    Panic,
    BusyHandle,
    Marshal,
}

/// A violated statement's form tag (`bumbledb::StatementKind`, spelled C).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_statement_kind {
    Functionality,
    Containment,
    Capacity,
}

/// A containment citation's violated side. Live only on the containment
/// payload arm.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_violation_direction {
    SourceUnsatisfied,
    TargetRequired,
}

/// Capacity measure as two u64 words (lo then hi). Live only on the
/// capacity payload arm.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_capacity_measure {
    pub lo: u64,
    pub hi: u64,
}

/// Per-kind payload of [`bdb_violation`]. Inspect the arm that matches
/// `kind`; the other cells are uninitialized.
#[repr(C)]
#[derive(Clone, Copy)]
pub union bdb_violation_payload {
    pub functionality: u8,
    pub containment: bdb_violation_direction,
    pub capacity: bdb_capacity_measure,
}

/// One rendered violation, viewed: statement id, form tag, canonical
/// spelling (borrowed from the owning [`bdb_violations`]), and the
/// kind's payload arm.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_violation {
    pub statement: u16,
    pub kind: bdb_statement_kind,
    pub spelling: bdb_string_view,
    pub payload: bdb_violation_payload,
}

struct OwnedViolation {
    statement: u16,
    kind: bdb_statement_kind,
    spelling: String,
    direction: Option<bdb_violation_direction>,
    measure: Option<u128>,
}

/// Owning handle for a rejected admission's violation set. Destroy with
/// [`bdb_violations_destroy`].
pub struct bdb_violations {
    violations: Vec<OwnedViolation>,
}

/// The opaque error handle: origin, kind, rendered message. Owned by the
/// caller after a `BDB_STATUS_ERROR` return; freed by [`bdb_error_destroy`].
pub struct bdb_error {
    origin: bdb_error_origin,
    kind: bdb_error_kind,
    message: String,
}

fn kind_of(error: &Error) -> bdb_error_kind {
    match error.family() {
        ErrorFamily::FormatMismatch => bdb_error_kind::FormatMismatch,
        ErrorFamily::SchemaMismatch => bdb_error_kind::SchemaMismatch,
        ErrorFamily::AlreadyInitialized => bdb_error_kind::AlreadyInitialized,
        ErrorFamily::DestinationExists => bdb_error_kind::DestinationExists,
        ErrorFamily::PublishedButUnsynced => bdb_error_kind::PublishedButUnsynced,
        ErrorFamily::EnvironmentLocked => bdb_error_kind::EnvironmentLocked,
        ErrorFamily::Io => bdb_error_kind::Io,
        ErrorFamily::Lmdb => bdb_error_kind::Lmdb,
        ErrorFamily::ReadersFull => bdb_error_kind::ReadersFull,
        ErrorFamily::Schema => bdb_error_kind::Schema,
        ErrorFamily::Validation => bdb_error_kind::Validation,
        ErrorFamily::FactShape => bdb_error_kind::FactShape,
        ErrorFamily::FreshExhausted => bdb_error_kind::FreshExhausted,
        ErrorFamily::ClosedRelationWrite => bdb_error_kind::ClosedRelationWrite,
        ErrorFamily::CommitSync => bdb_error_kind::CommitSync,
        ErrorFamily::TransactionPoisoned => bdb_error_kind::TransactionPoisoned,
        ErrorFamily::ForeignPreparedQuery => bdb_error_kind::ForeignPrepared,
        ErrorFamily::ForeignWitness => bdb_error_kind::ForeignWitness,
        ErrorFamily::Param => bdb_error_kind::Param,
        ErrorFamily::CapacityRayMeasure => bdb_error_kind::CapacityRayMeasure,
        ErrorFamily::DerivedBudgetExceeded => bdb_error_kind::DerivedBudgetExceeded,
        ErrorFamily::Overflow => bdb_error_kind::Overflow,
        ErrorFamily::ResultBytesOverflow => bdb_error_kind::ResultBytesOverflow,
        ErrorFamily::Corruption => bdb_error_kind::Corruption,
    }
}

fn owned_violations(violations: &Violations, descriptor: &SchemaDescriptor) -> Vec<OwnedViolation> {
    render_rejection(descriptor, violations)
        .into_iter()
        .map(|rendered| OwnedViolation {
            statement: rendered.statement().0,
            kind: match rendered.kind() {
                StatementKind::Functionality => bdb_statement_kind::Functionality,
                StatementKind::Containment => bdb_statement_kind::Containment,
                StatementKind::Capacity => bdb_statement_kind::Capacity,
            },
            spelling: rendered.spelling().to_owned(),
            direction: match rendered.direction() {
                None => None,
                Some(Direction::SourceUnsatisfied) => {
                    Some(bdb_violation_direction::SourceUnsatisfied)
                }
                Some(Direction::TargetRequired) => Some(bdb_violation_direction::TargetRequired),
            },
            measure: rendered.measure(),
        })
        .collect()
}

fn view_violation(violation: &OwnedViolation) -> bdb_violation {
    let payload = match violation.kind {
        bdb_statement_kind::Functionality => bdb_violation_payload { functionality: 0 },
        bdb_statement_kind::Containment => bdb_violation_payload {
            containment: violation
                .direction
                .unwrap_or(bdb_violation_direction::SourceUnsatisfied),
        },
        bdb_statement_kind::Capacity => {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "u128 → two u64 words, lo then hi — no truncation, a split"
            )]
            let (lo, hi) = match violation.measure {
                Some(measure) => (measure as u64, (measure >> 64) as u64),
                None => (0, 0),
            };
            bdb_violation_payload {
                capacity: bdb_capacity_measure { lo, hi },
            }
        }
    };
    bdb_violation {
        statement: violation.statement,
        kind: violation.kind,
        spelling: bdb_string_view::from_str(&violation.spelling),
        payload,
    }
}

impl bdb_violations {
    pub(crate) fn from_engine(violations: &Violations, descriptor: &SchemaDescriptor) -> Self {
        Self {
            violations: owned_violations(violations, descriptor),
        }
    }
}

impl bdb_error {
    /// An engine error rendered for the boundary.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "an engine error is SPENT by its rendering"
    )]
    pub(crate) fn from_engine(error: Error) -> Self {
        Self {
            origin: bdb_error_origin::Engine,
            kind: kind_of(&error),
            message: format!("bumbledb: {error}"),
        }
    }

    /// A caught panic rendered to the poisoned-store error.
    pub(crate) fn from_panic(payload: &(dyn std::any::Any + Send)) -> Self {
        let detail = payload
            .downcast_ref::<&'static str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("non-string panic payload");
        Self::bridge(
            bdb_error_kind::Panic,
            format!("bumbledb-c: panic across the bridge (store poisoned): {detail}"),
        )
    }

    pub(crate) fn bridge(kind: bdb_error_kind, message: String) -> Self {
        Self {
            origin: bdb_error_origin::Bridge,
            kind,
            message,
        }
    }
}

/// An engine failure as a [`Fail`].
pub(crate) fn fail_engine(error: Error) -> Fail {
    Fail::Error(Box::new(bdb_error::from_engine(error)))
}

/// A marshal shape refusal: data-shaped input the engine cannot
/// represent. Bridge origin, never an impersonated `FactShape`.
pub(crate) fn fail_shape(message: &str) -> Fail {
    Fail::Error(Box::new(bdb_error::bridge(
        bdb_error_kind::Marshal,
        format!("bumbledb-c marshal: {message}"),
    )))
}

/// Handle already in a callback or execute. Bridge origin, never an
/// impersonated `EnvironmentLocked`.
pub(crate) fn fail_busy(message: &str) -> Fail {
    Fail::Error(Box::new(bdb_error::bridge(
        bdb_error_kind::BusyHandle,
        format!("bumbledb-c: {message}"),
    )))
}

/// Schema-spec lowering failure, still the engine `Schema` kind with
/// bridge origin — the message is the engine's issue list.
pub(crate) fn fail_schema_message(message: &str) -> Fail {
    Fail::Error(Box::new(bdb_error::bridge(
        bdb_error_kind::Schema,
        format!("bumbledb: {message}"),
    )))
}

/// The error's origin. A null handle answers `Bridge`.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_error_get_origin(error: *const bdb_error) -> bdb_error_origin {
    guard_value(bdb_error_origin::Bridge, || match ref_in(error) {
        Ok(error) => error.origin,
        Err(_) => bdb_error_origin::Bridge,
    })
}

/// The error's kind. A null handle answers `Panic`.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_error_get_kind(error: *const bdb_error) -> bdb_error_kind {
    guard_value(bdb_error_kind::Panic, || match ref_in(error) {
        Ok(error) => error.kind,
        Err(_) => bdb_error_kind::Panic,
    })
}

/// The rendered message, borrowed from the error (valid until
/// `bdb_error_destroy`). UTF-8, NOT NUL-terminated.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_error_get_message(
    error: *const bdb_error,
    out_message: *mut bdb_string_view,
) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        let error = ref_in(error)?;
        out(out_message, bdb_string_view::from_str(&error.message))?;
        Ok(bdb_status::Ok)
    })
}

/// Frees an error. Exactly once per owned error; a null pointer is
/// misuse.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_error_destroy(error: *mut bdb_error) -> bdb_status {
    guard_statusless(|| {
        drop(box_in(error)?);
        Ok(bdb_status::Ok)
    })
}

/// The rendered violation count (0 for a null handle).
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_violations_len(violations: *const bdb_violations) -> usize {
    guard_value(0, || match ref_in(violations) {
        Ok(violations) => violations.violations.len(),
        Err(_) => 0,
    })
}

/// One rendered violation, viewed (the spelling borrows from the handle).
/// Bounds-checked: `BDB_STATUS_MISUSE` past [`bdb_violations_len`].
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_violations_get(
    violations: *const bdb_violations,
    index: usize,
    out_violation: *mut bdb_violation,
) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        let violations = ref_in(violations)?;
        let violation = violations.violations.get(index).ok_or(Fail::Misuse)?;
        out(out_violation, view_violation(violation))?;
        Ok(bdb_status::Ok)
    })
}

/// Frees a violations handle. A null pointer is misuse.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_violations_destroy(violations: *mut bdb_violations) -> bdb_status {
    guard_statusless(|| {
        drop(box_in(violations)?);
        Ok(bdb_status::Ok)
    })
}
