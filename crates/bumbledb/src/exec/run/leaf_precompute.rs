//! The single-subatom-leaf precompute.
use super::{LeafPrecompute, NodePrecompute, Source, ValidatedPlan};

impl LeafPrecompute {
    pub(super) fn of(
        plan: &ValidatedPlan,
        precompute: &[NodePrecompute],
        var_widths: &[(crate::ir::VarId, usize)],
    ) -> Self {
        let last = plan.nodes().len() - 1;
        let width_of = |var: crate::ir::VarId| -> usize {
            var_widths
                .iter()
                .find(|(v, _)| *v == var)
                .expect("plans bind every variable")
                .1
        };

        let single = plan.nodes()[last].subatoms.len() == 1
            && plan.nodes()[last].anti_probes.is_empty()
            && plan.nodes()[last].point_probes.is_empty()
            && plan.nodes()[last].word_residuals.is_empty()
            && plan.nodes()[last].allen_residuals.is_empty()
            && precompute[last]
                .residual_slots
                .iter()
                .all(|spec| spec.width == 1)
            && plan.nodes()[last].subatoms[0]
                .vars
                .iter()
                .all(|v| width_of(*v) == 1);
        if !single {
            return Self::Generic;
        }
        let cover_vars = &plan.nodes()[last].subatoms[0].vars;
        let mut scan_residuals = Vec::new();
        let mut const_residuals = Vec::new();
        for spec in &precompute[last].residual_slots {
            let resolve = |var: crate::ir::VarId, slot: usize| {
                cover_vars
                    .iter()
                    .position(|cv| *cv == var)
                    .map_or(Source::Slot(slot), Source::Batch)
            };
            let lhs = resolve(spec.lhs, spec.lhs_slot);
            let rhs = resolve(spec.rhs, spec.rhs_slot);
            match (lhs, rhs) {
                (Source::Slot(l), Source::Slot(r)) => const_residuals.push((spec.op, l, r)),
                _ => scan_residuals.push((spec.op, lhs, rhs)),
            }
        }
        Self::Fast {
            scan_residuals,
            const_residuals,
            row: vec![0u64; cover_vars.len().max(1)],
        }
    }
}
