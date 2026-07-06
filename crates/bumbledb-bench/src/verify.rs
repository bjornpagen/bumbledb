//! `verify` — the oracle command and the stamp (docs/architecture/50-validation.md): the
//! command that earns the right to time anything. Every family query and
//! N randomized queries must produce value-identical result multisets on
//! bumbledb and `SQLite`, or the run fails loudly with arbitration
//! bundles.
//!
//! Arbitration procedure (normative): an engine-vs-`SQLite` mismatch on a
//! *family* ⇒ compare the translator's output against the hand-written
//! golden (docs/architecture/50-validation.md). Golden ≠ translator ⇒ translator bug;
//! golden == translator ⇒ a human reads the semantics docs and rules
//! which engine is wrong. Randomized mismatches: minimize by re-running
//! the case's shape at smaller scales (manual; the bundle carries
//! everything needed).

use std::path::{Path, PathBuf};

use bumbledb::schema::ValueType;
use bumbledb::{Db, ResultBuffer, Value};

use crate::gen::{self, GenConfig, Rng};
use crate::schema::schema;
use crate::translate::translate;
use crate::{compare, corpus, families, querygen};

/// The verify run's identity: corpus config, randomized-case count, and
/// the tool-owned scratch directory (delete-and-recreated — never point
/// it at user data).
#[derive(Debug, Clone)]
pub struct VerifyConfig {
    pub gen: GenConfig,
    pub random_cases: u32,
    pub out_dir: PathBuf,
}

/// The default randomized-case count.
pub const DEFAULT_RANDOM_CASES: u32 = 500;

/// A successful verify: how much evidence was collected, and the stamp
/// that now gates timing runs.
#[derive(Debug, Clone)]
pub struct VerifyReport {
    /// Query × param-set executions compared.
    pub cases: u64,
    /// The stamp hex, also written to `out_dir/verify.stamp`.
    pub stamp: String,
}

/// A failed verify: the arbitration bundle directories (up to 8).
#[derive(Debug, Clone)]
pub struct VerifyFailure {
    pub bundles: Vec<PathBuf>,
}

impl std::fmt::Display for VerifyFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "verify FAILED: {} mismatch(es)", self.bundles.len())?;
        for bundle in &self.bundles {
            writeln!(f, "  {}", bundle.display())?;
        }
        Ok(())
    }
}

/// The running binary's blake3 fingerprint, computed once per process.
/// One hash covers the engine, the translator, the comparator, the
/// generator, and every param policy at once — a stamp bound to it
/// vouches for the exact code that earned it. Consequences, accepted:
/// any rebuild re-keys the stamp (over-invalidation by embedded paths
/// included — re-verification is the honest default), and
/// [`stamp_matches`] fails for any binary other than the one that
/// earned the stamp, which is precisely the contract.
///
/// # Panics
///
/// On tool-level I/O failure reading the running executable.
#[must_use]
pub fn binary_fingerprint() -> [u8; 32] {
    static FINGERPRINT: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
    *FINGERPRINT.get_or_init(|| {
        let exe = std::env::current_exe().expect("current_exe");
        let bytes = std::fs::read(exe).expect("read the running binary");
        let mut digest = bumbledb::digest::Digest::new();
        digest.update(&bytes);
        digest.finalize()
    })
}

/// The stamp value for a config: hex blake3 over the running binary's
/// fingerprint, the corpus digest, the family-list digest, the
/// randomized-case count, and the seed. Any ingredient change — any
/// rebuild — invalidates every stored stamp.
#[must_use]
pub fn stamp_value(cfg: &VerifyConfig) -> String {
    stamp_value_with(cfg, &binary_fingerprint())
}

/// [`stamp_value`] with an explicit binary fingerprint — the test seam
/// proving the fingerprint is a live ingredient.
fn stamp_value_with(cfg: &VerifyConfig, fingerprint: &[u8; 32]) -> String {
    let mut digest = bumbledb::digest::Digest::new();
    digest.update(fingerprint);
    digest.update(&gen::corpus_digest(cfg.gen));
    digest.update(&families::digest());
    digest.update(&cfg.random_cases.to_le_bytes());
    digest.update(&cfg.gen.seed.to_le_bytes());
    gen::digest_hex(&digest.finalize())
}

/// Whether `path` holds the stamp for this config — the gate the harness
/// (PRD 13) and the CLI (PRD 19) consume before timing anything.
#[must_use]
pub fn stamp_matches(cfg: &VerifyConfig, path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|stored| stored.trim() == stamp_value(cfg))
}

/// One case's identity, for bundles and progress.
struct Case<'a> {
    label: String,
    query: &'a bumbledb::Query,
    sql: &'a str,
    /// The family's hand-written golden, when the case is a family.
    golden_sql: Option<&'static str>,
}

/// Everything a run accumulates.
struct Run<'a> {
    db: &'a Db<'a>,
    conn: &'a rusqlite::Connection,
    out_dir: PathBuf,
    cases: u64,
    total: u64,
    bundles: Vec<PathBuf>,
}

/// How many mismatch bundles a run collects before giving up.
const MAX_BUNDLES: usize = 8;

impl Run<'_> {
    /// Executes one query × param set on both stores and compares. Returns
    /// `false` once the bundle budget is exhausted (stop the run).
    ///
    /// Divergence-by-error is a mismatch, not a panic: if either side
    /// errors at prepare or execute where the other answers, that is an
    /// arbitration bundle with the erring side's `ERROR: <text>` in
    /// place of its rows — the audit confirmed a real divergence class
    /// here (`SQLite`'s transient SUM overflow vs the i128 accumulator).
    /// Both-sides-error is a bundle too: no case is *expected* to error
    /// today, so agreement-in-error would hide a tool defect as
    /// verification. Setup errors (store open, corpus load) stay panics.
    fn check(
        &mut self,
        case: &Case<'_>,
        param_order: &[bumbledb::ParamId],
        params: &[Value],
    ) -> bool {
        // Column types come from the engine's prepared query; without
        // them the oracle's rows cannot even be decoded, so a prepare
        // failure is an engine-side error and the oracle records "not
        // executed" rather than a fabricated second error.
        let (ours, theirs): (
            Result<Vec<compare::Row>, String>,
            Result<Vec<compare::Row>, String>,
        ) = match self.db.prepare(case.query) {
            Err(e) => (
                Err(format!("{e}")),
                Err("not executed: no column types without a prepared query".to_owned()),
            ),
            Ok(mut prepared) => {
                let types: Vec<ValueType> = prepared.column_types().cloned().collect();
                let mut buffer = ResultBuffer::new();
                let ours = self
                    .db
                    .read(|snap| snap.execute(&mut prepared, params, &mut buffer))
                    .map(|()| compare::from_buffer(&buffer, &types))
                    .map_err(|e| format!("{e}"));
                let theirs = self
                    .conn
                    .prepare_cached(case.sql)
                    .map_err(|e| e.to_string())
                    .and_then(|mut stmt| {
                        compare::from_sqlite(&mut stmt, param_order, params, &types)
                    });
                (ours, theirs)
            }
        };

        self.cases += 1;
        if self.cases.is_multiple_of(100) {
            eprintln!("verify: {}/{} cases", self.cases, self.total);
        }

        let verdict: Result<(), (String, String, String)> = match (ours, theirs) {
            (Ok(ours), Ok(theirs)) => compare::multisets(ours.clone(), theirs.clone())
                .map_err(|m| (m.to_string(), render_rows(&ours), render_rows(&theirs))),
            (Err(engine), Ok(theirs)) => Err((
                "divergence by error: the engine errored where the oracle answered".to_owned(),
                format!("ERROR: {engine}"),
                render_rows(&theirs),
            )),
            (Ok(ours), Err(oracle)) => Err((
                "divergence by error: the oracle errored where the engine answered".to_owned(),
                render_rows(&ours),
                format!("ERROR: {oracle}"),
            )),
            (Err(engine), Err(oracle)) => Err((
                "both sides errored — a tool defect must not look like verification".to_owned(),
                format!("ERROR: {engine}"),
                format!("ERROR: {oracle}"),
            )),
        };

        if let Err((mismatch, ours_text, theirs_text)) = verdict {
            let bundle = self
                .out_dir
                .join(format!("mismatch-{}", self.bundles.len()));
            std::fs::create_dir_all(&bundle).expect("bundle dir");
            std::fs::write(
                bundle.join("query.txt"),
                format!("{}\n{:#?}\n", case.label, case.query),
            )
            .expect("bundle");
            std::fs::write(bundle.join("query.sql"), case.sql).expect("bundle");
            std::fs::write(bundle.join("params.txt"), format!("{params:#?}\n")).expect("bundle");
            std::fs::write(bundle.join("mismatch.txt"), mismatch).expect("bundle");
            std::fs::write(bundle.join("ours.txt"), ours_text).expect("bundle");
            std::fs::write(bundle.join("theirs.txt"), theirs_text).expect("bundle");
            if let Some(golden) = case.golden_sql {
                std::fs::write(bundle.join("golden.sql"), golden).expect("bundle");
            }
            eprintln!("verify: MISMATCH {} -> {}", case.label, bundle.display());
            self.bundles.push(bundle);
        }
        self.bundles.len() < MAX_BUNDLES
    }
}

/// Renders a comparison multiset for a bundle artifact.
fn render_rows(rows: &[compare::Row]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "{} row(s)", rows.len());
    for row in rows {
        let _ = writeln!(out, "{row:?}");
    }
    out
}

/// Runs the full oracle: load both stores, compare every family × its
/// param sets plus `random_cases` randomized queries × theirs, and stamp
/// on success.
///
/// # Errors
///
/// [`VerifyFailure`] carrying the arbitration bundle paths.
///
/// # Panics
///
/// On tool-level invariant violations (scratch I/O, either store
/// refusing a verified query) — never on a result mismatch.
pub fn run(cfg: &VerifyConfig) -> Result<VerifyReport, VerifyFailure> {
    run_with_sql_override(cfg, |_| None)
}

/// [`run`], with a test hook substituting the SQL sent to `SQLite` for a
/// named family (the mismatch path's test seam — a deliberately wrong
/// override must fail the run with a full bundle).
///
/// # Errors
///
/// As [`run`].
///
/// # Panics
///
/// As [`run`].
pub fn run_with_sql_override(
    cfg: &VerifyConfig,
    override_sql: impl Fn(&str) -> Option<String>,
) -> Result<VerifyReport, VerifyFailure> {
    // The out_dir is the tool's own scratch: delete-and-recreate.
    let _ = std::fs::remove_dir_all(&cfg.out_dir);
    std::fs::create_dir_all(&cfg.out_dir).expect("out_dir");

    eprintln!(
        "verify: loading corpus (seed {}, scale {})",
        cfg.gen.seed,
        cfg.gen.scale.label()
    );
    let db = Db::create(&cfg.out_dir.join("db"), schema()).expect("create store");
    corpus::load_bumbledb(&db, cfg.gen).expect("load bumbledb");
    let (conn, _) =
        corpus::load_sqlite(&cfg.out_dir.join("oracle.sqlite"), cfg.gen).expect("load oracle");
    run_prepared(cfg, &db, &conn, override_sql)
}

/// The oracle against *pre-loaded* stores (the CLI's digest-keyed cache
/// path): stale bundles and the stamp are cleared, the stores are left
/// untouched, and bundles/stamp land in `cfg.out_dir`.
///
/// # Errors
///
/// As [`run`].
///
/// # Panics
///
/// As [`run`].
pub fn run_prepared(
    cfg: &VerifyConfig,
    db: &Db<'_>,
    conn: &rusqlite::Connection,
    override_sql: impl Fn(&str) -> Option<String>,
) -> Result<VerifyReport, VerifyFailure> {
    std::fs::create_dir_all(&cfg.out_dir).expect("out_dir");
    if let Ok(entries) = std::fs::read_dir(&cfg.out_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with("mismatch-") {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    let _ = std::fs::remove_file(cfg.out_dir.join("verify.stamp"));

    let family_cases: u64 = families::all()
        .iter()
        .map(|f| (f.params)(&cfg.gen).len() as u64)
        .sum();
    let mut run = Run {
        db,
        conn,
        out_dir: cfg.out_dir.clone(),
        cases: 0,
        total: 2 * family_cases
            + (u64::from(cfg.random_cases) + u64::from(EMPTY_STORE_RANDOM_CASES)) * 4,
        bundles: Vec::new(),
    };

    'cases: {
        for family in families::all() {
            let query = (family.query)();
            let translated = translate(&query, schema()).expect("families translate");
            let sql = override_sql(family.name).unwrap_or_else(|| translated.sql.clone());
            let case = Case {
                label: format!("family {}", family.name),
                query: &query,
                sql: &sql,
                golden_sql: Some(family.golden_sql),
            };
            for params in (family.params)(&cfg.gen) {
                if !run.check(&case, &translated.params, &params) {
                    break 'cases;
                }
            }
        }
        let mut rng = Rng::new(cfg.gen.seed ^ 0x0112_0001);
        for index in 0..cfg.random_cases {
            let query = querygen::random_query(&mut rng, cfg.gen);
            let translated = translate(&query, schema()).expect("generated queries translate");
            let case = Case {
                label: format!("random {index}"),
                query: &query,
                sql: &translated.sql,
                golden_sql: None,
            };
            for params in querygen::params_for(&query, &mut rng, cfg.gen) {
                if !run.check(&case, &translated.params, &params) {
                    break 'cases;
                }
            }
        }
    }

    if run.bundles.len() < MAX_BUNDLES {
        run_empty_store(cfg, &mut run);
    }

    if !run.bundles.is_empty() {
        return Err(VerifyFailure {
            bundles: run.bundles,
        });
    }
    let stamp = stamp_value(cfg);
    std::fs::write(cfg.out_dir.join("verify.stamp"), &stamp).expect("stamp");
    eprintln!("verify: OK — {} cases, stamp {stamp}", run.cases);
    Ok(VerifyReport {
        cases: run.cases,
        stamp,
    })
}

/// The randomized slice of the empty-store pass.
const EMPTY_STORE_RANDOM_CASES: u32 = 100;

/// The empty-store pass (hardening PRD 09): a fresh store pair with the
/// schema loaded and **zero rows anywhere**, over which every family and
/// a seeded slice of randomized queries run and compare. Every gate is
/// false, every scan empty, every aggregate folds nothing (the
/// empty-set-not-NULL rule and the HAVING template earn their keep),
/// every selection misses — the entire empty-relation semantic surface,
/// oracle-checked in milliseconds with zero corpus churn. Cases count
/// into the stamp's evidence; bundles land beside the main run's.
///
/// # Panics
///
/// On tool-level invariant violations, including the structural gate
/// check: the randomized slice must contain at least one gate-bearing
/// query, so gate falsity is exercised by construction, not by luck.
fn run_empty_store(cfg: &VerifyConfig, run: &mut Run<'_>) {
    let empty_dir = cfg.out_dir.join("empty-db");
    let _ = std::fs::remove_dir_all(&empty_dir);
    let empty_db = Db::create(&empty_dir, schema()).expect("create empty store");
    let empty_conn = rusqlite::Connection::open_in_memory().expect("empty oracle");
    for statement in crate::sqlmap::ddl(schema()) {
        empty_conn.execute(&statement, []).expect("empty ddl");
    }
    let mut empty_run = Run {
        db: &empty_db,
        conn: &empty_conn,
        out_dir: run.out_dir.clone(),
        cases: run.cases,
        total: run.total,
        bundles: std::mem::take(&mut run.bundles),
    };
    let mut gate_bearing = 0u32;
    'empty: {
        for family in families::all() {
            let query = (family.query)();
            let translated = translate(&query, schema()).expect("families translate");
            let case = Case {
                label: format!("empty family {}", family.name),
                query: &query,
                sql: &translated.sql,
                golden_sql: Some(family.golden_sql),
            };
            for params in (family.params)(&cfg.gen) {
                if !empty_run.check(&case, &translated.params, &params) {
                    break 'empty;
                }
            }
        }
        let mut rng = Rng::new(cfg.gen.seed ^ 0x0112_0002);
        for index in 0..EMPTY_STORE_RANDOM_CASES {
            let query = querygen::random_query(&mut rng, cfg.gen);
            gate_bearing += u32::from(query.atoms.iter().any(|a| a.bindings.is_empty()));
            let translated = translate(&query, schema()).expect("generated queries translate");
            let case = Case {
                label: format!("empty random {index}"),
                query: &query,
                sql: &translated.sql,
                golden_sql: None,
            };
            for params in querygen::params_for(&query, &mut rng, cfg.gen) {
                if !empty_run.check(&case, &translated.params, &params) {
                    break 'empty;
                }
            }
        }
        assert!(
            gate_bearing > 0,
            "the empty-store slice generated no gate-bearing query"
        );
    }
    run.cases = empty_run.cases;
    run.bundles = empty_run.bundles;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::Scale;

    fn scratch(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("bumbledb-bench-verify-{tag}"))
    }

    fn cfg(tag: &str) -> VerifyConfig {
        VerifyConfig {
            gen: GenConfig {
                seed: 1,
                scale: Scale::S,
            },
            random_cases: 50,
            out_dir: scratch(tag),
        }
    }

    #[test]
    fn the_stamp_tracks_every_ingredient() {
        let base = cfg("stamp");
        let baseline = stamp_value(&base);
        assert_eq!(baseline, stamp_value(&base), "deterministic");
        let mut seed = base.clone();
        seed.gen.seed = 2;
        assert_ne!(stamp_value(&seed), baseline, "seed is an ingredient");
        let mut cases = base.clone();
        cases.random_cases = 51;
        assert_ne!(stamp_value(&cases), baseline, "case count is an ingredient");
    }

    /// PRD 07 (docs/hardening): the stamp is bound to the binary that
    /// earned it. The fingerprint ingredient is blake3 of the running
    /// executable, and flipping it flips the stamp — a stamp computed
    /// under any other fingerprint is rejected.
    #[test]
    fn the_stamp_is_bound_to_the_binary() {
        let base = cfg("stamp-binary");
        // The fingerprint is exactly blake3 of the running executable.
        let exe = std::env::current_exe().expect("exe");
        let bytes = std::fs::read(exe).expect("read");
        let mut digest = bumbledb::digest::Digest::new();
        digest.update(&bytes);
        assert_eq!(binary_fingerprint(), digest.finalize());

        // Flipping the fingerprint flips the stamp...
        let mut foreign = binary_fingerprint();
        foreign[0] ^= 0xFF;
        let foreign_stamp = stamp_value_with(&base, &foreign);
        assert_ne!(foreign_stamp, stamp_value(&base));

        // ...and stamp_matches rejects a stamp another binary earned.
        std::fs::create_dir_all(&base.out_dir).expect("dir");
        let path = base.out_dir.join("verify.stamp");
        std::fs::write(&path, &foreign_stamp).expect("write");
        assert!(!stamp_matches(&base, &path));
        std::fs::write(&path, stamp_value(&base)).expect("write");
        assert!(stamp_matches(&base, &path), "this binary's stamp accepts");
        let _ = std::fs::remove_dir_all(&base.out_dir);
    }

    /// PRD 07: one side erroring where the other answers is a mismatch
    /// bundle with an `ERROR:` artifact — never a panic, never a stamp.
    #[test]
    fn divergence_by_error_is_a_bundle_not_a_panic() {
        let mut config = cfg("error-divergence");
        config.random_cases = 0;
        let failure = run_with_sql_override(&config, |family| {
            (family == "point").then(|| "SELECT this is not sql".to_owned())
        })
        .expect_err("must fail");
        assert!(!failure.bundles.is_empty());
        let theirs =
            std::fs::read_to_string(failure.bundles[0].join("theirs.txt")).expect("artifact");
        assert!(theirs.starts_with("ERROR:"), "{theirs}");
        let ours = std::fs::read_to_string(failure.bundles[0].join("ours.txt")).expect("artifact");
        assert!(ours.contains("row(s)"), "the engine's rows render: {ours}");
        let mismatch =
            std::fs::read_to_string(failure.bundles[0].join("mismatch.txt")).expect("artifact");
        assert!(mismatch.contains("divergence by error"), "{mismatch}");
        assert!(
            !config.out_dir.join("verify.stamp").exists(),
            "no stamp on failure"
        );
        let _ = std::fs::remove_dir_all(&config.out_dir);
    }

    #[test]
    fn stamp_matches_accepts_and_rejects() {
        let base = cfg("stamp-match");
        std::fs::create_dir_all(&base.out_dir).expect("dir");
        let path = base.out_dir.join("verify.stamp");
        assert!(!stamp_matches(&base, &path), "missing file rejects");
        std::fs::write(&path, stamp_value(&base)).expect("write");
        assert!(stamp_matches(&base, &path));
        std::fs::write(&path, "not a stamp").expect("write");
        assert!(!stamp_matches(&base, &path));
        let _ = std::fs::remove_dir_all(&base.out_dir);
    }

    /// A deliberately wrong SQL for one family fails the run with full
    /// arbitration bundles.
    #[test]
    fn a_wrong_oracle_fails_with_a_bundle() {
        let mut config = cfg("mismatch");
        config.random_cases = 0;
        let failure = run_with_sql_override(&config, |family| {
            (family == "point").then(|| {
                // Off-by-one: the wrong posting's values on every hit.
                "SELECT DISTINCT t0.\"amount\", t0.\"at\" FROM \"Posting\" AS t0 \
                 WHERE t0.\"id\" = ?1 + 1"
                    .to_owned()
            })
        })
        .expect_err("must fail");
        assert!(!failure.bundles.is_empty());
        assert!(failure.to_string().contains("mismatch"));
        for name in [
            "query.txt",
            "query.sql",
            "params.txt",
            "mismatch.txt",
            "golden.sql",
        ] {
            let content = std::fs::read_to_string(failure.bundles[0].join(name)).expect("artifact");
            assert!(!content.is_empty(), "{name} must have content");
        }
        assert!(
            !config.out_dir.join("verify.stamp").exists(),
            "no stamp on failure"
        );
        let _ = std::fs::remove_dir_all(&config.out_dir);
    }

    /// The full oracle at S: families + 50 randomized cases agree, and the
    /// stamp lands.
    #[test]
    fn a_full_verify_at_s_succeeds() {
        let config = cfg("full");
        let report = run(&config).expect("verify succeeds");
        assert!(report.cases > 200, "families + 50 x 4 randomized");
        let stamp_path = config.out_dir.join("verify.stamp");
        assert!(stamp_matches(&config, &stamp_path));
        // A different config must not accept this stamp.
        let mut other = config.clone();
        other.random_cases += 1;
        assert!(!stamp_matches(&other, &stamp_path));
        let _ = std::fs::remove_dir_all(&config.out_dir);
    }
}
