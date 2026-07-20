//! The per-run driver loop — lockstep twins, per-lane wall-time
//! windows, maintenance-included honesty. This module IS timing code,
//! but the charter holds: it only ever executes in cargo tests at
//! `Tiny`/smoke scale asserting correctness and shape; every timed
//! NUMBER arrives via the owner's night session.
//!
//! Honesty by accounting shape, not by remembering: each lane owns a
//! `window_ns` that accumulates ONLY that lane's own transactions (and,
//! for the maintained lane, its own maintenance), so interleaved twins
//! cannot contaminate each other's throughput series — the number is
//! right by construction.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::churn::report::{Counters, Engine, LaneSeries, RunSeries, SamplePoint};
use crate::corpus_gen::Sizes;

use super::engines;
use super::lanes::{self, RunSpec};
use super::ops;
use super::probes;
use super::verify_end;

/// One `SQLite` twin's run-local state: the connection, its own
/// throughput window, its own maintenance ledger, and its series so
/// far. A local of [`run_spec`] (the `displaced::bench_families`
/// lifetime pattern — connections and prepared things never outlive the
/// run).
struct SqliteLaneState {
    label: &'static str,
    maintained: bool,
    path: PathBuf,
    conn: rusqlite::Connection,
    window_ns: u64,
    maintenance_ns: u64,
    samples: Vec<SamplePoint>,
}

/// The elapsed wall time since `start`, in nanoseconds.
fn elapsed_ns(start: &Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).expect("an elapsed span fits u64 nanoseconds")
}

/// One window's throughput: committed cycles per second of the lane's
/// OWN accumulated wall time.
#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn commits_per_sec(cycles: u64, window_ns: u64) -> f64 {
    cycles as f64 / (window_ns as f64 / 1e9)
}

/// One timed maintenance statement on a maintained lane. VACUUM/ANALYZE
/// time is charged to BOTH the throughput window and the maintenance
/// ledger — the mandate's explicit honesty rule: the operator's
/// maintenance is part of the lane's real life, so it lands in
/// `SQLite`'s own series, itemized.
fn maintain(lane: &mut SqliteLaneState, sql: &str) -> Result<(), String> {
    let start = Instant::now();
    lane.conn
        .execute_batch(sql)
        .map_err(|e| format!("churn maintenance {sql} ({}): {e}", lane.label))?;
    let elapsed = elapsed_ns(&start);
    lane.window_ns += elapsed;
    lane.maintenance_ns += elapsed;
    Ok(())
}

/// One non-negative PRAGMA counter, as the report's `u64`.
fn pragma_u64(conn: &rusqlite::Connection, pragma: &str) -> Result<u64, String> {
    let value: i64 = conn
        .query_row(pragma, [], |row| row.get(0))
        .map_err(|e| format!("churn sample {pragma}: {e}"))?;
    u64::try_from(value).map_err(|_| format!("churn sample {pragma}: negative count {value}"))
}

/// Drives one registry row end to end: builds the twins under
/// `scratch/<name>/`, applies every cycle's identical logical
/// operations to every store, charges maintenance into the maintained
/// lane's own window, samples the probe registry on the configured
/// stride (the per-sample cross-lane oracle gate rides the sampler by
/// construction — [`probes::sample_sqlite`] takes the ours-side
/// [`probes::ProbeRun`] by argument), and closes with the three-way end
/// gate plus the working-set law.
///
/// # Errors
///
/// Validation refusals, engine and `SQLite` errors, and gate
/// disagreements — each message names the lane or knob.
///
/// # Panics
///
/// On a broken working-set law at the end of the run, and on the
/// monotone-burn invariant inside [`engines::apply_ours`] — both
/// programmer errors, loud by design.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one run's full protocol, linear
pub fn run_spec(
    spec: &RunSpec,
    cfg: &ops::ChurnConfig,
    scratch: &Path,
) -> Result<RunSeries, String> {
    ops::validate(cfg, &spec.mix)?;
    let sizes = Sizes::of(cfg.r#gen.scale);
    let working_set = sizes.postings;
    let dir = scratch.join(spec.name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("churn scratch: {e}"))?;
    eprintln!("churn: run {}, loading twins", spec.name);
    let mut ours = engines::create_ours(&dir.join("ours"), cfg.r#gen, spec.ours)?;
    let mut mirrors: Vec<SqliteLaneState> = Vec::with_capacity(spec.sqlite.len());
    for kind in spec.sqlite {
        let path = dir.join(format!("{}.sqlite", kind.label()));
        let conn = engines::create_sqlite(&path, cfg.r#gen, kind.sync())?;
        mirrors.push(SqliteLaneState {
            label: kind.label(),
            maintained: kind.maintained(),
            path,
            conn,
            window_ns: 0,
            maintenance_ns: 0,
            samples: Vec::new(),
        });
    }
    let mut live = ops::LiveSet::from_corpus(cfg.r#gen);
    let mut ours_window_ns: u64 = 0;
    let mut ours_samples: Vec<SamplePoint> = Vec::new();
    for cycle in 1..=cfg.cycles {
        let plan = ops::cycle_plan(cfg.r#gen, &spec.mix, cycle, live.len());
        let removed = live.resolve(&plan);
        let start = Instant::now();
        let added = engines::apply_ours(&mut ours, &removed, &plan.bodies)?;
        ours_window_ns += elapsed_ns(&start);
        for lane in &mut mirrors {
            let start = Instant::now();
            engines::apply_sqlite(&lane.conn, &removed, &added)
                .map_err(|e| format!("{}: {e}", lane.label))?;
            lane.window_ns += elapsed_ns(&start);
        }
        // The operator's schedule, maintained lanes only — charged into
        // the lane's own window by `maintain` (honesty by accounting
        // shape).
        for lane in mirrors.iter_mut().filter(|lane| lane.maintained) {
            if cycle.is_multiple_of(cfg.vacuum_every) {
                maintain(lane, "VACUUM")?;
            }
            if cycle.is_multiple_of(cfg.analyze_every) {
                maintain(lane, "ANALYZE")?;
            }
        }
        live.apply(&plan, added);
        if cycle.is_multiple_of(cfg.sample_every) {
            eprintln!(
                "churn: run {}, sampling at cycle {cycle}/{}",
                spec.name, cfg.cycles
            );
            let mut ours_probes = Vec::with_capacity(probes::all().len());
            let mut mirror_probes: Vec<Vec<probes::ProbeSample>> = mirrors
                .iter()
                .map(|_| Vec::with_capacity(probes::all().len()))
                .collect();
            for probe in probes::all() {
                let sets = probes::draws(probe, cfg.r#gen, &live, cycle);
                let run = probes::sample_ours(&ours.db, probe, &sets)?;
                for (lane, taken) in mirrors.iter().zip(mirror_probes.iter_mut()) {
                    // The per-sample cross-lane oracle gate rides the
                    // sampler by construction: `sample_sqlite` takes the
                    // ours-side reference answers by argument.
                    taken.push(probes::sample_sqlite(
                        &lane.conn, probe, &sets, &run, lane.label,
                    )?);
                }
                ours_probes.push(run.sample);
            }
            let generation = ours
                .db
                .generation()
                .map_err(|e| format!("churn sample generation: {e:?}"))?
                .value();
            ours_samples.push(SamplePoint {
                cycle,
                probes: ours_probes,
                commits_per_sec: commits_per_sec(cfg.sample_every, ours_window_ns),
                maintenance_ns: 0,
                disk_bytes: ours
                    .db
                    .disk_size()
                    .map_err(|e| format!("churn sample disk size: {e:?}"))?,
                counters: Counters::Ours {
                    generation,
                    id_high_water: ours.last_minted,
                },
            });
            ours_window_ns = 0;
            for (lane, taken) in mirrors.iter_mut().zip(mirror_probes) {
                // The size-accounting checkpoint, run OUTSIDE the
                // throughput window: truncating the WAL here is the
                // sampler's artifact (so `disk_bytes` reads the main
                // file honestly), not the workload's — SQLite's own
                // autocheckpoints already ride inside the apply
                // timings.
                lane.conn
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
                    .map_err(|e| format!("churn sample checkpoint ({}): {e}", lane.label))?;
                let disk_bytes = std::fs::metadata(&lane.path)
                    .map_err(|e| format!("churn sample stat ({}): {e}", lane.label))?
                    .len();
                lane.samples.push(SamplePoint {
                    cycle,
                    probes: taken,
                    // Maintenance included: this window already carries
                    // the VACUUM/ANALYZE spans `maintain` charged in.
                    commits_per_sec: commits_per_sec(cfg.sample_every, lane.window_ns),
                    maintenance_ns: lane.maintenance_ns,
                    disk_bytes,
                    counters: Counters::Sqlite {
                        freelist_count: pragma_u64(&lane.conn, "PRAGMA freelist_count")?,
                        page_count: pragma_u64(&lane.conn, "PRAGMA page_count")?,
                    },
                });
                lane.window_ns = 0;
                lane.maintenance_ns = 0;
            }
        }
    }
    let refs: Vec<(&str, &rusqlite::Connection)> = mirrors
        .iter()
        .map(|lane| (lane.label, &lane.conn))
        .collect();
    verify_end::assert_end_state(&ours, &refs, &live)?;
    assert_eq!(
        u64::try_from(live.len()).expect("fits u64"),
        working_set + spec.mix.growth * cfg.cycles,
        "the working-set law: live == working_set + growth x cycles"
    );
    let mut out = Vec::with_capacity(1 + mirrors.len());
    out.push(LaneSeries {
        lane: lanes::ours_label(spec.ours).to_owned(),
        engine: Engine::Bumbledb,
        samples: ours_samples,
    });
    for lane in mirrors {
        out.push(LaneSeries {
            lane: lane.label.to_owned(),
            engine: Engine::Sqlite,
            samples: lane.samples,
        });
    }
    Ok(RunSeries {
        name: spec.name.to_owned(),
        mix: spec.mix.clone(),
        working_set,
        lanes: out,
    })
}
