//! The bumbledb ↔ `SQLite` mapping (docs/architecture/50-validation.md, 09): DDL from the
//! schema descriptors and the normative value mapping. One mapping, used
//! by the loader, the translator, and the runner — it cannot drift apart.
//!
//! Value mapping (normative, `docs/architecture/50-validation.md`):
//! Bool→INTEGER 0/1, Enum→INTEGER ordinal, U64→INTEGER (asserted < 2⁶³ —
//! the generator's axiom), I64→INTEGER, String→TEXT, Bytes→BLOB.

use bumbledb::schema::{ConstraintDescriptor, ValueType};
use bumbledb::{Schema, Value};

/// The family composites beyond unique/FK indexes (fairness: `SQLite` gets
/// every index the query families reward — see docs/architecture/50-validation.md
/// `FairnessCheck`, which asserts their presence).
pub const EXTRA_INDEXES: &[(&str, &str, &[&str])] = &[
    ("idx_posting_account_at", "Posting", &["account", "at"]),
    ("idx_posting_memo", "Posting", &["memo"]),
    ("idx_posting_instrument", "Posting", &["instrument"]),
];

fn sql_type(ty: &ValueType) -> &'static str {
    match ty {
        ValueType::Bool | ValueType::Enum { .. } | ValueType::U64 | ValueType::I64 => "INTEGER",
        ValueType::String => "TEXT",
        ValueType::Bytes => "BLOB",
    }
}

/// The single-field auto-unique on a serial field that becomes the
/// PRIMARY KEY (rowid alias — no separate index exists or is expected).
fn serial_pk(relation: &bumbledb::schema::Relation) -> Option<Box<str>> {
    relation.constraints().iter().find_map(|c| match c {
        ConstraintDescriptor::Unique { fields, .. } if fields.len() == 1 => {
            let field = &relation.fields()[usize::from(fields[0].0)];
            (field.generation == bumbledb::schema::Generation::Serial).then(|| field.name.clone())
        }
        _ => None,
    })
}

/// Every index the fairness contract requires, as `(table, index)` pairs
/// — the same walk [`ddl`] emits, so the contract and the DDL cannot
/// drift apart (`FairnessCheck`, docs/architecture/50-validation.md).
#[must_use]
pub fn expected_indexes(schema: &Schema) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for relation in schema.relations() {
        let pk = serial_pk(relation);
        for constraint in relation.constraints() {
            match constraint {
                ConstraintDescriptor::Unique { name, fields } => {
                    let covered_by_pk = match (&pk, fields.len()) {
                        (Some(pk), 1) => **pk == *relation.fields()[usize::from(fields[0].0)].name,
                        _ => false,
                    };
                    if !covered_by_pk {
                        out.push((
                            relation.name().to_owned(),
                            format!("uq_{}_{name}", relation.name()),
                        ));
                    }
                }
                ConstraintDescriptor::ForeignKey { name, .. } => {
                    out.push((
                        relation.name().to_owned(),
                        format!("ix_{}_{name}", relation.name()),
                    ));
                }
            }
        }
    }
    for (index, table, _) in EXTRA_INDEXES {
        out.push(((*table).to_owned(), (*index).to_owned()));
    }
    out
}

/// The full DDL: one STRICT table per relation (NOT NULL everywhere — no
/// nulls exist), a PRIMARY KEY on a lone serial auto-unique, UNIQUE
/// indexes for every other unique constraint, an index per FK field
/// list, plus [`EXTRA_INDEXES`].
#[must_use]
pub fn ddl(schema: &Schema) -> Vec<String> {
    let mut statements = Vec::new();
    for relation in schema.relations() {
        let mut columns: Vec<String> = Vec::new();
        for field in relation.fields() {
            columns.push(format!(
                "\"{}\" {} NOT NULL",
                field.name,
                sql_type(&field.value_type)
            ));
        }
        let serial_pk = serial_pk(relation);
        if let Some(pk) = &serial_pk {
            statements.push(format!(
                "CREATE TABLE \"{}\" ({}, PRIMARY KEY (\"{pk}\")) STRICT",
                relation.name(),
                columns.join(", "),
            ));
        } else {
            statements.push(format!(
                "CREATE TABLE \"{}\" ({}) STRICT",
                relation.name(),
                columns.join(", "),
            ));
        }
        for constraint in relation.constraints() {
            let field_names = |fields: &[bumbledb::FieldId]| {
                fields
                    .iter()
                    .map(|f| format!("\"{}\"", relation.fields()[usize::from(f.0)].name))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            match constraint {
                ConstraintDescriptor::Unique { name, fields } => {
                    // The PK already covers the lone serial auto-unique.
                    let covered_by_pk = match (&serial_pk, fields.len()) {
                        (Some(pk), 1) => **pk == *relation.fields()[usize::from(fields[0].0)].name,
                        _ => false,
                    };
                    if covered_by_pk {
                        continue;
                    }
                    statements.push(format!(
                        "CREATE UNIQUE INDEX \"uq_{}_{name}\" ON \"{}\" ({})",
                        relation.name(),
                        relation.name(),
                        field_names(fields),
                    ));
                }
                ConstraintDescriptor::ForeignKey { name, fields, .. } => {
                    statements.push(format!(
                        "CREATE INDEX \"ix_{}_{name}\" ON \"{}\" ({})",
                        relation.name(),
                        relation.name(),
                        field_names(fields),
                    ));
                }
            }
        }
    }
    for (index, table, columns) in EXTRA_INDEXES {
        let cols = columns
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        statements.push(format!("CREATE INDEX \"{index}\" ON \"{table}\" ({cols})"));
    }
    statements
}

/// One value into `SQLite`'s dynamic form (the normative mapping).
///
/// # Panics
///
/// On a `u64` at or above 2⁶³ — the generator's axiom guarantees the
/// corpus never produces one; anything else is a programmer error.
#[must_use]
pub fn to_sql_value(value: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sql;
    match value {
        Value::Bool(v) => Sql::Integer(i64::from(*v)),
        Value::Enum(ordinal) => Sql::Integer(i64::from(*ordinal)),
        Value::U64(v) => {
            Sql::Integer(i64::try_from(*v).expect("the SQLite mapping axiom: u64 < 2^63"))
        }
        Value::I64(v) => Sql::Integer(*v),
        Value::String(raw) => {
            Sql::Text(String::from_utf8(raw.to_vec()).expect("Value::String carries UTF-8"))
        }
        Value::Bytes(raw) => Sql::Blob(raw.to_vec()),
    }
}

/// One `SQLite` value back into the typed form, guided by the expected
/// column type (INTEGER is width-ambiguous without it).
///
/// # Errors
///
/// A message naming the mismatch (wrong storage class, negative INTEGER
/// for a `u64` column, out-of-range enum ordinal, non-UTF-8 TEXT).
pub fn from_sql_value(
    value: &rusqlite::types::Value,
    expected: &ValueType,
) -> Result<Value, String> {
    use rusqlite::types::Value as Sql;
    match (value, expected) {
        (Sql::Integer(v), ValueType::Bool) => match v {
            0 => Ok(Value::Bool(false)),
            1 => Ok(Value::Bool(true)),
            other => Err(format!("bool column holds {other}")),
        },
        (Sql::Integer(v), ValueType::Enum { variants }) => {
            let ordinal = u8::try_from(*v).map_err(|_| format!("enum ordinal {v}"))?;
            if usize::from(ordinal) < variants.len() {
                Ok(Value::Enum(ordinal))
            } else {
                Err(format!("enum ordinal {ordinal} out of range"))
            }
        }
        (Sql::Integer(v), ValueType::U64) => u64::try_from(*v)
            .map(Value::U64)
            .map_err(|_| format!("u64 column holds negative {v}")),
        (Sql::Integer(v), ValueType::I64) => Ok(Value::I64(*v)),
        (Sql::Text(text), ValueType::String) => Ok(Value::String(text.clone().into_bytes().into())),
        (Sql::Blob(raw), ValueType::Bytes) => Ok(Value::Bytes(raw.clone().into())),
        (got, want) => Err(format!("column class {got:?} for {want:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::schema;

    #[test]
    fn ddl_is_golden() {
        let statements = ddl(schema());
        // Spot-pin the load-bearing shapes; the full list is asserted by
        // count and a representative sample per statement kind.
        // 9 tables, 3 non-PK uniques (code, label, account_tag), 9 FK
        // indexes, 3 family composites.
        assert_eq!(
            statements.len(),
            9 + 3 + 9 + 3,
            "tables + uniques + fks + extras"
        );
        assert!(statements[0].starts_with(
            "CREATE TABLE \"Currency\" (\"id\" INTEGER NOT NULL, \"code\" TEXT NOT NULL, PRIMARY KEY (\"id\")) STRICT"
        ));
        assert!(statements
            .iter()
            .any(|s| s == "CREATE UNIQUE INDEX \"uq_Currency_code\" ON \"Currency\" (\"code\")"));
        assert!(statements
            .iter()
            .any(|s| s == "CREATE INDEX \"ix_Posting_account_fk\" ON \"Posting\" (\"account\")"));
        assert!(statements.iter().any(|s| s
            == "CREATE UNIQUE INDEX \"uq_AccountTag_account_tag\" ON \"AccountTag\" (\"account\", \"tag\")"));
        assert!(statements
            .iter()
            .any(|s| s
                == "CREATE INDEX \"idx_posting_account_at\" ON \"Posting\" (\"account\", \"at\")"));
    }

    #[test]
    fn values_round_trip_through_the_mapping() {
        let variants = ValueType::Enum {
            variants: ["A", "B", "C"].iter().map(|v| Box::from(*v)).collect(),
        };
        let cases: Vec<(Value, ValueType)> = vec![
            (Value::Bool(true), ValueType::Bool),
            (Value::Enum(2), variants),
            (Value::U64((1 << 63) - 1), ValueType::U64),
            (Value::I64(i64::MIN), ValueType::I64),
            (
                Value::String("héllo".as_bytes().to_vec().into()),
                ValueType::String,
            ),
            (Value::Bytes(vec![0, 255, 7].into()), ValueType::Bytes),
        ];
        for (value, ty) in cases {
            let sql = to_sql_value(&value);
            let back = from_sql_value(&sql, &ty).expect("round trip");
            assert_eq!(back, value);
        }
        // Mismatches are typed errors, not silent passes.
        assert!(from_sql_value(&rusqlite::types::Value::Integer(-1), &ValueType::U64).is_err());
        assert!(from_sql_value(&rusqlite::types::Value::Integer(9), &ValueType::Bool).is_err());
    }
}
