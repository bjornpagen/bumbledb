use std::path::{Path, PathBuf};

use crate::cli::{BenchArgs, CorpusArgs};
use crate::corpus_gen::{self, GenConfig};
use crate::harness::Protocol;
use crate::schema::Ledger;
use crate::{clockproxy, families, report, sqlite_run, verify};

use super::corpus::gen_config;
use super::write_families::write_families;
use super::{BenchRun, CASES_FILE, CorpusPaths, ensure_corpus};

/// The stamp-refusal message, with the user's own flags substituted.
pub(super) fn stamp_refusal(corpus: &CorpusArgs) -> String {
    format!(
        "bench refuses: no fresh verify stamp for this corpus.\n\
         run first: bumbledb-bench verify --scale {} --seed {} --dir {}\n\
         (or pass --i-am-lying to run unverified — the report will say so)",
        corpus.scale.label(),
        corpus.seed,
        corpus.dir.display(),
    )
}

pub(crate) fn obs_missing(what: &str) -> String {
    format!(
        "{what} needs an obs build; run:\n\
         cargo run -p bumbledb-bench --features obs --release -- …"
    )
}

fn stamp_is_fresh(paths: &CorpusPaths, cfg: GenConfig) -> bool {
    let Ok(raw) = std::fs::read_to_string(paths.root.join(CASES_FILE)) else {
        return false;
    };
    let Ok(cases) = raw.trim().parse::<u32>() else {
        return false;
    };
    let vcfg = verify::VerifyConfig {
        corpus_gen: cfg,
        random_cases: cases,
        out_dir: paths.root.clone(),
    };
    verify::stamp_matches(&vcfg, &paths.stamp)
}

fn bench_preflight(args: &BenchArgs, cfg: GenConfig) -> Result<(CorpusPaths, bool), String> {
    if args.alloc && !cfg!(feature = "obs") {
        return Err(obs_missing("--alloc"));
    }

    if args.trace && !cfg!(feature = "obs") {
        return Err(obs_missing("--trace"));
    }
    if args.alloc && args.trace {
        return Err("--alloc and --trace are mutually exclusive modes".to_owned());
    }

    // write_families checks itself). Before ensure_corpus: refuse
    // before generating anything onto the ram disk. The verify/

    crate::devhonesty::assert_disk_backed(&args.corpus.dir, "the timed read families")
        .map_err(|refusal| refusal.to_string())?;
    let paths = ensure_corpus(&args.corpus.dir, cfg)?;
    let verified = stamp_is_fresh(&paths, cfg);
    if !verified && !args.i_am_lying {
        return Err(stamp_refusal(&args.corpus));
    }

    let all_names: Vec<&str> = families::all()
        .iter()
        .map(|f| f.name)
        .chain(crate::calendar::families::all().iter().map(|f| f.name))
        .chain(crate::closure::all().iter().map(|f| f.name))
        .chain(crate::displaced::all().iter().map(|f| f.name))
        .chain(families::write_families().iter().map(|f| f.name))
        .collect();
    if let Some(filter) = &args.families {
        for name in filter {
            if !all_names.contains(&name.as_str()) {
                return Err(format!(
                    "unknown family `{name}` (families: {})",
                    all_names.join(", ")
                ));
            }
        }
    }
    Ok((paths, verified))
}

/// # Errors
/// # Panics
/// Only on tool-invariant violations.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // the run is one linear protocol: reads, closure lane, writes, report
pub fn cmd_bench(args: &BenchArgs) -> Result<i32, String> {
    let cfg = gen_config(&args.corpus);
    let (paths, verified) = bench_preflight(args, cfg)?;
    let selected = |name: &str| {
        args.families
            .as_ref()
            .is_none_or(|filter| filter.iter().any(|f| f == name))
    };

    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(report::timestamp_iso8601().replace(':', "-"))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;

    // One durability point remains (ENG-008): the stamped corpus opens
    // durable, always. The retired `--nosync`/`--ephemeral` flags refuse in
    // the parser.
    let lane = crate::duralane::DurabilityLane::Durable;

    let mode = lane.store_mode();
    let db = mode.open(&paths.db, Ledger)?;
    let cal_db = mode.open(&paths.cal_db, crate::calendar::Scheduling)?;
    let conn =
        sqlite_run::open_for_bench(&paths.oracle).map_err(|e| format!("open oracle: {e}"))?;
    sqlite_run::FairnessCheck::run(&conn)?;
    let cal_conn = sqlite_run::open_for_bench(&paths.cal_oracle)
        .map_err(|e| format!("open calendar oracle: {e}"))?;
    sqlite_run::FairnessCheck::run_calendar(&cal_conn)?;

    eprintln!("bench: warming clocks (200 ms spin)");
    clockproxy::warm_up(std::time::Duration::from_millis(200));

    let proto = Protocol {
        warmups: Protocol::WARM.warmups,
        samples: args.samples.unwrap_or(Protocol::WARM.samples),
    };
    let mut run = BenchRun {
        cfg,
        proto,
        alloc: args.alloc,
        trace: args.trace,
        proxy_per_rep: args.proxy_per_rep,
        first_family_warmed: false,
        trace_dir: out_dir.join("trace"),
        db: &db,
        conn: &conn,
        cal_db: &cal_db,
        cal_conn: &cal_conn,
        flames: Vec::new(),
    };
    let mut reads = Vec::new();
    for family in families::all() {
        if selected(family.name) {
            reads.push(run.read_family(family)?);
        }
    }

    // calendar benchmark): same protocol, second store pair; the DU

    for family in crate::calendar::families::all() {
        if selected(family.name) {
            reads.push(run.read_cal_family(family)?);
        }
    }
    let mut flames = std::mem::take(&mut run.flames);
    drop(run);

    // under the same protocol — report-only rows beside the reads. It
    // runs after the stamped read families (its corpus load commits
    // fsync) and before the write families (it times reads).
    reads.extend(crate::closure::bench_families(
        cfg,
        &out_dir.join("scratch"),
        &selected,
        proto,
        args.alloc,
        args.proxy_per_rep,
        lane.store_mode(),
    )?);

    // with the mass as the row's parameter. After the closure lane
    // (same reads-before-writes law), before the write families.
    reads.extend(crate::displaced::bench_families(
        cfg,
        &out_dir.join("scratch"),
        &selected,
        args.samples,
        args.alloc,
        args.proxy_per_rep,
        lane.store_mode(),
    )?);

    // Write families run AFTER every read family (measured): an

    let trace_dir = args.trace.then(|| out_dir.join("trace"));
    let writes = write_families(
        cfg,
        &out_dir.join("scratch"),
        &selected,
        lane,
        trace_dir.as_deref(),
        &mut flames,
    )?;

    // The image cache is gone with the transitional store (owned snapshots
    // replaced images), so the store block is the two file sizes.
    let store = report::StoreNumbers {
        db_bytes: db.disk_size().map_err(|e| format!("{e:?}"))?,
        sqlite_bytes: std::fs::metadata(&paths.oracle).map_or(0, |m| m.len()),
    };

    let run_report = report::RunReport {
        provenance: report::provenance(Path::new(".")),
        config: report::RunConfig {
            scale: cfg.scale.label(),
            seed: cfg.seed,
            samples: proto.samples,
            store: lane.store_mode().label(),
        },
        corpus_digest: corpus_gen::digest_hex(&corpus_gen::corpus_digest(cfg)),
        verify_stamp: if verified {
            let stamp = std::fs::read_to_string(&paths.stamp)
                .map_or_else(|_| "UNVERIFIED".to_owned(), |s| s.trim().to_owned());
            let cases = std::fs::read_to_string(paths.root.join(CASES_FILE))
                .map_or_else(|_| "?".to_owned(), |s| s.trim().to_owned());
            format!("{stamp} (families + {cases} randomized cases)")
        } else {
            "UNVERIFIED".to_owned()
        },
        budget_gates: cfg.scale == corpus_gen::Scale::L,
        partial: args.families.is_some(),
        reads,
        writes,
        store,
        flames,
    };
    report::write_artifacts(&run_report, &out_dir).map_err(|e| format!("artifacts: {e}"))?;
    print!("{}", report::to_markdown(&run_report));
    println!("artifacts: {}", out_dir.display());

    let gates_ok = run_report.all_win() && (!run_report.budget_gates || run_report.budget_ok());
    Ok(i32::from(!gates_ok))
}
