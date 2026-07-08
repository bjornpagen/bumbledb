//! The single-subatom-leaf precompute (docs/perf/ PRD 05).

use super::{LeafPrecompute, PlacedComparison, Source, ValidatedPlan};

impl LeafPrecompute {
    pub(super) fn of(plan: &ValidatedPlan, residual_slots: &[Vec<(PlacedComparison, usize, usize)>]) -> Self {
        let last = plan.nodes().len() - 1;
        let single = plan.nodes()[last].subatoms.len() == 1;
        if !single {
            return Self {
                single,
                residual_sources: Vec::new(),
                scan_residuals: Vec::new(),
                const_residuals: Vec::new(),
                row: Vec::new(),
            };
        }
        let cover_vars = &plan.nodes()[last].subatoms[0].vars;
        let residual_sources: Vec<(Source, Source)> = residual_slots[last]
            .iter()
            .map(|(residual, lhs_slot, rhs_slot)| {
                let resolve = |var: crate::ir::VarId, slot: usize| {
                    cover_vars
                        .iter()
                        .position(|cv| *cv == var)
                        .map_or(Source::Slot(slot), Source::Batch)
                };
                (
                    resolve(residual.lhs, *lhs_slot),
                    resolve(residual.rhs, *rhs_slot),
                )
            })
            .collect();
        let mut scan_residuals = Vec::new();
        let mut const_residuals = Vec::new();
        for (idx, (lhs, rhs)) in residual_sources.iter().enumerate() {
            let op = residual_slots[last][idx].0.op;
            match (lhs, rhs) {
                (Source::Slot(l), Source::Slot(r)) => const_residuals.push((op, *l, *r)),
                _ => scan_residuals.push((op, *lhs, *rhs)),
            }
        }
        Self {
            single,
            residual_sources,
            scan_residuals,
            const_residuals,
            row: vec![0u64; cover_vars.len().max(1)],
        }
    }
}
