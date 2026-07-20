//! The crud corpus: seeded rows and the durability-paired twin loader.
//! Every row is a pure function of `(seed, i)` (the displaced
//! `relation_rows` precedent), so the delete lane can re-derive any pool
//! row to hand `tx.delete` the full fact — no row store, no drift.

use std::path::Path;

use bumbledb::{Db, RelationId, Value};

use crate::corpus_gen::{Rng, mix};
use crate::duralane::DurabilityLane;
use crate::sqlmap;

use super::{CrudSizes, CrudWorld, ids, schema};

/// One seeded `Doc` row: id and key are the row index (the pool rows at
/// `docs..docs+delete_pool` included), `val` a small seeded i64, and a
/// seeded 32-byte payload (the points-world `doc_row` payload pattern).
fn doc_row(seed: u64, i: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(seed, ids::DOC, i));
    let val = i64::try_from(rng.range(1_000_000)).expect("small");
    let mut payload = Vec::with_capacity(32);
    for _ in 0..4 {
        payload.extend_from_slice(&rng.u64().to_le_bytes());
    }
    vec![
        Value::U64(i),
        Value::U64(i),
        Value::I64(val),
        Value::FixedBytes(payload.into()),
    ]
}

/// One relation's full row stream — a pure function of `(seed, sizes)`:
/// `Doc` rows `0..docs+delete_pool` (the pool rides above the standing
/// mass, [`CrudSizes`]), `Counter` rows `0..counters` starting at zero.
#[must_use]
pub fn relation_rows(
    sizes: CrudSizes,
    seed: u64,
    rel: RelationId,
) -> Box<dyn Iterator<Item = Vec<Value>>> {
    match rel {
        ids::DOC => Box::new((0..sizes.docs + sizes.delete_pool).map(move |i| doc_row(seed, i))),
        ids::COUNTER => Box::new((0..sizes.counters).map(|i| vec![Value::U64(i), Value::I64(0)])),
        _ => unreachable!("two crud relations"),
    }
}

/// Loads the crud corpus into a fresh durability-paired twin under
/// `dir` (delete-and-recreated — scratch, never user data): the engine
/// store through the lane's constructor, the `SQLite` mirror through
/// the lane's pragma set ([`DurabilityLane::configure`]) and the
/// schema-derived DDL (which emits the UNIQUE indexes for both key
/// statements — the upsert lane's `ON CONFLICT` target), then
/// `ANALYZE`, a truncating WAL checkpoint, and the parity readback
/// ([`DurabilityLane::assert_parity`]) — a misconfigured twin refuses
/// here, before any lane runs.
///
/// # Errors
///
/// Scratch I/O, engine, `SQLite`, and parity errors, stringified.
pub fn load_stores(
    dir: &Path,
    seed: u64,
    sizes: CrudSizes,
    lane: DurabilityLane,
) -> Result<(Db<CrudWorld>, rusqlite::Connection), String> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).map_err(|e| format!("crud scratch: {e}"))?;
    let db = lane.store_mode().create(&dir.join("db"), CrudWorld)?;
    for rel in [ids::DOC, ids::COUNTER] {
        db.bulk_load_dyn(rel, relation_rows(sizes, seed, rel))
            .map_err(|e| format!("load: {e:?}"))?;
    }
    let conn = rusqlite::Connection::open(dir.join("oracle.sqlite"))
        .map_err(|e| format!("oracle: {e}"))?;
    lane.configure(&conn)?;
    for statement in sqlmap::schema_ddl(schema()) {
        conn.execute(&statement, [])
            .map_err(|e| format!("ddl: {e}"))?;
    }
    for rel in [ids::DOC, ids::COUNTER] {
        crate::corpus::insert_rows(
            &conn,
            schema().relation(rel),
            relation_rows(sizes, seed, rel),
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
