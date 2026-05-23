use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use rusqlite::{Connection, params};

use super::*;
use crate::{Environment, Fact, InputBindings, Result, StorageSchema, Value};

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
    assert!(comparison.explain.contains("free_join_plan"));
    Ok(())
}

fn run_sqlite_query(facts: &[Fact], sql: &str, holder: i64, start: i64, end: i64) -> Result<usize> {
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
