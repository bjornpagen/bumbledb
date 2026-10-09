use std::path::Path;

use bumbledb::Db;
use rusqlite::Connection;

use super::{LawSizes, LawfulWorld, corpus, enforcement, ids, schema};

/// Loads the lawful corpus into a fresh durability-paired twin under `dir`
/// (delete-and-recreated scratch). Relations load targets before sources
/// (Task, Steer, Attempt, `SteerScope`). The `SQLite` mirror gets the durable
/// pragma set plus `PRAGMA foreign_keys=ON`, read back as 1 because the FK
/// rows of the enforcement map are dead letters without it, then the
/// map-derived DDL, the same row streams, `ANALYZE`, a truncating WAL
/// checkpoint, and the parity readback.
/// misconfigured twin refuses here, before any lane runs.
/// # Errors
pub fn load_stores(
    dir: &Path,
    _seed: u64,
    sizes: LawSizes,
) -> Result<(Db<LawfulWorld>, Connection), String> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).map_err(|e| format!("lawful scratch: {e}"))?;
    let db = crate::harness::create_db(&dir.join("db"), LawfulWorld)?;

    let order = [
        ids::TASK,
        ids::STEER,
        ids::ATTEMPT,
        ids::STEER_SCOPE,
        ids::VERDICT,
    ];
    for rel in order {
        db.write(crate::harness::bench_work(), |tx| {
            tx.insert_dyn(rel, corpus::relation_rows(sizes, rel))
                .map(bumbledb::MutationReport::changed)
        })
        .map_err(|e| format!("load: {e:?}"))?
        .unwrap();
    }
    let conn = Connection::open(dir.join("oracle.sqlite")).map_err(|e| format!("oracle: {e}"))?;
    crate::harness::sqlite_run::configure_durable(&conn)?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("pragma foreign_keys: {e}"))?;
    let fk: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(|e| format!("pragma foreign_keys: {e}"))?;
    if fk != 1 {
        return Err(format!("pragma foreign_keys: expected 1, found {fk}"));
    }
    for statement in enforcement::ddl() {
        conn.execute_batch(&statement)
            .map_err(|e| format!("ddl: {e}\n{statement}"))?;
    }

    for rel in order {
        crate::worlds::corpus::insert_rows(
            &conn,
            schema().relation(rel),
            corpus::relation_rows(sizes, rel),
        )
        .map_err(|e| format!("insert: {e}"))?;
    }
    conn.execute_batch("ANALYZE")
        .map_err(|e| format!("analyze: {e}"))?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .map_err(|e| format!("checkpoint: {e}"))?;
    crate::harness::sqlite_run::assert_durable_parity(&conn)?;
    Ok((db, conn))
}
