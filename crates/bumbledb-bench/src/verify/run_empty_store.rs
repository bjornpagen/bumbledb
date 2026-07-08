use super::{Case, Db, EMPTY_STORE_RANDOM_CASES, Run, VerifyConfig};

use crate::families;
use crate::gen::Rng;
use crate::querygen;
use crate::schema::schema;
use crate::translate::translate;

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
pub(super) fn run_empty_store(cfg: &VerifyConfig, run: &mut Run<'_>) {
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
