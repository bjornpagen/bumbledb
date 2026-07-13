//! Calendar corpus loading (docs/architecture/60-validation.md § the
//! calendar benchmark): one generator stream, two stores, identical
//! contents — the ledger loader's discipline
//! ([`crate::corpus`]) applied to the second theory.
//!
//! The engine load respects the statements it is judged under:
//! containment targets precede their sources (accounts → persons →
//! calendars → events; working hours before the claims they cover), and
//! the `Attendance == Claim` discriminated-union cluster loads through
//! **joint chunked write transactions** — either relation alone violates
//! one `==` direction mid-load, exactly the target corpus's
//! `JournalEntry == ImportBatch` precedent (`verify::run`).

use std::path::Path;
use std::time::Instant;

use bumbledb::{Db, RelationId, Value};
use rusqlite::Connection;

use crate::calendar::corpus_gen::{CalSizes, du_cluster_rows, relation_rows_sized};
use crate::calendar::{Scheduling, ids, schema};
use crate::corpus::{LoadStats, configure_sqlite, insert_rows};
use crate::corpus_gen::GenConfig;
use crate::sqlmap;

/// The engine load order minus the `==` cluster: every containment's
/// target precedes its source, and `WorkHours` precedes the claims whose
/// coverage it proves.
const ORDER: [RelationId; 7] = [
    ids::ACCOUNT,
    ids::PERSON,
    ids::CALENDAR,
    ids::WORK_HOURS,
    ids::EVENT,
    ids::ROOM,
    ids::BOOKING,
];

/// The joint chunk size of the `==` cluster (the engine's bulk chunk).
const CHUNK: usize = 4096;

/// Loads the calendar corpus into a bumbledb store: the containment-safe
/// prefix through `bulk_load`, then the `Attendance == Claim` cluster
/// through joint chunked write transactions.
///
/// # Errors
///
/// Engine errors verbatim — a corpus load has no resume story;
/// regenerate.
pub fn load_bumbledb(db: &Db<Scheduling>, cfg: GenConfig) -> Result<LoadStats, bumbledb::Error> {
    load_bumbledb_sized(db, cfg, CalSizes::of(cfg.scale))
}

/// [`load_bumbledb`] with explicit sizes — the unit-corpus seam (tests
/// and the naive lane shrink every axis through [`CalSizes::unit`]).
///
/// # Errors
///
/// As [`load_bumbledb`].
pub fn load_bumbledb_sized(
    db: &Db<Scheduling>,
    cfg: GenConfig,
    sizes: CalSizes,
) -> Result<LoadStats, bumbledb::Error> {
    let start = Instant::now();
    let mut facts = 0u64;
    for rel in ORDER {
        facts += db.bulk_load(rel, relation_rows_sized(cfg, sizes, rel))?;
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

/// Commits one joint chunk of the `==` cluster.
fn flush(
    db: &Db<Scheduling>,
    pending: &mut Vec<(RelationId, Vec<Value>)>,
) -> Result<u64, bumbledb::Error> {
    if pending.is_empty() {
        return Ok(0);
    }
    db.write(|tx| {
        for (rel, row) in pending.iter() {
            tx.insert_dyn(*rel, row)?;
        }
        Ok(())
    })?;
    let facts = pending.len() as u64;
    pending.clear();
    Ok(facts)
}

/// Creates, configures, and loads the calendar `SQLite` mirror: DDL from
/// the schema descriptors plus the family-owned indexes, every relation
/// via [`crate::corpus::insert_rows`], then `ANALYZE` and a truncating
/// WAL checkpoint — the ledger mirror's exact recipe.
///
/// # Errors
///
/// `SQLite` errors verbatim.
///
/// # Panics
///
/// Only on programmer-invariant violations (WAL refused; corpus values
/// breaking the mapping axiom).
pub fn load_sqlite(path: &Path, cfg: GenConfig) -> rusqlite::Result<(Connection, LoadStats)> {
    let conn = Connection::open(path)?;
    configure_sqlite(&conn)?;
    load_sqlite_into(&conn, cfg, CalSizes::of(cfg.scale)).map(|stats| (conn, stats))
}

/// [`load_sqlite`] against an already-open connection with explicit
/// sizes — the unit-corpus and in-memory seams.
///
/// # Errors
///
/// `SQLite` errors verbatim.
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

/// The calendar mirror's DDL: the statement-derived plan, the closed
/// vocabularies' extension INSERTs (`Rsvp`/`Arm` — schema surface, not
/// corpus: a closed relation is never empty), plus the family-owned
/// indexes (the honest opponent gets every index its queries reward —
/// `crate::calendar::families::index_ddl`).
#[must_use]
pub fn ddl() -> Vec<String> {
    let mut statements = sqlmap::schema_ddl(schema());
    statements.extend(sqlmap::extension_ddl(&bumbledb::Theory::descriptor(
        crate::calendar::Scheduling,
    )));
    statements.extend(crate::calendar::families::index_ddl());
    statements
}

/// Cross-store equality: per-relation counts against the generator's
/// derived sizes, then a seeded sample of events fetched from `SQLite`
/// by fresh id and compared value-for-value against the generator.
///
/// # Panics
///
/// On any inequality — this is test/verify support, not a soft check.
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

    // 100 seeded sample events, fetched from SQLite by id, compared to
    // the generator's row (engine equality to the generator is covered
    // transitively by counts + set semantics + the verify layer).
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
                        if matches!(
                            field.value_type,
                            bumbledb::schema::ValueType::Interval { element }
                                if matches!(element, bumbledb::schema::IntervalElement::I64)
                        ) {
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
