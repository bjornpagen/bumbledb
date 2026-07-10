//! Executor construction and the per-execution entry point.

use super::{
    AntiProbeSpec, Bindings, Colt, Counters, Cursor, Executor, LeafPrecompute, NodeScratch,
    PipeTables, PlacedAllen, PlacedComparison, PlacedWordComparison, PointProbeSpec, Sink,
    ValidatedPlan, BATCH,
};

/// The membership-filter column/slot table shared by both probe kinds:
/// per filter, the interval field's (start column, end column) through
/// the occurrence's span map, plus the point variable and its slot.
fn point_parts(
    plan: &ValidatedPlan,
    occ: usize,
    filters: &[(crate::schema::FieldId, crate::ir::VarId)],
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

/// Anti-probe specs (docs/architecture/40-execution.md, § anti-probe
/// filters), aligned with each node's `anti_probes` list: the negated
/// occurrence's single trie level in binding order, each variable with
/// its first slot and slot width — precomputed like `residual_slots` —
/// plus its var-sourced membership filters, evaluated inside the probe.
fn anti_probe_slots(plan: &ValidatedPlan) -> Vec<Vec<AntiProbeSpec>> {
    plan.nodes()
        .iter()
        .map(|node| {
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
                    AntiProbeSpec {
                        occ,
                        parts,
                        key_words: usize::from(occurrence.key_widths[0]),
                        point_parts: point_parts(plan, occ, &occurrence.point_filters),
                    }
                })
                .collect()
        })
        .collect()
}

/// Membership-probe specs, aligned with each node's `point_probes`.
fn point_probe_slots(plan: &ValidatedPlan) -> Vec<Vec<PointProbeSpec>> {
    plan.nodes()
        .iter()
        .map(|node| {
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
        })
        .collect()
}

impl Executor {
    /// An executor with the default batch size ([`BATCH`]).
    #[must_use]
    pub fn new(plan: &ValidatedPlan) -> Self {
        Self::with_batch_size(plan, BATCH)
    }

    /// An executor with an explicit batch size — the scalar/vectorized
    /// equality tests parameterize this; there is no mode, only the number.
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: a zero batch size.
    #[must_use]
    #[allow(clippy::too_many_lines)] // table construction, one table at a
                                     // time — splitting scatters the shapes
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
        // Word-level slot maps: an interval variable expands to its two
        // consecutive slots, so batch key words and binding slots stay in
        // 1:1 correspondence everywhere downstream (the SlotWidth layout).
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
        let residual_slots: Vec<Vec<(PlacedComparison, usize, usize, usize)>> = plan
            .nodes()
            .iter()
            .map(|node| {
                node.residuals
                    .iter()
                    .map(|r| {
                        debug_assert_eq!(
                            width_of(r.lhs),
                            width_of(r.rhs),
                            "validated: residual sides share a structural type"
                        );
                        (
                            *r,
                            plan.slot_of(r.lhs),
                            plan.slot_of(r.rhs),
                            width_of(r.lhs),
                        )
                    })
                    .collect()
            })
            .collect();
        // Decomposed interval word residuals: slots pre-offset to the
        // compared word (docs/architecture/20-query-ir.md — the three
        // fixed compositions over slot pairs).
        let word_residual_slots: Vec<Vec<(PlacedWordComparison, usize, usize)>> = plan
            .nodes()
            .iter()
            .map(|node| {
                node.word_residuals
                    .iter()
                    .map(|r| {
                        (
                            *r,
                            plan.slot_of(r.lhs.var) + r.lhs.word.offset(),
                            plan.slot_of(r.rhs.var) + r.rhs.word.offset(),
                        )
                    })
                    .collect()
            })
            .collect();
        // Allen residuals: base slots per side (evaluation reads the
        // pair at offsets 0/1), plus the resolved-mask table — literal
        // masks final here, param masks rewritten per execution
        // (`bind_allen_masks`).
        let allen_residual_slots: Vec<Vec<(PlacedAllen, usize, usize)>> = plan
            .nodes()
            .iter()
            .map(|node| {
                node.allen_residuals
                    .iter()
                    .map(|r| (*r, plan.slot_of(r.lhs), plan.slot_of(r.rhs)))
                    .collect()
            })
            .collect();
        let allen_masks: Vec<Vec<crate::allen::AllenMask>> = allen_residual_slots
            .iter()
            .map(|slots| {
                slots
                    .iter()
                    .map(|(residual, _, _)| match residual.mask {
                        crate::ir::MaskTerm::Literal(mask) => mask,
                        // Placeholder until the first bind — every
                        // execution entry rewrites param masks first.
                        crate::ir::MaskTerm::Param(_) => crate::allen::AllenMask::EMPTY,
                    })
                    .collect()
            })
            .collect();
        let anti_probe_slots = anti_probe_slots(plan);
        let point_probe_slots = point_probe_slots(plan);
        let scratch = plan
            .nodes()
            .iter()
            .enumerate()
            .zip(&anti_probe_slots)
            .map(|((node_idx, node), anti_specs)| {
                // Word-level arity: an interval variable is two key words.
                let max_arity = slot_map[node_idx]
                    .iter()
                    .map(Vec::len)
                    .max()
                    .unwrap_or(0)
                    .max(1);
                // Probe keys also hold anti-probe keys, whose width can
                // exceed every subatom arity (an interval variable is two
                // words; the negated occurrence joins no subatom).
                let max_key = anti_specs
                    .iter()
                    .map(|spec| spec.key_words)
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
                    anti_sources: anti_specs.iter().map(|_| Vec::new()).collect(),
                    point_checks: Vec::new(),
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
        let leaf = LeafPrecompute::of(plan, &residual_slots, &var_widths);
        Self {
            batch,
            cursors: Vec::new(),
            slot_map,
            residual_slots,
            word_residual_slots,
            allen_residual_slots,
            allen_masks,
            point_probe_slots,
            var_widths,
            anti_probe_slots,
            scratch,
            leaf_single: leaf.single,
            leaf_residual_sources: leaf.residual_sources,
            leaf_scan_residuals: leaf.scan_residuals,
            leaf_const_residuals: leaf.const_residuals,
            leaf_row: leaf.row,
            scan_filter: Vec::new(),
            pipe: (plan.nodes().len() >= 2).then(|| PipeTables::of(plan)),
            cancelled: Vec::new(),
            cancel_epoch: 0,
            next_origin: 0,
            all_cancelled: false,
            origin_overflow: false,
        }
    }

    /// Resolves this execution's Allen-residual masks in place: literal
    /// masks are re-copied (idempotent), param masks read the bind slice
    /// — with the ∅/full vacuity already rejected at bind, the hot path
    /// sees only honest predicates. Called by the prepared query before
    /// every join execution; the executor itself never touches params.
    pub fn bind_allen_masks(&mut self, params: &[crate::image::view::Const]) {
        for (node_slots, node_masks) in self.allen_residual_slots.iter().zip(&mut self.allen_masks)
        {
            for ((residual, _, _), mask) in node_slots.iter().zip(node_masks.iter_mut()) {
                *mask = match residual.mask {
                    crate::ir::MaskTerm::Literal(literal) => literal,
                    crate::ir::MaskTerm::Param(param) => match &params[usize::from(param.0)] {
                        crate::image::view::Const::Word(word) => crate::allen::AllenMask::new(
                            u16::try_from(*word).expect("bind stored 13-bit mask words"),
                        )
                        .expect("bind validated the mask"),
                        _ => unreachable!("validated: a mask param resolves to a word"),
                    },
                };
            }
        }
    }

    /// A variable's slot width in words (the `SlotWidth` layout, exported
    /// through the plan witness).
    pub(super) fn width_of(&self, var: crate::ir::VarId) -> usize {
        self.var_widths
            .iter()
            .find(|(v, _)| *v == var)
            .expect("plans bind every variable")
            .1
    }

    /// Runs the plan over the COLT sources (one per occurrence, indexed by
    /// occurrence id), emitting complete bindings to the sink.
    ///
    /// # Errors
    ///
    /// `Overflow` (origins) when the D2 origin counter would cross u32 —
    /// more than 2³² absorb-node survivors in one execution, beyond the
    /// scale axiom but valid input, so it errors rather than wrapping
    /// (a wrapped counter cancels the wrong origin: silently dropped
    /// valid rows).
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations (sources not matching the
    /// plan's occurrences).
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
        self.cursors.clear();
        // Each occurrence starts below its selection levels — the root
        // when it has none, the post-`select` cursor otherwise
        // (docs/architecture/40-execution.md).
        self.cursors
            .extend(colts.iter().map(|colt| (colt.start(), 0usize)));
        // The one executor: multi-node plans
        // pipeline — probes batch ACROSS parent entries, D2 skips cancel
        // origins — and single-node plans are one leaf pass. The
        // recursive per-survivor executor is gone.
        if self.pipe.is_some() {
            self.run_pipeline(plan, colts, bindings, sink, counters);
        } else {
            self.run_node(plan, 0, colts, bindings, sink, counters);
        }
        if self.origin_overflow {
            return Err(crate::error::Error::Overflow(
                crate::error::OverflowKind::Origins,
            ));
        }
        Ok(())
    }

    /// The pipelined executor: pending binding rows
    /// and carried cursor sets flow node to node; each middle node
    /// expands pending entries into shared probe batches (flushed on
    /// cover change), probes every sibling across parents, and appends
    /// survivors to the next node's pending. The last node runs per
    /// parent through the ordinary `run_node` machinery — leaf fast
    /// paths, counters, phases and all.
    fn run_pipeline<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) {
        let tables = self.pipe.take().expect("dispatched on Some");
        let slot_count = bindings.slot_count();
        for scratch in &mut self.scratch {
            scratch.pending_bindings.clear();
            scratch.pending_cursors.clear();
            scratch.pending_origins.clear();
            scratch.pending_len = 0;
        }
        // D2 state: a fresh epoch outlives any prior execution's
        // cancellations without clearing the high-water table.
        self.cancel_epoch = self.cancel_epoch.wrapping_add(1);
        self.next_origin = 0;
        self.all_cancelled = false;
        self.origin_overflow = false;
        // The virtual root entry: no bindings, no carried cursors.
        self.scratch[0].pending_bindings.resize(slot_count, 0);
        self.scratch[0].pending_len = 1;
        self.scratch[0].pending_origins.push(0);
        self.pump(&tables, plan, 0, colts, bindings, sink, counters);
        self.pipe = Some(tables);
    }
}
