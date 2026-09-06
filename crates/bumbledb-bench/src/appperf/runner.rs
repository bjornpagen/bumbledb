//! Executable core-side regime runners over the existing ledger corpus.
//!
//! These extend the existing measurement machinery (chapter 40: keep and
//! extend, no second benchmark framework):
//!
//! - **cold-open**: `Db::open` + first read, timed together — activation is
//!   part of the per-user cost, not warmed away;
//! - **warm**: prepared full-account projection over an open store;
//! - **post-write**: a real mutation before every timed sample
//!   (delete-commit and insert-commit alternate so same-command
//!   normalization cannot cancel the delta), then the first read is timed —
//!   the PERF-001 first-read rebuild measurement;
//! - **large-result**: prepared execution through `CompleteResult` and actual
//!   cursor-page delivery, with separate segments and one measured total;
//! - **tenant-churn**: many small tenant stores, skewed activation, close
//!   after use, with before/after file-descriptor counts (not a peak claim).
//!
//! Selective keyed probes (APP-FAST's direct-probe leg) run through the
//! existing verified read families (`bench --families`); duplicating those
//! queries here would create a second unverified path.
//!
//! Protocol 2 replaces the old metadata-count/vector-length scaffolds.
//! These are native API measurements, not TypeScript or hosted qualification.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bumbledb::{Answers, Db, PreparedQuery, RelationId};

use crate::cli::AppPerfArgs;
use crate::corpus_gen::{GenConfig, relation_rows};
use crate::harness::{self, Modes, Protocol, Stats};
use crate::report;
use crate::schema::{Ledger, ids};
use crate::space::store_source;

use super::{CostAccount, PhaseSplit, Regime};

fn work() -> bumbledb::WorkContext {
    harness::bench_work()
}

fn projection(relation: RelationId) -> bumbledb::Query {
    let fields = crate::schema::schema().relation(relation).fields();
    let vars: Vec<_> = (0..fields.len())
        .map(|index| bumbledb::VarId(u16::try_from(index).expect("sealed field count")))
        .collect();
    bumbledb::Query::single(bumbledb::Rule {
        finds: vars.iter().copied().map(bumbledb::FindTerm::Var).collect(),
        atoms: vec![bumbledb::Atom {
            source: bumbledb::AtomSource::Edb(relation),
            bindings: vars
                .iter()
                .map(|var| (bumbledb::FieldId(var.0), bumbledb::Term::Var(*var)))
                .collect(),
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn scan_oracle(
    db: &Db<Ledger>,
    relation: RelationId,
) -> Result<Vec<crate::compare::Answer>, String> {
    db.read(work(), |snap| {
        snap.scan(relation)?
            .map(|row| row.map(|row| crate::compare::from_fact(&row)))
            .collect()
    })
    .map_err(|e| format!("projection oracle scan: {e:?}"))
}

fn execute_projection(
    db: &Db<Ledger>,
    prepared: &mut PreparedQuery<Ledger>,
    answers: &mut Answers,
) -> Result<u64, String> {
    db.read(work(), |snap| {
        snap.execute(prepared, &[] as &[bumbledb::BindValue], answers)
    })
    .map_err(|e| format!("prepared projection: {e:?}"))?;
    Ok(std::hint::black_box(answers).len() as u64)
}

fn gate_projection(
    db: &Db<Ledger>,
    relation: RelationId,
    prepared: &mut PreparedQuery<Ledger>,
) -> Result<u64, String> {
    let expected = scan_oracle(db, relation)?;
    let mut answers = Answers::new();
    let count = execute_projection(db, prepared, &mut answers)?;
    let types: Vec<_> = db
        .schema()
        .relation(relation)
        .fields()
        .iter()
        .map(|field| field.value_type)
        .collect();
    crate::compare::multisets(crate::compare::from_answers(&answers, &types), expected)
        .map_err(|e| format!("projection disagrees with canonical scan: {e}"))?;
    Ok(count)
}

/// Consume the real result owner, including final framing and disposal.
fn deliver_result(
    result: bumbledb::CompleteResult,
    mut visit: impl FnMut(&Answers),
) -> Result<u64, String> {
    let expected = result.len();
    let mut cursor = result.into_cursor(1024);
    let mut delivered = 0;
    let mut terminal = false;
    while let Some(page) = cursor
        .next_page_with_work(&work(), 1 << 20)
        .map_err(|e| format!("cursor delivery: {e:?}"))?
    {
        if terminal {
            return Err("page after terminal result frame".to_owned());
        }
        terminal = page.terminal;
        delivered += page.rows.len() as u64;
        visit(&page.rows);
    }
    if !terminal || delivered != expected {
        return Err(format!(
            "incomplete cursor result: {delivered}/{expected}, terminal={terminal}"
        ));
    }
    Ok(delivered)
}

fn complete(
    db: &Db<Ledger>,
    prepared: &mut PreparedQuery<Ledger>,
) -> Result<bumbledb::CompleteResult, String> {
    db.read(work(), |snap| {
        snap.execute_complete(prepared, &[] as &[bumbledb::BindValue])
    })
    .map_err(|e| format!("complete projection: {e:?}"))
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

/// Cold open: time `open + prepare + first projection + drop` over an existing
/// corpus directory.
///
/// # Errors
pub fn cold_open(dir: &Path, samples: Option<u32>) -> Result<RegimeRow, String> {
    let query = projection(ids::ACCOUNT);
    let expected = {
        let db = Db::open(dir, Ledger, work()).map_err(|e| format!("oracle open: {e:?}"))?;
        let mut prepared = db
            .prepare(&query, work())
            .map_err(|e| format!("oracle prepare: {e:?}"))?;
        gate_projection(&db, ids::ACCOUNT, &mut prepared)?
    };
    let proto = Protocol {
        samples: samples.unwrap_or(Protocol::COLD.samples),
        ..Protocol::COLD
    };
    let m = harness::measure_batched(proto, Modes::default(), 1, || {
        let db = Db::open(dir, Ledger, work()).map_err(|e| format!("cold open: {e:?}"))?;
        let mut prepared = db
            .prepare(&query, work())
            .map_err(|e| format!("cold prepare: {e:?}"))?;
        let mut answers = Answers::new();
        let count = execute_projection(&db, &mut prepared, &mut answers)?;
        if count != expected {
            return Err("cold projection cardinality changed".to_owned());
        }
        drop(answers);
        drop(prepared);
        drop(db);
        Ok(count)
    })?;
    Ok(RegimeRow {
        regime: Regime::ColdOpen,
        cell: "ledger/cold-open+prepare+account-projection".to_owned(),
        stats: m.stats,
        work: m.work,
        phases: None,
        account: CostAccount::default(),
    })
}

/// Prepared full-account projection, gated against canonical-row scanning.
///
/// # Errors
pub fn warm_scan(db: &Db<Ledger>, samples: Option<u32>) -> Result<RegimeRow, String> {
    let mut prepared = db
        .prepare(&projection(ids::ACCOUNT), work())
        .map_err(|e| format!("warm prepare: {e:?}"))?;
    let expected = gate_projection(db, ids::ACCOUNT, &mut prepared)?;
    let mut answers = Answers::new();
    let proto = Protocol {
        warmups: Protocol::WARM.warmups,
        samples: samples.unwrap_or(Protocol::WARM.samples),
    };
    let m = harness::measure_batched(proto, Modes::default(), 1, || {
        let count = execute_projection(db, &mut prepared, &mut answers)?;
        if count != expected {
            return Err("warm projection cardinality changed".to_owned());
        }
        Ok(count)
    })?;
    Ok(RegimeRow {
        regime: Regime::Warm,
        cell: "ledger/warm-prepared-account-projection".to_owned(),
        stats: m.stats,
        work: m.work,
        phases: None,
        account: CostAccount::default(),
    })
}

/// Post-write first read (PERF-001 / APP-MUTATE): before every timed sample,
/// commit a real delta to `POSTING_TAG` (a leaf relation: no other law references its rows, so the delete admits). Deletion commits and reinsertion commits
/// alternate — two separate commands, so one-command normalization cannot
/// erase the mutation and the store returns to its loaded state every two
/// samples.
///
/// # Errors
pub fn post_write_first_read(
    db: &Db<Ledger>,
    cfg: GenConfig,
    samples: Option<u32>,
) -> Result<RegimeRow, String> {
    let victim = relation_rows(cfg, ids::POSTING_TAG)
        .next()
        .ok_or_else(|| "generated corpus has no posting tags".to_owned())?;
    let present = std::cell::Cell::new(true);
    let mut prepared = db
        .prepare(&projection(ids::POSTING_TAG), work())
        .map_err(|e| format!("post-write prepare: {e:?}"))?;
    let expected = gate_projection(db, ids::POSTING_TAG, &mut prepared)?;
    if expected == 0 {
        return Err("post-write corpus has no posting tags".to_owned());
    }
    let mut answers = Answers::new();
    let proto = Protocol {
        warmups: 4,
        samples: samples.unwrap_or(64),
    };
    let m = harness::measure_cold(
        proto,
        || {
            // The untimed mutation before each timed first read.
            let row = victim.clone();
            let outcome = if present.get() {
                db.write(work(), |tx| {
                    tx.delete_dyn(ids::POSTING_TAG, [row])
                        .map(bumbledb::MutationReport::changed)
                })
            } else {
                db.write(work(), |tx| {
                    tx.insert_dyn(ids::POSTING_TAG, [row])
                        .map(bumbledb::MutationReport::changed)
                })
            };
            let changed = match outcome.map_err(|e| format!("post-write mutation: {e:?}"))? {
                bumbledb::Admission::Accepted(committed) => committed.value,
                bumbledb::Admission::Rejected(violations) => {
                    return Err(format!("post-write mutation rejected: {violations:?}"));
                }
            };
            if changed != 1 {
                return Err(format!(
                    "post-write setup changed {changed} rows, expected one"
                ));
            }
            present.set(!present.get());
            Ok(())
        },
        || {
            let count = execute_projection(db, &mut prepared, &mut answers)?;
            if count != expected - u64::from(!present.get()) {
                return Err("post-write projection ignored the committed delta".to_owned());
            }
            Ok(count)
        },
    )?;
    // Odd sample counts leave the final measured state deleted. Restore it
    // outside timing so later regimes see the same canonical corpus.
    if !present.get() {
        let outcome = db
            .write(work(), |tx| {
                tx.insert_dyn(ids::POSTING_TAG, [victim])
                    .map(bumbledb::MutationReport::changed)
            })
            .map_err(|e| format!("post-write restore: {e:?}"))?;
        let changed = match outcome {
            bumbledb::Admission::Accepted(committed) => committed.value,
            bumbledb::Admission::Rejected(violations) => {
                return Err(format!("post-write restore rejected: {violations:?}"));
            }
        };
        if changed != 1 {
            return Err("post-write restore did not insert the deleted row".to_owned());
        }
    }
    gate_projection(db, ids::POSTING_TAG, &mut prepared)?;
    Ok(RegimeRow {
        regime: Regime::PostWrite,
        cell: "ledger/first-prepared-projection-after-delete-or-insert".to_owned(),
        stats: m.stats,
        work: m.work,
        phases: None,
        account: CostAccount::default(),
    })
}

/// Large-result delivery: execute and seal, then pull actual cursor pages
/// and visit every typed value. `end_to_end` is measured
/// around both; segments are attributed, never summed into the headline.
///
/// # Errors
pub fn large_result(db: &Db<Ledger>, samples: Option<u32>) -> Result<RegimeRow, String> {
    let count = samples.unwrap_or(16);
    let mut prepared = db
        .prepare(&projection(ids::POSTING), work())
        .map_err(|e| format!("large-result prepare: {e:?}"))?;
    let expected = {
        let oracle = scan_oracle(db, ids::POSTING)?;
        let types: Vec<_> = db
            .schema()
            .relation(ids::POSTING)
            .fields()
            .iter()
            .map(|field| field.value_type)
            .collect();
        let mut delivered = Vec::new();
        let count = deliver_result(complete(db, &mut prepared)?, |page| {
            delivered.extend(crate::compare::from_answers(page, &types));
        })?;
        crate::compare::multisets(delivered, oracle).map_err(|e| format!("cursor oracle: {e}"))?;
        count
    };
    let mut execute_ns = Vec::with_capacity(count as usize);
    let mut deliver_ns = Vec::with_capacity(count as usize);
    let mut end_ns = Vec::with_capacity(count as usize);
    let mut rows_delivered = 0u64;
    for _ in 0..count {
        let whole = Instant::now();
        let start = Instant::now();
        let owned = complete(db, &mut prepared)?;
        execute_ns.push(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX));
        let start = Instant::now();
        let delivered = deliver_result(owned, |page| {
            for answer in page.answers() {
                for column in 0..page.arity() {
                    std::hint::black_box(answer.get(column));
                }
            }
        })?;
        deliver_ns.push(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX));
        end_ns.push(u64::try_from(whole.elapsed().as_nanos()).unwrap_or(u64::MAX));
        if delivered != expected {
            return Err("large-result cardinality changed".to_owned());
        }
        rows_delivered += delivered;
    }
    let execute = harness::stats(&mut execute_ns);
    let deliver = harness::stats(&mut deliver_ns);
    let stats = harness::stats(&mut end_ns);
    Ok(RegimeRow {
        regime: Regime::LargeResult,
        cell: "ledger/posting-query+complete-result+cursor-values".to_owned(),
        stats,
        work: rows_delivered,
        phases: Some(PhaseSplit {
            prepare_ns: None,
            execute_ns: Some(execute.p50),
            deliver_ns: Some(deliver.p50),
            end_to_end_ns: stats.p50,
        }),
        account: CostAccount::default(),
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
/// activation = open + prepare + full-account projection + close. Reports
/// latency and before/after descriptor growth, not a sampled resource peak.
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
    let mut expected = Vec::with_capacity(tenants as usize);
    let query = projection(ids::ACCOUNT);
    for tenant in 0..tenants {
        let dir = base.join(format!("tenant-{tenant}"));
        let db = Db::create(&dir, Ledger, work())
            .map_err(|e| format!("tenant {tenant} create: {e:?}"))?
            .expect("accepted");
        crate::corpus::load_bumbledb(&db, cfg)
            .map_err(|e| format!("tenant {tenant} load: {e:?}"))?;
        let mut prepared = db
            .prepare(&query, work())
            .map_err(|e| format!("tenant prepare: {e:?}"))?;
        expected.push(gate_projection(&db, ids::ACCOUNT, &mut prepared)?);
        drop(prepared);
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
    for _ in 0..activations {
        // Skew: 50% of activations land on the two hottest tenants.
        let tenant = if next() % 2 == 0 {
            usize::try_from(next() % 2).expect("bounded")
        } else {
            usize::try_from(next() % u64::from(tenants)).expect("bounded")
        };
        let start = Instant::now();
        let db = Db::open(&dirs[tenant], Ledger, work())
            .map_err(|e| format!("tenant {tenant} open: {e:?}"))?;
        let mut prepared = db
            .prepare(&query, work())
            .map_err(|e| format!("tenant prepare: {e:?}"))?;
        let mut answers = Answers::new();
        let count = execute_projection(&db, &mut prepared, &mut answers)?;
        if count != expected[tenant] {
            return Err(format!("tenant {tenant} projection changed"));
        }
        rows_read += count;
        drop(answers);
        drop(prepared);
        drop(db);
        latencies.push(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX));
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
#[expect(
    clippy::too_many_lines,
    reason = "Keep the ordered execution and cleanup transitions together"
)]
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
    if let Some(parent) = out_dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("output parent: {e}"))?;
    }
    std::fs::create_dir(&out_dir)
        .map_err(|e| format!("app-perf needs a fresh output {}: {e}", out_dir.display()))?;
    let scratch = out_dir.join("scratch");
    std::fs::create_dir(&scratch).map_err(|e| format!("scratch {}: {e}", scratch.display()))?;
    let cfg = GenConfig {
        seed: args.seed,
        scale: args.scale,
    };
    let corpus_dir = scratch.join("corpus");
    let db = Db::create(&corpus_dir, Ledger, work())
        .map_err(|e| format!("corpus create: {e:?}"))?
        .expect("accepted");
    crate::corpus::load_bumbledb(&db, cfg).map_err(|e| format!("corpus load: {e:?}"))?;
    let map_work = work();
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
        rows.push(cold_open(&corpus_dir, args.samples)?);
    }
    if wanted("tenant-churn") {
        rows.push(tenant_churn(
            &scratch.join("tenants"),
            args.tenants,
            args.tenants * 8,
            args.seed,
        )?);
    }
    if let Some(row) = rows
        .iter_mut()
        .find(|row| row.regime != Regime::TenantChurn)
    {
        row.account.virtual_map_bytes = Some(map.virtual_map_bytes);
        row.account.disk_bytes = Some(map.populated_file_bytes);
        row.account.allocated_disk_bytes = allocated;
        row.account.roster_entries = roster_entries;
    }

    let mut out = String::new();
    out.push_str("{\"protocol\":2,\"scope\":\"native-prepared-and-paged\",\"provenance\":");
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
    std::fs::write(out_dir.join("app-perf.json"), &out).map_err(|e| format!("artifact: {e}"))?;
    let mut markdown = String::from(
        "# App-perf regimes\n\nProtocol 2: native prepared queries and real cursor delivery.\n\nNot comparable to the retired metadata-count/vector-length scaffolds.\n\n| regime | cell | p50 ns | p99 ns | output rows |\n|---|---|---:|---:|---:|\n",
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
