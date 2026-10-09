//! The write ladder: commit and delete throughput per batch size, then the
//! insertion stream, on a durability-paired twin (see
//! [`crate::harness::sqlite_run::DURABILITY`]). The post-state is verified before the
//! stream runs.
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use bumbledb::{Db, Value};
use rusqlite::Connection;

use crate::harness::report::Provenance;
use crate::harness::sqlite_run::POSTING_INSERT;
use crate::harness::{self, Measurement, Protocol, Stats};
use crate::json;
use crate::worlds::corpus_gen::{GenConfig, Rng, Sizes};
use crate::worlds::ledger::{Ledger, Posting, PostingId, ids};
use crate::worlds::{corpus, writebench};

#[derive(Debug, Clone, PartialEq)]
pub struct WritesReport {
    pub provenance: Provenance,
    pub scale: &'static str,
    pub seed: u64,
    pub samples: u32,
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
    out.push('}');
}

#[must_use]
pub fn to_json(report: &WritesReport) -> String {
    let mut out = String::new();
    out.push_str("{\"provenance\":");
    super::push_provenance(&mut out, &report.provenance);
    let _ = write!(
        out,
        ",\"scale\":\"{}\",\"seed\":{},\"samples\":{},\"sqlite_sync\":\"{}\",\"rows\":[",
        report.scale,
        report.seed,
        report.samples,
        crate::harness::sqlite_run::SQLITE_SYNC
    );
    for (index, row) in report.rows.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_row(&mut out, row);
    }
    out.push_str("]}");
    out
}

fn to_markdown(report: &WritesReport) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# writes lane — scale {}, seed {}, samples {}",
        report.scale, report.seed, report.samples
    );
    let _ = writeln!(
        out,
        "\nsqlite `{}`\n",
        crate::harness::sqlite_run::SQLITE_SYNC
    );
    out.push_str(
        "| family | batch | ours p50 ns | sqlite p50 ns | ours commits/s | sqlite commits/s | ours rows/s | sqlite rows/s |\n",
    );
    out.push_str("|---|---:|---:|---:|---:|---:|---:|---:|\n");
    for row in &report.rows {
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
    out
}

const COMMIT_SEED: u64 = 0x0117_0000;

const DELETE_SEED: u64 = 0x0117_0100;

const POSTING_DELETE: &str = "DELETE FROM \"Posting\" WHERE \"id\" = ?1";

fn commits_per_sec(stats: &Stats) -> f64 {
    1e9 / (stats.mean_ns.max(1) as f64)
}

fn ladder_row(name: String, batch: u32, ours: Stats, theirs: Stats) -> WriteRow {
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
    // Application-owned ids: MAX(id) + 1 probed before the timed window —
    // the exact mint the SQLite twin uses (`next_posting_id`), so the two
    // engines pay symmetric per-row work inside the samples.
    let mut mint = writebench::PostingMint::probe(db)?;
    harness::measure(proto, || {
        db.write(crate::harness::bench_work(), |tx| {
            for _ in 0..batch {
                let id = mint.next();
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
    let mut mint = writebench::PostingMint::probe(db)?;
    let mut recorded: VecDeque<Posting> = VecDeque::new();
    let mut remaining = total;
    while remaining > 0 {
        let chunk = remaining.min(1024);
        let committed = db
            .write(crate::harness::bench_work(), |tx| {
                let mut out = Vec::with_capacity(usize::try_from(chunk).expect("small chunk"));
                for _ in 0..chunk {
                    let id = mint.next();
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

/// Delete-bearing BY CONTRACT (the [`crate::worlds::writebench::posting_swap`]
/// precedent): a no-op delete returns `Err` INSIDE the closure — the in-closure
/// sentinel abort drops the delta whole, so a refused delete never commits the
/// batch's earlier deletes, and the lane can never silently degrade into an
/// insert-only (or partial) measurement.
fn delete_recorded(
    db: &Db<Ledger>,
    recorded: &mut VecDeque<Posting>,
    batch: u32,
) -> Result<u64, String> {
    db.write(crate::harness::bench_work(), |tx| {
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

fn verify_insert_stream_pair(scratch: &Path, expected_postings: u64) -> Result<(), String> {
    let dir = scratch.join("insert-stream-0.bdb");
    let db =
        crate::harness::open_db(&dir, Ledger).map_err(|e| format!("insert_stream re-open: {e}"))?;
    let ours = db
        .read(crate::harness::bench_work(), |snap| {
            Ok(snap.scan(ids::POSTING)?.count())
        })
        .map_err(|e| format!("insert_stream re-scan: {e:?}"))? as u64;
    let conn = Connection::open(scratch.join("insert-stream-oracle-0.sqlite"))
        .map_err(|e| format!("insert_stream oracle re-open: {e}"))?;
    let theirs: i64 = conn
        .query_row("SELECT COUNT(*) FROM \"Posting\"", [], |row| row.get(0))
        .map_err(|e| format!("insert_stream oracle count: {e}"))?;
    let theirs = u64::try_from(theirs).map_err(|e| format!("insert_stream oracle count: {e}"))?;
    if ours != expected_postings || theirs != expected_postings {
        return Err(format!(
            "insert_stream pair 0 diverges: engine {ours} vs sqlite {theirs} vs expected \
             {expected_postings} postings"
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

fn verify_post_state(
    db: &Db<Ledger>,
    conn: &Connection,
    corpus_ceiling: u64,
    expected_postings: u64,
) -> Result<(), String> {
    let engine_rows: Vec<bumbledb::canonical::DecodedRow> = db
        .read(crate::harness::bench_work(), |snap| {
            snap.scan(ids::POSTING)?.collect()
        })
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

/// Seed the twin pair, run the commit ladder, run the delete ladder, verify
/// the post-state, then `insert_stream`, always last: nothing measures after
/// its seconds of fsync.
fn run_ladder(
    cfg: GenConfig,
    proto: Protocol,
    batches: &[u32],
    scratch: &Path,
) -> Result<Vec<WriteRow>, String> {
    std::fs::create_dir_all(scratch).map_err(|e| format!("scratch: {e}"))?;
    let sizes = Sizes::of(cfg.scale);

    eprintln!("bench: writes — loading the scratch corpus");
    let db = crate::harness::create_db(&scratch.join("db.bdb"), Ledger)?;
    corpus::load_bumbledb(&db, cfg).map_err(|e| format!("load: {e:?}"))?;

    let (conn, _) = corpus::load_sqlite(&scratch.join("oracle.sqlite"), cfg)
        .map_err(|e| format!("oracle load: {e}"))?;
    crate::harness::sqlite_run::configure_durable(&conn)?;
    crate::harness::sqlite_run::assert_durable_parity(&conn)?;

    let calls = u64::from(proto.warmups + proto.samples);
    let mut inserted = 0u64;
    let mut deleted = 0u64;
    let mut rows = Vec::new();

    for &batch in batches {
        let name = format!("commit_b{batch}");
        eprintln!("bench: writes — {name}");
        let mut rng_ours = Rng::new(cfg.seed ^ COMMIT_SEED ^ u64::from(batch));
        let mut rng_theirs = Rng::new(cfg.seed ^ COMMIT_SEED ^ u64::from(batch));
        let ours = commit_engine(&db, cfg, proto, batch, &mut rng_ours)?;
        let theirs = commit_sqlite(&conn, cfg, proto, batch, &mut rng_theirs)?;
        inserted += calls * u64::from(batch);
        rows.push(ladder_row(name, batch, ours.stats, theirs.stats));
    }

    for &batch in batches {
        let name = format!("delete_b{batch}");
        eprintln!("bench: writes — {name}");
        let total = calls * u64::from(batch);
        let (mut recorded, mut mirrored) = seed_delete_rows(&db, &conn, cfg, total, batch)?;
        inserted += total;
        let ours = harness::measure(proto, || delete_recorded(&db, &mut recorded, batch))?;
        let theirs = delete_sqlite(&conn, &mut mirrored, proto, batch)?;
        if !recorded.is_empty() || !mirrored.is_empty() {
            return Err(format!(
                "delete_b{batch}: {} engine / {} sqlite rows survived the ladder \
                 (the deques must drain exactly)",
                recorded.len(),
                mirrored.len()
            ));
        }
        deleted += total;
        rows.push(ladder_row(name, batch, ours.stats, theirs.stats));
    }

    let expected = sizes.postings + inserted - deleted;
    verify_post_state(&db, &conn, sizes.postings, expected)
        .map_err(|e| format!("post-state: {e}"))?;

    eprintln!("bench: writes — insert_stream");
    let stream_scratch = scratch.join("insert-stream");
    std::fs::create_dir_all(&stream_scratch).map_err(|e| format!("insert_stream scratch: {e}"))?;
    let ours = writebench::insert_stream_bumbledb(cfg, &stream_scratch)?;
    let theirs = crate::harness::sqlite_run::insert_stream(cfg, &stream_scratch)?;
    verify_insert_stream_pair(&stream_scratch, sizes.postings)?;
    let facts = sizes.postings + sizes.posting_tags;
    let batch = u32::try_from(facts).expect("stream fits u32");
    rows.push(ladder_row(
        "insert_stream".to_owned(),
        batch,
        ours.stats,
        theirs.stats,
    ));

    debug_assert!(
        rows.iter()
            .position(|row| row.name == "insert_stream")
            .is_none_or(|index| index == rows.len() - 1),
        "insert_stream must be the last write row"
    );
    Ok(rows)
}

pub fn run(args: &crate::cli::WritesArgs) -> Result<i32, String> {
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-writes",
            crate::harness::report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;

    // Each sample includes a durable commit; use the smaller cold-family protocol.
    let proto = Protocol {
        warmups: 2,
        samples: args.samples.unwrap_or(32),
    };
    let cfg = GenConfig {
        seed: args.seed,
        scale: args.scale,
    };

    let rows = run_ladder(cfg, proto, &args.batches, &out_dir.join("scratch"))?;

    let report = WritesReport {
        provenance: crate::harness::report::provenance(Path::new(".")),
        scale: args.scale.label(),
        seed: args.seed,
        samples: proto.samples,
        rows,
    };
    std::fs::write(out_dir.join("writes-report.json"), to_json(&report))
        .map_err(|e| format!("artifact: {e}"))?;
    let markdown = to_markdown(&report);
    std::fs::write(out_dir.join("writes-report.md"), &markdown)
        .map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());

    let _ = std::fs::remove_dir_all(out_dir.join("scratch"));
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::Value;
    use crate::worlds::corpus_gen::{self, Scale};

    fn provenance() -> Provenance {
        Provenance {
            crate_version: "0.0.0-test".to_owned(),
            git_rev: "deadbeef".to_owned(),
            toolchain: "rustc test",
            timestamp: "2026-07-19T00:00:00Z".to_owned(),
            host: "test-host".to_owned(),
            shared: None,
            parallel_jobs: None,
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
        let dir = crate::fixture::scratch_path(format!("bumbledb-writes-lane-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn report_json_shape_is_pinned() {
        let report = WritesReport {
            provenance: provenance(),
            scale: "S",
            seed: 9,
            samples: 8,
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
                },
            ],
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
        assert_eq!(
            parsed.get("sqlite_sync").and_then(Value::as_str),
            Some("wal+synchronous=FULL+fullfsync=ON")
        );
        let rows = parsed.get("rows").and_then(Value::as_arr).expect("rows");
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
    }

    fn report_json(out: &Path) -> crate::json::Value {
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
            batches: vec![1, 10],
            samples: Some(4),
            out: Some(out.clone()),
        })
        .expect("the tiny ladder runs");
        assert_eq!(code, 0);
        let parsed = report_json(&out);
        let rows = parsed.get("rows").and_then(Value::as_arr).expect("rows");
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

    /// Deleting an absent posting refuses and commits nothing.
    #[test]
    fn delete_refuses_a_missing_row() {
        let dir = scratch("delete-refusal");
        let cfg = GenConfig {
            seed: 1,
            scale: Scale::Tiny,
        };
        let db = Db::create(&dir.join("db.bdb"), Ledger, crate::harness::bench_work())
            .expect("create")
            .expect("accepted");
        for rel in writebench::non_posting_relations() {
            db.write(crate::harness::bench_work(), |tx| {
                tx.insert_dyn(rel, corpus_gen::relation_rows(cfg, rel))
                    .map(bumbledb::MutationReport::changed)
            })
            .expect("seed")
            .unwrap();
        }
        let sizes = Sizes::of(cfg.scale);
        let mut rng = Rng::new(cfg.seed ^ DELETE_SEED ^ 1);
        let mut mint = writebench::PostingMint::probe(&db).expect("probe");
        let posting = db
            .write(crate::harness::bench_work(), |tx| {
                let posting = writebench::prepared_posting(&mut rng, &sizes, mint.next());
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
        let generation = db
            .generation(crate::harness::bench_work())
            .expect("generation");
        let refusal = delete_recorded(&db, &mut recorded, 1);
        let err = refusal.expect_err("a no-op delete must abort the transaction");
        assert!(
            err.contains("Io("),
            "a refused delete is the Io sentinel (the message is not on the wire): {err}"
        );
        assert_eq!(
            db.generation(crate::harness::bench_work())
                .expect("generation"),
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
        let db = Db::create(&dir.join("db.bdb"), Ledger, crate::harness::bench_work())
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
}
