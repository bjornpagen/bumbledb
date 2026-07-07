//! Command orchestration (docs/architecture/50-validation.md): the digest-keyed corpus
//! cache, verify-before-time enforcement, and the bench run that turns
//! measurements into PRD 18 artifacts. Every failure message names the
//! next action.

use std::path::{Path, PathBuf};

use bumbledb::{Db, ResultBuffer};
use rusqlite::Connection;

use crate::cli::{BenchArgs, CorpusArgs};
use crate::gen::{self, GenConfig};
use crate::harness::{self, Modes, Protocol, Rotation};
use crate::schema::schema;
use crate::translate::translate;
use crate::{clockproxy, corpus, families, json, report, sqlite_run, trace_out, verify, writebench};

/// The digest-keyed corpus locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusPaths {
    /// `<dir>/<digest-prefix>/` — everything lives inside.
    pub root: PathBuf,
    pub db: PathBuf,
    pub oracle: PathBuf,
    pub stamp: PathBuf,
}

fn gen_config(corpus: &CorpusArgs) -> GenConfig {
    GenConfig {
        seed: corpus.seed,
        scale: corpus.scale,
    }
}

/// Resolves the digest-keyed directory for a corpus config (the digest
/// is the corpus identity — PRD 07).
#[must_use]
pub fn corpus_paths(dir: &Path, cfg: GenConfig) -> CorpusPaths {
    let digest = gen::digest_hex(&gen::corpus_digest(cfg));
    let root = dir.join(&digest[..16]);
    CorpusPaths {
        db: root.join("db"),
        oracle: root.join("oracle.sqlite"),
        stamp: root.join("verify.stamp"),
        root,
    }
}

const CORPUS_MARKER: &str = "corpus.ok";

/// [`ensure_corpus`] with an injectable loader — the reuse-logic test
/// seam (a counter hook proves the marker short-circuits regeneration).
///
/// # Errors
///
/// The loader's error; scratch I/O as a message.
pub fn ensure_corpus_with(
    dir: &Path,
    cfg: GenConfig,
    load: &mut dyn FnMut(&CorpusPaths) -> Result<(), String>,
) -> Result<CorpusPaths, String> {
    let paths = corpus_paths(dir, cfg);
    if paths.root.join(CORPUS_MARKER).exists() {
        return Ok(paths);
    }
    let _ = std::fs::remove_dir_all(&paths.root);
    std::fs::create_dir_all(&paths.root)
        .map_err(|e| format!("create {}: {e}", paths.root.display()))?;
    load(&paths)?;
    std::fs::write(paths.root.join(CORPUS_MARKER), "ok").map_err(|e| format!("marker: {e}"))?;
    Ok(paths)
}

/// Generates + loads both stores into the digest-keyed directory,
/// reusing an existing one carrying the `corpus.ok` marker (regeneration
/// is identity; the cache is convenience for L).
///
/// # Errors
///
/// Load errors as messages.
pub fn ensure_corpus(dir: &Path, cfg: GenConfig) -> Result<CorpusPaths, String> {
    ensure_corpus_with(dir, cfg, &mut |paths| {
        eprintln!(
            "gen: loading corpus (seed {}, scale {}) into {}",
            cfg.seed,
            cfg.scale.label(),
            paths.root.display()
        );
        // Load into a scratch sibling, then compact into place
        // (docs/architecture/40-storage.md): a bulk load is exactly the CoW-churn-heavy
        // case — ~40% of the loaded file is freelist — and the cached
        // corpus is write-once, so it ships live-sized.
        let load_dir = paths.root.join("db-load");
        let db = Db::create(&load_dir, schema()).map_err(|e| format!("create db: {e:?}"))?;
        corpus::load_bumbledb(&db, cfg).map_err(|e| format!("load bumbledb: {e:?}"))?;
        db.compact(&paths.db)
            .map_err(|e| format!("compact: {e:?}"))?;
        drop(db);
        std::fs::remove_dir_all(&load_dir).map_err(|e| format!("remove db-load: {e}"))?;
        corpus::load_sqlite(&paths.oracle, cfg).map_err(|e| format!("load sqlite: {e}"))?;
        Ok(())
    })
}

/// `gen`.
///
/// # Errors
///
/// As [`ensure_corpus`].
pub fn cmd_gen(corpus: &CorpusArgs) -> Result<(), String> {
    let paths = ensure_corpus(&corpus.dir, gen_config(corpus))?;
    println!("corpus ready: {}", paths.root.display());
    Ok(())
}

/// The sidecar recording which case count the stamp was earned with —
/// bench reconstructs the full `VerifyConfig` from it.
const CASES_FILE: &str = "verify.cases";

/// `verify`: the oracle against the digest directory, stamp inside it.
/// Returns the process exit code (1 on mismatch).
///
/// # Errors
///
/// Setup errors as messages (mismatches are an exit code, not an error —
/// the bundles are the artifact).
pub fn cmd_verify(corpus: &CorpusArgs, cases: u32) -> Result<i32, String> {
    let cfg = gen_config(corpus);
    let paths = ensure_corpus(&corpus.dir, cfg)?;
    let db = Db::open(&paths.db, schema()).map_err(|e| format!("open db: {e:?}"))?;
    let conn = Connection::open(&paths.oracle).map_err(|e| format!("open oracle: {e}"))?;
    corpus::configure_sqlite(&conn).map_err(|e| format!("configure oracle: {e}"))?;
    let vcfg = verify::VerifyConfig {
        gen: cfg,
        random_cases: cases,
        out_dir: paths.root.clone(),
    };
    match verify::run_prepared(&vcfg, &db, &conn, |_| None) {
        Ok(report) => {
            std::fs::write(paths.root.join(CASES_FILE), cases.to_string())
                .map_err(|e| format!("cases sidecar: {e}"))?;
            println!("verify OK: {} cases, stamp {}", report.cases, report.stamp);
            Ok(0)
        }
        Err(failure) => {
            eprint!("{failure}");
            Ok(1)
        }
    }
}

/// `scenarios`: the non-ledger worlds — load, oracle-gate, time, and
/// write the markdown artifact. Report-class: always exit 0 unless a
/// gate (engine disagreement) or setup fails.
///
/// # Errors
///
/// Setup failures and oracle disagreements, as messages.
pub fn cmd_scenarios(args: &crate::cli::ScenarioArgs) -> Result<i32, String> {
    let proto = Protocol {
        warmups: 8,
        samples: args.samples.unwrap_or(64),
    };
    let (markdown, _) = crate::scenarios::run(&args.dir, args.seed, proto, args.only.as_deref())?;
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-scenarios",
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;
    std::fs::write(out_dir.join("scenarios.md"), &markdown)
        .map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    Ok(0)
}

/// The stamp-refusal message, with the user's own flags substituted.
fn stamp_refusal(corpus: &CorpusArgs) -> String {
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
fn obs_missing(what: &str) -> String {
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
        gen: cfg,
        random_cases: cases,
        out_dir: paths.root.clone(),
    };
    verify::stamp_matches(&vcfg, &paths.stamp)
}

fn exec_digest(stats: &bumbledb::ExecutionStats) -> report::ExecDigest {
    use std::fmt::Write as _;
    let mut worst = 1.0_f64;
    let mut covers = String::new();
    for (index, node) in stats.nodes.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let (estimate, actual) = (node.estimate.max(1) as f64, node.actual.max(1) as f64);
        worst = worst.max((estimate / actual).max(actual / estimate));
        if index > 0 {
            covers.push(' ');
        }
        let _ = write!(covers, "n{index}:");
        for (position, cover) in node.covers.iter().enumerate() {
            if position > 0 {
                covers.push('/');
            }
            let _ = write!(
                covers,
                "s{}x{}",
                cover.subatom,
                cover.chosen_exact + cover.chosen_estimate
            );
        }
    }
    report::ExecDigest {
        worst_estimate_factor: worst,
        covers,
        emits: stats.emits,
    }
}

#[cfg(feature = "obs")]
fn alloc_report(
    snapshot: Option<bumbledb::alloc_counter::AllocSnapshot>,
) -> Option<report::AllocReport> {
    snapshot.map(|s| report::AllocReport {
        allocs: s.allocs,
        deallocs: s.deallocs,
        alloc_bytes: s.alloc_bytes,
        dealloc_bytes: s.dealloc_bytes,
    })
}

/// The per-run context the bench families share.
struct BenchRun<'a> {
    cfg: GenConfig,
    proto: Protocol,
    alloc: bool,
    trace: bool,
    trace_dir: PathBuf,
    db: &'a Db<'a>,
    conn: &'a Connection,
    flames: Vec<report::FlameEmbed>,
}

/// The stamp merge for a family whose ours/theirs blocks were guarded
/// as one bracket pair each: the reported bracket is the WORST of the
/// two (contamination of either engine's block dirties the ratio).
fn merge_stamps(ours: clockproxy::GhzStamp, theirs: clockproxy::GhzStamp) -> report::GhzReport {
    report::GhzReport {
        pre: ours.pre.min(theirs.pre),
        post: ours.post.min(theirs.post),
        retried: ours.retried || theirs.retried,
        contaminated: ours.contaminated() || theirs.contaminated(),
    }
}

impl BenchRun<'_> {
    /// One read family on both engines.
    fn read_family(
        &mut self,
        family: &families::Family,
    ) -> Result<report::ReadFamilyReport, String> {
        eprintln!("bench: read family {}", family.name);
        let query = (family.query)();
        let mut prepared = self
            .db
            .prepare(&query)
            .map_err(|e| format!("{}: prepare: {e:?}", family.name))?;
        let sets = (family.params)(&self.cfg);
        let types: Vec<bumbledb::schema::ValueType> = prepared.column_types().cloned().collect();

        let mut rotation = Rotation::new(sets.clone());
        let mut buffer = ResultBuffer::new();
        let db = self.db;
        let mut run_ours = move |prepared: &mut bumbledb::PreparedQuery<'_>| {
            let params = rotation.next_set().to_vec();
            db.read(|snap| snap.execute(prepared, &params, &mut buffer))
                .map_err(|e| format!("execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        };
        let modes = Modes {
            alloc_window: self.alloc,
            trace: false,
        };
        let proto = self.proto;
        let (ours, ghz_ours) = clockproxy::guarded(|| {
            harness::measure_with(proto, modes, || run_ours(&mut prepared))
        })?;
        // The quantum guard: a gated p50 below 12 timer ticks would be
        // quantization, not measurement — batch executes and divide.
        let batch = if ours.stats.p50 < harness::QUANTUM_FLOOR_NS {
            16
        } else {
            1
        };
        let (ours, ghz_ours) = if batch > 1 {
            eprintln!(
                "bench: {} p50 under the {} ns quantum floor — re-measuring at batch {batch}",
                family.name,
                harness::QUANTUM_FLOOR_NS
            );
            clockproxy::guarded(|| {
                harness::measure_batched(proto, modes, batch, || run_ours(&mut prepared))
            })?
        } else {
            (ours, ghz_ours)
        };
        if self.trace {
            let (_, events) = harness::traced_sample(&mut || run_ours(&mut prepared))?;
            let (engine, harness_events) = trace_out::split_harness(events);
            trace_out::write_trace_file(
                &self.trace_dir,
                &format!("{}.warm", family.name),
                &engine,
                &harness_events,
            )
            .map_err(|e| format!("trace: {e}"))?;
            let mut table = trace_out::FlameSummary::compute(&engine).render_top(10);
            if let Some(phases) = trace_out::render_phase_table(&engine) {
                table.push('\n');
                table.push_str(&phases);
            }
            self.flames.push(report::FlameEmbed {
                name: family.name.to_owned(),
                table,
            });
        }
        let (_, stats) = self
            .db
            .read(|snap| snap.profile(&mut prepared, &sets[0]))
            .map_err(|e| format!("profile: {e:?}"))?;

        let translated = translate(&query, schema()).map_err(|e| format!("translate: {e}"))?;
        let mut sqlite_family = sqlite_run::PreparedFamily::new(self.conn, &translated, types)?;
        let mut rotation = Rotation::new(sets);
        let (theirs, ghz_theirs) = clockproxy::guarded(|| {
            harness::measure_batched(proto, Modes::default(), batch, || {
                sqlite_run::sample(&mut sqlite_family, rotation.next_set())
            })
        })?;

        #[allow(clippy::cast_precision_loss)]
        let ratio_p50 = ours.stats.p50 as f64 / theirs.stats.p50.max(1) as f64;
        #[cfg(feature = "obs")]
        let alloc = alloc_report(ours.alloc);
        #[cfg(not(feature = "obs"))]
        let alloc = None;
        Ok(report::ReadFamilyReport {
            name: family.name.to_owned(),
            verdict: report::verdict(family.kind, ours.stats.p50, theirs.stats.p50),
            p99_within_budget: report::within_budget(ours.stats.p99),
            ours: ours.stats,
            theirs: theirs.stats,
            ratio_p50,
            alloc,
            exec: Some(exec_digest(&stats)),
            ghz: Some(merge_stamps(ghz_ours, ghz_theirs)),
        })
    }
}

#[allow(clippy::cast_precision_loss)]
fn facts_per_sec(m: &harness::Measurement, samples: u32) -> f64 {
    let total_secs = (m.stats.mean_ns * u64::from(samples)) as f64 / 1e9;
    m.work as f64 / total_secs.max(f64::EPSILON)
}

/// The write/cold families, run against a scratch corpus loaded under
/// `scratch` — bench never mutates the verified digest-dir corpus, so
/// the stamp stays honest.
fn write_families(
    cfg: GenConfig,
    scratch: &Path,
    selected: &dyn Fn(&str) -> bool,
) -> Result<Vec<report::WriteFamilyReport>, String> {
    let mut out = Vec::new();
    let commit_selected = selected("commit_single") || selected("commit_batch");
    let cold_selected = selected("cold_fk_walk");

    if commit_selected || cold_selected {
        eprintln!("bench: loading the scratch write corpus");
        let db = Db::create(&scratch.join("db"), schema()).map_err(|e| format!("{e:?}"))?;
        corpus::load_bumbledb(&db, cfg).map_err(|e| format!("{e:?}"))?;
        let (conn, _) =
            corpus::load_sqlite(&scratch.join("oracle.sqlite"), cfg).map_err(|e| format!("{e}"))?;
        if selected("commit_single") {
            eprintln!("bench: commit_single");
            let ((ours, theirs), ghz) = clockproxy::stamped(|| {
                Ok((
                    writebench::commit_single_bumbledb(&db, cfg)?,
                    sqlite_run::commit_single(&conn, cfg)?,
                ))
            })?;
            out.push(report::WriteFamilyReport {
                name: "commit_single".to_owned(),
                ours: ours.stats,
                theirs: Some(theirs.stats),
                facts_per_sec: None,
                ghz: Some(write_ghz(ghz)),
            });
        }
        if selected("commit_batch") {
            eprintln!("bench: commit_batch");
            let ((ours, theirs), ghz) = clockproxy::stamped(|| {
                Ok((
                    writebench::commit_batch_bumbledb(&db, cfg)?,
                    sqlite_run::commit_batch(&conn, cfg)?,
                ))
            })?;
            out.push(report::WriteFamilyReport {
                name: "commit_batch".to_owned(),
                ours: ours.stats,
                theirs: Some(theirs.stats),
                facts_per_sec: None,
                ghz: Some(write_ghz(ghz)),
            });
        }
        if cold_selected {
            eprintln!("bench: cold_fk_walk");
            let ((ours, theirs), ghz) = clockproxy::stamped(|| {
                Ok((
                    writebench::cold_fk_walk(&db, cfg)?,
                    sqlite_run::cold_fk_walk(&conn, cfg)?,
                ))
            })?;
            out.push(report::WriteFamilyReport {
                name: "cold_fk_walk".to_owned(),
                ours: ours.stats,
                theirs: Some(theirs.stats),
                facts_per_sec: None,
                ghz: Some(write_ghz(ghz)),
            });
        }
    }
    if selected("bulk") {
        eprintln!("bench: bulk");
        let proto = families::write_families()
            .iter()
            .find(|f| f.name == "bulk")
            .expect("registered")
            .protocol;
        let ((ours, theirs), ghz) = clockproxy::stamped(|| {
            Ok((
                writebench::bulk_bumbledb(cfg, scratch)?,
                sqlite_run::bulk(cfg, scratch)?,
            ))
        })?;
        out.push(report::WriteFamilyReport {
            name: "bulk".to_owned(),
            facts_per_sec: Some(facts_per_sec(&ours, proto.samples)),
            ours: ours.stats,
            theirs: Some(theirs.stats),
            ghz: Some(write_ghz(ghz)),
        });
    }
    Ok(out)
}

/// A write family's block is guarded as one bracket over both engines.
fn write_ghz(stamp: clockproxy::GhzStamp) -> report::GhzReport {
    report::GhzReport {
        pre: stamp.pre,
        post: stamp.post,
        retried: stamp.retried,
        contaminated: stamp.contaminated(),
    }
}

/// `merge`: N run directories' `report.json` → the min-of-runs table on
/// stdout (docs/silicon/00-baseline-and-harness.md).
///
/// # Errors
///
/// Unreadable or unparseable report files, named.
pub fn cmd_merge(dirs: &[PathBuf]) -> Result<i32, String> {
    let runs: Vec<(String, json::Value)> = dirs
        .iter()
        .map(|dir| {
            let path = dir.join("report.json");
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("read {}: {e}", path.display()))?;
            let parsed = json::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            let label = dir
                .file_name()
                .map_or_else(|| dir.display().to_string(), |n| n.to_string_lossy().into_owned());
            Ok((label, parsed))
        })
        .collect::<Result<_, String>>()?;
    print!("{}", report::merge_markdown(&runs)?);
    Ok(0)
}

fn bench_preflight(args: &BenchArgs, cfg: GenConfig) -> Result<(CorpusPaths, bool), String> {
    if args.alloc && !cfg!(feature = "obs") {
        return Err(obs_missing("--alloc"));
    }
    if args.alloc && args.trace {
        return Err("--alloc and --trace are mutually exclusive modes".to_owned());
    }
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

    let db = Db::open(&paths.db, schema()).map_err(|e| format!("open db: {e:?}"))?;
    let conn =
        sqlite_run::open_for_bench(&paths.oracle).map_err(|e| format!("open oracle: {e}"))?;
    sqlite_run::FairnessCheck::run(&conn)?;

    // The DVFS ramp eater (docs/silicon/00): ≥ 200 ms of warm work before
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
        trace_dir: out_dir.join("trace"),
        db: &db,
        conn: &conn,
        flames: Vec::new(),
    };
    let mut reads = Vec::new();
    for family in families::all() {
        if selected(family.name) {
            reads.push(run.read_family(family)?);
        }
    }
    let flames = std::mem::take(&mut run.flames);
    drop(run);

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
        corpus_digest: gen::digest_hex(&gen::corpus_digest(cfg)),
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
        budget_gates: cfg.scale == gen::Scale::L,
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

/// `trace`: one traced warm+cold pair for one read family — artifacts
/// only, the quick-look tool.
///
/// # Errors
///
/// Unknown family; setup errors.
pub fn cmd_trace(corpus: &CorpusArgs, family_name: &str) -> Result<(), String> {
    let cfg = gen_config(corpus);
    let family = families::all()
        .iter()
        .find(|f| f.name == family_name)
        .ok_or_else(|| {
            format!(
                "unknown family `{family_name}` (families: {})",
                families::all()
                    .iter()
                    .map(|f| f.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let paths = ensure_corpus(&corpus.dir, cfg)?;

    // The cold half touches (commits), so trace runs on a scratch copy —
    // never the verified corpus.
    let scratch = paths.root.join("trace-scratch");
    let _ = std::fs::remove_dir_all(&scratch);
    let db = Db::create(&scratch.join("db"), schema()).map_err(|e| format!("{e:?}"))?;
    corpus::load_bumbledb(&db, cfg).map_err(|e| format!("{e:?}"))?;

    let query = (family.query)();
    let mut prepared = db.prepare(&query).map_err(|e| format!("prepare: {e:?}"))?;
    let mut rotation = Rotation::new((family.params)(&cfg));
    let mut buffer = ResultBuffer::new();
    let mut run = || {
        let params = rotation.next_set().to_vec();
        db.read(|snap| snap.execute(&mut prepared, &params, &mut buffer))
            .map_err(|e| format!("execute: {e:?}"))?;
        Ok(buffer.len() as u64)
    };
    for _ in 0..4 {
        run()?;
    }
    let trace_dir = paths.root.join("trace");
    let (_, events) = harness::traced_sample(&mut run)?;
    let (engine, harness_events) = trace_out::split_harness(events);
    let warm = trace_out::write_trace_file(
        &trace_dir,
        &format!("{family_name}.warm"),
        &engine,
        &harness_events,
    )
    .map_err(|e| format!("trace: {e}"))?;
    print!("{}", trace_out::FlameSummary::compute(&engine).render());
    if let Some(phases) = trace_out::render_phase_table(&engine) {
        print!("{phases}");
    }

    let (_, events) = harness::traced_cold_sample(&mut harness::tag_touch(&db), &mut run)?;
    let (engine, harness_events) = trace_out::split_harness(events);
    let cold = trace_out::write_trace_file(
        &trace_dir,
        &format!("{family_name}.cold"),
        &engine,
        &harness_events,
    )
    .map_err(|e| format!("trace: {e}"))?;
    println!("traces: {} / {}", warm.display(), cold.display());
    drop(db);
    let _ = std::fs::remove_dir_all(&scratch);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::Scale;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bumbledb-bench-driver-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    const CFG: GenConfig = GenConfig {
        seed: 1,
        scale: Scale::S,
    };

    /// The marker short-circuits regeneration (the counter hook).
    #[test]
    fn the_digest_directory_is_reused() {
        let dir = scratch("reuse");
        let mut loads = 0;
        let mut loader = |paths: &CorpusPaths| {
            loads += 1;
            std::fs::create_dir_all(&paths.db).map_err(|e| e.to_string())
        };
        let first = ensure_corpus_with(&dir, CFG, &mut loader).expect("first");
        let second = ensure_corpus_with(&dir, CFG, &mut loader).expect("second");
        assert_eq!(first, second);
        assert_eq!(loads, 1, "the marker short-circuits regeneration");
        // A different seed keys a different directory.
        let other = corpus_paths(
            &dir,
            GenConfig {
                seed: 2,
                scale: Scale::S,
            },
        );
        assert_ne!(first.root, other.root);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_refusal_messages_substitute_the_flags() {
        let corpus = CorpusArgs {
            scale: Scale::M,
            seed: 9,
            dir: PathBuf::from("/tmp/corpora"),
        };
        assert_eq!(
            stamp_refusal(&corpus),
            "bench refuses: no fresh verify stamp for this corpus.\n\
             run first: bumbledb-bench verify --scale M --seed 9 --dir /tmp/corpora\n\
             (or pass --i-am-lying to run unverified — the report will say so)"
        );
        assert_eq!(
            obs_missing("--alloc"),
            "--alloc needs an obs build; run:\n\
             cargo run -p bumbledb-bench --features obs --release -- …"
        );
    }

    /// An unstamped bench refuses; the message names verify.
    #[test]
    fn bench_refuses_without_a_stamp() {
        let dir = scratch("refuse");
        let args = BenchArgs {
            corpus: CorpusArgs {
                scale: Scale::S,
                seed: 1,
                dir: dir.clone(),
            },
            families: Some(vec!["point".to_owned()]),
            samples: Some(8),
            trace: false,
            alloc: false,
            out: Some(dir.join("out")),
            i_am_lying: false,
        };
        let err = cmd_bench(&args).unwrap_err();
        assert!(err.contains("bumbledb-bench verify"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(not(feature = "obs"))]
    #[test]
    fn alloc_without_obs_names_the_cargo_invocation() {
        let args = BenchArgs {
            corpus: CorpusArgs::default(),
            families: None,
            samples: None,
            trace: false,
            alloc: true,
            out: None,
            i_am_lying: false,
        };
        let err = cmd_bench(&args).unwrap_err();
        assert!(
            err.contains("cargo run -p bumbledb-bench --features obs --release"),
            "{err}"
        );
    }

    /// The suite's own integration point (unit-scale by S's size):
    /// gen → verify → bench --families point --samples 8, three
    /// artifacts, PARTIAL verdict, and the UNVERIFIED override branding.
    #[test]
    fn the_full_sequence_runs_at_s() {
        let dir = scratch("e2e");
        let corpus = CorpusArgs {
            scale: Scale::S,
            seed: 1,
            dir: dir.clone(),
        };
        cmd_gen(&corpus).expect("gen");
        let paths = corpus_paths(&dir, CFG);
        assert!(paths.db.join("data.mdb").exists(), "compacted store");
        assert!(
            !paths.root.join("db-load").exists(),
            "no load-scratch residue (docs/architecture/40-storage.md)"
        );
        assert_eq!(cmd_verify(&corpus, 25).expect("verify"), 0);

        let out = dir.join("out");
        let args = BenchArgs {
            corpus: corpus.clone(),
            families: Some(vec!["point".to_owned()]),
            samples: Some(8),
            trace: false,
            alloc: false,
            out: Some(out.clone()),
            i_am_lying: false,
        };
        let code = cmd_bench(&args).expect("bench");
        assert!(code == 0 || code == 1, "a gate verdict, not a refusal");
        let mut names: Vec<String> = std::fs::read_dir(&out)
            .expect("read out")
            .map(|e| e.expect("entry").file_name().into_string().expect("utf-8"))
            .filter(|name| {
                std::path::Path::new(name)
                    .extension()
                    .is_some_and(|ext| ext == "md" || ext == "json")
            })
            .collect();
        names.sort();
        assert_eq!(names, ["QUERIES.md", "report.json", "report.md"]);
        let md = std::fs::read_to_string(out.join("report.md")).expect("read");
        assert!(md.contains("PARTIAL — filtered run"), "{md}");
        assert!(!md.contains("UNVERIFIED"), "verified run");
        assert!(
            md.contains("(families + 25 randomized cases)"),
            "the provenance shows how much evidence earned the stamp: {md}"
        );

        // The override path brands the report.
        let lying = BenchArgs {
            families: Some(vec!["point".to_owned()]),
            out: Some(dir.join("lying-out")),
            i_am_lying: true,
            ..args.clone()
        };
        // Invalidate the stamp by changing the recorded case count.
        std::fs::write(paths.root.join(super::CASES_FILE), "26").expect("tamper");
        cmd_bench(&lying).expect("bench --i-am-lying");
        let md = std::fs::read_to_string(dir.join("lying-out").join("report.md")).expect("read");
        assert!(md.contains("UNVERIFIED"), "{md}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
