//! Executable core-side regime runners over the existing ledger corpus.
//!
//! These extend the existing measurement machinery (chapter 40: keep and
//! extend, no second benchmark framework):
//!
//! - **cold-open**: `Db::open` + first read, timed together — activation is
//!   part of the per-user cost, not warmed away;
//! - **warm**: repeated read over an open store (the warm read *families*
//!   with verified oracles stay in the existing `bench`/`scenarios` lanes;
//!   this cell is the regime scaffold that the merged report joins to them);
//! - **post-write**: a real mutation before every timed sample
//!   (delete-commit and insert-commit alternate so same-command
//!   normalization cannot cancel the delta), then the first read is timed —
//!   the PERF-001 first-read rebuild measurement;
//! - **large-result**: execution and owned delivery timed as separate
//!   segments (never summed into one number);
//! - **tenant-churn**: many small tenant stores, skewed activation, close
//!   after use, with file-descriptor high-water evidence that eviction
//!   actually releases resources.
//!
//! Selective keyed probes (APP-FAST's direct-probe leg) run through the
//! existing verified read families (`bench --families`); duplicating those
//! queries here would create a second unverified path.
//!
//! F1 note: no function here runs before F3. The engine surface used is the
//! preserved stable one (`Db::create/open/read/write/scan/insert_dyn/
//! delete_dyn/disk_size`); when P02/P03 land the successor owner/snapshot
//! API, this file follows in the same integration pass (recorded in the
//! packet as a tracked seam, not silently).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bumbledb::Db;

use crate::cli::AppPerfArgs;
use crate::corpus_gen::{GenConfig, relation_rows};
use crate::harness::{self, Modes, Protocol, Stats};
use crate::report;
use crate::schema::{Ledger, ids};
use crate::space::store_source;

use super::{CostAccount, PhaseSplit, Regime};

fn work() -> Result<bumbledb::WorkContext, String> {
    harness::bench_work()
}


#[derive(Debug, Clone)]
pub struct RegimeRow {
    pub regime: Regime,
    pub cell: String,
    pub stats: Stats,
    pub work: u64,
    pub phases: Option<PhaseSplit>,
    pub account: CostAccount,
}

/// Cold open: time `open + first scan + drop` per sample over an existing
/// corpus directory.
///
/// # Errors
pub fn cold_open(dir: &Path) -> Result<RegimeRow, String> {
    let m = harness::measure_batched(Protocol::COLD, Modes::default(), 1, || {
        let db = Db::open(dir, Ledger, work()?).map_err(|e| format!("cold open: {e:?}"))?;
        let read_work = work()?;
        let count = db
            .read(read_work.clone(), |snap| snap.count(ids::ACCOUNT))
            .map_err(|e| format!("cold first read: {e:?}"))?;
        drop(db);
        Ok(count)
    })?;
    Ok(RegimeRow {
        regime: Regime::ColdOpen,
        cell: "ledger/cold-open+first-read".to_owned(),
        stats: m.stats,
        work: m.work,
        phases: None,
        account: CostAccount {
            source_visits: Some(m.work),
            ..CostAccount::default()
        },
    })
}

/// Warm read regime scaffold: repeated full-account scan on an open store.
///
/// # Errors
pub fn warm_scan(db: &Db<Ledger>, samples: Option<u32>) -> Result<RegimeRow, String> {
    let proto = Protocol {
        warmups: Protocol::WARM.warmups,
        samples: samples.unwrap_or(Protocol::WARM.samples),
    };
    let m = harness::measure_batched(proto, Modes::default(), 1, || {
        db.read(work()?, |snap| snap.count(ids::ACCOUNT))
            .map_err(|e| format!("warm scan: {e:?}"))
    })?;
    Ok(RegimeRow {
        regime: Regime::Warm,
        cell: "ledger/warm-scan".to_owned(),
        stats: m.stats,
        work: m.work,
        phases: None,
        account: CostAccount {
            source_visits: Some(m.work),
            ..CostAccount::default()
        },
    })
}

/// Post-write first read (PERF-001 / APP-MUTATE): before every timed sample,
/// commit a real delta to `POSTING_TAG` (a leaf relation: no other law references its rows, so the delete admits). Deletion commits and reinsertion commits
/// alternate — two separate commands, so one-command normalization cannot
/// erase the mutation and the store returns to its loaded state every two
/// samples.
///
/// # Errors
/// # Panics
/// When the generated corpus is empty — a corpus contract violation.
pub fn post_write_first_read(
    db: &Db<Ledger>,
    cfg: GenConfig,
    samples: Option<u32>,
) -> Result<RegimeRow, String> {
    let victim = relation_rows(cfg, ids::POSTING_TAG)
        .next()
        .expect("the generated corpus has at least one posting tag");
    let mut present = true;
    let proto = Protocol {
        warmups: 4,
        samples: samples.unwrap_or(64),
    };
    let m = harness::measure_interleaved(
        proto,
        Modes::default(),
        1,
        || {
            // The untimed mutation before each timed first read.
            let row = victim.clone();
            let outcome = if present {
                db.write(work().expect("write work"), |tx| {
                    tx.delete_dyn(ids::POSTING_TAG, [row])?;
                    Ok(())
                })
            } else {
                db.write(work().expect("write work"), |tx| {
                    tx.insert_dyn(ids::POSTING_TAG, [row])?;
                    Ok(())
                })
            };
            outcome
                .expect("post-write mutation commits")
                .expect("accepted");
            present = !present;
        },
        || {
            db.read(work()?, |snap| snap.count(ids::POSTING_TAG))
                .map_err(|e| format!("first read after mutation: {e:?}"))
        },
    )?;
    Ok(RegimeRow {
        regime: Regime::PostWrite,
        cell: "ledger/first-read-after-delete-or-insert".to_owned(),
        stats: m.stats,
        work: m.work,
        phases: None,
        account: CostAccount {
            source_visits: Some(m.work),
            ..CostAccount::default()
        },
    })
}

/// Large-result delivery split: execute (materialize owned rows) and deliver
/// (walk owned pages) timed as separate segments. `end_to_end` is measured
/// around both; segments are attributed, never summed into the headline.
///
/// # Errors
pub fn large_result(db: &Db<Ledger>, samples: Option<u32>) -> Result<RegimeRow, String> {
    const PAGE_ROWS: usize = 1024;
    let count = samples.unwrap_or(16);
    let mut execute_ns = Vec::with_capacity(count as usize);
    let mut deliver_ns = Vec::with_capacity(count as usize);
    let mut end_ns = Vec::with_capacity(count as usize);
    let mut rows_delivered = 0u64;
    for _ in 0..count {
        let whole = Instant::now();
        let start = Instant::now();
        let owned: Vec<Vec<bumbledb::Value>> = db
            .read(work()?, |snap| {
                snap.scan(ids::POSTING)?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|e| format!("large-result execute: {e:?}"))?;
        execute_ns.push(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX));
        let start = Instant::now();
        for page in owned.chunks(PAGE_ROWS) {
            rows_delivered += std::hint::black_box(page.len()) as u64;
        }
        deliver_ns.push(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX));
        end_ns.push(u64::try_from(whole.elapsed().as_nanos()).unwrap_or(u64::MAX));
        drop(owned);
    }
    let execute = harness::stats(&mut execute_ns);
    let deliver = harness::stats(&mut deliver_ns);
    let stats = harness::stats(&mut end_ns);
    Ok(RegimeRow {
        regime: Regime::LargeResult,
        cell: "ledger/full-posting-materialize+page-walk".to_owned(),
        stats,
        work: rows_delivered,
        phases: Some(PhaseSplit {
            prepare_ns: None,
            execute_ns: Some(execute.p50),
            deliver_ns: Some(deliver.p50),
            end_to_end_ns: stats.p50,
        }),
        account: CostAccount {
            source_visits: Some(rows_delivered),
            ..CostAccount::default()
        },
    })
}

/// Count open file descriptors where the platform exposes them; `None`
/// elsewhere (a hole, never a zero).
#[must_use]
pub fn open_fd_count() -> Option<u64> {
    for dir in ["/proc/self/fd", "/dev/fd"] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            return Some(entries.count() as u64);
        }
    }
    None
}

/// Tenant churn: `tenants` small stores under `base`, a skewed activation
/// schedule (half the activations hit the two hottest tenants), each
/// activation = open + keyed-scale read + close. Reports activation latency
/// and file-descriptor high water against the baseline — eviction must
/// actually release.
///
/// # Errors
/// # Panics
pub fn tenant_churn(
    base: &Path,
    tenants: u32,
    activations: u32,
    seed: u64,
) -> Result<RegimeRow, String> {
    assert!(tenants >= 2, "churn needs at least two tenants");
    let cfg = GenConfig {
        seed,
        scale: crate::corpus_gen::Scale::Tiny,
    };
    let mut dirs = Vec::with_capacity(tenants as usize);
    for tenant in 0..tenants {
        let dir = base.join(format!("tenant-{tenant}"));
        let db = Db::create(&dir, Ledger, work()?)
            .map_err(|e| format!("tenant {tenant} create: {e:?}"))?
            .expect("accepted");
        crate::corpus::load_bumbledb(&db, cfg)
            .map_err(|e| format!("tenant {tenant} load: {e:?}"))?;
        drop(db);
        dirs.push(dir);
    }
    let fd_baseline = open_fd_count();
    let mut state = seed ^ 0x5445_4E41_4E54_5331; // "TENANTS1"
    let mut next = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let mut latencies = Vec::with_capacity(activations as usize);
    let mut rows_read = 0u64;
    let mut fd_high_water = fd_baseline.unwrap_or(0);
    for _ in 0..activations {
        // Skew: 50% of activations land on the two hottest tenants.
        let tenant = if next() % 2 == 0 {
            usize::try_from(next() % 2).expect("bounded")
        } else {
            usize::try_from(next() % u64::from(tenants)).expect("bounded")
        };
        let start = Instant::now();
        let db = Db::open(&dirs[tenant], Ledger, work()?)
            .map_err(|e| format!("tenant {tenant} open: {e:?}"))?;
        rows_read += db
            .read(work()?, |snap| snap.count(ids::ACCOUNT))
            .map_err(|e| format!("tenant {tenant} read: {e:?}"))?;
        drop(db);
        latencies.push(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX));
        if let Some(fds) = open_fd_count() {
            fd_high_water = fd_high_water.max(fds);
        }
    }
    let latency_stats = harness::stats(&mut latencies);
    let fd_after = open_fd_count();
    let leaked = match (fd_baseline, fd_after) {
        (Some(before), Some(after)) => Some(after.saturating_sub(before)),
        _ => None,
    };
    Ok(RegimeRow {
        regime: Regime::TenantChurn,
        cell: format!("ledger-tiny/{tenants}-tenants-{activations}-activations"),
        stats: latency_stats,
        work: rows_read,
        phases: None,
        account: CostAccount {
            live_resources: leaked,
            ..CostAccount::default()
        },
    })
}

fn push_row(out: &mut String, row: &RegimeRow) {
    let _ = write!(out, "{{\"regime\":\"{}\",\"cell\":", row.regime.label());
    crate::json::push_str_lit(out, &row.cell);
    let _ = write!(
        out,
        ",\"p50_ns\":{},\"p95_ns\":{},\"p99_ns\":{},\"min_ns\":{},\"max_ns\":{},\"mean_ns\":{},\"work\":{}",
        row.stats.p50,
        row.stats.p95,
        row.stats.p99,
        row.stats.min,
        row.stats.max,
        row.stats.mean_ns,
        row.work,
    );
    if let Some(phases) = &row.phases {
        let _ = write!(out, ",\"end_to_end_p50_ns\":{}", phases.end_to_end_ns);
        if let Some(ns) = phases.execute_ns {
            let _ = write!(out, ",\"execute_p50_ns\":{ns}");
        }
        if let Some(ns) = phases.deliver_ns {
            let _ = write!(out, ",\"deliver_p50_ns\":{ns}");
        }
    }
    if let Some(leaked) = row.account.live_resources {
        let _ = write!(out, ",\"fd_growth\":{leaked}");
    }
    if let Some(visits) = row.account.source_visits {
        let _ = write!(out, ",\"source_visits\":{visits}");
    }
    if let Some(map) = row.account.virtual_map_bytes {
        let _ = write!(out, ",\"virtual_map_bytes\":{map}");
    }
    if let Some(disk) = row.account.disk_bytes {
        let _ = write!(out, ",\"populated_file_bytes\":{disk}");
    }
    if let Some(alloc) = row.account.allocated_disk_bytes {
        let _ = write!(out, ",\"allocated_disk_bytes\":{alloc}");
    }
    if let Some(roster) = row.account.roster_entries {
        let _ = write!(out, ",\"roster_entries\":{roster}");
    }
    out.push('}');
}

/// The `app-perf` CLI lane: build the corpus once, run the requested regimes,
/// write artifacts. `--plan` prints the L21 input table and exits without
/// timing. Hosted/maintenance stay `not-run-here` with their owner.
///
/// # Errors
pub fn run(args: &AppPerfArgs) -> Result<i32, String> {
    if args.plan {
        print!("{}", super::plan::render());
        return Ok(0);
    }
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-app-perf",
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    let scratch = out_dir.join("scratch");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).map_err(|e| format!("scratch {}: {e}", scratch.display()))?;
    let cfg = GenConfig {
        seed: args.seed,
        scale: args.scale,
    };
    let corpus_dir = scratch.join("corpus");
    let db = Db::create(&corpus_dir, Ledger, work()?)
        .map_err(|e| format!("corpus create: {e:?}"))?
        .expect("accepted");
    crate::corpus::load_bumbledb(&db, cfg).map_err(|e| format!("corpus load: {e:?}"))?;
    let map_work = work()?;
    let map = db
        .integration_store()
        .map_report(&map_work)
        .map_err(|e| format!("map report: {e:?}"))?;
    let data = store_source::data_mdb(&corpus_dir);
    let allocated = crate::space::census::allocated_bytes(&data).ok();
    let roster_entries = db.schema().compiled_theory().ok().map(|theory| {
        (0..ids::RELATIONS)
            .map(|rel| {
                theory
                    .projections_of_relation(bumbledb::RelationId(rel))
                    .len()
            })
            .sum::<usize>() as u64
    });

    let wanted = |regime: &str| {
        args.regimes
            .as_ref()
            .is_none_or(|only| only.iter().any(|r| r == regime))
    };
    let mut rows = Vec::new();
    if wanted("warm") {
        rows.push(warm_scan(&db, args.samples)?);
    }
    if wanted("post-write") {
        rows.push(post_write_first_read(&db, cfg, args.samples)?);
    }
    if wanted("large-result") {
        rows.push(large_result(&db, args.samples)?);
    }
    drop(db);
    if wanted("cold-open") {
        rows.push(cold_open(&corpus_dir)?);
    }
    if wanted("tenant-churn") {
        rows.push(tenant_churn(
            &scratch.join("tenants"),
            args.tenants,
            args.tenants * 8,
            args.seed,
        )?);
    }
    if let Some(row) = rows.first_mut() {
        row.account.virtual_map_bytes = Some(map.virtual_map_bytes);
        row.account.disk_bytes = Some(map.populated_file_bytes);
        row.account.allocated_disk_bytes = allocated;
        row.account.roster_entries = roster_entries;
    }

    let mut out = String::new();
    out.push_str("{\"provenance\":");
    report::push_provenance(&mut out, &report::provenance(Path::new(".")));
    let _ = write!(out, ",\"seed\":{},\"rows\":[", args.seed);
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_row(&mut out, row);
    }
    // The regimes this lane cannot run and who owns them — recorded, not
    // silently absent.
    out.push_str(
        "],\"not_run_here\":[\
         {\"regime\":\"selective\",\"lane\":\"bench --families (verified keyed probes)\"},\
         {\"regime\":\"hosted-contention\",\"lane\":\"appperf::hosted driver over the successor log (F3)\"},\
         {\"regime\":\"maintenance\",\"lane\":\"log checkpoint/GC overlap lane (P05 harness, F3)\"}]}",
    );
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;
    std::fs::write(out_dir.join("app-perf.json"), &out).map_err(|e| format!("artifact: {e}"))?;
    let mut markdown = String::from(
        "# App-perf regimes\n\n| regime | cell | p50 ns | p99 ns | work |\n|---|---|---:|---:|---:|\n",
    );
    for row in &rows {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} |",
            row.regime.label(),
            row.cell,
            row.stats.p50,
            row.stats.p99,
            row.work
        );
    }
    std::fs::write(out_dir.join("app-perf.md"), &markdown).map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    let _ = std::fs::remove_dir_all(&scratch);
    Ok(0)
}
