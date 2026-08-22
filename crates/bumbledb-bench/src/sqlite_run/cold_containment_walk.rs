use bumbledb::schema::ValueType;
use rusqlite::Connection;

use crate::corpus_gen::{GenConfig, Sizes};
use crate::harness::{self, Measurement};
use crate::schema::schema;
use crate::writebench::write_protocol;

use super::{PreparedFamily, sample_args};

/// `cold_containment_walk` on `SQLite`: the identical cold protocol — a write
/// commit (the org touch, mirroring `harness::org_touch`) before every
/// # Errors
/// # Panics
pub fn cold_containment_walk(conn: &Connection, cfg: GenConfig) -> Result<Measurement, String> {
    let family = crate::families::all()
        .iter()
        .find(|f| f.name == "containment_walk")
        .expect("containment_walk is registered");
    let query = (family.query)();
    let translated = crate::translate::translate(&query, schema(), &[])
        .map_err(|e| format!("translate: {e}"))?;

    let types = vec![ValueType::String, ValueType::I64];
    let mut prepared = PreparedFamily::new(conn, &translated, types)?;
    let mut rotation = harness::Rotation::new((family.params)(&cfg));

    let mut touch_id = Sizes::of(cfg.scale).orgs + 10_000_000;
    harness::measure_cold(
        write_protocol("cold_containment_walk"),
        || {
            let run = || -> rusqlite::Result<()> {
                conn.execute_batch("BEGIN IMMEDIATE")?;
                conn.prepare_cached("INSERT INTO \"Org\" VALUES (?1, ?2)")?
                    .execute(rusqlite::params![
                        i64::try_from(touch_id).expect("small"),
                        format!("__touch_{touch_id}"),
                    ])?;
                conn.execute_batch("COMMIT")
            };
            run().map_err(|e| format!("cold touch sqlite: {e}"))?;
            touch_id += 1;
            Ok(())
        },
        || sample_args(&mut prepared, rotation.next_set()),
    )
}
