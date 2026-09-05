use std::path::Path;
use std::time::{Duration, Instant};

use bumbledb::{Db, RelationId, Value};
use rusqlite::Connection;

use crate::corpus_gen::{GenConfig, Sizes, relation_rows};
use crate::schema::{Ledger, ids, schema};
use crate::sqlmap;

#[derive(Debug, Clone, Copy)]
pub struct LoadStats {
    pub facts: u64,
    pub wall: Duration,
    pub facts_per_sec: f64,
}

impl LoadStats {
    #[must_use]
    pub fn of(facts: u64, wall: Duration) -> Self {
        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )]
        let facts_per_sec = facts as f64 / wall.as_secs_f64().max(f64::EPSILON);
        Self {
            facts,
            wall,
            facts_per_sec,
        }
    }
}

/// # Errors
pub fn load_bumbledb(db: &Db<Ledger>, cfg: GenConfig) -> Result<LoadStats, bumbledb::Error> {
    let start = Instant::now();
    let mut facts = 0u64;
    for rel in 0..ids::RELATIONS {
        let rel = RelationId(rel);
        let work = bumbledb::start_operation(crate::harness::bench_policy())?;
        facts += db
            .write(work, |tx| {
                tx.insert_dyn(rel, relation_rows(cfg, rel))
                    .map(bumbledb::MutationReport::changed)
            })?
            .expect("load insert admits")
            .value;
    }
    Ok(LoadStats::of(facts, start.elapsed()))
}

/// # Errors
/// # Panics
/// If WAL refuses to engage — the fairness protocol is unconditional.
pub fn configure_sqlite(conn: &Connection) -> rusqlite::Result<()> {
    let mode: String =
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    assert_eq!(mode.to_lowercase(), "wal", "WAL must engage");
    conn.pragma_update(None, "synchronous", "FULL")?;
    conn.pragma_update(None, "fullfsync", "ON")?;
    conn.pragma_update(None, "checkpoint_fullfsync", "ON")?;
    conn.pragma_update(None, "cache_size", -262_144)?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    Ok(())
}

/// # Errors
/// # Panics
pub fn insert_rows(
    conn: &Connection,
    relation: &bumbledb::schema::Relation,
    rows: impl Iterator<Item = Vec<Value>>,
) -> rusqlite::Result<u64> {
    let insert = sqlmap::insert_sql(relation);
    let mut facts = 0u64;
    let mut rows = rows.peekable();
    while rows.peek().is_some() {
        conn.execute_batch("BEGIN IMMEDIATE")?;
        {
            let mut stmt = conn.prepare_cached(&insert)?;
            for row in rows.by_ref().take(4096) {
                stmt.execute(rusqlite::params_from_iter(sqlmap::to_sql_row(&row)))?;
                facts += 1;
            }
        }
        conn.execute_batch("COMMIT")?;
    }
    Ok(facts)
}

/// # Errors
/// # Panics
pub fn load_sqlite_relation(
    conn: &Connection,
    cfg: GenConfig,
    rel: RelationId,
) -> rusqlite::Result<u64> {
    insert_rows(conn, schema().relation(rel), relation_rows(cfg, rel))
}

/// # Errors
/// # Panics
/// Only on programmer-invariant violations (WAL refused; corpus values
pub fn load_sqlite(path: &Path, cfg: GenConfig) -> rusqlite::Result<(Connection, LoadStats)> {
    let conn = Connection::open(path)?;
    configure_sqlite(&conn)?;
    for statement in sqlmap::ddl(schema()) {
        conn.execute(&statement, [])?;
    }
    for statement in sqlmap::extension_ddl(&bumbledb::Theory::descriptor(Ledger)) {
        conn.execute(&statement, [])?;
    }

    let start = Instant::now();
    let mut facts = 0u64;
    for rel in 0..ids::RELATIONS {
        facts += load_sqlite_relation(&conn, cfg, RelationId(rel))?;
    }
    conn.execute_batch("ANALYZE")?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    Ok((conn, LoadStats::of(facts, start.elapsed())))
}

/// # Panics
pub fn assert_loaded_equal(db: &Db<Ledger>, conn: &Connection, cfg: GenConfig) {
    let schema = schema();
    let sizes = Sizes::of(cfg.scale);
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

    for rel in ids::RELATIONS..u32::try_from(schema.relations().len()).expect("small") {
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
        assert_eq!(ours as u64, theirs, "extension counts diverge for {name}");
        assert!(theirs > 0, "a closed relation is never empty: {name}");
    }

    let mut rng = crate::corpus_gen::Rng::new(cfg.seed ^ 0xA5A5);
    for _ in 0..100 {
        let i = rng.range(sizes.postings);
        let expected = crate::corpus_gen::row(&cfg, &sizes, ids::POSTING, i);
        let relation = schema.relation(ids::POSTING);
        let got: Vec<Value> = conn
            .query_row(
                "SELECT * FROM \"Posting\" WHERE \"id\" = ?1",
                [i64::try_from(i).expect("axiom")],
                |row| {
                    let mut values = Vec::new();
                    for (idx, field) in relation.fields().iter().enumerate() {
                        let raw: rusqlite::types::Value = row.get(idx)?;
                        values.push(
                            sqlmap::from_sql_value(&raw, &field.value_type)
                                .expect("mapped value decodes"),
                        );
                    }
                    Ok(values)
                },
            )
            .expect("sample fetch");
        assert_eq!(got, expected, "posting {i} diverges");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus_gen::Scale;

    #[test]
    fn both_stores_load_the_same_corpus() {
        let scratch = crate::fixture::TempDir::new("corpus-load");
        let dir = scratch.path();
        let cfg = GenConfig {
            seed: 1,
            // Loader parity needs all relations, not a benchmark-sized corpus.
            scale: Scale::Tiny,
        };
        let db = Db::create(&dir.join("db"), Ledger)
            .expect("create")
            .expect("accepted");
        let ours = load_bumbledb(&db, cfg).expect("bumbledb load");
        let (conn, theirs) = load_sqlite(&dir.join("oracle.sqlite"), cfg).expect("sqlite load");
        assert_eq!(ours.facts, theirs.facts);
        assert!(ours.facts_per_sec > 0.0 && theirs.facts_per_sec > 0.0);
        assert_loaded_equal(&db, &conn, cfg);

        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("pragma");
        assert_eq!(mode.to_lowercase(), "wal");
        let sync: i64 = conn
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .expect("pragma");
        assert_eq!(sync, 2, "FULL");

        drop((db, conn));
    }
}
