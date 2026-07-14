//! The pipelined executor's static shape tables.

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
        // Cursor USES extend appearances: a membership probe reads its
        // occurrence's advanced cursor at the node it attaches to, so
        // pending entries must carry that cursor there even when the
        // occurrence's own subatoms all sit earlier.
        let mut uses = appears.clone();
        for (node_idx, node) in plan.nodes().iter().enumerate() {
            for probe in &node.point_probes {
                uses[usize::from(probe.occ.0)][node_idx] = true;
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
                let at_or_after = uses[occ][node_idx..].iter().any(|b| *b);
                if before && at_or_after {
                    cols[occ] = Some(occs.len());
                    occs.push(occ);
                }
            }
            entry_level.push(levels);
            carried.push(occs);
            carried_col.push(cols);
        }
        let absorb = (0..n_nodes)
            .rev()
            .find(|&m| plan.nodes()[m].suffix_skip == crate::plan::fj::SuffixSkip::Forbidden);
        Self {
            entry_level,
            carried,
            carried_col,
            absorb,
        }
    }
}
