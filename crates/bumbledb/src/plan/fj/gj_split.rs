use super::{FjPlan, Subatom};
use crate::ir::VarId;
use std::collections::BTreeMap;

/// The GJ split (docs/architecture/40-execution.md § search; ruled
/// 2026-07-23, R19): after `factor()`, a probe subatom carrying two or
/// more variables first bound at different nodes splits into
/// per-variable lookup subatoms, each placed at the node where its
/// variable is first bound — the lowering that produces plans at the GJ
/// end of the Free Join spectrum for cyclic rules, and the step that
/// gives a production node its second cover (under `binary2fj` +
/// `factor()` alone every node has exactly one, so dynamic cover choice
/// never has a choice). Acyclic plans carry no such subatom and pass
/// through unchanged. The split mints no machinery: trie schemas derive
/// from the split subatoms per §3.3, the partition check admits one
/// occurrence's variables spread across nodes, and carried cursors
/// route a multi-node occurrence forward.
pub fn gj_split(plan: &mut FjPlan) {
    // First-bound node per variable. Invariant under the split: a
    // variable's lookup lands exactly at its binding node, never earlier.
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
            // Per binding node, not literally per variable: two
            // variables bound at one node stay one lookup — a node
            // admits one subatom per occurrence, and a two-word probe
            // into one submap is the right access for them anyway.
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
