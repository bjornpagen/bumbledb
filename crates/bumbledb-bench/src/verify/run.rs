use super::{
    Case, Db, EMPTY_STORE_RANDOM_CASES, MAX_BUNDLES, Run, VerifyConfig, VerifyFailure,
    VerifyReport, stamp_value,
};

use bumbledb::Value;

use crate::corpus_gen::Rng;
use crate::families::set_bindings;
use crate::naive::{NaiveDb, ParamValue};
use crate::querygen::{self, ParamDraw, target};
use crate::schema::{Ledger, schema};
use crate::translate::{Inexpressible, LaneCase, sqlite_expressible, translate};
use crate::{corpus, differential, families, sqlmap};

use super::run_empty_store::run_empty_store;
use super::run_naive::run_naive_slice;

/// # Errors
/// # Panics
/// On tool-level invariant violations (scratch I/O, either store
pub fn run(cfg: &VerifyConfig) -> Result<VerifyReport, VerifyFailure> {
    run_with_sql_override(cfg, |_| None)
}

/// # Errors
/// # Panics
pub fn run_with_sql_override(
    cfg: &VerifyConfig,
    override_sql: impl Fn(&str) -> Option<String>,
) -> Result<VerifyReport, VerifyFailure> {
    let _ = std::fs::remove_dir_all(&cfg.out_dir);
    std::fs::create_dir_all(&cfg.out_dir).expect("out_dir");

    eprintln!(
        "verify: loading corpus (seed {}, scale {})",
        cfg.corpus_gen.seed,
        cfg.corpus_gen.scale.label()
    );
    let db = Db::create(&cfg.out_dir.join("db"), Ledger)
        .expect("create store")
        .expect("accepted");
    corpus::load_bumbledb(&db, cfg.corpus_gen).expect("load bumbledb");
    let (conn, _) = corpus::load_sqlite(&cfg.out_dir.join("oracle.sqlite"), cfg.corpus_gen)
        .expect("load oracle");
    eprintln!("verify: loading the calendar corpus");
    let cal_db = Db::create(&cfg.out_dir.join("cal-db"), crate::calendar::Scheduling)
        .expect("create calendar store")
        .expect("accepted");
    crate::calendar::corpus::load_bumbledb(&cal_db, cfg.corpus_gen).expect("load calendar");
    let (cal_conn, _) = crate::calendar::corpus::load_sqlite(
        &cfg.out_dir.join("cal-oracle.sqlite"),
        cfg.corpus_gen,
    )
    .expect("load calendar oracle");
    run_prepared(cfg, &db, &conn, &cal_db, &cal_conn, override_sql)
}

impl<S> Run<'_, S> {
    pub(super) fn lane<T>(
        &mut self,
        db: &Db<T>,
        conn: &rusqlite::Connection,
        body: impl FnOnce(&mut Run<'_, T>),
    ) {
        let mut sub = Run {
            db,
            conn,
            out_dir: self.out_dir.clone(),
            cases: self.cases,
            total: self.total,
            bundles: std::mem::take(&mut self.bundles),
        };
        body(&mut sub);
        self.cases = sub.cases;
        self.bundles = sub.bundles;
    }
}

pub(super) fn family_lane<S>(
    run: &mut Run<'_, S>,
    cfg: &VerifyConfig,
    label: &str,
    override_sql: &dyn Fn(&str) -> Option<String>,
) {
    'families: for family in families::all() {
        let query = (family.query)();
        for params in (family.params)(&cfg.corpus_gen) {
            let translated =
                translate(&query, schema(), &set_bindings(&params)).expect("families translate");
            let sql = override_sql(family.name).unwrap_or(translated.sql);
            let case = Case {
                label: format!("{label} {}", family.name),
                query: &query,
                sql: &sql,
                golden_sql: Some(family.golden_sql),
            };
            if !run.check(&case, &translated.params, &params) {
                break 'families;
            }
        }
    }
}

pub(super) fn random_lane<S>(
    run: &mut Run<'_, S>,
    cfg: &VerifyConfig,
    cases: u32,
    seed_salt: u64,
    label: &str,
    mut on_query: impl FnMut(&bumbledb::Query),
    naive_routed: &mut Vec<differential::Op>,
) {
    let mut rng = Rng::new(cfg.corpus_gen.seed ^ seed_salt);
    'random: for index in 0..cases {
        let query = querygen::random_query(&mut rng, cfg.corpus_gen);
        on_query(&query);

        let expressible = sqlite_expressible(&LaneCase::Query(&query));
        for draw in querygen::params_for(&query, &mut rng, cfg.corpus_gen) {
            match expressible {
                Ok(()) => {
                    let translated = translate(&query, target::schema(), &draw.sets)
                        .expect("expressible queries translate");
                    let case = Case {
                        label: format!("{label} {index}"),
                        query: &query,
                        sql: &translated.sql,
                        golden_sql: None,
                    };
                    if !run.check(&case, &translated.params, &positional(&draw)) {
                        break 'random;
                    }
                }
                Err(Inexpressible::PackAggregate | Inexpressible::IntervalDerivedColumn) => {
                    naive_routed.push(differential::Op::Query {
                        query: query.clone(),
                        params: positional(&draw),
                    });
                }
                Err(other) => unreachable!("query grammar routing hit {other:?}"),
            }
        }
    }
}

pub(super) fn naive_routed_lane<S>(
    run: &mut Run<'_, S>,
    label: &str,
    db: &Db<target::Target>,
    naive: &mut NaiveDb,
    ops: &[differential::Op],
) {
    eprintln!(
        "verify: {} naive-routed {label} cases (Pack — \
         SQLite-inexpressible by the typed gate, enumerated, never silently skipped)",
        ops.len()
    );
    match differential::run(db, naive, ops) {
        Ok(summary) => run.cases += summary.queries,
        Err(divergence) => {
            let bundle = run.out_dir.join(format!("mismatch-{}", run.bundles.len()));
            std::fs::create_dir_all(&bundle).expect("bundle dir");
            std::fs::write(
                bundle.join("mismatch.txt"),
                format!("naive-routed {label} slice diverged:\n{divergence:#?}\n"),
            )
            .expect("bundle");
            eprintln!("verify: NAIVE MISMATCH -> {}", bundle.display());
            run.bundles.push(bundle);
        }
    }
}

pub(super) fn positional(draw: &ParamDraw) -> Vec<ParamValue> {
    let len = draw.scalars.len() + draw.sets.len();
    let mut out: Vec<ParamValue> = vec![ParamValue::Scalar(Value::Bool(false)); len];
    for (param, value) in &draw.scalars {
        out[usize::from(param.0)] = ParamValue::Scalar(value.clone());
    }
    for (param, values) in &draw.sets {
        out[usize::from(param.0)] = ParamValue::Set(values.clone());
    }
    out
}

pub(super) fn load_target_stores(
    dir: &std::path::Path,
    cfg: crate::corpus_gen::GenConfig,
) -> (Db<target::Target>, rusqlite::Connection) {
    let _ = std::fs::remove_dir_all(dir);
    let db = target::publish_admitted(dir);
    let conn = rusqlite::Connection::open_in_memory().expect("target oracle");
    for statement in sqlmap::schema_ddl(target::schema()) {
        conn.execute(&statement, []).expect("target ddl");
    }

    for statement in sqlmap::extension_ddl(&target::descriptor()) {
        conn.execute(&statement, []).expect("target extension");
    }
    for relation in target::schema().relations() {
        let skip_id = usize::from(relation.body().closed_rows().is_some());
        for field in relation.fields().iter().skip(skip_id) {
            let columns = if field.value_type.is_interval() {
                format!("\"{0}_start\", \"{0}_end\"", field.name)
            } else {
                format!("\"{}\"", field.name)
            };
            conn.execute(
                &format!(
                    "CREATE INDEX \"ix_oracle_{}_{}\" ON \"{}\" ({columns})",
                    relation.name(),
                    field.name,
                    relation.name(),
                ),
                [],
            )
            .expect("target oracle index");
        }
    }
    for rel in 0..target::TARGET_RELATIONS {
        let rel = bumbledb::RelationId(rel);
        match rel {
            target::ids::JOURNAL_ENTRY => load_du_cluster(&db, cfg),
            target::ids::IMPORT_BATCH => {}
            _ => {
                db.write(|tx| {
                    tx.insert_dyn(rel, target::corpus_relation_rows(cfg, rel))
                        .map(bumbledb::MutationReport::changed)
                })
                .expect("target insert")
                .unwrap();
            }
        }
        corpus::insert_rows(
            &conn,
            target::schema().relation(rel),
            target::corpus_relation_rows(cfg, rel),
        )
        .expect("target insert");
    }
    conn.execute_batch("ANALYZE").expect("analyze");
    (db, conn)
}

fn load_du_cluster(db: &Db<target::Target>, cfg: crate::corpus_gen::GenConfig) {
    const CHUNK: u64 = 4096;
    let domains = target::Domains::of(cfg.scale);
    let entries = target::corpus_rows(&domains, target::ids::JOURNAL_ENTRY);
    let batches = target::corpus_rows(&domains, target::ids::IMPORT_BATCH);
    let mut next_batch = 0u64;
    let mut start = 0u64;
    while start < entries {
        let end = (start + CHUNK).min(entries);
        db.write(|tx| {
            for i in start..end {
                let row = target::corpus_row(cfg, &domains, target::ids::JOURNAL_ENTRY, i);
                tx.insert_dyn(target::ids::JOURNAL_ENTRY, [&row])?;
            }
            while next_batch < batches && target::import_batch_entry(next_batch) < end {
                let row = target::corpus_row(cfg, &domains, target::ids::IMPORT_BATCH, next_batch);
                tx.insert_dyn(target::ids::IMPORT_BATCH, [&row])?;
                next_batch += 1;
            }
            Ok(())
        })
        .expect("target DU cluster load")
        .unwrap();
        start = end;
    }
}

pub(super) fn case_total(cfg: &VerifyConfig) -> u64 {
    let family_cases: u64 = families::all()
        .iter()
        .map(|f| (f.params)(&cfg.corpus_gen).len() as u64)
        .sum();
    2 * family_cases
        + super::run_calendar::calendar_case_count(cfg)
        + super::run_calendar::calendar_fixed_count(cfg)
        + (u64::from(cfg.random_cases) + u64::from(EMPTY_STORE_RANDOM_CASES)) * 4
}

/// # Errors
/// # Panics
pub fn run_prepared(
    cfg: &VerifyConfig,
    db: &Db<Ledger>,
    conn: &rusqlite::Connection,
    cal_db: &Db<crate::calendar::Scheduling>,
    cal_conn: &rusqlite::Connection,
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

    let mut run = Run {
        db,
        conn,
        out_dir: cfg.out_dir.clone(),
        cases: 0,
        total: case_total(cfg),
        bundles: Vec::new(),
    };

    family_lane(&mut run, cfg, "family", &override_sql);

    if run.bundles.len() < MAX_BUNDLES {
        run.lane(cal_db, cal_conn, |lane| {
            super::run_calendar::calendar_lane(lane, cfg, "calendar", true);
        });
    }

    if run.bundles.len() < MAX_BUNDLES && cfg.random_cases > 0 {
        eprintln!("verify: loading the randomized lane's target corpus");
        let (target_db, target_conn) =
            load_target_stores(&cfg.out_dir.join("target-db"), cfg.corpus_gen);
        let mut naive_routed = Vec::new();
        run.lane(&target_db, &target_conn, |lane| {
            random_lane(
                lane,
                cfg,
                cfg.random_cases,
                0x0112_0001,
                "random",
                |_| {},
                &mut naive_routed,
            );
            if lane.bundles.len() < MAX_BUNDLES {
                eprintln!("verify: converse-property lane");
                super::run_converse::converse_lane(lane, cfg);
            }
        });

        if run.bundles.len() < MAX_BUNDLES && !naive_routed.is_empty() {
            let mut world = crate::conformance::build_world(cfg.corpus_gen.seed);
            naive_routed_lane(
                &mut run,
                "random",
                &world.db,
                &mut world.naive,
                &naive_routed,
            );
        }
    }

    if run.bundles.len() < MAX_BUNDLES {
        run_empty_store(cfg, &mut run);
    }

    if run.bundles.len() < MAX_BUNDLES {
        run_naive_slice(cfg, &mut run);
    }

    if run.bundles.len() < MAX_BUNDLES {
        super::run_calendar::run_calendar_naive(cfg, &mut run);
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
