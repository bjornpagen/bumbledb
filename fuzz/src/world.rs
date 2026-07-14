//! The query lanes' shared world (the crucible packet (git ecec1dc3)): the querygen
//! target theory at `Scale::Tiny`, loaded once per (thread, seed) into
//! the three stores the parity oracles compare — the engine, the naive
//! model, and the `SQLite` mirror. The corpus is deterministic in its
//! `GenConfig` and the query lanes never write, so caching across fuzz
//! iterations changes no verdict; only the queries and params are
//! fuzzer-driven per iteration.

use std::cell::RefCell;
use std::rc::Rc;

use bumbledb::schema::ValueType;
use bumbledb::{Db, Error, Query, RelationId};
use bumbledb_bench::corpus_gen::{GenConfig, Scale};
use bumbledb_bench::differential::Answers;
use bumbledb_bench::naive::{Delta, NaiveDb, ParamValue, Tuple};
use bumbledb_bench::querygen::{ParamDraw, target};
use bumbledb_bench::{compare, corpus, families, sqlmap};

use crate::StoreDir;

/// The world seeds: a small fixed set (stores are cached per seed —
/// fuzzer entropy picks among them, so data variation costs no per-
/// iteration rebuild). Tiny scale is the fuzz-iteration point.
pub(crate) const WORLD_SEEDS: [u64; 2] = [0x0113_0001, 0x0113_0002];

/// One seed's [`GenConfig`] — queries, params, and corpus all derive
/// from it (params recompute corpus values, so the seed must match).
pub(crate) fn config(index: usize) -> GenConfig {
    GenConfig {
        seed: WORLD_SEEDS[index],
        scale: Scale::Tiny,
    }
}

/// The three loaded stores of one corpus seed.
pub(crate) struct World {
    pub(crate) db: Db<target::Target>,
    pub(crate) naive: NaiveDb,
    pub(crate) conn: rusqlite::Connection,
    _store: StoreDir,
}

thread_local! {
    static WORLDS: RefCell<[Option<Rc<World>>; WORLD_SEEDS.len()]> =
        const { RefCell::new([None, None]) };
}

/// Runs `f` against the world of seed `index`, building it on first use.
pub(crate) fn with_world<T>(index: usize, f: impl FnOnce(&World) -> T) -> T {
    let world = WORLDS.with(|worlds| {
        let mut worlds = worlds.borrow_mut();
        worlds[index]
            .get_or_insert_with(|| Rc::new(build(config(index))))
            .clone()
    });
    f(&world)
}

/// Loads one Tiny target corpus into all three stores (the engine half
/// mirrors `verify::run::load_target_stores`: bulk loads in declaration
/// order, the discriminated-union cluster in joint chunks so every
/// commit's final state satisfies both `==` directions).
fn build(cfg: GenConfig) -> World {
    let store = StoreDir::new();
    let db = match Db::create(store.path(), target::Target) {
        Ok(db) => db,
        Err(err) => panic!("the target theory must create: {err:?}"),
    };
    let mut naive = NaiveDb::new(&target::descriptor());
    let mut delta = Delta::default();
    let conn = rusqlite::Connection::open_in_memory().expect("open the SQLite mirror");
    for statement in sqlmap::schema_ddl(target::schema()) {
        conn.execute(&statement, []).expect("target ddl");
    }
    // Closed vocabularies are schema surface, not corpus: their ground axioms
    // ride with the DDL.
    for statement in sqlmap::extension_ddl(&target::descriptor()) {
        conn.execute(&statement, []).expect("target extension");
    }
    // One index per column (interval halves composite): the mirror is a
    // correctness oracle, never timed — indexes are pure win.
    for relation in target::schema().relations() {
        let skip_id = usize::from(relation.is_closed());
        for field in relation.fields().iter().skip(skip_id) {
            let columns = if matches!(field.value_type, ValueType::Interval { .. }) {
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
        let rel = RelationId(rel);
        match rel {
            target::ids::JOURNAL_ENTRY => load_du_cluster(&db, cfg),
            target::ids::IMPORT_BATCH => {} // loaded with its entries
            _ => {
                db.bulk_load(rel, target::corpus_relation_rows(cfg, rel))
                    .expect("target bulk load");
            }
        }
        corpus::insert_rows(
            &conn,
            target::schema().relation(rel),
            target::corpus_relation_rows(cfg, rel),
        )
        .expect("target mirror insert");
        for fact in target::corpus_relation_rows(cfg, rel) {
            delta.inserts.push((rel, fact));
        }
    }
    conn.execute_batch("ANALYZE").expect("analyze");
    // The model judges the whole corpus as one delta over the final
    // state — the corpus is valid under every statement by construction.
    naive
        .apply(&delta)
        .expect("the Tiny corpus satisfies the statements");
    World {
        db,
        naive,
        conn,
        _store: store,
    }
}

/// The `JournalEntry == ImportBatch` cluster in joint chunks (the DU
/// `==` statement holds in neither one-relation prefix — the same move
/// as the verify harness's loader).
fn load_du_cluster(db: &Db<target::Target>, cfg: GenConfig) {
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
                let fact = target::corpus_row(cfg, &domains, target::ids::JOURNAL_ENTRY, i);
                tx.insert_dyn(target::ids::JOURNAL_ENTRY, &fact)?;
            }
            while next_batch < batches && target::import_batch_entry(next_batch) < end {
                let fact = target::corpus_row(cfg, &domains, target::ids::IMPORT_BATCH, next_batch);
                tx.insert_dyn(target::ids::IMPORT_BATCH, &fact)?;
                next_batch += 1;
            }
            Ok(())
        })
        .expect("target DU cluster load");
        start = end;
    }
}

/// One engine execution: the differential's [`Answers`] verdict plus — when
/// the query answered — the canonical answer form the `SQLite` lane
/// compares, and the prepared predicate's column types (the decode
/// authority for the oracle's answers).
pub(crate) struct Execution {
    pub(crate) verdict: Answers,
    pub(crate) canonical: Option<Vec<compare::Answer>>,
    pub(crate) types: Vec<ValueType>,
}

/// One query × draw through the REAL public API: prepare, bind, execute,
/// collect. Generated (valid-arm) queries must validate — a prepare
/// refusal here is a finding, not a verdict.
pub(crate) fn execute(db: &Db<target::Target>, query: &Query, params: &[ParamValue]) -> Execution {
    let mut prepared = match db.prepare(query) {
        Ok(prepared) => prepared,
        Err(err) => panic!("a generated query failed to prepare: {err:?}\n{query:#?}"),
    };
    let types: Vec<ValueType> = prepared
        .predicate()
        .columns
        .iter()
        .map(|column| column.ty.clone())
        .collect();
    let args = families::param_args(params);
    match db.read(|snap| snap.execute_collect_args(&mut prepared, &args)) {
        Ok(buffer) => Execution {
            verdict: Answers::Ok(
                buffer
                    .answers()
                    .map(|answer| {
                        Tuple(
                            (0..buffer.arity())
                                .map(|column| owned_value(answer.get(column)))
                                .collect(),
                        )
                    })
                    .collect(),
            ),
            canonical: Some(compare::from_answers(&buffer, &types)),
            types,
        },
        Err(err) => Execution {
            verdict: runtime_refusal(err),
            canonical: None,
            types,
        },
    }
}

/// The naive model's verdict for the same query × draw, in the same
/// comparable shape (typed runtime errors included — error parity).
pub(crate) fn model(naive: &NaiveDb, query: &Query, params: &[ParamValue]) -> Answers {
    use bumbledb_bench::naive::query::QueryError;
    match naive.query(query, params) {
        Ok(answers) => Answers::Ok(answers),
        Err(QueryError::Overflow { .. }) => Answers::Overflow,
        Err(QueryError::MeasureOfRay) => Answers::MeasureOfRay,
    }
}

/// The boundary: a validated query's execution refuses through the two
/// defined runtime errors and nothing else. Every other variant is named
/// — never a catch-all — and is a finding on this path.
fn runtime_refusal(err: Error) -> Answers {
    match err {
        Error::Overflow(_) => Answers::Overflow,
        Error::MeasureOfRay { .. } => Answers::MeasureOfRay,
        other @ (Error::Schema(_)
        | Error::FormatMismatch { .. }
        | Error::SchemaMismatch { .. }
        | Error::AlreadyInitialized
        | Error::EnvironmentLocked
        | Error::Io(_)
        | Error::Lmdb(_)
        | Error::ReadersFull { .. }
        | Error::Validation(_)
        | Error::FactShape(_)
        | Error::CommitRejected { .. }
        | Error::FreshExhausted { .. }
        | Error::ClosedRelationWrite { .. }
        | Error::GenerationMoved { .. }
        | Error::CommitSync { .. }
        | Error::BulkLoad { .. }
        | Error::ForeignPreparedQuery
        | Error::ForeignSnapshot
        | Error::ParamCountMismatch { .. }
        | Error::ParamTypeMismatch { .. }
        | Error::ParamSetExpected { .. }
        | Error::ParamScalarExpected { .. }
        | Error::ParamElementTypeMismatch { .. }
        | Error::PointParamAtCeiling { .. }
        | Error::AllenMaskParamExpected { .. }
        | Error::EmptyAllenMaskParam { .. }
        | Error::FullAllenMaskParam { .. }
        | Error::Corruption(_)
        | Error::ResultBytesOverflow) => {
            panic!("non-runtime error from a generated execution: {other:?}")
        }
    }
}

/// One result value, owned — the world's copy of the differential's
/// total mapping (a new `AnswerValue` variant is a compile error here).
fn owned_value(value: bumbledb::AnswerValue<'_>) -> bumbledb::Value {
    use bumbledb::{AnswerValue, Value};
    match value {
        AnswerValue::Bool(v) => Value::Bool(v),
        AnswerValue::U64(v) => Value::U64(v),
        AnswerValue::I64(v) => Value::I64(v),
        AnswerValue::String(v) => Value::String(Box::from(v.as_bytes())),
        AnswerValue::FixedBytes(v) => Value::FixedBytes(Box::from(v)),
        AnswerValue::IntervalU64(iv) => Value::IntervalU64(iv),
        AnswerValue::IntervalI64(iv) => Value::IntervalI64(iv),
    }
}

/// One randomized draw as positional [`ParamValue`]s (dense `ParamId`s)
/// — the verify harness's `positional`, transcribed.
pub(crate) fn positional(draw: &ParamDraw) -> Vec<ParamValue> {
    let len = draw.scalars.len() + draw.sets.len();
    let mut out: Vec<ParamValue> = vec![ParamValue::Scalar(bumbledb::Value::Bool(false)); len];
    for (param, value) in &draw.scalars {
        out[usize::from(param.0)] = ParamValue::Scalar(value.clone());
    }
    for (param, values) in &draw.sets {
        out[usize::from(param.0)] = ParamValue::Set(values.clone());
    }
    out
}
