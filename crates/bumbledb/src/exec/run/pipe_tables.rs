//! The pipelined executor's static shape tables.
use super::{CursorSrc, PipeTables, ValidatedPlan};

impl PipeTables {
    pub(super) fn of(plan: &ValidatedPlan) -> Self {
        let n_nodes = plan.nodes().len();
        let n_occ = plan.occurrences().len();
        // Only positive occurrences acquire levels. The initial zero for an
        // unused occurrence never makes it live because its level stays zero.
        let mut last_use = vec![0; n_occ];
        for (node_idx, node) in plan.nodes().iter().enumerate() {
            for occ in node
                .subatoms
                .iter()
                .map(|sub| sub.occ)
                .chain(node.point_probes.iter().map(|probe| probe.occ))
            {
                last_use[usize::from(occ.0)] = node_idx;
            }
        }

        let mut levels = vec![0; n_occ];
        let mut entry_level = Vec::with_capacity(n_nodes);
        let mut carried = Vec::with_capacity(n_nodes);
        let mut outgoing = Vec::with_capacity(n_nodes);
        for (node_idx, node) in plan.nodes().iter().enumerate() {
            let mut occs = Vec::new();
            let mut sources = vec![CursorSrc::Start; n_occ];
            for (occ, &level) in levels.iter().enumerate() {
                if level > 0 && last_use[occ] >= node_idx {
                    sources[occ] = CursorSrc::Carried(occs.len());
                    occs.push(occ);
                }
            }
            entry_level.push(levels.clone());
            // A current subatom replaces its incoming cursor with the matched
            // child, independently of which subatom becomes the batch cover.
            for (sub_idx, subatom) in node.subatoms.iter().enumerate() {
                let occ = usize::from(subatom.occ.0);
                sources[occ] = CursorSrc::Subatom(sub_idx);
                levels[occ] += 1;
            }
            carried.push(occs);
            outgoing.push(sources);
        }
        let absorb = (0..n_nodes)
            .rev()
            .find(|&m| plan.nodes()[m].suffix_skip == crate::plan::fj::SuffixSkip::Forbidden)
            .map_or(super::SkipAbsorb::Root, super::SkipAbsorb::Node);
        Self {
            entry_level,
            carried,
            outgoing,
            absorb,
        }
    }

    pub(super) fn carried_index(&self, node: usize, occ: usize) -> Option<usize> {
        self.carried[node].iter().position(|&o| o == occ)
    }
}
