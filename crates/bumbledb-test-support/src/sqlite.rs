//! SQLite comparison helpers.

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_lmdb::{Error, Result, Row, Value};
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
                    params![id(row, "id"), string(row, "name")],
                )
                .map_err(sqlite_error)?;
            }
            "Account" => {
                conn.execute(
                    "INSERT INTO account (id, holder, currency) VALUES (?1, ?2, ?3)",
                    params![id(row, "id"), rf(row, "holder"), symbol(row, "currency")],
                )
                .map_err(sqlite_error)?;
            }
            "Posting" => {
                conn.execute(
                    "INSERT INTO posting (id, account, amount, at) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        id(row, "id"),
                        rf(row, "account"),
                        decimal(row, "amount"),
                        ts(row, "at")
                    ],
                )
                .map_err(sqlite_error)?;
            }
            "AccountTag" => {
                conn.execute(
                    "INSERT INTO account_tag (account, tag) VALUES (?1, ?2)",
                    params![rf(row, "account"), symbol(row, "tag")],
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
    Error::Internal(format!("sqlite test error: {error}"))
}

fn id(row: &Row, field: &str) -> i64 {
    match row.value(field).unwrap() {
        Value::Id(value) => *value as i64,
        other => panic!("expected id for {field}, got {other:?}"),
    }
}

fn rf(row: &Row, field: &str) -> i64 {
    match row.value(field).unwrap() {
        Value::Ref(value) => *value as i64,
        other => panic!("expected ref for {field}, got {other:?}"),
    }
}

fn symbol(row: &Row, field: &str) -> i64 {
    match row.value(field).unwrap() {
        Value::Symbol(value) => *value as i64,
        other => panic!("expected symbol for {field}, got {other:?}"),
    }
}

fn decimal(row: &Row, field: &str) -> i64 {
    match row.value(field).unwrap() {
        Value::Decimal(DecimalRaw(value)) => *value as i64,
        other => panic!("expected decimal for {field}, got {other:?}"),
    }
}

fn ts(row: &Row, field: &str) -> i64 {
    match row.value(field).unwrap() {
        Value::Timestamp(TimestampMicros(value)) => *value,
        other => panic!("expected timestamp for {field}, got {other:?}"),
    }
}

fn string(row: &Row, field: &str) -> String {
    match row.value(field).unwrap() {
        Value::String(value) => value.clone(),
        other => panic!("expected string for {field}, got {other:?}"),
    }
}
