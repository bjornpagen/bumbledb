use super::{
    stamp_value, Case, Db, Run, VerifyConfig, VerifyFailure, VerifyReport,
    EMPTY_STORE_RANDOM_CASES, MAX_BUNDLES,
};

use bumbledb::Value;

use crate::families::set_bindings;
use crate::gen::Rng;
use crate::naive::ParamValue;
use crate::querygen::{self, target, ParamDraw};
use crate::schema::{schema, Ledger};
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
        cfg.gen.seed,
        cfg.gen.scale.label()
    );
    let db = Db::create(&cfg.out_dir.join("db"), Ledger).expect("create store");
    corpus::load_bumbledb(&db, cfg.gen).expect("load bumbledb");
    let (conn, _) =
        corpus::load_sqlite(&cfg.out_dir.join("oracle.sqlite"), cfg.gen).expect("load oracle");
    run_prepared(cfg, &db, &conn, override_sql)
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
    cfg: crate::gen::GenConfig,
) -> (Db<target::Target>, rusqlite::Connection) {
    let _ = std::fs::remove_dir_all(dir);
    let db = Db::create(dir, target::Target).expect("create target store");
    let conn = rusqlite::Connection::open_in_memory().expect("target oracle");
    for statement in sqlmap::schema_ddl(target::schema()) {
        conn.execute(&statement, []).expect("target ddl");
    }
    for relation in target::schema().relations() {
        for field in relation.fields() {
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
                db.bulk_load(rel, target::corpus_relation_rows(cfg, rel))
                    .expect("target bulk load");
            }
        }
        let insert = sqlmap::insert_sql(target::schema().relation(rel));
        let mut rows = target::corpus_relation_rows(cfg, rel).peekable();
        while rows.peek().is_some() {
            conn.execute_batch("BEGIN IMMEDIATE").expect("begin");
            {
                let mut stmt = conn.prepare_cached(&insert).expect("prepare");
                for row in rows.by_ref().take(4096) {
                    stmt.execute(rusqlite::params_from_iter(sqlmap::to_sql_row(&row)))
                        .expect("target insert");
                }
            }
            conn.execute_batch("COMMIT").expect("commit");
        }
    }
    conn.execute_batch("ANALYZE").expect("analyze");
    (db, conn)
}

/// Loads the `JournalEntry == ImportBatch` cluster in joint chunks:
/// each write transaction inserts a slice of entries plus exactly the
/// `ImportBatch` rows naming entries in that slice
/// (`target::import_batch_entry` — row `k` names entry `3k + 1`), so
/// every commit's final state satisfies both `==` directions.
fn load_du_cluster(db: &Db<target::Target>, cfg: crate::gen::GenConfig) {
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

/// The oracle against *pre-loaded* ledger stores (the CLI's digest-keyed
/// cache path): stale bundles and the stamp are cleared, the stores are
/// left untouched, and bundles/stamp land in `cfg.out_dir`. The
/// randomized lane's target-schema store pair is tool scratch, built
/// here per run.
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

    // The family lane: the ledger corpus, per-draw re-rendered SQL
    // (set params embed as literals — prepared-statement parity is not
    // claimed for set-bound families).
    'families: for family in families::all() {
        let query = (family.query)();
        for params in (family.params)(&cfg.gen) {
            let translated =
                translate(&query, schema(), &set_bindings(&params)).expect("families translate");
            let sql = override_sql(family.name).unwrap_or_else(|| translated.sql.clone());
            let case = Case {
                label: format!("family {}", family.name),
                query: &query,
                sql: &sql,
                golden_sql: Some(family.golden_sql),
            };
            if !run.check(&case, &translated.params, &params) {
                break 'families;
            }
        }
    }

    // The randomized lane: seeded random queries over the generator's
    // target schema and its own corpus (the target module carries the
    // coverage extensions the seven-type matrix needs).
    if run.bundles.len() < MAX_BUNDLES && cfg.random_cases > 0 {
        eprintln!("verify: loading the randomized lane's target corpus");
        let (target_db, target_conn) = load_target_stores(&cfg.out_dir.join("target-db"), cfg.gen);
        let mut random_run = Run {
            db: &target_db,
            conn: &target_conn,
            out_dir: run.out_dir.clone(),
            cases: run.cases,
            total: run.total,
            bundles: std::mem::take(&mut run.bundles),
        };
        let mut rng = Rng::new(cfg.gen.seed ^ 0x0112_0001);
        'random: for index in 0..cfg.random_cases {
            let query = querygen::random_query(&mut rng, cfg.gen);
            for draw in querygen::params_for(&query, &mut rng, cfg.gen) {
                let translated = translate(&query, target::schema(), &draw.sets)
                    .expect("generated queries translate");
                let case = Case {
                    label: format!("random {index}"),
                    query: &query,
                    sql: &translated.sql,
                    golden_sql: None,
                };
                if !random_run.check(&case, &translated.params, &positional(&draw)) {
                    break 'random;
                }
            }
        }
        run.cases = random_run.cases;
        run.bundles = random_run.bundles;
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
