use super::{
    Case, Db, EMPTY_STORE_RANDOM_CASES, MAX_BUNDLES, Run, VerifyConfig, VerifyFailure, VerifyReport,
    stamp_value,
};

use crate::gen::Rng;
use crate::querygen;
use crate::schema::schema;
use crate::translate::translate;
use crate::{corpus, families};

use super::run_empty_store::run_empty_store;

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
