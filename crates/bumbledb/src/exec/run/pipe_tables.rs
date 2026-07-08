//! The pipelined executor's static shape tables (docs/perf/ PRD 09).

use super::{PipeTables, ValidatedPlan};

impl PipeTables {
    pub(super) fn of(plan: &ValidatedPlan) -> Self {
        let n_nodes = plan.nodes().len();
        let n_occ = plan.occurrences().len();
        let mut appears = vec![vec![false; n_nodes]; n_occ];
        for (node_idx, node) in plan.nodes().iter().enumerate() {
            for subatom in &node.subatoms {
                appears[usize::from(subatom.occ.0)][node_idx] = true;
            }
        }
        let mut entry_level = Vec::with_capacity(n_nodes);
        let mut carried = Vec::with_capacity(n_nodes);
        let mut carried_col = Vec::with_capacity(n_nodes);
        for node_idx in 0..n_nodes {
            let mut levels = Vec::with_capacity(n_occ);
            let mut occs = Vec::new();
            let mut cols = vec![None; n_occ];
            for (occ, at) in appears.iter().enumerate() {
                levels.push(at[..node_idx].iter().filter(|b| **b).count());
                let before = at[..node_idx].iter().any(|b| *b);
                let at_or_after = at[node_idx..].iter().any(|b| *b);
                if before && at_or_after {
                    cols[occ] = Some(occs.len());
                    occs.push(occ);
                }
            }
            entry_level.push(levels);
            carried.push(occs);
            carried_col.push(cols);
        }
        let absorb = (0..n_nodes).rev().find(|&m| plan.nodes()[m].sink_relevant);
        Self {
            entry_level,
            carried,
            carried_col,
            absorb,
        }
    }
}
