//! The typed error crossing: the opaque [`bdb_error`]
//! owns the rendered engine failure; accessor functions expose structured
//! payloads. Formatting happened once at construction (cold path — the
//! error IS the diagnostic); accessors only hand out views.
//!
//! The kind table is the FOURTH spelling of the engine taxonomy (Rust
//! enum, TypeScript union, tags.json, this C header). The sync mechanism
//! is mechanical: [`kind_of`] matches `bumbledb::Error` EXHAUSTIVELY — no
//! wildcard arm anywhere — so a new engine variant breaks this crate's
//! compile, exactly the discipline the Node bridge's `wire_tags!` tables
//! enforce. `BDB_ERROR_KIND_PARAM` covers the nine bind-time parameter
//! variants; `BDB_ERROR_KIND_PANIC` is bridge-synthesized (§30), never
//! engine-originated.

use bumbledb::{Error, SchemaDescriptor, render_rejection};

use crate::value::bdb_string_view;
use crate::{Fail, bdb_status, box_in, guard, out, ref_in};

/// The C error kind — one constant per engine error family, plus the
/// bridge-synthesized `Panic`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_error_kind {
    Schema,
    SchemaMismatch,
    FormatMismatch,
    AlreadyInitialized,
    NotInitialized,
    EnvironmentLocked,
    StoreKindMismatch,
    DescriptorMissing,
    ReadersFull,
    Validation,
    CommitRejected,
    CommitSync,
    GenerationMoved,
    ForeignSnapshot,
    ForeignPrepared,
    FactShape,
    ClosedRelationWrite,
    FreshExhausted,
    BulkLoad,
    Param,
    MeasureOfRay,
    CapacityRayMeasure,
    FixpointBudgetExceeded,
    Overflow,
    ResultBytesOverflow,
    Corruption,
    Io,
    Lmdb,
    Panic,
}

/// A violated statement's form tag (`bumbledb::StatementKind`, spelled C).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_statement_kind {
    Functionality,
    Containment,
    Capacity,
}

/// A containment citation's violated side; `None` for key and capacity
/// citations.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_violation_direction {
    None,
    SourceUnsatisfied,
    TargetRequired,
}

/// One rendered violation of a rejected commit, viewed: the statement's
/// fingerprint-pinned id, its form tag, its canonical spelling (borrowed
/// from the owning [`bdb_error`]), the containment direction where the
/// form has one, and the capacity measure (u128 as two u64 words) where
/// the form has one.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_violation {
    pub statement: u16,
    pub kind: bdb_statement_kind,
    pub spelling: bdb_string_view,
    pub direction: bdb_violation_direction,
    pub has_measure: bool,
    pub measure_lo: u64,
    pub measure_hi: u64,
}

/// One rendered violation, owned by the error.
pub(crate) struct OwnedViolation {
    statement: u16,
    kind: bdb_statement_kind,
    spelling: String,
    direction: bdb_violation_direction,
    measure: Option<u128>,
}

/// The opaque error handle: kind + rendered message + the structured
/// payloads the C++ SDK reads back. Owned by the caller after a
/// `BDB_STATUS_ERROR` return; freed by [`bdb_error_destroy`].
pub struct bdb_error {
    kind: bdb_error_kind,
    message: String,
    generation_moved: Option<(u64, u64)>,
    bulk_committed: Option<u64>,
    violations: Vec<OwnedViolation>,
}

/// The engine error's C kind — the EXHAUSTIVE match (module doc): a new
/// `bumbledb::Error` variant fails to compile here, on purpose. The nine
/// bind-time parameter variants collapse to `Param`; nothing else
/// collapses.
fn kind_of(error: &Error) -> bdb_error_kind {
    match error {
        Error::FormatMismatch { .. } => bdb_error_kind::FormatMismatch,
        Error::SchemaMismatch { .. } => bdb_error_kind::SchemaMismatch,
        Error::AlreadyInitialized => bdb_error_kind::AlreadyInitialized,
        Error::NotInitialized => bdb_error_kind::NotInitialized,
        Error::EnvironmentLocked => bdb_error_kind::EnvironmentLocked,
        Error::StoreKindMismatch { .. } => bdb_error_kind::StoreKindMismatch,
        Error::DescriptorMissing => bdb_error_kind::DescriptorMissing,
        Error::Io(_) => bdb_error_kind::Io,
        Error::Lmdb(_) => bdb_error_kind::Lmdb,
        Error::ReadersFull { .. } => bdb_error_kind::ReadersFull,
        Error::Schema(_) => bdb_error_kind::Schema,
        Error::Validation(_) => bdb_error_kind::Validation,
        Error::FactShape(_) => bdb_error_kind::FactShape,
        Error::CommitRejected { .. } => bdb_error_kind::CommitRejected,
        Error::FreshExhausted { .. } => bdb_error_kind::FreshExhausted,
        Error::ClosedRelationWrite { .. } => bdb_error_kind::ClosedRelationWrite,
        Error::GenerationMoved { .. } => bdb_error_kind::GenerationMoved,
        Error::CommitSync { .. } => bdb_error_kind::CommitSync,
        Error::BulkLoad { .. } => bdb_error_kind::BulkLoad,
        Error::ForeignPreparedQuery => bdb_error_kind::ForeignPrepared,
        Error::ForeignSnapshot => bdb_error_kind::ForeignSnapshot,
        Error::ParamCountMismatch { .. }
        | Error::ParamTypeMismatch { .. }
        | Error::ParamSetExpected { .. }
        | Error::ParamScalarExpected { .. }
        | Error::ParamElementTypeMismatch { .. }
        | Error::PointParamAtCeiling { .. }
        | Error::AllenMaskParamExpected { .. }
        | Error::EmptyAllenMaskParam { .. }
        | Error::FullAllenMaskParam { .. } => bdb_error_kind::Param,
        Error::MeasureOfRay { .. } => bdb_error_kind::MeasureOfRay,
        Error::CapacityRayMeasure { .. } => bdb_error_kind::CapacityRayMeasure,
        Error::FixpointBudgetExceeded { .. } => bdb_error_kind::FixpointBudgetExceeded,
        Error::Overflow(_) => bdb_error_kind::Overflow,
        Error::ResultBytesOverflow => bdb_error_kind::ResultBytesOverflow,
        Error::Corruption(_) => bdb_error_kind::Corruption,
    }
}

impl bdb_error {
    /// An engine error rendered for the boundary. `descriptor` (present at
    /// every db-scoped call site) lets a `CommitRejected` render its
    /// complete violation set through the engine's own
    /// [`render_rejection`] — the dumb-bridge law: no second renderer
    /// exists here.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "an engine error is SPENT by its rendering — the move is the \
                  API (every call site owns exactly one error and this is its \
                  one crossing)"
    )]
    pub(crate) fn from_engine(error: Error, descriptor: Option<&SchemaDescriptor>) -> Self {
        let kind = kind_of(&error);
        let generation_moved = match &error {
            Error::GenerationMoved { witnessed, current } => {
                Some((witnessed.value(), current.value()))
            }
            _ => None,
        };
        let bulk_committed = match &error {
            Error::BulkLoad { committed, .. } => Some(*committed),
            _ => None,
        };
        let violations = match (&error, descriptor) {
            (Error::CommitRejected { violations }, Some(descriptor)) => {
                render_rejection(descriptor, violations)
                    .into_iter()
                    .map(|rendered| OwnedViolation {
                        statement: rendered.statement.0,
                        kind: match rendered.kind {
                            bumbledb::StatementKind::Functionality => {
                                bdb_statement_kind::Functionality
                            }
                            bumbledb::StatementKind::Containment => {
                                bdb_statement_kind::Containment
                            }
                            bumbledb::StatementKind::Capacity => bdb_statement_kind::Capacity,
                        },
                        spelling: rendered.spelling,
                        direction: match rendered.direction {
                            None => bdb_violation_direction::None,
                            Some(bumbledb::Direction::SourceUnsatisfied) => {
                                bdb_violation_direction::SourceUnsatisfied
                            }
                            Some(bumbledb::Direction::TargetRequired) => {
                                bdb_violation_direction::TargetRequired
                            }
                        },
                        measure: rendered.measure,
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        Self {
            kind,
            message: format!("bumbledb: {error}"),
            generation_moved,
            bulk_committed,
            violations,
        }
    }

    /// A caught panic rendered to the poisoned-store error (§30).
    pub(crate) fn from_panic(payload: &(dyn std::any::Any + Send)) -> Self {
        let detail = payload
            .downcast_ref::<&'static str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("non-string panic payload");
        Self::synthesized(
            bdb_error_kind::Panic,
            format!("bumbledb-cpp: panic across the bridge (store poisoned): {detail}"),
        )
    }

    /// A bridge-synthesized error: marshal shape refusals
    /// (`BDB_ERROR_KIND_FACT_SHAPE`), the re-entrant write refusal
    /// (`BDB_ERROR_KIND_ENVIRONMENT_LOCKED`), spec-lowering failures
    /// (`BDB_ERROR_KIND_SCHEMA`), and the panic wall.
    pub(crate) fn synthesized(kind: bdb_error_kind, message: String) -> Self {
        Self {
            kind,
            message,
            generation_moved: None,
            bulk_committed: None,
            violations: Vec::new(),
        }
    }
}

/// An engine failure as a [`Fail`], one spelling per call site.
pub(crate) fn fail_engine(error: Error, descriptor: Option<&SchemaDescriptor>) -> Fail {
    Fail::Error(Box::new(bdb_error::from_engine(error, descriptor)))
}

/// A marshal shape refusal (a bad tag payload, an empty interval, an
/// invalid Allen mask, non-UTF-8 text): data-shaped input the engine
/// cannot represent — typed `BDB_ERROR_KIND_FACT_SHAPE`, mirroring the
/// engine's own dynamic-surface taxonomy.
pub(crate) fn fail_shape(message: &str) -> Fail {
    Fail::Error(Box::new(bdb_error::synthesized(
        bdb_error_kind::FactShape,
        format!("bumbledb-cpp marshal: {message}"),
    )))
}

/// The error's kind. A null handle answers `Panic` — the accessor cannot
/// carry a status, and `Panic` is the one kind that always means "stop
/// trusting this process's bridge state".
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_error_get_kind(error: *const bdb_error) -> bdb_error_kind {
    match ref_in(error) {
        Ok(error) => error.kind,
        Err(_) => bdb_error_kind::Panic,
    }
}

/// The rendered message, borrowed from the error (valid until
/// `bdb_error_destroy`). UTF-8, NOT NUL-terminated — the length is the
/// contract.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
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

/// The `GenerationMoved` payload: the witnessed and current generations.
/// `BDB_STATUS_MISUSE` when the error is not `BDB_ERROR_KIND_GENERATION_MOVED`.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_error_get_generation_moved(
    error: *const bdb_error,
    out_witnessed: *mut u64,
    out_current: *mut u64,
) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        let error = ref_in(error)?;
        let (witnessed, current) = error.generation_moved.ok_or(Fail::Misuse)?;
        out(out_witnessed, witnessed)?;
        out(out_current, current)?;
        Ok(bdb_status::Ok)
    })
}

/// The `BulkLoad` payload: facts durable in the chunks committed before
/// the failure. `BDB_STATUS_MISUSE` when the error is
/// not `BDB_ERROR_KIND_BULK_LOAD`.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_error_get_bulk_committed(
    error: *const bdb_error,
    out_committed: *mut u64,
) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        let error = ref_in(error)?;
        let committed = error.bulk_committed.ok_or(Fail::Misuse)?;
        out(out_committed, committed)?;
        Ok(bdb_status::Ok)
    })
}

/// The rendered violation count of a `BDB_ERROR_KIND_COMMIT_REJECTED` error
/// (0 for every other kind, and for a null handle).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_error_violation_count(error: *const bdb_error) -> usize {
    match ref_in(error) {
        Ok(error) => error.violations.len(),
        Err(_) => 0,
    }
}

/// One rendered violation, viewed (the spelling borrows from the error —
/// valid until `bdb_error_destroy`). Bounds-checked:
/// `BDB_STATUS_MISUSE` past [`bdb_error_violation_count`].
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_error_get_violation(
    error: *const bdb_error,
    index: usize,
    out_violation: *mut bdb_violation,
) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        let error = ref_in(error)?;
        let violation = error.violations.get(index).ok_or(Fail::Misuse)?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "u128 → two u64 words, lo then hi — no truncation, a split"
        )]
        let (measure_lo, measure_hi) = match violation.measure {
            Some(measure) => (measure as u64, (measure >> 64) as u64),
            None => (0, 0),
        };
        out(
            out_violation,
            bdb_violation {
                statement: violation.statement,
                kind: violation.kind,
                spelling: bdb_string_view::from_str(&violation.spelling),
                direction: violation.direction,
                has_measure: violation.measure.is_some(),
                measure_lo,
                measure_hi,
            },
        )?;
        Ok(bdb_status::Ok)
    })
}

/// Frees an error. Exactly once per owned error; a null pointer is
/// misuse.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_error_destroy(error: *mut bdb_error) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        drop(box_in(error)?);
        Ok(bdb_status::Ok)
    })
}
