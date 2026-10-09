use super::{AllenKeep, OccInfo};

pub(super) fn estimate(
    prefix_est: u64,
    prefix_vars: u128,
    occs: &[OccInfo],
    allen: &[AllenKeep],
    last: usize,
) -> u64 {
    let r = &occs[last];
    let join_vars = r.vars & prefix_vars;
    let base = if join_vars == 0 {
        prefix_est.saturating_mul(r.rows)
    } else if r.key_var_sets.iter().any(|set| set & join_vars == *set) {
        prefix_est
    } else {
        let fanout = r
            .var_distincts
            .iter()
            .filter(|(bit, _)| bit & join_vars != 0)
            .map(|(_, distinct)| (r.rows / (*distinct).clamp(1, r.rows.max(1))).max(1))
            .min()
            // pessimistic product, exactly as before this model existed —
            .unwrap_or_else(|| r.rows.max(1));
        prefix_est.saturating_mul(fanout)
    };
    let covered = prefix_vars | r.vars;
    let mut est = base;
    for keep in allen {
        if keep.vars & covered == keep.vars && keep.vars & prefix_vars != keep.vars {
            est = (est.saturating_mul(keep.keep_num) / keep.keep_den).max(1);
        }
    }
    est
}
