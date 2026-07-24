use super::{AllenKeep, OccInfo};

/// One join step's cardinality: the prefix estimate times the new
/// occurrence's per-binding **fanout** (docs/architecture/40-execution.md). A disconnected
/// occurrence is a cross product. A connected one contributes
/// `rows / distinct(field of v)` for its most selective join variable —
/// reference walks fan out by rows-per-key instead of the old
/// `min(prefix, rows)` rule, which priced a 200-postings-per-account
/// walk as 1 and misled introspection by 12,703x on the balance family. A
/// key (`Functionality` statement) whose projection is covered by the
/// join variables pins the fanout to 1 (compound keys included —
/// per-var distincts cannot see those). Coverage means the FULL
/// projection: for a pointwise key the interval field must be a join
/// variable bound by value — a scalar-prefix-only join takes the
/// general fanout below (the determinant is `densify`'s translation).
/// An `Allen` residual completed by this step then credits its mask's
/// measure ([`AllenKeep`]) — the one residual class priced at the join
/// step (R19): without it the canonical temporal join (two atoms
/// related only by a mask) priced as a bare Cartesian product.
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
            // A join var with no recorded distinct (hand-built stats): the
            // pessimistic product, exactly as before this model existed —
            // optimism without evidence is how plans go wrong.
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
