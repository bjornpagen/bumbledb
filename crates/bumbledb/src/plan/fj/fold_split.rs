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
pub fn fold_split(plan: &mut FjPlan, group: &BTreeSet<VarId>) {
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
            if lookup.vars.iter().any(|v| suffix[0].vars.contains(v)) {
                suffix.push(lookup);
            } else {
                prefix.push(lookup);
            }
        }
        let estimate = node.estimate;
        plan.nodes.insert(
            i,
            Node {
                subatoms: suffix,
                estimate,
            },
        );
        plan.nodes.insert(
            i,
            Node {
                subatoms: prefix,
                estimate,
            },
        );
        i += 2;
    }
}
