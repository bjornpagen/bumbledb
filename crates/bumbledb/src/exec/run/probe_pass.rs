//! One cross-parent probe pass (docs/perf/ PRD 09).

use super::anti_probe::anti_probe_pass;
use super::{
    Bindings, Colt, Counters, Cursor, Executor, Flow, JoinPhase, NodeScratch, PipeTables, Sink,
    Source, ValidatedPlan, PREFETCH_WIDTH_FLOOR,
};

impl Executor {
    /// One cross-parent probe pass (docs/perf/ PRD 09): hashes, prefetch,
    /// probes, and residuals run over `fill` elements drawn from many
    /// pending entries; survivors either append to the child's pending
    /// (middle child — flushed when a full batch accumulates) or run the
    /// last node per parent through `run_node`.
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn probe_pass<S: Sink, C: Counters>(
        &mut self,
        tables: &PipeTables,
        plan: &ValidatedPlan,
        node_idx: usize,
        cover_sub: usize,
        arity: usize,
        fill: usize,
        scratch: &mut NodeScratch,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) {
        let n_nodes = plan.nodes().len();
        let slot_count = bindings.slot_count();
        let carried_w = tables.carried[node_idx].len();
        let node = &plan.nodes()[node_idx];
        let cover_occ = usize::from(node.subatoms[cover_sub].occ.0);
        scratch.survivors.clear();
        scratch
            .survivors
            .extend(0..u32::try_from(fill).expect("batch fits u32"));

        // Sibling passes: per-parent Slot reads and per-parent cursors.
        // Instruction diet (docs/silicon/02): value sources resolve once
        // per (pass, subatom) — never a per-element variable search —
        // loop invariants (carried column, start cursor) hoist, and the
        // inner loops write pre-sized buffers by index (a `Vec::push`'s
        // grow branch blocks LICM and unrolling in exactly these loops).
        for sub_idx in 0..node.subatoms.len() {
            if sub_idx == cover_sub || scratch.survivors.is_empty() {
                continue;
            }
            let subatom = &node.subatoms[sub_idx];
            let sub_arity = subatom.vars.len();
            let occ = usize::from(subatom.occ.0);
            let s_level = tables.entry_level[node_idx][occ];
            let cover_vars = &node.subatoms[cover_sub].vars;
            counters.phase_start(node_idx, JoinPhase::Hash);
            scratch.sources[sub_idx].clear();
            for (i, var) in subatom.vars.iter().enumerate() {
                let source = cover_vars.iter().position(|cv| cv == var).map_or(
                    Source::Slot(self.slot_map[node_idx][sub_idx][i]),
                    Source::Batch,
                );
                scratch.sources[sub_idx].push(source);
            }
            let n = scratch.survivors.len();
            scratch.hashes.clear();
            scratch.hashes.resize(n, 0);
            // One gather loop for every source shape (docs/silicon2/10:
            // the single-batch-word specialized twin measured < 2% at
            // family level post-bucket-layout and was deleted).
            {
                let survivors = &scratch.survivors[..n];
                let entry_keys = &scratch.entry_keys[..];
                let parents = &scratch.parents[..];
                let pending_bindings = &scratch.pending_bindings[..];
                let sources = &scratch.sources[sub_idx][..];
                let probe_keys = &mut scratch.probe_keys[..n * sub_arity];
                let hashes = &mut scratch.hashes[..n];
                for (k, &e) in survivors.iter().enumerate() {
                    let element = usize::try_from(e).expect("batch fits usize");
                    let parent = parents[element] as usize;
                    for i in 0..sub_arity {
                        probe_keys[k * sub_arity + i] = match sources[i] {
                            Source::Batch(word) => entry_keys[element * arity + word],
                            Source::Slot(slot) => pending_bindings[parent * slot_count + slot],
                        };
                    }
                    counters.probe_hash(node_idx, sub_idx);
                    hashes[k] = crate::exec::colt::hash_key(
                        &probe_keys[k * sub_arity..(k + 1) * sub_arity],
                    );
                }
            }
            counters.phase_end(node_idx, JoinPhase::Hash);
            let carried = tables.carried_col[node_idx][occ];
            let start_cursor = colts[occ].start();
            // Phase 1.5, width-floor gated — see run_node.
            if scratch.survivors.len() >= PREFETCH_WIDTH_FLOOR {
                crate::obs::event(
                    crate::obs::names::PREFETCH_PASS,
                    crate::obs::Category::Execute,
                    scratch.survivors.len() as u64,
                    colts[occ].probe_footprint_bytes() as u64,
                );
                for (k, &e) in scratch.survivors.iter().enumerate() {
                    let parent = scratch.parents[e as usize] as usize;
                    let cursor = carried.map_or(start_cursor, |col| {
                        scratch.pending_cursors[parent * carried_w + col]
                    });
                    colts[occ].prefetch_bucket(cursor, scratch.hashes[k]);
                }
            }
            counters.phase_start(node_idx, JoinPhase::Probe);
            scratch.mask.clear();
            scratch.mask.resize(n, 0);
            // The exp-19 shape itself (docs/silicon2/07): reads
            // survivors/parents/pending_cursors/probe_keys/hashes,
            // writes sibling_children/mask — all hoisted to disjoint
            // locals so the stores cannot alias the read headers.
            {
                let survivors = &scratch.survivors[..n];
                let parents = &scratch.parents[..];
                let pending_cursors = &scratch.pending_cursors[..];
                let probe_keys = &scratch.probe_keys[..n * sub_arity];
                let hashes = &scratch.hashes[..n];
                let sibling_children = &mut scratch.sibling_children[sub_idx][..];
                let mask = &mut scratch.mask[..n];
                let colt = &mut colts[occ];
                for k in 0..n {
                    let element = usize::try_from(survivors[k]).expect("batch fits usize");
                    let parent = parents[element] as usize;
                    let cursor = carried.map_or(start_cursor, |col| {
                        pending_cursors[parent * carried_w + col]
                    });
                    let hit = colt.get_prehashed(
                        cursor,
                        s_level,
                        &probe_keys[k * sub_arity..(k + 1) * sub_arity],
                        hashes[k],
                    );
                    counters.probe(node_idx, sub_idx, hit.is_some());
                    sibling_children[element] = hit.unwrap_or(Cursor::Row(0));
                    mask[k] = u8::from(hit.is_some());
                }
            }
            crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            counters.phase_end(node_idx, JoinPhase::Probe);
            scratch.hashes.clear();
        }

        // Residuals: per-parent Slot reads.
        counters.phase_start(node_idx, JoinPhase::Residual);
        for (residual, lhs_slot, rhs_slot) in &self.residual_slots[node_idx] {
            let cover_vars = &node.subatoms[cover_sub].vars;
            let lhs_word = cover_vars.iter().position(|cv| *cv == residual.lhs);
            let rhs_word = cover_vars.iter().position(|cv| *cv == residual.rhs);
            let n = scratch.survivors.len();
            scratch.mask.clear();
            scratch.mask.resize(n, 0);
            for k in 0..n {
                let element = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
                let parent = scratch.parents[element] as usize;
                let value = |word: Option<usize>, slot: usize| match word {
                    Some(word) => scratch.entry_keys[element * arity + word],
                    None => scratch.pending_bindings[parent * slot_count + slot],
                };
                let pass = residual
                    .op
                    .compare(&value(lhs_word, *lhs_slot), &value(rhs_word, *rhs_slot));
                counters.residual(node_idx, pass);
                scratch.mask[k] = u8::from(pass);
            }
            crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
        }
        counters.phase_end(node_idx, JoinPhase::Residual);

        // Anti-probes: the residual step's sibling (docs/architecture/
        // 40-execution.md, § anti-probe filters) — hits are compacted
        // away on the same cursor-write. Slot reads go through each
        // element's parent row, exactly like the residuals above.
        anti_probe_pass(
            &self.anti_probe_slots[node_idx],
            node_idx,
            &node.subatoms[cover_sub].vars,
            arity,
            colts,
            &scratch.entry_keys,
            &mut scratch.survivors,
            &mut scratch.probe_keys,
            &mut scratch.hashes,
            &mut scratch.mask,
            &mut scratch.anti_sources,
            |element, slot| {
                let parent = scratch.parents[element] as usize;
                scratch.pending_bindings[parent * slot_count + slot]
            },
            counters,
        );

        // Survivor routing. Origins (PRD 10): the absorb node mints one
        // fresh origin per routed survivor — the cancellation unit is
        // exactly "one absorb-element's subtree"; deeper nodes inherit.
        counters.phase_start(node_idx, JoinPhase::Descend);
        let leaf = node_idx + 2 == n_nodes;
        let child_carried = &tables.carried[node_idx + 1];
        let mints_origins = tables.absorb == Some(node_idx);
        for k in 0..scratch.survivors.len() {
            if self.all_cancelled {
                break;
            }
            let element = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
            let parent = scratch.parents[element] as usize;
            let origin = if mints_origins {
                let minted = self.next_origin;
                self.next_origin += 1;
                minted
            } else {
                scratch.element_origins[element]
            };
            // Real origins exist strictly below the absorb node; the
            // seed id above it must never match a minted id.
            if tables.absorb.is_some_and(|a| node_idx > a) && self.origin_cancelled(origin) {
                continue;
            }
            let assemble = |occ: usize| -> Cursor {
                // Advanced at this node: the cover's child or a probed
                // sibling's; otherwise inherited from the parent (or the
                // colt's start when never advanced).
                if occ == cover_occ {
                    return scratch.children[element];
                }
                if let Some(sub_idx) = node
                    .subatoms
                    .iter()
                    .position(|sub| usize::from(sub.occ.0) == occ)
                {
                    debug_assert_ne!(sub_idx, cover_sub, "distinct occs per node");
                    return scratch.sibling_children[sub_idx][element];
                }
                match tables.carried_col[node_idx][occ] {
                    Some(col) => scratch.pending_cursors[parent * carried_w + col],
                    None => colts[occ].start(),
                }
            };
            if leaf {
                // The last node runs per parent through the ordinary
                // machinery: bindings row + cursors restored, then
                // run_node — leaf fast paths, counters, phases and all.
                bindings.load_row(
                    &scratch.pending_bindings[parent * slot_count..(parent + 1) * slot_count],
                );
                for (i, slot) in self.slot_map[node_idx][cover_sub].iter().enumerate() {
                    bindings.set(*slot, scratch.entry_keys[element * arity + i]);
                }
                let leaf_node = &plan.nodes()[node_idx + 1];
                for subatom in &leaf_node.subatoms {
                    let occ = usize::from(subatom.occ.0);
                    self.cursors[occ] = (assemble(occ), tables.entry_level[node_idx + 1][occ]);
                }
                let flow = self.run_node(plan, node_idx + 1, colts, bindings, sink, counters);
                if flow == Flow::SkipSuffix {
                    // The leaf skipped (D2): everything descended from
                    // this survivor's origin can only duplicate rows.
                    // The origin is real exactly when this node is at or
                    // below the absorb (minted here or inherited).
                    counters.skip(node_idx);
                    match tables.absorb {
                        Some(a) if node_idx >= a => self.cancel_origin(origin),
                        Some(_) => {}
                        None => self.all_cancelled = true,
                    }
                }
            } else {
                let child = &mut self.scratch[node_idx + 1];
                let start = child.pending_bindings.len();
                child.pending_bindings.extend_from_slice(
                    &scratch.pending_bindings[parent * slot_count..(parent + 1) * slot_count],
                );
                for (i, slot) in self.slot_map[node_idx][cover_sub].iter().enumerate() {
                    child.pending_bindings[start + slot] = scratch.entry_keys[element * arity + i];
                }
                for &occ in child_carried {
                    let cursor = assemble(occ);
                    self.scratch[node_idx + 1].pending_cursors.push(cursor);
                }
                self.scratch[node_idx + 1].pending_origins.push(origin);
                self.scratch[node_idx + 1].pending_len += 1;
            }
        }
        counters.phase_end(node_idx, JoinPhase::Descend);
        scratch.parents.clear();
        scratch.element_origins.clear();
        // Cascade at one accumulated batch. Bounded memory: the child
        // holds at most two batches transiently (the 1×batch trigger
        // plus one pass's appends before the next check). The 2×-batch
        // threshold (docs/silicon/14) measured 0.0–0.6% once exp 14
        // priced the per-pass overhead at 11–30 ns — reverted to the
        // simpler contract (docs/silicon2/08).
        if !leaf && self.scratch[node_idx + 1].pending_len >= self.batch {
            self.pump(tables, plan, node_idx + 1, colts, bindings, sink, counters);
        }
    }
}
