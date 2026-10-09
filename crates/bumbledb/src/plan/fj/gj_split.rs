use super::{FjPlan, Subatom};
use crate::ir::VarId;
use std::collections::BTreeMap;

/// The GJ split: after `factor`, a probe subatom carrying two or more
/// variables first bound at different nodes splits into one lookup subatom
/// per such node, placed at that node. This
/// moves cyclic rules to the Generic Join end of the Free Join spectrum and
/// gives a node its second cover; acyclic plans pass through unchanged.
pub(crate) fn gj_split(plan: &mut FjPlan) {
    // First-bound node per variable; the split never changes it.
    let mut first_bound: BTreeMap<VarId, usize> = BTreeMap::new();
    for (node_idx, node) in plan.nodes.iter().enumerate() {
        for subatom in &node.subatoms {
            for var in &subatom.vars {
                first_bound.entry(*var).or_insert(node_idx);
            }
        }
    }
    for i in 0..plan.nodes.len() {
        let mut s = 0;
        while s < plan.nodes[i].subatoms.len() {
            let vars = &plan.nodes[i].subatoms[s].vars;
            if vars.iter().all(|v| first_bound[v] == first_bound[&vars[0]]) {
                s += 1;
                continue;
            }
            let subatom = plan.nodes[i].subatoms.remove(s);

            let mut lookups: BTreeMap<usize, Vec<VarId>> = BTreeMap::new();
            for var in subatom.vars {
                lookups.entry(first_bound[&var]).or_default().push(var);
            }
            for (node, vars) in lookups {
                plan.nodes[node].subatoms.push(Subatom {
                    occ: subatom.occ,
                    vars,
                });
            }
        }
    }
}
