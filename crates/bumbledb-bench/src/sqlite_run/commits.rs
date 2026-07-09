use rusqlite::Connection;

use crate::gen::{GenConfig, Rng, Sizes};
use crate::harness::{self, Measurement};
use crate::schema::PostingId;
use crate::writebench::{seeded_posting, write_protocol};

use super::POSTING_INSERT;

fn sqlite_posting_params(rng: &mut Rng, sizes: &Sizes, id: u64) -> [rusqlite::types::Value; 6] {
    use rusqlite::types::Value as Sql;
    let posting = seeded_posting(rng, sizes, PostingId(id));
    [
        Sql::Integer(i64::try_from(id).expect("axiom")),
        Sql::Integer(i64::try_from(posting.entry.0).expect("axiom")),
        Sql::Integer(i64::try_from(posting.account.0).expect("axiom")),
        Sql::Integer(i64::try_from(posting.instrument.0).expect("axiom")),
        Sql::Integer(posting.amount),
        Sql::Integer(posting.at),
    ]
}

fn next_posting_id(conn: &Connection) -> Result<u64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(\"id\"), -1) + 1 FROM \"Posting\"",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|next| u64::try_from(next).expect("dense ids"))
    .map_err(|e| format!("next id: {e}"))
}

/// `commit_single` on `SQLite`: one sample = one bound `INSERT` on a
/// reused prepared statement inside `BEGIN IMMEDIATE … COMMIT` (the WAL +
/// `synchronous=FULL` session — the same fsync bill the engine pays).
///
/// # Errors
///
/// `SQLite` errors, stringified.
pub fn commit_single(conn: &Connection, cfg: GenConfig) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0001);
    let mut next = next_posting_id(conn)?;
    harness::measure(write_protocol("commit_single"), || {
        let mut run = || -> rusqlite::Result<()> {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            conn.prepare_cached(POSTING_INSERT)?
                .execute(sqlite_posting_params(&mut rng, &sizes, next))?;
            conn.execute_batch("COMMIT")
        };
        run().map_err(|e| format!("commit_single sqlite: {e}"))?;
        next += 1;
        Ok(1)
    })
}

/// `commit_batch` on `SQLite`: one sample = 512 bound executions in one
/// transaction.
///
/// # Errors
///
/// `SQLite` errors, stringified.
pub fn commit_batch(conn: &Connection, cfg: GenConfig) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0002);
    let mut next = next_posting_id(conn)?;
    harness::measure(write_protocol("commit_batch"), || {
        let mut run = || -> rusqlite::Result<()> {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            {
                let mut stmt = conn.prepare_cached(POSTING_INSERT)?;
                for _ in 0..512 {
                    stmt.execute(sqlite_posting_params(&mut rng, &sizes, next))?;
                    next += 1;
                }
            }
            conn.execute_batch("COMMIT")
        };
        run().map_err(|e| format!("commit_batch sqlite: {e}"))?;
        Ok(512)
    })
}
