use std::path::Path;

use bumbledb::schema::Schema;
use bumbledb::{Db, RelationId, Value};
use rusqlite::Connection;

use super::{Scenario, Stores};
use crate::{corpus, sqlmap};

/// Loads one scenario into a fresh store pair under
/// `<dir>/scenarios/<name>` (delete-and-recreated — scenario stores are
/// tool scratch, never user data).
pub(super) fn load(dir: &Path, scenario: &Scenario, seed: u64) -> Result<Stores, String> {
    let root = dir.join("scenarios").join(scenario.name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).map_err(|e| format!("scenario dir: {e}"))?;
    let schema = (scenario.schema)();

    let db = Db::create(&root.join("db"), schema).map_err(|e| format!("create db: {e:?}"))?;
    let conn = Connection::open(root.join("oracle.sqlite")).map_err(|e| format!("sqlite: {e}"))?;
    corpus::configure_sqlite(&conn).map_err(|e| format!("configure sqlite: {e}"))?;
    for statement in sqlmap::schema_ddl(schema) {
        conn.execute(&statement, [])
            .map_err(|e| format!("ddl: {e}"))?;
    }

    let mut total = 0u64;
    for (rel, rows) in (scenario.rows)(seed) {
        let rows: Vec<Vec<Value>> = rows.collect();
        total += rows.len() as u64;
        db.bulk_load(rel, rows.iter().cloned())
            .map_err(|e| format!("{}: bulk_load: {e}", scenario.name))?;
        load_sqlite_rows(&conn, schema, rel, &rows)?;
    }
    for statement in scenario.extra_indexes {
        conn.execute(statement, [])
            .map_err(|e| format!("index: {e}"))?;
    }
    conn.execute_batch("ANALYZE")
        .map_err(|e| format!("analyze: {e}"))?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .map_err(|e| format!("checkpoint: {e}"))?;
    eprintln!(
        "scenario {}: loaded {total} facts x 2 engines",
        scenario.name
    );
    Ok(Stores { db, conn })
}

/// The `SQLite` mirror load for one relation (the ledger loader is
/// generator-coupled; this one takes the rows).
fn load_sqlite_rows(
    conn: &Connection,
    schema: &Schema,
    rel: RelationId,
    rows: &[Vec<Value>],
) -> Result<(), String> {
    let relation = schema.relation(rel);
    let placeholders = (1..=relation.fields().len())
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let insert = format!(
        "INSERT INTO \"{}\" VALUES ({placeholders})",
        relation.name()
    );
    for chunk in rows.chunks(4096) {
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("begin: {e}"))?;
        {
            let mut stmt = conn
                .prepare_cached(&insert)
                .map_err(|e| format!("prepare: {e}"))?;
            for row in chunk {
                let params: Vec<rusqlite::types::Value> =
                    row.iter().map(sqlmap::to_sql_value).collect();
                stmt.execute(rusqlite::params_from_iter(params))
                    .map_err(|e| format!("insert: {e}"))?;
            }
        }
        conn.execute_batch("COMMIT")
            .map_err(|e| format!("commit: {e}"))?;
    }
    Ok(())
}
