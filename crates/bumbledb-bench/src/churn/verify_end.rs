//! The churn lane's three-way end gate — the write-verification pattern
//! extended by representation: the driver's [`LiveSet`] IS the naive
//! model of the churned relation, so end-state verification is one
//! multiset equality chain (model vs engine vs every `SQLite` mirror)
//! plus the store sweeper, never an extra checker.

use bumbledb::{Db, Value};

use crate::compare::{self, Owned};
use crate::schema::{Ledger, ids};

use super::engines::{OursLane, posting_values};
use super::ops::LiveSet;

/// One posting row into the canonical answer form — every `Posting`
/// cell is `U64` or `I64` by schema.
fn owned_row(row: &[Value]) -> compare::Answer {
    row.iter()
        .map(|value| match value {
            Value::U64(v) => Owned::U64(*v),
            Value::I64(v) => Owned::I64(*v),
            other => unreachable!("a Posting cell is U64 or I64, got {other:?}"),
        })
        .collect()
}

/// The engine's posting multiset, scanned whole through one snapshot.
///
/// # Errors
///
/// Engine errors, stringified.
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

/// One mirror's posting multiset, decoded through the normative value
/// mapping against the schema's field types.
///
/// # Errors
///
/// `SQLite` errors verbatim; a mapping mismatch naming the column.
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

/// The model's posting multiset — the [`LiveSet`]'s rows rendered
/// through the same dynamic form the mirror inserts.
#[must_use]
pub fn model_multiset(live: &LiveSet) -> Vec<compare::Answer> {
    live.rows()
        .iter()
        .map(|posting| owned_row(&posting_values(posting)))
        .collect()
}

/// The three-way end gate: (1) the model vs the engine (the `LiveSet`
/// model is normative), (2) the engine vs every mirror (the lane label
/// in the error), (3) the store sweeper — `verify_store` findings must
/// be EMPTY, closing the run.
///
/// # Errors
///
/// The first diverging multiset, rendered with its pair named; a
/// populated sweeper report, rendered whole.
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
    if !report.findings.is_empty() {
        return Err(format!(
            "churn store sweep found desyncs: {:?}",
            report.findings
        ));
    }
    Ok(())
}
