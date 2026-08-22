use std::path::Path;

use bumbledb::Db;
use rusqlite::Connection;

use crate::duralane::DurabilityLane;

use super::{LawSizes, LawfulWorld, corpus, enforcement, ids, schema};

/// Loads the lawful corpus into a fresh durability-paired twin under `dir`
/// (delete-and-recreated — scratch, never user data): the engine store through
/// the lane's constructor, inserted in containment Verdict seeds none); the
/// `SQLite` mirror through the lane's pragma set plus `PRAGMA foreign_keys=ON`
/// (asserted read back as 1 — the FK rows of the enforcement map are dead
/// letters without it), the map-derived DDL, the same row streams through the
/// shared prepared insert loop, then `ANALYZE`, a truncating WAL checkpoint,
/// and the order (targets before sources: Task, Steer, Attempt, `SteerScope`;
/// misconfigured twin refuses here, before any lane runs.
/// # Errors
pub fn load_stores(
    dir: &Path,
    _seed: u64,
    sizes: LawSizes,
    lane: DurabilityLane,
) -> Result<(Db<LawfulWorld>, Connection), String> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).map_err(|e| format!("lawful scratch: {e}"))?;
    let db = lane.store_mode().create(&dir.join("db"), LawfulWorld)?;

    let order = [
        ids::TASK,
        ids::STEER,
        ids::ATTEMPT,
        ids::STEER_SCOPE,
        ids::VERDICT,
    ];
    for rel in order {
        db.write(|tx| {
            tx.insert_dyn(rel, corpus::relation_rows(sizes, rel))
                .map(bumbledb::MutationReport::changed)
        })
        .map_err(|e| format!("load: {e:?}"))?
        .unwrap();
    }
    let conn = Connection::open(dir.join("oracle.sqlite")).map_err(|e| format!("oracle: {e}"))?;
    lane.configure(&conn)?;
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
        crate::corpus::insert_rows(
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
    lane.assert_parity(&conn)?;
    Ok((db, conn))
}
