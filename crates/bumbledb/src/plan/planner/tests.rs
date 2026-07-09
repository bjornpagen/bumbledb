use super::densify::densify;
use super::estimate::estimate;
use super::*;
use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::{NormalizedQuery, Occurrence, Polarity, SlotWidth};
use crate::ir::CmpOp as ViewCmp;
use crate::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, RelationId, Schema,
    SchemaDescriptor, StatementDescriptor, ValueType,
};
use std::collections::BTreeMap;

/// Builds a schema of `n` relations, each with `arity` U64 fields; the
/// first field of each relation is serial (its auto-key).
fn schema(n: usize, arity: usize) -> Schema {
    SchemaDescriptor {
        relations: (0..n)
            .map(|r| RelationDescriptor {
                name: format!("R{r}").into(),
                fields: (0..arity)
                    .map(|f| FieldDescriptor {
                        name: format!("f{f}").into(),
                        value_type: ValueType::U64,
                        generation: if f == 0 {
                            Generation::Serial
                        } else {
                            Generation::None
                        },
                    })
                    .collect(),
            })
            .collect(),
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn occurrence(occ: u16, relation: u32, vars: Vec<(u16, u16)>) -> Occurrence {
    Occurrence {
        occ_id: OccId(occ),
        relation: RelationId(relation),
        polarity: Polarity::Positive,
        vars: vars
            .into_iter()
            .map(|(f, v)| (FieldId(f), VarId(v)))
            .collect(),
        filters: vec![],
    }
}

/// Assembles a `NormalizedQuery` around hand-built occurrences (scalar
/// slot widths; no residuals or anti-probes — the planner reads none of
/// them).
fn normalized(occurrences: Vec<Occurrence>) -> NormalizedQuery {
    let slot_widths: BTreeMap<VarId, SlotWidth> = occurrences
        .iter()
        .flat_map(|o| o.vars.iter().map(|(_, v)| (*v, SlotWidth::One)))
        .collect();
    NormalizedQuery {
        occurrences,
        residuals: vec![],
        word_residuals: vec![],
        anti_probes: vec![],
        slot_widths,
    }
}

fn stats(rows: &[u64]) -> Vec<OccStats> {
    rows.iter()
        .enumerate()
        .map(|(i, r)| OccStats {
            occ_id: OccId(u16::try_from(i).expect("small")),
            rows: *r,
            // Hand-built stats: unit fanout (no distinct info).
            var_distincts: Vec::new(),
        })
        .collect()
}

/// Cost of a specific order under the same estimator (for brute-force
/// comparison in tests).
fn order_cost(
    normalized: &NormalizedQuery,
    schema: &Schema,
    stats: &[OccStats],
    order: &[usize],
) -> u64 {
    // Re-plan restricted: walk the order, applying the estimator.
    let occ = |i: usize| &normalized.occurrences[i];
    let rows = |i: usize| {
        stats
            .iter()
            .find(|s| s.occ_id == occ(i).occ_id)
            .expect("stats")
            .rows
    };
    let mut var_index = std::collections::BTreeMap::new();
    for o in &normalized.occurrences {
        for (_, v) in &o.vars {
            let next = var_index.len();
            var_index.entry(*v).or_insert(next);
        }
    }
    let var_set = |i: usize| {
        occ(i)
            .vars
            .iter()
            .fold(0u128, |acc, (_, v)| acc | 1 << var_index[v])
    };
    let key_sets = |i: usize| -> Vec<u128> {
        let relation = schema.relation(occ(i).relation);
        relation
            .keys()
            .iter()
            .filter_map(|id| {
                let StatementDescriptor::Functionality { projection, .. } =
                    &schema.statement(*id).descriptor
                else {
                    unreachable!("keys() indexes Functionality statements")
                };
                let mut set = 0u128;
                for field in projection {
                    let (_, var) = occ(i).vars.iter().find(|(f, _)| f == field)?;
                    set |= 1 << var_index[var];
                }
                Some(set)
            })
            .collect()
    };

    let mut est = rows(order[0]);
    let mut cost = est;
    let mut prefix_vars = var_set(order[0]);
    for &next in &order[1..] {
        // Mirror of the production estimator (docs/architecture/40-execution.md): key
        // coverage pins the fanout to 1; hand-built stats carry no
        // distinct counts, so everything else is the pessimistic
        // product.
        let join_vars = var_set(next) & prefix_vars;
        let step = if join_vars != 0 && key_sets(next).iter().any(|s| s & join_vars == *s) {
            est
        } else {
            est.saturating_mul(rows(next))
        };
        cost = cost.saturating_add(step);
        est = step;
        prefix_vars |= var_set(next);
    }
    cost
}

#[test]
fn selective_filtered_occurrence_leads_a_reference_walk() {
    // Occ 0: Posting-like, 10_000 rows. Occ 1: Account-like with a
    // filter measured to 1 survivor; the walk joins on occ 1's serial
    // key (var 0). The planner must iterate the 1-row side first.
    let schema = schema(2, 2);
    let mut occ1 = occurrence(1, 1, vec![(0, 0)]);
    occ1.filters.push(FilterPredicate::Compare {
        field: FieldId(1),
        op: ViewCmp::Eq,
        value: Const::Word(7),
    });
    let query = normalized(vec![occurrence(0, 0, vec![(1, 0), (0, 1)]), occ1]);
    // The Posting-like side records its join field's distinct count
    // (5_000 accounts over 10_000 postings — a fanout of 2); the old
    // prefix-side-covered rule priced this walk at min(1, 10_000) = 1,
    // exactly the dishonesty docs/architecture/40-execution.md killed.
    let mut occ_stats = stats(&[10_000, 1]);
    occ_stats[0].var_distincts = vec![(VarId(0), 5_000), (VarId(1), 10_000)];
    let order = plan(&query, &schema, &occ_stats);
    assert_eq!(order.order, vec![OccId(1), OccId(0)]);
    // Step estimates: 1 survivor, then 1 x fanout(10_000 / 5_000) = 2.
    assert_eq!(order.estimates, vec![1, 2]);
}

#[test]
fn non_key_join_is_priced_pessimistically_and_pushed_last() {
    // Occs 0-1 join on occ 1's serial key; occ 2 joins occ 0 on a
    // genuinely non-key shared var (occ 0 field 2 = var 3, occ 2 field
    // 1 = var 3). Pessimism must order occ 2 last.
    let schema = schema(3, 3);
    let query = normalized(vec![
        occurrence(0, 0, vec![(0, 0), (1, 1), (2, 3)]),
        occurrence(1, 1, vec![(0, 1)]),
        occurrence(2, 2, vec![(1, 3)]),
    ]);
    let order = plan(&query, &schema, &stats(&[100, 50, 40]));
    assert_eq!(*order.order.last().expect("nonempty"), OccId(2));
    // The last step is the pessimistic product.
    let last = *order.estimates.last().expect("nonempty");
    assert_eq!(last, order.estimates[1].saturating_mul(40).min(last));
    assert!(last >= 40, "non-key join priced as a product");
}

#[test]
fn key_coverage_fires_through_the_serial_auto_key() {
    // Two occurrences joined on var 0 = occ 1's serial field: joining
    // occ 1 INTO occ 0 must estimate |occ 0| (a reference walk), not a
    // product.
    let schema = schema(2, 2);
    let query = normalized(vec![
        occurrence(0, 0, vec![(1, 0)]),
        occurrence(1, 1, vec![(0, 0)]),
    ]);
    let order = plan(&query, &schema, &stats(&[70, 500]));
    assert_eq!(order.order, vec![OccId(0), OccId(1)]);
    assert_eq!(order.estimates, vec![70, 70]);
}

/// D(acct), Cover(acct, period interval — the pointwise key
/// Cover(acct, period) -> Cover); the driver relation optionally binds
/// the interval by value too.
fn pointwise_schema() -> Schema {
    let interval = ValueType::Interval {
        element: IntervalElement::U64,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "D".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "acct".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "p".into(),
                        value_type: interval.clone(),
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                name: "Cover".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "acct".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "period".into(),
                        value_type: interval,
                        generation: Generation::None,
                    },
                ],
            },
        ],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(1),
            projection: Box::new([FieldId(0), FieldId(1)]),
        }],
    }
    .validate()
    .expect("valid fixture")
}

/// Stats for the pointwise fixtures: a 5-row driver into a 1000-row
/// Cover with 100 distinct accounts (general fanout 10) and 250
/// distinct periods (general fanout 4).
fn pointwise_stats() -> Vec<OccStats> {
    let mut occ_stats = stats(&[5, 1000]);
    occ_stats[1].var_distincts = vec![(VarId(0), 100), (VarId(1), 250)];
    occ_stats
}

/// The pointwise-key guard, direction one (PRD 15 criterion): a join
/// binding only the scalar prefix of a pointwise key does NOT certify
/// fanout 1 — two facts may share the prefix with disjoint intervals —
/// so the step takes the general per-binding fanout.
#[test]
fn pointwise_prefix_join_takes_the_general_fanout() {
    let schema = pointwise_schema();
    // Driver shares only acct (var 0); Cover binds acct and period by
    // value, so the key's full var set exists — but the join covers
    // just the prefix.
    let query = normalized(vec![
        occurrence(0, 0, vec![(0, 0)]),
        occurrence(1, 1, vec![(0, 0), (1, 1)]),
    ]);
    let positive: Vec<&Occurrence> = query.occurrences.iter().collect();
    let occs = densify(&positive, &schema, &pointwise_stats());
    let est = estimate(5, occs[0].vars, &occs, 1);
    assert_eq!(est, 50, "the general case: 5 x fanout(1000/100) = 50");
    assert_ne!(est, 5, "the scalar prefix must not certify fanout 1");
}

/// The pointwise-key guard, direction two: binding the FULL projection —
/// the interval field by value included — covers the key and pins the
/// fanout to 1.
#[test]
fn full_pointwise_projection_bound_by_value_pins_fanout_one() {
    let schema = pointwise_schema();
    let query = normalized(vec![
        occurrence(0, 0, vec![(0, 0), (1, 1)]),
        occurrence(1, 1, vec![(0, 0), (1, 1)]),
    ]);
    let positive: Vec<&Occurrence> = query.occurrences.iter().collect();
    let occs = densify(&positive, &schema, &pointwise_stats());
    let est = estimate(5, occs[0].vars, &occs, 1);
    assert_eq!(est, 5, "full key coverage: the reference-walk bound");
    // Control: without the key the same join would price at the general
    // fanout min(1000/100, 1000/250) = 4 per binding.
    let no_key = OccInfo {
        key_var_sets: Vec::new(),
        vars: occs[1].vars,
        rows: occs[1].rows,
        var_distincts: occs[1].var_distincts.clone(),
    };
    let occs_no_key = [
        OccInfo {
            key_var_sets: Vec::new(),
            vars: occs[0].vars,
            rows: occs[0].rows,
            var_distincts: occs[0].var_distincts.clone(),
        },
        no_key,
    ];
    assert_eq!(estimate(5, occs_no_key[0].vars, &occs_no_key, 1), 20);
}

/// A membership-bound interval field never enters `vars` (normalization
/// lowers it to a filter), so the pointwise key's var set does not exist
/// and coverage cannot fire — the other face of the same guard.
#[test]
fn membership_bound_interval_disables_key_coverage() {
    let schema = pointwise_schema();
    let query = normalized(vec![
        occurrence(0, 0, vec![(0, 0)]),
        // Cover's period is membership-bound: absent from vars.
        occurrence(1, 1, vec![(0, 0)]),
    ]);
    let positive: Vec<&Occurrence> = query.occurrences.iter().collect();
    let mut occ_stats = pointwise_stats();
    occ_stats[1].var_distincts = vec![(VarId(0), 100)];
    let occs = densify(&positive, &schema, &occ_stats);
    assert!(occs[1].key_var_sets.is_empty());
    assert_eq!(estimate(5, occs[0].vars, &occs, 1), 50);
}

/// Negated occurrences are excluded from the DP state entirely: the
/// order covers the positive occurrences and nothing else, and no stats
/// are consulted for the negated one.
#[test]
fn negated_occurrences_enter_no_dp_state() {
    let schema = schema(3, 2);
    let mut occurrences = vec![
        occurrence(0, 0, vec![(1, 0)]),
        occurrence(1, 1, vec![(0, 0)]),
    ];
    occurrences.push(Occurrence {
        polarity: Polarity::Negated,
        ..occurrence(2, 2, vec![(1, 0)])
    });
    let query = normalized(occurrences);
    // Stats cover the positive occurrences only — the planner must not
    // ask about occ 2.
    let order = plan(&query, &schema, &stats(&[70, 500]));
    assert_eq!(order.order, vec![OccId(0), OccId(1)]);
    assert_eq!(order.estimates.len(), 2);
}

#[test]
fn dp_beats_greedy_on_a_constructed_counterexample() {
    // A(x big), B(serial x, y), C(y), D(y): greedy grabs the cheapest
    // immediate pair (C x D, a small product) and pays for it; the DP
    // routes through B's serial key.
    let schema = schema(4, 2);
    let query = normalized(vec![
        occurrence(0, 0, vec![(1, 0)]),         // A: x, non-key
        occurrence(1, 1, vec![(0, 0), (1, 1)]), // B: serial x, y
        occurrence(2, 2, vec![(1, 1)]),         // C: y, non-key
        occurrence(3, 3, vec![(1, 1)]),         // D: y, non-key
    ]);
    let occ_stats = stats(&[10, 10, 2, 2]);

    let planned = plan(&query, &schema, &occ_stats);
    let planned_order: Vec<usize> = planned.order.iter().map(|o| usize::from(o.0)).collect();
    let planned_cost = order_cost(&query, &schema, &occ_stats, &planned_order);

    // Brute force: the DP result must be a global optimum.
    let mut best = u64::MAX;
    let mut permutations = vec![];
    permute(&mut vec![0, 1, 2, 3], 0, &mut permutations);
    for p in &permutations {
        best = best.min(order_cost(&query, &schema, &occ_stats, p));
    }
    assert_eq!(planned_cost, best, "DP finds the optimum");

    // Greedy (min immediate estimate at each step) is provably worse on
    // this fixture — the counterexample is real.
    let greedy = greedy_order(&query, &schema, &occ_stats);
    let greedy_cost = order_cost(&query, &schema, &occ_stats, &greedy);
    assert!(
        greedy_cost > planned_cost,
        "greedy {greedy_cost} must exceed DP {planned_cost} (greedy order {greedy:?})"
    );
}

fn permute(items: &mut Vec<usize>, k: usize, out: &mut Vec<Vec<usize>>) {
    if k == items.len() {
        out.push(items.clone());
        return;
    }
    for i in k..items.len() {
        items.swap(k, i);
        permute(items, k + 1, out);
        items.swap(k, i);
    }
}

/// The strawman: start from the smallest relation, repeatedly append
/// the occurrence with the smallest immediate estimate.
fn greedy_order(
    normalized: &NormalizedQuery,
    schema: &Schema,
    occ_stats: &[OccStats],
) -> Vec<usize> {
    let n = normalized.occurrences.len();
    let rows = |i: usize| occ_stats[i].rows;
    let mut remaining: Vec<usize> = (0..n).collect();
    let start = *remaining
        .iter()
        .min_by_key(|&&i| (rows(i), i))
        .expect("nonempty");
    remaining.retain(|&i| i != start);
    let mut order = vec![start];
    while !remaining.is_empty() {
        let next = *remaining
            .iter()
            .min_by_key(|&&i| {
                let mut candidate = order.clone();
                candidate.push(i);
                (order_cost(normalized, schema, occ_stats, &candidate), i)
            })
            .expect("nonempty");
        remaining.retain(|&i| i != next);
        order.push(next);
    }
    order
}

#[test]
fn deterministic_across_shuffled_stats_input() {
    let schema = schema(3, 2);
    let query = normalized(vec![
        occurrence(0, 0, vec![(0, 0)]),
        occurrence(1, 1, vec![(0, 0)]),
        occurrence(2, 2, vec![(1, 0)]),
    ]);
    let forward = stats(&[10, 10, 10]);
    let mut shuffled = forward.clone();
    shuffled.reverse();
    let a = plan(&query, &schema, &forward);
    let b = plan(&query, &schema, &shuffled);
    assert_eq!(a, b);
}

#[test]
fn the_dp_accepts_large_inputs_under_the_cap() {
    // The over-cap rejection lives at the validation boundary
    // (ir::validate); the planner's contract is that anything under
    // the cap plans. 16 occurrences (a 2^16-state table) keeps the
    // debug-build suite fast; the full 2^20 cap is exercised by the
    // same code path with a bigger constant.
    let schema = schema(1, 2);
    let occurrences: Vec<Occurrence> = (0..16)
        .map(|i| occurrence(u16::try_from(i).expect("small"), 0, vec![(0, 0)]))
        .collect();
    let occ_stats: Vec<OccStats> = occurrences
        .iter()
        .map(|o| OccStats {
            occ_id: o.occ_id,
            rows: 1,
            var_distincts: Vec::new(),
        })
        .collect();
    let query = normalized(occurrences);
    let order = plan(&query, &schema, &occ_stats);
    assert_eq!(order.order.len(), 16);
}
