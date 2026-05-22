//! SQLite comparison helpers.

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_lmdb::{Error, Fact, InternalError, Result, Value};
use rusqlite::{Connection, params};

/// Loads the small ledger facts into an indexed SQLite database.
pub fn load_ledger(facts: &[Fact]) -> Result<Connection> {
    let conn = Connection::open_in_memory().map_err(sqlite_error)?;
    conn.execute_batch(
        r#"
        CREATE TABLE holder (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
        CREATE TABLE account (id INTEGER PRIMARY KEY, holder INTEGER NOT NULL, currency INTEGER NOT NULL);
        CREATE TABLE posting (id INTEGER PRIMARY KEY, account INTEGER NOT NULL, amount INTEGER NOT NULL, at INTEGER NOT NULL);
        CREATE TABLE account_tag (account INTEGER NOT NULL, tag INTEGER NOT NULL, PRIMARY KEY (account, tag));
        CREATE INDEX account_holder ON account(holder, id);
        CREATE INDEX posting_account ON posting(account, id);
        CREATE INDEX posting_at ON posting(at, id);
        CREATE INDEX tag_account ON account_tag(account, tag);
        "#,
    )
    .map_err(sqlite_error)?;

    for fact in facts {
        match fact.relation() {
            "Holder" => {
                conn.execute(
                    "INSERT INTO holder (id, name) VALUES (?1, ?2)",
                    params![id(fact, "id")?, string(fact, "name")?],
                )
                .map_err(sqlite_error)?;
            }
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
                    "INSERT INTO posting (id, account, amount, at) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        id(fact, "id")?,
                        rf(fact, "account")?,
                        decimal(fact, "amount")?,
                        ts(fact, "at")?
                    ],
                )
                .map_err(sqlite_error)?;
            }
            "AccountTag" => {
                conn.execute(
                    "INSERT INTO account_tag (account, tag) VALUES (?1, ?2)",
                    params![rf(fact, "account")?, symbol(fact, "tag")?],
                )
                .map_err(sqlite_error)?;
            }
            _ => {}
        }
    }
    Ok(conn)
}

/// Runs a SQLite statement and returns all integer fact facts.
pub fn query_i64_rows(conn: &Connection, sql: &str, args: &[i64]) -> Result<Vec<Vec<i64>>> {
    let mut stmt = conn.prepare(sql).map_err(sqlite_error)?;
    let column_count = stmt.column_count();
    let facts = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), |fact| {
            let mut values = Vec::new();
            for index in 0..column_count {
                values.push(fact.get::<_, i64>(index)?);
            }
            Ok(values)
        })
        .map_err(sqlite_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sqlite_error)?;
    Ok(facts)
}

fn sqlite_error(error: rusqlite::Error) -> Error {
    Error::Internal(InternalError::Invariant {
        message: format!("sqlite test error: {error}"),
    })
}

fn id(fact: &Fact, field: &str) -> Result<i64> {
    match required_value(fact, field)? {
        Value::Serial(value) => Ok(*value as i64),
        other => Err(unexpected_value(field, "id", other)),
    }
}

fn rf(fact: &Fact, field: &str) -> Result<i64> {
    match required_value(fact, field)? {
        Value::Serial(value) => Ok(*value as i64),
        other => Err(unexpected_value(field, "ref", other)),
    }
}

fn symbol(fact: &Fact, field: &str) -> Result<i64> {
    match required_value(fact, field)? {
        Value::Enum(value) => Ok(i64::from(*value)),
        Value::U64(value) => Ok(*value as i64),
        other => Err(unexpected_value(field, "symbol", other)),
    }
}

fn decimal(fact: &Fact, field: &str) -> Result<i64> {
    match required_value(fact, field)? {
        Value::Decimal(DecimalRaw(value)) => Ok(*value as i64),
        other => Err(unexpected_value(field, "decimal", other)),
    }
}

fn ts(fact: &Fact, field: &str) -> Result<i64> {
    match required_value(fact, field)? {
        Value::Timestamp(TimestampMicros(value)) => Ok(*value),
        other => Err(unexpected_value(field, "timestamp", other)),
    }
}

fn string(fact: &Fact, field: &str) -> Result<String> {
    match required_value(fact, field)? {
        Value::String(value) => Ok(value.clone()),
        other => Err(unexpected_value(field, "string", other)),
    }
}

fn required_value<'a>(fact: &'a Fact, field: &str) -> Result<&'a Value> {
    fact.value(field)
        .ok_or_else(|| internal_error(format!("missing field {field}")))
}

fn unexpected_value(field: &str, expected: &str, actual: &Value) -> Error {
    internal_error(format!("expected {expected} for {field}, got {actual:?}"))
}

fn internal_error(message: String) -> Error {
    Error::Internal(InternalError::Invariant { message })
}
