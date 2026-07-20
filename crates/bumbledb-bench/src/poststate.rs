//! The shared post-state comparator — the write-verification extension
//! of the writebench pattern: after a write lane runs on both twins,
//! post-state equality is ONE comparator over ONE canonical row form
//! ([`crate::compare::Answer`]), shared by every write family. "Verified
//! by post-state comparison" is a fold both worlds reuse, never
//! per-family prose: scan the engine, `SELECT` the mirror in
//! field-declaration order, and judge the multisets through the same
//! diff every read family already trusts ([`crate::compare::multisets`]).

use bumbledb::schema::{Relation, ValueType};
use bumbledb::{Db, RelationId, Value};
use rusqlite::Connection;

use crate::compare::{self, Answer, Owned};
use crate::sqlmap;

/// One stored [`Value`] into the canonical cell — total over all eight
/// arms; a mask is a typed error, never a row.
///
/// # Errors
///
/// `AllenMask` (mask values are comparison arguments, never stored
/// fields) and non-UTF-8 string payloads, named.
fn owned(value: &Value) -> Result<Owned, String> {
    match value {
        Value::Bool(v) => Ok(Owned::Bool(*v)),
        Value::U64(v) => Ok(Owned::U64(*v)),
        Value::I64(v) => Ok(Owned::I64(*v)),
        Value::String(raw) => String::from_utf8(raw.to_vec())
            .map(Owned::Str)
            .map_err(|_| "non-UTF-8 text".to_owned()),
        Value::FixedBytes(raw) => Ok(Owned::Bytes(raw.to_vec())),
        Value::IntervalU64(interval) => Ok(Owned::IntervalU64(interval.start(), interval.end())),
        Value::IntervalI64(interval) => Ok(Owned::IntervalI64(interval.start(), interval.end())),
        Value::AllenMask(_) => Err("mask values are never stored fields".to_owned()),
    }
}

/// One relation's full committed state on the engine, as canonical
/// answers: a snapshot scan decoded cell-by-cell through the total
/// [`Value`] → [`Owned`] map.
///
/// # Errors
///
/// Engine scan errors, stringified; a mask cell (impossible in a stored
/// relation), named.
pub fn engine_rows<S>(db: &Db<S>, rel: RelationId) -> Result<Vec<Answer>, String> {
    let rows: Vec<Vec<Value>> = db
        .read(|snap| snap.scan(rel)?.collect())
        .map_err(|e| format!("engine scan: {e:?}"))?;
    rows.iter()
        .map(|row| row.iter().map(owned).collect())
        .collect()
}

/// One relation's full committed state on the `SQLite` mirror, as
/// canonical answers: a `SELECT` of the quoted column list in
/// field-declaration order (an interval-typed field contributes its two
/// `_start`/`_end` columns — the [`sqlmap::schema_ddl`] column naming),
/// decoded per field through the normative mapping
/// ([`sqlmap::from_sql_value`] / [`sqlmap::interval_from_sql`]).
///
/// # Errors
///
/// `SQLite` errors, stringified; mapping mismatches with the field
/// named.
pub fn sqlite_rows(conn: &Connection, relation: &Relation) -> Result<Vec<Answer>, String> {
    let mut columns: Vec<String> = Vec::new();
    for field in relation.fields() {
        if matches!(field.value_type, ValueType::Interval { .. }) {
            columns.push(format!("\"{}_start\"", field.name));
            columns.push(format!("\"{}_end\"", field.name));
        } else {
            columns.push(format!("\"{}\"", field.name));
        }
    }
    let sql = format!("SELECT {} FROM \"{}\"", columns.join(", "), relation.name());
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let mut answer = Vec::with_capacity(relation.fields().len());
        let mut column = 0usize;
        for field in relation.fields() {
            let value = if let ValueType::Interval { element, .. } = &field.value_type {
                let start: rusqlite::types::Value = row.get(column).map_err(|e| e.to_string())?;
                let end: rusqlite::types::Value = row.get(column + 1).map_err(|e| e.to_string())?;
                column += 2;
                sqlmap::interval_from_sql(&start, &end, *element)
                    .map_err(|e| format!("{}: {e}", field.name))?
            } else {
                let raw: rusqlite::types::Value = row.get(column).map_err(|e| e.to_string())?;
                column += 1;
                sqlmap::from_sql_value(&raw, &field.value_type)
                    .map_err(|e| format!("{}: {e}", field.name))?
            };
            answer.push(owned(&value).map_err(|e| format!("{}: {e}", field.name))?);
        }
        out.push(answer);
    }
    Ok(out)
}

/// The post-state judgment: value-identical multiset agreement, with a
/// failure naming the world and relation — the twins performed the same
/// mutations, or the run is invalid.
///
/// # Errors
///
/// The [`compare::multisets`] mismatch, rendered under the world and
/// relation names.
pub fn assert_identical(
    world: &str,
    relation: &str,
    ours: Vec<Answer>,
    theirs: Vec<Answer>,
) -> Result<(), String> {
    compare::multisets(ours, theirs).map_err(|mismatch| {
        format!(
            "{world}/{relation}: POST-STATES DIVERGE — the twins did not perform \
             the same mutations\n{mismatch}"
        )
    })
}
