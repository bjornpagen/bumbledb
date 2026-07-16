use super::{
    Case, Db, EMPTY_STORE_RANDOM_CASES, MAX_BUNDLES, Run, VerifyConfig, VerifyFailure,
    VerifyReport, stamp_value,
};

use bumbledb::Value;

use crate::corpus_gen::Rng;
use crate::families::set_bindings;
use crate::naive::ParamValue;
use crate::querygen::{self, ParamDraw, target};
use crate::schema::{Ledger, schema};
use crate::translate::translate;
use crate::{corpus, families, sqlmap};

use super::run_empty_store::run_empty_store;
use super::run_naive::run_naive_slice;

/// Runs the full oracle: load both store pairs (the ledger corpus for
/// the families, the generator-target corpus for the randomized lane),
/// compare every family × its param draws plus `random_cases` randomized
/// queries × theirs, replay the naive-model differential slice, and
/// stamp on success.
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
        cfg.corpus_gen.seed,
        cfg.corpus_gen.scale.label()
    );
    let db = Db::create(&cfg.out_dir.join("db"), Ledger).expect("create store");
    corpus::load_bumbledb(&db, cfg.corpus_gen).expect("load bumbledb");
    let (conn, _) = corpus::load_sqlite(&cfg.out_dir.join("oracle.sqlite"), cfg.corpus_gen)
        .expect("load oracle");
    eprintln!("verify: loading the calendar corpus");
    let cal_db = Db::create(&cfg.out_dir.join("cal-db"), crate::calendar::Scheduling)
        .expect("create calendar store");
    crate::calendar::corpus::load_bumbledb(&cal_db, cfg.corpus_gen).expect("load calendar");
    let (cal_conn, _) = crate::calendar::corpus::load_sqlite(
        &cfg.out_dir.join("cal-oracle.sqlite"),
        cfg.corpus_gen,
    )
    .expect("load calendar oracle");
    run_prepared(cfg, &db, &conn, &cal_db, &cal_conn, override_sql)
}

impl<S> Run<'_, S> {
    /// Runs one lane against a different store pair: the accumulator's
    /// case count and bundle list flow through the sub-run and back —
    /// the lanes share the harness and differ only in the store pair.
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

/// The family lane: every family × its param draws, per-draw re-rendered
/// SQL (set params embed as literals — prepared-statement parity is not
/// claimed for set-bound families). `override_sql` is the mismatch
/// path's test seam; the empty-store pass passes none.
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

/// The randomized lane: `cases` seeded random queries over the
/// generator's target schema × their four param draws each. `on_query`
/// is the structural hook (the empty-store pass counts gate-bearing
/// queries with it).
pub(super) fn random_lane<S>(
    run: &mut Run<'_, S>,
    cfg: &VerifyConfig,
    cases: u32,
    seed_salt: u64,
    label: &str,
    mut on_query: impl FnMut(&bumbledb::Query),
) {
    let mut rng = Rng::new(cfg.corpus_gen.seed ^ seed_salt);
    'random: for index in 0..cases {
        let query = querygen::random_query(&mut rng, cfg.corpus_gen);
        on_query(&query);
        for draw in querygen::params_for(&query, &mut rng, cfg.corpus_gen) {
            let translated = translate(&query, target::schema(), &draw.sets)
                .expect("generated queries translate");
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
    }
}

/// One randomized draw as positional [`ParamValue`]s (dense `ParamId`s).
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

/// Loads the generator-target corpus (the randomized lane's world —
/// `querygen::target` owns its schema and value functions) into a fresh
/// engine store under `dir` and an in-memory `SQLite` mirror. The
/// mirror gets one index per column (interval halves as a composite):
/// an unindexed oracle turns random joins into minutes of nested loops
/// — this is the correctness lane, never timed, so indexes are pure
/// win. Engine loading is `bulk_load` in declaration order (every
/// containment's target precedes its source), except the
/// discriminated-union cluster: `JournalEntry == ImportBatch` holds in
/// neither one-relation prefix, so the pair loads through joint chunked
/// write transactions ([`load_du_cluster`]).
pub(super) fn load_target_stores(
    dir: &std::path::Path,
    cfg: crate::corpus_gen::GenConfig,
) -> (Db<target::Target>, rusqlite::Connection) {
    let _ = std::fs::remove_dir_all(dir);
    let db = Db::create(dir, target::Target).expect("create target store");
    let conn = rusqlite::Connection::open_in_memory().expect("target oracle");
    for statement in sqlmap::schema_ddl(target::schema()) {
        conn.execute(&statement, []).expect("target ddl");
    }
    // The closed vocabularies' rows are schema surface, not corpus:
    // extension INSERTs ride with the DDL (a closed relation is never
    // empty).
    for statement in sqlmap::extension_ddl(&target::descriptor()) {
        conn.execute(&statement, []).expect("target extension");
    }
    for relation in target::schema().relations() {
        // A closed table's synthetic id is already the PRIMARY KEY;
        // its payload columns get the same per-column indexes as any
        // ordinary table (≤256 rows — pure win, never timed).
        let skip_id = usize::from(relation.is_closed());
        for field in relation.fields().iter().skip(skip_id) {
            let columns = if matches!(
                field.value_type,
                bumbledb::schema::ValueType::Interval { .. }
            ) {
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
            // The DU cluster: entries and their import batches commit
            // together (either alone violates one `==` direction).
            target::ids::JOURNAL_ENTRY => load_du_cluster(&db, cfg),
            target::ids::IMPORT_BATCH => {} // loaded with its entries
            _ => {
                db.bulk_load_dyn(rel, target::corpus_relation_rows(cfg, rel))
                    .expect("target bulk load");
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

/// Loads the `JournalEntry == ImportBatch` cluster in joint chunks:
/// each write transaction inserts a slice of entries plus exactly the
/// `ImportBatch` rows naming entries in that slice
/// (`target::import_batch_entry` — row `k` names entry `3k + 1`), so
/// every commit's final state satisfies both `==` directions.
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
                tx.insert_dyn(target::ids::JOURNAL_ENTRY, &row)?;
            }
            while next_batch < batches && target::import_batch_entry(next_batch) < end {
                let row = target::corpus_row(cfg, &domains, target::ids::IMPORT_BATCH, next_batch);
                tx.insert_dyn(target::ids::IMPORT_BATCH, &row)?;
                next_batch += 1;
            }
            Ok(())
        })
        .expect("target DU cluster load");
        start = end;
    }
}

/// The progress denominator — one owner for the enumerable-in-advance
/// lanes: two passes over the ledger families (loaded + empty store),
/// the calendar roster (randomized draws + fixed rotations), and the
/// randomized lane's draws ((random + empty-store random) × the four
/// draws per query). The converse, error-parity, and naive-slice lanes
/// count per executed comparison, so the COMPLETED run's `cases`
/// exceeds this floor — the README's published oracle count is the
/// completed count, pinned end-to-end by
/// `a_full_verify_at_s_succeeds`.
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

/// The oracle against *pre-loaded* ledger and calendar stores (the
/// CLI's digest-keyed cache path): stale bundles and the stamp are
/// cleared, the stores are left untouched, and bundles/stamp land in
/// `cfg.out_dir`. The randomized lane's target-schema store pair is
/// tool scratch, built here per run.
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

    // The family lane: the ledger corpus.
    family_lane(&mut run, cfg, "family", &override_sql);

    // The calendar family lane (fixed rotations + the randomized
    // slice) against the calendar store pair.
    if run.bundles.len() < MAX_BUNDLES {
        run.lane(cal_db, cal_conn, |lane| {
            super::run_calendar::calendar_lane(lane, cfg, "calendar", true);
        });
    }

    // The randomized lane: seeded random queries over the generator's
    // target schema and its own corpus (the target module carries the
    // coverage extensions the six-type matrix needs), plus the Allen
    // converse-property lane over the same store.
    if run.bundles.len() < MAX_BUNDLES && cfg.random_cases > 0 {
        eprintln!("verify: loading the randomized lane's target corpus");
        let (target_db, target_conn) =
            load_target_stores(&cfg.out_dir.join("target-db"), cfg.corpus_gen);
        run.lane(&target_db, &target_conn, |lane| {
            random_lane(lane, cfg, cfg.random_cases, 0x0112_0001, "random", |_| {});
            if lane.bundles.len() < MAX_BUNDLES {
                eprintln!("verify: converse-property lane");
                super::run_converse::converse_lane(lane, cfg);
            }
        });
    }

    if run.bundles.len() < MAX_BUNDLES {
        run_empty_store(cfg, &mut run);
    }

    // The naive-model lane (docs/architecture/60-validation.md § the two
    // oracles): the unit-scale differential slice — judgment verdicts
    // and family queries against the brute-force model.
    if run.bundles.len() < MAX_BUNDLES {
        run_naive_slice(cfg, &mut run);
    }

    // The calendar naive slice: the second theory's corpus stream, its
    // four judgment-violating deltas, and every calendar family against
    // the brute-force model.
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
