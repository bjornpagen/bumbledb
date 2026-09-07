//! The single in-order pass over a middle node's pending entries.
use super::{
    BatchToken, Bindings, Colt, Counters, Executor, KeyCount, NodeScratch, PipeTables, Sink,
    ValidatedPlan, better_cover,
};

impl Executor {
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )]
    #[expect(
        clippy::too_many_arguments,
        reason = "the split borrows and execution context are clearer unpacked"
    )]
    pub(super) fn pump<S: Sink, C: Counters>(
        &mut self,
        tables: &PipeTables,
        plan: &ValidatedPlan,
        node_idx: usize,
        buffers: &mut [NodeScratch],
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) {
        let n_nodes = plan.nodes().len();
        debug_assert!(node_idx + 1 < n_nodes, "the leaf runs per parent");
        let (scratch, below) = buffers.split_first_mut().expect("one buffer per plan node");
        let carried_w = tables.carried[node_idx].len();

        let node = &plan.nodes()[node_idx];

        let below_absorb = matches!(tables.absorb, super::SkipAbsorb::Node(a) if node_idx > a);
        let mut fill = 0usize;

        let mut group: Option<(usize, usize)> = None;

        for entry in 0..scratch.pending_len {
            if !matches!(self.drive_state, super::DriveState::Running) {
                break;
            }

            if below_absorb && self.origin_cancelled(scratch.pending_origins[entry]) {
                continue;
            }
            counters.node_entry(node_idx);
            let mut best: Option<(usize, KeyCount)> = None;
            for &cover in &node.covers {
                let sub_idx = usize::from(cover);
                let occ = usize::from(node.subatoms[sub_idx].occ.0);
                let cursor = match tables.carried_index(node_idx, occ) {
                    Some(col) => scratch.pending_cursors[entry * carried_w + col],
                    None => colts[occ].start(),
                };
                let count = colts[occ].key_count(cursor);
                let better = match &best {
                    None => true,
                    Some((_, incumbent)) => better_cover(count, *incumbent),
                };
                if better {
                    best = Some((sub_idx, count));
                }
            }
            let (cover_sub, count) = best.expect("validated plans have non-empty cover sets");
            counters.cover_choice(node_idx, cover_sub, count);
            let cover_occ = usize::from(node.subatoms[cover_sub].occ.0);
            let cover_level = tables.entry_level[node_idx][cover_occ];

            let cur_arity = self.slot_map[node_idx][cover_sub].len();
            if let Some((open_sub, open_arity)) = group
                && open_sub != cover_sub
                && fill > 0
            {
                self.probe_pass(
                    tables, plan, node_idx, open_sub, open_arity, fill, scratch, below, colts,
                    bindings, sink, counters,
                );
                fill = 0;
            }
            group = Some((cover_sub, cur_arity));
            let cover_cursor = match tables.carried_index(node_idx, cover_occ) {
                Some(col) => scratch.pending_cursors[entry * carried_w + col],
                None => colts[cover_occ].start(),
            };

            let gate_cover = cur_arity == 0 && !self.point_probed[cover_occ];
            let needs_children = !scratch.children[cover_sub].is_empty();

            if S::may_use_distinct_traversal()
                && self.physical_distinct.is_some()
                && self
                    .colt_ok(colts[cover_occ].force_distinct_iteration(cover_cursor, cover_level))
                    .is_none()
            {
                break;
            }

            let entry_u32 = u32::try_from(entry).expect("pending fits u32");
            let entry_origin = scratch.pending_origins[entry];
            let mut token = BatchToken::default();
            loop {
                if !matches!(self.drive_state, super::DriveState::Running) {
                    break;
                }
                let want = if gate_cover { 1 } else { self.batch - fill };
                let batch = if needs_children {
                    colts[cover_occ].iter_batch(
                        cover_cursor,
                        cover_level,
                        token,
                        &mut scratch.entry_keys[fill * cur_arity..],
                        &mut scratch.children[cover_sub][fill..],
                        want,
                    )
                } else {
                    colts[cover_occ].iter_keys_batch(
                        cover_cursor,
                        cover_level,
                        token,
                        &mut scratch.entry_keys[fill * cur_arity..],
                        want,
                    )
                };
                let Some((yielded, next)) = self.colt_ok(batch) else {
                    break;
                };

                if yielded > 0 {
                    counters.batch(node_idx, yielded);
                }

                scratch
                    .parents
                    .extend(std::iter::repeat_n(entry_u32, yielded));
                scratch
                    .element_origins
                    .extend(std::iter::repeat_n(entry_origin, yielded));
                fill += yielded;
                token = next;
                // The bounded-quantum ledger poll on binding exploration
                // (chapter 12 §7); a refusal poisons the drive and the
                // Running checks above unwind every level.
                if !self.note_explored(yielded) {
                    break;
                }
                if fill == self.batch {
                    self.probe_pass(
                        tables, plan, node_idx, cover_sub, cur_arity, fill, scratch, below, colts,
                        bindings, sink, counters,
                    );
                    fill = 0;
                    if !gate_cover && yielded == want {
                        continue;
                    }
                }
                if gate_cover || yielded < want {
                    break;
                }
            }
        }
        if fill > 0
            && let Some((open_sub, open_arity)) = group
        {
            self.probe_pass(
                tables, plan, node_idx, open_sub, open_arity, fill, scratch, below, colts,
                bindings, sink, counters,
            );
        }
        scratch.pending_len = 0;
        scratch.pending_bindings.clear();
        scratch.pending_cursors.clear();
        scratch.pending_origins.clear();
        scratch.parents.clear();
        scratch.element_origins.clear();
    }
}
