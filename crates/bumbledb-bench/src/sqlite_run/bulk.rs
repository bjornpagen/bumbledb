use std::path::Path;

use rusqlite::Connection;

use crate::gen::GenConfig;
use crate::harness::{self, Measurement};
use crate::schema::schema;
use crate::writebench::{non_posting_relations, write_protocol};
use crate::{corpus, sqlmap};

/// `bulk` on `SQLite`: pre-seeded throwaway files (the corpus minus
/// postings, built before any timing), the full posting stream timed in
/// 4096-row transactions per sample.
///
/// # Errors
///
/// `SQLite` errors, stringified.
///
/// # Panics
///
/// On scratch I/O failures.
pub fn bulk(cfg: GenConfig, scratch: &Path) -> Result<Measurement, String> {
    use std::cell::RefCell;
    let proto = write_protocol("bulk");
    let mut pending = std::collections::VecDeque::new();
    for sample in 0..proto.warmups + proto.samples {
        let path = scratch.join(format!("bulk-oracle-{sample}.sqlite"));
        let conn = Connection::open(&path).map_err(|e| format!("open: {e}"))?;
        corpus::configure_sqlite(&conn).map_err(|e| format!("configure: {e}"))?;
        for statement in sqlmap::ddl(schema()) {
            conn.execute(&statement, [])
                .map_err(|e| format!("ddl: {e}"))?;
        }
        for rel in non_posting_relations() {
            corpus::load_sqlite_relation(&conn, cfg, rel).map_err(|e| format!("seed: {e}"))?;
        }
        pending.push_back(conn);
    }
    let pending = RefCell::new(pending);
    let done = RefCell::new(Vec::new());
    harness::measure(proto, || {
        let conn = pending.borrow_mut().pop_front().expect("pre-seeded store");
        let mut facts = corpus::load_sqlite_relation(&conn, cfg, crate::schema::ids::POSTING)
            .map_err(|e| format!("bulk sqlite: {e}"))?;
        facts += corpus::load_sqlite_relation(&conn, cfg, crate::schema::ids::POSTING_TAG)
            .map_err(|e| format!("bulk sqlite tags: {e}"))?;
        done.borrow_mut().push(conn);
        Ok(facts)
    })
}
