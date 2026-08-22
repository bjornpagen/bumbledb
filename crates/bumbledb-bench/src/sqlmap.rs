use bumbledb::schema::{
    FieldDescriptor, Generation, IntervalElement, Relation, StatementId, StatementView, ValueType,
};
use bumbledb::{Schema, Value};

fn sql_type(ty: &ValueType) -> &'static str {
    match ty {
        ValueType::Bool
        | ValueType::U64
        | ValueType::I64
        | ValueType::Interval { .. }
        | ValueType::FixedInterval { .. } => "INTEGER",
        ValueType::String => "TEXT",
        ValueType::FixedBytes { .. } => "BLOB",
    }
}

pub(crate) fn field_columns(field: &FieldDescriptor) -> Vec<(String, &'static str)> {
    match &field.value_type {
        ValueType::Interval { .. } | ValueType::FixedInterval { .. } => vec![
            (format!("{}_start", field.name), "INTEGER"),
            (format!("{}_end", field.name), "INTEGER"),
        ],
        scalar => vec![(field.name.to_string(), sql_type(scalar))],
    }
}

fn rowid_alias(relation: &Relation) -> Option<&str> {
    if relation.body().closed_rows().is_some() {
        return relation.fields().first().map(|field| &*field.name);
    }
    relation
        .fields()
        .iter()
        .find(|field| field.generation == Generation::Fresh)
        .map(|field| &*field.name)
}

struct IndexSpec {
    table: String,
    name: String,

    key: bool,
    columns: Vec<String>,
}

fn index_plan(schema: &Schema) -> Vec<IndexSpec> {
    let mut plan = Vec::new();
    let statement_count =
        schema.keys().len() + schema.containments().len() + schema.capacities().len();
    for sid in 0..statement_count {
        let id = StatementId(u16::try_from(sid).expect("statement count fits u16"));
        match schema.statement(id) {
            StatementView::Key(_, statement) => {
                let rel = schema.relation(statement.relation);

                let covered_by_rowid = statement.projection.len() == 1
                    && rowid_alias(rel)
                        == Some(&*rel.fields()[usize::from(statement.projection[0].0)].name);
                if covered_by_rowid {
                    continue;
                }
                let key = !statement.form().is_pointwise();
                plan.push(IndexSpec {
                    table: rel.name().to_owned(),
                    name: format!("{}_{}_s{sid}", if key { "uq" } else { "ix" }, rel.name()),
                    key,
                    columns: statement
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
            StatementView::Containment(_, statement) => {
                let rel = schema.relation(statement.source.relation);

                if rel.body().closed_rows().is_some() {
                    continue;
                }
                plan.push(IndexSpec {
                    table: rel.name().to_owned(),
                    name: format!("ix_{}_s{sid}", rel.name()),
                    key: false,
                    columns: statement
                        .source
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

            StatementView::Capacity(..) => {}
        }
    }
    plan
}

#[must_use]
pub fn expected_indexes(schema: &Schema) -> Vec<(String, String)> {
    index_plan(schema)
        .into_iter()
        .map(|spec| (spec.table, spec.name))
        .collect()
}

#[must_use]
pub fn ddl(schema: &Schema) -> Vec<String> {
    let mut statements = schema_ddl(schema);
    statements.extend(crate::families::index_ddl());
    statements
}

#[must_use]
pub fn table_ddl(schema: &Schema) -> Vec<String> {
    let mut statements = Vec::new();
    for relation in schema.relations() {
        let mut columns: Vec<String> = Vec::new();
        for field in relation.fields() {
            for (name, sql_ty) in field_columns(field) {
                columns.push(format!("\"{name}\" {sql_ty} NOT NULL"));
            }
        }
        if let Some(alias) = rowid_alias(relation) {
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
    statements
}

#[must_use]
pub fn schema_ddl(schema: &Schema) -> Vec<String> {
    let mut statements = table_ddl(schema);
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

/// # Panics
fn sql_literal(value: &Value) -> String {
    match value {
        Value::Bool(v) => format!("{}", i64::from(*v)),
        Value::U64(v) => {
            format!(
                "{}",
                i64::try_from(*v).expect("the SQLite mapping axiom: u64 < 2^63")
            )
        }
        Value::I64(v) => format!("{v}"),
        Value::String(text) => format!("'{}'", text.replace('\'', "''")),
        Value::FixedBytes(raw) => {
            let hex = raw.iter().fold(String::new(), |mut acc, byte| {
                use std::fmt::Write as _;
                let _ = write!(acc, "{byte:02X}");
                acc
            });
            format!("X'{hex}'")
        }
        Value::IntervalU64(..) | Value::IntervalI64(..) => {
            panic!("an interval maps to two columns — rows split before rendering")
        }
    }
}

#[must_use]
pub fn extension_rows(relation: &bumbledb::schema::RelationDescriptor) -> Vec<Vec<Value>> {
    let Some(extension) = &relation.extension else {
        return Vec::new();
    };
    extension
        .iter()
        .enumerate()
        .map(|(row, axiom)| {
            let mut fact = vec![Value::U64(row as u64)];
            fact.extend(axiom.values.iter().cloned());
            fact
        })
        .collect()
}

/// mirror must never consume engine encodings.
#[must_use]
pub fn extension_ddl(descriptor: &bumbledb::schema::SchemaDescriptor) -> Vec<String> {
    let mut statements = Vec::new();
    for relation in &descriptor.relations {
        for fact in extension_rows(relation) {
            let mut values = Vec::new();
            for value in &fact {
                match value {
                    Value::IntervalU64(..) | Value::IntervalI64(..) => {
                        let (start, end) = interval_halves(value);
                        let render = |half: rusqlite::types::Value| match half {
                            rusqlite::types::Value::Integer(v) => format!("{v}"),
                            other => unreachable!("interval halves are INTEGER, got {other:?}"),
                        };
                        values.push(render(start));
                        values.push(render(end));
                    }
                    scalar => values.push(sql_literal(scalar)),
                }
            }
            statements.push(format!(
                "INSERT INTO \"{}\" VALUES ({})",
                relation.name,
                values.join(", ")
            ));
        }
    }
    statements
}

#[must_use]
pub fn insert_sql(relation: &Relation) -> String {
    let count: usize = relation
        .fields()
        .iter()
        .map(|field| if field.value_type.is_interval() { 2 } else { 1 })
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

/// # Panics
#[must_use]
pub fn to_sql_value(value: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sql;
    match value {
        Value::Bool(v) => Sql::Integer(i64::from(*v)),
        Value::U64(v) => {
            Sql::Integer(i64::try_from(*v).expect("the SQLite mapping axiom: u64 < 2^63"))
        }
        Value::I64(v) => Sql::Integer(*v),
        Value::String(text) => Sql::Text(text.to_string()),
        Value::FixedBytes(raw) => Sql::Blob(raw.to_vec()),
        Value::IntervalU64(..) | Value::IntervalI64(..) => {
            panic!("an interval maps to two columns — split through interval_halves")
        }
    }
}

/// # Panics
#[must_use]
pub fn interval_halves(value: &Value) -> (rusqlite::types::Value, rusqlite::types::Value) {
    use rusqlite::types::Value as Sql;
    match value {
        Value::IntervalU64(interval) => (
            Sql::Integer(
                i64::try_from(interval.start()).expect("the SQLite mapping axiom: u64 < 2^63"),
            ),
            Sql::Integer(
                i64::try_from(interval.end()).expect("the SQLite mapping axiom: u64 < 2^63"),
            ),
        ),
        Value::IntervalI64(interval) => {
            (Sql::Integer(interval.start()), Sql::Integer(interval.end()))
        }
        scalar => panic!("interval_halves on a scalar {scalar:?}"),
    }
}

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

/// # Errors
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
        (Sql::Integer(v), ValueType::U64) => u64::try_from(*v)
            .map(Value::U64)
            .map_err(|_| format!("u64 column holds negative {v}")),
        (Sql::Integer(v), ValueType::I64) => Ok(Value::I64(*v)),
        (Sql::Text(text), ValueType::String) => Ok(Value::String(text.clone().into())),
        (Sql::Blob(raw), ValueType::FixedBytes { .. }) => Ok(Value::FixedBytes(raw.clone().into())),
        (_, ValueType::Interval { .. } | ValueType::FixedInterval { .. }) => {
            Err("an interval spans two columns — decode through interval_from_sql".to_owned())
        }
        (got, want) => Err(format!("column class {got:?} for {want:?}")),
    }
}

/// # Errors
/// half for a U64 element, or `start >= end` — the stored invariant, so
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
        IntervalElement::U64 => {
            let start =
                u64::try_from(*start).map_err(|_| format!("u64 interval start holds {start}"))?;
            let end = u64::try_from(*end).map_err(|_| format!("u64 interval end holds {end}"))?;
            bumbledb::Interval::<u64>::new(start, end)
                .map(Value::IntervalU64)
                .ok_or_else(|| format!("interval columns hold start {start} >= end {end}"))
        }
        IntervalElement::I64 => bumbledb::Interval::<i64>::new(*start, *end)
            .map(Value::IntervalI64)
            .ok_or_else(|| format!("interval columns hold start {start} >= end {end}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{field, fresh};
    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::schema::{RelationDescriptor, SchemaDescriptor, Side, StatementDescriptor};
    use bumbledb::{FieldId, RelationId};

    fn mini_schema() -> Schema {
        mini_descriptor()
            .validate()
            .expect("the mini schema validates")
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )]
    fn mini_descriptor() -> SchemaDescriptor {
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
                RelationDescriptor {
                    extension: Some(Box::new([
                        bumbledb::schema::Row {
                            handle: "On".into(),
                            values: Box::new([Value::Bool(true)]),
                        },
                        bumbledb::schema::Row {
                            handle: "Off".into(),
                            values: Box::new([Value::Bool(false)]),
                        },
                    ])),
                    name: "Kind".into(),
                    fields: vec![field("flag", ValueType::Bool)],
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
                StatementDescriptor::Containment {
                    source: Side {
                        relation: RelationId(3),
                        projection: Box::new([FieldId(0)]),
                        selection: Box::new([]),
                    },
                    target: Side {
                        relation: RelationId(4),
                        projection: Box::new([FieldId(0)]),
                        selection: Box::new([]),
                    },
                },
            ],
        }
    }

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
                "CREATE TABLE \"Kind\" (\"id\" INTEGER NOT NULL, \"flag\" INTEGER NOT NULL, PRIMARY KEY (\"id\")) STRICT",
                "CREATE UNIQUE INDEX \"uq_Account_s3\" ON \"Account\" (\"code\")",
                "CREATE INDEX \"ix_Mandate_s4\" ON \"Mandate\" (\"account\")",
                "CREATE INDEX \"ix_Mandate_s5\" ON \"Mandate\" (\"org\")",
                "CREATE INDEX \"ix_Mandate_s6\" ON \"Mandate\" (\"account\", \"active_start\", \"active_end\")",
                "CREATE INDEX \"ix_Span_s7\" ON \"Span\" (\"id\")",
            ]
        );

        assert_eq!(
            extension_ddl(&mini_descriptor()),
            vec![
                "INSERT INTO \"Kind\" VALUES (0, 1)",
                "INSERT INTO \"Kind\" VALUES (1, 0)",
            ]
        );

        assert_eq!(
            expected_indexes(&schema)[..5],
            [
                ("Account".to_owned(), "uq_Account_s3".to_owned()),
                ("Mandate".to_owned(), "ix_Mandate_s4".to_owned()),
                ("Mandate".to_owned(), "ix_Mandate_s5".to_owned()),
                ("Mandate".to_owned(), "ix_Mandate_s6".to_owned()),
                ("Span".to_owned(), "ix_Span_s7".to_owned()),
            ]
        );
        assert_eq!(
            insert_sql(schema.relation(RelationId(2))),
            "INSERT INTO \"Mandate\" VALUES (?1, ?2, ?3, ?4)",
            "the placeholder count follows the split"
        );
    }

    #[test]
    fn table_ddl_is_the_index_free_prefix() {
        let schema = crate::schema::schema();
        let tables = table_ddl(schema);
        let full = schema_ddl(schema);
        assert!(!tables.is_empty());
        for statement in &tables {
            assert!(
                statement.starts_with("CREATE TABLE "),
                "not a table: {statement}"
            );
        }
        assert_eq!(full[..tables.len()], tables[..], "the index-free prefix");
        for statement in &full[tables.len()..] {
            assert!(
                statement.starts_with("CREATE INDEX ")
                    || statement.starts_with("CREATE UNIQUE INDEX "),
                "not an index: {statement}"
            );
        }
        assert_eq!(
            full.len(),
            tables.len() + index_plan(schema).len(),
            "the lengths add up"
        );
    }

    #[test]
    fn values_round_trip_through_the_mapping() {
        let cases: Vec<(Value, ValueType)> = vec![
            (Value::Bool(true), ValueType::Bool),
            (Value::U64((1 << 63) - 1), ValueType::U64),
            (Value::I64(i64::MIN), ValueType::I64),
            (Value::String("héllo".into()), ValueType::String),
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

        assert!(from_sql_value(&rusqlite::types::Value::Integer(-1), &ValueType::U64).is_err());
        assert!(from_sql_value(&rusqlite::types::Value::Integer(9), &ValueType::Bool).is_err());

        assert!(
            from_sql_value(
                &rusqlite::types::Value::Integer(0),
                &ValueType::Interval {
                    element: IntervalElement::I64,
                }
            )
            .is_err()
        );
    }

    #[test]
    fn interval_halves_reassemble_through_the_pair_decode() {
        use rusqlite::types::Value as Sql;
        for value in [
            Value::IntervalI64(
                bumbledb::Interval::<i64>::new(i64::MIN, i64::MAX).expect("nonempty interval"),
            ),
            Value::IntervalI64(bumbledb::Interval::<i64>::new(-5, 9).expect("nonempty interval")),
            Value::IntervalU64(
                bumbledb::Interval::<u64>::new(0, (1 << 63) - 1).expect("nonempty interval"),
            ),
            Value::IntervalU64(bumbledb::Interval::<u64>::new(5, 6).expect("nonempty interval")),
        ] {
            let (start, end) = interval_halves(&value);
            let element = match value {
                Value::IntervalU64(..) => IntervalElement::U64,
                _ => IntervalElement::I64,
            };
            assert_eq!(interval_from_sql(&start, &end, element), Ok(value));
        }

        assert!(
            interval_from_sql(&Sql::Integer(5), &Sql::Integer(5), IntervalElement::I64).is_err()
        );
        assert!(
            interval_from_sql(&Sql::Integer(-1), &Sql::Integer(4), IntervalElement::U64).is_err()
        );
        assert!(
            interval_from_sql(
                &Sql::Text("3".to_owned()),
                &Sql::Integer(4),
                IntervalElement::I64
            )
            .is_err()
        );
    }

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
                Value::IntervalI64(
                    bumbledb::Interval::<i64>::new(i64::MIN, i64::MAX).expect("nonempty interval"),
                ),
            ],
            vec![
                Value::U64(2),
                Value::U64(1),
                Value::IntervalI64(
                    bumbledb::Interval::<i64>::new(-5, 9).expect("nonempty interval"),
                ),
            ],
            vec![
                Value::U64(3),
                Value::U64(1),
                Value::IntervalI64(
                    bumbledb::Interval::<i64>::new(-9, -8).expect("nonempty interval"),
                ),
            ],
        ];
        let spans = [
            vec![
                Value::U64(1),
                Value::IntervalU64(
                    bumbledb::Interval::<u64>::new(0, 1).expect("nonempty interval"),
                ),
            ],
            vec![
                Value::U64(2),
                Value::IntervalU64(
                    bumbledb::Interval::<u64>::new(0, (1 << 63) - 1).expect("nonempty interval"),
                ),
            ],
            vec![
                Value::U64(3),
                Value::IntervalU64(
                    bumbledb::Interval::<u64>::new(5, 6).expect("nonempty interval"),
                ),
            ],
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
