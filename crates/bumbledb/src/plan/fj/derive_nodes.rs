use super::{FjPlan, PlanError, PlanNode};
use crate::ir::VarId;
use std::collections::BTreeSet;

pub(super) fn derive_nodes(plan: &FjPlan) -> Result<Vec<PlanNode>, PlanError> {
    let mut nodes = Vec::with_capacity(plan.nodes.len());
    let mut available: BTreeSet<VarId> = BTreeSet::new();
    for (node_idx, node) in plan.nodes.iter().enumerate() {
        for (idx, subatom) in node.subatoms.iter().enumerate() {
            if node.subatoms[..idx].iter().any(|s| s.occ == subatom.occ) {
                return Err(PlanError::DuplicateOccurrenceInNode {
                    node: node_idx,
                    occ: subatom.occ,
                });
            }
        }
        let node_vars: BTreeSet<VarId> = node
            .subatoms
            .iter()
            .flat_map(|s| s.vars.iter().copied())
            .collect();
        let new_vars: Vec<VarId> = node_vars
            .iter()
            .copied()
            .filter(|v| !available.contains(v))
            .collect();

        let covers: Vec<u8> = node
            .subatoms
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.vars.len() == new_vars.len() && new_vars.iter().all(|v| s.vars.contains(v))
            })
            .map(|(i, _)| u8::try_from(i).expect("subatoms per node fit u8"))
            .collect();
        if covers.is_empty() {
            return Err(PlanError::NoCover { node: node_idx });
        }
        available.extend(node_vars);
        nodes.push(PlanNode {
            subatoms: node.subatoms.clone(),
            covers,
            residuals: Vec::new(),
            word_residuals: Vec::new(),
            allen_residuals: Vec::new(),
            anti_probes: Vec::new(),
            point_probes: Vec::new(),
            new_vars,
            suffix_skip: super::SuffixSkip::Forbidden,
            estimate: node.estimate,
        });
    }
    Ok(nodes)
}
