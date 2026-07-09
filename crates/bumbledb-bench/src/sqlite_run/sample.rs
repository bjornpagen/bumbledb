use bumbledb::schema::ValueType;
use bumbledb::Value;

use super::{bind_params, PreparedFamily};

/// One timed sample: bind via the normative mapping (interval params as
/// their two endpoint slots), drain ALL rows with typed reads on every
/// column — an interval find spans two INTEGER result columns — and
/// return the row count (the harness's black-box/work contract).
///
/// # Errors
///
/// `SQLite` errors, stringified.
pub fn sample(family: &mut PreparedFamily<'_>, params: &[Value]) -> Result<u64, String> {
    let bound = bind_params(&family.param_order, params);
    let mut rows = family
        .stmt
        .query(rusqlite::params_from_iter(bound))
        .map_err(|e| format!("query: {e}"))?;
    let mut count = 0u64;
    while let Some(row) = rows.next().map_err(|e| format!("step: {e}"))? {
        let mut column = 0usize;
        for ty in &family.result_types {
            match ty {
                ValueType::Bool | ValueType::Enum { .. } | ValueType::U64 | ValueType::I64 => {
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
                ValueType::Bytes => {
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
