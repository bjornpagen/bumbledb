//! Reproducible benchmark fixtures for the normalized ledger workload.

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::schema::{
    FieldDescriptor, PrimaryKeyDescriptor, RelationDescriptor, RelationKind, SchemaDescriptor,
    ValueType,
};

use crate::{Row, Value};

/// A named benchmark query with equivalent Datalog and SQLite SQL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkQuery {
    /// Stable query name.
    pub name: &'static str,
    /// Datalog query text.
    pub datalog: &'static str,
    /// SQLite SQL query text.
    pub sqlite: &'static str,
}

/// Benchmark run output summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkComparison {
    /// Query name.
    pub query: String,
    /// Number of Bumbledb output rows.
    pub bumbledb_rows: usize,
    /// Number of SQLite output rows.
    pub sqlite_rows: usize,
    /// Bumbledb explain plan text.
    pub explain: String,
}

/// Returns the benchmark schema from the Rosetta Stone workload.
pub fn benchmark_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "BenchmarkLedgerDb",
        vec![
            entity(
                "Holder",
                "HolderId",
                vec![FieldDescriptor::new("name", ValueType::String)],
            ),
            entity(
                "Org",
                "OrgId",
                vec![FieldDescriptor::new("name", ValueType::String)],
            ),
            entity(
                "Instrument",
                "InstrumentId",
                vec![FieldDescriptor::new("symbol", ValueType::String)],
            ),
            entity(
                "SourceDocument",
                "SourceDocumentId",
                vec![FieldDescriptor::new("payload", ValueType::Bytes)],
            ),
            RelationDescriptor::new(
                "Account",
                RelationKind::Entity,
                vec![
                    id_field("AccountId", "Account"),
                    ref_field("HolderId", "holder", "Holder"),
                    FieldDescriptor::new(
                        "currency",
                        ValueType::Symbol {
                            name: "Currency".to_owned(),
                        },
                    ),
                ],
                PrimaryKeyDescriptor::new(["id"]),
            )
            .with_generated_id(bumbledb_core::schema::GeneratedIdDescriptor::new("id")),
            RelationDescriptor::new(
                "JournalEntry",
                RelationKind::Event,
                vec![
                    id_field("JournalEntryId", "JournalEntry"),
                    ref_field("SourceDocumentId", "source", "SourceDocument"),
                    FieldDescriptor::new("created_at", ValueType::TimestampMicros).range_indexed(),
                ],
                PrimaryKeyDescriptor::new(["id"]),
            )
            .with_generated_id(bumbledb_core::schema::GeneratedIdDescriptor::new("id")),
            RelationDescriptor::new(
                "Posting",
                RelationKind::Event,
                vec![
                    id_field("PostingId", "Posting"),
                    ref_field("JournalEntryId", "entry", "JournalEntry"),
                    ref_field("AccountId", "account", "Account"),
                    ref_field("InstrumentId", "instrument", "Instrument"),
                    FieldDescriptor::new("amount", ValueType::Decimal { scale: 4 }),
                    FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                ],
                PrimaryKeyDescriptor::new(["id"]),
            )
            .with_generated_id(bumbledb_core::schema::GeneratedIdDescriptor::new("id")),
            RelationDescriptor::new(
                "PostingTag",
                RelationKind::Edge,
                vec![
                    ref_field("PostingId", "posting", "Posting"),
                    FieldDescriptor::new(
                        "tag",
                        ValueType::Symbol {
                            name: "Tag".to_owned(),
                        },
                    ),
                ],
                PrimaryKeyDescriptor::new(["posting", "tag"]),
            ),
            RelationDescriptor::new(
                "OrgParent",
                RelationKind::Edge,
                vec![
                    ref_field("OrgId", "child", "Org"),
                    ref_field("OrgId", "parent", "Org"),
                ],
                PrimaryKeyDescriptor::new(["child", "parent"]),
            ),
            RelationDescriptor::new(
                "AuthorizationEdge",
                RelationKind::Edge,
                vec![
                    ref_field("OrgId", "subject", "Org"),
                    ref_field("OrgId", "object", "Org"),
                    FieldDescriptor::new(
                        "permission",
                        ValueType::Symbol {
                            name: "Permission".to_owned(),
                        },
                    ),
                ],
                PrimaryKeyDescriptor::new(["subject", "object", "permission"]),
            ),
            RelationDescriptor::new(
                "ExchangeRate",
                RelationKind::Event,
                vec![
                    id_field("ExchangeRateId", "ExchangeRate"),
                    ref_field("InstrumentId", "base", "Instrument"),
                    ref_field("InstrumentId", "quote", "Instrument"),
                    FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                    FieldDescriptor::new("rate", ValueType::Decimal { scale: 8 }),
                ],
                PrimaryKeyDescriptor::new(["id"]),
            )
            .with_generated_id(bumbledb_core::schema::GeneratedIdDescriptor::new("id")),
        ],
    )
}

/// Generates deterministic benchmark rows.
pub fn benchmark_rows(scale: u64) -> Vec<Row> {
    let mut rows = Vec::new();
    let scale = scale.max(1);

    for id in 1..=scale {
        rows.push(Row::new(
            "Holder",
            [
                ("id", Value::Id(id)),
                ("name", Value::String(format!("holder-{id}"))),
            ],
        ));
        rows.push(Row::new(
            "Org",
            [
                ("id", Value::Id(id)),
                ("name", Value::String(format!("org-{id}"))),
            ],
        ));
    }
    for id in 1..=3 {
        rows.push(Row::new(
            "Instrument",
            [
                ("id", Value::Id(id)),
                ("symbol", Value::String(format!("SYM{id}"))),
            ],
        ));
    }
    for id in 1..=scale {
        rows.push(Row::new(
            "SourceDocument",
            [
                ("id", Value::Id(id)),
                ("payload", Value::Bytes(format!("source-{id}").into_bytes())),
            ],
        ));
    }
    for id in 1..=scale {
        rows.push(Row::new(
            "Account",
            [
                ("id", Value::Id(id)),
                ("holder", Value::Ref(id)),
                ("currency", Value::Symbol(840)),
            ],
        ));
    }
    for id in 1..=scale {
        rows.push(Row::new(
            "JournalEntry",
            [
                ("id", Value::Id(id)),
                ("source", Value::Ref(id)),
                (
                    "created_at",
                    Value::Timestamp(TimestampMicros(id as i64 * 10)),
                ),
            ],
        ));
    }
    let mut posting_id = 1;
    for account in 1..=scale {
        for offset in 0..3 {
            rows.push(Row::new(
                "Posting",
                [
                    ("id", Value::Id(posting_id)),
                    ("entry", Value::Ref(account)),
                    ("account", Value::Ref(account)),
                    ("instrument", Value::Ref((offset % 3) + 1)),
                    (
                        "amount",
                        Value::Decimal(DecimalRaw((posting_id as i128) * 100)),
                    ),
                    (
                        "at",
                        Value::Timestamp(TimestampMicros(posting_id as i64 * 10)),
                    ),
                ],
            ));
            rows.push(Row::new(
                "PostingTag",
                [
                    ("posting", Value::Ref(posting_id)),
                    ("tag", Value::Symbol(1 + offset)),
                ],
            ));
            posting_id += 1;
        }
    }
    for id in 2..=scale {
        rows.push(Row::new(
            "OrgParent",
            [("child", Value::Ref(id)), ("parent", Value::Ref(1))],
        ));
        rows.push(Row::new(
            "AuthorizationEdge",
            [
                ("subject", Value::Ref(id)),
                ("object", Value::Ref(1)),
                ("permission", Value::Symbol(7)),
            ],
        ));
    }
    for id in 1..=3 {
        rows.push(Row::new(
            "ExchangeRate",
            [
                ("id", Value::Id(id)),
                ("base", Value::Ref(id)),
                ("quote", Value::Ref(1)),
                ("at", Value::Timestamp(TimestampMicros(id as i64 * 10))),
                ("rate", Value::Decimal(DecimalRaw(100_000_000))),
            ],
        ));
    }

    rows
}

/// Returns the benchmark query set.
pub fn benchmark_queries() -> Vec<BenchmarkQuery> {
    vec![BenchmarkQuery {
        name: "postings_for_holder_range",
        datalog: r#"
            find ?posting ?amount
            where
              Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
              Account(id: ?account, holder: $holder)
              ?t >= $start
              ?t < $end
        "#,
        sqlite: r#"
            SELECT p.id, p.amount
            FROM posting p
            JOIN account a ON a.id = p.account
            WHERE a.holder = ?1 AND p.at >= ?2 AND p.at < ?3
        "#,
    }]
}

fn entity(name: &str, id_type: &str, fields: Vec<FieldDescriptor>) -> RelationDescriptor {
    let mut all = vec![id_field(id_type, name)];
    all.extend(fields);
    RelationDescriptor::new(
        name,
        RelationKind::Entity,
        all,
        PrimaryKeyDescriptor::new(["id"]),
    )
    .with_generated_id(bumbledb_core::schema::GeneratedIdDescriptor::new("id"))
}

fn id_field(id_type: &str, relation: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        "id",
        ValueType::Id {
            name: id_type.to_owned(),
            relation: relation.to_owned(),
        },
    )
}

fn ref_field(id_type: &str, field: &str, target: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        field,
        ValueType::Ref {
            name: id_type.to_owned(),
            target_relation: target.to_owned(),
        },
    )
}

#[cfg(test)]
mod tests {
    use bumbledb_core::datalog::parse_and_typecheck;
    use rusqlite::{Connection, params};

    use super::*;
    use crate::{Environment, InputBindings, Result, StorageSchema};

    #[test]
    fn benchmark_schema_loads_and_sqlite_comparison_runs() {
        let dir = tempfile::tempdir().unwrap();
        let env = Environment::open(dir.path()).unwrap();
        let schema = StorageSchema::new(benchmark_schema(), env.max_key_size()).unwrap();
        let rows = benchmark_rows(4);

        env.write(|txn| {
            for row in &rows {
                txn.insert(&schema, row.clone())?;
            }
            Ok::<(), crate::Error>(())
        })
        .unwrap();

        let query = &benchmark_queries()[0];
        let typed = parse_and_typecheck(schema.descriptor(), query.datalog).unwrap();
        let bumbledb = env
            .read(|txn| {
                txn.execute_query(
                    &schema,
                    &typed,
                    &InputBindings::from_values([
                        ("holder", Value::Ref(1)),
                        ("start", Value::Timestamp(TimestampMicros(0))),
                        ("end", Value::Timestamp(TimestampMicros(1000))),
                    ]),
                )
            })
            .unwrap();

        let sqlite_rows = run_sqlite_query(&rows, query.sqlite, 1, 0, 1000).unwrap();
        let comparison = BenchmarkComparison {
            query: query.name.to_owned(),
            bumbledb_rows: bumbledb.rows.len(),
            sqlite_rows,
            explain: bumbledb.explain(),
        };

        assert_eq!(comparison.bumbledb_rows, comparison.sqlite_rows);
        assert!(comparison.bumbledb_rows > 0);
        assert!(comparison.explain.contains("rows_scanned"));
        assert!(comparison.explain.contains("by_at") || comparison.explain.contains("by_holder"));
    }

    fn run_sqlite_query(
        rows: &[Row],
        sql: &str,
        holder: i64,
        start: i64,
        end: i64,
    ) -> Result<usize> {
        let conn = Connection::open_in_memory().map_err(sqlite_error)?;
        conn.execute_batch(
            r#"
            CREATE TABLE account (id INTEGER PRIMARY KEY, holder INTEGER NOT NULL, currency INTEGER NOT NULL);
            CREATE TABLE posting (id INTEGER PRIMARY KEY, entry INTEGER NOT NULL, account INTEGER NOT NULL, instrument INTEGER NOT NULL, amount INTEGER NOT NULL, at INTEGER NOT NULL);
            CREATE INDEX account_holder ON account(holder, id);
            CREATE INDEX posting_account ON posting(account, id);
            CREATE INDEX posting_at ON posting(at, id);
            "#,
        )
        .map_err(sqlite_error)?;

        for row in rows {
            match row.relation() {
                "Account" => {
                    conn.execute(
                        "INSERT INTO account (id, holder, currency) VALUES (?1, ?2, ?3)",
                        params![id(row, "id"), rf(row, "holder"), symbol(row, "currency")],
                    )
                    .map_err(sqlite_error)?;
                }
                "Posting" => {
                    conn.execute(
                        "INSERT INTO posting (id, entry, account, instrument, amount, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![id(row, "id"), rf(row, "entry"), rf(row, "account"), rf(row, "instrument"), decimal(row, "amount"), ts(row, "at")],
                    )
                    .map_err(sqlite_error)?;
                }
                _ => {}
            }
        }

        let mut stmt = conn.prepare(sql).map_err(sqlite_error)?;
        let count = stmt
            .query_map(params![holder, start, end], |_| Ok(()))
            .map_err(sqlite_error)?
            .count();
        Ok(count)
    }

    fn sqlite_error(error: rusqlite::Error) -> crate::Error {
        crate::Error::Internal(format!("sqlite benchmark error: {error}"))
    }

    fn id(row: &Row, field: &str) -> i64 {
        match row.value(field).unwrap() {
            Value::Id(value) => *value as i64,
            other => panic!("expected id, got {other:?}"),
        }
    }

    fn rf(row: &Row, field: &str) -> i64 {
        match row.value(field).unwrap() {
            Value::Ref(value) => *value as i64,
            other => panic!("expected ref, got {other:?}"),
        }
    }

    fn symbol(row: &Row, field: &str) -> i64 {
        match row.value(field).unwrap() {
            Value::Symbol(value) => *value as i64,
            other => panic!("expected symbol, got {other:?}"),
        }
    }

    fn decimal(row: &Row, field: &str) -> i64 {
        match row.value(field).unwrap() {
            Value::Decimal(DecimalRaw(value)) => *value as i64,
            other => panic!("expected decimal, got {other:?}"),
        }
    }

    fn ts(row: &Row, field: &str) -> i64 {
        match row.value(field).unwrap() {
            Value::Timestamp(TimestampMicros(value)) => *value,
            other => panic!("expected timestamp, got {other:?}"),
        }
    }
}
