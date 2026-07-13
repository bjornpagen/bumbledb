use bumbledb::Value;
use bumbledb::schema::ValueType;

use crate::naive::ParamValue;

use super::{PreparedFamily, bind_args, bind_params};

/// One timed sample: bind via the normative mapping (interval params as
/// their two endpoint slots), drain ALL rows with typed reads on every
/// column — an interval find spans two INTEGER result columns — and
/// return the row count (the harness's black-box/work contract).
///
/// # Errors
///
/// `SQLite` errors, stringified.
pub fn sample(family: &mut PreparedFamily<'_>, params: &[Value]) -> Result<u64, String> {
    drain(family, bind_params(&family.param_order, params))
}

/// [`sample`] over a family draw (set element lists already live in the
/// re-rendered SQL; scalar positions bind through the slot order).
///
/// # Errors
///
/// `SQLite` errors, stringified.
pub fn sample_args(family: &mut PreparedFamily<'_>, draw: &[ParamValue]) -> Result<u64, String> {
    drain(family, bind_args(&family.param_order, draw))
}

fn drain(
    family: &mut PreparedFamily<'_>,
    bound: Vec<rusqlite::types::Value>,
) -> Result<u64, String> {
    let mut rows = family
        .stmt
        .query(rusqlite::params_from_iter(bound))
        .map_err(|e| format!("query: {e}"))?;
    let mut count = 0u64;
    while let Some(row) = rows.next().map_err(|e| format!("step: {e}"))? {
        let mut column = 0usize;
        for ty in &family.signature {
            match ty {
                ValueType::Bool | ValueType::U64 | ValueType::I64 => {
                    let value = row.get_ref(column).map_err(|e| format!("read: {e}"))?;
                    std::hint::black_box(value.as_i64().map_err(|e| format!("i64: {e}"))?);
                    column += 1;
                }
                ValueType::Interval { .. } => {
                    // The two half columns of one interval find.
                    for half in [column, column + 1] {
                        let value = row.get_ref(half).map_err(|e| format!("read: {e}"))?;
                        std::hint::black_box(value.as_i64().map_err(|e| format!("i64: {e}"))?);
                    }
                    column += 2;
                }
                ValueType::String => {
                    let value = row.get_ref(column).map_err(|e| format!("read: {e}"))?;
                    std::hint::black_box(value.as_str().map_err(|e| format!("str: {e}"))?);
                    column += 1;
                }
                ValueType::FixedBytes { .. } => {
                    let value = row.get_ref(column).map_err(|e| format!("read: {e}"))?;
                    std::hint::black_box(value.as_blob().map_err(|e| format!("blob: {e}"))?);
                    column += 1;
                }
            }
        }
        count += 1;
    }
    Ok(count)
}
