//! The single in-order pass over a middle node's pending entries.

use super::{
    better_cover, BatchToken, Bindings, Colt, Counters, Executor, KeyCount, PipeTables, Sink,
    ValidatedPlan,
};

impl Executor {
    /// Consumes every pending entry at a middle node, cascading full
    /// child batches immediately and draining the remainder at the end.
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )] // one batch loop; the invariants read in order
    #[expect(
        clippy::too_many_arguments,
        reason = "the split borrows and execution context are clearer unpacked"
    )]
    pub(super) fn pump<S: Sink, C: Counters>(
        &mut self,
        tables: &PipeTables,
        plan: &ValidatedPlan,
        node_idx: usize,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) {
        let n_nodes = plan.nodes().len();
        debug_assert!(node_idx + 1 < n_nodes, "the leaf runs per parent");
        let mut scratch = std::mem::take(&mut self.scratch[node_idx]);
        let carried_w = tables.carried[node_idx].len();

        // One in-order pass: per-entry dynamic cover
        // choice at processing time, probe_pass flushed on cover change.
        // Cover-stable segregation precomputed covers
        // and grouped entries to lift probe-batch means 37 → 39 — then
        // the per-pass overhead it amortizes was priced at 11–30 ns,
        // TWENTY TIMES below the assumption behind it, making the whole
        // batch-mean lever class a ~1% effect; the two-pass machinery is
        // deleted. Cross-call fill carry is rejected by the same number
        // before ever being built: lifting batch means to ~128 is worth
        // 0.2–1.2% of triangle p50 at the measured pass overhead
        // — the lever class is closed. The cover
        // choice itself is a performance heuristic — any cover is
        // correct — so choosing from live colt state (a force during an
        // earlier flush could have flipped an Estimate to Exact) changes
        // nothing semantic.
        let node = &plan.nodes()[node_idx];
        let mut fill = 0usize;
        // The open cover run: (cover_sub, arity, occ, level).
        let mut group: Option<(usize, usize, usize, usize)> = None;
        for entry in 0..scratch.pending_len {
            if self.all_cancelled {
                break;
            }
            // D2: a cancelled origin's pending work is dead —
            // its outputs could only duplicate rows already seen. Origin
            // ids are meaningful strictly BELOW the absorb node (minted
            // at its routing); above it entries carry the meaningless
            // seed and must never be filtered. Cancellation fired during
            // an earlier entry's flush is seen here: the check runs at
            // each entry's turn.
            if tables.absorb.is_some_and(|a| node_idx > a)
                && self.origin_cancelled(scratch.pending_origins[entry])
            {
                continue;
            }
            counters.node_entry(node_idx);
            let mut best: Option<(usize, KeyCount)> = None;
            for &cover in &node.covers {
                let sub_idx = usize::from(cover);
                let occ = usize::from(node.subatoms[sub_idx].occ.0);
                let cursor = match tables.carried_col[node_idx][occ] {
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
            counters.cover_choice(node_idx, cover_sub, matches!(count, KeyCount::Exact(_)));
            let cover_occ = usize::from(node.subatoms[cover_sub].occ.0);
            let cover_level = tables.entry_level[node_idx][cover_occ];
            // Word-level batch arity (the SlotWidth layout).
            let cur_arity = self.slot_map[node_idx][cover_sub].len();
            if let Some((open_sub, open_arity, _, _)) = group {
                if open_sub != cover_sub && fill > 0 {
                    self.probe_pass(
                        tables,
                        plan,
                        node_idx,
                        open_sub,
                        open_arity,
                        fill,
                        &mut scratch,
                        colts,
                        bindings,
                        sink,
                        counters,
                    );
                    fill = 0;
                }
            }
            group = Some((cover_sub, cur_arity, cover_occ, cover_level));
            let cover_cursor = match tables.carried_col[node_idx][cover_occ] {
                Some(col) => scratch.pending_cursors[entry * carried_w + col],
                None => colts[cover_occ].start(),
            };
            let mut token = BatchToken::default();
            loop {
                let want = self.batch - fill;
                let (yielded, next) = colts[cover_occ].iter_batch(
                    cover_cursor,
                    cover_level,
                    token,
                    &mut scratch.entry_keys[fill * cur_arity..],
                    &mut scratch.children[fill..],
                    want,
                );
                counters.batch(node_idx, yielded);
                for _ in 0..yielded {
                    scratch
                        .parents
                        .push(u32::try_from(entry).expect("pending fits u32"));
                    scratch.element_origins.push(scratch.pending_origins[entry]);
                }
                fill += yielded;
                token = next;
                if fill == self.batch {
                    self.probe_pass(
                        tables,
                        plan,
                        node_idx,
                        cover_sub,
                        cur_arity,
                        fill,
                        &mut scratch,
                        colts,
                        bindings,
                        sink,
                        counters,
                    );
                    fill = 0;
                    if yielded == want {
                        continue; // the entry may have more; resume its token
                    }
                }
                if yielded < want {
                    break; // entry exhausted
                }
            }
        }
        if fill > 0 {
            if let Some((open_sub, open_arity, _, _)) = group {
                self.probe_pass(
                    tables,
                    plan,
                    node_idx,
                    open_sub,
                    open_arity,
                    fill,
                    &mut scratch,
                    colts,
                    bindings,
                    sink,
                    counters,
                );
            }
        }
        scratch.pending_len = 0;
        scratch.pending_bindings.clear();
        scratch.pending_cursors.clear();
        scratch.pending_origins.clear();
        scratch.parents.clear();
        scratch.element_origins.clear();
        self.scratch[node_idx] = scratch;
        // Drain the child's sub-batch remainder (full batches already flushed downstream
        // inside probe_pass's flush check — see its tail).
        if node_idx + 2 < n_nodes && self.scratch[node_idx + 1].pending_len > 0 {
            self.pump(tables, plan, node_idx + 1, colts, bindings, sink, counters);
        }
    }
}
