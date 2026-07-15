//! The shared fuzz harness (docs/architecture/60-validation.md § the
//! fuzzing charter): fuzzer bytes → [`Rng::from_bytes`] → generation in
//! `bumbledb-bench`'s `corpus_gen` → a scenario runner returning typed
//! verdicts. Each target in `fuzz_targets/` is one thin `fuzz_target!`
//! call into one runner here; the harness owns no logic worth fuzzing
//! (refusal: we do not fuzz the harness).
//!
//! Error matches in this crate are TOTAL — zero catch-all arms over
//! engine error enums, so a future variant addition is a compile error
//! here: the matcher is itself a census instrument.

pub mod crash;
pub mod query;
pub mod rewrites;
pub(crate) mod world;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use bumbledb::error::SchemaError;
use bumbledb::schema::SchemaDescriptor;
use bumbledb::schema::fingerprint::{self, SchemaFingerprint};
use bumbledb::{AnswerValue, Db, Error, PreparedQuery, Query, RelationId, Value};
use bumbledb_bench::corpus_gen::Rng;
use bumbledb_bench::corpus_gen::opgen::{self, FuzzOp, OpScenario};
use bumbledb_bench::corpus_gen::theorygen;
use bumbledb_bench::differential::{self, Answers, Op, Verdict as WriteVerdict};
use bumbledb_bench::families;
use bumbledb_bench::naive::query::QueryError;
use bumbledb_bench::naive::{Delta, NaiveDb, ParamValue, Tuple};
use bumbledb_bench::querygen::target;

/// The theory target: fuzzer bytes → a structurally-free
/// [`SchemaDescriptor`] (the random-descriptor arm — deliberately-invalid
/// shapes alongside valid ones) → the engine's acceptance judgment, under
/// three oracles. Oracle 1 (no-panic totality) is the run itself: any
/// panic below — engine or harness assert — is a finding by definition.
pub fn theory(data: &[u8]) {
    let mut rng = Rng::from_bytes(data);
    let descriptor = theorygen::random_descriptor(&mut rng);

    theory_oracles(&descriptor);

    // The arity arm is an extension: every legacy decision and all three
    // legacy oracles have completed before the first new entropy draw.
    let mut arity_rng = Rng::from_bytes(data);
    let case = theorygen::random_arity_descriptor(&mut arity_rng);
    let verdict = theory_oracles(&case.descriptor);
    assert_arity_expectation(case.coverage.expectation, &verdict);
    record_arity(ArityLane::Theory, case.coverage);
}

fn theory_oracles(descriptor: &SchemaDescriptor) -> Verdict {
    // Oracle 3: judgment determinism — the same descriptor against two
    // fresh stores yields the identical verdict (rejections compared
    // payload-exact, acceptances by schema fingerprint).
    let store = StoreDir::new();
    let first = judge(descriptor, store.path());
    let twin = StoreDir::new();
    let second = judge(descriptor, twin.path());
    assert_eq!(first, second, "judgment determinism");

    if let Verdict::Accepted(created) = &first {
        // Oracle 3, continued: an accepted schema re-opens cleanly on the
        // store it created, to the same sealed theory, and `verify_store`
        // passes on the empty store.
        let db = match Db::open(store.path(), descriptor.clone()) {
            Ok(db) => db,
            Err(err) => panic!("accepted schema failed to reopen: {err:?}"),
        };
        let report = match db.verify_store() {
            Ok(report) => report,
            Err(err) => panic!("verify_store errored on a fresh store: {err:?}"),
        };
        assert!(
            report.findings.is_empty(),
            "fresh store of an accepted schema has findings: {:?} (fingerprint {created:?})",
            report.findings
        );
    }
    first
}

/// The ops runner — the flagship lifecycle target
/// (the crucible packet (git ecec1dc3)): fuzzer bytes → one generated op
/// sequence over the querygen target theory (`corpus_gen::opgen`, Tiny
/// scale) → the live engine with the naive model in lockstep, the
/// two-oracle discipline extended over TIME.
///
/// Model mapping (what each verb means on either side):
///
/// | verb | engine | naive model |
/// |---|---|---|
/// | insert/delete/mixed batch | staged into the pending delta (batching is the transaction) | the same pending delta |
/// | commit | one write tx applies the pending delta; the dependency judgment fires | [`NaiveDb::apply`] — an abort leaves the model untouched by construction |
/// | rollback | the delta runs inside a write closure that returns `Err` (the documented abandon: the delta drops, LMDB untouched) | "don't apply" — the pending delta is discarded |
/// | execute | a pooled [`PreparedQuery`] with live params | [`NaiveDb::query`] with the same params |
/// | re-prepare | [`Db::prepare`] replaces the pool slot | no-op — the model holds no prepared state |
/// | view read | a snapshot [`Snapshot::scan`](bumbledb::Snapshot::scan) of one ordinary relation | [`NaiveDb::relation`] |
/// | reopen | the env drops; [`Db::open`] from disk; the pool re-prepares, the pending delta dies with the env | no-op on state — a reopen changes nothing |
/// | verify_store | the store's internal auditor | no verb — the model trusts itself |
///
/// The closed relations (`Currency`/`Source`/`Tag`) are ground axioms:
/// their write surface is the closed-case arm of the generator, and
/// their contents are schema, not store state, so the view-read and
/// full-contents comparisons range over the ordinary relations.
///
/// Five oracles, beyond the standing no-panic totality:
/// 1. **Verdict parity** per commit — accept/reject matches the naive
///    judgment by STRICT EQUALITY of the COMPLETE violation sets,
///    order included: a rejection IS the sealed set (every violated
///    statement, cited once per direction, in materialized statement
///    order) on both sides, so the engine's `CommitRejected` payload
///    (mapped to typed identities by [`differential::cited`]) must
///    equal [`NaiveDb::apply`]'s rejection exactly. The campaign's
///    first ops finding (the multi-violation citation gap,
///    the crucible packet (git ecec1dc3) § conflict) is resolved by this
///    representation — the interim set-membership oracle collapsed
///    back to equality per its reactivation clause, and the pinned
///    trophy now replays the ORDER end-to-end.
/// 2. **Query parity** per execution — set-semantic result equality,
///    the differential's comparator ([`Answers`]).
/// 3. **Reopen equivalence** — after every reopen, every ordinary
///    relation's full contents equal the model's.
/// 4. **`verify_store` green** after every commit and every reopen
///    (and on the drawn verb) — the auditor agrees continuously.
/// 5. **Rejected commits change nothing** — after a judged rejection
///    (and after every rollback), full contents equal the model's
///    untouched state.
pub fn ops(data: &[u8]) {
    let _note = ReplayNote::new(data);
    let mut rng = Rng::from_bytes(data);
    let scenario = opgen::random_scenario(&mut rng);
    let store = StoreDir::new();
    let mut naive = NaiveDb::new(&target::descriptor());
    // Reopen is an epoch boundary: each segment runs against its own
    // freshly opened env, so the prepared pool's borrows never outlive
    // the `Db` they were prepared on.
    let mut segments = scenario.ops.split(|op| matches!(op, FuzzOp::Reopen));
    let first = segments.next().expect("split yields at least one segment");
    {
        let db = match Db::create(store.path(), target::Target) {
            Ok(db) => db,
            Err(err) => panic!("the target theory must create: {err:?}"),
        };
        epoch(&db, &mut naive, &scenario, first);
    }
    for segment in segments {
        let db = match Db::open(store.path(), target::Target) {
            Ok(db) => db,
            Err(err) => panic!("reopen from disk failed: {err:?}"),
        };
        // Oracles 3 and 4: the reopen changed nothing, and the store's
        // own auditor agrees.
        assert_contents(&db, &naive, "reopen");
        assert_green(&db, "reopen");
        epoch(&db, &mut naive, &scenario, segment);
    }

    // As in the theory lane, the existing scenario's generation and replay
    // finish before the extension consumes entropy.
    let mut arity_rng = Rng::from_bytes(data);
    let case = theorygen::random_valid_arity_ops(&mut arity_rng);
    arity_ops(case);
}

fn arity_ops(case: theorygen::ArityOpsCase) {
    let store = StoreDir::new();
    let db = match Db::create(store.path(), case.descriptor.clone()) {
        Ok(db) => db,
        Err(err) => panic!("valid arity theory failed to create: {err:?}"),
    };
    let mut naive = NaiveDb::new(&case.descriptor);
    let ops: Vec<Op> = case.deltas.into_iter().map(Op::Write).collect();
    let summary = differential::run(&db, &mut naive, &ops)
        .unwrap_or_else(|finding| panic!("high-arity verdict divergence: {finding:?}"));
    assert_eq!(summary.commits, 1, "the matching seed pair must commit");
    assert_eq!(summary.aborts, 3, "all three enforcement probes must abort");
    assert_green(&db, "high-arity parity stream");
    record_arity(ArityLane::Ops, case.coverage);
}

/// One epoch: the ops between two reopens, against one live env. The
/// pending delta and the prepared pool are epoch state — both die with
/// the env, exactly as the mapping table states.
fn epoch(db: &Db<target::Target>, naive: &mut NaiveDb, scenario: &OpScenario, ops: &[FuzzOp]) {
    let mut pool: Vec<PreparedQuery<'_, target::Target>> = scenario
        .queries
        .iter()
        .map(|query| prepare(db, query))
        .collect();
    let mut pending = Delta::default();
    for op in ops {
        match op {
            FuzzOp::InsertBatch(delta) | FuzzOp::DeleteBatch(delta) | FuzzOp::MixedBatch(delta) => {
                pending.deletes.extend(delta.deletes.iter().cloned());
                pending.inserts.extend(delta.inserts.iter().cloned());
            }
            FuzzOp::Commit => {
                let delta = std::mem::take(&mut pending);
                // Oracle 1: one write judged on both sides — verdict
                // and the COMPLETE violation set, compared by strict
                // equality (order included): both sides derive the same
                // sealed total object, so any difference — membership,
                // multiplicity, or order — is a finding.
                let engine = engine_write(db, &delta);
                let model = match naive.apply(&delta) {
                    Ok(()) => WriteVerdict::Committed,
                    Err(violations) => WriteVerdict::Aborted(violations),
                };
                assert_eq!(engine, model, "commit verdict divergence");
                // Oracle 4: green after every commit, either verdict.
                assert_green(db, "commit");
                if matches!(engine, WriteVerdict::Aborted(_)) {
                    // Oracle 5: a judged rejection changed nothing.
                    assert_contents(db, naive, "rejected commit");
                }
            }
            FuzzOp::Rollback => {
                let delta = std::mem::take(&mut pending);
                let abandoned: Result<(), Error> = db.write(|tx| {
                    for (rel, fact) in &delta.deletes {
                        tx.delete_dyn(*rel, fact)?;
                    }
                    for (rel, fact) in &delta.inserts {
                        tx.insert_dyn(*rel, fact)?;
                    }
                    Err(Error::Io(std::io::Error::other("deliberate abandon")))
                });
                assert!(abandoned.is_err(), "an abandoned write cannot commit");
                // Oracle 5's sibling: the abandon changed nothing.
                assert_contents(db, naive, "rollback");
            }
            FuzzOp::Execute { slot, params } => {
                // Oracle 2: set-semantic result parity, typed runtime
                // errors included.
                let engine = execute(db, &mut pool[*slot], params);
                let model = match naive.query(&scenario.queries[*slot], params) {
                    Ok(answers) => Answers::Ok(answers),
                    Err(QueryError::Overflow { .. }) => Answers::Overflow,
                    Err(QueryError::MeasureOfRay) => Answers::MeasureOfRay,
                };
                assert_eq!(engine, model, "query parity (pool slot {slot})");
            }
            FuzzOp::Reprepare { slot } => pool[*slot] = prepare(db, &scenario.queries[*slot]),
            FuzzOp::ViewRead { relation } => {
                let contents = scan(db, *relation);
                assert_eq!(
                    &contents,
                    naive.relation(*relation),
                    "view read diverges (relation {})",
                    relation.0
                );
            }
            FuzzOp::VerifyStore => assert_green(db, "the drawn verify_store"),
            FuzzOp::Reopen => unreachable!("reopen is an epoch boundary"),
        }
    }
    // A pending delta staged but never committed dies with the epoch's
    // env — neither side applies it.
}

/// One delta through the engine's write path — deletes then inserts,
/// the same order [`NaiveDb::apply`] replays, mapped to the shared
/// [`WriteVerdict`]: the rejection's sealed violation set carries over
/// whole ([`bumbledb_bench::differential::cited`]). The two judgment
/// refusals are the legal aborts; every other variant is named — never
/// a catch-all — and is a finding on this path.
fn engine_write(db: &Db<target::Target>, delta: &Delta) -> WriteVerdict {
    use bumbledb_bench::naive::Violation;
    let outcome = db.write(|tx| {
        for (rel, fact) in &delta.deletes {
            tx.delete_dyn(*rel, fact)?;
        }
        for (rel, fact) in &delta.inserts {
            tx.insert_dyn(*rel, fact)?;
        }
        Ok(())
    });
    match outcome {
        Ok(()) => WriteVerdict::Committed,
        Err(Error::CommitRejected { violations }) => {
            WriteVerdict::Aborted(bumbledb_bench::differential::cited(&violations))
        }
        Err(Error::ClosedRelationWrite { relation }) => {
            WriteVerdict::Aborted(vec![Violation::ClosedRelationWrite { relation }])
        }
        Err(
            other @ (Error::Schema(_)
            | Error::FormatMismatch { .. }
            | Error::SchemaMismatch { .. }
            | Error::AlreadyInitialized
            | Error::EnvironmentLocked
            | Error::StoreKindMismatch { .. }
            | Error::Io(_)
            | Error::Lmdb(_)
            | Error::ReadersFull { .. }
            | Error::Validation(_)
            | Error::FactShape(_)
            | Error::FreshExhausted { .. }
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
            | Error::MeasureOfRay { .. }
            | Error::FixpointBudgetExceeded { .. }
            | Error::Overflow(_)
            | Error::ResultBytesOverflow
            | Error::Corruption(_)),
        ) => panic!("non-judgment error from a generated write: {other:?}"),
    }
}

/// One prepared execution as a [`Answers`] verdict — the pooled sibling of
/// the differential's per-op query path (that one re-prepares; this one
/// exercises the prepared-state lifecycle).
fn execute(
    db: &Db<target::Target>,
    prepared: &mut PreparedQuery<'_, target::Target>,
    params: &[ParamValue],
) -> Answers {
    let args = families::param_args(params);
    match db.read(|snap| snap.execute_collect_args(prepared, &args)) {
        Ok(buffer) => Answers::Ok(
            buffer
                .answers()
                .map(|answer| {
                    Tuple(
                        (0..buffer.arity())
                            .map(|column| owned(answer.get(column)))
                            .collect(),
                    )
                })
                .collect(),
        ),
        Err(err) => query_refusal(err),
    }
}

/// The boundary: a generated query's execution refuses through the two
/// defined runtime errors and nothing else. Every other variant is
/// named — never a catch-all — and is a finding on this path.
fn query_refusal(err: Error) -> Answers {
    match err {
        Error::Overflow(_) => Answers::Overflow,
        Error::MeasureOfRay { .. } => Answers::MeasureOfRay,
        // The budget trip is a typed execution error on `MeasureOfRay`'s
        // model — typed identity, never a harness crash (the naive
        // fixpoint is unbudgeted, so a trip surfaces as a readable
        // parity divergence).
        Error::FixpointBudgetExceeded { .. } => Answers::FixpointBudget,
        other @ (Error::Schema(_)
        | Error::FormatMismatch { .. }
        | Error::SchemaMismatch { .. }
        | Error::AlreadyInitialized
        | Error::EnvironmentLocked
        | Error::StoreKindMismatch { .. }
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

/// One result value, owned — the harness's copy of the differential's
/// total mapping (a new `AnswerValue` variant is a compile error here).
fn owned(value: AnswerValue<'_>) -> Value {
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

/// One relation's full committed contents through the export scan.
fn scan(db: &Db<target::Target>, rel: RelationId) -> BTreeSet<Tuple> {
    let outcome = db.read(|snap| {
        let mut set = BTreeSet::new();
        for fact in snap.scan(rel)? {
            set.insert(Tuple(fact?));
        }
        Ok(set)
    });
    match outcome {
        Ok(set) => set,
        Err(err) => panic!("a full-relation scan refused: {err:?}"),
    }
}

/// Oracles 3 and 5: every ordinary relation's full contents equal the
/// model's, compared whole.
fn assert_contents(db: &Db<target::Target>, naive: &NaiveDb, when: &str) {
    for rel in 0..target::TARGET_RELATIONS {
        let rel = RelationId(rel);
        let engine = scan(db, rel);
        assert_eq!(
            &engine,
            naive.relation(rel),
            "contents diverge after {when} (relation {})",
            rel.0
        );
    }
}

/// Oracle 4: the store's own internal auditor agrees, continuously.
fn assert_green<S>(db: &Db<S>, when: &str) {
    let report = match db.verify_store() {
        Ok(report) => report,
        Err(err) => panic!("verify_store errored after {when}: {err:?}"),
    };
    assert!(
        report.findings.is_empty(),
        "verify_store findings after {when}: {:?}",
        report.findings
    );
}

fn prepare<'db>(db: &'db Db<target::Target>, query: &Query) -> PreparedQuery<'db, target::Target> {
    match db.prepare(query) {
        Ok(prepared) => prepared,
        Err(err) => panic!("a generated query failed to prepare: {err:?}"),
    }
}

/// Prints the failing input's identity when a panic unwinds through the
/// runner — the whole run derives from the byte string, so the saved
/// artifact replays it exactly: `cargo fuzz run ops <artifact>`.
struct ReplayNote {
    len: usize,
    fnv: u64,
}

impl ReplayNote {
    fn new(data: &[u8]) -> Self {
        let mut fnv = 0xCBF2_9CE4_8422_2325_u64;
        for byte in data {
            fnv ^= u64::from(*byte);
            fnv = fnv.wrapping_mul(0x0000_0100_0000_01B3);
        }
        Self {
            len: data.len(),
            fnv,
        }
    }
}

impl Drop for ReplayNote {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!(
                "ops finding: input of {} bytes, fnv1a {:016x} — the saved artifact replays it: \
                 cargo fuzz run ops <artifact>",
                self.len, self.fnv
            );
        }
    }
}

/// The engine's judgment of one descriptor, as a comparable value.
#[derive(Debug, PartialEq)]
enum Verdict {
    /// Accepted: the sealed schema's fingerprint.
    Accepted(SchemaFingerprint),
    /// Rejected: the variant name (the total-match token) plus the full
    /// typed payload.
    Rejected(&'static str, SchemaError),
}

fn assert_arity_expectation(expectation: theorygen::ArityExpectation, verdict: &Verdict) {
    use theorygen::ArityExpectation;
    match (expectation, verdict) {
        (ArityExpectation::Accepted, Verdict::Accepted(_)) => {}
        (
            ArityExpectation::DeterminantKeyTooWide { width: expected },
            Verdict::Rejected(
                "DeterminantKeyTooWide",
                SchemaError::DeterminantKeyTooWide { width, .. },
            ),
        ) if *width == expected => {}
        (
            ArityExpectation::MissingSourceKey,
            Verdict::Rejected(
                "NoMatchingTargetKey",
                SchemaError::NoMatchingTargetKey { target, .. },
            ),
        ) if *target == RelationId(0) => {}
        (
            ArityExpectation::MissingTargetKey,
            Verdict::Rejected(
                "NoMatchingTargetKey",
                SchemaError::NoMatchingTargetKey { target, .. },
            ),
        ) if *target == RelationId(1) => {}
        _ => panic!("arity-case verdict mismatch: expected {expectation:?}, got {verdict:?}"),
    }
}

#[derive(Debug, Clone, Copy)]
enum ArityLane {
    Theory,
    Ops,
}

#[derive(Debug, Default)]
struct ArityHistogram {
    runs: u64,
    arities: [u64; theorygen::MAX_MIXED_ARITY + 2],
    type_occurrences: [u64; 5],
    selections: [u64; 3],
    equalities: u64,
    reordered_keys: u64,
    width_rejections: u64,
    missing_source_keys: u64,
    missing_target_keys: u64,
}

static THEORY_ARITY_HISTOGRAM: OnceLock<Mutex<ArityHistogram>> = OnceLock::new();
static OPS_ARITY_HISTOGRAM: OnceLock<Mutex<ArityHistogram>> = OnceLock::new();

fn record_arity(lane: ArityLane, coverage: theorygen::ArityCoverage) {
    let histogram = match lane {
        ArityLane::Theory => &THEORY_ARITY_HISTOGRAM,
        ArityLane::Ops => &OPS_ARITY_HISTOGRAM,
    };
    let mut histogram = histogram
        .get_or_init(|| Mutex::new(ArityHistogram::default()))
        .lock()
        .expect("arity histogram lock");
    histogram.runs += 1;
    histogram.arities[coverage.arity] += 1;
    for (total, count) in histogram
        .type_occurrences
        .iter_mut()
        .zip(coverage.type_counts)
    {
        *total += count as u64;
    }
    let selection = match coverage.selection {
        theorygen::SelectionPlacement::Source => 0,
        theorygen::SelectionPlacement::Target => 1,
        theorygen::SelectionPlacement::Both => 2,
    };
    histogram.selections[selection] += 1;
    histogram.equalities += u64::from(coverage.equality);
    histogram.reordered_keys += u64::from(coverage.reordered_key);
    match coverage.expectation {
        theorygen::ArityExpectation::Accepted => {}
        theorygen::ArityExpectation::DeterminantKeyTooWide { .. } => {
            histogram.width_rejections += 1;
        }
        theorygen::ArityExpectation::MissingSourceKey => histogram.missing_source_keys += 1,
        theorygen::ArityExpectation::MissingTargetKey => histogram.missing_target_keys += 1,
    }
    if histogram.runs % 5_000 == 0 {
        eprintln!("{lane:?} arity histogram: {histogram:?}");
    }
}

/// One acceptance pass through the REAL public API: `Db::create` on a
/// fresh directory. An accepted descriptor must also validate standalone
/// (the fingerprint's source); disagreement between the two entry points
/// is a finding.
fn judge(descriptor: &SchemaDescriptor, dir: &Path) -> Verdict {
    match Db::create(dir, descriptor.clone()) {
        Ok(db) => {
            drop(db);
            let schema = match descriptor.clone().validate() {
                Ok(schema) => schema,
                Err(err) => panic!("Db::create accepted what validate rejects: {err:?}"),
            };
            Verdict::Accepted(fingerprint::fingerprint(&schema))
        }
        Err(err) => {
            let (token, rejection) = schema_rejection(err);
            Verdict::Rejected(token, rejection)
        }
    }
}

/// Oracle 2, the boundary: schema acceptance rejects through
/// `Error::Schema` and nothing else. Every other variant is named — never
/// a catch-all — and is a finding on this path.
fn schema_rejection(err: Error) -> (&'static str, SchemaError) {
    match err {
        Error::Schema(rejection) => (schema_variant(&rejection), rejection),
        other @ (Error::FormatMismatch { .. }
        | Error::SchemaMismatch { .. }
        | Error::AlreadyInitialized
        | Error::EnvironmentLocked
        | Error::StoreKindMismatch { .. }
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
        | Error::MeasureOfRay { .. }
        | Error::FixpointBudgetExceeded { .. }
        | Error::Overflow(_)
        | Error::ResultBytesOverflow
        | Error::Corruption(_)) => {
            panic!("non-schema error from schema acceptance: {other:?}")
        }
    }
}

/// Oracle 2, the census: every rejection is a NAMED `SchemaError` variant.
/// Total match, zero catch-alls — a new variant is a compile error here.
fn schema_variant(rejection: &SchemaError) -> &'static str {
    match rejection {
        SchemaError::DuplicateRelationName { .. } => "DuplicateRelationName",
        SchemaError::DuplicateFieldName { .. } => "DuplicateFieldName",
        SchemaError::FreshOnNonU64 { .. } => "FreshOnNonU64",
        SchemaError::FixedBytesWidthOutOfRange { .. } => "FixedBytesWidthOutOfRange",
        SchemaError::IntervalWidthOutOfRange { .. } => "IntervalWidthOutOfRange",
        SchemaError::RelationTooManyColumns { .. } => "RelationTooManyColumns",
        SchemaError::TooManyStatements { .. } => "TooManyStatements",
        SchemaError::EmptyExtension { .. } => "EmptyExtension",
        SchemaError::ExtensionTooManyRows { .. } => "ExtensionTooManyRows",
        SchemaError::DuplicateExtensionHandle { .. } => "DuplicateExtensionHandle",
        SchemaError::ExtensionArityMismatch { .. } => "ExtensionArityMismatch",
        SchemaError::ExtensionValueTypeMismatch { .. } => "ExtensionValueTypeMismatch",
        SchemaError::ExtensionIntervalRay { .. } => "ExtensionIntervalRay",
        SchemaError::StrOnClosedRelation { .. } => "StrOnClosedRelation",
        SchemaError::FreshOnClosedRelation { .. } => "FreshOnClosedRelation",
        SchemaError::StatementUnknownRelation { .. } => "StatementUnknownRelation",
        SchemaError::StatementUnknownField { .. } => "StatementUnknownField",
        SchemaError::EmptyProjection { .. } => "EmptyProjection",
        SchemaError::DuplicateProjectionField { .. } => "DuplicateProjectionField",
        SchemaError::DuplicateSelectionField { .. } => "DuplicateSelectionField",
        SchemaError::DegenerateSelectionSet { .. } => "DegenerateSelectionSet",
        SchemaError::DuplicateSelectionLiteral { .. } => "DuplicateSelectionLiteral",
        SchemaError::CardinalityIntervalPosition { .. } => "CardinalityIntervalPosition",
        SchemaError::FunctionalityMultipleIntervals { .. } => "FunctionalityMultipleIntervals",
        SchemaError::FunctionalityIntervalNotLast { .. } => "FunctionalityIntervalNotLast",
        SchemaError::DuplicateFunctionality { .. } => "DuplicateFunctionality",
        SchemaError::DeterminantKeyTooWide { .. } => "DeterminantKeyTooWide",
        SchemaError::ContainmentArityMismatch { .. } => "ContainmentArityMismatch",
        SchemaError::ContainmentTypeMismatch { .. } => "ContainmentTypeMismatch",
        SchemaError::SelectedFieldProjected { .. } => "SelectedFieldProjected",
        SchemaError::SelectionLiteralTypeMismatch { .. } => "SelectionLiteralTypeMismatch",
        SchemaError::SelectionLiteralNotUtf8 { .. } => "SelectionLiteralNotUtf8",
        SchemaError::NoMatchingTargetKey { .. } => "NoMatchingTargetKey",
        SchemaError::NoPointwiseTargetKey { .. } => "NoPointwiseTargetKey",
        SchemaError::ClosedContainmentInterval { .. } => "ClosedContainmentInterval",
        SchemaError::ClosedStatementRefuted { .. } => "ClosedStatementRefuted",
        SchemaError::DuplicateStatement { .. } => "DuplicateStatement",
    }
}

/// A per-iteration LMDB store directory under the system temp root:
/// created fresh, removed on drop — Tiny-scale stores keep this cheap.
/// (The query/rewrites worlds hold one for the process lifetime.)
pub(crate) struct StoreDir(PathBuf);

static STORE_SEQ: AtomicU64 = AtomicU64::new(0);

impl StoreDir {
    pub(crate) fn new() -> Self {
        let seq = STORE_SEQ.fetch_add(1, Ordering::Relaxed);
        // BUMBLEDB_SCRATCH_DIR points the fuzz lanes' scratch stores at
        // a RAM-backed volume (`scripts/ramdisk.sh`,
        // docs/architecture/60-validation.md § the ramdisk sanction) —
        // the verify/fuzz lanes check answers, not clocks, so RAM is
        // sanctioned for them. Unset, the system temp dir stands.
        let root =
            std::env::var_os("BUMBLEDB_SCRATCH_DIR").map_or_else(std::env::temp_dir, PathBuf::from);
        let path = root.join(format!("bumbledb-fuzz-{}-{seq}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create fuzz store dir");
        Self(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for StoreDir {
    fn drop(&mut self) {
        // Truncate the data file before unlinking it. An EPHEMERAL
        // store's data.mdb was ftruncated to the full 4 GiB map
        // (`MDB_WRITEMAP`) and its dirty pages outlive the close in the
        // page cache — a plain unlink on a non-sparse volume (the HFS+
        // ramdisk, `scripts/ramdisk.sh`) then frees the blocks
        // ASYNCHRONOUSLY, seconds later, so back-to-back crash-sweep
        // cases outrun reclamation and die StorageFull even on a volume
        // that comfortably holds one store (the fixit record).
        // `set_len(0)` discards the dirty pages and frees the blocks
        // synchronously; on durable stores and non-RAM volumes it is a
        // harmless no-op-sized truncate.
        if let Ok(file) = std::fs::OpenOptions::new()
            .write(true)
            .open(self.0.join("data.mdb"))
        {
            let _ = file.set_len(0);
        }
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
