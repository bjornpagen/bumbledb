//! The scenario suites (docs/architecture/50-validation.md, extended):
//! additional schema+corpus+query worlds beyond the ledger, each
//! stressing a different regime — join-order pressure, graph fan-out,
//! OLAP rollups, point-lookup overhead. Every scenario runs under the
//! ledger benchmark's exact protocol (`SQLite` file-backed, WAL,
//! `synchronous=FULL`, fully indexed, prepared statements reused,
//! `ANALYZE`, DISTINCT in the timed SQL, median-of-samples), and every
//! query is **oracle-gated before it is timed**: each query × param set
//! must produce value-identical multisets on both engines or the run
//! fails loudly — no timing without agreement.
//!
//! Scenarios are `Kind::Report`-class by design: they exist to *measure*
//! regimes, not to gate the suite. The ledger's ten families remain the
//! gate.

pub mod graph;
pub mod joins;
pub mod olap;
pub mod points;

use std::path::Path;

use bumbledb::schema::{Schema, ValueType};
use bumbledb::{Db, Query, RelationId, ResultBuffer, Value};
use rusqlite::Connection;

use crate::harness::{self, Protocol, Rotation};
use crate::sqlite_run::PreparedFamily;
use crate::translate::translate;
use crate::{compare, corpus, sqlmap};

/// One scenario query: IR + seeded param sets + a one-line regime note.
pub struct ScenarioQuery {
    pub name: &'static str,
    pub query: fn() -> Query,
    /// Seeded param sets; rotation order is the measurement order.
    pub params: fn(u64) -> Vec<Vec<Value>>,
    /// What regime this query stresses (rendered in the report).
    pub about: &'static str,
}

/// One scenario: a schema, a deterministic corpus, extra `SQLite`
/// indexes for its predicate columns (FK/unique indexes come from the
/// schema constraints via [`sqlmap::expected_indexes`]), and a query
/// list.
pub struct Scenario {
    pub name: &'static str,
    pub about: &'static str,
    pub schema: fn() -> &'static Schema,
    /// Relations in FK dependency order with their row iterators.
    #[allow(clippy::type_complexity)]
    pub rows: fn(u64) -> Vec<(RelationId, Box<dyn Iterator<Item = Vec<Value>>>)>,
    /// `CREATE INDEX` statements for predicate columns the constraint
    /// registry does not already cover.
    pub extra_indexes: &'static [&'static str],
    pub queries: fn() -> Vec<ScenarioQuery>,
}

/// The registry, in report order.
#[must_use]
pub fn all() -> Vec<Scenario> {
    vec![
        joins::scenario(),
        graph::scenario(),
        olap::scenario(),
        points::scenario(),
    ]
}

/// One measured query row of the scenario report.
pub struct QueryReport {
    pub scenario: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    /// Median result rows across the rotation (the work sanity check).
    pub rows: u64,
    pub ours: harness::Stats,
    pub theirs: harness::Stats,
    pub ratio_p50: f64,
}

/// A loaded scenario store pair.
struct Stores {
    db: Db<'static>,
    conn: Connection,
}

/// Deterministic per-row seed (the same construction as the ledger
/// generator's: corpus content is a pure function of (seed, rel, row)).
#[must_use]
pub fn mix(seed: u64, rel: u32, row: u64) -> u64 {
    let mut h = seed ^ 0x9E37_79B9_7F4A_7C15;
    h ^= u64::from(rel).wrapping_mul(0xA24B_AED4_963E_E407);
    h ^= row.wrapping_mul(0x9FB2_1C65_1E98_DF25);
    h ^= h >> 28;
    h = h.wrapping_mul(0x2545_F491_4F6C_DD1D);
    h ^ (h >> 28)
}

/// Loads one scenario into a fresh store pair under
/// `<dir>/scenarios/<name>` (delete-and-recreated — scenario stores are
/// tool scratch, never user data).
fn load(dir: &Path, scenario: &Scenario, seed: u64) -> Result<Stores, String> {
    let root = dir.join("scenarios").join(scenario.name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).map_err(|e| format!("scenario dir: {e}"))?;
    let schema = (scenario.schema)();

    let db = Db::create(&root.join("db"), schema).map_err(|e| format!("create db: {e:?}"))?;
    let conn = Connection::open(root.join("oracle.sqlite")).map_err(|e| format!("sqlite: {e}"))?;
    corpus::configure_sqlite(&conn).map_err(|e| format!("configure sqlite: {e}"))?;
    for statement in sqlmap::schema_ddl(schema) {
        conn.execute(&statement, [])
            .map_err(|e| format!("ddl: {e}"))?;
    }

    let mut total = 0u64;
    for (rel, rows) in (scenario.rows)(seed) {
        let rows: Vec<Vec<Value>> = rows.collect();
        total += rows.len() as u64;
        db.bulk_load(rel, rows.iter().cloned())
            .map_err(|e| format!("{}: bulk_load: {e}", scenario.name))?;
        load_sqlite_rows(&conn, schema, rel, &rows)?;
    }
    for statement in scenario.extra_indexes {
        conn.execute(statement, [])
            .map_err(|e| format!("index: {e}"))?;
    }
    conn.execute_batch("ANALYZE")
        .map_err(|e| format!("analyze: {e}"))?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .map_err(|e| format!("checkpoint: {e}"))?;
    eprintln!(
        "scenario {}: loaded {total} facts x 2 engines",
        scenario.name
    );
    Ok(Stores { db, conn })
}

/// The `SQLite` mirror load for one relation (the ledger loader is
/// generator-coupled; this one takes the rows).
fn load_sqlite_rows(
    conn: &Connection,
    schema: &Schema,
    rel: RelationId,
    rows: &[Vec<Value>],
) -> Result<(), String> {
    let relation = schema.relation(rel);
    let placeholders = (1..=relation.fields().len())
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let insert = format!(
        "INSERT INTO \"{}\" VALUES ({placeholders})",
        relation.name()
    );
    for chunk in rows.chunks(4096) {
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("begin: {e}"))?;
        {
            let mut stmt = conn
                .prepare_cached(&insert)
                .map_err(|e| format!("prepare: {e}"))?;
            for row in chunk {
                let params: Vec<rusqlite::types::Value> =
                    row.iter().map(sqlmap::to_sql_value).collect();
                stmt.execute(rusqlite::params_from_iter(params))
                    .map_err(|e| format!("insert: {e}"))?;
            }
        }
        conn.execute_batch("COMMIT")
            .map_err(|e| format!("commit: {e}"))?;
    }
    Ok(())
}

/// Gates then times one query on both engines. The gate compares every
/// param set's result multisets (`compare::multisets`) — a disagreement
/// is an error naming the query and set, and nothing gets timed.
fn run_query(
    stores: &Stores,
    scenario: &Scenario,
    sq: &ScenarioQuery,
    seed: u64,
    proto: Protocol,
) -> Result<QueryReport, String> {
    let schema = (scenario.schema)();
    let query = (sq.query)();
    let sets = (sq.params)(seed);
    let mut prepared = stores
        .db
        .prepare(&query)
        .map_err(|e| format!("{}/{}: prepare: {e:?}", scenario.name, sq.name))?;
    let types: Vec<ValueType> = prepared.column_types().cloned().collect();
    let translated =
        translate(&query, schema).map_err(|e| format!("{}/{}: {e}", scenario.name, sq.name))?;

    // The oracle gate: agreement on every param set before any timing.
    for (idx, params) in sets.iter().enumerate() {
        let mut buffer = ResultBuffer::new();
        stores
            .db
            .read(|snap| snap.execute(&mut prepared, params, &mut buffer))
            .map_err(|e| format!("{}/{}: execute: {e:?}", scenario.name, sq.name))?;
        let ours = compare::from_buffer(&buffer, &types);
        let mut stmt = stores
            .conn
            .prepare_cached(&translated.sql)
            .map_err(|e| format!("{}/{}: oracle prepare: {e}", scenario.name, sq.name))?;
        let theirs = compare::from_sqlite(&mut stmt, &translated.params, params, &types)
            .map_err(|e| format!("{}/{}: oracle execute: {e}", scenario.name, sq.name))?;
        compare::multisets(ours, theirs).map_err(|mismatch| {
            format!(
                "{}/{} param set {idx}: ENGINES DISAGREE — not timing a wrong answer\n{mismatch}",
                scenario.name, sq.name
            )
        })?;
    }

    // Timing, the ledger protocol: rotation across param sets, medians.
    let mut rotation = Rotation::new(sets.clone());
    let mut buffer = ResultBuffer::new();
    let db = &stores.db;
    let ours = harness::measure(proto, || {
        let params = rotation.next_set().to_vec();
        db.read(|snap| snap.execute(&mut prepared, &params, &mut buffer))
            .map_err(|e| format!("execute: {e:?}"))?;
        Ok(buffer.len() as u64)
    })?;

    let mut sqlite_family = PreparedFamily::new(&stores.conn, &translated, types)?;
    let mut rotation = Rotation::new(sets);
    let theirs = harness::measure(proto, || {
        crate::sqlite_run::sample(&mut sqlite_family, rotation.next_set())
    })?;

    #[allow(clippy::cast_precision_loss)]
    let ratio_p50 = ours.stats.p50 as f64 / theirs.stats.p50.max(1) as f64;
    Ok(QueryReport {
        scenario: scenario.name,
        name: sq.name,
        about: sq.about,
        rows: ours.work / u64::from(proto.samples.max(1)),
        ours: ours.stats,
        theirs: theirs.stats,
        ratio_p50,
    })
}

/// Runs every scenario (or the selected subset): load, gate, time,
/// report. Returns the rendered markdown; the caller writes artifacts.
///
/// # Errors
///
/// Load/prepare/translate failures and oracle disagreements, as
/// messages naming the scenario and query.
pub fn run(
    dir: &Path,
    seed: u64,
    proto: Protocol,
    only: Option<&[String]>,
) -> Result<(String, Vec<QueryReport>), String> {
    let mut reports = Vec::new();
    for scenario in all() {
        if let Some(only) = only {
            if !only.iter().any(|n| n == scenario.name) {
                continue;
            }
        }
        let stores = load(dir, &scenario, seed)?;
        for sq in (scenario.queries)() {
            eprintln!("scenario {}: {}", scenario.name, sq.name);
            reports.push(run_query(&stores, &scenario, &sq, seed, proto)?);
        }
    }
    if reports.is_empty() {
        return Err("no scenario selected".to_owned());
    }
    Ok((render(&reports, proto), reports))
}

/// Geometric mean of the p50 ratios (the honest cross-query summary:
/// ratios multiply, so the geomean is the scale-free center).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn geomean(reports: &[&QueryReport]) -> f64 {
    if reports.is_empty() {
        return 1.0;
    }
    let log_sum: f64 = reports.iter().map(|r| r.ratio_p50.max(1e-9).ln()).sum();
    (log_sum / reports.len() as f64).exp()
}

#[allow(clippy::cast_precision_loss)]
fn us(ns: u64) -> f64 {
    ns as f64 / 1000.0
}

/// Renders the scenario report as markdown.
#[must_use]
pub fn render(reports: &[QueryReport], proto: Protocol) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "# Scenario benchmarks\n");
    let _ = writeln!(
        out,
        "Report-class measurements over non-ledger worlds; every query \
         oracle-gated (value-identical results on both engines) before \
         timing. Protocol: {} warmups, {} samples, medians; `SQLite` \
         file-backed WAL `synchronous=FULL`, fully indexed, prepared \
         statements reused, ANALYZE run. ratio = ours/theirs (lower is \
         better; <1 = bumbledb faster).\n",
        proto.warmups, proto.samples,
    );
    let mut scenario = "";
    for r in reports {
        if r.scenario != scenario {
            scenario = r.scenario;
            let in_scenario: Vec<&QueryReport> =
                reports.iter().filter(|q| q.scenario == scenario).collect();
            let _ = writeln!(
                out,
                "\n## {scenario} (geomean ratio {:.2})\n",
                geomean(&in_scenario)
            );
            let _ = writeln!(
                out,
                "| query | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |"
            );
            let _ = writeln!(out, "|---|---:|---:|---:|---:|---|");
        }
        let _ = writeln!(
            out,
            "| {} | {} | {:.1} | {:.1} | {:.2} | {} |",
            r.name,
            r.rows,
            us(r.ours.p50),
            us(r.theirs.p50),
            r.ratio_p50,
            r.about,
        );
    }
    let every: Vec<&QueryReport> = reports.iter().collect();
    let _ = writeln!(
        out,
        "\nOverall geomean ratio across {} queries: **{:.2}**.",
        every.len(),
        geomean(&every)
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every scenario query validates, prepares, and translates against
    /// its own schema (no corpus needed), and its param sets are seeded
    /// deterministic with at least one set.
    #[test]
    fn every_scenario_query_prepares_and_translates() {
        for scenario in all() {
            let dir =
                std::env::temp_dir().join(format!("bumbledb-scenario-check-{}", scenario.name));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("scratch dir");
            let schema = (scenario.schema)();
            let db = Db::create(&dir, schema).expect("create");
            for sq in (scenario.queries)() {
                db.prepare(&(sq.query)())
                    .unwrap_or_else(|e| panic!("{}/{}: validation: {e:?}", scenario.name, sq.name));
                translate(&(sq.query)(), schema)
                    .unwrap_or_else(|e| panic!("{}/{}: translation: {e}", scenario.name, sq.name));
                let a = (sq.params)(1);
                let b = (sq.params)(1);
                assert_eq!(a, b, "{}/{}: params must be seeded", scenario.name, sq.name);
                assert!(
                    !a.is_empty(),
                    "{}/{}: at least one param set",
                    scenario.name,
                    sq.name
                );
            }
            drop(db);
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// Scenario corpora are pure functions of the seed: the first row of
    /// every relation reproduces.
    #[test]
    fn scenario_rows_are_deterministic() {
        for scenario in all() {
            let first = |seed: u64| -> Vec<Vec<Value>> {
                (scenario.rows)(seed)
                    .into_iter()
                    .filter_map(|(_, mut rows)| rows.next())
                    .collect()
            };
            assert_eq!(first(7), first(7), "{}: rows must be seeded", scenario.name);
        }
    }
}
