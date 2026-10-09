use std::path::{Path, PathBuf};

use crate::cli::{BenchArgs, CorpusArgs};
use crate::harness::Protocol;
use crate::harness::{report, sqlite_run};
use crate::oracle::sqlite::verify;
use crate::worlds::corpus_gen::{self, GenConfig};
use crate::worlds::families;
use crate::worlds::ledger::Ledger;

use super::corpus::gen_config;
use super::write_families::write_families;
use super::{BenchRun, CASES_FILE, CorpusPaths, ensure_corpus};

/// The stamp-refusal message, with the user's own flags substituted.
pub(super) fn stamp_refusal(corpus: &CorpusArgs) -> String {
    format!(
        "no fresh verify stamp for this corpus.\n\
         run first: bumbledb-bench verify --scale {} --seed {} --dir {}",
        corpus.scale.label(),
        corpus.seed,
        corpus.dir.display(),
    )
}

pub(super) fn stamp_is_fresh(paths: &CorpusPaths, cfg: GenConfig) -> bool {
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
    let paths = ensure_corpus(&args.corpus.dir, cfg)?;
    let verified = stamp_is_fresh(&paths, cfg);
    if !verified && !args.i_am_lying {
        return Err(stamp_refusal(&args.corpus));
    }

    let all_names: Vec<&str> = families::all()
        .iter()
        .map(|f| f.name)
        .chain(
            crate::worlds::calendar::families::all()
                .iter()
                .map(|f| f.name),
        )
        .chain(crate::worlds::closure::all().iter().map(|f| f.name))
        .chain(crate::worlds::displaced::all().iter().map(|f| f.name))
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
// the run is one linear protocol: reads, closure lane, writes, report
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

    let db = crate::harness::open_db(&paths.db, Ledger)?;
    let cal_db = crate::harness::open_db(&paths.cal_db, crate::worlds::calendar::Scheduling)?;
    let conn =
        sqlite_run::open_for_bench(&paths.oracle).map_err(|e| format!("open oracle: {e}"))?;
    sqlite_run::FairnessCheck::run(&conn)?;
    let cal_conn = sqlite_run::open_for_bench(&paths.cal_oracle)
        .map_err(|e| format!("open calendar oracle: {e}"))?;
    sqlite_run::FairnessCheck::run_calendar(&cal_conn)?;

    let proto = Protocol {
        warmups: Protocol::WARM.warmups,
        samples: args.samples.unwrap_or(Protocol::WARM.samples),
    };
    let mut run = BenchRun {
        cfg,
        proto,
        read_batch: args.read_batch,
        first_family_warmed: false,
        db: &db,
        conn: &conn,
        cal_db: &cal_db,
        cal_conn: &cal_conn,
    };
    let mut reads = Vec::new();
    for family in families::all() {
        if selected(family.name) {
            reads.push(run.read_family(family)?);
        }
    }

    // Calendar families use the same protocol over their own store pair.
    for family in crate::worlds::calendar::families::all() {
        if selected(family.name) {
            reads.push(run.read_cal_family(family)?);
        }
    }
    // The closure and displaced worlds are report-only rows beside the reads.
    // Their corpus loads commit with fsync, so they run after the stamped
    // read families and before the write families.
    reads.extend(crate::worlds::closure::bench_families(
        cfg,
        &out_dir.join("scratch"),
        &selected,
        proto,
        args.read_batch,
    )?);
    reads.extend(crate::worlds::displaced::bench_families(
        cfg,
        &out_dir.join("scratch"),
        &selected,
        args.samples,
        args.read_batch,
    )?);

    // Fsync-heavy write families follow every read family.
    let writes = write_families(cfg, &out_dir.join("scratch"), &selected)?;

    // File sizes are disk measurements, not image-cache memory usage.
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
    };
    report::write_artifacts(&run_report, &out_dir).map_err(|e| format!("artifacts: {e}"))?;
    print!("{}", report::to_markdown(&run_report));
    println!("artifacts: {}", out_dir.display());

    let gates_ok = run_report.all_win() && (!run_report.budget_gates || run_report.budget_ok());
    Ok(i32::from(!gates_ok))
}
