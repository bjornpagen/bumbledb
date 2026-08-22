//! The durability axis is [`crate::duralane::DurabilityLane`] — the one
//! `ANALYZE` after load, `wal_checkpoint(TRUNCATE)` after load — then the
//! pragmas back: a misconfigured twin fails before flattering

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
use crate::{clockproxy, corpus, sqlmap, trace_out, writebench};

#[derive(Debug, Clone, PartialEq)]
pub struct WritesReport {
    pub provenance: Provenance,
    pub scale: &'static str,
    pub seed: u64,
    pub samples: u32,
    pub lanes: Vec<LaneReport>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaneReport {
    pub lane: &'static str,
    pub sqlite_sync: &'static str,
    pub rows: Vec<WriteRow>,
}

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

fn to_markdown(report: &WritesReport, flames: &[(&'static str, String, String)]) -> String {
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
    if !flames.is_empty() {
        let _ = writeln!(out, "\n## Flame summaries (per cell, --trace)\n");
        for (lane, family, table) in flames {
            let _ = writeln!(out, "### {lane} / {family}\n");
            let _ = writeln!(out, "```text\n{table}```\n");
        }
    }
    out
}

const COMMIT_SEED: u64 = 0x0117_0000;

const DELETE_SEED: u64 = 0x0117_0100;

const POSTING_DELETE: &str = "DELETE FROM \"Posting\" WHERE \"id\" = ?1";

#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn commits_per_sec(stats: &Stats) -> f64 {
    1e9 / (stats.mean_ns.max(1) as f64)
}

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

fn next_posting_id(conn: &Connection) -> Result<u64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(\"id\"), -1) + 1 FROM \"Posting\"",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|next| u64::try_from(next).expect("dense ids"))
    .map_err(|e| format!("next id: {e}"))
}

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

fn commit_engine(
    db: &Db<Ledger>,
    cfg: GenConfig,
    proto: Protocol,
    batch: u32,
    rng: &mut Rng,
) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    harness::measure(proto, || {
        db.write(|tx| {
            for _ in 0..batch {
                let id: PostingId = tx.reserve(1)?.start().expect("nonempty");
                tx.insert([&writebench::prepared_posting(rng, &sizes, id)])?;
            }
            Ok(())
        })
        .map(|admission| {
            admission.unwrap();
            u64::from(batch)
        })
        .map_err(|e| format!("commit_b{batch}: {e:?}"))
    })
}

fn commit_sqlite(
    conn: &Connection,
    cfg: GenConfig,
    proto: Protocol,
    batch: u32,
    rng: &mut Rng,
) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut next = next_posting_id(conn)?;
    harness::measure(proto, || {
        let mut run = || -> rusqlite::Result<()> {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            {
                let mut stmt = conn.prepare_cached(POSTING_INSERT)?;
                for _ in 0..batch {
                    let body = writebench::prepared_posting(rng, &sizes, PostingId(next));
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
                    let id: PostingId = tx.reserve(1)?.start().expect("nonempty");
                    let posting = writebench::prepared_posting(&mut rng, &sizes, id);
                    tx.insert([&posting])?;
                    out.push(posting);
                }
                Ok(out)
            })
            .map_err(|e| format!("delete_b{batch} pre-phase: {e:?}"))?
            .unwrap()
            .value;
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
                    ..*posting
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

/// Delete-bearing BY CONTRACT (the [`crate::writebench::posting_swap`]
/// precedent): a no-op delete returns `Err` INSIDE the closure — the in-closure
/// sentinel abort drops the delta whole, so a refused delete never commits the
/// batch's earlier deletes, and the lane can never silently degrade into an
/// insert-only (or partial) measurement.
/// # Panics
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
            if tx.delete([&victim])?.changed() == 0 {
                return Err(bumbledb::Error::from(std::io::Error::other(
                    "the delete lane must be delete-bearing: a recorded posting was absent",
                )));
            }
        }
        Ok(())
    })
    .map(|admission| {
        admission.unwrap();
        u64::from(batch)
    })
    .map_err(|e| format!("delete_b{batch}: {e:?}"))
}

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

/// `insert_stream` on `SQLite`, lane-local (`sqlite_run::insert_stream`
/// hardwires [`corpus::configure_sqlite`] = FULL, so this variant applies the
/// lane's pragmas after the standing config on every throwaway file):
/// pre-seeded throwaway files (the corpus minus postings, built before any
/// timing), the full posting stream timed as a host loop of 4096-row
/// transactions per sample.
fn insert_stream_sqlite(
    cfg: GenConfig,
    scratch: &Path,
    lane: DurabilityLane,
) -> Result<Measurement, String> {
    use std::cell::RefCell;
    let proto = writebench::write_protocol("insert_stream");
    let mut pending = VecDeque::new();
    for sample in 0..proto.warmups + proto.samples {
        let path = scratch.join(format!("insert-stream-oracle-{sample}.sqlite"));
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
            .map_err(|e| format!("insert_stream sqlite: {e}"))?;
        facts += corpus::load_sqlite_relation(&conn, cfg, ids::POSTING_TAG)
            .map_err(|e| format!("insert_stream sqlite tags: {e}"))?;
        done.borrow_mut().push(conn);
        Ok(facts)
    })
}

fn verify_insert_stream_pair(
    scratch: &Path,
    lane: DurabilityLane,
    expected_postings: u64,
) -> Result<(), String> {
    let dir = scratch.join("insert-stream-bumbledb-0");
    let db = lane
        .store_mode()
        .open(&dir, Ledger)
        .map_err(|e| format!("insert_stream re-open ({}): {e}", lane.label()))?;
    let ours = db
        .read(|snap| Ok(snap.scan(ids::POSTING)?.count()))
        .map_err(|e| format!("insert_stream re-scan: {e:?}"))? as u64;
    let conn = Connection::open(scratch.join("insert-stream-oracle-0.sqlite"))
        .map_err(|e| format!("insert_stream oracle re-open: {e}"))?;
    let theirs: i64 = conn
        .query_row("SELECT COUNT(*) FROM \"Posting\"", [], |row| row.get(0))
        .map_err(|e| format!("insert_stream oracle count: {e}"))?;
    let theirs = u64::try_from(theirs).map_err(|e| format!("insert_stream oracle count: {e}"))?;
    if ours != expected_postings || theirs != expected_postings {
        return Err(format!(
            "insert_stream pair 0 diverges ({}): engine {ours} vs sqlite {theirs} vs expected \
             {expected_postings} postings",
            lane.label()
        ));
    }
    Ok(())
}

fn cell_u64(row: &[Value], index: usize) -> Result<u64, String> {
    match row.get(index) {
        Some(Value::U64(v)) => Ok(*v),
        other => Err(format!(
            "posting cell {index}: expected u64, found {other:?}"
        )),
    }
}

fn cell_i64(row: &[Value], index: usize) -> Result<i64, String> {
    match row.get(index) {
        Some(Value::I64(v)) => Ok(*v),
        other => Err(format!(
            "posting cell {index}: expected i64, found {other:?}"
        )),
    }
}

type Body = (u64, u64, u64, i64, i64);

/// # Errors
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

/// One durability lane, whole: seed the twin pair, run the commit ladder, run
/// the delete ladder, verify the post-state, then `insert_stream` — LAST,
/// always (seconds of fsync leave the deepest clock shadow; nothing measures
/// after it — the `write_families` order pin, carried here by the same
/// `debug_assert!`).
#[expect(
    clippy::too_many_lines,
    reason = "one durability lane, whole — the measured order is the content"
)]
fn run_lane(
    lane: DurabilityLane,
    cfg: GenConfig,
    proto: Protocol,
    batches: &[u32],
    scratch: &Path,
    trace_dir: Option<&Path>,
) -> Result<(LaneReport, Vec<(String, String)>), String> {
    std::fs::create_dir_all(scratch).map_err(|e| format!("scratch: {e}"))?;
    let sizes = Sizes::of(cfg.scale);

    eprintln!(
        "bench: writes {} — loading the scratch corpus",
        lane.label()
    );
    let db = lane.store_mode().create(&scratch.join("db"), Ledger)?;
    corpus::load_bumbledb(&db, cfg).map_err(|e| format!("load ({}): {e:?}", lane.label()))?;

    let (conn, _) = corpus::load_sqlite(&scratch.join("oracle.sqlite"), cfg)
        .map_err(|e| format!("oracle load ({}): {e}", lane.label()))?;
    lane.configure(&conn)?;
    lane.assert_parity(&conn)?;

    let calls = u64::from(proto.warmups + proto.samples) + u64::from(trace_dir.is_some());
    let mut inserted = 0u64;
    let mut deleted = 0u64;
    let mut rows = Vec::new();
    let mut flames: Vec<(String, String)> = Vec::new();

    for &batch in batches {
        let name = format!("commit_b{batch}");
        eprintln!("bench: writes {} — {name}", lane.label());
        let mut rng_ours = Rng::new(cfg.seed ^ COMMIT_SEED ^ u64::from(batch));
        let mut rng_theirs = Rng::new(cfg.seed ^ COMMIT_SEED ^ u64::from(batch));
        let ((ours, theirs), ghz) = clockproxy::stamped(|| {
            Ok((
                commit_engine(&db, cfg, proto, batch, &mut rng_ours)?,
                commit_sqlite(&conn, cfg, proto, batch, &mut rng_theirs)?,
            ))
        })?;
        if let Some(table) = trace_out::traced_twin(
            trace_dir,
            &name,
            &mut |p| commit_engine(&db, cfg, p, batch, &mut rng_ours),
            &mut |p| commit_sqlite(&conn, cfg, p, batch, &mut rng_theirs),
        )? {
            flames.push((name.clone(), table));
        }
        inserted += calls * u64::from(batch);
        rows.push(ladder_row(
            name,
            batch,
            ours.stats,
            theirs.stats,
            Some(ghz.into()),
        ));
    }

    for &batch in batches {
        let name = format!("delete_b{batch}");
        eprintln!("bench: writes {} — {name}", lane.label());
        let total = calls * u64::from(batch);
        let (mut recorded, mut mirrored) = seed_delete_rows(&db, &conn, cfg, total, batch)?;
        inserted += total;
        let ((ours, theirs), ghz) = clockproxy::stamped(|| {
            Ok((
                harness::measure(proto, || delete_recorded(&db, &mut recorded, batch))?,
                delete_sqlite(&conn, &mut mirrored, proto, batch)?,
            ))
        })?;
        if let Some(table) = trace_out::traced_twin(
            trace_dir,
            &name,
            &mut |p| harness::measure(p, || delete_recorded(&db, &mut recorded, batch)),
            &mut |p| delete_sqlite(&conn, &mut mirrored, p, batch),
        )? {
            flames.push((name.clone(), table));
        }
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

    // (e) Post-state verification — BEFORE insert_stream: the stream

    // in the symmetry re-check below). The gate must pass before the

    let expected = sizes.postings + inserted - deleted;
    verify_post_state(&db, &conn, sizes.postings, expected)
        .map_err(|e| format!("post-state ({}): {e}", lane.label()))?;

    eprintln!("bench: writes {} — insert_stream", lane.label());
    let stream_scratch = scratch.join("insert-stream");
    std::fs::create_dir_all(&stream_scratch).map_err(|e| format!("insert_stream scratch: {e}"))?;
    let ((ours, theirs), ghz) = clockproxy::stamped(|| {
        Ok((
            writebench::insert_stream_bumbledb(cfg, &stream_scratch, lane.store_mode())?,
            insert_stream_sqlite(cfg, &stream_scratch, lane)?,
        ))
    })?;
    verify_insert_stream_pair(&stream_scratch, lane, sizes.postings)?;
    let facts = sizes.postings + sizes.posting_tags;
    let batch = u32::try_from(facts).expect("stream fits u32");
    rows.push(ladder_row(
        "insert_stream".to_owned(),
        batch,
        ours.stats,
        theirs.stats,
        Some(ghz.into()),
    ));

    debug_assert!(
        rows.iter()
            .position(|row| row.name == "insert_stream")
            .is_none_or(|index| index == rows.len() - 1),
        "insert_stream must be the last write row"
    );
    Ok((
        LaneReport {
            lane: lane.label(),
            sqlite_sync: lane.sqlite_sync_label(),
            rows,
        },
        flames,
    ))
}

/// clock shadow, so they land after every nosync sample), then the two
/// # Errors
/// The device-honesty refusal; setup failures; the post-state gate.
pub fn run(args: &crate::cli::WritesArgs) -> Result<i32, String> {
    if args.trace && !cfg!(feature = "obs") {
        return Err(crate::driver::obs_missing("--trace"));
    }
    // Device honesty FIRST, before creating anything: the timed write

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

    let proto = Protocol {
        warmups: 2,
        samples: args.samples.unwrap_or(32),
    };
    let cfg = GenConfig {
        seed: args.seed,
        scale: args.scale,
    };

    let mut lanes = Vec::new();
    let mut flames: Vec<(&'static str, String, String)> = Vec::new();
    for lane in &args.lanes {
        let scratch = out_dir.join("scratch").join(lane.label());
        let trace_dir = args
            .trace
            .then(|| out_dir.join("trace").join("writes").join(lane.label()));
        let (report, lane_flames) = run_lane(
            *lane,
            cfg,
            proto,
            &args.batches,
            &scratch,
            trace_dir.as_deref(),
        )?;
        lanes.push(report);
        flames.extend(
            lane_flames
                .into_iter()
                .map(|(family, table)| (lane.label(), family, table)),
        );
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
    let markdown = to_markdown(&report, &flames);
    std::fs::write(out_dir.join("writes-report.md"), &markdown)
        .map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    if args.trace {
        println!("traces: {}", out_dir.join("trace").join("writes").display());
    }

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
            crate::storemode::StoreMode::Nosync
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

        let ghz = rows[0].get("ghz").expect("ghz");
        assert_eq!(ghz.get("pre").and_then(Value::as_f64), Some(3.5));
        assert_eq!(ghz.get("post").and_then(Value::as_f64), Some(3.25));
        assert_eq!(ghz.get("retried").and_then(Value::as_bool), Some(false));
        assert_eq!(rows[1].get("ghz"), Some(&Value::Null));
    }

    fn lane_rows(out: &Path) -> crate::json::Value {
        let raw = std::fs::read_to_string(out.join("writes-report.json")).expect("artifact");
        crate::json::parse(&raw).expect("valid JSON")
    }

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
            trace: false,
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
                "insert_stream"
            ],
            "the ladder rows, insert_stream last"
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

        assert!(!out.join("scratch").exists());
        assert!(out.join("writes-report.md").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

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
            trace: false,
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
        assert_eq!(names, vec!["commit_b1", "delete_b1", "insert_stream"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// same body again REFUSES — and the refusal commits NOTHING (the

    #[test]
    fn delete_refuses_a_missing_row() {
        let dir = scratch("delete-refusal");
        let cfg = GenConfig {
            seed: 1,
            scale: Scale::Tiny,
        };
        let db = Db::create(&dir.join("db"), Ledger)
            .expect("create")
            .expect("accepted");
        for rel in writebench::non_posting_relations() {
            db.write(|tx| {
                tx.insert_dyn(rel, corpus_gen::relation_rows(cfg, rel))
                    .map(bumbledb::MutationReport::changed)
            })
            .expect("seed")
            .unwrap();
        }
        let sizes = Sizes::of(cfg.scale);
        let mut rng = Rng::new(cfg.seed ^ DELETE_SEED ^ 1);
        let posting = db
            .write(|tx| {
                let id: PostingId = tx.reserve(1)?.start().expect("nonempty");
                let posting = writebench::prepared_posting(&mut rng, &sizes, id);
                tx.insert([&posting])?;
                Ok(posting)
            })
            .expect("seed posting")
            .unwrap()
            .value;
        let mut recorded = VecDeque::from([posting, posting]);
        assert_eq!(
            delete_recorded(&db, &mut recorded, 1).expect("live delete"),
            1
        );
        let generation = db.generation().expect("generation");
        let refusal = delete_recorded(&db, &mut recorded, 1);
        let err = refusal.expect_err("a no-op delete must abort the transaction");
        assert!(
            err.contains("Io("),
            "a refused delete is the Io sentinel (the message is not on the wire): {err}"
        );
        assert_eq!(
            db.generation().expect("generation"),
            generation,
            "a refused delete leaves the store untouched"
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn post_state_catches_a_divergence() {
        let dir = scratch("post-state");
        let cfg = GenConfig {
            seed: 1,
            scale: Scale::Tiny,
        };
        let db = Db::create(&dir.join("db"), Ledger)
            .expect("create")
            .expect("accepted");
        corpus::load_bumbledb(&db, cfg).expect("load");
        let (conn, _) = corpus::load_sqlite(&dir.join("oracle.sqlite"), cfg).expect("oracle");
        let sizes = Sizes::of(cfg.scale);
        verify_post_state(&db, &conn, sizes.postings, sizes.postings)
            .expect("the twins agree before the divergence");

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

    #[cfg(feature = "obs")]
    #[test]
    fn traced_writes_land_the_lmdb_commit_span() {
        let dir = scratch("traced-ladder");
        let out = dir.join("out");
        let code = run(&crate::cli::WritesArgs {
            scale: Scale::Tiny,
            seed: 1,
            dir: dir.clone(),
            lanes: vec![DurabilityLane::Nosync],
            batches: vec![1],
            samples: Some(2),
            trace: true,
            out: Some(out.clone()),
        })
        .expect("the traced tiny ladder runs (post-state gate included)");
        assert_eq!(code, 0);
        let md = std::fs::read_to_string(out.join("writes-report.md")).expect("markdown");
        assert!(md.contains("Flame summaries"), "{md}");
        let lane_dir = out.join("trace").join("writes").join("nosync");
        for cell in ["commit_b1", "delete_b1"] {
            let json_path = lane_dir.join(format!("{cell}.json"));
            let text = std::fs::read_to_string(&json_path)
                .unwrap_or_else(|e| panic!("{}: {e}", json_path.display()));
            assert!(
                text.starts_with("[\n") && text.ends_with("\n]\n"),
                "{} parses as a Chrome array",
                json_path.display()
            );
            let folded = std::fs::read_to_string(lane_dir.join(format!("{cell}.folded")))
                .expect("the folded twin lands beside the json");
            assert!(!folded.is_empty(), "a non-degenerate fold: {cell}");
            for line in folded.lines() {
                let count = line.rsplit(' ').next().expect("a self-ns tail");
                assert!(count.parse::<u64>().is_ok(), "folded self-ns: {line}");
            }
        }
        let commit = std::fs::read_to_string(lane_dir.join("commit_b1.json")).expect("commit");
        assert!(
            commit.contains(bumbledb::obs::names::LMDB_COMMIT.label()),
            "the LMDB commit span reaches the commit cell's artifact"
        );

        assert!(!lane_dir.join("insert_stream.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
