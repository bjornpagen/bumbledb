//! Executor construction and the per-execution entry point.

use super::{
    AntiProbeSpec, Bindings, Colt, Counters, Cursor, Executor, LeafPrecompute, NodeScratch,
    PipeTables, PlacedComparison, Sink, ValidatedPlan, BATCH,
};

/// Anti-probe specs (docs/architecture/40-execution.md, § anti-probe
/// filters), aligned with each node's `anti_probes` list: the negated
/// occurrence's single trie level in binding order, each variable with
/// its first slot and slot width — precomputed like `residual_slots`.
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
    pub fn with_batch_size(plan: &ValidatedPlan, batch: usize) -> Self {
        assert!(
            batch > 0,
            "a batch has at least one element (set_batch_size is the caller-facing knob)"
        );
        let slot_map: Vec<Vec<Vec<usize>>> = plan
            .nodes()
            .iter()
            .map(|node| {
                node.subatoms
                    .iter()
                    .map(|s| s.vars.iter().map(|v| plan.slot_of(*v)).collect())
                    .collect()
            })
            .collect();
        let residual_slots: Vec<Vec<(PlacedComparison, usize, usize)>> = plan
            .nodes()
            .iter()
            .map(|node| {
                node.residuals
                    .iter()
                    .map(|r| (*r, plan.slot_of(r.lhs), plan.slot_of(r.rhs)))
                    .collect()
            })
            .collect();
        let anti_probe_slots = anti_probe_slots(plan);
        let scratch = plan
            .nodes()
            .iter()
            .zip(&anti_probe_slots)
            .map(|(node, anti_specs)| {
                let max_arity = node
                    .subatoms
                    .iter()
                    .map(|s| s.vars.len())
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
                    anti_sources: anti_specs.iter().map(|_| Vec::new()).collect(),
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
        let leaf = LeafPrecompute::of(plan, &residual_slots);
        Self {
            batch,
            cursors: Vec::new(),
            slot_map,
            residual_slots,
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
        }
    }

    /// Runs the plan over the COLT sources (one per occurrence, indexed by
    /// occurrence id), emitting complete bindings to the sink.
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
    ) {
        assert_eq!(colts.len(), plan.occurrences().len());
        debug_assert_eq!(plan.nodes().len(), self.scratch.len(), "same plan shape");
        bindings.reset();
        self.cursors.clear();
        // Each occurrence starts below its selection levels — the root
        // when it has none, the post-`select` cursor otherwise
        // (docs/architecture/30-execution.md).
        self.cursors
            .extend(colts.iter().map(|colt| (colt.start(), 0usize)));
        // The one executor (docs/perf/ PRD 09/10): multi-node plans
        // pipeline — probes batch ACROSS parent entries, D2 skips cancel
        // origins — and single-node plans are one leaf pass. The
        // recursive per-survivor executor is gone.
        if self.pipe.is_some() {
            self.run_pipeline(plan, colts, bindings, sink, counters);
        } else {
            self.run_node(plan, 0, colts, bindings, sink, counters);
        }
    }

    /// The pipelined executor (docs/perf/ PRD 09): pending binding rows
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
        // D2 state (PRD 10): a fresh epoch outlives any prior execution's
        // cancellations without clearing the high-water table.
        self.cancel_epoch = self.cancel_epoch.wrapping_add(1);
        self.next_origin = 0;
        self.all_cancelled = false;
        // The virtual root entry: no bindings, no carried cursors.
        self.scratch[0].pending_bindings.resize(slot_count, 0);
        self.scratch[0].pending_len = 1;
        self.scratch[0].pending_origins.push(0);
        self.pump(&tables, plan, 0, colts, bindings, sink, counters);
        self.pipe = Some(tables);
    }
}
