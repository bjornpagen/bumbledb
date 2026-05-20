//! SQLite comparison helpers.

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_lmdb::{Error, IdentityValue, InternalError, Result, Row, Value};
use rusqlite::{Connection, params};

/// Loads the small ledger rows into an indexed SQLite database.
pub fn load_ledger(rows: &[Row]) -> Result<Connection> {
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

    for row in rows {
        match row.relation() {
            "Holder" => {
                conn.execute(
                    "INSERT INTO holder (id, name) VALUES (?1, ?2)",
                    params![id(row, "id")?, string(row, "name")?],
                )
                .map_err(sqlite_error)?;
            }
            "Account" => {
                conn.execute(
                    "INSERT INTO account (id, holder, currency) VALUES (?1, ?2, ?3)",
                    params![id(row, "id")?, rf(row, "holder")?, symbol(row, "currency")?],
                )
                .map_err(sqlite_error)?;
            }
            "Posting" => {
                conn.execute(
                    "INSERT INTO posting (id, account, amount, at) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        id(row, "id")?,
                        rf(row, "account")?,
                        decimal(row, "amount")?,
                        ts(row, "at")?
                    ],
                )
                .map_err(sqlite_error)?;
            }
            "AccountTag" => {
                conn.execute(
                    "INSERT INTO account_tag (account, tag) VALUES (?1, ?2)",
                    params![rf(row, "account")?, symbol(row, "tag")?],
                )
                .map_err(sqlite_error)?;
            }
            _ => {}
        }
    }
    Ok(conn)
}

/// Runs a SQLite statement and returns all integer tuple rows.
pub fn query_i64_rows(conn: &Connection, sql: &str, args: &[i64]) -> Result<Vec<Vec<i64>>> {
    let mut stmt = conn.prepare(sql).map_err(sqlite_error)?;
    let column_count = stmt.column_count();
    let rows = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), |row| {
            let mut values = Vec::new();
            for index in 0..column_count {
                values.push(row.get::<_, i64>(index)?);
            }
            Ok(values)
        })
        .map_err(sqlite_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sqlite_error)?;
    Ok(rows)
}

fn sqlite_error(error: rusqlite::Error) -> Error {
    Error::Internal(InternalError::Invariant {
        message: format!("sqlite test error: {error}"),
    })
}

fn id(row: &Row, field: &str) -> Result<i64> {
    match required_value(row, field)? {
        Value::Identity(IdentityValue::Serial(value)) => Ok(*value as i64),
        other => Err(unexpected_value(field, "id", other)),
    }
}

fn rf(row: &Row, field: &str) -> Result<i64> {
    match required_value(row, field)? {
        Value::Identity(IdentityValue::Serial(value)) => Ok(*value as i64),
        other => Err(unexpected_value(field, "ref", other)),
    }
}

fn symbol(row: &Row, field: &str) -> Result<i64> {
    match required_value(row, field)? {
        Value::Enum(value) | Value::U64(value) => Ok(*value as i64),
        other => Err(unexpected_value(field, "symbol", other)),
    }
}

fn decimal(row: &Row, field: &str) -> Result<i64> {
    match required_value(row, field)? {
        Value::Decimal(DecimalRaw(value)) => Ok(*value as i64),
        other => Err(unexpected_value(field, "decimal", other)),
    }
}

fn ts(row: &Row, field: &str) -> Result<i64> {
    match required_value(row, field)? {
        Value::Timestamp(TimestampMicros(value)) => Ok(*value),
        other => Err(unexpected_value(field, "timestamp", other)),
    }
}

fn string(row: &Row, field: &str) -> Result<String> {
    match required_value(row, field)? {
        Value::String(value) => Ok(value.clone()),
        other => Err(unexpected_value(field, "string", other)),
    }
}

fn required_value<'a>(row: &'a Row, field: &str) -> Result<&'a Value> {
    row.value(field)
        .ok_or_else(|| internal_error(format!("missing field {field}")))
}

fn unexpected_value(field: &str, expected: &str, actual: &Value) -> Error {
    internal_error(format!("expected {expected} for {field}, got {actual:?}"))
}

fn internal_error(message: String) -> Error {
    Error::Internal(InternalError::Invariant { message })
}
