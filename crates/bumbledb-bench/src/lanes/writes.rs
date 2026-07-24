//! The writes lane: bulk append, commits/sec at batch 1/10/100/1000, and
//! delete throughput, across the two durability lanes — REPORT-class
//! ([`crate::lanes`] carries the charter: numbers are claimed only from
//! the owner's measurement sessions; a tool run never times for
//! publication).
//!
//! The durability axis is [`crate::duralane::DurabilityLane`] — the one
//! constructor of both sides' config and the authority for every pragma
//! (`docs/architecture/60-validation.md`; the writes-local twin enum
//! died into it, finding 071) — so a lane/pragma cross-match is
//! unrepresentable, and every report row rides inside a lane object
//! carrying its `lane` and `sqlite_sync` labels: a number can never be
//! quoted without its durability context.
//!
//! **The `SQLite` parity config, per lane** (carried in the report
//! labels): both lanes load their oracle twin through
//! [`crate::corpus::load_sqlite`] — WAL asserted, `synchronous=FULL`,
//! `fullfsync=ON`, `checkpoint_fullfsync=ON`, 256 MiB page cache,
//! `temp_store=MEMORY`, prepared statements reused via `prepare_cached`,
//! `ANALYZE` after load, `wal_checkpoint(TRUNCATE)` after load — then
//! [`DurabilityLane::configure`] applies the lane's whole session
//! envelope (the sync trio per arm, plus the shared whole-file mmap and
//! `wal_autocheckpoint=0`) and [`DurabilityLane::assert_parity`] reads
//! the pragmas back: a misconfigured twin fails before flattering
//! anyone.
//!
//! **Post-state verification is arithmetic over representations, not
//! spot-checking:** both engines consume the identical seeded
//! posting-body stream (one [`Rng`], one seed constant per family), so
//! the expected post-state is exactly `corpus + inserted − deleted`
//! counts, plus a body-multiset equality with ids projected OUT — the
//! engine fresh-mints ids (never reissued) and `SQLite` mints `MAX+1`;
//! those are different representations of the same bodies, so comparing
//! bodies, not ids, is the honest equality ([`verify_post_state`]).
//!
//! **The delete lane is delete-bearing BY CONTRACT** (the
//! [`crate::writebench::posting_swap`] precedent): a no-op delete
//! returns `Err` inside the write closure, so the transaction aborts
//! whole and the lane can never silently degrade into an insert-only
//! measurement.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use bumbledb::{Db, Value};
use rusqlite::Connection;

use crate::corpus_gen::{GenConfig, Rng, Sizes};
use crate::duralane::DurabilityLane;
use crate::harness::{self, Measurement, Protocol, Stats};
use crate::json;
use crate::report::{GhzReport, Provenance};
use crate::schema::{Ledger, Posting, PostingId, ids, schema};
use crate::sqlite_run::POSTING_INSERT;
use crate::{clockproxy, corpus, sqlmap, writebench};

/// The whole writes report, plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct WritesReport {
    pub provenance: Provenance,
    pub scale: &'static str,
    pub seed: u64,
    pub samples: u32,
    pub lanes: Vec<LaneReport>,
}

/// One durability lane's ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneReport {
    pub lane: &'static str,
    pub sqlite_sync: &'static str,
    pub rows: Vec<WriteRow>,
}

/// One (family, batch) cell, both engines.
#[derive(Debug, Clone, PartialEq)]
pub struct WriteRow {
    pub name: String,
    pub batch: u32,
    pub ours: Stats,
    pub theirs: Stats,
    pub commits_per_sec_ours: f64,
    pub commits_per_sec_theirs: f64,
    pub rows_per_sec_ours: f64,
    pub rows_per_sec_theirs: f64,
    pub ghz: Option<GhzReport>,
}

fn push_row(out: &mut String, row: &WriteRow) {
    out.push_str("{\"name\":");
    json::push_str_lit(out, &row.name);
    let _ = write!(out, ",\"batch\":{},\"ours\":", row.batch);
    super::push_stats(out, &row.ours);
    out.push_str(",\"theirs\":");
    super::push_stats(out, &row.theirs);
    let _ = write!(
        out,
        ",\"commits_per_sec_ours\":{:.2},\"commits_per_sec_theirs\":{:.2},\"rows_per_sec_ours\":{:.2},\"rows_per_sec_theirs\":{:.2}",
        row.commits_per_sec_ours,
        row.commits_per_sec_theirs,
        row.rows_per_sec_ours,
        row.rows_per_sec_theirs,
    );
    super::push_ghz(out, row.ghz);
    out.push('}');
}

/// The machine-consumable writes artifact — hand-rolled, like
/// `report/json_out.rs`.
#[must_use]
pub fn to_json(report: &WritesReport) -> String {
    let mut out = String::new();
    out.push_str("{\"provenance\":");
    super::push_provenance(&mut out, &report.provenance);
    let _ = write!(
        out,
        ",\"scale\":\"{}\",\"seed\":{},\"samples\":{},\"lanes\":[",
        report.scale, report.seed, report.samples
    );
    for (index, lane) in report.lanes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"lane\":\"{}\",\"sqlite_sync\":\"{}\",\"rows\":[",
            lane.lane, lane.sqlite_sync
        );
        for (row_index, row) in lane.rows.iter().enumerate() {
            if row_index > 0 {
                out.push(',');
            }
            push_row(&mut out, row);
        }
        out.push_str("]}");
    }
    out.push_str("]}");
    out
}

/// The human artifact: one table per lane, the parity labels in the
/// heading so no number travels without its durability context.
fn to_markdown(report: &WritesReport) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# writes lane — scale {}, seed {}, samples {}",
        report.scale, report.seed, report.samples
    );
    for lane in &report.lanes {
        let _ = writeln!(
            out,
            "\n## lane `{}` — sqlite `{}`\n",
            lane.lane, lane.sqlite_sync
        );
        out.push_str(
            "| family | batch | ours p50 ns | sqlite p50 ns | ours commits/s | sqlite commits/s | ours rows/s | sqlite rows/s |\n",
        );
        out.push_str("|---|---:|---:|---:|---:|---:|---:|---:|\n");
        for row in &lane.rows {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {:.1} | {:.1} | {:.1} | {:.1} |",
                row.name,
                row.batch,
                row.ours.p50,
                row.theirs.p50,
                row.commits_per_sec_ours,
                row.commits_per_sec_theirs,
                row.rows_per_sec_ours,
                row.rows_per_sec_theirs,
            );
        }
    }
    out
}

/// The commit ladder's seed page (the writebench `cfg.seed ^
/// 0x0115_000N` idiom, this lane's own page): each batch point draws its
/// own stream — `cfg.seed ^ COMMIT_SEED ^ batch` — consumed verbatim by
/// BOTH engines, so their inserted bodies are identical by construction.
const COMMIT_SEED: u64 = 0x0117_0000;

/// The delete pre-phase's seed page — its own page, so the ladder and
/// delete streams never overlap.
const DELETE_SEED: u64 = 0x0117_0100;

/// The bulk transaction chunk (rows per commit inside one bulk sample):
/// the engine's `bulk_load` chunk and the [`corpus::insert_rows`] mirror
/// both commit 4096 rows per transaction — the `bulk_append` row's
/// `batch` column reports it, and its commits/sec derives from it.
const BULK_TX_CHUNK: u32 = 4096;

/// The `SQLite` posting delete, mirroring [`POSTING_INSERT`]'s shape.
const POSTING_DELETE: &str = "DELETE FROM \"Posting\" WHERE \"id\" = ?1";

/// One committed transaction per sample, so the mean sample time
/// inverts to commits/sec.
#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn commits_per_sec(stats: &Stats) -> f64 {
    1e9 / (stats.mean_ns.max(1) as f64)
}

/// One ladder cell from its pair of measurements: `commits_per_sec =
/// 1e9 / mean_ns`, `rows_per_sec = commits_per_sec × batch`.
fn ladder_row(
    name: String,
    batch: u32,
    ours: Stats,
    theirs: Stats,
    ghz: Option<GhzReport>,
) -> WriteRow {
    let cps_ours = commits_per_sec(&ours);
    let cps_theirs = commits_per_sec(&theirs);
    WriteRow {
        name,
        batch,
        ours,
        theirs,
        commits_per_sec_ours: cps_ours,
        commits_per_sec_theirs: cps_theirs,
        rows_per_sec_ours: cps_ours * f64::from(batch),
        rows_per_sec_theirs: cps_theirs * f64::from(batch),
        ghz,
    }
}

/// The `SQLite` side's `MAX+1` id mint (the `sqlite_run/commits.rs`
/// shape): dense corpus ids make `MAX+1` a valid fresh id.
fn next_posting_id(conn: &Connection) -> Result<u64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(\"id\"), -1) + 1 FROM \"Posting\"",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|next| u64::try_from(next).expect("dense ids"))
    .map_err(|e| format!("next id: {e}"))
}

/// One posting body as the `SQLite` insert's positional params.
fn posting_params(posting: &Posting) -> [rusqlite::types::Value; 6] {
    use rusqlite::types::Value as Sql;
    [
        Sql::Integer(i64::try_from(posting.id.0).expect("axiom")),
        Sql::Integer(i64::try_from(posting.entry.0).expect("axiom")),
        Sql::Integer(i64::try_from(posting.account.0).expect("axiom")),
        Sql::Integer(i64::try_from(posting.instrument.0).expect("axiom")),
        Sql::Integer(posting.amount),
        Sql::Integer(posting.at),
    ]
}

/// `commit_b{batch}` on the engine: one sample = one `db.write`
/// allocating and inserting `batch` seeded postings — the
/// `commit_batch_bumbledb` shape generalized over the ladder.
fn commit_engine(
    db: &Db<Ledger>,
    cfg: GenConfig,
    proto: Protocol,
    batch: u32,
) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ COMMIT_SEED ^ u64::from(batch));
    harness::measure(proto, || {
        db.write(|tx| {
            for _ in 0..batch {
                let id: PostingId = tx.alloc()?;
                tx.insert(&writebench::prepared_posting(&mut rng, &sizes, id))?;
            }
            Ok(())
        })
        .map(|()| u64::from(batch))
        .map_err(|e| format!("commit_b{batch}: {e:?}"))
    })
}

/// `commit_b{batch}` on `SQLite`: one sample = `BEGIN IMMEDIATE` +
/// `batch` bound executions of the reused prepared insert + `COMMIT`,
/// drawing from THE SAME rng stream as the engine and minting ids
/// `MAX+1` (the `sqlite_run/commits.rs` shape).
fn commit_sqlite(
    conn: &Connection,
    cfg: GenConfig,
    proto: Protocol,
    batch: u32,
) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ COMMIT_SEED ^ u64::from(batch));
    let mut next = next_posting_id(conn)?;
    harness::measure(proto, || {
        let mut run = || -> rusqlite::Result<()> {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            {
                let mut stmt = conn.prepare_cached(POSTING_INSERT)?;
                for _ in 0..batch {
                    let body = writebench::prepared_posting(&mut rng, &sizes, PostingId(next));
                    stmt.execute(posting_params(&body))?;
                    next += 1;
                }
            }
            conn.execute_batch("COMMIT")
        };
        run().map_err(|e| format!("commit_b{batch} sqlite: {e}"))?;
        Ok(u64::from(batch))
    })
}

/// The delete pre-phase (untimed): inserts exactly `total` seeded
/// postings through the engine path in chunked commits, RECORDING each
/// committed body; then mirrors the SAME bodies (recorded, never
/// re-drawn) into `SQLite`, ids minted `MAX+1` and remembered in a
/// parallel deque. These are the lane's OWN rows — the corpus's
/// `PostingTag`s reference corpus ids only, so deleting them is
/// containment-safe by construction (the `posting_swap` precedent).
/// Rows are consumed in insertion order on both sides, so the surviving
/// multisets stay equal.
fn seed_delete_rows(
    db: &Db<Ledger>,
    conn: &Connection,
    cfg: GenConfig,
    total: u64,
    batch: u32,
) -> Result<(VecDeque<Posting>, VecDeque<u64>), String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ DELETE_SEED ^ u64::from(batch));
    let mut recorded: VecDeque<Posting> = VecDeque::new();
    let mut remaining = total;
    while remaining > 0 {
        let chunk = remaining.min(1024);
        let committed = db
            .write(|tx| {
                let mut out = Vec::with_capacity(usize::try_from(chunk).expect("small chunk"));
                for _ in 0..chunk {
                    let id: PostingId = tx.alloc()?;
                    let posting = writebench::prepared_posting(&mut rng, &sizes, id);
                    tx.insert(&posting)?;
                    out.push(posting);
                }
                Ok(out)
            })
            .map_err(|e| format!("delete_b{batch} pre-phase: {e:?}"))?;
        recorded.extend(committed);
        remaining -= chunk;
    }
    let mut mirrored: VecDeque<u64> = VecDeque::new();
    let mut next = next_posting_id(conn)?;
    let mut run = || -> rusqlite::Result<()> {
        conn.execute_batch("BEGIN IMMEDIATE")?;
        {
            let mut stmt = conn.prepare_cached(POSTING_INSERT)?;
            for posting in &recorded {
                let twin = Posting {
                    id: PostingId(next),
                    ..posting.clone()
                };
                stmt.execute(posting_params(&twin))?;
                mirrored.push_back(next);
                next += 1;
            }
        }
        conn.execute_batch("COMMIT")
    };
    run().map_err(|e| format!("delete_b{batch} pre-phase sqlite: {e}"))?;
    Ok((recorded, mirrored))
}

/// One timed delete commit on the engine: pops `batch` recorded
/// postings and deletes each inside ONE `db.write`. Delete-bearing BY
/// CONTRACT (the [`crate::writebench::posting_swap`] precedent): a
/// no-op delete returns `Err` INSIDE the closure — the in-closure
/// sentinel abort drops the delta whole, so a refused delete never
/// commits the batch's earlier deletes, and the lane can never silently
/// degrade into an insert-only (or partial) measurement.
///
/// # Panics
///
/// On an undersized deque (a programmer error — the pre-phase sizes it
/// to `(warmups + samples) × batch` exactly).
fn delete_recorded(
    db: &Db<Ledger>,
    recorded: &mut VecDeque<Posting>,
    batch: u32,
) -> Result<u64, String> {
    db.write(|tx| {
        for _ in 0..batch {
            let victim = recorded
                .pop_front()
                .expect("the pre-phase sized the deque to (warmups + samples) × batch exactly");
            if !tx.delete(&victim)? {
                // The in-closure sentinel abort (the posting_swap
                // idiom): returning `Err` here drops the delta whole,
                // so nothing this sample deleted ever reaches the
                // store.
                return Err(bumbledb::Error::Io(std::io::Error::other(
                    "the delete lane must be delete-bearing: a recorded posting was absent",
                )));
            }
        }
        Ok(())
    })
    .map(|()| u64::from(batch))
    .map_err(|e| format!("delete_b{batch}: {e:?}"))
}

/// `delete_b{batch}` on `SQLite`: one sample = `BEGIN IMMEDIATE` +
/// `batch` bound deletes popping the parallel deque — each asserted to
/// affect exactly one row — + `COMMIT`.
fn delete_sqlite(
    conn: &Connection,
    mirrored: &mut VecDeque<u64>,
    proto: Protocol,
    batch: u32,
) -> Result<Measurement, String> {
    harness::measure(proto, || {
        let mut run = || -> Result<(), String> {
            conn.execute_batch("BEGIN IMMEDIATE")
                .map_err(|e| e.to_string())?;
            {
                let mut stmt = conn
                    .prepare_cached(POSTING_DELETE)
                    .map_err(|e| e.to_string())?;
                for _ in 0..batch {
                    let id = mirrored
                        .pop_front()
                        .expect("the mirror deque is sized like the engine's");
                    let affected = stmt
                        .execute([i64::try_from(id).expect("axiom")])
                        .map_err(|e| e.to_string())?;
                    if affected != 1 {
                        return Err(format!("id {id} affected {affected} rows (must be 1)"));
                    }
                }
            }
            conn.execute_batch("COMMIT").map_err(|e| e.to_string())
        };
        run().map_err(|e| format!("delete_b{batch} sqlite: {e}"))?;
        Ok(u64::from(batch))
    })
}

/// `bulk_append` on `SQLite`, lane-local (`sqlite_run::bulk` hardwires
/// [`corpus::configure_sqlite`] = FULL, so this variant applies the
/// lane's pragmas after the standing config on every throwaway file):
/// pre-seeded throwaway files (the corpus minus postings, built before
/// any timing), the full posting stream timed in
/// [`BULK_TX_CHUNK`]-row transactions per sample.
fn bulk_sqlite(
    cfg: GenConfig,
    scratch: &Path,
    lane: DurabilityLane,
) -> Result<Measurement, String> {
    use std::cell::RefCell;
    let proto = writebench::write_protocol("bulk");
    let mut pending = VecDeque::new();
    for sample in 0..proto.warmups + proto.samples {
        let path = scratch.join(format!("bulk-oracle-{sample}.sqlite"));
        let conn = Connection::open(&path).map_err(|e| format!("open: {e}"))?;
        corpus::configure_sqlite(&conn).map_err(|e| format!("configure: {e}"))?;
        lane.configure(&conn)?;
        lane.assert_parity(&conn)?;
        for statement in sqlmap::ddl(schema()) {
            conn.execute(&statement, [])
                .map_err(|e| format!("ddl: {e}"))?;
        }
        for rel in writebench::non_posting_relations() {
            corpus::load_sqlite_relation(&conn, cfg, rel).map_err(|e| format!("seed: {e}"))?;
        }
        pending.push_back(conn);
    }
    let pending = RefCell::new(pending);
    let done = RefCell::new(Vec::new());
    harness::measure(proto, || {
        let conn = pending.borrow_mut().pop_front().expect("pre-seeded store");
        let mut facts = corpus::load_sqlite_relation(&conn, cfg, ids::POSTING)
            .map_err(|e| format!("bulk sqlite: {e}"))?;
        facts += corpus::load_sqlite_relation(&conn, cfg, ids::POSTING_TAG)
            .map_err(|e| format!("bulk sqlite tags: {e}"))?;
        done.borrow_mut().push(conn);
        Ok(facts)
    })
}

/// The bulk throwaway symmetry re-check: bulk runs against throwaway
/// pairs, not the lane db, so [`verify_post_state`] cannot see it — its
/// generator-stream count is already pinned by writebench's own test,
/// and this cheap witness re-opens pair 0 (the durable lane through
/// `Db::open`, the nosync lane through `Db::ephemeral`, its
/// create-or-open constructor) and confirms both sides hold exactly the
/// posting mass.
fn verify_bulk_pair(
    scratch: &Path,
    lane: DurabilityLane,
    expected_postings: u64,
) -> Result<(), String> {
    let dir = scratch.join("bulk-bumbledb-0");
    let db = match lane.store_mode() {
        crate::storemode::StoreMode::Durable => Db::open(&dir, Ledger),
        crate::storemode::StoreMode::Ephemeral => Db::ephemeral(&dir, Ledger),
    }
    .map_err(|e| format!("bulk re-open ({}): {e:?}", lane.label()))?;
    let ours = db
        .read(|snap| Ok(snap.scan(ids::POSTING)?.count()))
        .map_err(|e| format!("bulk re-scan: {e:?}"))? as u64;
    let conn = Connection::open(scratch.join("bulk-oracle-0.sqlite"))
        .map_err(|e| format!("bulk oracle re-open: {e}"))?;
    let theirs: i64 = conn
        .query_row("SELECT COUNT(*) FROM \"Posting\"", [], |row| row.get(0))
        .map_err(|e| format!("bulk oracle count: {e}"))?;
    let theirs = u64::try_from(theirs).map_err(|e| format!("bulk oracle count: {e}"))?;
    if ours != expected_postings || theirs != expected_postings {
        return Err(format!(
            "bulk pair 0 diverges ({}): engine {ours} vs sqlite {theirs} vs expected \
             {expected_postings} postings",
            lane.label()
        ));
    }
    Ok(())
}

/// One posting cell as `u64`, with the arm named on mismatch.
fn cell_u64(row: &[Value], index: usize) -> Result<u64, String> {
    match row.get(index) {
        Some(Value::U64(v)) => Ok(*v),
        other => Err(format!(
            "posting cell {index}: expected u64, found {other:?}"
        )),
    }
}

/// One posting cell as `i64`, with the arm named on mismatch.
fn cell_i64(row: &[Value], index: usize) -> Result<i64, String> {
    match row.get(index) {
        Some(Value::I64(v)) => Ok(*v),
        other => Err(format!(
            "posting cell {index}: expected i64, found {other:?}"
        )),
    }
}

/// A posting body with the id projected out — the shared representation
/// both engines' minted rows reduce to.
type Body = (u64, u64, u64, i64, i64);

/// The post-state gate — arithmetic over representations, not
/// spot-checking:
///
/// 1. engine `scan(Posting).count()` == sqlite `COUNT(*)` ==
///    `expected_postings` (corpus + inserted − deleted, tracked exactly
///    by the runners);
/// 2. body-multiset equality above the corpus id ceiling, ids projected
///    OUT by design: the engine fresh-mints ids (never reissued) and
///    the mirror mints `MAX+1` — different representations of the same
///    bodies, so the (entry, account, instrument, amount, at)
///    projection is the honest shared equality (sorted-vec compare).
///
/// # Errors
///
/// Any inequality, naming the counts (the caller brands the lane).
fn verify_post_state(
    db: &Db<Ledger>,
    conn: &Connection,
    corpus_ceiling: u64,
    expected_postings: u64,
) -> Result<(), String> {
    let engine_rows: Vec<Vec<Value>> = db
        .read(|snap| snap.scan(ids::POSTING)?.collect())
        .map_err(|e| format!("engine scan: {e:?}"))?;
    let ours_count = engine_rows.len() as u64;
    let theirs_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM \"Posting\"", [], |row| row.get(0))
        .map_err(|e| format!("sqlite count: {e}"))?;
    let theirs_count = u64::try_from(theirs_count).map_err(|e| format!("sqlite count: {e}"))?;
    if ours_count != expected_postings || theirs_count != expected_postings {
        return Err(format!(
            "posting counts diverge: engine {ours_count}, sqlite {theirs_count}, \
             expected {expected_postings}"
        ));
    }
    let mut ours: Vec<Body> = Vec::new();
    for row in &engine_rows {
        if cell_u64(row, 0)? >= corpus_ceiling {
            ours.push((
                cell_u64(row, 1)?,
                cell_u64(row, 2)?,
                cell_u64(row, 3)?,
                cell_i64(row, 4)?,
                cell_i64(row, 5)?,
            ));
        }
    }
    let mut stmt = conn
        .prepare(
            "SELECT \"entry\", \"account\", \"instrument\", \"amount\", \"at\" \
             FROM \"Posting\" WHERE \"id\" >= ?1",
        )
        .map_err(|e| e.to_string())?;
    let mut theirs: Vec<Body> = stmt
        .query_map([i64::try_from(corpus_ceiling).expect("axiom")], |row| {
            Ok((
                u64::try_from(row.get::<_, i64>(0)?).expect("axiom"),
                u64::try_from(row.get::<_, i64>(1)?).expect("axiom"),
                u64::try_from(row.get::<_, i64>(2)?).expect("axiom"),
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<_>>()
        .map_err(|e| e.to_string())?;
    ours.sort_unstable();
    theirs.sort_unstable();
    if ours != theirs {
        return Err(format!(
            "the post-corpus posting bodies diverge (ids projected out): engine holds {} \
             rows above id {corpus_ceiling}, sqlite {}",
            ours.len(),
            theirs.len()
        ));
    }
    Ok(())
}

/// One durability lane, whole: seed the twin pair, run the commit
/// ladder, run the delete ladder, verify the post-state, then bulk —
/// LAST, always (seconds of fsync leave the deepest clock shadow;
/// nothing measures after it — the `write_families` order pin, carried
/// here by the same `debug_assert!`).
fn run_lane(
    lane: DurabilityLane,
    cfg: GenConfig,
    proto: Protocol,
    batches: &[u32],
    scratch: &Path,
) -> Result<LaneReport, String> {
    std::fs::create_dir_all(scratch).map_err(|e| format!("scratch: {e}"))?;
    let sizes = Sizes::of(cfg.scale);

    // (a) The seeded referenced-rows store: the whole corpus (the
    // non-posting containment targets PLUS the posting mass and its
    // tags), so the post-state arithmetic starts from the known corpus
    // counts.
    eprintln!(
        "bench: writes {} — loading the scratch corpus",
        lane.label()
    );
    let db = lane.store_mode().create(&scratch.join("db"), Ledger)?;
    corpus::load_bumbledb(&db, cfg).map_err(|e| format!("load ({}): {e:?}", lane.label()))?;
    // The oracle twin: fresh file, the standing parity config
    // (WAL/FULL/fullfsync/cache/temp_store), full DDL + extension DDL,
    // every relation loaded, ANALYZE, truncating WAL checkpoint — then
    // the lane's pragmas.
    let (conn, _) = corpus::load_sqlite(&scratch.join("oracle.sqlite"), cfg)
        .map_err(|e| format!("oracle load ({}): {e}", lane.label()))?;
    lane.configure(&conn)?;
    lane.assert_parity(&conn)?;

    let per_family = u64::from(proto.warmups + proto.samples);
    let mut inserted = 0u64;
    let mut deleted = 0u64;
    let mut rows = Vec::new();

    // (b) The commit ladder: one family per batch point, both engines
    // bracketed by one proxy stamp (the write_families precedent).
    for &batch in batches {
        let name = format!("commit_b{batch}");
        eprintln!("bench: writes {} — {name}", lane.label());
        let ((ours, theirs), ghz) = clockproxy::stamped(|| {
            Ok((
                commit_engine(&db, cfg, proto, batch)?,
                commit_sqlite(&conn, cfg, proto, batch)?,
            ))
        })?;
        inserted += per_family * u64::from(batch);
        rows.push(ladder_row(
            name,
            batch,
            ours.stats,
            theirs.stats,
            Some(ghz.into()),
        ));
    }

    // (c) Delete throughput: the same batch ladder, delete-bearing by
    // contract; the pre-phase is untimed.
    for &batch in batches {
        let name = format!("delete_b{batch}");
        eprintln!("bench: writes {} — {name}", lane.label());
        let total = per_family * u64::from(batch);
        let (mut recorded, mut mirrored) = seed_delete_rows(&db, &conn, cfg, total, batch)?;
        inserted += total;
        let ((ours, theirs), ghz) = clockproxy::stamped(|| {
            Ok((
                harness::measure(proto, || delete_recorded(&db, &mut recorded, batch))?,
                delete_sqlite(&conn, &mut mirrored, proto, batch)?,
            ))
        })?;
        if !recorded.is_empty() || !mirrored.is_empty() {
            return Err(format!(
                "delete_b{batch}: {} engine / {} sqlite rows survived the ladder \
                 (the deques must drain exactly)",
                recorded.len(),
                mirrored.len()
            ));
        }
        deleted += total;
        rows.push(ladder_row(
            name,
            batch,
            ours.stats,
            theirs.stats,
            Some(ghz.into()),
        ));
    }

    // (e) Post-state verification — BEFORE bulk: bulk runs on throwaway
    // stores, not the lane db (its pairs verify in the symmetry
    // re-check below). The gate must pass before the lane's rows are
    // accepted.
    let expected = sizes.postings + inserted - deleted;
    verify_post_state(&db, &conn, sizes.postings, expected)
        .map_err(|e| format!("post-state ({}): {e}", lane.label()))?;

    // (d) BULK — LAST in the lane (the fsync-shadow law).
    eprintln!("bench: writes {} — bulk_append", lane.label());
    let bulk_scratch = scratch.join("bulk");
    std::fs::create_dir_all(&bulk_scratch).map_err(|e| format!("bulk scratch: {e}"))?;
    let bulk_proto = writebench::write_protocol("bulk");
    let ((ours, theirs), ghz) = clockproxy::stamped(|| {
        Ok((
            writebench::bulk_bumbledb(cfg, &bulk_scratch, lane.store_mode())?,
            bulk_sqlite(cfg, &bulk_scratch, lane)?,
        ))
    })?;
    verify_bulk_pair(&bulk_scratch, lane, sizes.postings)?;
    let ours_rate = harness::facts_per_sec(&ours, bulk_proto.samples);
    let theirs_rate = harness::facts_per_sec(&theirs, bulk_proto.samples);
    rows.push(WriteRow {
        name: "bulk_append".to_owned(),
        batch: BULK_TX_CHUNK,
        ours: ours.stats,
        theirs: theirs.stats,
        commits_per_sec_ours: ours_rate / f64::from(BULK_TX_CHUNK),
        commits_per_sec_theirs: theirs_rate / f64::from(BULK_TX_CHUNK),
        rows_per_sec_ours: ours_rate,
        rows_per_sec_theirs: theirs_rate,
        ghz: Some(ghz.into()),
    });
    // The write-order pin: bulk is the last row, always.
    debug_assert!(
        rows.iter()
            .position(|row| row.name == "bulk_append")
            .is_none_or(|index| index == rows.len() - 1),
        "bulk_append must be the last write row"
    );
    Ok(LaneReport {
        lane: lane.label(),
        sqlite_sync: lane.sqlite_sync_label(),
        rows,
    })
}

/// The writes lane entry point: device honesty first, then one
/// [`run_lane`] per requested lane in args order (the default runs
/// `Nosync` first and `Durable` last — the fsync-shadow law lifted to
/// the lane axis: the durable lane's seconds of fsync leave the deepest
/// clock shadow, so they land after every nosync sample), then the two
/// artifacts.
///
/// # Errors
///
/// The device-honesty refusal; setup failures; the post-state gate.
pub fn run(args: &crate::cli::WritesArgs) -> Result<i32, String> {
    // Device honesty FIRST, before creating anything: the timed write
    // lanes are fsync-bound, so a RAM-backed volume would report a
    // number physics never signed (the `driver/write_families.rs`
    // precedent).
    crate::devhonesty::assert_disk_backed(&args.dir, "the timed write lanes")
        .map_err(|refusal| refusal.to_string())?;

    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-writes",
            crate::report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;

    // Write-appropriate protocol, COLD-family-sized: every sample pays
    // a real commit (fsync-bound on the durable lane), so few warmups
    // are needed and 32 samples keep the tail meaningful without hours
    // of fsync.
    let proto = Protocol {
        warmups: 2,
        samples: args.samples.unwrap_or(32),
    };
    let cfg = GenConfig {
        seed: args.seed,
        scale: args.scale,
    };

    let mut lanes = Vec::new();
    for lane in &args.lanes {
        let scratch = out_dir.join("scratch").join(lane.label());
        lanes.push(run_lane(*lane, cfg, proto, &args.batches, &scratch)?);
    }

    let report = WritesReport {
        provenance: crate::report::provenance(Path::new(".")),
        scale: args.scale.label(),
        seed: args.seed,
        samples: proto.samples,
        lanes,
    };
    std::fs::write(out_dir.join("writes-report.json"), to_json(&report))
        .map_err(|e| format!("artifact: {e}"))?;
    let markdown = to_markdown(&report);
    std::fs::write(out_dir.join("writes-report.md"), &markdown)
        .map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    // The scratch stores served their purpose; the artifacts are the
    // run.
    let _ = std::fs::remove_dir_all(out_dir.join("scratch"));
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus_gen::{self, Scale};
    use crate::json::Value;

    fn provenance() -> Provenance {
        Provenance {
            crate_version: "0.0.0-test".to_owned(),
            git_rev: "deadbeef".to_owned(),
            timestamp: "2026-07-19T00:00:00Z".to_owned(),
            host: "test-host".to_owned(),
            shared: None,
        }
    }

    fn stats(base: u64) -> Stats {
        Stats {
            min: base,
            p50: base + 1,
            p90: base + 2,
            p95: base + 3,
            p99: base + 4,
            max: base + 5,
            mean_ns: base + 2,
        }
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bumbledb-writes-lane-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn the_durability_axis_has_exactly_two_points() {
        assert_eq!(DurabilityLane::Durable.label(), "durable");
        assert_eq!(DurabilityLane::Nosync.label(), "nosync");
        assert_eq!(
            DurabilityLane::Durable.sqlite_sync_label(),
            "wal+synchronous=FULL+fullfsync=ON"
        );
        assert_eq!(
            DurabilityLane::Nosync.sqlite_sync_label(),
            "wal+synchronous=OFF"
        );
        assert_eq!(
            DurabilityLane::Durable.store_mode(),
            crate::storemode::StoreMode::Durable
        );
        assert_eq!(
            DurabilityLane::Nosync.store_mode(),
            crate::storemode::StoreMode::Ephemeral
        );
    }

    #[test]
    fn report_json_shape_is_pinned() {
        let report = WritesReport {
            provenance: provenance(),
            scale: "S",
            seed: 9,
            samples: 8,
            lanes: vec![LaneReport {
                lane: DurabilityLane::Nosync.label(),
                sqlite_sync: DurabilityLane::Nosync.sqlite_sync_label(),
                rows: vec![
                    WriteRow {
                        name: "append".to_owned(),
                        batch: 10,
                        ours: stats(100),
                        theirs: stats(200),
                        commits_per_sec_ours: 1234.25,
                        commits_per_sec_theirs: 617.5,
                        rows_per_sec_ours: 12342.5,
                        rows_per_sec_theirs: 6175.0,
                        ghz: Some(GhzReport {
                            pre: 3.5,
                            post: 3.25,
                            retried: false,
                            contaminated: false,
                        }),
                    },
                    WriteRow {
                        name: "delete".to_owned(),
                        batch: 1,
                        ours: stats(300),
                        theirs: stats(400),
                        commits_per_sec_ours: 100.5,
                        commits_per_sec_theirs: 50.25,
                        rows_per_sec_ours: 100.5,
                        rows_per_sec_theirs: 50.25,
                        ghz: None,
                    },
                ],
            }],
        };
        let parsed = crate::json::parse(&to_json(&report)).expect("valid JSON");
        assert_eq!(
            parsed
                .get("provenance")
                .and_then(|p| p.get("host"))
                .and_then(Value::as_str),
            Some("test-host")
        );
        assert!(
            parsed
                .get("provenance")
                .and_then(|p| p.get("shared_machine"))
                .is_none(),
            "boost-off keeps the pre-boost provenance shape"
        );
        assert_eq!(parsed.get("scale").and_then(Value::as_str), Some("S"));
        assert_eq!(parsed.get("seed").and_then(Value::as_f64), Some(9.0));
        assert_eq!(parsed.get("samples").and_then(Value::as_f64), Some(8.0));
        let lanes = parsed.get("lanes").and_then(Value::as_arr).expect("lanes");
        assert_eq!(lanes.len(), 1);
        assert_eq!(lanes[0].get("lane").and_then(Value::as_str), Some("nosync"));
        assert_eq!(
            lanes[0].get("sqlite_sync").and_then(Value::as_str),
            Some("wal+synchronous=OFF")
        );
        let rows = lanes[0].get("rows").and_then(Value::as_arr).expect("rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("name").and_then(Value::as_str), Some("append"));
        assert_eq!(rows[0].get("batch").and_then(Value::as_f64), Some(10.0));
        // Stats objects carry the exact report/json_out.rs shape.
        let ours = rows[0].get("ours").expect("ours");
        assert_eq!(ours.get("min").and_then(Value::as_f64), Some(100.0));
        assert_eq!(ours.get("p99").and_then(Value::as_f64), Some(104.0));
        assert_eq!(ours.get("mean_ns").and_then(Value::as_f64), Some(102.0));
        let theirs = rows[0].get("theirs").expect("theirs");
        assert_eq!(theirs.get("p50").and_then(Value::as_f64), Some(201.0));
        assert_eq!(
            rows[0].get("commits_per_sec_ours").and_then(Value::as_f64),
            Some(1234.25)
        );
        assert_eq!(
            rows[0]
                .get("commits_per_sec_theirs")
                .and_then(Value::as_f64),
            Some(617.5)
        );
        assert_eq!(
            rows[0].get("rows_per_sec_ours").and_then(Value::as_f64),
            Some(12342.5)
        );
        assert_eq!(
            rows[0].get("rows_per_sec_theirs").and_then(Value::as_f64),
            Some(6175.0)
        );
        // Ghz renders like push_ghz: present on row 0, null on row 1.
        let ghz = rows[0].get("ghz").expect("ghz");
        assert_eq!(ghz.get("pre").and_then(Value::as_f64), Some(3.5));
        assert_eq!(ghz.get("post").and_then(Value::as_f64), Some(3.25));
        assert_eq!(ghz.get("retried").and_then(Value::as_bool), Some(false));
        assert_eq!(rows[1].get("ghz"), Some(&Value::Null));
    }

    /// One parsed lane's row names plus the report handle — the shared
    /// spine of the two end-to-end tests.
    fn lane_rows(out: &Path) -> crate::json::Value {
        let raw = std::fs::read_to_string(out.join("writes-report.json")).expect("artifact");
        crate::json::parse(&raw).expect("valid JSON")
    }

    /// The whole tiny ladder on the nosync lane: both engines run every
    /// family, the post-state gate passes (the run returns 0 only past
    /// it), and the artifact carries every row with positive stats and
    /// rates under the lane's durability labels.
    #[test]
    fn tiny_ladder_runs_and_verifies_post_state() {
        let dir = scratch("tiny-ladder");
        let out = dir.join("out");
        let code = run(&crate::cli::WritesArgs {
            scale: Scale::Tiny,
            seed: 1,
            dir: dir.clone(),
            lanes: vec![DurabilityLane::Nosync],
            batches: vec![1, 10],
            samples: Some(4),
            out: Some(out.clone()),
        })
        .expect("the tiny ladder runs");
        assert_eq!(code, 0);
        let parsed = lane_rows(&out);
        let lanes = parsed.get("lanes").and_then(Value::as_arr).expect("lanes");
        assert_eq!(lanes.len(), 1);
        assert_eq!(lanes[0].get("lane").and_then(Value::as_str), Some("nosync"));
        assert_eq!(
            lanes[0].get("sqlite_sync").and_then(Value::as_str),
            Some("wal+synchronous=OFF")
        );
        let rows = lanes[0].get("rows").and_then(Value::as_arr).expect("rows");
        let names: Vec<&str> = rows
            .iter()
            .filter_map(|row| row.get("name").and_then(Value::as_str))
            .collect();
        assert_eq!(
            names,
            vec![
                "commit_b1",
                "commit_b10",
                "delete_b1",
                "delete_b10",
                "bulk_append"
            ],
            "the ladder rows, bulk last"
        );
        for row in rows {
            for side in ["ours", "theirs"] {
                let min = row
                    .get(side)
                    .and_then(|stats| stats.get("min"))
                    .and_then(Value::as_f64)
                    .expect("min");
                assert!(min > 0.0, "{side} stats must be positive");
            }
            for key in [
                "commits_per_sec_ours",
                "commits_per_sec_theirs",
                "rows_per_sec_ours",
                "rows_per_sec_theirs",
            ] {
                let rate = row.get(key).and_then(Value::as_f64).expect("rate");
                assert!(rate > 0.0, "{key} must be positive");
            }
        }
        // Scratch is removed on success; the artifacts remain.
        assert!(!out.join("scratch").exists());
        assert!(out.join("writes-report.md").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The durable lane runs the identical contract — same families,
    /// same gate — under its own labels.
    #[test]
    fn durable_lane_runs_the_same_contract() {
        let dir = scratch("durable-lane");
        let out = dir.join("out");
        let code = run(&crate::cli::WritesArgs {
            scale: Scale::Tiny,
            seed: 1,
            dir: dir.clone(),
            lanes: vec![DurabilityLane::Durable],
            batches: vec![1],
            samples: Some(4),
            out: Some(out.clone()),
        })
        .expect("the durable lane runs");
        assert_eq!(code, 0);
        let parsed = lane_rows(&out);
        let lanes = parsed.get("lanes").and_then(Value::as_arr).expect("lanes");
        assert_eq!(lanes.len(), 1);
        assert_eq!(
            lanes[0].get("lane").and_then(Value::as_str),
            Some("durable")
        );
        assert_eq!(
            lanes[0].get("sqlite_sync").and_then(Value::as_str),
            Some("wal+synchronous=FULL+fullfsync=ON")
        );
        let rows = lanes[0].get("rows").and_then(Value::as_arr).expect("rows");
        let names: Vec<&str> = rows
            .iter()
            .filter_map(|row| row.get("name").and_then(Value::as_str))
            .collect();
        assert_eq!(names, vec!["commit_b1", "delete_b1", "bulk_append"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The delete-bearing contract, falsified from both sides (the
    /// `posting_swap` test shape): a recorded row deletes once, and the
    /// same body again REFUSES — and the refusal commits NOTHING (the
    /// generation stands still).
    #[test]
    fn delete_refuses_a_missing_row() {
        let dir = scratch("delete-refusal");
        let cfg = GenConfig {
            seed: 1,
            scale: Scale::Tiny,
        };
        let db = Db::create(&dir.join("db"), Ledger).expect("create");
        for rel in writebench::non_posting_relations() {
            db.bulk_load_dyn(rel, corpus_gen::relation_rows(cfg, rel))
                .expect("seed");
        }
        let sizes = Sizes::of(cfg.scale);
        let mut rng = Rng::new(cfg.seed ^ DELETE_SEED ^ 1);
        let posting = db
            .write(|tx| {
                let id: PostingId = tx.alloc()?;
                let posting = writebench::prepared_posting(&mut rng, &sizes, id);
                tx.insert(&posting)?;
                Ok(posting)
            })
            .expect("seed posting");
        let mut recorded = VecDeque::from([posting.clone(), posting]);
        assert_eq!(
            delete_recorded(&db, &mut recorded, 1).expect("live delete"),
            1
        );
        let generation = db.generation().expect("generation");
        let refusal = delete_recorded(&db, &mut recorded, 1);
        let err = refusal.expect_err("a no-op delete must abort the transaction");
        assert!(err.contains("delete-bearing"), "{err}");
        assert_eq!(
            db.generation().expect("generation"),
            generation,
            "a refused delete leaves the store untouched"
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The post-state gate catches a one-row divergence and names the
    /// count.
    #[test]
    fn post_state_catches_a_divergence() {
        let dir = scratch("post-state");
        let cfg = GenConfig {
            seed: 1,
            scale: Scale::Tiny,
        };
        let db = Db::create(&dir.join("db"), Ledger).expect("create");
        corpus::load_bumbledb(&db, cfg).expect("load");
        let (conn, _) = corpus::load_sqlite(&dir.join("oracle.sqlite"), cfg).expect("oracle");
        let sizes = Sizes::of(cfg.scale);
        verify_post_state(&db, &conn, sizes.postings, sizes.postings)
            .expect("the twins agree before the divergence");
        // One extra row into sqlite only — the gate must refuse, naming
        // the divergent count.
        conn.execute(
            POSTING_INSERT,
            rusqlite::params![
                i64::try_from(sizes.postings).expect("axiom"),
                0i64,
                0i64,
                0i64,
                1i64,
                corpus_gen::AT_BASE
            ],
        )
        .expect("extra row");
        let err = verify_post_state(&db, &conn, sizes.postings, sizes.postings)
            .expect_err("the gate must catch the extra row");
        assert!(err.contains("counts diverge"), "{err}");
        assert!(
            err.contains(&(sizes.postings + 1).to_string()),
            "the divergent count is named: {err}"
        );
        drop((db, conn));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
