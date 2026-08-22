use bumbledb::{Db, Value};

use crate::compare::{self, Owned};
use crate::schema::{Ledger, ids};

use super::engines::{OursLane, posting_values};
use super::ops::LiveSet;

fn owned_row(row: &[Value]) -> compare::Answer {
    row.iter()
        .map(|value| match value {
            Value::U64(v) => Owned::U64(*v),
            Value::I64(v) => Owned::I64(*v),
            other => unreachable!("a Posting cell is U64 or I64, got {other:?}"),
        })
        .collect()
}

/// # Errors
pub fn posting_multiset_ours(db: &Db<Ledger>) -> Result<Vec<compare::Answer>, String> {
    db.read(|snap| {
        let mut out = Vec::new();
        for row in snap.scan(ids::POSTING)? {
            out.push(owned_row(&row?));
        }
        Ok(out)
    })
    .map_err(|e| format!("churn end scan: {e:?}"))
}

/// # Errors
pub fn posting_multiset_sqlite(
    conn: &rusqlite::Connection,
) -> Result<Vec<compare::Answer>, String> {
    let relation = crate::schema::schema().relation(ids::POSTING);
    let mut stmt = conn
        .prepare("SELECT * FROM \"Posting\"")
        .map_err(|e| format!("churn mirror end scan: {e}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("churn mirror end scan: {e}"))?;
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("churn mirror end scan: {e}"))?
    {
        let mut answer = Vec::with_capacity(relation.fields().len());
        for (index, field) in relation.fields().iter().enumerate() {
            let raw: rusqlite::types::Value = row
                .get(index)
                .map_err(|e| format!("churn mirror end scan: {e}"))?;
            let value = crate::sqlmap::from_sql_value(&raw, &field.value_type)
                .map_err(|e| format!("churn mirror column {index}: {e}"))?;
            answer.push(match value {
                Value::U64(v) => Owned::U64(v),
                Value::I64(v) => Owned::I64(v),
                other => unreachable!("a Posting cell is U64 or I64, got {other:?}"),
            });
        }
        out.push(answer);
    }
    Ok(out)
}

#[must_use]
pub fn model_multiset(live: &LiveSet) -> Vec<compare::Answer> {
    live.rows()
        .iter()
        .map(|posting| owned_row(&posting_values(posting)))
        .collect()
}

/// # Errors
pub fn assert_end_state(
    ours: &OursLane,
    mirrors: &[(&str, &rusqlite::Connection)],
    live: &LiveSet,
) -> Result<(), String> {
    let engine = posting_multiset_ours(&ours.db)?;
    compare::multisets(model_multiset(live), engine.clone())
        .map_err(|m| format!("churn end gate (model vs ours): {m}"))?;
    for (label, conn) in mirrors {
        let mirror = posting_multiset_sqlite(conn)?;
        compare::multisets(engine.clone(), mirror)
            .map_err(|m| format!("churn end gate (ours vs sqlite-{label}): {m}"))?;
    }
    let report = ours
        .db
        .verify_store()
        .map_err(|e| format!("churn store sweep: {e:?}"))?;
    if !report.findings().is_empty() {
        return Err(format!(
            "churn store sweep found desyncs: {:?}",
            report.findings()
        ));
    }
    Ok(())
}
