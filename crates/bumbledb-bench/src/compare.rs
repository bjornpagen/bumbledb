//! Canonical results and multiset comparison (docs/architecture/60-validation.md):
//! one owned row form both engines decode into, and one diff whose
//! mismatches are undeniable and debuggable — the verify layer's core.

use bumbledb::schema::ValueType;
use bumbledb::{ResultBuffer, ResultValue, Value};

use crate::naive::ParamValue;
use crate::sqlmap;
use crate::translate::ParamSlot;

/// One canonical cell. Total order = the canonical multiset order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Owned {
    Bool(bool),
    U64(u64),
    I64(i64),
    Enum(u8),
    Str(String),
    Bytes(Vec<u8>),
    IntervalU64(u64, u64),
    IntervalI64(i64, i64),
}

/// One canonical row.
pub type Row = Vec<Owned>;

/// Decodes a bumbledb result buffer into canonical rows (column types
/// from `PreparedQuery::column_types`).
#[must_use]
pub fn from_buffer(buffer: &ResultBuffer, types: &[ValueType]) -> Vec<Row> {
    buffer
        .rows()
        .map(|row| {
            (0..types.len())
                .map(|column| match row.get(column) {
                    ResultValue::Bool(v) => Owned::Bool(v),
                    ResultValue::U64(v) => Owned::U64(v),
                    ResultValue::I64(v) => Owned::I64(v),
                    ResultValue::Enum(v) => Owned::Enum(v),
                    ResultValue::String(v) => Owned::Str(v.to_owned()),
                    ResultValue::FixedBytes(v) => Owned::Bytes(v.to_vec()),
                    ResultValue::IntervalU64(iv) => Owned::IntervalU64(iv.start(), iv.end()),
                    ResultValue::IntervalI64(iv) => Owned::IntervalI64(iv.start(), iv.end()),
                })
                .collect()
        })
        .collect()
}

/// Executes a prepared `SQLite` statement with the given typed params and
/// decodes every row into canonical form, guided by the expected column
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
) -> Result<Vec<Row>, String> {
    let bound = crate::sqlite_run::bind_args(param_order, args);
    let mut rows = stmt
        .query(rusqlite::params_from_iter(bound))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let mut canonical = Vec::with_capacity(types.len());
        let mut column = 0usize;
        for ty in types {
            let value = if let ValueType::Interval { element } = ty {
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
            canonical.push(match value {
                Value::Bool(v) => Owned::Bool(v),
                Value::U64(v) => Owned::U64(v),
                Value::I64(v) => Owned::I64(v),
                Value::Enum(v) => Owned::Enum(v),
                Value::String(raw) => Owned::Str(
                    String::from_utf8(raw.to_vec())
                        .map_err(|_| format!("column {}: non-UTF-8 text", column - 1))?,
                ),
                Value::FixedBytes(raw) => Owned::Bytes(raw.to_vec()),
                Value::IntervalU64(start, end) => Owned::IntervalU64(start, end),
                Value::IntervalI64(start, end) => Owned::IntervalI64(start, end),
                Value::AllenMask(_) => {
                    return Err(format!(
                        "column {}: mask values are not results",
                        column - 1
                    ))
                }
            });
        }
        out.push(canonical);
    }
    Ok(out)
}

/// How many exemplar rows a mismatch carries per side.
const EXEMPLARS: usize = 8;

/// A failed multiset comparison: sizes plus up to [`EXEMPLARS`] rows each
/// side has that the other lacks (multiset difference — duplicate-count
/// differences surface here too).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    pub ours_len: usize,
    pub theirs_len: usize,
    pub ours_only: Vec<Row>,
    pub theirs_only: Vec<Row>,
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "result multisets diverge: ours {} rows, theirs {} rows",
            self.ours_len, self.theirs_len
        )?;
        for row in &self.ours_only {
            writeln!(f, "  ours only:   {row:?}")?;
        }
        for row in &self.theirs_only {
            writeln!(f, "  theirs only: {row:?}")?;
        }
        Ok(())
    }
}

/// Multiset equality via sort + two-pointer diff, collecting exemplars.
///
/// # Errors
///
/// The [`Mismatch`] when the multisets differ.
pub fn multisets(mut ours: Vec<Row>, mut theirs: Vec<Row>) -> Result<(), Mismatch> {
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

    fn row(values: &[i64]) -> Row {
        values.iter().map(|v| Owned::I64(*v)).collect()
    }

    #[test]
    fn equal_multisets_pass_shuffled() {
        let a = vec![row(&[1, 2]), row(&[3, 4]), row(&[1, 2])];
        let b = vec![row(&[3, 4]), row(&[1, 2]), row(&[1, 2])];
        assert!(multisets(a, b).is_ok());
    }

    #[test]
    fn a_one_row_difference_lands_on_the_correct_side() {
        let a = vec![row(&[1]), row(&[2])];
        let b = vec![row(&[1]), row(&[3])];
        let mismatch = multisets(a, b).unwrap_err();
        assert_eq!(mismatch.ours_only, vec![row(&[2])]);
        assert_eq!(mismatch.theirs_only, vec![row(&[3])]);
    }

    #[test]
    fn duplicate_count_differences_are_detected() {
        let a = vec![row(&[7]), row(&[7])];
        let b = vec![row(&[7])];
        let mismatch = multisets(a, b).unwrap_err();
        assert_eq!(mismatch.ours_len, 2);
        assert_eq!(mismatch.theirs_len, 1);
        assert_eq!(mismatch.ours_only, vec![row(&[7])]);
        assert!(mismatch.theirs_only.is_empty());
    }

    #[test]
    fn the_display_is_golden() {
        let mismatch = multisets(vec![row(&[1])], vec![row(&[2])]).unwrap_err();
        assert_eq!(
            mismatch.to_string(),
            "result multisets diverge: ours 1 rows, theirs 1 rows\n  \
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
            ValueType::Enum {
                variants: ["A", "B", "C"].iter().map(|v| Box::from(*v)).collect(),
            },
            ValueType::U64,
            ValueType::I64,
            ValueType::String,
            ValueType::FixedBytes { len: 2 },
        ];
        let mut stmt = conn.prepare("SELECT * FROM t").expect("prepare");
        let rows = from_sqlite(&mut stmt, &[], &[], &types).expect("decode");
        assert_eq!(
            rows,
            vec![vec![
                Owned::Bool(true),
                Owned::Enum(2),
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
            ValueType::Enum {
                variants: ["A", "B", "C"].iter().map(|v| Box::from(*v)).collect(),
            },
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
