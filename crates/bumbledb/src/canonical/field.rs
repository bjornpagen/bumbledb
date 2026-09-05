//! Shared canonical field parsing — one interval law for strict value decode
//! and column-word decode (CORE-001). Every trusted reader applies the same
//! `value_matches` checks, including fixed-interval width and F64 endpoint
//! validity; the image walker never uses a text sentinel as float validity.

use bumbledb_theory::schema::FieldDescriptor;

use crate::schema::{ValueType, value_matches};
use crate::{F64, Interval, Value};

use super::RowError;

/// Decode one stored U64 interval pair under the field descriptor.
pub(crate) fn decode_interval_u64(
    start: u64,
    end: u64,
    descriptor: &FieldDescriptor,
    field: usize,
) -> Result<Value, RowError> {
    let interval = Interval::new(start, end).ok_or(RowError::InvalidInterval { field })?;
    let value = Value::IntervalU64(interval);
    value_matches(&value, &descriptor.value_type).map_err(|_| RowError::Type { field })?;
    Ok(value)
}

/// Decode one stored I64 interval pair under the field descriptor.
pub(crate) fn decode_interval_i64(
    start: i64,
    end: i64,
    descriptor: &FieldDescriptor,
    field: usize,
) -> Result<Value, RowError> {
    let interval = Interval::new(start, end).ok_or(RowError::InvalidInterval { field })?;
    let value = Value::IntervalI64(interval);
    value_matches(&value, &descriptor.value_type).map_err(|_| RowError::Type { field })?;
    Ok(value)
}

/// Decode one stored canonical F64 interval pair under the field descriptor.
pub(crate) fn decode_interval_f64(
    start: F64,
    end: F64,
    descriptor: &FieldDescriptor,
    field: usize,
) -> Result<Value, RowError> {
    let interval = Interval::new(start, end).ok_or(RowError::InvalidInterval { field })?;
    let value = Value::IntervalF64(interval);
    value_matches(&value, &descriptor.value_type).map_err(|_| RowError::Type { field })?;
    Ok(value)
}

/// Total-order words for one U64 or fixed-U64 interval after the shared law.
pub(crate) fn interval_u64_order_words(
    start: u64,
    end: u64,
    descriptor: &FieldDescriptor,
) -> Result<(u64, u64), RowError> {
    match decode_interval_u64(start, end, descriptor, 0)? {
        Value::IntervalU64(interval) => Ok((interval.start(), interval.end())),
        _ => unreachable!("decode_interval_u64 returns IntervalU64"),
    }
}

/// Total-order words for one I64 or fixed-I64 interval after the shared law.
pub(crate) fn interval_i64_order_words(
    start: i64,
    end: i64,
    descriptor: &FieldDescriptor,
) -> Result<(u64, u64), RowError> {
    match decode_interval_i64(start, end, descriptor, 0)? {
        Value::IntervalI64(interval) => Ok((i64_word(interval.start()), i64_word(interval.end()))),
        _ => unreachable!("decode_interval_i64 returns IntervalI64"),
    }
}

/// Total-order words for one F64 interval after the shared law.
pub(crate) fn interval_f64_order_words(
    start: F64,
    end: F64,
    descriptor: &FieldDescriptor,
) -> Result<(u64, u64), RowError> {
    match decode_interval_f64(start, end, descriptor, 0)? {
        Value::IntervalF64(interval) => Ok((
            interval.start().to_order_key(),
            interval.end().to_order_key(),
        )),
        _ => unreachable!("decode_interval_f64 returns IntervalF64"),
    }
}

/// Whether this descriptor's interval arm uses tag 6 on the wire.
pub(crate) const fn interval_tag_u64(descriptor: &FieldDescriptor) -> bool {
    matches!(
        descriptor.value_type,
        ValueType::Interval {
            element: bumbledb_theory::schema::IntervalElement::U64,
        }
        | ValueType::FixedInterval {
            element: bumbledb_theory::schema::FixedIntervalElement::U64,
            ..
        }
    )
}

/// Whether this descriptor's interval arm uses tag 7 on the wire.
pub(crate) const fn interval_tag_i64(descriptor: &FieldDescriptor) -> bool {
    matches!(
        descriptor.value_type,
        ValueType::Interval {
            element: bumbledb_theory::schema::IntervalElement::I64,
        }
        | ValueType::FixedInterval {
            element: bumbledb_theory::schema::FixedIntervalElement::I64,
            ..
        }
    )
}

/// Whether this descriptor's interval arm uses tag 9 on the wire.
pub(crate) const fn interval_tag_f64(descriptor: &FieldDescriptor) -> bool {
    matches!(
        descriptor.value_type,
        ValueType::Interval {
            element: bumbledb_theory::schema::IntervalElement::F64,
        }
    )
}

pub(crate) const fn i64_word(value: i64) -> u64 {
    value.cast_unsigned() ^ (1 << 63)
}
