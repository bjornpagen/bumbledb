use rusqlite::Connection;

use bumbledb::schema::ValueType;

use crate::corpus_gen::{GenConfig, Rng, Sizes};
use crate::harness::{self, Measurement};
use crate::schema::schema;
use crate::writebench::write_protocol;

use super::commits::sqlite_posting_params;
use super::{POSTING_INSERT, PreparedFamily, sample_args};

/// protocol behind the identical delete-bearing touch — one
/// # Errors
/// # Panics
pub fn cold_containment_walk_delete(
    conn: &Connection,
    cfg: GenConfig,
) -> Result<Measurement, String> {
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
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0004);

    let mut prev_id = sizes.postings + 20_000_000;

    let mut seed_row = || -> rusqlite::Result<()> {
        conn.execute_batch("BEGIN IMMEDIATE")?;
        conn.prepare_cached(POSTING_INSERT)?
            .execute(sqlite_posting_params(&mut rng, &sizes, prev_id))?;
        conn.execute_batch("COMMIT")
    };
    seed_row().map_err(|e| format!("swap seed sqlite: {e}"))?;
    let mut next_id = prev_id + 1;
    harness::measure_cold(
        write_protocol("cold_containment_walk_delete"),
        || {
            let mut run = || -> rusqlite::Result<usize> {
                conn.execute_batch("BEGIN IMMEDIATE")?;
                let deleted = conn
                    .prepare_cached("DELETE FROM \"Posting\" WHERE \"id\" = ?1")?
                    .execute([i64::try_from(prev_id).expect("small")])?;
                conn.prepare_cached(POSTING_INSERT)?
                    .execute(sqlite_posting_params(&mut rng, &sizes, next_id))?;
                conn.execute_batch("COMMIT")?;
                Ok(deleted)
            };
            let deleted = run().map_err(|e| format!("swap touch sqlite: {e}"))?;
            if deleted != 1 {
                return Err(
                    "the swap touch must be delete-bearing: the previous revision was absent"
                        .to_owned(),
                );
            }
            prev_id = next_id;
            next_id += 1;
            Ok(())
        },
        || sample_args(&mut prepared, rotation.next_set()),
    )
}
