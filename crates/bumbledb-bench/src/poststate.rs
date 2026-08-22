//! "Verified by post-state comparison" is a fold both worlds reuse, never
//! per-family prose: scan the engine, `SELECT` the mirror in field-declaration
//! order, and judge the multisets through the same of the writebench pattern:
//! after a write lane runs on both twins,
use bumbledb::schema::Relation;
use bumbledb::{Db, RelationId, Value};
use rusqlite::Connection;

use crate::compare::{self, Answer, Owned};
use crate::sqlmap;

fn owned(value: &Value) -> Owned {
    match value {
        Value::Bool(v) => Owned::Bool(*v),
        Value::U64(v) => Owned::U64(*v),
        Value::I64(v) => Owned::I64(*v),
        Value::String(text) => Owned::Str(text.to_string()),
        Value::FixedBytes(raw) => Owned::Bytes(raw.to_vec()),
        Value::IntervalU64(interval) => Owned::IntervalU64(interval.start(), interval.end()),
        Value::IntervalI64(interval) => Owned::IntervalI64(interval.start(), interval.end()),
    }
}

/// # Errors
pub fn engine_rows<S>(db: &Db<S>, rel: RelationId) -> Result<Vec<Answer>, String> {
    let rows: Vec<Vec<Value>> = db
        .read(|snap| snap.scan(rel)?.collect())
        .map_err(|e| format!("engine scan: {e:?}"))?;
    Ok(rows
        .iter()
        .map(|row| row.iter().map(owned).collect())
        .collect())
}

/// # Errors
pub fn sqlite_rows(conn: &Connection, relation: &Relation) -> Result<Vec<Answer>, String> {
    let mut columns: Vec<String> = Vec::new();
    for field in relation.fields() {
        if field.value_type.is_interval() {
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
            let value = if let Some(element) = field.value_type.interval_element() {
                let start: rusqlite::types::Value = row.get(column).map_err(|e| e.to_string())?;
                let end: rusqlite::types::Value = row.get(column + 1).map_err(|e| e.to_string())?;
                column += 2;
                sqlmap::interval_from_sql(&start, &end, element)
                    .map_err(|e| format!("{}: {e}", field.name))?
            } else {
                let raw: rusqlite::types::Value = row.get(column).map_err(|e| e.to_string())?;
                column += 1;
                sqlmap::from_sql_value(&raw, &field.value_type)
                    .map_err(|e| format!("{}: {e}", field.name))?
            };
            answer.push(owned(&value));
        }
        out.push(answer);
    }
    Ok(out)
}

/// # Errors
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
