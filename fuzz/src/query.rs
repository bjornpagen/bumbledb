//! The query target (the crucible packet (git ecec1dc3)): three-way parity per
//! iteration. The valid arm draws a querygen query (valid **by
//! construction**) over a cached Tiny world and compares the prepared
//! engine, the naive model, and — where the shape is expressible — the
//! `SQLite` lane (the differential's ψ-subset mapping; inexpressible
//! shapes drop to two-way, counted and logged, never silently). The
//! hostile arm draws structurally-free IR (`corpus_gen::irgen`) for the
//! validation-totality oracle: typed rejection, no panic, deterministic
//! verdicts.
//!
//! Oracles: result-set equality across all lanes; prepare/execute
//! determinism (the same query × draw twice → identical rows); the
//! hostile arm's typed-rejection census (a TOTAL match — a new
//! `ValidationError` variant is a compile error here).

use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::error::ValidationError;
use bumbledb::{Db, Error, Query};
use bumbledb_bench::compare;
use bumbledb_bench::corpus_gen::{Rng, irgen};
use bumbledb_bench::querygen;
use bumbledb_bench::querygen::target;
use bumbledb_bench::translate::{LaneCase, sqlite_expressible, translate};

use crate::world::{self, WORLD_SEEDS, with_world};

/// The SQLite-lane coverage counters: three-way comparisons, shapes the
/// mapping cannot express (`Pack` heads), and typed-error outcomes the
/// oracle has no twin for (the naive lane already matched them typed-
/// identically). Logged every [`LOG_EVERY`] draws — the drop is counted,
/// never silent.
static THREE_WAY: AtomicU64 = AtomicU64::new(0);
static INEXPRESSIBLE: AtomicU64 = AtomicU64::new(0);
static ERROR_OUTCOMES: AtomicU64 = AtomicU64::new(0);

const LOG_EVERY: u64 = 10_000;

/// The query runner: one fuzz iteration.
pub fn run(data: &[u8]) {
    let mut rng = Rng::from_bytes(data);
    // One draw in four is hostile — enough weight that the validation
    // boundary sees constant fire without starving the parity lanes.
    if rng.chance(1, 4) {
        hostile(&mut rng);
        return;
    }
    let index = usize::try_from(rng.range(WORLD_SEEDS.len() as u64)).expect("index fits usize");
    let cfg = world::config(index);
    let query = querygen::random_query(&mut rng, cfg);
    let draws = querygen::params_for(&query, &mut rng, cfg);
    with_world(index, |world| {
        for draw in draws {
            let params = world::positional(&draw);
            let first = world::execute(&world.db, &query, &params);
            // Oracle: prepare/execute determinism — the same query and
            // draw through a second prepare yield identical rows.
            let second = world::execute(&world.db, &query, &params);
            assert_eq!(
                first.verdict, second.verdict,
                "prepare/execute determinism: {query:#?}"
            );
            // Oracle: engine vs the naive model, typed errors included.
            let model = world::model(&world.naive, &query, &params);
            assert_eq!(
                first.verdict, model,
                "engine and model disagree: {query:#?}\nparams: {params:#?}"
            );
            // The SQLite lane, where the ψ-subset mapping expresses the
            // shape — the counted drops are Pack heads and typed-error
            // outcomes (SQL has no typed twin; the naive lane already
            // agreed on those payload-exactly).
            match sqlite_expressible(&LaneCase::Query(&query)) {
                Err(_) => {
                    INEXPRESSIBLE.fetch_add(1, Ordering::Relaxed);
                }
                Ok(()) => match &first.canonical {
                    None => {
                        ERROR_OUTCOMES.fetch_add(1, Ordering::Relaxed);
                    }
                    Some(ours) => {
                        THREE_WAY.fetch_add(1, Ordering::Relaxed);
                        let translated = translate(&query, target::schema(), &draw.sets)
                            .expect("expressible generated queries translate");
                        let mut stmt = world
                            .conn
                            .prepare_cached(&translated.sql)
                            .expect("the oracle prepares translated SQL");
                        let theirs = compare::from_sqlite(
                            &mut stmt,
                            &translated.params,
                            &params,
                            &first.types,
                        )
                        .unwrap_or_else(|err| {
                            panic!(
                                "the oracle errored where the engine answered: {err}\n\
                                         {query:#?}\n{}",
                                translated.sql
                            )
                        });
                        if let Err(mismatch) = compare::multisets(ours.clone(), theirs) {
                            panic!(
                                "engine and SQLite disagree: {mismatch}\n{query:#?}\n{}",
                                translated.sql
                            );
                        }
                    }
                },
            }
            log_ratio();
        }
    });
}

/// The session ratio, printed every [`LOG_EVERY`] draws: if the
/// three-way lane covers under half the generated shapes, that is a
/// generator-bias note for the human register, never a silent drop.
fn log_ratio() {
    let three = THREE_WAY.load(Ordering::Relaxed);
    let inexpressible = INEXPRESSIBLE.load(Ordering::Relaxed);
    let errors = ERROR_OUTCOMES.load(Ordering::Relaxed);
    let total = three + inexpressible + errors;
    if total > 0 && total.is_multiple_of(LOG_EVERY) {
        eprintln!(
            "query target coverage: {three}/{total} three-way \
             ({inexpressible} SQL-inexpressible, {errors} typed-error outcomes) — \
             ratio {:.3}",
            three as f64 / total as f64
        );
    }
}

/// The hostile arm: structurally-free IR against `Db::prepare` — the
/// validation-totality oracle. Any panic is a finding by definition;
/// the verdict must be deterministic across two prepares.
fn hostile(rng: &mut Rng) {
    let query = irgen::random_query(rng);
    with_world(0, |world| {
        let first = judge(&world.db, &query);
        let second = judge(&world.db, &query);
        assert_eq!(
            first, second,
            "prepare-verdict determinism (hostile arm): {query:#?}"
        );
    });
}

/// The engine's judgment of one structurally-free query, as a comparable
/// value.
#[derive(Debug, PartialEq)]
enum Verdict {
    Accepted,
    /// Rejected: the variant name (the total-match token) plus the full
    /// typed payload.
    Rejected(&'static str, ValidationError),
}

/// One prepare through the REAL public API. The boundary: hostile IR
/// rejects through `Error::Validation` and nothing else — every other
/// variant is named, never a catch-all, and is a finding on this path.
fn judge(db: &Db<target::Target>, query: &Query) -> Verdict {
    match db.prepare(query) {
        Ok(prepared) => {
            drop(prepared);
            Verdict::Accepted
        }
        Err(Error::Validation(rejection)) => {
            Verdict::Rejected(validation_variant(&rejection), rejection)
        }
        Err(
            other @ (Error::Schema(_)
            | Error::FormatMismatch { .. }
            | Error::SchemaMismatch { .. }
            | Error::AlreadyInitialized
            | Error::EnvironmentLocked
            | Error::Io(_)
            | Error::Lmdb(_)
            | Error::ReadersFull { .. }
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
            | Error::Overflow(_)
            | Error::ResultBytesOverflow
            | Error::Corruption(_)),
        ) => {
            panic!("non-validation error from prepare: {other:?}\n{query:#?}")
        }
    }
}

/// The census: every rejection is a NAMED `ValidationError` variant.
/// Total match, zero catch-alls — a new variant is a compile error here.
fn validation_variant(rejection: &ValidationError) -> &'static str {
    match rejection {
        ValidationError::EmptyRuleSet => "EmptyRuleSet",
        ValidationError::TooManyRules { .. } => "TooManyRules",
        ValidationError::DnfExceedsRules { .. } => "DnfExceedsRules",
        ValidationError::ConditionNestingTooDeep { .. } => "ConditionNestingTooDeep",
        ValidationError::HeadArityMismatch { .. } => "HeadArityMismatch",
        ValidationError::HeadTypeMismatch { .. } => "HeadTypeMismatch",
        ValidationError::HeadAggregateMismatch { .. } => "HeadAggregateMismatch",
        ValidationError::ArgAcrossRules { .. } => "ArgAcrossRules",
        ValidationError::UnknownRelation { .. } => "UnknownRelation",
        ValidationError::UnknownField { .. } => "UnknownField",
        ValidationError::DuplicateFieldBinding { .. } => "DuplicateFieldBinding",
        ValidationError::VariableTypeConflict { .. } => "VariableTypeConflict",
        ValidationError::LiteralTypeMismatch { .. } => "LiteralTypeMismatch",
        ValidationError::PointLiteralAtCeiling { .. } => "PointLiteralAtCeiling",
        ValidationError::ParamIdGap { .. } => "ParamIdGap",
        ValidationError::ParamTypeConflict { .. } => "ParamTypeConflict",
        ValidationError::ParamScalarAndSet { .. } => "ParamScalarAndSet",
        ValidationError::ParamSetComparison { .. } => "ParamSetComparison",
        ValidationError::IntervalParamSet { .. } => "IntervalParamSet",
        ValidationError::IllegalComparison { .. } => "IllegalComparison",
        ValidationError::OrderComparisonOnInterval { .. } => "OrderComparisonOnInterval",
        ValidationError::OrderComparisonOnFixedBytes { .. } => "OrderComparisonOnFixedBytes",
        ValidationError::OrderComparisonOnString { .. } => "OrderComparisonOnString",
        ValidationError::OrderComparisonOnBool { .. } => "OrderComparisonOnBool",
        ValidationError::ConstantComparison { .. } => "ConstantComparison",
        ValidationError::SelfComparison { .. } => "SelfComparison",
        ValidationError::ComparisonPointLiteralAtCeiling { .. } => {
            "ComparisonPointLiteralAtCeiling"
        }
        ValidationError::EmptyAllenMask { .. } => "EmptyAllenMask",
        ValidationError::FullAllenMask { .. } => "FullAllenMask",
        ValidationError::MembershipOnlyVariable { .. } => "MembershipOnlyVariable",
        ValidationError::NegatedVariableUnbound { .. } => "NegatedVariableUnbound",
        ValidationError::UnboundFindVariable { .. } => "UnboundFindVariable",
        ValidationError::ComparisonOnlyVariable { .. } => "ComparisonOnlyVariable",
        ValidationError::EmptyFinds => "EmptyFinds",
        ValidationError::DuplicateFindTerm { .. } => "DuplicateFindTerm",
        ValidationError::NoPositiveAtoms => "NoPositiveAtoms",
        ValidationError::AggregateInputType { .. } => "AggregateInputType",
        ValidationError::CountWithVariable { .. } => "CountWithVariable",
        ValidationError::AggregateWithoutVariable { .. } => "AggregateWithoutVariable",
        ValidationError::AggregateOverGroupKey { .. } => "AggregateOverGroupKey",
        ValidationError::MixedArgAndFold { .. } => "MixedArgAndFold",
        ValidationError::ArgKeyMismatch { .. } => "ArgKeyMismatch",
        ValidationError::NonOrderableArgKey { .. } => "NonOrderableArgKey",
        ValidationError::MultiplePackTerms { .. } => "MultiplePackTerms",
        ValidationError::MixedPackAndFold { .. } => "MixedPackAndFold",
        ValidationError::MixedPackAndArg { .. } => "MixedPackAndArg",
        ValidationError::PackInputType { .. } => "PackInputType",
        ValidationError::DurationInBinding { .. } => "DurationInBinding",
        ValidationError::DurationOverNonInterval { .. } => "DurationOverNonInterval",
        ValidationError::DurationAggregateOp { .. } => "DurationAggregateOp",
        ValidationError::DurationComparisonOperator { .. } => "DurationComparisonOperator",
        ValidationError::DurationBothSides { .. } => "DurationBothSides",
        ValidationError::TooManyAtoms { .. } => "TooManyAtoms",
        ValidationError::TooManyVariables { .. } => "TooManyVariables",
    }
}
