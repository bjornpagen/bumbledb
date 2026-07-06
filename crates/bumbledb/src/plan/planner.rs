//! Statistics and the DP planner (docs/architecture/30-execution.md): real statistics in, one
//! left-deep atom order out (`docs/architecture/30-execution.md`).
//!
//! Statistics are exact row counts (or measured filtered-view survivor
//! counts) plus schema constraint knowledge — nothing else exists: no NDV
//! fields, no histograms, no magic selectivity constants (the post-mortem's
//! central engine finding, §30).

use crate::ir::normalize::{NormalizedQuery, OccId};
use crate::ir::VarId;
use crate::schema::Schema;

/// Hard cap on occurrences the exhaustive subset DP accepts. The 30-execution doc named
/// 32 (the bitmask width), but 2³² DP states is ~170 GB of table — memory-
/// infeasible; 2²⁰ is ~24 MB and instant, and the doc's own envelope is
/// "≤ ~12 atoms" (amendment recorded in docs/architecture/30-execution.md).
pub const MAX_OCCURRENCES: usize = 20;

/// Distinct-variable cap for the planner's dense var bitsets.
pub(crate) const MAX_DISTINCT_VARS: usize = 128;

/// The planner's per-occurrence statistics (docs/architecture/30-execution.md): the
/// selectivity-shaped cardinality estimate, plus the base-relation
/// distinct count of every bound variable's field (from the same
/// ladder — unique-exact, image-exact, schema bounds, floor). The
/// distincts drive the join-step fanout model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OccStats {
    pub occ_id: OccId,
    /// Estimated cardinality after this occurrence's own predicates.
    pub rows: u64,
    /// `(var, distinct count of its field over the base relation)`.
    pub var_distincts: Vec<(VarId, u64)>,
}

/// The chosen left-deep join order, with per-step estimates retained for
/// EXPLAIN (docs/architecture/30-execution.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinOrder {
    /// Occurrences in join order (first = the iterated relation).
    pub order: Vec<OccId>,
    /// The estimator's cardinality after each step; `estimates[0]` is the
    /// first occurrence's row count.
    pub estimates: Vec<u64>,
}

/// One DP table entry: cheapest left-deep plan covering the mask.
#[derive(Clone, Copy)]
struct State {
    cost: u64,
    est: u64,
    last: u8,
}

/// Per-occurrence planning inputs, densified.
struct OccInfo {
    rows: u64,
    /// This occurrence's variables as a dense bitset.
    vars: u128,
    /// `(var bit, base-relation distinct count of its field)` — the
    /// join-step fanout inputs (docs/architecture/30-execution.md).
    var_distincts: Vec<(u128, u64)>,
    /// Var bitsets of unique constraints whose every field is var-bound in
    /// this occurrence (constraints with literal-bound fields are skipped —
    /// simple and faithful to the doc's estimator).
    unique_var_sets: Vec<u128>,
}

/// One join step's cardinality: the prefix estimate times the new
/// occurrence's per-binding **fanout** (docs/architecture/30-execution.md). A disconnected
/// occurrence is a cross product. A connected one contributes
/// `rows / distinct(field of v)` for its most selective join variable —
/// FK walks fan out by rows-per-key instead of the old
/// `min(prefix, rows)` rule, which priced a 200-postings-per-account
/// walk as 1 and misled EXPLAIN by 12,703x on the balance family. A
/// unique constraint covered by the join variables pins the fanout to 1
/// (compound uniques included — per-var distincts cannot see those).
fn estimate(prefix_est: u64, prefix_vars: u128, occs: &[OccInfo], last: usize) -> u64 {
    let r = &occs[last];
    let join_vars = r.vars & prefix_vars;
    if join_vars == 0 {
        return prefix_est.saturating_mul(r.rows);
    }
    if r.unique_var_sets.iter().any(|set| set & join_vars == *set) {
        return prefix_est;
    }
    let fanout = r
        .var_distincts
        .iter()
        .filter(|(bit, _)| bit & join_vars != 0)
        .map(|(_, distinct)| (r.rows / (*distinct).clamp(1, r.rows.max(1))).max(1))
        .min()
        // A join var with no recorded distinct (hand-built stats): the
        // pessimistic product, exactly as before this model existed —
        // optimism without evidence is how plans go wrong.
        .unwrap_or_else(|| r.rows.max(1));
    prefix_est.saturating_mul(fanout)
}

/// Plans a left-deep join order by exhaustive DP over occurrence subsets,
/// minimizing the sum of intermediate-result estimates. Deterministic:
/// ties break toward the smaller trailing occurrence id, independent of
/// `stats` input order.
///
/// # Errors
///
/// `TooManyAtoms` above [`MAX_OCCURRENCES`]; `TooManyVariables` above 128
/// distinct variables (both documented planner caps).
///
/// # Panics
///
/// Only on programmer-invariant violations: `stats` missing an occurrence
/// the normalized query contains.
pub fn plan(normalized: &NormalizedQuery, schema: &Schema, stats: &[OccStats]) -> JoinOrder {
    let n = normalized.occurrences.len();
    debug_assert!(
        n <= MAX_OCCURRENCES,
        "validation rejects over-cap queries at the boundary"
    );
    let occs = densify(normalized, schema, stats);

    // Exhaustive left-deep DP; the cost is the sum of every prefix estimate
    // including the base relation's rows (the root iteration is real work,
    // and counting it breaks ties toward iterating the small side).
    let full = (1u32 << n) - 1;
    let mut best: Vec<Option<State>> = vec![None; (full as usize) + 1];
    for (i, occ) in occs.iter().enumerate() {
        best[1 << i] = Some(State {
            cost: occ.rows,
            est: occ.rows,
            last: u8::try_from(i).expect("n <= 20"),
        });
    }
    for mask in 1..=full {
        if mask.count_ones() < 2 {
            continue;
        }
        let mut candidate: Option<State> = None;
        for last in 0..n {
            if mask & (1 << last) == 0 {
                continue;
            }
            let prev_mask = mask & !(1 << last);
            let prev = best[prev_mask as usize].expect("smaller masks filled first");
            let prefix_vars = (0..n)
                .filter(|i| prev_mask & (1 << i) != 0)
                .fold(0u128, |acc, i| acc | occs[i].vars);
            let est = estimate(prev.est, prefix_vars, &occs, last);
            let cost = prev.cost.saturating_add(est);
            let better = match candidate {
                None => true,
                // Strict less: ties keep the earlier (smaller) last id.
                Some(existing) => cost < existing.cost,
            };
            if better {
                candidate = Some(State {
                    cost,
                    est,
                    last: u8::try_from(last).expect("n <= 20"),
                });
            }
        }
        best[mask as usize] = candidate;
    }

    // Reconstruct the order back-to-front.
    let mut order = vec![OccId(0); n];
    let mut estimates = vec![0u64; n];
    let mut mask = full;
    for step in (0..n).rev() {
        let chosen = best[mask as usize].expect("full DP table");
        order[step] = normalized.occurrences[usize::from(chosen.last)].occ_id;
        estimates[step] = chosen.est;
        mask &= !(1 << chosen.last);
    }
    JoinOrder { order, estimates }
}

/// Densifies occurrences into bitset form, resolving stats and translating
/// unique-constraint field sets to variable sets.
fn densify(normalized: &NormalizedQuery, schema: &Schema, stats: &[OccStats]) -> Vec<OccInfo> {
    let mut var_index: std::collections::BTreeMap<VarId, usize> = std::collections::BTreeMap::new();
    for occurrence in &normalized.occurrences {
        for (_, var) in &occurrence.vars {
            let next = var_index.len();
            var_index.entry(*var).or_insert(next);
        }
    }
    debug_assert!(
        var_index.len() <= MAX_DISTINCT_VARS,
        "validation rejects over-cap queries at the boundary"
    );
    normalized
        .occurrences
        .iter()
        .map(|occurrence| {
            let stat = stats
                .iter()
                .find(|s| s.occ_id == occurrence.occ_id)
                .expect("stats cover every occurrence");
            let rows = stat.rows;
            let mut vars = 0u128;
            for (_, var) in &occurrence.vars {
                vars |= 1 << var_index[var];
            }
            let var_distincts: Vec<(u128, u64)> = stat
                .var_distincts
                .iter()
                .map(|(var, distinct)| (1u128 << var_index[var], *distinct))
                .collect();
            // Translate each unique constraint's field set to a var bitset;
            // skip constraints with any non-var-bound field.
            let relation = schema.relation(occurrence.relation);
            let unique_var_sets = relation
                .unique_constraints()
                .iter()
                .filter_map(|cid| {
                    let mut set = 0u128;
                    for field in relation.constraint(*cid).fields() {
                        let (_, var) = occurrence.vars.iter().find(|(f, _)| f == field)?;
                        set |= 1 << var_index[var];
                    }
                    Some(set)
                })
                .collect();
            OccInfo {
                rows,
                vars,
                var_distincts,
                unique_var_sets,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::view::{Const, FilterPredicate};
    use crate::ir::normalize::Occurrence;
    use crate::ir::CmpOp as ViewCmp;
    use crate::schema::{
        FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
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
}
