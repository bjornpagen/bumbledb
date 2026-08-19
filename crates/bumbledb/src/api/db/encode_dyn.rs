use crate::encoding::{FixedBytesValue, ValueRef};
use crate::error::{FactShapeError, Mismatch, Result};
use crate::ir::Value;
use bumbledb_theory::schema::{FieldDescriptor, FieldId, RelationId};

/// The one [`crate::schema::ValueMismatch`] → [`FactShapeError`] translation,
/// shared by every dynamic write/read surface (`insert_dyn`/`delete_dyn`/
/// `contains_dyn`/`get_dyn`, both transaction kinds).
pub(super) fn shape_mismatch(
    rel: RelationId,
    field: FieldId,
    mismatch: bumbledb_theory::schema::ValueMismatch,
) -> FactShapeError {
    match mismatch {
        bumbledb_theory::schema::ValueMismatch::Type => FactShapeError::TypeMismatch {
            relation: rel,
            field,
        },
    }
}

/// One arity-correct cell after shape parse. Strings stay `&str` until
/// apply interns them — the parse is the proof, not a discarded check.
#[derive(Clone, Copy)]
pub(super) enum ParsedCell<'a> {
    Bool(bool),
    U64(u64),
    I64(i64),
    IntervalU64(bumbledb_theory::Interval<u64>),
    IntervalI64(bumbledb_theory::Interval<i64>),
    FixedBytes(FixedBytesValue),
    String(&'a str),
}

/// A dynamic row that passed arity and type-kind. Apply only interns and
/// encodes.
pub(super) struct ParsedRow<'a> {
    cells: Box<[ParsedCell<'a>]>,
}

/// Parse one dynamic row: arity and type-kind. Strings remain intern-pending.
pub(super) fn parse_dyn_row<'a>(
    rel: RelationId,
    values: &'a [Value],
    fields: &[FieldDescriptor],
) -> Result<ParsedRow<'a>> {
    if values.len() != fields.len() {
        return Err(FactShapeError::ArityMismatch {
            relation: rel,
            mismatch: Mismatch {
                witnessed: values.len(),
                required: fields.len(),
            },
        }
        .into());
    }
    let mut cells = Vec::with_capacity(fields.len());
    for (idx, (value, field)) in values.iter().zip(fields).enumerate() {
        let field_id = FieldId(u16::try_from(idx).expect("field count fits u16"));
        if let Err(mismatch) = bumbledb_theory::schema::value_matches(value, &field.value_type) {
            return Err(shape_mismatch(rel, field_id, mismatch).into());
        }
        let cell = match value {
            Value::String(text) => ParsedCell::String(text),
            Value::Bool(v) => ParsedCell::Bool(*v),
            Value::U64(v) => ParsedCell::U64(*v),
            Value::I64(v) => ParsedCell::I64(*v),
            Value::IntervalU64(interval) => ParsedCell::IntervalU64(*interval),
            Value::IntervalI64(interval) => ParsedCell::IntervalI64(*interval),
            Value::FixedBytes(raw) => ParsedCell::FixedBytes(FixedBytesValue::new(raw)),
        };
        cells.push(cell);
    }
    Ok(ParsedRow {
        cells: cells.into_boxed_slice(),
    })
}

/// Intern pending strings and fill `refs`. `Ok(false)` = a resolve miss:
/// the fact cannot exist.
pub(super) fn intern_parsed_row(
    row: &ParsedRow<'_>,
    refs: &mut Vec<ValueRef>,
    mut resolve_str: impl FnMut(&str) -> Result<Option<crate::encoding::InternId>>,
) -> Result<bool> {
    refs.clear();
    for cell in &row.cells {
        let value_ref = match *cell {
            ParsedCell::String(text) => {
                let Some(id) = resolve_str(text)? else {
                    return Ok(false);
                };
                ValueRef::String(id)
            }
            ParsedCell::Bool(v) => ValueRef::Bool(v),
            ParsedCell::U64(v) => ValueRef::U64(v),
            ParsedCell::I64(v) => ValueRef::I64(v),
            ParsedCell::IntervalU64(interval) => ValueRef::IntervalU64(interval),
            ParsedCell::IntervalI64(interval) => ValueRef::IntervalI64(interval),
            ParsedCell::FixedBytes(raw) => ValueRef::bytes(raw.as_bytes()),
        };
        refs.push(value_ref);
    }
    Ok(true)
}
