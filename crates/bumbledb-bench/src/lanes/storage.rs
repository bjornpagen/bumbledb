//! The storage lane: on-disk bytes per corpus scale, both engines —
//! REPORT-class ([`crate::lanes`] carries the charter).
//!
//! Byte accounting is one measuring function over one on-disk
//! representation — [`file_bytes`] (`std::fs::metadata` length) applied
//! to the engine's `data.mdb`, the `SQLite` file, and its `-wal`
//! sibling — so "measured after checkpoint/sync" is not a convention
//! but the only expressible reading: every `SQLite` lane checkpoints
//! (TRUNCATE) and drops its connection before any stat, and the wal
//! size is a REPORTED FIELD, so an uncheckpointed emission is visible
//! in the data instead of silently inflating a number.
//!
//! **Not a timed lane**: no wall clock feeds this report, so
//! `devhonesty::assert_disk_backed` is NOT required here — the
//! verify-lane exemption precedent (docs/architecture/60-validation.md:
//! lanes that check answers rather than clocks may run on the ram
//! disk). It is still oracle-disciplined: every load is the one
//! generator stream both stores share, and per-relation row counts are
//! cross-checked across the engine store, the generator sizes, and
//! both `SQLite` lanes before a single byte is recorded — an
//! inequality is `Err`, nothing is reported.
//!
//! The four byte lanes per (scale, world):
//!
//! - **engine raw**: `Db::create` + the world loader, `disk_size()`
//!   (which equals the `data.mdb` stat — a unit test pins the two
//!   reads together so the churn stat path and the live path cannot
//!   drift).
//! - **engine compacted**: `Db::compact` into a sibling directory (the
//!   `ensure_corpus` discipline, `driver/corpus.rs`), reopened and
//!   re-statted.
//! - **sqlite indexed** — the parity config, documented per lane
//!   ([`crate::corpus::configure_sqlite`]): WAL, `synchronous=FULL`,
//!   `fullfsync=ON`, 256 MiB page cache, `temp_store=MEMORY`; full DDL
//!   including the family indexes and the closed vocabularies'
//!   extension INSERTs; prepared-statement inserts in 4096-row
//!   transactions; `ANALYZE`; truncating WAL checkpoint.
//! - **sqlite table-only**: [`crate::sqlmap::table_ddl`] ONLY — the
//!   representational split in `sqlmap`, never a string filter over
//!   the indexed DDL — plus the extension INSERTs (extension rows are
//!   schema surface; without them the corpora differ — their tables
//!   carry only the PRIMARY KEY); the same row streams; **no
//!   ANALYZE** — table-only is the no-secondary-structure lane, and
//!   `sqlite_stat1` would itself be secondary structure.
//!
//! `facts` is the generator row count summed over the writable
//! relations (`0..RELATIONS` per world); the closed vocabularies are
//! virtual/extension surface, excluded from the fact count.
//!
//! ## The churn-checkpoint seam
//!
//! `--churn-dir` names a directory whose immediate subdirectories, in
//! lexicographic name order, are churn checkpoints. Inside each:
//! `db/data.mdb` (optional) and `oracle.sqlite` plus its optional
//! `oracle.sqlite-wal` sibling (optional). One [`ChurnRow`] per
//! checkpoint, `None` for absent artifacts; an empty churn directory
//! is an `Err` naming the contract. A future churn harness composes
//! with this lane through data on disk — checkpoint subdirectories —
//! not through code coupling, and the reported wal bytes are the
//! honesty mechanism: a churn protocol that forgot to checkpoint shows
//! a fat wal, visibly.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use bumbledb::{Db, RelationId};
use rusqlite::Connection;

use crate::calendar::corpus_gen::CalSizes;
use crate::cli::StorageArgs;
use crate::corpus_gen::{GenConfig, Sizes};
use crate::json;
use crate::report::{self, Provenance};
use crate::sqlmap;

/// The whole storage report, plain data. (`PartialEq` only: the
/// provenance's shared-machine stamp carries load-average floats.)
#[derive(Debug, Clone, PartialEq)]
pub struct StorageReport {
    pub provenance: Provenance,
    pub seed: u64,
    pub scales: Vec<ScaleStorage>,
    pub churn: Vec<ChurnRow>,
}

/// One corpus scale's worlds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaleStorage {
    pub scale: &'static str,
    pub worlds: Vec<WorldStorage>,
}

/// One world's byte accounting at one scale, both engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldStorage {
    pub world: &'static str,
    pub facts: u64,
    pub engine_raw_bytes: u64,
    pub engine_compacted_bytes: u64,
    pub sqlite_indexed_bytes: u64,
    pub sqlite_indexed_wal_bytes: u64,
    pub sqlite_tableonly_bytes: u64,
    pub sqlite_tableonly_wal_bytes: u64,
}

/// One churn-ladder step's post-state bytes (`None` = not measured).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChurnRow {
    pub name: String,
    pub engine_bytes: Option<u64>,
    pub sqlite_bytes: Option<u64>,
    pub sqlite_wal_bytes: Option<u64>,
}

fn push_world(out: &mut String, world: &WorldStorage) {
    let _ = write!(
        out,
        "{{\"world\":\"{}\",\"facts\":{},\"engine_raw_bytes\":{},\"engine_compacted_bytes\":{},\"engine_bytes_per_fact\":{:.4},\"sqlite_indexed_bytes\":{},\"sqlite_indexed_wal_bytes\":{},\"sqlite_indexed_bytes_per_fact\":{:.4},\"sqlite_tableonly_bytes\":{},\"sqlite_tableonly_wal_bytes\":{},\"sqlite_tableonly_bytes_per_fact\":{:.4}}}",
        world.world,
        world.facts,
        world.engine_raw_bytes,
        world.engine_compacted_bytes,
        super::per_unit(world.engine_compacted_bytes, world.facts),
        world.sqlite_indexed_bytes,
        world.sqlite_indexed_wal_bytes,
        super::per_unit(world.sqlite_indexed_bytes, world.facts),
        world.sqlite_tableonly_bytes,
        world.sqlite_tableonly_wal_bytes,
        super::per_unit(world.sqlite_tableonly_bytes, world.facts),
    );
}

fn push_opt_u64(out: &mut String, value: Option<u64>) {
    match value {
        Some(v) => {
            let _ = write!(out, "{v}");
        }
        None => out.push_str("null"),
    }
}

fn push_churn(out: &mut String, row: &ChurnRow) {
    out.push_str("{\"name\":");
    json::push_str_lit(out, &row.name);
    out.push_str(",\"engine_bytes\":");
    push_opt_u64(out, row.engine_bytes);
    out.push_str(",\"sqlite_bytes\":");
    push_opt_u64(out, row.sqlite_bytes);
    out.push_str(",\"sqlite_wal_bytes\":");
    push_opt_u64(out, row.sqlite_wal_bytes);
    out.push('}');
}

/// The machine-consumable storage artifact — hand-rolled, like
/// `report/json_out.rs`.
#[must_use]
pub fn to_json(report: &StorageReport) -> String {
    let mut out = String::new();
    out.push_str("{\"provenance\":");
    super::push_provenance(&mut out, &report.provenance);
    let _ = write!(out, ",\"seed\":{},\"scales\":[", report.seed);
    for (index, scale) in report.scales.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(out, "{{\"scale\":\"{}\",\"worlds\":[", scale.scale);
        for (world_index, world) in scale.worlds.iter().enumerate() {
            if world_index > 0 {
                out.push(',');
            }
            push_world(&mut out, world);
        }
        out.push_str("]}");
    }
    out.push_str("],\"churn\":[");
    for (index, row) in report.churn.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_churn(&mut out, row);
    }
    out.push_str("]}");
    out
}

/// One file's on-disk bytes — THE measuring function of this lane
/// (`std::fs::metadata` length). A missing file is an `Err` naming the
/// path; the one sanctioned absence is the `-wal` sibling, which
/// [`wal_bytes`] reads as 0 (a truncated-checkpoint wal may be
/// unlinked).
fn file_bytes(path: &Path) -> Result<u64, String> {
    std::fs::metadata(path)
        .map(|meta| meta.len())
        .map_err(|e| format!("stat {}: {e}", path.display()))
}

/// The `-wal` sibling of a `SQLite` file.
fn wal_path(db_file: &Path) -> PathBuf {
    PathBuf::from(format!("{}-wal", db_file.display()))
}

/// The wal sibling's bytes; absence reads 0 (a truncating checkpoint
/// may unlink it) — the reported-field honesty mechanism.
fn wal_bytes(db_file: &Path) -> Result<u64, String> {
    let wal = wal_path(db_file);
    if wal.exists() {
        file_bytes(&wal)
    } else {
        Ok(0)
    }
}

/// One world's lane wiring: the theory value, the schema surfaces, the
/// generator's expected per-relation counts, and the three loaders —
/// the worlds differ only in these, so the byte protocol itself is
/// written once ([`measure_world`]).
struct WorldSpec<'a, S> {
    world: &'static str,
    theory: S,
    schema: &'static bumbledb::Schema,
    descriptor: bumbledb::schema::SchemaDescriptor,
    /// Generator rows per writable relation (index = relation id) —
    /// the count oracle every lane must agree with.
    expected: Vec<u64>,
    load_engine: &'a dyn Fn(&Db<S>) -> Result<(), String>,
    /// The full parity loader (DDL + indexes + ANALYZE + checkpoint),
    /// returning the live connection for the count cross-check.
    load_indexed: &'a dyn Fn(&Path) -> Result<Connection, String>,
    /// The shared per-relation row streams, into whatever tables the
    /// connection carries — the table-only lane's row source.
    load_rows: &'a dyn Fn(&Connection) -> Result<(), String>,
}

/// Per-relation `SELECT COUNT(*)` over the writable relations.
fn sqlite_counts(
    conn: &Connection,
    schema: &bumbledb::Schema,
    relations: usize,
) -> Result<Vec<u64>, String> {
    (0..relations)
        .map(|rel| {
            let rel = RelationId(u32::try_from(rel).expect("relation ids fit u32"));
            let name = schema.relation(rel).name();
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM \"{name}\""), [], |row| {
                    row.get(0)
                })
                .map_err(|e| format!("COUNT {name}: {e}"))?;
            u64::try_from(count).map_err(|_| format!("COUNT {name}: negative {count}"))
        })
        .collect()
}

/// The lane-local table-only loader: [`sqlmap::table_ddl`] ONLY (plus
/// the closed vocabularies' extension INSERTs — schema surface, their
/// tables carrying only the PRIMARY KEY), the shared row streams, no
/// ANALYZE (table-only is the no-secondary-structure lane;
/// `sqlite_stat1` would itself be secondary structure). The caller
/// counts, checkpoints, drops, and stats.
fn load_sqlite_tableonly<S>(path: &Path, spec: &WorldSpec<'_, S>) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    crate::corpus::configure_sqlite(&conn).map_err(|e| format!("configure: {e}"))?;
    for statement in sqlmap::table_ddl(spec.schema) {
        conn.execute(&statement, [])
            .map_err(|e| format!("table ddl: {e}"))?;
    }
    for statement in sqlmap::extension_ddl(&spec.descriptor) {
        conn.execute(&statement, [])
            .map_err(|e| format!("extension ddl: {e}"))?;
    }
    (spec.load_rows)(&conn)?;
    Ok(conn)
}

/// A truncating WAL checkpoint — the only reading the byte stat admits.
fn checkpoint_truncate(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .map_err(|e| format!("wal_checkpoint(TRUNCATE): {e}"))
}

/// The byte protocol for one (scale, world): engine raw → engine
/// compacted → sqlite indexed → sqlite table-only, then the count
/// cross-check (engine scans == generator sizes == both `SQLite`
/// lanes) before anything is recorded.
fn measure_world<S: bumbledb::Theory + Copy>(
    scale_dir: &Path,
    spec: &WorldSpec<'_, S>,
) -> Result<WorldStorage, String> {
    let world = spec.world;
    let fail = |stage: &str, detail: String| format!("{world}: {stage}: {detail}");

    // ENGINE RAW: create + load, stat live.
    let raw_dir = scale_dir.join(format!("{world}-raw"));
    let db = Db::create(&raw_dir, spec.theory).map_err(|e| fail("create raw", format!("{e:?}")))?;
    (spec.load_engine)(&db)?;
    let engine_raw_bytes = db
        .disk_size()
        .map_err(|e| fail("raw disk_size", format!("{e:?}")))?;

    // ENGINE COMPACTED: compact into a sibling, drop raw, reopen.
    let compacted_dir = scale_dir.join(format!("{world}-compacted"));
    db.compact(&compacted_dir)
        .map_err(|e| fail("compact", format!("{e:?}")))?;
    drop(db);
    let db = Db::open(&compacted_dir, spec.theory)
        .map_err(|e| fail("open compacted", format!("{e:?}")))?;
    let engine_compacted_bytes = db
        .disk_size()
        .map_err(|e| fail("compacted disk_size", format!("{e:?}")))?;
    let mut engine_counts = Vec::with_capacity(spec.expected.len());
    for rel in 0..spec.expected.len() {
        let rel = RelationId(u32::try_from(rel).expect("relation ids fit u32"));
        let count = db
            .read(|snap| Ok(snap.scan(rel)?.count()))
            .map_err(|e| fail("engine scan count", format!("{e:?}")))?;
        engine_counts.push(count as u64);
    }
    drop(db);

    // SQLITE INDEXED: the full parity loader, counts on the live
    // connection, truncating checkpoint, drop, stat.
    let indexed_file = scale_dir.join(format!("{world}-indexed.sqlite"));
    let conn = (spec.load_indexed)(&indexed_file)?;
    let indexed_counts =
        sqlite_counts(&conn, spec.schema, spec.expected.len()).map_err(|e| fail("indexed", e))?;
    checkpoint_truncate(&conn).map_err(|e| fail("indexed", e))?;
    drop(conn);
    let sqlite_indexed_bytes = file_bytes(&indexed_file)?;
    let sqlite_indexed_wal_bytes = wal_bytes(&indexed_file)?;

    // SQLITE TABLE-ONLY: same discipline over the index-free tables.
    let tableonly_file = scale_dir.join(format!("{world}-tableonly.sqlite"));
    let conn = load_sqlite_tableonly(&tableonly_file, spec).map_err(|e| fail("table-only", e))?;
    let tableonly_counts = sqlite_counts(&conn, spec.schema, spec.expected.len())
        .map_err(|e| fail("table-only", e))?;
    checkpoint_truncate(&conn).map_err(|e| fail("table-only", e))?;
    drop(conn);
    let sqlite_tableonly_bytes = file_bytes(&tableonly_file)?;
    let sqlite_tableonly_wal_bytes = wal_bytes(&tableonly_file)?;

    // COUNT CROSS-CHECK: every lane against the generator, before any
    // byte is recorded.
    for (rel, expected) in spec.expected.iter().enumerate() {
        let name = spec
            .schema
            .relation(RelationId(
                u32::try_from(rel).expect("relation ids fit u32"),
            ))
            .name();
        for (lane, got) in [
            ("engine", engine_counts[rel]),
            ("sqlite-indexed", indexed_counts[rel]),
            ("sqlite-tableonly", tableonly_counts[rel]),
        ] {
            if got != *expected {
                return Err(format!(
                    "count cross-check: world {world}, relation {name}, lane {lane}: \
                     {got} rows, generator expects {expected} — nothing reported"
                ));
            }
        }
    }

    Ok(WorldStorage {
        world,
        facts: spec.expected.iter().sum(),
        engine_raw_bytes,
        engine_compacted_bytes,
        sqlite_indexed_bytes,
        sqlite_indexed_wal_bytes,
        sqlite_tableonly_bytes,
        sqlite_tableonly_wal_bytes,
    })
}

/// The ledger world's lane wiring at one config.
fn measure_ledger(scale_dir: &Path, cfg: GenConfig) -> Result<WorldStorage, String> {
    let sizes = Sizes::of(cfg.scale);
    let expected: Vec<u64> = (0..crate::schema::ids::RELATIONS)
        .map(|rel| sizes.rows(RelationId(rel)))
        .collect();
    measure_world(
        scale_dir,
        &WorldSpec {
            world: "ledger",
            theory: crate::schema::Ledger,
            schema: crate::schema::schema(),
            descriptor: bumbledb::Theory::descriptor(crate::schema::Ledger),
            expected,
            load_engine: &|db| {
                crate::corpus::load_bumbledb(db, cfg)
                    .map(|_| ())
                    .map_err(|e| format!("ledger: engine load: {e:?}"))
            },
            load_indexed: &|path| {
                crate::corpus::load_sqlite(path, cfg)
                    .map(|(conn, _)| conn)
                    .map_err(|e| format!("ledger: sqlite indexed load: {e}"))
            },
            load_rows: &|conn| {
                for rel in 0..crate::schema::ids::RELATIONS {
                    crate::corpus::load_sqlite_relation(conn, cfg, RelationId(rel))
                        .map_err(|e| format!("relation {rel} rows: {e}"))?;
                }
                Ok(())
            },
        },
    )
}

/// The calendar world's lane wiring at one config.
fn measure_calendar(scale_dir: &Path, cfg: GenConfig) -> Result<WorldStorage, String> {
    let sizes = CalSizes::of(cfg.scale);
    let expected: Vec<u64> = (0..crate::calendar::ids::RELATIONS)
        .map(|rel| sizes.rows(RelationId(rel)))
        .collect();
    measure_world(
        scale_dir,
        &WorldSpec {
            world: "calendar",
            theory: crate::calendar::Scheduling,
            schema: crate::calendar::schema(),
            descriptor: bumbledb::Theory::descriptor(crate::calendar::Scheduling),
            expected,
            load_engine: &|db| {
                crate::calendar::corpus::load_bumbledb(db, cfg)
                    .map(|_| ())
                    .map_err(|e| format!("calendar: engine load: {e:?}"))
            },
            load_indexed: &|path| {
                crate::calendar::corpus::load_sqlite(path, cfg)
                    .map(|(conn, _)| conn)
                    .map_err(|e| format!("calendar: sqlite indexed load: {e}"))
            },
            load_rows: &|conn| {
                for rel in 0..crate::calendar::ids::RELATIONS {
                    let rel = RelationId(rel);
                    crate::corpus::insert_rows(
                        conn,
                        crate::calendar::schema().relation(rel),
                        crate::calendar::corpus_gen::relation_rows(cfg, rel),
                    )
                    .map_err(|e| format!("relation {} rows: {e}", rel.0))?;
                }
                Ok(())
            },
        },
    )
}

/// Reads the churn-checkpoint directory contract: immediate
/// subdirectories in lexicographic name order, one [`ChurnRow`] each,
/// `None` for absent artifacts. An empty churn directory is an `Err`
/// naming the contract.
fn measure_churn(dir: &Path) -> Result<Vec<ChurnRow>, String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("churn dir {}: {e}", dir.display()))?;
    let mut checkpoints: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("churn dir {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            checkpoints.push((entry.file_name().to_string_lossy().into_owned(), path));
        }
    }
    if checkpoints.is_empty() {
        return Err(format!(
            "churn contract: {} holds no checkpoint subdirectories — each checkpoint \
             is one immediate subdirectory holding `db/data.mdb` (optional) and/or \
             `oracle.sqlite` (+ optional `oracle.sqlite-wal`)",
            dir.display()
        ));
    }
    checkpoints.sort_by(|a, b| a.0.cmp(&b.0));
    checkpoints
        .into_iter()
        .map(|(name, path)| {
            let engine = path.join("db").join("data.mdb");
            let engine_bytes = if engine.exists() {
                Some(file_bytes(&engine)?)
            } else {
                None
            };
            let oracle = path.join("oracle.sqlite");
            let (sqlite_bytes, sqlite_wal_bytes) = if oracle.exists() {
                (Some(file_bytes(&oracle)?), Some(wal_bytes(&oracle)?))
            } else {
                (None, None)
            };
            Ok(ChurnRow {
                name,
                engine_bytes,
                sqlite_bytes,
                sqlite_wal_bytes,
            })
        })
        .collect()
}

fn opt_cell(value: Option<u64>) -> String {
    value.map_or_else(|| "—".to_owned(), |v| v.to_string())
}

/// The human-readable table — hand-rolled, like `scenarios/render.rs`.
fn render(report: &StorageReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Storage report\n");
    let _ = writeln!(
        out,
        "On-disk bytes per corpus scale, both engines — report-class, no clocks. \
         Engine bytes = `data.mdb` (raw after load; compacted via `Db::compact`). \
         `SQLite` parity config: WAL, `synchronous=FULL`, `fullfsync=ON`, 256 MiB \
         cache, prepared statements in 4096-row transactions; indexed lane = full \
         DDL + family indexes + ANALYZE; table-only lane = index-free STRICT \
         tables, no ANALYZE. Every file statted after a truncating WAL checkpoint \
         with the connection dropped; wal bytes are reported so an uncheckpointed \
         emission is visible. facts = writable-relation rows (closed vocabularies \
         excluded). seed {}.\n",
        report.seed
    );
    let _ = writeln!(
        out,
        "| scale | world | facts | engine raw | engine compacted | B/fact | sqlite indexed | B/fact | sqlite table-only | B/fact | wal ix/to |"
    );
    let _ = writeln!(
        out,
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    for scale in &report.scales {
        for world in &scale.worlds {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {:.1} | {} | {:.1} | {} | {:.1} | {}/{} |",
                scale.scale,
                world.world,
                world.facts,
                world.engine_raw_bytes,
                world.engine_compacted_bytes,
                super::per_unit(world.engine_compacted_bytes, world.facts),
                world.sqlite_indexed_bytes,
                super::per_unit(world.sqlite_indexed_bytes, world.facts),
                world.sqlite_tableonly_bytes,
                super::per_unit(world.sqlite_tableonly_bytes, world.facts),
                world.sqlite_indexed_wal_bytes,
                world.sqlite_tableonly_wal_bytes,
            );
        }
    }
    if !report.churn.is_empty() {
        let _ = writeln!(out, "\n## Churn checkpoints\n");
        let _ = writeln!(
            out,
            "| checkpoint | engine bytes | sqlite bytes | sqlite wal |"
        );
        let _ = writeln!(out, "|---|---:|---:|---:|");
        for row in &report.churn {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} |",
                row.name,
                opt_cell(row.engine_bytes),
                opt_cell(row.sqlite_bytes),
                opt_cell(row.sqlite_wal_bytes),
            );
        }
    }
    out
}

/// The storage lane entry point: per scale, per world, the four byte
/// lanes under the count cross-check; then the churn checkpoints if a
/// churn directory was named. Writes `storage-report.json` and
/// `storage-report.md` under the out directory, prints the markdown,
/// and removes the scratch stores (the report is the artifact;
/// L-scale scratch is gigabytes).
///
/// # Errors
///
/// Setup/IO failures, engine or `SQLite` load failures, a count
/// cross-check inequality (named by world/relation/lane), or an empty
/// churn directory — all as messages; nothing is reported on error.
pub fn run(args: &StorageArgs) -> Result<i32, String> {
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-storage",
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    let scratch = out_dir.join("scratch");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).map_err(|e| format!("scratch {}: {e}", scratch.display()))?;

    let mut scales = Vec::new();
    for scale in &args.scales {
        let cfg = GenConfig {
            seed: args.seed,
            scale: *scale,
        };
        let scale_dir = scratch.join(scale.label());
        std::fs::create_dir_all(&scale_dir)
            .map_err(|e| format!("scratch {}: {e}", scale_dir.display()))?;
        scales.push(ScaleStorage {
            scale: scale.label(),
            worlds: vec![
                measure_ledger(&scale_dir, cfg)?,
                measure_calendar(&scale_dir, cfg)?,
            ],
        });
    }

    let churn = match &args.churn_dir {
        Some(dir) => measure_churn(dir)?,
        None => Vec::new(),
    };

    let report = StorageReport {
        provenance: report::provenance(Path::new(".")),
        seed: args.seed,
        scales,
        churn,
    };
    std::fs::write(out_dir.join("storage-report.json"), to_json(&report))
        .map_err(|e| format!("artifact: {e}"))?;
    let markdown = render(&report);
    std::fs::write(out_dir.join("storage-report.md"), &markdown)
        .map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    std::fs::remove_dir_all(&scratch).map_err(|e| format!("scratch cleanup: {e}"))?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus_gen::Scale;
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

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bumbledb-bench-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn report_json_shape_is_pinned() {
        let report = StorageReport {
            provenance: provenance(),
            seed: 7,
            scales: vec![ScaleStorage {
                scale: "S",
                worlds: vec![WorldStorage {
                    world: "ledger",
                    facts: 1000,
                    engine_raw_bytes: 4000,
                    engine_compacted_bytes: 2000,
                    sqlite_indexed_bytes: 8000,
                    sqlite_indexed_wal_bytes: 128,
                    sqlite_tableonly_bytes: 3000,
                    sqlite_tableonly_wal_bytes: 64,
                }],
            }],
            churn: vec![ChurnRow {
                name: "delete-half".to_owned(),
                engine_bytes: Some(1500),
                sqlite_bytes: None,
                sqlite_wal_bytes: None,
            }],
        };
        let parsed = crate::json::parse(&to_json(&report)).expect("valid JSON");
        assert_eq!(
            parsed
                .get("provenance")
                .and_then(|p| p.get("git_rev"))
                .and_then(Value::as_str),
            Some("deadbeef")
        );
        assert!(
            parsed
                .get("provenance")
                .and_then(|p| p.get("shared_machine"))
                .is_none(),
            "boost-off keeps the pre-boost provenance shape"
        );
        assert_eq!(parsed.get("seed").and_then(Value::as_f64), Some(7.0));
        let scales = parsed
            .get("scales")
            .and_then(Value::as_arr)
            .expect("scales");
        assert_eq!(scales.len(), 1);
        assert_eq!(scales[0].get("scale").and_then(Value::as_str), Some("S"));
        let worlds = scales[0]
            .get("worlds")
            .and_then(Value::as_arr)
            .expect("worlds");
        let world = &worlds[0];
        assert_eq!(world.get("world").and_then(Value::as_str), Some("ledger"));
        assert_eq!(world.get("facts").and_then(Value::as_f64), Some(1000.0));
        assert_eq!(
            world.get("engine_raw_bytes").and_then(Value::as_f64),
            Some(4000.0)
        );
        assert_eq!(
            world.get("engine_compacted_bytes").and_then(Value::as_f64),
            Some(2000.0)
        );
        // The derived per-fact columns: compacted/facts and friends.
        assert_eq!(
            world.get("engine_bytes_per_fact").and_then(Value::as_f64),
            Some(2.0)
        );
        assert_eq!(
            world
                .get("sqlite_indexed_bytes_per_fact")
                .and_then(Value::as_f64),
            Some(8.0)
        );
        assert_eq!(
            world
                .get("sqlite_tableonly_bytes_per_fact")
                .and_then(Value::as_f64),
            Some(3.0)
        );
        assert_eq!(
            world
                .get("sqlite_indexed_wal_bytes")
                .and_then(Value::as_f64),
            Some(128.0)
        );
        assert_eq!(
            world
                .get("sqlite_tableonly_wal_bytes")
                .and_then(Value::as_f64),
            Some(64.0)
        );
        let churn = parsed.get("churn").and_then(Value::as_arr).expect("churn");
        assert_eq!(
            churn[0].get("name").and_then(Value::as_str),
            Some("delete-half")
        );
        assert_eq!(
            churn[0].get("engine_bytes").and_then(Value::as_f64),
            Some(1500.0)
        );
        assert_eq!(churn[0].get("sqlite_bytes"), Some(&Value::Null));
        assert_eq!(churn[0].get("sqlite_wal_bytes"), Some(&Value::Null));
    }

    /// The whole lane at Tiny: both worlds measured, every byte field
    /// positive, compaction never grows the store, dropping the
    /// secondary structure never grows the `SQLite` file, wal fields
    /// present, and `facts` equals the generator sums computed
    /// independently here.
    #[test]
    #[expect(
        clippy::cast_precision_loss,
        reason = "reporting accepts lossy integer-to-float conversion"
    )]
    fn tiny_end_to_end_measures_both_engines() {
        let dir = scratch("storage-lane-e2e");
        let out = dir.join("out");
        let code = run(&StorageArgs {
            scales: vec![Scale::Tiny],
            seed: 1,
            dir: dir.clone(),
            churn_dir: None,
            out: Some(out.clone()),
        })
        .expect("the lane runs");
        assert_eq!(code, 0);
        let text = std::fs::read_to_string(out.join("storage-report.json")).expect("json artifact");
        let parsed = crate::json::parse(&text).expect("valid JSON");
        assert_eq!(parsed.get("seed").and_then(Value::as_f64), Some(1.0));
        let scales = parsed
            .get("scales")
            .and_then(Value::as_arr)
            .expect("scales");
        assert_eq!(scales.len(), 1);
        assert_eq!(scales[0].get("scale").and_then(Value::as_str), Some("Tiny"));
        let worlds = scales[0]
            .get("worlds")
            .and_then(Value::as_arr)
            .expect("worlds");
        assert_eq!(worlds.len(), 2, "two worlds");

        let ledger_sizes = Sizes::of(Scale::Tiny);
        let ledger_facts: u64 = (0..crate::schema::ids::RELATIONS)
            .map(|rel| ledger_sizes.rows(RelationId(rel)))
            .sum();
        let cal_sizes = CalSizes::of(Scale::Tiny);
        let cal_facts: u64 = (0..crate::calendar::ids::RELATIONS)
            .map(|rel| cal_sizes.rows(RelationId(rel)))
            .sum();
        for (world, expected_facts) in [(&worlds[0], ledger_facts), (&worlds[1], cal_facts)] {
            for field in [
                "facts",
                "engine_raw_bytes",
                "engine_compacted_bytes",
                "sqlite_indexed_bytes",
                "sqlite_tableonly_bytes",
            ] {
                let value = world.get(field).and_then(Value::as_f64).expect(field);
                assert!(value > 0.0, "{field} must be positive, got {value}");
            }
            // Wal bytes are REPORTED fields (present, possibly 0 —
            // the truncating checkpoint is exactly what makes 0 the
            // honest reading).
            assert!(
                world
                    .get("sqlite_indexed_wal_bytes")
                    .and_then(Value::as_f64)
                    .is_some()
            );
            assert!(
                world
                    .get("sqlite_tableonly_wal_bytes")
                    .and_then(Value::as_f64)
                    .is_some()
            );
            let raw = world
                .get("engine_raw_bytes")
                .and_then(Value::as_f64)
                .expect("raw");
            let compacted = world
                .get("engine_compacted_bytes")
                .and_then(Value::as_f64)
                .expect("compacted");
            assert!(compacted <= raw, "compaction never grows the store");
            let indexed = world
                .get("sqlite_indexed_bytes")
                .and_then(Value::as_f64)
                .expect("indexed");
            let tableonly = world
                .get("sqlite_tableonly_bytes")
                .and_then(Value::as_f64)
                .expect("tableonly");
            assert!(
                tableonly <= indexed,
                "dropping the indexes never grows the file"
            );
            assert_eq!(
                world.get("facts").and_then(Value::as_f64),
                Some(expected_facts as f64),
                "facts equals the generator sum"
            );
        }
        assert_eq!(
            worlds[0].get("world").and_then(Value::as_str),
            Some("ledger")
        );
        assert_eq!(
            worlds[1].get("world").and_then(Value::as_str),
            Some("calendar")
        );
        // The report is the artifact; the scratch stores are gone.
        assert!(!out.join("scratch").exists(), "scratch removed");
        assert!(out.join("storage-report.md").exists(), "markdown artifact");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The engine's `disk_size()` equals the `data.mdb` stat — the
    /// churn seam's stat path and the live path cannot drift.
    #[test]
    fn disk_size_equals_the_stat_path() {
        let dir = scratch("storage-lane-disksize");
        let store = dir.join("db");
        let db = Db::create(&store, crate::schema::Ledger).expect("create");
        crate::corpus::load_bumbledb(
            &db,
            GenConfig {
                seed: 1,
                scale: Scale::Tiny,
            },
        )
        .expect("load");
        assert_eq!(
            db.disk_size().expect("disk_size"),
            file_bytes(&store.join("data.mdb")).expect("stat")
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The churn-checkpoint directory contract: subdirectories in name
    /// order, absent artifacts as JSON null.
    #[test]
    fn churn_checkpoints_are_measured() {
        let dir = scratch("storage-lane-churn");
        let churn = dir.join("churn");
        let c0 = churn.join("c0");
        let c1 = churn.join("c1");
        std::fs::create_dir_all(&c0).expect("c0");
        std::fs::create_dir_all(&c1).expect("c1");
        let cfg = GenConfig {
            seed: 1,
            scale: Scale::Tiny,
        };
        // c0: a Tiny compacted engine store under db/ plus a Tiny
        // sqlite store; c1: only a sqlite file.
        let load_dir = dir.join("db-load");
        let db = Db::create(&load_dir, crate::schema::Ledger).expect("create");
        crate::corpus::load_bumbledb(&db, cfg).expect("load");
        db.compact(&c0.join("db")).expect("compact");
        drop(db);
        let (conn, _) =
            crate::corpus::load_sqlite(&c0.join("oracle.sqlite"), cfg).expect("sqlite load");
        drop(conn);
        let conn = Connection::open(c1.join("oracle.sqlite")).expect("open");
        conn.execute_batch("CREATE TABLE t(x INTEGER); INSERT INTO t VALUES (1)")
            .expect("fill");
        drop(conn);

        let out = dir.join("out");
        let code = run(&StorageArgs {
            scales: vec![],
            seed: 1,
            dir: dir.clone(),
            churn_dir: Some(churn),
            out: Some(out.clone()),
        })
        .expect("the lane runs");
        assert_eq!(code, 0);
        let text = std::fs::read_to_string(out.join("storage-report.json")).expect("json artifact");
        let parsed = crate::json::parse(&text).expect("valid JSON");
        let churn = parsed.get("churn").and_then(Value::as_arr).expect("churn");
        assert_eq!(churn.len(), 2, "two checkpoints");
        assert_eq!(churn[0].get("name").and_then(Value::as_str), Some("c0"));
        assert_eq!(churn[1].get("name").and_then(Value::as_str), Some("c1"));
        assert!(
            churn[0]
                .get("engine_bytes")
                .and_then(Value::as_f64)
                .expect("c0 engine bytes")
                > 0.0
        );
        assert!(
            churn[0]
                .get("sqlite_bytes")
                .and_then(Value::as_f64)
                .expect("c0 sqlite bytes")
                > 0.0
        );
        // c1 carries no engine store: null in the JSON, by contract.
        assert_eq!(churn[1].get("engine_bytes"), Some(&Value::Null));
        assert!(
            churn[1]
                .get("sqlite_bytes")
                .and_then(Value::as_f64)
                .expect("c1 sqlite bytes")
                > 0.0
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
