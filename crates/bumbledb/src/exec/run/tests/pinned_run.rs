//! The pinned-run arm (the fold pushdown for probe-pinned leaves,
//! `leaf.rs`): one batched emit over many pinned leaf rows is
//! value-identical to the recursive per-survivor path — combined
//! outer+leaf key layout, leaf residuals included.

use super::*;
use crate::exec::sink::{AggSpec, AggregateSink, FindSpec, FoldOp};
use crate::ir::WordCmp;

/// Hand-built two-node plan `[R(g)][R(x)]` over one occurrence — the
/// dimension-bound shape whose leaf the probe pass pins per survivor.
/// All-vars sinks: aggregate plans mark every node sink-relevant.
fn split_plan(normalized: &NormalizedQuery, schema: &Schema) -> ValidatedPlan {
    let node = |vars: &[u16]| crate::plan::fj::Node {
        subatoms: vec![crate::plan::fj::Subatom {
            occ: OccId(0),
            vars: vars.iter().map(|v| VarId(*v)).collect(),
        }],
    };
    let plan = crate::plan::fj::FjPlan {
        nodes: vec![node(&[0]), node(&[1])],
    };
    validate(&plan, normalized, schema, vec![0; 2], &all_vars(normalized)).expect("valid plan")
}

/// GROUP BY g: Count + Sum(x) — the Sum reads a leaf key word, so the
/// batched arm's gather fold runs against real survivors.
fn finds(plan: &ValidatedPlan) -> Vec<FindSpec> {
    vec![
        FindSpec::Var {
            slot: plan.slot_of(VarId(0)),
            width: 1,
        },
        FindSpec::Agg(AggSpec::Count),
        FindSpec::Agg(AggSpec::Fold {
            op: FoldOp::Sum,
            slot: plan.slot_of(VarId(1)),
            width: 1,
            signed: false,
        }),
    ]
}

fn answers_of(sink: AggregateSink) -> Vec<Vec<u64>> {
    let mut rows = sink.into_answers().expect("in range");
    rows.sort_unstable();
    rows
}

/// Each storage position's parent-cover word, read back through the
/// colt (level 0 holds the g column): the probe pass carries exactly
/// this pairing — a pinned position under the parent whose keys
/// probed it.
fn outer_words_of(colt: &Colt, positions: &[u32]) -> Vec<u64> {
    positions
        .iter()
        .map(|&position| {
            let mut word = [0u64; 1];
            colt.gather_row(0, position, &mut word);
            word[0]
        })
        .collect()
}

#[test]
fn pinned_run_matches_the_recursive_path() {
    let rows = vec![(1u64, 10u64), (1, 11), (1, 12), (2, 1), (3, 2), (3, 6)];
    for residuals in [
        vec![],
        // g < x at the leaf: kills (2,1) and (3,2) — the batched arm's
        // residual pass must compact exactly as the recursive one.
        vec![PlacedComparison {
            op: WordCmp::Lt,
            lhs: VarId(0),
            rhs: VarId(1),
        }],
    ] {
        let n_residuals = residuals.len();
        let dir = TempDir::new("pinned-run");
        let schema = schema(1);
        let views = views_of(&dir, &schema, std::slice::from_ref(&rows));
        let query = normalized(vec![occurrence(0, 0, &[(0, 0), (1, 1)])], residuals);
        let plan = split_plan(&query, &schema);

        // The recursive reference: the ordinary executor, same sink
        // configuration.
        let mut colts = colts_for(&plan, &views);
        let mut bindings = Bindings::new(plan.slot_count());
        let mut reference = AggregateSink::new(finds(&plan), plan.slot_count());
        Executor::new(&plan)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut reference,
                &mut NoopCounters,
            )
            .expect("execute");

        // The pinned run: every leaf row pinned by position, its
        // parent's cover word riding as a batch word — ONE emit for
        // the whole run.
        let colts = colts_for(&plan, &views);
        let bindings = Bindings::new(plan.slot_count());
        let mut sink = AggregateSink::new(finds(&plan), plan.slot_count());
        let positions: Vec<u32> = (0..u32::try_from(rows.len()).expect("small")).collect();
        let outer_keys = outer_words_of(&colts[0], &positions);
        let key_slots = vec![plan.slot_of(VarId(0)), plan.slot_of(VarId(1))];
        let mut executor = Executor::new(&plan);
        let flow = executor.run_leaf_pinned_run(
            &plan,
            1,
            0,
            1,
            &positions,
            &outer_keys,
            &key_slots,
            &colts,
            &bindings,
            &mut sink,
            &mut NoopCounters,
        );
        assert_eq!(flow, Flow::Continue);
        assert_eq!(
            answers_of(sink),
            answers_of(reference),
            "residuals: {n_residuals}"
        );
    }
}

/// Splitting one run into several (the caller flushes at parent
/// boundaries and batch capacity) folds to the same answers — the
/// batched arm is insensitive to run partitioning.
#[test]
fn pinned_run_partitioning_is_transparent() {
    let rows = vec![(1u64, 10u64), (1, 11), (1, 12), (2, 1), (3, 2), (3, 6)];
    let dir = TempDir::new("pinned-run-parts");
    let schema = schema(1);
    let views = views_of(&dir, &schema, std::slice::from_ref(&rows));
    let query = normalized(vec![occurrence(0, 0, &[(0, 0), (1, 1)])], vec![]);
    let plan = split_plan(&query, &schema);
    let key_slots = vec![plan.slot_of(VarId(0)), plan.slot_of(VarId(1))];
    let positions: Vec<u32> = (0..u32::try_from(rows.len()).expect("small")).collect();
    let outer_keys = outer_words_of(&colts_for(&plan, &views)[0], &positions);
    let mut reference: Option<Vec<Vec<u64>>> = None;
    // The one-part partitioning IS a single-range slice — the whole
    // window in one piece, not a six-element collect.
    #[allow(clippy::single_range_in_vec_init)]
    let partitionings: [&[std::ops::Range<usize>]; 3] =
        [&[0..6], &[0..3, 3..4, 4..6], &[0..1, 1..6]];
    for splits in partitionings {
        let colts = colts_for(&plan, &views);
        let bindings = Bindings::new(plan.slot_count());
        let mut sink = AggregateSink::new(finds(&plan), plan.slot_count());
        let mut executor = Executor::new(&plan);
        for range in splits {
            executor.run_leaf_pinned_run(
                &plan,
                1,
                0,
                1,
                &positions[range.clone()],
                &outer_keys[range.clone()],
                &key_slots,
                &colts,
                &bindings,
                &mut sink,
                &mut NoopCounters,
            );
        }
        let rows = answers_of(sink);
        match &reference {
            None => reference = Some(rows),
            Some(r) => assert_eq!(*r, rows),
        }
    }
}
