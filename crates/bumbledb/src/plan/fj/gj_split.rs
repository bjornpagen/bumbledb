use super::{FjPlan, Subatom};
use crate::ir::VarId;
use std::collections::BTreeMap;

/// The GJ split: after `factor`, a probe subatom carrying two or
/// per-variable lookup subatoms, each placed at the node where its
/// end of the Free Join spectrum for cyclic rules, and the step that
/// gives a production node its second cover (under `binary2fj` +
/// `factor` alone every node has exactly one, so dynamic cover choice
/// never has a choice). Acyclic plans carry no such subatom and pass
/// through unchanged. The split mints no machinery: trie schemas derive
/// from the split subatoms per §3.3, the partition check admits one
pub fn gj_split(plan: &mut FjPlan) {
    // First-bound node per variable. Invariant under the split: a

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
