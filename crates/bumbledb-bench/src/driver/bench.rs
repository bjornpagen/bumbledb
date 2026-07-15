use std::path::{Path, PathBuf};

use bumbledb::Db;

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

/// The feature-missing message: the exact cargo invocation to use.
pub(super) fn obs_missing(what: &str) -> String {
    format!(
        "{what} needs an obs build; run:\n\
         cargo run -p bumbledb-bench --features obs --release -- …"
    )
}

/// Whether the digest directory carries a stamp matching this corpus at
/// the case count recorded beside it.
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
    if args.alloc && args.trace {
        return Err("--alloc and --trace are mutually exclusive modes".to_owned());
    }
    // The device-honesty rule is symmetric (docs/architecture/
    // 60-validation.md): EVERY timed lane refuses a RAM-backed target.
    // The read families time against the corpus under --dir, so the
    // corpus dir is checked exactly like the write scratch (which
    // write_families checks itself). Before ensure_corpus: refuse
    // before generating anything onto the ram disk. The verify/
    // differential/fuzz lanes stay exempt — they check answers, not
    // wall clocks.
    crate::devhonesty::assert_disk_backed(&args.corpus.dir, "the timed read families")
        .map_err(|refusal| refusal.to_string())?;
    let paths = ensure_corpus(&args.corpus.dir, cfg)?;
    let verified = stamp_is_fresh(&paths, cfg);
    if !verified && !args.i_am_lying {
        return Err(stamp_refusal(&args.corpus));
    }
    // Family selection: filtering never bypasses gate semantics — a
    // filtered run's verdict is PARTIAL.
    let all_names: Vec<&str> = families::all()
        .iter()
        .map(|f| f.name)
        .chain(crate::calendar::families::all().iter().map(|f| f.name))
        .chain(crate::closure::all().iter().map(|f| f.name))
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

/// `bench`. Returns the exit code: 0 when every selected gate family
/// won (and the budget held where it gates), 1 otherwise.
///
/// # Errors
///
/// Refusals (stamp, feature, unknown family) and setup errors — each
/// message names the next action.
///
/// # Panics
///
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

    let db = Db::open(&paths.db, Ledger).map_err(|e| format!("open db: {e:?}"))?;
    let conn =
        sqlite_run::open_for_bench(&paths.oracle).map_err(|e| format!("open oracle: {e}"))?;
    sqlite_run::FairnessCheck::run(&conn)?;
    let cal_db = Db::open(&paths.cal_db, crate::calendar::Scheduling)
        .map_err(|e| format!("open calendar db: {e:?}"))?;
    let cal_conn = sqlite_run::open_for_bench(&paths.cal_oracle)
        .map_err(|e| format!("open calendar oracle: {e}"))?;
    sqlite_run::FairnessCheck::run_calendar(&cal_conn)?;

    // The DVFS ramp eater (measured): ≥ 200 ms of warm work before
    // the first family, so opening samples measure a settled clock.
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
    // The calendar family set (docs/architecture/60-validation.md § the
    // calendar benchmark): same protocol, second store pair; the DU
    // whole-read exercises the spanning multi-rule union.
    for family in crate::calendar::families::all() {
        if selected(family.name) {
            reads.push(run.read_cal_family(family)?);
        }
    }
    let flames = std::mem::take(&mut run.flames);
    drop(run);

    // The closure lane (the roster extension): its own scratch world,
    // verified inline (the recursion surface is translator-
    // inexpressible, so it sits outside the stamped registry), timed
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
    )?);

    // Write families run AFTER every read family (measured): an
    // fsync drops the core to its DVFS floor with
    // demand-driven recovery, so any read family measured in that
    // shadow reads slow-clock time. `bulk` (seconds of fsync) is last
    // of all — asserted inside write_families.
    let writes = write_families(cfg, &out_dir.join("scratch"), &selected)?;

    // Cache residency needs the engine's trace feature (the obs build).
    #[cfg(feature = "obs")]
    let (cache_images, cache_bytes) = db.cache_resident();
    #[cfg(not(feature = "obs"))]
    let (cache_images, cache_bytes) = (0, 0);
    let store = report::StoreNumbers {
        db_bytes: db.disk_size().map_err(|e| format!("{e:?}"))?,
        sqlite_bytes: std::fs::metadata(&paths.oracle).map_or(0, |m| m.len()),
        cache_images,
        cache_bytes,
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
            // The provenance shows how much evidence earned the stamp:
            // a --cases 0 run is legal (families-only verification is
            // honest) but the report visibly says '0 randomized cases'.
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
