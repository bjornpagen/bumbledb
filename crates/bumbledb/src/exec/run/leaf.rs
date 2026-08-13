//! The leaf fast-path dispatcher, the pinned-row arm, and the
//! pinned-run arm (the fold pushdown for probe-pinned leaves).

use super::{
    Bindings, Colt, Counters, Cursor, Executor, Flow, JoinPhase, LeafBatch, Sink, SkipCapability,
    Source, ValidatedPlan, grow_scratch,
};

impl Executor {
    /// The leaf fast paths. `None` = declined —
    /// multi-position forced nodes the sink cannot scan, sinks without
    /// scan support, byte-column folds — and the generic batch path runs
    /// instead (conservative by construction: correctness never depends
    /// on a fast path firing).
    ///
    /// MEASURED LAW (cleanup-0.5.0 ruling 6, the Measure phase,
    /// 2026-07-19): the leaf-elision
    /// complex — the single-subatom classification
    /// (`leaf_precompute.rs`), this dispatcher, and the pinned-row arm
    /// below — measured **1.69–1.71× generic/elided** end-to-end on a
    /// mixed pinned+scan self-join (700 answers/exec, warm DRAM,
    /// interleaved min-of-7, two process runs) against the same plan
    /// with the classification forced off. The pre-stated bar was
    /// 1.09 (the crucible ADOPT precedent); the branch is KEEP-AS-LAW
    /// (`docs/architecture/40-execution.md` § the leaf fast paths).
    /// **Reverses if:** a ledger-suite A/B on the generic batch
    /// machinery ever lands within the house bar of this path.
    pub(super) fn run_leaf_fast<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Option<Flow> {
        let node = &plan.nodes()[node_idx];
        let occ = usize::from(node.subatoms[0].occ.0);
        let (cursor, level) = self.cursors[occ];
        match cursor {
            Cursor::Row(position) => Some(self.run_leaf_pinned(
                plan, node_idx, occ, level, position, colts, bindings, sink, counters,
            )),
            Cursor::Node(_) => self.run_leaf_scan(
                plan, node_idx, occ, level, cursor, colts, bindings, sink, counters,
            ),
        }
    }

    /// The pinned-row arm: a batch of exactly one, with every batch
    /// scaffold skipped — gather, residuals, emit.
    #[expect(
        clippy::too_many_arguments,
        reason = "the split borrows and execution context are clearer unpacked"
    )] // the run_node context, unpacked
    fn run_leaf_pinned<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        occ: usize,
        level: usize,
        position: u32,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Flow {
        let node = &plan.nodes()[node_idx];
        {
            let key_slots = &self.slot_map[node_idx][0];
            let arity = key_slots.len();
            counters.node_entry(node_idx);
            counters.cover_choice(node_idx, 0, false);
            counters.batch(node_idx, 1);
            counters.phase_start(node_idx, JoinPhase::Descend);
            colts[occ].gather_row(level, position, &mut self.leaf_row[..arity.max(1)]);
            for (idx, (lhs_src, rhs_src)) in self.leaf_residual_sources.iter().enumerate() {
                let value = |src: &Source| match *src {
                    Source::Batch(word) => self.leaf_row[word],
                    Source::Slot(slot) => bindings.get(slot),
                };
                let op = self.residual_slots[node_idx][idx].0.op;
                let pass = op.compare(&value(lhs_src), &value(rhs_src));
                counters.residual(node_idx, pass);
                if !pass {
                    counters.phase_end(node_idx, JoinPhase::Descend);
                    return Flow::Continue;
                }
            }
            let batch = LeafBatch {
                keys: &self.leaf_row,
                arity,
                survivors: &[0],
                key_slots,
                bindings,
            };
            let stop_on_skip = node.suffix_skip == crate::plan::fj::SuffixSkip::Licensed
                && sink.skip_capability() == super::SkipCapability::Licensed;
            let flow = sink.emit_batch(&batch, stop_on_skip);
            counters.emit();
            counters.phase_end(node_idx, JoinPhase::Descend);
            if flow == Flow::SkipSuffix {
                counters.skip(node_idx);
                return Flow::SkipSuffix;
            }
            Flow::Continue
        }
    }

    /// The pinned-run arm — the fold pushdown for probe-pinned leaves:
    /// N probe-pass survivors of ONE parent, each pinned to a single
    /// leaf row (`Cursor::Row`), gathered and emitted as **one**
    /// [`LeafBatch`] instead of N batches-of-one. The batch-of-one
    /// shape paid 53–69 ns/tuple of per-emit scaffolding — bindings
    /// row restore, `run_node` dispatch, per-batch sink staging — for
    /// exactly one row each (the o4/j5 lanes); here that scaffolding
    /// amortizes over the run and the sink's constant-group folds see
    /// real batches (the 0.63 ns/row gather-fold floor). The
    /// `probe_pass.rs:498` gravestone litigated routing-loop copies
    /// only, never this aggregation.
    ///
    /// Contract (the `probe_pass` leaf arm — the caller):
    /// - `bindings` is the shared parent row. The per-survivor words
    ///   the recursive path would have bound per element — the
    ///   penultimate cover's keys — arrive as batch words instead:
    ///   `outer_keys` entry-major, one row of `key_slots.len() −
    ///   leaf_arity` words per position.
    /// - `key_slots` is the combined slot table: the outer words'
    ///   slots first, then the leaf cover's own (`slot_map[node][0]`)
    ///   — asserted below. The sink resolves its sources against the
    ///   combined table, so outer-varying group keys and fold inputs
    ///   read the batch, never a stale binding.
    /// - The sink must be unable to skip (aggregates; the tripwire
    ///   below): one batched emit cannot attribute a `SkipSuffix` to
    ///   one survivor's D2 origin, and no aggregate may skip anyway.
    /// - Leaf residuals are evaluated here against the combined
    ///   layout; anti-probes, membership probes, and multi-word values
    ///   never reach this arm (`leaf_precompute.rs` classifies them
    ///   off the fast paths, and the caller routes through
    ///   [`Executor::run_leaf_fast`]'s decline in that case).
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "HANDOFF: the probe_pass leaf arm (join-probe-core) and the \
                      prepared-api build wiring call this; landed callee-first \
                      with its equivalence test"
        )
    )]
    #[expect(
        clippy::too_many_arguments,
        reason = "the split borrows and execution context are clearer unpacked"
    )] // the run_node context, unpacked
    pub(super) fn run_leaf_pinned_run<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        occ: usize,
        level: usize,
        positions: &[u32],
        outer_keys: &[u64],
        key_slots: &[usize],
        colts: &[Colt],
        bindings: &Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Flow {
        let n = positions.len();
        let leaf_slots = &self.slot_map[node_idx][0];
        let leaf_arity = leaf_slots.len();
        let arity = key_slots.len();
        let outer_arity = arity - leaf_arity;
        debug_assert_eq!(outer_keys.len(), n * outer_arity, "entry-major outer rows");
        debug_assert_eq!(
            &key_slots[outer_arity..],
            &leaf_slots[..],
            "combined table = outer slots ++ the leaf cover's slots"
        );
        debug_assert!(
            sink.skip_capability() == SkipCapability::Forbidden
                || plan.nodes()[node_idx].suffix_skip != crate::plan::fj::SuffixSkip::Licensed,
            "a pinned run under a skip-licensed leaf loses origin attribution"
        );
        counters.node_entry(node_idx);
        counters.cover_choice(node_idx, 0, false);
        counters.batch(node_idx, n);
        counters.phase_start(node_idx, JoinPhase::Descend);
        let mut scratch = std::mem::take(&mut self.scratch[node_idx]);
        // Assemble combined entries: the outer words, then the pinned
        // row's key words gathered straight off the columns.
        grow_scratch(&mut scratch.entry_keys, n * arity);
        for (i, &position) in positions.iter().enumerate() {
            let row = &mut scratch.entry_keys[i * arity..(i + 1) * arity];
            row[..outer_arity].copy_from_slice(&outer_keys[i * outer_arity..][..outer_arity]);
            colts[occ].gather_row(level, position, &mut row[outer_arity..]);
        }
        scratch.survivors.clear();
        scratch
            .survivors
            .extend(0..u32::try_from(n).expect("batch fits u32"));
        // Leaf residuals against the combined layout: a `Slot` source
        // naming an outer word's slot re-aims at its batch word (that
        // slot is batch-varying here, not a binding); everything else
        // reads as the pinned arm does. Resolution is per (residual,
        // side) — never per entry.
        for (r_idx, (lhs_src, rhs_src)) in self.leaf_residual_sources.iter().enumerate() {
            let op = self.residual_slots[node_idx][r_idx].0.op;
            let resolve = |src: &Source| match *src {
                Source::Batch(word) => Source::Batch(outer_arity + word),
                Source::Slot(slot) => key_slots[..outer_arity]
                    .iter()
                    .position(|s| *s == slot)
                    .map_or(Source::Slot(slot), Source::Batch),
            };
            let (lhs, rhs) = (resolve(lhs_src), resolve(rhs_src));
            let k_max = scratch.survivors.len();
            grow_scratch(&mut scratch.mask, k_max);
            for k in 0..k_max {
                let entry = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
                let value = |src: Source| match src {
                    Source::Batch(word) => scratch.entry_keys[entry * arity + word],
                    Source::Slot(slot) => bindings.get(slot),
                };
                let pass = op.compare(&value(lhs), &value(rhs));
                counters.residual(node_idx, pass);
                scratch.mask[k] = u8::from(pass);
            }
            crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
        }
        if !scratch.survivors.is_empty() {
            let batch = LeafBatch {
                keys: &scratch.entry_keys,
                arity,
                survivors: &scratch.survivors,
                key_slots,
                bindings,
            };
            // `stop_on_skip` is structurally false here (the contract
            // tripwire above), so the sink consumes the whole batch.
            let flow = sink.emit_batch(&batch, false);
            debug_assert_eq!(flow, Flow::Continue, "non-skipping sinks never skip");
            for _ in 0..scratch.survivors.len() {
                counters.emit();
            }
        }
        counters.phase_end(node_idx, JoinPhase::Descend);
        self.scratch[node_idx] = scratch;
        Flow::Continue
    }
}
