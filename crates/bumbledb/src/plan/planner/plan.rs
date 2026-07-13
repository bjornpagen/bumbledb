use super::{JoinOrder, MAX_OCCURRENCES, OccStats, State, densify::densify, estimate::estimate};
use crate::ir::normalize::{NormalizedQuery, OccId, Occurrence};
use crate::schema::Schema;

/// Plans a left-deep join order by exhaustive DP over **participating**
/// occurrence subsets, minimizing the sum of intermediate-result
/// estimates. Negated occurrences enter no DP state — they never join;
/// they only shrink results, and the planner treats them as free filters
/// (docs/architecture/40-execution.md). Chase-eliminated occurrences left
/// planning entirely (`plan/chase.rs`). Deterministic: ties break toward
/// the smaller trailing occurrence id, independent of `stats` input order.
///
/// # Panics
///
/// Only on programmer-invariant violations: `stats` missing a
/// participating occurrence, or a query over the caps the validation
/// boundary enforces.
pub fn plan(normalized: &NormalizedQuery, schema: &Schema, stats: &[OccStats]) -> JoinOrder {
    let participating: Vec<&Occurrence> = normalized
        .occurrences
        .iter()
        .filter(|o| o.role.participates())
        .collect();
    let n = participating.len();
    debug_assert!(
        n <= MAX_OCCURRENCES,
        "validation rejects over-cap queries at the boundary"
    );
    let occs = densify(&participating, schema, stats);

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
    // Per-mask prefix-variable memo: vars(mask) folds once per mask
    // (the lowest bit's occurrence unioned with the rest), so the inner
    // candidate loop reads one entry instead of refolding all n
    // occurrences per (mask, last) pair — the audit's O(2ⁿ·n²) note.
    let mut mask_vars: Vec<u128> = vec![0; (full as usize) + 1];
    for mask in 1..=full {
        let low = usize::try_from(mask.trailing_zeros()).expect("small");
        mask_vars[mask as usize] = mask_vars[(mask & (mask - 1)) as usize] | occs[low].vars;
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
            let est = estimate(prev.est, mask_vars[prev_mask as usize], &occs, last);
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
        order[step] = participating[usize::from(chosen.last)].occ_id;
        estimates[step] = chosen.est;
        mask &= !(1 << chosen.last);
    }
    JoinOrder { order, estimates }
}
