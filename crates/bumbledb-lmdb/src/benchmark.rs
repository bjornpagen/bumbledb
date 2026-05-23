//! Reproducible benchmark fixtures for the normalized ledger workload.

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_builder::{OperandRef, QueryBuildResult, QueryBuilder};
use bumbledb_core::query_ir::{ComparisonOperator, TypedQuery};
use bumbledb_core::schema::{
    ConstraintDescriptor, FieldDescriptor, IndexDescriptor, RelationDescriptor, SchemaDescriptor,
    ValueType,
};

use crate::{Fact, Value};

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
    /// Number of Bumbledb output facts.
    pub bumbledb_facts: usize,
    /// Number of SQLite output facts.
    pub sqlite_facts: usize,
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
                vec![
                    serial_key_field("AccountId", "Account"),
                    serial_field("HolderId", "holder", "Holder"),
                    FieldDescriptor::new(
                        "currency",
                        ValueType::Enum {
                            name: "Currency".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "holder",
                ["holder"],
                "Holder",
                "id",
            )),
            RelationDescriptor::new(
                "JournalEntry",
                vec![
                    serial_key_field("JournalEntryId", "JournalEntry"),
                    serial_field("SourceDocumentId", "source", "SourceDocument"),
                    FieldDescriptor::new("created_at", ValueType::TimestampMicros).range_indexed(),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "source",
                ["source"],
                "SourceDocument",
                "id",
            )),
            RelationDescriptor::new(
                "Posting",
                vec![
                    serial_key_field("PostingId", "Posting"),
                    serial_field("JournalEntryId", "entry", "JournalEntry"),
                    serial_field("AccountId", "account", "Account"),
                    serial_field("InstrumentId", "instrument", "Instrument"),
                    FieldDescriptor::new("amount", ValueType::Decimal { scale: 4 }),
                    FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "entry",
                ["entry"],
                "JournalEntry",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "account",
                ["account"],
                "Account",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "instrument",
                ["instrument"],
                "Instrument",
                "id",
            )),
            RelationDescriptor::new(
                "PostingTag",
                vec![
                    serial_field("PostingId", "posting", "Posting"),
                    FieldDescriptor::new(
                        "tag",
                        ValueType::Enum {
                            name: "Tag".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique("posting_tag", ["posting", "tag"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "posting",
                ["posting"],
                "Posting",
                "id",
            ))
            .with_index(IndexDescriptor::permutation("by_tag", ["tag", "posting"])),
            RelationDescriptor::new(
                "OrgParent",
                vec![
                    serial_field("OrgId", "child", "Org"),
                    serial_field("OrgId", "parent", "Org"),
                ],
            )
            .with_unique("child_parent", ["child", "parent"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "child",
                ["child"],
                "Org",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "parent",
                ["parent"],
                "Org",
                "id",
            )),
            RelationDescriptor::new(
                "AuthorizationEdge",
                vec![
                    serial_field("OrgId", "subject", "Org"),
                    serial_field("OrgId", "object", "Org"),
                    FieldDescriptor::new(
                        "permission",
                        ValueType::Enum {
                            name: "Permission".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique(
                "subject_object_permission",
                ["subject", "object", "permission"],
            )
            .with_constraint(ConstraintDescriptor::foreign_key(
                "subject",
                ["subject"],
                "Org",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "object",
                ["object"],
                "Org",
                "id",
            )),
            RelationDescriptor::new(
                "ExchangeRate",
                vec![
                    serial_key_field("ExchangeRateId", "ExchangeRate"),
                    serial_field("InstrumentId", "base", "Instrument"),
                    serial_field("InstrumentId", "quote", "Instrument"),
                    FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                    FieldDescriptor::new("rate", ValueType::Decimal { scale: 8 }),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "base",
                ["base"],
                "Instrument",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "quote",
                ["quote"],
                "Instrument",
                "id",
            )),
        ],
    )
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Currency",
        [1],
    ))
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Tag",
        [1, 2, 3],
    ))
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Permission",
        [7],
    ))
}

/// Generates deterministic benchmark facts.
pub fn benchmark_facts(scale: u64) -> Vec<Fact> {
    let mut facts = Vec::new();
    let scale = scale.max(1);

    for id in 1..=scale {
        facts.push(Fact::new(
            "Holder",
            [
                ("id", Value::Serial(id)),
                ("name", Value::String(format!("holder-{id}"))),
            ],
        ));
        facts.push(Fact::new(
            "Org",
            [
                ("id", Value::Serial(id)),
                ("name", Value::String(format!("org-{id}"))),
            ],
        ));
    }
    for id in 1..=3 {
        facts.push(Fact::new(
            "Instrument",
            [
                ("id", Value::Serial(id)),
                ("symbol", Value::String(format!("SYM{id}"))),
            ],
        ));
    }
    for id in 1..=scale {
        facts.push(Fact::new(
            "SourceDocument",
            [
                ("id", Value::Serial(id)),
                ("payload", Value::Bytes(format!("source-{id}").into_bytes())),
            ],
        ));
    }
    for id in 1..=scale {
        facts.push(Fact::new(
            "Account",
            [
                ("id", Value::Serial(id)),
                ("holder", Value::Serial(id)),
                ("currency", Value::Enum(1)),
            ],
        ));
    }
    for id in 1..=scale {
        facts.push(Fact::new(
            "JournalEntry",
            [
                ("id", Value::Serial(id)),
                ("source", Value::Serial(id)),
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
            facts.push(Fact::new(
                "Posting",
                [
                    ("id", Value::Serial(posting_id)),
                    ("entry", Value::Serial(account)),
                    ("account", Value::Serial(account)),
                    ("instrument", Value::Serial((offset % 3) + 1)),
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
            facts.push(Fact::new(
                "PostingTag",
                [
                    ("posting", Value::Serial(posting_id)),
                    ("tag", Value::Enum((1 + offset) as u8)),
                ],
            ));
            posting_id += 1;
        }
    }
    for id in 2..=scale {
        facts.push(Fact::new(
            "OrgParent",
            [("child", Value::Serial(id)), ("parent", Value::Serial(1))],
        ));
        facts.push(Fact::new(
            "AuthorizationEdge",
            [
                ("subject", Value::Serial(id)),
                ("object", Value::Serial(1)),
                ("permission", Value::Enum(7)),
            ],
        ));
    }
    for id in 1..=3 {
        facts.push(Fact::new(
            "ExchangeRate",
            [
                ("id", Value::Serial(id)),
                ("base", Value::Serial(id)),
                ("quote", Value::Serial(1)),
                ("at", Value::Timestamp(TimestampMicros(id as i64 * 10))),
                ("rate", Value::Decimal(DecimalRaw(100_000_000))),
            ],
        ));
    }

    facts
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
    let mut all = vec![serial_key_field(id_type, name)];
    all.extend(fields);
    RelationDescriptor::new(name, all).with_unique("id", ["id"])
}

fn serial_key_field(id_type: &str, relation: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        "id",
        ValueType::Serial {
            type_name: id_type.to_owned(),
            owning_relation: relation.to_owned(),
        },
    )
}

fn serial_field(id_type: &str, field: &str, target: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        field,
        ValueType::Serial {
            type_name: id_type.to_owned(),
            owning_relation: target.to_owned(),
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
        let facts = benchmark_facts(4);

        env.write(|txn| {
            for fact in &facts {
                txn.insert(&schema, fact.clone())?;
            }
            Ok::<(), crate::Error>(())
        })?;

        let query = &benchmark_queries()[0];
        let typed = (query.build)(schema.descriptor())?;
        let bumbledb = env.read(|txn| {
            txn.execute_query(
                &schema,
                &typed,
                &InputBindings::from_values([
                    ("holder", Value::Serial(1)),
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    ("end", Value::Timestamp(TimestampMicros(1000))),
                ]),
            )
        })?;

        let sqlite_facts = run_sqlite_query(&facts, query.sqlite, 1, 0, 1000)?;
        let comparison = BenchmarkComparison {
            query: query.name.to_owned(),
            bumbledb_facts: bumbledb.result.facts.len(),
            sqlite_facts,
            explain: bumbledb.explain(),
        };

        assert_eq!(comparison.bumbledb_facts, comparison.sqlite_facts);
        assert!(comparison.bumbledb_facts > 0);
        assert!(comparison.explain.contains("facts_scanned"));
        assert!(comparison.explain.contains("candidate_plan"));
        Ok(())
    }

    fn run_sqlite_query(
        facts: &[Fact],
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

        for fact in facts {
            match fact.relation() {
                "Account" => {
                    conn.execute(
                        "INSERT INTO account (id, holder, currency) VALUES (?1, ?2, ?3)",
                        params![
                            id(fact, "id")?,
                            rf(fact, "holder")?,
                            symbol(fact, "currency")?
                        ],
                    )
                    .map_err(sqlite_error)?;
                }
                "Posting" => {
                    conn.execute(
                        "INSERT INTO posting (id, entry, account, instrument, amount, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![id(fact, "id")?, rf(fact, "entry")?, rf(fact, "account")?, rf(fact, "instrument")?, decimal(fact, "amount")?, ts(fact, "at")?],
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

    fn id(fact: &Fact, field: &str) -> Result<i64> {
        match required_value(fact, field)? {
            Value::Serial(value) => Ok(*value as i64),
            other => Err(unexpected_value("id", other)),
        }
    }

    fn rf(fact: &Fact, field: &str) -> Result<i64> {
        match required_value(fact, field)? {
            Value::Serial(value) => Ok(*value as i64),
            other => Err(unexpected_value("ref", other)),
        }
    }

    fn symbol(fact: &Fact, field: &str) -> Result<i64> {
        match required_value(fact, field)? {
            Value::Enum(value) => Ok(i64::from(*value)),
            Value::U64(value) => Ok(*value as i64),
            other => Err(unexpected_value("symbol", other)),
        }
    }

    fn decimal(fact: &Fact, field: &str) -> Result<i64> {
        match required_value(fact, field)? {
            Value::Decimal(DecimalRaw(value)) => Ok(*value as i64),
            other => Err(unexpected_value("decimal", other)),
        }
    }

    fn ts(fact: &Fact, field: &str) -> Result<i64> {
        match required_value(fact, field)? {
            Value::Timestamp(TimestampMicros(value)) => Ok(*value),
            other => Err(unexpected_value("timestamp", other)),
        }
    }

    fn required_value<'a>(fact: &'a Fact, field: &str) -> Result<&'a Value> {
        fact.value(field)
            .ok_or_else(|| crate::Error::internal(format!("missing field {field}")))
    }

    fn unexpected_value(expected: &str, actual: &Value) -> crate::Error {
        crate::Error::internal(format!("expected {expected}, got {actual:?}"))
    }
}
