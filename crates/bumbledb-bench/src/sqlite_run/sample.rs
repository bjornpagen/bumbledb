use bumbledb::Value;
use bumbledb::schema::ValueType;

use crate::naive::ParamValue;

use super::{CapMs, CapOutcome, PreparedFamily, bind_args, bind_params, with_cap};

/// One timed sample: bind via the normative mapping (interval params as
/// their two endpoint slots), drain ALL rows with typed reads on every
/// column — an interval find spans two INTEGER result columns — and
/// return the row count (the harness's black-box/work contract).
///
/// # Errors
///
/// `SQLite` errors, stringified.
pub fn sample(family: &mut PreparedFamily<'_>, params: &[Value]) -> Result<u64, String> {
    drain_typed(family, bind_params(&family.param_order, params))
        .map_err(|e| format!("sample: {e}"))
}

/// [`sample`] over a family draw (set element lists already live in the
/// re-rendered SQL; scalar positions bind through the slot order).
///
/// # Errors
///
/// `SQLite` errors, stringified.
pub fn sample_args(family: &mut PreparedFamily<'_>, draw: &[ParamValue]) -> Result<u64, String> {
    drain_typed(family, bind_args(&family.param_order, draw)).map_err(|e| format!("sample: {e}"))
}

/// [`sample`] under the DNF cap ([`with_cap`]): the drain runs with the
/// progress handler installed, and an interrupt folds to
/// [`CapOutcome::Tripped`] instead of a fake count. `PreparedFamily`
/// borrows the `Connection` immutably, so the handler's `&Connection`
/// coexists with the statement borrow.
///
/// # Errors
///
/// `SQLite` errors other than the cap's interrupt, stringified.
pub fn sample_capped(
    family: &mut PreparedFamily<'_>,
    conn: &rusqlite::Connection,
    cap: CapMs,
    params: &[Value],
) -> Result<CapOutcome<u64>, String> {
    with_cap(conn, cap, || {
        drain_typed(family, bind_params(&family.param_order, params))
    })
}

/// The one drain, typed errors: the capped path must distinguish the
/// progress-handler interrupt from every other failure, so the error
/// stays `rusqlite::Error` here and stringifies only in the wrappers.
pub(crate) fn drain_typed(
    family: &mut PreparedFamily<'_>,
    bound: Vec<rusqlite::types::Value>,
) -> Result<u64, rusqlite::Error> {
    let mut rows = family.stmt.query(rusqlite::params_from_iter(bound))?;
    let mut count = 0u64;
    while let Some(row) = rows.next()? {
        let mut column = 0usize;
        for ty in &family.signature {
            match ty {
                ValueType::Bool | ValueType::U64 | ValueType::I64 => {
                    let value = row.get_ref(column)?;
                    std::hint::black_box(value.as_i64()?);
                    column += 1;
                }
                ValueType::Interval { .. } => {
                    // The two half columns of one interval find.
                    for half in [column, column + 1] {
                        let value = row.get_ref(half)?;
                        std::hint::black_box(value.as_i64()?);
                    }
                    column += 2;
                }
                ValueType::String => {
                    let value = row.get_ref(column)?;
                    std::hint::black_box(value.as_str()?);
                    column += 1;
                }
                ValueType::FixedBytes { .. } => {
                    let value = row.get_ref(column)?;
                    std::hint::black_box(value.as_blob()?);
                    column += 1;
                }
            }
        }
        count += 1;
    }
    Ok(count)
}
