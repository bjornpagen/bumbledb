//! The bumbledb ↔ `SQLite` mapping (`docs/architecture/60-validation.md`
//! § value mapping, normative): DDL from the schema descriptors and the
//! typed value mapping. One mapping, used by the loader, the translator,
//! and the runner — it cannot drift apart.
//!
//! Value mapping: Bool→INTEGER 0/1, Enum→INTEGER ordinal, U64→INTEGER
//! (asserted < 2⁶³ — the generator's axiom), I64→INTEGER, String→TEXT,
//! Bytes→BLOB, and `Interval(E)` → **two INTEGER columns**
//! `<name>_start` / `<name>_end`. The halves cross the boundary as the
//! raw typed endpoints — never the engine's sign-flipped word encoding —
//! and the comparison decode reassembles `Value::IntervalU64` /
//! `Value::IntervalI64` from the pair ([`interval_from_sql`]).

use bumbledb::schema::{
    FieldDescriptor, Generation, IntervalElement, Relation, Resolved, StatementDescriptor,
    ValueType,
};
use bumbledb::{Schema, Value};

/// The SQL storage class of one scalar type. Intervals never reach here:
/// they split into two INTEGER columns first ([`field_columns`]).
fn sql_type(ty: &ValueType) -> &'static str {
    match ty {
        ValueType::Bool
        | ValueType::Enum { .. }
        | ValueType::U64
        | ValueType::I64
        | ValueType::Interval { .. } => "INTEGER",
        ValueType::String => "TEXT",
        ValueType::FixedBytes { .. } => "BLOB",
    }
}

/// The SQL column(s) of one field: scalars map to one column of the same
/// name; an `Interval` field splits into `<name>_start`, `<name>_end`.
fn field_columns(field: &FieldDescriptor) -> Vec<(String, &'static str)> {
    match &field.value_type {
        ValueType::Interval { .. } => vec![
            (format!("{}_start", field.name), "INTEGER"),
            (format!("{}_end", field.name), "INTEGER"),
        ],
        scalar => vec![(field.name.to_string(), sql_type(scalar))],
    }
}

/// The rowid-alias column: the relation's first `Fresh` field. Its
/// auto-key statement becomes the table's PRIMARY KEY — no separate
/// index exists or is expected.
fn fresh_column(relation: &Relation) -> Option<&str> {
    relation
        .fields()
        .iter()
        .find(|field| field.generation == Generation::Fresh)
        .map(|field| &*field.name)
}

/// One index the fairness contract expects, beyond the PRIMARY KEY.
struct IndexSpec {
    table: String,
    name: String,
    /// A scalar key statement's index enforces its one-row rule (the SQL
    /// `UNIQUE` kind); everything else is a plain probe index.
    key: bool,
    columns: Vec<String>,
}

/// The statement-derived index plan — one walk shared by [`schema_ddl`]
/// and [`expected_indexes`], so the DDL and the contract cannot drift
/// apart. A scalar key statement (functionality) gets a UNIQUE index
/// (the lone-fresh auto-key is covered by the PRIMARY KEY and skipped);
/// a pointwise key gets the composite `(scalars..., start, end)` index —
/// the best SQL can do, the judgment itself being the naive lane's
/// ([`crate::translate::sqlite_expressible`]); a containment source gets
/// a plain index over its projection. Index names carry the statement id
/// (statements are anonymous — materialized order is their identity).
fn index_plan(schema: &Schema) -> Vec<IndexSpec> {
    let mut plan = Vec::new();
    for (sid, statement) in schema.statements().iter().enumerate() {
        match &statement.descriptor {
            StatementDescriptor::Functionality {
                relation,
                projection,
            } => {
                let rel = schema.relation(*relation);
                let covered_by_rowid = projection.len() == 1
                    && fresh_column(rel) == Some(&*rel.fields()[usize::from(projection[0].0)].name);
                if covered_by_rowid {
                    continue;
                }
                let key = matches!(
                    statement.resolved,
                    Resolved::Functionality {
                        interval_position: None
                    }
                );
                plan.push(IndexSpec {
                    table: rel.name().to_owned(),
                    name: format!("{}_{}_s{sid}", if key { "uq" } else { "ix" }, rel.name()),
                    key,
                    columns: projection
                        .iter()
                        .flat_map(|field| {
                            field_columns(&rel.fields()[usize::from(field.0)])
                                .into_iter()
                                .map(|(name, _)| name)
                        })
                        .collect(),
                });
            }
            StatementDescriptor::Containment { source, .. } => {
                let rel = schema.relation(source.relation);
                plan.push(IndexSpec {
                    table: rel.name().to_owned(),
                    name: format!("ix_{}_s{sid}", rel.name()),
                    key: false,
                    columns: source
                        .projection
                        .iter()
                        .flat_map(|field| {
                            field_columns(&rel.fields()[usize::from(field.0)])
                                .into_iter()
                                .map(|(name, _)| name)
                        })
                        .collect(),
                });
            }
        }
    }
    plan
}

/// Every statement-derived index the fairness contract requires, as
/// `(table, index)` pairs — the same walk [`schema_ddl`] emits
/// (`FairnessCheck`, `docs/architecture/60-validation.md`). The
/// family-owned composites live beside the families
/// (`crate::families::expected_indexes`).
#[must_use]
pub fn expected_indexes(schema: &Schema) -> Vec<(String, String)> {
    index_plan(schema)
        .into_iter()
        .map(|spec| (spec.table, spec.name))
        .collect()
}

/// The ledger DDL: [`schema_ddl`] plus the family-owned index registry
/// (`crate::families::index_ddl` — the honest opponent gets every index
/// the query families reward).
#[must_use]
pub fn ddl(schema: &Schema) -> Vec<String> {
    let mut statements = schema_ddl(schema);
    statements.extend(crate::families::index_ddl());
    statements
}

/// The schema-derived DDL: one STRICT table per relation (NOT NULL
/// everywhere — no nulls exist; interval fields split into their half
/// columns), a PRIMARY KEY on the lone fresh auto-key, then the
/// statement-derived indexes ([`index_plan`]). The scenario loaders
/// enter here (each scenario carries its own predicate-column indexes).
#[must_use]
pub fn schema_ddl(schema: &Schema) -> Vec<String> {
    let mut statements = Vec::new();
    for relation in schema.relations() {
        let mut columns: Vec<String> = Vec::new();
        for field in relation.fields() {
            for (name, sql_ty) in field_columns(field) {
                columns.push(format!("\"{name}\" {sql_ty} NOT NULL"));
            }
        }
        if let Some(alias) = fresh_column(relation) {
            statements.push(format!(
                "CREATE TABLE \"{}\" ({}, PRIMARY KEY (\"{alias}\")) STRICT",
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
    }
    for spec in index_plan(schema) {
        let cols = spec
            .columns
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        statements.push(format!(
            "CREATE {}INDEX \"{}\" ON \"{}\" ({cols})",
            if spec.key { "UNIQUE " } else { "" },
            spec.name,
            spec.table,
        ));
    }
    statements
}

/// The positional INSERT for one relation — the placeholder count
/// follows the split column count (an interval field contributes two).
#[must_use]
pub fn insert_sql(relation: &Relation) -> String {
    let count: usize = relation
        .fields()
        .iter()
        .map(|field| {
            if matches!(field.value_type, ValueType::Interval { .. }) {
                2
            } else {
                1
            }
        })
        .sum();
    let placeholders = (1..=count)
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "INSERT INTO \"{}\" VALUES ({placeholders})",
        relation.name()
    )
}

/// One scalar value into `SQLite`'s dynamic form (the normative mapping).
///
/// # Panics
///
/// On a `u64` at or above 2⁶³ — the generator's axiom guarantees the
/// corpus never produces one — and on an interval, which maps to two
/// columns ([`to_sql_row`] / [`interval_halves`] own the split).
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
        Value::FixedBytes(raw) => Sql::Blob(raw.to_vec()),
        Value::IntervalU64(..) | Value::IntervalI64(..) => {
            panic!("an interval maps to two columns — split through interval_halves")
        }
        Value::AllenMask(_) => panic!("mask values are comparison arguments, never columns"),
    }
}

/// An interval's two INTEGER halves — the raw typed endpoints, never the
/// engine's sign-flipped word encoding (u64 halves under the same `< 2⁶³`
/// axiom as scalar u64).
///
/// # Panics
///
/// On a scalar value, or a u64 endpoint at or above 2⁶³.
#[must_use]
pub fn interval_halves(value: &Value) -> (rusqlite::types::Value, rusqlite::types::Value) {
    use rusqlite::types::Value as Sql;
    match value {
        Value::IntervalU64(start, end) => (
            Sql::Integer(i64::try_from(*start).expect("the SQLite mapping axiom: u64 < 2^63")),
            Sql::Integer(i64::try_from(*end).expect("the SQLite mapping axiom: u64 < 2^63")),
        ),
        Value::IntervalI64(start, end) => (Sql::Integer(*start), Sql::Integer(*end)),
        scalar => panic!("interval_halves on a scalar {scalar:?}"),
    }
}

/// One decoded fact row into positional SQL values — the insert path's
/// side of the interval split (pairs with [`insert_sql`]).
#[must_use]
pub fn to_sql_row(fact: &[Value]) -> Vec<rusqlite::types::Value> {
    let mut out = Vec::with_capacity(fact.len());
    for value in fact {
        match value {
            Value::IntervalU64(..) | Value::IntervalI64(..) => {
                let (start, end) = interval_halves(value);
                out.push(start);
                out.push(end);
            }
            scalar => out.push(to_sql_value(scalar)),
        }
    }
    out
}

/// One `SQLite` value back into the typed scalar form, guided by the
/// expected column type (INTEGER is width-ambiguous without it).
///
/// # Errors
///
/// A message naming the mismatch (wrong storage class, negative INTEGER
/// for a `u64` column, out-of-range enum ordinal, non-UTF-8 TEXT, an
/// interval type — which spans two columns and decodes through
/// [`interval_from_sql`]).
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
        (Sql::Blob(raw), ValueType::FixedBytes { .. }) => Ok(Value::FixedBytes(raw.clone().into())),
        (_, ValueType::Interval { .. }) => {
            Err("an interval spans two columns — decode through interval_from_sql".to_owned())
        }
        (got, want) => Err(format!("column class {got:?} for {want:?}")),
    }
}

/// The comparison decode's interval reassembly: the two INTEGER half
/// columns back into the typed value.
///
/// # Errors
///
/// A message naming the mismatch (non-INTEGER storage class, a negative
/// half for a U64 element, or `start >= end` — the stored invariant, so
/// a violating pair is corrupt data, not a value).
pub fn interval_from_sql(
    start: &rusqlite::types::Value,
    end: &rusqlite::types::Value,
    element: IntervalElement,
) -> Result<Value, String> {
    use rusqlite::types::Value as Sql;
    let (Sql::Integer(start), Sql::Integer(end)) = (start, end) else {
        return Err(format!("interval columns hold {start:?}, {end:?}"));
    };
    if start >= end {
        return Err(format!("interval columns hold start {start} >= end {end}"));
    }
    match element {
        IntervalElement::U64 => Ok(Value::IntervalU64(
            u64::try_from(*start).map_err(|_| format!("u64 interval start holds {start}"))?,
            u64::try_from(*end).map_err(|_| format!("u64 interval end holds {end}"))?,
        )),
        IntervalElement::I64 => Ok(Value::IntervalI64(*start, *end)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::schema::{RelationDescriptor, SchemaDescriptor, Side};
    use bumbledb::{FieldId, RelationId};

    fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
        FieldDescriptor {
            name: name.into(),
            value_type,
            generation: Generation::None,
        }
    }

    fn fresh(name: &str) -> FieldDescriptor {
        FieldDescriptor {
            name: name.into(),
            value_type: ValueType::U64,
            generation: Generation::Fresh,
        }
    }

    /// A miniature of the ledger's statement shapes: fresh auto-keys
    /// (the PRIMARY KEYs), a declared scalar key, two containments, a
    /// pointwise key over an i64 interval, and a keyless relation with a
    /// u64 interval for the round trip.
    fn mini_schema() -> Schema {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Account".into(),
                    fields: vec![fresh("id"), field("code", ValueType::String)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Org".into(),
                    fields: vec![fresh("id")],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Mandate".into(),
                    fields: vec![
                        field("account", ValueType::U64),
                        field("org", ValueType::U64),
                        field(
                            "active",
                            ValueType::Interval {
                                element: IntervalElement::I64,
                            },
                        ),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Span".into(),
                    fields: vec![
                        field("id", ValueType::U64),
                        field(
                            "u",
                            ValueType::Interval {
                                element: IntervalElement::U64,
                            },
                        ),
                    ],
                },
            ],
            statements: vec![
                StatementDescriptor::Functionality {
                    relation: RelationId(0),
                    projection: Box::new([FieldId(1)]),
                },
                StatementDescriptor::Containment {
                    source: Side {
                        relation: RelationId(2),
                        projection: Box::new([FieldId(0)]),
                        selection: Box::new([]),
                    },
                    target: Side {
                        relation: RelationId(0),
                        projection: Box::new([FieldId(0)]),
                        selection: Box::new([]),
                    },
                },
                StatementDescriptor::Containment {
                    source: Side {
                        relation: RelationId(2),
                        projection: Box::new([FieldId(1)]),
                        selection: Box::new([]),
                    },
                    target: Side {
                        relation: RelationId(1),
                        projection: Box::new([FieldId(0)]),
                        selection: Box::new([]),
                    },
                },
                StatementDescriptor::Functionality {
                    relation: RelationId(2),
                    projection: Box::new([FieldId(0), FieldId(2)]),
                },
            ],
        }
        .validate()
        .expect("the mini schema validates")
    }

    /// The DDL golden, byte-pinned: split interval columns, the PRIMARY
    /// KEY on the fresh auto-key (s0/s1 emit no index), a UNIQUE index
    /// for the declared scalar key, plain indexes for the containment
    /// sources, and the pointwise key's composite `(scalar, start, end)`.
    #[test]
    fn ddl_is_golden() {
        let schema = mini_schema();
        assert_eq!(
            schema_ddl(&schema),
            vec![
                "CREATE TABLE \"Account\" (\"id\" INTEGER NOT NULL, \"code\" TEXT NOT NULL, PRIMARY KEY (\"id\")) STRICT",
                "CREATE TABLE \"Org\" (\"id\" INTEGER NOT NULL, PRIMARY KEY (\"id\")) STRICT",
                "CREATE TABLE \"Mandate\" (\"account\" INTEGER NOT NULL, \"org\" INTEGER NOT NULL, \"active_start\" INTEGER NOT NULL, \"active_end\" INTEGER NOT NULL) STRICT",
                "CREATE TABLE \"Span\" (\"id\" INTEGER NOT NULL, \"u_start\" INTEGER NOT NULL, \"u_end\" INTEGER NOT NULL) STRICT",
                "CREATE UNIQUE INDEX \"uq_Account_s2\" ON \"Account\" (\"code\")",
                "CREATE INDEX \"ix_Mandate_s3\" ON \"Mandate\" (\"account\")",
                "CREATE INDEX \"ix_Mandate_s4\" ON \"Mandate\" (\"org\")",
                "CREATE INDEX \"ix_Mandate_s5\" ON \"Mandate\" (\"account\", \"active_start\", \"active_end\")",
            ]
        );
        // The contract walk agrees with the DDL walk by construction.
        assert_eq!(
            expected_indexes(&schema)[..4],
            [
                ("Account".to_owned(), "uq_Account_s2".to_owned()),
                ("Mandate".to_owned(), "ix_Mandate_s3".to_owned()),
                ("Mandate".to_owned(), "ix_Mandate_s4".to_owned()),
                ("Mandate".to_owned(), "ix_Mandate_s5".to_owned()),
            ]
        );
        assert_eq!(
            insert_sql(schema.relation(RelationId(2))),
            "INSERT INTO \"Mandate\" VALUES (?1, ?2, ?3, ?4)",
            "the placeholder count follows the split"
        );
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
            (
                Value::FixedBytes(vec![0, 255, 7].into()),
                ValueType::FixedBytes { len: 3 },
            ),
        ];
        for (value, ty) in cases {
            let sql = to_sql_value(&value);
            let back = from_sql_value(&sql, &ty).expect("round trip");
            assert_eq!(back, value);
        }
        // Mismatches are typed errors, not silent passes.
        assert!(from_sql_value(&rusqlite::types::Value::Integer(-1), &ValueType::U64).is_err());
        assert!(from_sql_value(&rusqlite::types::Value::Integer(9), &ValueType::Bool).is_err());
        // An interval type never decodes from one column.
        assert!(from_sql_value(
            &rusqlite::types::Value::Integer(0),
            &ValueType::Interval {
                element: IntervalElement::I64
            }
        )
        .is_err());
    }

    #[test]
    fn interval_halves_reassemble_through_the_pair_decode() {
        use rusqlite::types::Value as Sql;
        for value in [
            Value::IntervalI64(i64::MIN, i64::MAX),
            Value::IntervalI64(-5, 9),
            Value::IntervalU64(0, (1 << 63) - 1),
            Value::IntervalU64(5, 6),
        ] {
            let (start, end) = interval_halves(&value);
            let element = match value {
                Value::IntervalU64(..) => IntervalElement::U64,
                _ => IntervalElement::I64,
            };
            assert_eq!(interval_from_sql(&start, &end, element), Ok(value));
        }
        // Corrupt pairs are named errors: reversed bounds, a negative
        // half under a U64 element, a wrong storage class.
        assert!(
            interval_from_sql(&Sql::Integer(5), &Sql::Integer(5), IntervalElement::I64).is_err()
        );
        assert!(
            interval_from_sql(&Sql::Integer(-1), &Sql::Integer(4), IntervalElement::U64).is_err()
        );
        assert!(interval_from_sql(
            &Sql::Text("3".to_owned()),
            &Sql::Integer(4),
            IntervalElement::I64
        )
        .is_err());
    }

    /// The boundary round trip: interval facts inserted through the DDL
    /// split re-read as equal `Value::IntervalU64`/`IntervalI64` —
    /// boundary endpoints, negative starts, and `start + 1 == end`
    /// minimal intervals included.
    #[test]
    fn intervals_round_trip_through_sqlite() {
        let schema = mini_schema();
        let conn = rusqlite::Connection::open_in_memory().expect("open");
        for statement in schema_ddl(&schema) {
            conn.execute(&statement, []).expect("ddl");
        }
        let mandates = [
            vec![
                Value::U64(1),
                Value::U64(1),
                Value::IntervalI64(i64::MIN, i64::MAX),
            ],
            vec![Value::U64(2), Value::U64(1), Value::IntervalI64(-5, 9)],
            vec![Value::U64(3), Value::U64(1), Value::IntervalI64(-9, -8)],
        ];
        let spans = [
            vec![Value::U64(1), Value::IntervalU64(0, 1)],
            vec![Value::U64(2), Value::IntervalU64(0, (1 << 63) - 1)],
            vec![Value::U64(3), Value::IntervalU64(5, 6)],
        ];
        let mandate = schema.relation(RelationId(2));
        let span = schema.relation(RelationId(3));
        for fact in &mandates {
            conn.execute(
                &insert_sql(mandate),
                rusqlite::params_from_iter(to_sql_row(fact)),
            )
            .expect("insert");
        }
        for fact in &spans {
            conn.execute(
                &insert_sql(span),
                rusqlite::params_from_iter(to_sql_row(fact)),
            )
            .expect("insert");
        }
        let read_back = |sql: &str, element: IntervalElement| -> Vec<Value> {
            let mut stmt = conn.prepare(sql).expect("prepare");
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, rusqlite::types::Value>(0)?,
                        row.get::<_, rusqlite::types::Value>(1)?,
                    ))
                })
                .expect("query");
            rows.map(|pair| {
                let (start, end) = pair.expect("row");
                interval_from_sql(&start, &end, element).expect("reassembles")
            })
            .collect()
        };
        assert_eq!(
            read_back(
                "SELECT \"active_start\", \"active_end\" FROM \"Mandate\" ORDER BY \"account\"",
                IntervalElement::I64,
            ),
            mandates.iter().map(|f| f[2].clone()).collect::<Vec<_>>()
        );
        assert_eq!(
            read_back(
                "SELECT \"u_start\", \"u_end\" FROM \"Span\" ORDER BY \"id\"",
                IntervalElement::U64,
            ),
            spans.iter().map(|f| f[1].clone()).collect::<Vec<_>>()
        );
    }
}
