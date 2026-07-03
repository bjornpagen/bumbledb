//! `verify` — the oracle command and the stamp (docs/benchmarks/12): the
//! command that earns the right to time anything. Every family query and
//! N randomized queries must produce value-identical result multisets on
//! bumbledb and `SQLite`, or the run fails loudly with arbitration
//! bundles.
//!
//! Arbitration procedure (normative): an engine-vs-`SQLite` mismatch on a
//! *family* ⇒ compare the translator's output against the hand-written
//! golden (docs/benchmarks/09). Golden ≠ translator ⇒ translator bug;
//! golden == translator ⇒ a human reads the semantics docs and rules
//! which engine is wrong. Randomized mismatches: minimize by re-running
//! the case's shape at smaller scales (manual; the bundle carries
//! everything needed).

use std::path::{Path, PathBuf};

use bumbledb::schema::ValueType;
use bumbledb::{Db, ResultBuffer, Value};

use crate::gen::{self, GenConfig, Rng, Sizes};
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

/// The stamp value for a config: hex blake3 over the crate version, the
/// corpus digest, the family-list digest, the randomized-case count, and
/// the seed. Any ingredient change invalidates every stored stamp.
#[must_use]
pub fn stamp_value(cfg: &VerifyConfig) -> String {
    let mut digest = bumbledb::digest::Digest::new();
    digest.update(env!("CARGO_PKG_VERSION").as_bytes());
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
    fn check(
        &mut self,
        case: &Case<'_>,
        param_order: &[bumbledb::ParamId],
        params: &[Value],
    ) -> bool {
        let mut prepared = self
            .db
            .prepare(case.query)
            .expect("verified queries prepare");
        let types: Vec<ValueType> = prepared.column_types().cloned().collect();
        let mut buffer = ResultBuffer::new();
        self.db
            .read(|snap| snap.execute(&mut prepared, params, &mut buffer))
            .expect("engine executes");
        let ours = compare::from_buffer(&buffer, &types);

        let mut stmt = self.conn.prepare_cached(case.sql).expect("oracle prepares");
        let theirs =
            compare::from_sqlite(&mut stmt, param_order, params, &types).expect("oracle executes");

        self.cases += 1;
        if self.cases.is_multiple_of(100) {
            eprintln!("verify: {}/{} cases", self.cases, self.total);
        }

        if let Err(mismatch) = compare::multisets(ours, theirs) {
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
            std::fs::write(bundle.join("mismatch.txt"), mismatch.to_string()).expect("bundle");
            if let Some(golden) = case.golden_sql {
                std::fs::write(bundle.join("golden.sql"), golden).expect("bundle");
            }
            eprintln!("verify: MISMATCH {} -> {}", case.label, bundle.display());
            self.bundles.push(bundle);
        }
        self.bundles.len() < MAX_BUNDLES
    }
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

    let sizes = Sizes::of(cfg.gen.scale);
    let family_cases: u64 = families::all()
        .iter()
        .map(|f| (f.params)(&cfg.gen).len() as u64)
        .sum();
    let mut run = Run {
        db,
        conn,
        out_dir: cfg.out_dir.clone(),
        cases: 0,
        total: family_cases + u64::from(cfg.random_cases) * 4,
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
            let query = querygen::random_query(&mut rng, &sizes);
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
