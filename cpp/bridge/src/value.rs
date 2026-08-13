//! The value crossing: one `#[repr(C)]` tagged POD,
//! [`bdb_value`], carries every `bumbledb::Value` variant in BOTH
//! directions. Inbound, the bridge copies the view into Rust-owned data
//! before any engine call (no borrowed C++ memory survives the entry);
//! outbound, variable-width payloads BORROW from the Rust-owned carrier
//! (`bdb_row_set`, `bdb_answers`, `bdb_error`) named at each accessor.
//!
//! Intervals are CHECKED at the boundary exactly as the engine checks
//! them (`start < end`); an empty interval, an invalid Allen mask, or
//! non-UTF-8 string bytes are `BDB_ERROR_KIND_FACT_SHAPE` marshal refusals,
//! never a silent repair.

use bumbledb::{AllenMask, AnswerValue, BindValue, Interval, Value};

use crate::error::fail_shape;
use crate::{BridgeResult, slice_in};

/// A borrowed UTF-8 text view (NOT NUL-terminated; the length is the
/// contract). A null `data` with `len == 0` is the empty string; a null
/// `data` under a nonzero `len` is misuse. In optional positions
/// (`bdb_field_spec.newtype`) a null `data` means ABSENT.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_string_view {
    pub data: *const u8,
    pub len: usize,
}

impl bdb_string_view {
    pub(crate) fn from_str(text: &str) -> Self {
        Self {
            data: text.as_ptr(),
            len: text.len(),
        }
    }

    /// The view's text, UTF-8-checked and borrowed (copied by every
    /// caller before the engine sees it).
    pub(crate) fn as_str<'a>(&self, what: &str) -> BridgeResult<&'a str> {
        let bytes = slice_in(self.data, self.len)?;
        std::str::from_utf8(bytes).map_err(|_| fail_shape(&format!("non-UTF-8 {what}")))
    }

    /// An optional-position view: null `data` is `None`.
    pub(crate) fn as_opt_str<'a>(&self, what: &str) -> BridgeResult<Option<&'a str>> {
        if self.data.is_null() && self.len == 0 {
            return Ok(None);
        }
        self.as_str(what).map(Some)
    }
}

/// A borrowed raw-byte view (`bytes<N>` payloads). Null/len rules as
/// [`bdb_string_view`].
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_bytes_view {
    pub data: *const u8,
    pub len: usize,
}

impl bdb_bytes_view {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            data: bytes.as_ptr(),
            len: bytes.len(),
        }
    }

    pub(crate) fn as_bytes<'a>(&self) -> BridgeResult<&'a [u8]> {
        slice_in(self.data, self.len)
    }
}

/// The value tag — one constant per `bumbledb::Value` variant.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_value_kind {
    Bool,
    U64,
    I64,
    String,
    FixedBytes,
    IntervalU64,
    IntervalI64,
    AllenMask,
}

/// One tagged value. Only the fields the `kind` names are read; the rest
/// are ignored inbound and zeroed outbound. Boring and flat by design —
/// no union, no packing.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_value {
    pub kind: bdb_value_kind,
    pub bool_value: bool,
    pub u64_value: u64,
    pub i64_value: i64,
    /// `String`: UTF-8 text (checked at the boundary).
    pub string_value: bdb_string_view,
    /// `FixedBytes`: exactly the field's N bytes (the engine checks N).
    pub bytes_value: bdb_bytes_view,
    /// `IntervalU64`: half-open `[start, end)`, `start < end` checked.
    pub interval_u64_start: u64,
    pub interval_u64_end: u64,
    /// `IntervalI64`: half-open `[start, end)`, `start < end` checked.
    pub interval_i64_start: i64,
    pub interval_i64_end: i64,
    /// `AllenMask`: the low-13-bit mask (checked at the boundary).
    pub allen_mask: u16,
}

impl bdb_value {
    /// The zeroed skeleton every outbound constructor fills.
    pub(crate) fn blank(kind: bdb_value_kind) -> Self {
        Self {
            kind,
            bool_value: false,
            u64_value: 0,
            i64_value: 0,
            string_value: bdb_string_view {
                data: std::ptr::null(),
                len: 0,
            },
            bytes_value: bdb_bytes_view {
                data: std::ptr::null(),
                len: 0,
            },
            interval_u64_start: 0,
            interval_u64_end: 0,
            interval_i64_start: 0,
            interval_i64_end: 0,
            allen_mask: 0,
        }
    }
}

/// One inbound tagged value, copied into the engine's owned [`Value`].
pub(crate) fn value_in(view: &bdb_value) -> BridgeResult<Value> {
    match view.kind {
        bdb_value_kind::Bool => Ok(Value::Bool(view.bool_value)),
        bdb_value_kind::U64 => Ok(Value::U64(view.u64_value)),
        bdb_value_kind::I64 => Ok(Value::I64(view.i64_value)),
        bdb_value_kind::String => {
            let text = view.string_value.as_str("string value")?;
            Ok(Value::String(text.as_bytes().to_vec().into_boxed_slice()))
        }
        bdb_value_kind::FixedBytes => {
            let bytes = view.bytes_value.as_bytes()?;
            Ok(Value::FixedBytes(bytes.to_vec().into_boxed_slice()))
        }
        bdb_value_kind::IntervalU64 => {
            let (start, end) = (view.interval_u64_start, view.interval_u64_end);
            Interval::<u64>::new(start, end)
                .map(Value::IntervalU64)
                .ok_or_else(|| fail_shape(&format!("empty interval (start {start} >= end {end})")))
        }
        bdb_value_kind::IntervalI64 => {
            let (start, end) = (view.interval_i64_start, view.interval_i64_end);
            Interval::<i64>::new(start, end)
                .map(Value::IntervalI64)
                .ok_or_else(|| fail_shape(&format!("empty interval (start {start} >= end {end})")))
        }
        bdb_value_kind::AllenMask => {
            let bits = view.allen_mask;
            AllenMask::new(bits)
                .map(Value::AllenMask)
                .ok_or_else(|| fail_shape(&format!("invalid allen mask bits {bits}")))
        }
    }
}

/// One inbound `(values, count)` row, copied whole.
pub(crate) fn row_in(values: *const bdb_value, count: usize) -> BridgeResult<Vec<Value>> {
    slice_in(values, count)?.iter().map(value_in).collect()
}

/// One outbound engine value, viewed — variable-width payloads borrow
/// `value` (the accessor names the owning carrier and its lifetime).
pub(crate) fn value_out(value: &Value) -> bdb_value {
    match value {
        Value::Bool(v) => {
            let mut view = bdb_value::blank(bdb_value_kind::Bool);
            view.bool_value = *v;
            view
        }
        Value::U64(v) => {
            let mut view = bdb_value::blank(bdb_value_kind::U64);
            view.u64_value = *v;
            view
        }
        Value::I64(v) => {
            let mut view = bdb_value::blank(bdb_value_kind::I64);
            view.i64_value = *v;
            view
        }
        Value::String(bytes) => {
            let mut view = bdb_value::blank(bdb_value_kind::String);
            view.string_value = bdb_string_view {
                data: bytes.as_ptr(),
                len: bytes.len(),
            };
            view
        }
        Value::FixedBytes(bytes) => {
            let mut view = bdb_value::blank(bdb_value_kind::FixedBytes);
            view.bytes_value = bdb_bytes_view::from_bytes(bytes);
            view
        }
        Value::IntervalU64(interval) => {
            let mut view = bdb_value::blank(bdb_value_kind::IntervalU64);
            view.interval_u64_start = interval.start();
            view.interval_u64_end = interval.end();
            view
        }
        Value::IntervalI64(interval) => {
            let mut view = bdb_value::blank(bdb_value_kind::IntervalI64);
            view.interval_i64_start = interval.start();
            view.interval_i64_end = interval.end();
            view
        }
        Value::AllenMask(mask) => {
            let mut view = bdb_value::blank(bdb_value_kind::AllenMask);
            view.allen_mask = mask.bits();
            view
        }
    }
}

/// One outbound answer cell, viewed — string/bytes payloads borrow the
/// `bdb_answers` carrier.
pub(crate) fn answer_out(value: AnswerValue<'_>) -> bdb_value {
    match value {
        AnswerValue::Bool(v) => {
            let mut view = bdb_value::blank(bdb_value_kind::Bool);
            view.bool_value = v;
            view
        }
        AnswerValue::U64(v) => {
            let mut view = bdb_value::blank(bdb_value_kind::U64);
            view.u64_value = v;
            view
        }
        AnswerValue::I64(v) => {
            let mut view = bdb_value::blank(bdb_value_kind::I64);
            view.i64_value = v;
            view
        }
        AnswerValue::String(text) => {
            let mut view = bdb_value::blank(bdb_value_kind::String);
            view.string_value = bdb_string_view::from_str(text);
            view
        }
        AnswerValue::FixedBytes(bytes) => {
            let mut view = bdb_value::blank(bdb_value_kind::FixedBytes);
            view.bytes_value = bdb_bytes_view::from_bytes(bytes);
            view
        }
        AnswerValue::IntervalU64(interval) => {
            let mut view = bdb_value::blank(bdb_value_kind::IntervalU64);
            view.interval_u64_start = interval.start();
            view.interval_u64_end = interval.end();
            view
        }
        AnswerValue::IntervalI64(interval) => {
            let mut view = bdb_value::blank(bdb_value_kind::IntervalI64);
            view.interval_i64_start = interval.start();
            view.interval_i64_end = interval.end();
            view
        }
    }
}

/// The execute-parameter tag: a scalar (a [`bdb_value`]) or a param set
/// (a value array — points only; the engine types the elements).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_param_kind {
    Scalar,
    Set,
}

/// One positional execution argument — the C mirror of the engine's
/// public `ParamArg` shape (Scalar | Set; an Allen mask travels as a
/// scalar `AllenMask` value).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_param {
    pub kind: bdb_param_kind,
    /// `Scalar`: the value.
    pub scalar: bdb_value,
    /// `Set`: `set_len` tagged values.
    pub set: *const bdb_value,
    pub set_len: usize,
}

/// One positional argument, owned (the ts bridge's `OwnedParam`): params
/// are copied off the C views before binding.
pub(crate) enum OwnedParam {
    Scalar(Value),
    Set(Vec<Value>),
}

/// The inbound params array, copied whole.
pub(crate) fn params_in(params: *const bdb_param, count: usize) -> BridgeResult<Vec<OwnedParam>> {
    slice_in(params, count)?
        .iter()
        .map(|param| match param.kind {
            bdb_param_kind::Scalar => Ok(OwnedParam::Scalar(value_in(&param.scalar)?)),
            bdb_param_kind::Set => Ok(OwnedParam::Set(row_in(param.set, param.set_len)?)),
        })
        .collect()
}

/// One owned scalar to the engine's bind value (the ts bridge's
/// `bind_value`, verbatim): string payloads re-borrow as `&str` —
/// marshaling admitted only UTF-8, so the re-check cannot fire, but a
/// corrupt payload is refused typed rather than unwrapped.
pub(crate) fn bind_value(value: &Value) -> BridgeResult<BindValue<'_>> {
    Ok(match value {
        Value::Bool(v) => BindValue::Bool(*v),
        Value::U64(v) => BindValue::U64(*v),
        Value::I64(v) => BindValue::I64(*v),
        Value::String(bytes) => BindValue::Str(
            std::str::from_utf8(bytes)
                .map_err(|_| fail_shape("non-UTF-8 string param"))?,
        ),
        Value::FixedBytes(bytes) => BindValue::FixedBytes(bytes),
        Value::IntervalU64(interval) => BindValue::IntervalU64(interval.start(), interval.end()),
        Value::IntervalI64(interval) => BindValue::IntervalI64(interval.start(), interval.end()),
        Value::AllenMask(mask) => BindValue::AllenMask(*mask),
    })
}

/// Owned params to the engine's positional bind arguments.
pub(crate) fn param_args(
    params: &[OwnedParam],
) -> BridgeResult<Vec<bumbledb::ParamArg<'_>>> {
    params
        .iter()
        .map(|param| match param {
            OwnedParam::Set(values) => Ok(bumbledb::ParamArg::Set(values)),
            OwnedParam::Scalar(value) => Ok(bumbledb::ParamArg::Scalar(bind_value(value)?)),
        })
        .collect()
}
