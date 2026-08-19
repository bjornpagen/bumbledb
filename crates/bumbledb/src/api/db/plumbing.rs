use super::{ReadInstance, RelationId, ValueRef};
use crate::encoding::InternId;
use crate::error::{CorruptionError, Error, FactShapeError, Result};

/// The typed path's fixed-width interval boundary: checks the host's
/// checked interval against the field's declared width — exactly the
/// `value_matches` rule (width equality and the Q2 ray bar; the width
/// is the type, so a wide or narrow value is a type mismatch) — and
/// wraps it as the one-word-encoding [`ValueRef`].
///
/// # Errors
///
/// [`FactShapeError::TypeMismatch`] when the interval's width is not the
/// declared `width` or its end is the domain ceiling (a ray).
pub fn fixed_interval_u64(
    relation: RelationId,
    field: bumbledb_theory::schema::FieldId,
    interval: bumbledb_theory::Interval<u64>,
    width: u64,
) -> Result<ValueRef> {
    if interval.end() - interval.start() == width && !interval.is_ray() {
        Ok(ValueRef::IntervalU64(interval))
    } else {
        Err(FactShapeError::TypeMismatch { relation, field }.into())
    }
}

/// The `i64` sibling of [`fixed_interval_u64`].
///
/// # Errors
///
/// As [`fixed_interval_u64`].
pub fn fixed_interval_i64(
    relation: RelationId,
    field: bumbledb_theory::schema::FieldId,
    interval: bumbledb_theory::Interval<i64>,
    width: u64,
) -> Result<ValueRef> {
    if interval.end().abs_diff(interval.start()) == width && !interval.is_ray() {
        Ok(ValueRef::IntervalI64(interval))
    } else {
        Err(FactShapeError::TypeMismatch { relation, field }.into())
    }
}

/// Resolves an intern id to a `&str` view of the committed dictionary
/// (decode boundary): mmap pages, transaction-stable by LMDB `CoW`. UTF-8
/// is validated here, without a copy (parse, don't validate).
///
/// # Errors
///
/// `Corruption` on a dangling id or non-UTF-8 stored bytes.
pub fn resolve_string<'a, S>(instance: &'a ReadInstance<'_, S>, id: InternId) -> Result<&'a str> {
    let raw = instance.core.source.catalog().dict_resolve(id)?;
    std::str::from_utf8(raw)
        .map_err(|_| Error::Corruption(CorruptionError::NonUtf8Intern(id.raw())))
}

/// Appends the canonical fact bytes for a codec encode.
pub fn encode_fact_for<S, C: super::CodecRead<S>>(
    context: &C,
    rel: RelationId,
    values: &[ValueRef],
    out: &mut Vec<u8>,
) {
    crate::encoding::encode_fact(values, context.schema().relation(rel).layout(), out);
}
