//! Canonical answers and multiset comparison (docs/architecture/60-validation.md):
//! one owned answer form both engines decode into, and one diff whose
//! mismatches are undeniable and debuggable — the verify layer's core.

use bumbledb::schema::ValueType;
use bumbledb::{AnswerValue, Answers, Value};

use crate::naive::ParamValue;
use crate::sqlmap;
use crate::translate::ParamSlot;

/// One canonical cell. Total order = the canonical multiset order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Owned {
    Bool(bool),
    U64(u64),
    I64(i64),
    Str(String),
    Bytes(Vec<u8>),
    IntervalU64(u64, u64),
    IntervalI64(i64, i64),
}

/// One canonical answer.
pub type Answer = Vec<Owned>;

/// Decodes bumbledb [`Answers`] into canonical answers (column types
/// from the prepared query's predicate — the answer-typing authority).
#[must_use]
pub fn from_answers(answers: &Answers, types: &[ValueType]) -> Vec<Answer> {
    answers
        .answers()
        .map(|answer| {
            (0..types.len())
                .map(|column| match answer.get(column) {
                    AnswerValue::Bool(v) => Owned::Bool(v),
                    AnswerValue::U64(v) => Owned::U64(v),
                    AnswerValue::I64(v) => Owned::I64(v),
                    AnswerValue::String(v) => Owned::Str(v.to_owned()),
                    AnswerValue::FixedBytes(v) => Owned::Bytes(v.to_vec()),
                    AnswerValue::IntervalU64(iv) => Owned::IntervalU64(iv.start(), iv.end()),
                    AnswerValue::IntervalI64(iv) => Owned::IntervalI64(iv.start(), iv.end()),
                })
                .collect()
        })
        .collect()
}

/// One engine [`Value`] in canonical form — the ONE `Value` → [`Owned`]
/// mapping, shared by the `SQLite` decode ([`from_sqlite`]) and the
/// keyed-get fact decode ([`from_fact`]).
fn owned_value(value: &Value) -> Result<Owned, String> {
    Ok(match value {
        Value::Bool(v) => Owned::Bool(*v),
        Value::U64(v) => Owned::U64(*v),
        Value::I64(v) => Owned::I64(*v),
        Value::String(raw) => {
            Owned::Str(String::from_utf8(raw.to_vec()).map_err(|_| "non-UTF-8 text".to_owned())?)
        }
        Value::FixedBytes(raw) => Owned::Bytes(raw.to_vec()),
        Value::IntervalU64(interval) => Owned::IntervalU64(interval.start(), interval.end()),
        Value::IntervalI64(interval) => Owned::IntervalI64(interval.start(), interval.end()),
        Value::AllenMask(_) => return Err("mask values are not results".to_owned()),
    })
}

/// Decodes one dynamic fact (owned engine [`Value`]s in field declaration
/// order — the keyed-get surface's answer shape) into a canonical answer.
///
/// # Errors
///
/// Non-UTF-8 text or a mask value, as a message naming the field —
/// neither exists in a decoded fact.
pub fn from_fact(fact: &[Value]) -> Result<Answer, String> {
    fact.iter()
        .enumerate()
        .map(|(field, value)| owned_value(value).map_err(|e| format!("field {field}: {e}")))
        .collect()
}

/// Executes a prepared `SQLite` statement with the given typed params and
/// decodes every answer into canonical form, guided by the expected column
/// types (the engine side already knows them — aggregate columns
/// included; an interval find spans two INTEGER result columns and
/// reassembles through the pair decode).
///
/// # Errors
///
/// `SQLite` errors verbatim; a mapping mismatch (wrong storage class,
/// negative INTEGER in a u64 column) as a message naming the column.
///
/// # Panics
///
/// On a set arg bound to a placeholder slot (a translator invariant).
pub fn from_sqlite(
    stmt: &mut rusqlite::Statement<'_>,
    param_order: &[ParamSlot],
    args: &[ParamValue],
    types: &[ValueType],
) -> Result<Vec<Answer>, String> {
    let bound = crate::sqlite_run::bind_args(param_order, args);
    let mut rows = stmt
        .query(rusqlite::params_from_iter(bound))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let mut canonical = Vec::with_capacity(types.len());
        let mut column = 0usize;
        for ty in types {
            let value = if let ValueType::Interval { element, .. } = ty {
                let start: rusqlite::types::Value = row.get(column).map_err(|e| e.to_string())?;
                let end: rusqlite::types::Value = row.get(column + 1).map_err(|e| e.to_string())?;
                column += 2;
                sqlmap::interval_from_sql(&start, &end, *element)
                    .map_err(|e| format!("columns {}-{}: {e}", column - 2, column - 1))?
            } else {
                let raw: rusqlite::types::Value = row.get(column).map_err(|e| e.to_string())?;
                column += 1;
                sqlmap::from_sql_value(&raw, ty)
                    .map_err(|e| format!("column {}: {e}", column - 1))?
            };
            canonical.push(owned_value(&value).map_err(|e| format!("column {}: {e}", column - 1))?);
        }
        out.push(canonical);
    }
    Ok(out)
}

/// How many exemplar answers a mismatch carries per side.
const EXEMPLARS: usize = 8;

/// A failed multiset comparison: sizes plus up to [`EXEMPLARS`] answers each
/// side has that the other lacks (multiset difference — duplicate-count
/// differences surface here too).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    pub ours_len: usize,
    pub theirs_len: usize,
    pub ours_only: Vec<Answer>,
    pub theirs_only: Vec<Answer>,
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "result multisets diverge: ours {} answers, theirs {} answers",
            self.ours_len, self.theirs_len
        )?;
        for answer in &self.ours_only {
            writeln!(f, "  ours only:   {answer:?}")?;
        }
        for answer in &self.theirs_only {
            writeln!(f, "  theirs only: {answer:?}")?;
        }
        Ok(())
    }
}

/// Multiset equality via sort + two-pointer diff, collecting exemplars.
///
/// # Errors
///
/// The [`Mismatch`] when the multisets differ.
pub fn multisets(mut ours: Vec<Answer>, mut theirs: Vec<Answer>) -> Result<(), Mismatch> {
    ours.sort();
    theirs.sort();
    if ours == theirs {
        return Ok(());
    }
    let mut mismatch = Mismatch {
        ours_len: ours.len(),
        theirs_len: theirs.len(),
        ours_only: Vec::new(),
        theirs_only: Vec::new(),
    };
    let (mut i, mut j) = (0, 0);
    while i < ours.len() || j < theirs.len() {
        let advance_ours = match (ours.get(i), theirs.get(j)) {
            (Some(a), Some(b)) if a == b => {
                i += 1;
                j += 1;
                continue;
            }
            (Some(a), Some(b)) => a < b,
            (mine, _) => mine.is_some(),
        };
        if advance_ours {
            if mismatch.ours_only.len() < EXEMPLARS {
                mismatch.ours_only.push(ours[i].clone());
            }
            i += 1;
        } else {
            if mismatch.theirs_only.len() < EXEMPLARS {
                mismatch.theirs_only.push(theirs[j].clone());
            }
            j += 1;
        }
    }
    Err(mismatch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(values: &[i64]) -> Answer {
        values.iter().map(|v| Owned::I64(*v)).collect()
    }

    #[test]
    fn equal_multisets_pass_shuffled() {
        let a = vec![answer(&[1, 2]), answer(&[3, 4]), answer(&[1, 2])];
        let b = vec![answer(&[3, 4]), answer(&[1, 2]), answer(&[1, 2])];
        assert!(multisets(a, b).is_ok());
    }

    #[test]
    fn a_one_answer_difference_lands_on_the_correct_side() {
        let a = vec![answer(&[1]), answer(&[2])];
        let b = vec![answer(&[1]), answer(&[3])];
        let mismatch = multisets(a, b).unwrap_err();
        assert_eq!(mismatch.ours_only, vec![answer(&[2])]);
        assert_eq!(mismatch.theirs_only, vec![answer(&[3])]);
    }

    #[test]
    fn duplicate_count_differences_are_detected() {
        let a = vec![answer(&[7]), answer(&[7])];
        let b = vec![answer(&[7])];
        let mismatch = multisets(a, b).unwrap_err();
        assert_eq!(mismatch.ours_len, 2);
        assert_eq!(mismatch.theirs_len, 1);
        assert_eq!(mismatch.ours_only, vec![answer(&[7])]);
        assert!(mismatch.theirs_only.is_empty());
    }

    #[test]
    fn the_display_is_golden() {
        let mismatch = multisets(vec![answer(&[1])], vec![answer(&[2])]).unwrap_err();
        assert_eq!(
            mismatch.to_string(),
            "result multisets diverge: ours 1 answers, theirs 1 answers\n  \
             ours only:   [I64(1)]\n  theirs only: [I64(2)]\n"
        );
    }

    #[test]
    fn sqlite_round_trips_all_six_types() {
        let conn = rusqlite::Connection::open_in_memory().expect("open");
        conn.execute_batch(
            "CREATE TABLE t (b INTEGER, e INTEGER, u INTEGER, i INTEGER, s TEXT, y BLOB)",
        )
        .expect("ddl");
        conn.execute("INSERT INTO t VALUES (1, 2, 42, -7, 'héllo', X'00FF')", [])
            .expect("insert");
        let types = vec![
            ValueType::Bool,
            ValueType::U64,
            ValueType::U64,
            ValueType::I64,
            ValueType::String,
            ValueType::FixedBytes { len: 2 },
        ];
        let mut stmt = conn.prepare("SELECT * FROM t").expect("prepare");
        let answers = from_sqlite(&mut stmt, &[], &[], &types).expect("decode");
        assert_eq!(
            answers,
            vec![vec![
                Owned::Bool(true),
                Owned::U64(2),
                Owned::U64(42),
                Owned::I64(-7),
                Owned::Str("héllo".to_owned()),
                Owned::Bytes(vec![0x00, 0xFF]),
            ]]
        );

        // Width confusion is a typed error, not a wrong pass: a negative
        // INTEGER read as u64 must refuse.
        let wrong = vec![
            ValueType::Bool,
            ValueType::U64,
            ValueType::U64,
            ValueType::U64, // the -7 column misdeclared
            ValueType::String,
            ValueType::FixedBytes { len: 2 },
        ];
        let mut stmt = conn.prepare("SELECT * FROM t").expect("prepare");
        let err = from_sqlite(&mut stmt, &[], &[], &wrong).unwrap_err();
        assert!(err.contains("column 3"), "{err}");
    }
}
