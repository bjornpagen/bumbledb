use super::OccInfo;

/// One join step's cardinality: the prefix estimate times the new
/// occurrence's per-binding **fanout** (docs/architecture/30-execution.md). A disconnected
/// occurrence is a cross product. A connected one contributes
/// `rows / distinct(field of v)` for its most selective join variable —
/// FK walks fan out by rows-per-key instead of the old
/// `min(prefix, rows)` rule, which priced a 200-postings-per-account
/// walk as 1 and misled EXPLAIN by 12,703x on the balance family. A
/// unique constraint covered by the join variables pins the fanout to 1
/// (compound uniques included — per-var distincts cannot see those).
pub(super) fn estimate(prefix_est: u64, prefix_vars: u128, occs: &[OccInfo], last: usize) -> u64 {
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
