//! The independent staged relation-expression evaluator (P11; chapter 12,
//! chapter 13 §C; gates `Q-IR`, `Q-GROUP`, `Q-RECUR`, `F-SET`,
//! `F-OPT-NEG`; audit ASS-002 routing). Lean twin:
//! `lean/Bumbledb/Query/Stages.lean`.
//!
//! Stages evaluate in an acyclic order — each is a total function of the
//! EARLIER stages' complete outputs, producing a complete deduplicated row
//! set or a semantic error. Aggregate/computed stages are ordinary stages
//! whose outputs later stages consume; a name never forces materialization;
//! a required producer's error surfaces through every consumer; and the
//! restricted recursive node stays inside its frozen finite domain. Float
//! aggregate expectations come from the independent bit/rational oracle,
//! never host float folds or production kernels.

use std::collections::BTreeSet;

/// A model row: a small tuple of words. Set semantics via `BTreeSet`.
pub type Row = Vec<u64>;

/// One stage outcome: a complete row SET or a semantic error (overflow,
/// invalid cast, measure failure — the model needs only the propagation
/// discipline, not the reason roster).
pub type Outcome = Result<BTreeSet<Row>, StageError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageError;

/// One stage: declared reads (indices of earlier stages) and a total
/// function of exactly those tables — a stage cannot consult anything it
/// did not declare, by construction.
pub struct Stage {
    pub reads: Vec<usize>,
    #[expect(
        clippy::type_complexity,
        reason = "the boxed function IS the stage meaning; aliasing it would hide the contract"
    )]
    pub eval: Box<dyn Fn(&[&BTreeSet<Row>]) -> Outcome>,
}

impl Stage {
    #[must_use]
    pub fn new(reads: Vec<usize>, eval: impl Fn(&[&BTreeSet<Row>]) -> Outcome + 'static) -> Self {
        Self {
            reads,
            eval: Box::new(eval),
        }
    }
}

/// Evaluate the whole graph left to right. A stage reading an erroring
/// producer (or an out-of-range index) is itself an error — its own
/// function never runs on incomplete input, so a downstream filter cannot
/// suppress a required upstream error.
#[must_use]
pub fn eval_graph(graph: &[Stage]) -> Vec<Outcome> {
    let mut outcomes: Vec<Outcome> = Vec::with_capacity(graph.len());
    for stage in graph {
        let mut tables: Vec<&BTreeSet<Row>> = Vec::with_capacity(stage.reads.len());
        let mut poisoned = false;
        for &read in &stage.reads {
            if let Some(Ok(rows)) = outcomes.get(read) {
                tables.push(rows);
            } else {
                poisoned = true;
                break;
            }
        }
        if poisoned {
            outcomes.push(Err(StageError));
        } else {
            outcomes.push((stage.eval)(&tables));
        }
    }
    outcomes
}

/// A base table stage.
#[must_use]
pub fn table(rows: &[Row]) -> Stage {
    let set: BTreeSet<Row> = rows.iter().cloned().collect();
    Stage::new(vec![], move |_| Ok(set.clone()))
}

/// A projection stage: map each row, DEDUPLICATING the result — an explicit
/// projection forms a new relation and therefore a new aggregate grain.
#[must_use]
pub fn project(read: usize, f: impl Fn(&Row) -> Row + 'static) -> Stage {
    Stage::new(vec![read], move |tables| {
        Ok(tables[0].iter().map(&f).collect())
    })
}

/// A total-predicate filter stage.
#[must_use]
pub fn filter(read: usize, keep: impl Fn(&Row) -> bool + 'static) -> Stage {
    Stage::new(vec![read], move |tables| {
        Ok(tables[0].iter().filter(|row| keep(row)).cloned().collect())
    })
}

/// A grouped count over the DISTINCT rows of the input, keyed by a
/// projection: one output row per group, `[key.., count]`.
#[must_use]
pub fn group_count(read: usize, key: impl Fn(&Row) -> Row + 'static) -> Stage {
    Stage::new(vec![read], move |tables| {
        let mut groups: std::collections::BTreeMap<Row, u64> = std::collections::BTreeMap::new();
        for row in tables[0] {
            *groups.entry(key(row)).or_insert(0) += 1;
        }
        Ok(groups
            .into_iter()
            .map(|(mut key_row, count)| {
                key_row.push(count);
                key_row
            })
            .collect())
    })
}

/// The frozen-finite-domain recursive node: semi-naive rounds of a
/// projection-only step over the frozen inputs, refusing any step output
/// outside the frozen active domain — the model-level validation fence
/// against aggregation/arithmetic/value creation in the feedback cycle.
///
/// # Errors
/// `StageError` when the step manufactures a row outside the domain.
pub fn recurse_within(
    domain: &BTreeSet<Row>,
    base: &BTreeSet<Row>,
    step: impl Fn(&BTreeSet<Row>) -> BTreeSet<Row>,
) -> Result<BTreeSet<Row>, StageError> {
    if !base.iter().all(|row| domain.contains(row)) {
        return Err(StageError);
    }
    let mut seen = base.clone();
    loop {
        let derived = step(&seen);
        if !derived.iter().all(|row| domain.contains(row)) {
            return Err(StageError);
        }
        let before = seen.len();
        seen.extend(derived);
        if seen.len() == before {
            return Ok(seen);
        }
        // Contained in the finite domain and strictly growing: the loop
        // terminates within |domain| rounds.
        debug_assert!(seen.len() <= domain.len());
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::verify::f64_oracle::{self, mean_bits};

    use super::{
        Outcome, Row, Stage, StageError, eval_graph, filter, group_count, project, recurse_within,
        table,
    };

    fn rows(raw: &[&[u64]]) -> Vec<Row> {
        raw.iter().map(|r| r.to_vec()).collect()
    }

    fn set(raw: &[&[u64]]) -> BTreeSet<Row> {
        rows(raw).into_iter().collect()
    }

    #[test]
    fn consumer_filter_cannot_hide_producer_error() {
        // producer (errors: an overflowing aggregate) -> consumer filter
        // that would discard everything anyway. The error still surfaces.
        let graph = vec![
            table(&rows(&[&[1], &[2], &[3]])),
            Stage::new(vec![0], |_| Err(StageError)),
            filter(1, |_| false),
        ];
        let outcomes = eval_graph(&graph);
        assert_eq!(outcomes[1], Err(StageError));
        assert_eq!(
            outcomes[2],
            Err(StageError),
            "a filter cannot un-require a required producer"
        );
        // The same consumer over a healthy producer succeeds empty: the
        // filter itself is not the error.
        let healthy = vec![
            table(&rows(&[&[1], &[2], &[3]])),
            project(0, Clone::clone),
            filter(1, |_| false),
        ];
        let outcomes = eval_graph(&healthy);
        assert_eq!(outcomes[2], Ok(BTreeSet::new()));
    }

    #[test]
    fn naming_does_not_force_materialization() {
        // Interposing a NAMED identity stage (or any unread stage) changes
        // no other outcome: names are compositional handles.
        let direct = vec![
            table(&rows(&[&[1, 10], &[2, 10], &[3, 20]])),
            group_count(0, |row| vec![row[1]]),
        ];
        let named = vec![
            table(&rows(&[&[1, 10], &[2, 10], &[3, 20]])),
            project(0, Clone::clone), // the name
            group_count(1, |row| vec![row[1]]),
            // An unreferenced definition: never evaluated into anyone's
            // answer, and an ERRORING unreferenced definition changes no
            // referenced outcome either.
            Stage::new(vec![0], |_| Err(StageError)),
        ];
        let d = eval_graph(&direct);
        let n = eval_graph(&named);
        assert_eq!(d[1], n[2], "the named plan answers identically");
        assert_eq!(n[3], Err(StageError), "the unread stage may even error");
    }

    #[test]
    fn inline_and_materialized_stages_agree() {
        // consumer(producer(x)) as two stages versus one fused stage:
        // identical values AND identical errors.
        let base = rows(&[&[1, 5], &[2, 5], &[3, 7]]);
        let two_stage = vec![
            table(&base),
            project(0, |row| vec![row[1]]),
            group_count(1, |_| vec![0]),
        ];
        let fused = vec![
            table(&base),
            Stage::new(vec![0], |tables| {
                // Inline: project THEN count within one stage, preserving
                // the projection's dedup grain.
                let projected: BTreeSet<Row> = tables[0].iter().map(|row| vec![row[1]]).collect();
                let count = u64::try_from(projected.len()).expect("small");
                Ok([vec![0, count]].into_iter().collect())
            }),
        ];
        assert_eq!(eval_graph(&two_stage)[2], eval_graph(&fused)[1]);
        // The error path fuses identically: an erroring producer makes the
        // fused stage error exactly like the consumer of a separate
        // erroring stage.
        let two_stage_err = vec![
            table(&base),
            Stage::new(vec![0], |_| Err(StageError)),
            group_count(1, |_| vec![0]),
        ];
        let fused_err = vec![table(&base), Stage::new(vec![0], |_| Err(StageError))];
        assert_eq!(
            eval_graph(&two_stage_err)[2],
            eval_graph(&fused_err)[1],
            "fusion preserves the producer's error boundary"
        );
    }

    #[test]
    fn aggregate_grain_is_the_distinct_input_row_set() {
        // Attempts (attempt-id, student): counting attempt bindings counts
        // attempts; projecting to student FIRST counts students. Naming
        // changes neither (covered above).
        let attempts = rows(&[&[1, 10], &[2, 10], &[3, 20]]);
        let count_attempts = vec![table(&attempts), group_count(0, |_| vec![0])];
        let count_students = vec![
            table(&attempts),
            project(0, |row| vec![row[1]]),
            group_count(1, |_| vec![0]),
        ];
        assert_eq!(eval_graph(&count_attempts)[1], Ok(set(&[&[0, 3]])));
        assert_eq!(eval_graph(&count_students)[2], Ok(set(&[&[0, 2]])));
        // Equal-valued measures on DISTINCT bindings contribute separately;
        // projecting identity away first deliberately leaves one row.
        let amounts = rows(&[&[1, 5], &[2, 5]]);
        let sum_full = vec![
            table(&amounts),
            Stage::new(vec![0], |tables| {
                let total: u64 = tables[0].iter().map(|row| row[1]).sum();
                Ok([vec![total]].into_iter().collect())
            }),
        ];
        let sum_projected = vec![
            table(&amounts),
            project(0, |row| vec![row[1]]),
            Stage::new(vec![1], |tables| {
                let total: u64 = tables[0].iter().map(|row| row[0]).sum();
                Ok([vec![total]].into_iter().collect())
            }),
        ];
        assert_eq!(eval_graph(&sum_full)[1], Ok(set(&[&[10]])));
        assert_eq!(eval_graph(&sum_projected)[2], Ok(set(&[&[5]])));
        // No input rows, no group — the aggregate of empty input is the
        // empty answer set, never a fabricated zero row.
        let empty = vec![table(&[]), group_count(0, |_| vec![0])];
        assert_eq!(eval_graph(&empty)[1], Ok(BTreeSet::new()));
    }

    #[test]
    fn mean_of_means_is_not_the_global_mean() {
        // Aggregate-derived stages expose FINALIZED canonical scalars: a
        // downstream mean of per-group means averages once-rounded values
        // and differs from the global mean — fusing them is not legal
        // without a binding-equivalence proof. Expectations come from the
        // independent bit/rational oracle.
        let one = 0x3ff0_0000_0000_0000u64; // 1.0
        let two = 0x4000_0000_0000_0000u64; // 2.0
        let four = 0x4010_0000_0000_0000u64; // 4.0
        // Groups: {1.0} and {2.0, 4.0}.
        let mean_a = mean_bits(&[one]).expect("nonempty");
        let mean_b = mean_bits(&[two, four]).expect("nonempty");
        let mean_of_means = mean_bits(&[mean_a, mean_b]).expect("nonempty");
        let global = mean_bits(&[one, two, four]).expect("nonempty");
        assert_ne!(
            mean_of_means, global,
            "two means of means: (1 + 3)/2 = 2 versus (1 + 2 + 4)/3"
        );
        // And the once-rounded group means are exact canonical scalars:
        // the consumer never sees an accumulator.
        assert_eq!(mean_a, one);
        assert_eq!(mean_b, 0x4008_0000_0000_0000, "mean{{2,4}} = 3.0 exactly");
        assert_eq!(
            f64_oracle::classify(mean_of_means),
            f64_oracle::Class::Finite
        );
    }

    #[test]
    fn frozen_computed_predecessors_stay_in_domain() {
        // The frozen active domain includes aggregate/computed PREDECESSOR
        // outputs — here a computed table of even numbers — frozen once
        // before the fixpoint. Projection-only steps stay inside it.
        let computed: BTreeSet<Row> = (0u64..10).map(|n| vec![n * 2]).collect();
        let domain = computed.clone();
        let base: BTreeSet<Row> = [vec![0u64]].into_iter().collect();
        // Step: follow the "successor even number" edge — pure selection
        // from the frozen domain.
        let reached = recurse_within(&domain, &base, |seen| {
            seen.iter()
                .map(|row| vec![row[0] + 2])
                .filter(|row| domain.contains(row))
                .collect()
        })
        .expect("projection-only steps stay inside the frozen domain");
        assert_eq!(reached, computed, "the closure reaches the whole chain");
    }

    #[test]
    fn value_creation_feedback_is_refused() {
        // One `+1` in the feedback cycle manufactures rows outside every
        // frozen finite domain: the fence refuses instead of diverging.
        let domain: BTreeSet<Row> = (0u64..4).map(|n| vec![n]).collect();
        let base: BTreeSet<Row> = [vec![0u64]].into_iter().collect();
        let refused = recurse_within(&domain, &base, |seen| {
            seen.iter().map(|row| vec![row[0] + 1]).collect()
        });
        assert_eq!(
            refused,
            Err(StageError),
            "value creation in the cycle escapes the domain and is refused"
        );
    }

    #[test]
    fn later_stages_cannot_rewrite_earlier_outcomes() {
        let graph = vec![
            table(&rows(&[&[1]])),
            Stage::new(vec![0], |_| Err(StageError)),
            table(&rows(&[&[9]])),
        ];
        let outcomes = eval_graph(&graph);
        assert_eq!(outcomes[0], Ok(set(&[&[1]])));
        assert_eq!(outcomes[2], Ok(set(&[&[9]])));
        // Out-of-range reads (a validator refusal in the engine) poison
        // in the model rather than panicking.
        let dangling: Vec<Stage> = vec![Stage::new(vec![5], |_| Ok(BTreeSet::new()))];
        let outcomes: Vec<Outcome> = eval_graph(&dangling);
        assert_eq!(outcomes[0], Err(StageError));
    }
}
