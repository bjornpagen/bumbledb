use super::FjPlan;
use crate::ir::VarId;
use std::collections::BTreeSet;

/// The paper's Fig. 8 conservative hoist: traverse nodes in reverse; move a
/// *lookup* subatom (never a node's first subatom — that is its opened
/// iterate) to the previous node iff its variables are all available
/// before this node and the previous node lacks that occurrence, stopping
/// per node at the first non-hoistable lookup (preserving the probe order
/// the cost-based order implies).
pub fn factor(plan: &mut FjPlan) {
    for i in (1..plan.nodes.len()).rev() {
        let available: BTreeSet<VarId> = plan.nodes[..i]
            .iter()
            .flat_map(|n| n.subatoms.iter())
            .flat_map(|s| s.vars.iter().copied())
            .collect();
        // Lookups start at index 1; hoisting shifts the next lookup into
        // index 1, so the loop re-examines that slot.
        while plan.nodes[i].subatoms.len() > 1 {
            let candidate = &plan.nodes[i].subatoms[1];
            let hoistable = candidate.vars.iter().all(|v| available.contains(v))
                && !plan.nodes[i - 1]
                    .subatoms
                    .iter()
                    .any(|s| s.occ == candidate.occ);
            if !hoistable {
                break;
            }
            let moved = plan.nodes[i].subatoms.remove(1);
            plan.nodes[i - 1].subatoms.push(moved);
        }
    }
}
