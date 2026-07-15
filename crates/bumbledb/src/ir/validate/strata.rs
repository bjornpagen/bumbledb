//! The strata judge — the stratification fence (R2;
//! `docs/architecture/20-query-ir.md` § engine recursion, the Lean truth
//! `lean/Bumbledb/Query/Syntax.lean: Program.StratifiedBy` and the
//! monotonicity it buys, `lean/Bumbledb/Exec/Fixpoint.lean:
//! stratumOp_mono`).
//!
//! **The predicate dependency graph.** One node per `PredId`; an edge
//! P → Q for every body atom of a P-rule over predicate Q — positive or
//! negated, the label carried by the occurrence's position exactly as
//! negation is a position in a rule. The graph is bounded by
//! [`crate::ir::MAX_PREDICATES`] × per-predicate
//! [`crate::ir::MAX_RULES`] — boundary caps documented at their
//! definitions, checked before this judge runs.
//!
//! **SCC condensation, iteratively.** Strata are the condensation's
//! topological order, computed with an explicit work-list (iterative
//! Tarjan) — the trust-boundary law's convention: the nesting-depth
//! judge is already iterative *because* the walks it guards recurse,
//! and the strata judge follows it. Mutual recursion within one SCC is
//! ordinary: the stratum's predicates iterate jointly under one round
//! loop (the fixpoint driver, `api/prepared/fixpoint.rs`).
//!
//! **The refusals, typed** — the safety roster whose theorem is
//! termination (`lean/Bumbledb/Exec/Fixpoint.lean: program_den_finite`;
//! the walls on the other side are `lean/Bumbledb/Countermodels.lean:
//! odd_not_monotone` and `succ_prefixed_infinite`):
//!
//! - `NegationThroughCycle` — a negated atom whose target shares the
//!   atom's own SCC. Negation *of* lower strata is legal: a lower
//!   stratum is a finished set before this stratum's operator runs.
//! - `AggregationThroughCycle` — a fold in a head whose rule body reads
//!   the head's own SCC. Aggregation *of* lower strata is legal for the
//!   same reason: an `Idb` atom under a fold reads a finished set.
//! - `MeasureInRecursiveHead` — a `Measure` find in a recursive
//!   predicate's head: the measure is a computation, not a binding, so
//!   it exits the bound-variables-only premise; and the error-timing
//!   ruling (the ray error must not depend on iteration order) pins the
//!   refusal at the boundary.
//!
//! Together the roster keeps recursive heads projecting **bound
//! variables only** — the creation quarantine
//! (`docs/architecture/20-query-ir.md` § the creation quarantine)
//! restated for fixpoint topology: one law, two enforcement sites.
//!
//! The judge also runs the **well-formedness screen** first
//! (`lean/Bumbledb/Query/Syntax.lean: Program.WellFormed`, spent by
//! `lean/Bumbledb/Exec/Fixpoint.lean: wellFormed_reads_real`): every
//! `Idb` source names a real predicate and addresses within its arity —
//! without the screen a NEGATED phantom read would be vacuously
//! satisfied, and `StratifiedBy` alone never refuses the shape (a
//! stratum witness may map the phantom low).

use crate::error::ValidationError;
use crate::ir::normalize::LoweredRule;
use crate::ir::{FindTerm, PredId};

/// Judges the program's dependency graph: the screen, the SCC
/// condensation, and the safety roster — returning the stratification
/// witness (`strata[p]` = predicate `p`'s condensation index: positive
/// edges non-increasing, negated edges strictly decreasing, the judged
/// form of `Program.StratifiedBy`).
///
/// # Errors
///
/// `UnknownPredicate` / `PredicateColumnOutOfRange` (the screen), then
/// `NegationThroughCycle` / `AggregationThroughCycle` /
/// `MeasureInRecursiveHead` (the safety roster), first failing
/// predicate first.
pub(super) fn stratify(
    arities: &[usize],
    predicates: &[Vec<LoweredRule>],
) -> Result<Box<[u16]>, ValidationError> {
    // The screen: every `Idb` source in range, every binding inside the
    // target's arity — positive and negated occurrences alike, numbered
    // as diagnostics number them (positives first).
    for rules in predicates {
        for rule in rules {
            let occurrences = rule.atoms.iter().chain(&rule.negated);
            for (occ_idx, atom) in occurrences.enumerate() {
                let Some(pred) = atom.source.idb() else {
                    continue;
                };
                let Some(arity) = arities.get(usize::from(pred.0)) else {
                    return Err(ValidationError::UnknownPredicate {
                        atom: occ_idx,
                        pred,
                    });
                };
                for (field, _) in &atom.bindings {
                    if usize::from(field.0) >= *arity {
                        return Err(ValidationError::PredicateColumnOutOfRange {
                            atom: occ_idx,
                            field: *field,
                        });
                    }
                }
            }
        }
    }

    // The dependency graph: successors per predicate (positive and
    // negated edges merged for the condensation; the refusals below
    // re-read the position that labels them).
    let successors: Vec<Vec<usize>> = predicates
        .iter()
        .map(|rules| {
            let mut targets: Vec<usize> = rules
                .iter()
                .flat_map(|rule| rule.atoms.iter().chain(&rule.negated))
                .filter_map(|atom| atom.source.idb())
                .map(|pred| usize::from(pred.0))
                .collect();
            targets.sort_unstable();
            targets.dedup();
            targets
        })
        .collect();
    let scc = condense(&successors);
    let mut scc_sizes = vec![0usize; predicates.len()];
    for &component in &scc {
        scc_sizes[component] += 1;
    }

    // The safety roster, per predicate in id order.
    for (index, rules) in predicates.iter().enumerate() {
        let pred = PredId(u16::try_from(index).expect("predicate count capped at 16"));
        let recursive = scc_sizes[scc[index]] > 1 || successors[index].contains(&index);
        for rule in rules {
            for atom in &rule.negated {
                if let Some(via) = atom.source.idb()
                    && scc[usize::from(via.0)] == scc[index]
                {
                    return Err(ValidationError::NegationThroughCycle { pred, via });
                }
            }
            let has_fold = rule.finds.iter().any(|term| {
                matches!(
                    term,
                    FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. }
                )
            });
            if has_fold {
                for atom in rule.atoms.iter().chain(&rule.negated) {
                    if let Some(via) = atom.source.idb()
                        && scc[usize::from(via.0)] == scc[index]
                    {
                        return Err(ValidationError::AggregationThroughCycle { pred, via });
                    }
                }
            }
            if recursive
                && rule
                    .finds
                    .iter()
                    .any(|term| matches!(term, FindTerm::Measure(_)))
            {
                return Err(ValidationError::MeasureInRecursiveHead { pred });
            }
        }
    }

    Ok(scc
        .into_iter()
        .map(|component| u16::try_from(component).expect("component count capped at 16"))
        .collect())
}

/// Iterative Tarjan SCC condensation. Components are numbered in
/// completion order, which is reverse-topological along the reads
/// relation: an edge P → Q crossing components lands `scc[Q] < scc[P]`,
/// so the component index IS the stratification witness — positive
/// targets sit no higher, and (post-roster) negated targets strictly
/// lower. Explicit work-list frames, no recursion — the iterative-judge
/// convention (module doc).
fn condense(successors: &[Vec<usize>]) -> Vec<usize> {
    const UNVISITED: usize = usize::MAX;
    let count = successors.len();
    let mut index = vec![UNVISITED; count];
    let mut low = vec![0usize; count];
    let mut on_stack = vec![false; count];
    let mut stack: Vec<usize> = Vec::new();
    let mut scc = vec![0usize; count];
    let mut next_index = 0usize;
    let mut components = 0usize;
    // One frame per open node: (node, next successor cursor).
    let mut frames: Vec<(usize, usize)> = Vec::new();
    for start in 0..count {
        if index[start] != UNVISITED {
            continue;
        }
        index[start] = next_index;
        low[start] = next_index;
        next_index += 1;
        stack.push(start);
        on_stack[start] = true;
        frames.push((start, 0));
        while let Some(&(node, cursor)) = frames.last() {
            if let Some(&next) = successors[node].get(cursor) {
                frames.last_mut().expect("frame just read").1 += 1;
                if index[next] == UNVISITED {
                    index[next] = next_index;
                    low[next] = next_index;
                    next_index += 1;
                    stack.push(next);
                    on_stack[next] = true;
                    frames.push((next, 0));
                } else if on_stack[next] {
                    low[node] = low[node].min(index[next]);
                }
            } else {
                frames.pop();
                if let Some(&(parent, _)) = frames.last() {
                    low[parent] = low[parent].min(low[node]);
                }
                if low[node] == index[node] {
                    while let Some(member) = stack.pop() {
                        on_stack[member] = false;
                        scc[member] = components;
                        if member == node {
                            break;
                        }
                    }
                    components += 1;
                }
            }
        }
    }
    scc
}

#[cfg(test)]
mod tests {
    use super::condense;

    #[test]
    fn condensation_orders_components_reverse_topologically() {
        // 0 → 1 → 2, 2 → 1 (cycle {1, 2}), 3 isolated.
        let successors = vec![vec![1], vec![2], vec![1], vec![]];
        let scc = condense(&successors);
        assert_eq!(scc[1], scc[2], "the cycle is one component");
        assert!(scc[1] < scc[0], "a reader sits strictly above its read");
        assert_ne!(scc[3], scc[0]);
    }

    #[test]
    fn self_loop_is_its_own_component() {
        let successors = vec![vec![0], vec![0]];
        let scc = condense(&successors);
        assert_ne!(scc[0], scc[1]);
        assert!(scc[0] < scc[1]);
    }

    #[test]
    fn diamond_reaches_every_node_once() {
        // 0 → {1, 2} → 3.
        let successors = vec![vec![1, 2], vec![3], vec![3], vec![]];
        let scc = condense(&successors);
        assert!(scc[3] < scc[1] && scc[3] < scc[2]);
        assert!(scc[1] < scc[0] && scc[2] < scc[0]);
    }
}
