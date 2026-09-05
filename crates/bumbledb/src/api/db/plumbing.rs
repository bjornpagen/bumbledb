//! `schema!` expansion plumbing over canonical values. The generated
//! `Fact::append_values` impls call these; nothing here is documented API.

use crate::Value;
use crate::error::{FactShapeError, Result};
use bumbledb_theory::Interval;
use bumbledb_theory::schema::{FieldId, RelationId};

/// The typed path's fixed-width interval boundary: checks the host's
/// checked interval against the field's declared width — exactly the
/// `value_matches` rule (width equality and the ray bar; the width is the
/// type, so a wide or narrow value is a type mismatch) — and wraps it as a
/// canonical [`Value`].
/// # Errors
/// [`FactShapeError::TypeMismatch`] when the interval's width is not the
/// declared `width` or its end is the domain ceiling (a ray).
pub fn fixed_interval_u64(
    relation: RelationId,
    field: FieldId,
    interval: Interval<u64>,
    width: u64,
) -> Result<Value> {
    if interval.end() - interval.start() == width && !interval.is_ray() {
        Ok(Value::IntervalU64(interval))
    } else {
        Err(FactShapeError::TypeMismatch { relation, field }.into())
    }
}

/// The `i64` sibling of [`fixed_interval_u64`].
/// # Errors
/// As [`fixed_interval_u64`].
pub fn fixed_interval_i64(
    relation: RelationId,
    field: FieldId,
    interval: Interval<i64>,
    width: u64,
) -> Result<Value> {
    if interval.end().abs_diff(interval.start()) == width && !interval.is_ray() {
        Ok(Value::IntervalI64(interval))
    } else {
        Err(FactShapeError::TypeMismatch { relation, field }.into())
    }
}
