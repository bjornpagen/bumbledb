use bumbledb::schema::ValueType;
use bumbledb::Value;

use crate::sqlmap;

use super::PreparedFamily;

/// One timed sample: bind via the normative mapping, drain ALL rows with
/// typed reads on every column, return the row count (the harness's
/// black-box/work contract).
///
/// # Errors
///
/// `SQLite` errors, stringified.
pub fn sample(family: &mut PreparedFamily<'_>, params: &[Value]) -> Result<u64, String> {
    let bound: Vec<rusqlite::types::Value> = family
        .param_order
        .iter()
        .map(|p| sqlmap::to_sql_value(&params[usize::from(p.0)]))
        .collect();
    let mut rows = family
        .stmt
        .query(rusqlite::params_from_iter(bound))
        .map_err(|e| format!("query: {e}"))?;
    let mut count = 0u64;
    while let Some(row) = rows.next().map_err(|e| format!("step: {e}"))? {
        for (column, ty) in family.result_types.iter().enumerate() {
            let value = row.get_ref(column).map_err(|e| format!("read: {e}"))?;
            match ty {
                ValueType::Bool | ValueType::Enum { .. } | ValueType::U64 | ValueType::I64 => {
                    std::hint::black_box(value.as_i64().map_err(|e| format!("i64: {e}"))?);
                }
                ValueType::String => {
                    std::hint::black_box(value.as_str().map_err(|e| format!("str: {e}"))?);
                }
                ValueType::Bytes => {
                    std::hint::black_box(value.as_blob().map_err(|e| format!("blob: {e}"))?);
                }
            }
        }
        count += 1;
    }
    Ok(count)
}
