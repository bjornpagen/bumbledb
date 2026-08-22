//! The leaf fast-path dispatcher, the pinned-row arm, and the
//! pinned-run arm (the fold pushdown for probe-pinned leaves).

use super::{
    Bindings, Colt, Counters, Cursor, Executor, Flow, JoinPhase, LeafBatch, Sink, SkipCapability,
    Source, ValidatedPlan, grow_scratch,
};

impl Executor {
    /// 2026-07-19): the leaf-elision

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

    #[expect(
        clippy::too_many_arguments,
        reason = "the split borrows and execution context are clearer unpacked"
    )]
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
        let super::LeafPrecompute::Fast {
            scan_residuals,
            const_residuals,
            row,
        } = &mut self.leaf
        else {
            unreachable!("fast path is classified Fast");
        };
        {
            let key_slots = &self.slot_map[node_idx][0];
            let arity = key_slots.len();
            counters.node_entry(node_idx);
            counters.cover_choice(node_idx, 0, crate::exec::colt::KeyCount::Estimate(0));
            counters.batch(node_idx, 1);
            counters.phase_start(node_idx, JoinPhase::Descend);
            colts[occ].gather_row(level, position, &mut row[..arity.max(1)]);
            let mut failed = false;
            for (op, lhs, rhs) in const_residuals.iter() {
                let pass = op.compare(&bindings.get(*lhs), &bindings.get(*rhs));
                counters.residual(node_idx, pass);
                if !pass {
                    failed = true;
                    break;
                }
            }
            if !failed {
                for (op, lhs_src, rhs_src) in scan_residuals.iter() {
                    let value = |src: &Source| match *src {
                        Source::Batch(word) => row[word],
                        Source::Slot(slot) => bindings.get(slot),
                    };
                    let pass = op.compare(&value(lhs_src), &value(rhs_src));
                    counters.residual(node_idx, pass);
                    if !pass {
                        failed = true;
                        break;
                    }
                }
            }
            if failed {
                counters.phase_end(node_idx, JoinPhase::Descend);
                return Flow::Continue;
            }
            let batch = LeafBatch {
                keys: row,
                arity,
                survivors: &[0],
                key_slots,
                bindings,
            };
            let flow = super::emit_node_batch(sink, node.suffix_skip, &batch);
            counters.emit();
            counters.phase_end(node_idx, JoinPhase::Descend);
            if flow == Flow::SkipSuffix {
                counters.skip(node_idx);
                return Flow::SkipSuffix;
            }
            Flow::Continue
        }
    }

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
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )]
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
        counters.cover_choice(node_idx, 0, crate::exec::colt::KeyCount::Estimate(0));
        counters.batch(node_idx, n);
        counters.phase_start(node_idx, JoinPhase::Descend);
        let mut scratch = std::mem::take(&mut self.scratch[node_idx]);

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

        let residual_lists = match &self.leaf {
            super::LeafPrecompute::Fast {
                scan_residuals,
                const_residuals,
                ..
            } => (scan_residuals.as_slice(), const_residuals.as_slice()),
            super::LeafPrecompute::Generic => {
                unreachable!("fast path is classified Fast")
            }
        };
        let (scan_residuals, const_residuals) = residual_lists;
        let resolve_slot = |slot: usize| {
            key_slots[..outer_arity]
                .iter()
                .position(|s| *s == slot)
                .map_or(Source::Slot(slot), Source::Batch)
        };
        let resolve = |src: Source| match src {
            Source::Batch(word) => Source::Batch(outer_arity + word),
            Source::Slot(slot) => resolve_slot(slot),
        };
        for (op, lhs, rhs) in const_residuals {
            let (lhs, rhs) = (resolve_slot(*lhs), resolve_slot(*rhs));
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
        for (op, lhs_src, rhs_src) in scan_residuals {
            let (lhs, rhs) = (resolve(*lhs_src), resolve(*rhs_src));
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

            let flow = sink.emit_batch(&batch);
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
