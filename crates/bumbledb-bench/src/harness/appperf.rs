//! The `app-perf` regime lane over the ledger corpus: cold open, warm
//! prepared reads, the first read after a committed write, large-result
//! cursor delivery, and tenant churn. Every regime is gated against a
//! canonical scan before it is timed.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bumbledb::{Answers, Db, PreparedQuery, RelationId};

use crate::cli::AppPerfArgs;
use crate::harness::report;
use crate::harness::{self, Protocol, Stats};
use crate::worlds::corpus_gen::{GenConfig, relation_rows};
use crate::worlds::ledger::{Ledger, ids};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    /// Warm prepared reuse.
    Warm,
    /// `Db::open` plus the first read, timed together.
    ColdOpen,
    /// The first read after a committed insert or delete.
    PostWrite,
    /// Execution and cursor delivery of a large result.
    LargeResult,
    /// Many small tenant stores opened and closed on a skewed schedule.
    TenantChurn,
}

impl Regime {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Warm => "warm",
            Self::ColdOpen => "cold-open",
            Self::PostWrite => "post-write",
            Self::LargeResult => "large-result",
            Self::TenantChurn => "tenant-churn",
        }
    }

    /// # Errors
    pub fn parse(label: &str) -> Result<Self, String> {
        REGIMES
            .into_iter()
            .find(|regime| regime.label() == label)
            .ok_or_else(|| {
                format!(
                    "unknown regime `{label}` (expected warm, cold-open, post-write, \
                     large-result, or tenant-churn)"
                )
            })
    }
}

pub const REGIMES: [Regime; 5] = [
    Regime::Warm,
    Regime::ColdOpen,
    Regime::PostWrite,
    Regime::LargeResult,
    Regime::TenantChurn,
];

/// Execution and delivery p50s of one large-result cell. `end_to_end_ns` is
/// measured around both and is never their sum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseSplit {
    pub execute_ns: u64,
    pub deliver_ns: u64,
    pub end_to_end_ns: u64,
}

fn work() -> bumbledb::WorkContext {
    harness::bench_work()
}

fn projection(relation: RelationId) -> bumbledb::Query {
    let fields = crate::worlds::ledger::schema().relation(relation).fields();
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
) -> Result<Vec<crate::oracle::compare::Answer>, String> {
    db.read(work(), |snap| {
        snap.scan(relation)?
            .map(|row| row.map(|row| crate::oracle::compare::from_fact(&row)))
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
    crate::oracle::compare::multisets(
        crate::oracle::compare::from_answers(&answers, &types),
        expected,
    )
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
        .next_page(&work())
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
    /// Descriptor growth across the churn schedule, where the platform
    /// exposes open descriptors.
    pub fd_growth: Option<u64>,
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
    let m = harness::measure_batched(proto, 1, || {
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
        fd_growth: None,
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
    let m = harness::measure_batched(proto, 1, || {
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
        fd_growth: None,
    })
}

/// Post-write first read: before every timed sample, commit a real delta to
/// `POSTING_TAG` (a leaf relation: no other law references its rows, so the
/// delete admits). Deletion commits and reinsertion commits
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
            let changed = crate::harness::committed("post-write mutation", outcome)?;
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
        let changed = crate::harness::committed(
            "post-write restore",
            db.write(work(), |tx| {
                tx.insert_dyn(ids::POSTING_TAG, [victim])
                    .map(bumbledb::MutationReport::changed)
            }),
        )?;
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
        fd_growth: None,
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
            delivered.extend(crate::oracle::compare::from_answers(page, &types));
        })?;
        crate::oracle::compare::multisets(delivered, oracle)
            .map_err(|e| format!("cursor oracle: {e}"))?;
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
            execute_ns: execute.p50,
            deliver_ns: deliver.p50,
            end_to_end_ns: stats.p50,
        }),
        fd_growth: None,
    })
}

/// Open file descriptors, where the platform exposes them.
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
        scale: crate::worlds::corpus_gen::Scale::Tiny,
    };
    let mut dirs = Vec::with_capacity(tenants as usize);
    let mut expected = Vec::with_capacity(tenants as usize);
    let query = projection(ids::ACCOUNT);
    for tenant in 0..tenants {
        let dir = base.join(format!("tenant-{tenant}.bdb"));
        let db = harness::create_db(&dir, Ledger)?;
        crate::worlds::corpus::load_bumbledb(&db, cfg)
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
        fd_growth: leaked,
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
        let _ = write!(
            out,
            ",\"end_to_end_p50_ns\":{},\"execute_p50_ns\":{},\"deliver_p50_ns\":{}",
            phases.end_to_end_ns, phases.execute_ns, phases.deliver_ns
        );
    }
    if let Some(growth) = row.fd_growth {
        let _ = write!(out, ",\"fd_growth\":{growth}");
    }
    out.push('}');
}

/// The `app-perf` CLI lane: build the corpus once, run the requested regimes,
/// write `app-perf.json` and `app-perf.md` into a fresh output directory.
///
/// # Errors
pub fn run(args: &AppPerfArgs) -> Result<i32, String> {
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
    let corpus_dir = scratch.join("corpus.bdb");
    let db = harness::create_db(&corpus_dir, Ledger)?;
    crate::worlds::corpus::load_bumbledb(&db, cfg).map_err(|e| format!("corpus load: {e:?}"))?;
    let data = corpus_dir.join("data.mdb");
    let meta = std::fs::metadata(&data).map_err(|e| format!("stat {}: {e}", data.display()))?;
    let file_bytes = meta.len();
    let allocated_bytes = std::os::unix::fs::MetadataExt::blocks(&meta) * 512;

    let wanted = |regime: Regime| {
        args.regimes
            .as_ref()
            .is_none_or(|only| only.contains(&regime))
    };
    let mut rows = Vec::new();
    if wanted(Regime::Warm) {
        rows.push(warm_scan(&db, args.samples)?);
    }
    if wanted(Regime::PostWrite) {
        rows.push(post_write_first_read(&db, cfg, args.samples)?);
    }
    if wanted(Regime::LargeResult) {
        rows.push(large_result(&db, args.samples)?);
    }
    drop(db);
    if wanted(Regime::ColdOpen) {
        rows.push(cold_open(&corpus_dir, args.samples)?);
    }
    if wanted(Regime::TenantChurn) {
        rows.push(tenant_churn(
            &scratch.join("tenants"),
            args.tenants,
            args.tenants * 8,
            args.seed,
        )?);
    }

    let mut out = String::new();
    out.push_str("{\"provenance\":");
    report::push_provenance(&mut out, &report::provenance(Path::new(".")));
    let _ = write!(
        out,
        ",\"seed\":{},\"store\":{{\"file_bytes\":{file_bytes},\"allocated_bytes\":{allocated_bytes}}},\"rows\":[",
        args.seed
    );
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_row(&mut out, row);
    }
    out.push_str("]}");
    std::fs::write(out_dir.join("app-perf.json"), &out).map_err(|e| format!("artifact: {e}"))?;
    let mut markdown = String::from(
        "# App-perf regimes\n\n| regime | cell | p50 ns | p99 ns | output rows |\n|---|---|---:|---:|---:|\n",
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

#[cfg(test)]
mod tests;
