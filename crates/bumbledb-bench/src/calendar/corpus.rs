//! The engine load respects the statements it is judged under: containment
//! targets precede their sources (accounts → persons → the `Attendance ==
//! Claim` discriminated-union cluster loads through calendars → events; working
//! hours before the claims they cover), and
use std::path::Path;
use std::time::Instant;

use bumbledb::{Db, RelationId, Value};
use rusqlite::Connection;

use crate::calendar::corpus_gen::{CalSizes, du_cluster_rows, relation_rows_sized};
use crate::calendar::{Scheduling, ids, schema};
use crate::corpus::{LoadStats, configure_sqlite, insert_rows};
use crate::corpus_gen::GenConfig;
use crate::sqlmap;

const ORDER: [RelationId; 8] = [
    ids::ACCOUNT,
    ids::PERSON,
    ids::CALENDAR,
    ids::WORK_HOURS,
    ids::EVENT,
    ids::ROOM,
    ids::BOOKING,
    ids::SLOT,
];

const CHUNK: usize = 4096;

/// # Errors
pub fn load_bumbledb(db: &Db<Scheduling>, cfg: GenConfig) -> Result<LoadStats, bumbledb::Error> {
    load_bumbledb_sized(db, cfg, CalSizes::of(cfg.scale))
}

/// # Errors
pub fn load_bumbledb_sized(
    db: &Db<Scheduling>,
    cfg: GenConfig,
    sizes: CalSizes,
) -> Result<LoadStats, bumbledb::Error> {
    let start = Instant::now();
    let mut facts = 0u64;
    for rel in ORDER {
        facts += db
            .write(|tx| {
                tx.insert_dyn(rel, relation_rows_sized(cfg, sizes, rel))
                    .map(bumbledb::MutationReport::changed)
            })?
            .unwrap()
            .value;
    }
    let mut pending: Vec<(RelationId, Vec<Value>)> = Vec::with_capacity(CHUNK + 4);
    for (attendances, claim) in du_cluster_rows(cfg, sizes) {
        for row in attendances {
            pending.push((ids::ATTENDANCE, row));
        }
        pending.push((ids::CLAIM, claim));
        if pending.len() >= CHUNK {
            facts += flush(db, &mut pending)?;
        }
    }
    facts += flush(db, &mut pending)?;
    Ok(LoadStats::of(facts, start.elapsed()))
}

fn flush(
    db: &Db<Scheduling>,
    pending: &mut Vec<(RelationId, Vec<Value>)>,
) -> Result<u64, bumbledb::Error> {
    if pending.is_empty() {
        return Ok(0);
    }
    db.write(|tx| {
        for (rel, row) in pending.iter() {
            tx.insert_dyn(*rel, [row])?;
        }
        Ok(())
    })?
    .unwrap();
    let facts = pending.len() as u64;
    pending.clear();
    Ok(facts)
}

/// # Errors
/// # Panics
/// Only on programmer-invariant violations (WAL refused; corpus values
pub fn load_sqlite(path: &Path, cfg: GenConfig) -> rusqlite::Result<(Connection, LoadStats)> {
    let conn = Connection::open(path)?;
    configure_sqlite(&conn)?;
    load_sqlite_into(&conn, cfg, CalSizes::of(cfg.scale)).map(|stats| (conn, stats))
}

/// # Errors
pub fn load_sqlite_into(
    conn: &Connection,
    cfg: GenConfig,
    sizes: CalSizes,
) -> rusqlite::Result<LoadStats> {
    for statement in ddl() {
        conn.execute(&statement, [])?;
    }
    let start = Instant::now();
    let mut facts = 0u64;
    for rel in 0..ids::RELATIONS {
        let rel = RelationId(rel);
        facts += insert_rows(
            conn,
            schema().relation(rel),
            relation_rows_sized(cfg, sizes, rel),
        )?;
    }
    conn.execute_batch("ANALYZE")?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    Ok(LoadStats::of(facts, start.elapsed()))
}

#[must_use]
pub fn ddl() -> Vec<String> {
    let mut statements = sqlmap::schema_ddl(schema());
    statements.extend(sqlmap::extension_ddl(&bumbledb::Theory::descriptor(
        crate::calendar::Scheduling,
    )));
    statements.extend(crate::calendar::families::index_ddl());
    statements
}

/// # Panics
pub fn assert_loaded_equal(db: &Db<Scheduling>, conn: &Connection, cfg: GenConfig) {
    let schema = schema();
    let sizes = CalSizes::of(cfg.scale);
    for rel in 0..ids::RELATIONS {
        let rel = RelationId(rel);
        let name = schema.relation(rel).name();
        let ours = db
            .read(|snap| Ok(snap.scan(rel)?.count()))
            .expect("scan counts");
        let theirs: u64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM \"{name}\""), [], |row| {
                row.get(0)
            })
            .expect("count");
        assert_eq!(ours as u64, theirs, "row counts diverge for {name}");
        assert_eq!(ours as u64, sizes.rows(rel), "generator count for {name}");
    }

    let mut rng = crate::corpus_gen::Rng::new(cfg.seed ^ 0xCA1E);
    let events: Vec<Vec<Value>> = relation_rows_sized(cfg, sizes, ids::EVENT).collect();
    let relation = schema.relation(ids::EVENT);
    for _ in 0..100 {
        let i = rng.range(sizes.events);
        let expected = &events[usize::try_from(i).expect("fits")];
        let got: Vec<Value> = conn
            .query_row(
                "SELECT * FROM \"Event\" WHERE \"id\" = ?1",
                [i64::try_from(i).expect("axiom")],
                |row| {
                    let mut values = Vec::new();
                    let mut column = 0;
                    for field in relation.fields() {
                        if field.value_type.interval_element()
                            == Some(bumbledb::schema::IntervalElement::I64)
                        {
                            let start: rusqlite::types::Value = row.get(column)?;
                            let end: rusqlite::types::Value = row.get(column + 1)?;
                            values.push(
                                sqlmap::interval_from_sql(
                                    &start,
                                    &end,
                                    bumbledb::schema::IntervalElement::I64,
                                )
                                .expect("interval reassembles"),
                            );
                            column += 2;
                        } else {
                            let raw: rusqlite::types::Value = row.get(column)?;
                            values.push(
                                sqlmap::from_sql_value(&raw, &field.value_type)
                                    .expect("mapped value decodes"),
                            );
                            column += 1;
                        }
                    }
                    Ok(values)
                },
            )
            .expect("sample fetch");
        assert_eq!(&got, expected, "event {i} diverges");
    }
}
