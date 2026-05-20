//! Reproducible benchmark fixtures for the normalized ledger workload.

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_builder::{OperandRef, QueryBuildResult, QueryBuilder};
use bumbledb_core::query_ir::{ComparisonOperator, TypedQuery};
use bumbledb_core::schema::{
    FieldDescriptor, IdentityAllocation, IndexDescriptor, PrimaryKeyDescriptor, RelationDescriptor,
    RelationKind, SchemaDescriptor, ValueType,
};

use crate::{IdentityValue, Row, Value};

/// Builds a typed benchmark query for a schema descriptor.
pub type BenchmarkQueryBuilder = fn(&SchemaDescriptor) -> QueryBuildResult<TypedQuery>;

/// A named benchmark query with equivalent typed Bumbledb and SQLite SQL.
#[derive(Clone, Debug)]
pub struct BenchmarkQuery {
    /// Stable query name.
    pub name: &'static str,
    /// Typed query builder.
    pub build: BenchmarkQueryBuilder,
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
                        ValueType::Enum {
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
                        ValueType::Enum {
                            name: "Tag".to_owned(),
                        },
                    ),
                ],
                PrimaryKeyDescriptor::new(["posting", "tag"]),
            )
            .with_index(IndexDescriptor::permutation("by_tag", ["tag", "posting"])),
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
                        ValueType::Enum {
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
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Currency",
        [840],
    ))
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Tag",
        [1, 2, 3],
    ))
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Permission",
        [7],
    ))
    .with_ref_foreign_keys()
}

/// Generates deterministic benchmark rows.
pub fn benchmark_rows(scale: u64) -> Vec<Row> {
    let mut rows = Vec::new();
    let scale = scale.max(1);

    for id in 1..=scale {
        rows.push(Row::new(
            "Holder",
            [
                ("id", Value::Identity(IdentityValue::Serial(id))),
                ("name", Value::String(format!("holder-{id}"))),
            ],
        ));
        rows.push(Row::new(
            "Org",
            [
                ("id", Value::Identity(IdentityValue::Serial(id))),
                ("name", Value::String(format!("org-{id}"))),
            ],
        ));
    }
    for id in 1..=3 {
        rows.push(Row::new(
            "Instrument",
            [
                ("id", Value::Identity(IdentityValue::Serial(id))),
                ("symbol", Value::String(format!("SYM{id}"))),
            ],
        ));
    }
    for id in 1..=scale {
        rows.push(Row::new(
            "SourceDocument",
            [
                ("id", Value::Identity(IdentityValue::Serial(id))),
                ("payload", Value::Bytes(format!("source-{id}").into_bytes())),
            ],
        ));
    }
    for id in 1..=scale {
        rows.push(Row::new(
            "Account",
            [
                ("id", Value::Identity(IdentityValue::Serial(id))),
                ("holder", Value::Identity(IdentityValue::Serial(id))),
                ("currency", Value::Enum(840)),
            ],
        ));
    }
    for id in 1..=scale {
        rows.push(Row::new(
            "JournalEntry",
            [
                ("id", Value::Identity(IdentityValue::Serial(id))),
                ("source", Value::Identity(IdentityValue::Serial(id))),
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
                    ("id", Value::Identity(IdentityValue::Serial(posting_id))),
                    ("entry", Value::Identity(IdentityValue::Serial(account))),
                    ("account", Value::Identity(IdentityValue::Serial(account))),
                    (
                        "instrument",
                        Value::Identity(IdentityValue::Serial((offset % 3) + 1)),
                    ),
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
                    (
                        "posting",
                        Value::Identity(IdentityValue::Serial(posting_id)),
                    ),
                    ("tag", Value::Enum(1 + offset)),
                ],
            ));
            posting_id += 1;
        }
    }
    for id in 2..=scale {
        rows.push(Row::new(
            "OrgParent",
            [
                ("child", Value::Identity(IdentityValue::Serial(id))),
                ("parent", Value::Identity(IdentityValue::Serial(1))),
            ],
        ));
        rows.push(Row::new(
            "AuthorizationEdge",
            [
                ("subject", Value::Identity(IdentityValue::Serial(id))),
                ("object", Value::Identity(IdentityValue::Serial(1))),
                ("permission", Value::Enum(7)),
            ],
        ));
    }
    for id in 1..=3 {
        rows.push(Row::new(
            "ExchangeRate",
            [
                ("id", Value::Identity(IdentityValue::Serial(id))),
                ("base", Value::Identity(IdentityValue::Serial(id))),
                ("quote", Value::Identity(IdentityValue::Serial(1))),
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
        build: postings_for_holder_range_query,
        sqlite: r#"
            SELECT p.id, p.amount
            FROM posting p
            JOIN account a ON a.id = p.account
            WHERE a.holder = ?1 AND p.at >= ?2 AND p.at < ?3
        "#,
    }]
}

fn postings_for_holder_range_query(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    QueryBuilder::new(schema)
        .rel("Posting")?
        .var("id", "posting")?
        .var("account", "account")?
        .var("amount", "amount")?
        .var("at", "t")?
        .done()
        .rel("Account")?
        .var("id", "account")?
        .input("holder", "holder")?
        .done()
        .cmp(
            OperandRef::var("t"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?
        .cmp(
            OperandRef::var("t"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?
        .find_var("posting")?
        .find_var("amount")?
        .finish()
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
        ValueType::Identity {
            type_name: id_type.to_owned(),
            owning_relation: relation.to_owned(),
            allocation: IdentityAllocation::Serial,
        },
    )
}

fn ref_field(id_type: &str, field: &str, target: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        field,
        ValueType::Identity {
            type_name: id_type.to_owned(),
            owning_relation: target.to_owned(),
            allocation: IdentityAllocation::Serial,
        },
    )
}

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};

    use super::*;
    use crate::{Environment, InputBindings, Result, StorageSchema};

    #[test]
    fn benchmark_schema_loads_and_sqlite_comparison_runs()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = StorageSchema::new(benchmark_schema(), env.max_key_size())?;
        let rows = benchmark_rows(4);

        env.write(|txn| {
            for row in &rows {
                txn.insert(&schema, row.clone())?;
            }
            Ok::<(), crate::Error>(())
        })?;

        let query = &benchmark_queries()[0];
        let typed = (query.build)(schema.descriptor())?;
        let prepared = env.prepare_query(&schema, &typed)?;
        let bumbledb = env.read(|txn| {
            txn.execute_prepared_query(
                &schema,
                &prepared,
                &InputBindings::from_values([
                    ("holder", Value::Identity(IdentityValue::Serial(1))),
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    ("end", Value::Timestamp(TimestampMicros(1000))),
                ]),
            )
        })?;

        let sqlite_rows = run_sqlite_query(&rows, query.sqlite, 1, 0, 1000)?;
        let comparison = BenchmarkComparison {
            query: query.name.to_owned(),
            bumbledb_rows: bumbledb.rows.len(),
            sqlite_rows,
            explain: bumbledb.explain(),
        };

        assert_eq!(comparison.bumbledb_rows, comparison.sqlite_rows);
        assert!(comparison.bumbledb_rows > 0);
        assert!(comparison.explain.contains("rows_scanned"));
        assert!(comparison.explain.contains("candidate_plan"));
        Ok(())
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
                        params![id(row, "id")?, rf(row, "holder")?, symbol(row, "currency")?],
                    )
                    .map_err(sqlite_error)?;
                }
                "Posting" => {
                    conn.execute(
                        "INSERT INTO posting (id, entry, account, instrument, amount, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![id(row, "id")?, rf(row, "entry")?, rf(row, "account")?, rf(row, "instrument")?, decimal(row, "amount")?, ts(row, "at")?],
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
        crate::Error::internal(format!("sqlite benchmark error: {error}"))
    }

    fn id(row: &Row, field: &str) -> Result<i64> {
        match required_value(row, field)? {
            Value::Identity(IdentityValue::Serial(value)) => Ok(*value as i64),
            other => Err(unexpected_value("id", other)),
        }
    }

    fn rf(row: &Row, field: &str) -> Result<i64> {
        match required_value(row, field)? {
            Value::Identity(IdentityValue::Serial(value)) => Ok(*value as i64),
            other => Err(unexpected_value("ref", other)),
        }
    }

    fn symbol(row: &Row, field: &str) -> Result<i64> {
        match required_value(row, field)? {
            Value::Enum(value) | Value::U64(value) => Ok(*value as i64),
            other => Err(unexpected_value("symbol", other)),
        }
    }

    fn decimal(row: &Row, field: &str) -> Result<i64> {
        match required_value(row, field)? {
            Value::Decimal(DecimalRaw(value)) => Ok(*value as i64),
            other => Err(unexpected_value("decimal", other)),
        }
    }

    fn ts(row: &Row, field: &str) -> Result<i64> {
        match required_value(row, field)? {
            Value::Timestamp(TimestampMicros(value)) => Ok(*value),
            other => Err(unexpected_value("timestamp", other)),
        }
    }

    fn required_value<'a>(row: &'a Row, field: &str) -> Result<&'a Value> {
        row.value(field)
            .ok_or_else(|| crate::Error::internal(format!("missing field {field}")))
    }

    fn unexpected_value(expected: &str, actual: &Value) -> crate::Error {
        crate::Error::internal(format!("expected {expected}, got {actual:?}"))
    }
}
