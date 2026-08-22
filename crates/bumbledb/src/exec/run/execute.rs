//! Executor construction and the per-execution entry point.

use super::{
    AllenResidualSpec, AntiProbeSpec, BATCH, Bindings, Colt, Counters, Cursor, Drive, Executor,
    LeafPrecompute, NodePrecompute, NodeScratch, PipeTables, PointProbeSpec, ResidualSpec, Sink,
    ValidatedPlan, WordResidualSpec,
};
use crate::plan::fj::PlanNode;
use std::num::NonZeroUsize;

fn point_parts(
    plan: &ValidatedPlan,
    occ: usize,
    filters: &[(bumbledb_theory::schema::FieldId, crate::ir::VarId)],
) -> Vec<(usize, usize, crate::ir::VarId, usize)> {
    let occurrence = &plan.occurrences()[occ];
    filters
        .iter()
        .map(|(field, var)| {
            let span = occurrence.spans[usize::from(field.0)];
            let first = usize::from(span.first_column);
            (first, first + 1, *var, plan.slot_of(*var))
        })
        .collect()
}

fn anti_probes_of(plan: &ValidatedPlan, node: &PlanNode) -> Vec<AntiProbeSpec> {
    node.anti_probes
        .iter()
        .map(|anti_probe| {
            let occ = usize::from(anti_probe.occurrence.0);
            let occurrence = &plan.occurrences()[occ];
            debug_assert_eq!(
                occurrence.trie_schema.len(),
                1,
                "a negated occurrence's trie schema is one probe level"
            );
            let parts: Vec<(crate::ir::VarId, usize, usize)> = occurrence.trie_schema[0]
                .iter()
                .map(|var| {
                    let (_, width) = plan
                        .slots()
                        .iter()
                        .find(|(slot_var, _)| slot_var == var)
                        .expect("anti-probe variables are slot-bound");
                    (*var, plan.slot_of(*var), width.slots())
                })
                .collect();
            let key_words = usize::from(occurrence.key_widths[0]);
            let form = match NonZeroUsize::new(key_words) {
                None => super::AntiProbeForm::Gate,
                Some(key_words) => super::AntiProbeForm::Keyed { parts, key_words },
            };
            AntiProbeSpec {
                occ,
                form,
                point_parts: point_parts(plan, occ, &occurrence.point_filters),
            }
        })
        .collect()
}

fn point_probes_of(plan: &ValidatedPlan, node: &PlanNode) -> Vec<PointProbeSpec> {
    node.point_probes
        .iter()
        .map(|probe| {
            let occ = usize::from(probe.occ.0);
            PointProbeSpec {
                occ,
                parts: point_parts(plan, occ, &probe.filters),
            }
        })
        .collect()
}

impl NodePrecompute {
    fn of(
        plan: &ValidatedPlan,
        node: &PlanNode,
        width_of: impl Fn(crate::ir::VarId) -> usize,
    ) -> Self {
        let residual_slots = node
            .residuals
            .iter()
            .map(|r| {
                let (left, right, op) = r.compare_sides();
                debug_assert_eq!(
                    width_of(left.var()),
                    width_of(right.var()),
                    "validated: residual sides share a structural type"
                );
                ResidualSpec {
                    op,
                    lhs: left.var(),
                    rhs: right.var(),
                    lhs_slot: plan.slot_of(left.var()),
                    rhs_slot: plan.slot_of(right.var()),
                    width: width_of(left.var()),
                }
            })
            .collect();
        let word_residual_slots = node
            .word_residuals
            .iter()
            .map(|r| {
                let (left, right, op) = r.compare_sides();
                WordResidualSpec {
                    op,
                    left,
                    right,
                    lhs_slot: plan.slot_of(left.var()) + left.offset(),
                    rhs_slot: plan.slot_of(right.var()) + right.offset(),
                }
            })
            .collect();
        let allen_residual_slots: Vec<AllenResidualSpec> = node
            .allen_residuals
            .iter()
            .map(|r| {
                let (left, right, mask) = r.allen_sides();
                AllenResidualSpec {
                    lhs: left.var(),
                    rhs: right.var(),
                    lhs_slot: plan.slot_of(left.var()),
                    rhs_slot: plan.slot_of(right.var()),
                    mask,
                }
            })
            .collect();
        let allen_masks = allen_residual_slots.iter().map(|spec| spec.mask).collect();
        Self {
            residual_slots,
            word_residual_slots,
            allen_residual_slots,
            allen_masks,
            point_probes: point_probes_of(plan, node),
            anti_probes: anti_probes_of(plan, node),
        }
    }
}

impl Executor {
    #[must_use]
    pub fn new(plan: &ValidatedPlan) -> Self {
        Self::with_batch_size(plan, BATCH)
    }

    /// # Panics

    /// Only on a programmer-invariant violation: a zero batch size.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )]

    pub fn with_batch_size(plan: &ValidatedPlan, batch: usize) -> Self {
        assert!(
            batch > 0,
            "a batch has at least one element (set_batch_size is the caller-facing knob)"
        );

        let var_widths: Vec<(crate::ir::VarId, usize)> = plan
            .slots()
            .iter()
            .map(|(var, width)| (*var, width.slots()))
            .collect();
        let width_of = |var: crate::ir::VarId| -> usize {
            var_widths
                .iter()
                .find(|(v, _)| *v == var)
                .expect("plans bind every variable")
                .1
        };

        let slot_map: Vec<Vec<Vec<usize>>> = plan
            .nodes()
            .iter()
            .map(|node| {
                node.subatoms
                    .iter()
                    .map(|s| {
                        let mut words = Vec::new();
                        for var in &s.vars {
                            let slot = plan.slot_of(*var);
                            for offset in 0..width_of(*var) {
                                words.push(slot + offset);
                            }
                        }
                        words
                    })
                    .collect()
            })
            .collect();
        let precompute: Vec<NodePrecompute> = plan
            .nodes()
            .iter()
            .map(|node| NodePrecompute::of(plan, node, width_of))
            .collect();

        let mut point_probed = vec![false; plan.occurrences().len()];
        for node in plan.nodes() {
            for probe in &node.point_probes {
                point_probed[usize::from(probe.occ.0)] = true;
            }
        }
        let scratch = plan
            .nodes()
            .iter()
            .enumerate()
            .zip(&precompute)
            .map(|((node_idx, node), pre)| {
                let max_arity = slot_map[node_idx]
                    .iter()
                    .map(Vec::len)
                    .max()
                    .unwrap_or(0)
                    .max(1);

                let max_key = pre
                    .anti_probes
                    .iter()
                    .map(super::AntiProbeSpec::key_words)
                    .max()
                    .unwrap_or(0)
                    .max(max_arity);
                NodeScratch {
                    entry_keys: vec![0; batch * max_arity],
                    children: vec![Cursor::Row(0); batch],
                    survivors: Vec::with_capacity(batch),
                    probe_keys: vec![0; batch * max_key],
                    hashes: Vec::with_capacity(batch),
                    sibling_children: node
                        .subatoms
                        .iter()
                        .map(|_| vec![Cursor::Row(0); batch])
                        .collect(),

                    sources: node.subatoms.iter().map(|_| Vec::new()).collect(),
                    residual_sources: Vec::new(),
                    word_residual_sources: Vec::new(),
                    allen_sources: Vec::new(),
                    allen_gather: Vec::new(),
                    allen_codes: Vec::new(),
                    anti_sources: pre.anti_probes.iter().map(|_| Vec::new()).collect(),
                    point_checks: Vec::new(),
                    point_sources: Vec::new(),
                    point_rows: Vec::new(),
                    point_row_ks: Vec::new(),
                    cursor_srcs: Vec::new(),
                    mask: Vec::with_capacity(batch),
                    parents: Vec::with_capacity(batch),
                    pending_bindings: Vec::new(),
                    pending_cursors: Vec::new(),
                    pending_len: 0,
                    pending_origins: Vec::new(),
                    element_origins: Vec::with_capacity(batch),
                }
            })
            .collect();
        let leaf = LeafPrecompute::of(plan, &precompute, &var_widths);
        Self {
            batch,
            cursors: Vec::new(),
            slot_map,
            precompute,
            point_probed,
            var_widths,
            scratch,
            leaf,
            scan_filter: Vec::new(),
            drive: if plan.nodes().len() >= 2 {
                Drive::Pipeline(std::rc::Rc::new(PipeTables::of(plan)))
            } else {
                Drive::Leaf
            },
            cancelled: Vec::new(),
            cancel_epoch: 0,
            next_origin: 0,
            drive_state: super::DriveState::Running,
            overlap: crate::interval::overlap::OverlapCache::default(),
            overlap_hits: Vec::new(),
            overlap_key: Vec::new(),
        }
    }

    pub fn bind_allen_masks(&mut self, _params: &[crate::image::view::Const]) {
        for node in &mut self.precompute {
            for (spec, mask) in node
                .allen_residual_slots
                .iter()
                .zip(node.allen_masks.iter_mut())
            {
                *mask = spec.mask;
            }
        }
    }

    pub(super) fn width_of(&self, var: crate::ir::VarId) -> usize {
        self.var_widths
            .iter()
            .find(|(v, _)| *v == var)
            .expect("plans bind every variable")
            .1
    }

    /// # Errors

    /// # Panics

    /// Only on programmer-invariant violations (sources not matching the
    pub fn execute<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> crate::error::Result<()> {
        assert_eq!(colts.len(), plan.occurrences().len());
        debug_assert_eq!(plan.nodes().len(), self.scratch.len(), "same plan shape");
        bindings.reset();
        self.drive_state = super::DriveState::Running;

        self.overlap.reset();
        self.cursors.clear();

        self.cursors
            .extend(colts.iter().map(|colt| (colt.start(), 0usize)));

        match &self.drive {
            Drive::Pipeline(_) => {
                self.run_pipeline(plan, colts, bindings, sink, counters);
            }
            Drive::Leaf => {
                self.run_node(plan, 0, colts, bindings, sink, counters);
            }
        }

        match std::mem::replace(&mut self.drive_state, super::DriveState::Running) {
            super::DriveState::Poisoned(super::Poison::OriginOverflow) => Err(
                crate::error::Error::Overflow(crate::error::OverflowKind::OriginCapacity),
            ),
            super::DriveState::Running | super::DriveState::SkipDone => Ok(()),
        }
    }

    fn run_pipeline<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) {
        let tables = match &self.drive {
            Drive::Pipeline(tables) => std::rc::Rc::clone(tables),
            Drive::Leaf => unreachable!("dispatched on Pipeline"),
        };
        let slot_count = bindings.slot_count();
        for scratch in &mut self.scratch {
            scratch.pending_bindings.clear();
            scratch.pending_cursors.clear();
            scratch.pending_origins.clear();
            scratch.pending_len = 0;
        }

        // origin and silently drop answers).
        self.advance_cancel_epoch();
        self.next_origin = 0;
        self.drive_state = super::DriveState::Running;

        self.scratch[0].pending_bindings.resize(slot_count, 0);
        self.scratch[0].pending_len = 1;
        self.scratch[0].pending_origins.push(0);
        self.pump(&tables, plan, 0, colts, bindings, sink, counters);

        for i in 1..plan.nodes().len() - 1 {
            if self.scratch[i].pending_len > 0 {
                self.pump(&tables, plan, i, colts, bindings, sink, counters);
            }
        }
    }
}
