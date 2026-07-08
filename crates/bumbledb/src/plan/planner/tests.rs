use super::*;
use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::{NormalizedQuery, Occurrence};
use crate::ir::CmpOp as ViewCmp;
use crate::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    ValueType,
};

/// Builds a schema of `n` relations, each with `arity` U64 fields; the
/// first field of each relation is serial (auto-unique).
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
                constraints: vec![],
            })
            .collect(),
    }
    .validate()
    .expect("valid fixture")
}

fn occurrence(occ: u16, relation: u32, vars: Vec<(u16, u16)>) -> Occurrence {
    Occurrence {
        occ_id: OccId(occ),
        relation: RelationId(relation),
        vars: vars
            .into_iter()
            .map(|(f, v)| (FieldId(f), VarId(v)))
            .collect(),
        filters: vec![],
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
    let unique_sets = |i: usize| -> Vec<u128> {
        let relation = schema.relation(occ(i).relation);
        relation
            .unique_constraints()
            .iter()
            .filter_map(|cid| {
                let mut set = 0u128;
                for field in relation.constraint(*cid).fields() {
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
        // Mirror of the production estimator (docs/architecture/30-execution.md): unique
        // coverage pins the fanout to 1; hand-built stats carry no
        // distinct counts, so everything else is the pessimistic
        // product.
        let join_vars = var_set(next) & prefix_vars;
        let step = if join_vars != 0 && unique_sets(next).iter().any(|s| s & join_vars == *s) {
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
fn selective_filtered_occurrence_leads_an_fk_walk() {
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
    let normalized = NormalizedQuery {
        occurrences: vec![occurrence(0, 0, vec![(1, 0), (0, 1)]), occ1],
        residuals: vec![],
    };
    // The Posting-like side records its join field's distinct count
    // (5_000 accounts over 10_000 postings — a fanout of 2); the old
    // prefix-side-covered rule priced this walk at min(1, 10_000) = 1,
    // exactly the dishonesty docs/architecture/30-execution.md killed.
    let mut occ_stats = stats(&[10_000, 1]);
    occ_stats[0].var_distincts = vec![(VarId(0), 5_000), (VarId(1), 10_000)];
    let order = plan(&normalized, &schema, &occ_stats);
    assert_eq!(order.order, vec![OccId(1), OccId(0)]);
    // Step estimates: 1 survivor, then 1 x fanout(10_000 / 5_000) = 2.
    assert_eq!(order.estimates, vec![1, 2]);
}

#[test]
fn non_key_join_is_priced_pessimistically_and_pushed_last() {
    // Occs 0-1 join on occ 1's serial key; occ 2 shares a non-key var
    // with occ 0. Pessimism must order occ 2 last.
    let schema = schema(3, 3);
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, vec![(0, 0), (1, 1)]),
            occurrence(1, 1, vec![(0, 1)]),
            occurrence(2, 2, vec![(1, 2), (2, 0)]),
        ],
        residuals: vec![],
    };
    // Wait: occ 2 shares var 0 with occ 0 — var 0 is occ 0's serial
    // field, so the prefix side is covered. Rebind: occ 2 joins on a
    // non-serial field of occ 0 (var 1 is serial-of-occ-1...). Use a
    // genuinely non-key shared var: occ 0 field 2 = var 3, occ 2 field
    // 1 = var 3.
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, vec![(0, 0), (1, 1), (2, 3)]),
            occurrence(1, 1, vec![(0, 1)]),
            occurrence(2, 2, vec![(1, 3)]),
        ],
        ..normalized
    };
    let order = plan(&normalized, &schema, &stats(&[100, 50, 40]));
    assert_eq!(*order.order.last().expect("nonempty"), OccId(2));
    // The last step is the pessimistic product.
    let last = *order.estimates.last().expect("nonempty");
    assert_eq!(last, order.estimates[1].saturating_mul(40).min(last));
    assert!(last >= 40, "non-key join priced as a product");
}

#[test]
fn unique_coverage_fires_through_the_serial_auto_unique() {
    // Two occurrences joined on var 0 = occ 1's serial field: joining
    // occ 1 INTO occ 0 must estimate |occ 0| (an FK walk), not a
    // product.
    let schema = schema(2, 2);
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, vec![(1, 0)]),
            occurrence(1, 1, vec![(0, 0)]),
        ],
        residuals: vec![],
    };
    let order = plan(&normalized, &schema, &stats(&[70, 500]));
    assert_eq!(order.order, vec![OccId(0), OccId(1)]);
    assert_eq!(order.estimates, vec![70, 70]);
}

#[test]
fn dp_beats_greedy_on_a_constructed_counterexample() {
    // A(x big), B(serial x, y), C(y), D(y): greedy grabs the cheapest
    // immediate pair (C x D, a small product) and pays for it; the DP
    // routes through B's serial key.
    let schema = schema(4, 2);
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, vec![(1, 0)]),         // A: x, non-key
            occurrence(1, 1, vec![(0, 0), (1, 1)]), // B: serial x, y
            occurrence(2, 2, vec![(1, 1)]),         // C: y, non-key
            occurrence(3, 3, vec![(1, 1)]),         // D: y, non-key
        ],
        residuals: vec![],
    };
    let occ_stats = stats(&[10, 10, 2, 2]);

    let planned = plan(&normalized, &schema, &occ_stats);
    let planned_order: Vec<usize> = planned.order.iter().map(|o| usize::from(o.0)).collect();
    let planned_cost = order_cost(&normalized, &schema, &occ_stats, &planned_order);

    // Brute force: the DP result must be a global optimum.
    let mut best = u64::MAX;
    let mut permutations = vec![];
    permute(&mut vec![0, 1, 2, 3], 0, &mut permutations);
    for p in &permutations {
        best = best.min(order_cost(&normalized, &schema, &occ_stats, p));
    }
    assert_eq!(planned_cost, best, "DP finds the optimum");

    // Greedy (min immediate estimate at each step) is provably worse on
    // this fixture — the counterexample is real.
    let greedy = greedy_order(&normalized, &schema, &occ_stats);
    let greedy_cost = order_cost(&normalized, &schema, &occ_stats, &greedy);
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
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, vec![(0, 0)]),
            occurrence(1, 1, vec![(0, 0)]),
            occurrence(2, 2, vec![(1, 0)]),
        ],
        residuals: vec![],
    };
    let forward = stats(&[10, 10, 10]);
    let mut shuffled = forward.clone();
    shuffled.reverse();
    let a = plan(&normalized, &schema, &forward);
    let b = plan(&normalized, &schema, &shuffled);
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
    let normalized = NormalizedQuery {
        occurrences,
        residuals: vec![],
    };
    let order = plan(&normalized, &schema, &occ_stats);
    assert_eq!(order.order.len(), 16);
}
