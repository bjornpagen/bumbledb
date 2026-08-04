use super::{FjPlan, Node, Subatom};
use crate::ir::VarId;
use std::collections::BTreeSet;

/// The fold-aware level split (the scan-fold pushdown's planner half):
/// under an aggregate head, a node whose opening subatom mixes
/// group-key variables with fold-domain variables splits into two
/// nodes — the group variables first as their own prefix level, the
/// fold-domain remainder (with the node's lookups) after — so the
/// leaf's scan runs are group-constant and the aggregate sink's
/// scan-fold pushdown can fire (`exec/sink/aggregate/sink.rs`
/// `begin_scan` declines any group word among the scan's key slots;
/// the single-atom GROUP BY otherwise puts every group variable in the
/// one flat leaf level). The split mints no machinery — the two-node
/// shape is exactly the dimension-bound form the pushdown already
/// serves. `estimates` stays node-aligned: the two nodes cover one DP
/// step, so its estimate duplicates.
///
/// The node's lookups partition by fold-domain contact: a lookup
/// touching no fold variable moves to the group-prefix node — probed
/// once per **group**, never once per fold element — where it is also
/// a second cover candidate for the dynamic cover choice (`gj_split`
/// cannot rescue it later: a lookup whose variables all bind at one
/// node is exactly the shape its split skips, so left behind it stays
/// behind). Lookups touching the fold domain stay with the suffix.
pub fn fold_split(plan: &mut FjPlan, group: &BTreeSet<VarId>, estimates: &mut Vec<u64>) {
    let mut i = 0;
    while i < plan.nodes.len() {
        let opening = &plan.nodes[i].subatoms[0];
        let (group_vars, fold_vars): (Vec<VarId>, Vec<VarId>) = opening
            .vars
            .iter()
            .copied()
            .partition(|v| group.contains(v));
        if group_vars.is_empty() || fold_vars.is_empty() {
            i += 1;
            continue;
        }
        let occ = opening.occ;
        let node = plan.nodes.remove(i);
        let mut prefix = vec![Subatom {
            occ,
            vars: group_vars,
        }];
        let mut suffix = vec![Subatom {
            occ,
            vars: fold_vars,
        }];
        for lookup in node.subatoms.into_iter().skip(1) {
            // Contact with the suffix opening's variables is the one
            // legality question: everything else the lookup reads is
            // bound at or above the prefix node by construction.
            if lookup.vars.iter().any(|v| suffix[0].vars.contains(v)) {
                suffix.push(lookup);
            } else {
                prefix.push(lookup);
            }
        }
        plan.nodes.insert(i, Node { subatoms: suffix });
        plan.nodes.insert(i, Node { subatoms: prefix });
        if i < estimates.len() {
            let estimate = estimates[i];
            estimates.insert(i, estimate);
        }
        i += 2;
    }
}
