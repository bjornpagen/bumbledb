//! The leaf pass over one node's cover batch (single-node and last-node).

use super::anti_probe::anti_probe_pass;
use super::{
    better_cover, BatchToken, Bindings, Colt, Counters, Cursor, Executor, Flow, JoinPhase,
    KeyCount, LeafBatch, Sink, Source, ValidatedPlan, PREFETCH_WIDTH_FLOOR,
};

impl Executor {
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )] // the one hot loop; splitting it would
       // scatter the batch invariants the comments walk through in order
    pub(super) fn run_node<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Flow {
        // The one caller class: the LAST node — the
        // pipeline pumps every middle node, single-node plans call this
        // directly. Zero-node plans are unrepresentable (validation
        // rule 14 rejects atom-less queries).
        assert!(
            node_idx + 1 == plan.nodes().len(),
            "run_node is the leaf pass; middle nodes pump"
        );
        // The leaf fast paths: pinned-row elision and
        // the scan-fold pushdown. A `None` decline falls through to the
        // generic batch machinery with no counters fired.
        if self.leaf_single {
            if let Some(flow) = self.run_leaf_fast(plan, node_idx, colts, bindings, sink, counters)
            {
                return flow;
            }
        }
        counters.node_entry(node_idx);

        // Dynamic cover choice (§4.4): compare magnitudes first across
        // Exact and Estimate alike; Exact wins only a magnitude tie.
        // A full tie keeps the lowest subatom index (40-execution).
        let cover_sub = self.choose_cover(plan, node_idx, colts);
        let node = &plan.nodes()[node_idx];
        let cover_occ = usize::from(node.subatoms[cover_sub].occ.0);
        let (cover_cursor, cover_level) = self.cursors[cover_occ];
        counters.cover_choice(
            node_idx,
            cover_sub,
            matches!(colts[cover_occ].key_count(cover_cursor), KeyCount::Exact(_)),
        );

        // Word-level batch arity: an interval cover variable contributes
        // its two key words (the SlotWidth layout).
        let arity = self.slot_map[node_idx][cover_sub].len();
        let mut scratch = std::mem::take(&mut self.scratch[node_idx]);

        // Resolve value sources against the runtime cover choice, one
        // source per key WORD: a var bound by the chosen cover reads the
        // batch key words at its word base; everything else reads its
        // (already bound) outer slots.
        let cover_vars = &plan.nodes()[node_idx].subatoms[cover_sub].vars;
        for (sub_idx, subatom) in plan.nodes()[node_idx].subatoms.iter().enumerate() {
            scratch.sources[sub_idx].clear();
            let mut word = 0;
            for var in &subatom.vars {
                let width = self.width_of(*var);
                let base = super::word_base(cover_vars, *var, |v| self.width_of(v));
                for offset in 0..width {
                    scratch.sources[sub_idx].push(match base {
                        Some(base) => Source::Batch(base + offset),
                        None => Source::Slot(self.slot_map[node_idx][sub_idx][word + offset]),
                    });
                }
                word += width;
            }
        }
        scratch.residual_sources.clear();
        for (residual, lhs_slot, rhs_slot, _) in &self.residual_slots[node_idx] {
            let resolve = |var: crate::ir::VarId, slot: usize| {
                super::word_base(cover_vars, var, |v| self.width_of(v))
                    .map_or(Source::Slot(slot), Source::Batch)
            };
            scratch.residual_sources.push((
                resolve(residual.lhs, *lhs_slot),
                resolve(residual.rhs, *rhs_slot),
            ));
        }
        // Word residuals: sources pre-offset to the compared word — a
        // cover-bound side reads its word base plus the residual's
        // Start/End offset.
        scratch.word_residual_sources.clear();
        for (residual, lhs_slot, rhs_slot) in &self.word_residual_slots[node_idx] {
            let resolve = |side: crate::ir::normalize::VarWord, slot: usize| {
                super::word_base(cover_vars, side.var, |v| self.width_of(v))
                    .map_or(Source::Slot(slot), |base| {
                        Source::Batch(base + side.word.offset())
                    })
            };
            scratch.word_residual_sources.push((
                resolve(residual.lhs, *lhs_slot),
                resolve(residual.rhs, *rhs_slot),
            ));
        }
        // Allen residuals: one base source per side; evaluation reads
        // the (start, end) pair at offsets 0/1.
        scratch.allen_sources.clear();
        for (residual, lhs_slot, rhs_slot) in &self.allen_residual_slots[node_idx] {
            let resolve = |var: crate::ir::VarId, slot: usize| {
                super::word_base(cover_vars, var, |v| self.width_of(v))
                    .map_or(Source::Slot(slot), Source::Batch)
            };
            scratch.allen_sources.push((
                resolve(residual.lhs, *lhs_slot),
                resolve(residual.rhs, *rhs_slot),
            ));
        }
        // Measure residuals: the interval side at its word base (pair
        // read at offsets 0/1), the u64 side at its single word.
        scratch.duration_sources.clear();
        for (residual, interval_slot, scalar_slot) in &self.duration_residual_slots[node_idx] {
            let resolve = |var: crate::ir::VarId, slot: usize| {
                super::word_base(cover_vars, var, |v| self.width_of(v))
                    .map_or(Source::Slot(slot), Source::Batch)
            };
            scratch.duration_sources.push((
                resolve(residual.interval, *interval_slot),
                resolve(residual.scalar, *scalar_slot),
            ));
        }

        let mut token = BatchToken::default();
        let mut flow = Flow::Continue;

        'outer: loop {
            counters.phase_start(node_idx, JoinPhase::Iter);
            let (yielded, next_token) = colts[cover_occ].iter_batch(
                cover_cursor,
                cover_level,
                token,
                &mut scratch.entry_keys,
                &mut scratch.children,
                self.batch,
            );
            counters.phase_end(node_idx, JoinPhase::Iter);
            if yielded == 0 {
                break;
            }
            counters.batch(node_idx, yielded);
            token = next_token;
            scratch.survivors.clear();
            scratch
                .survivors
                .extend(0..u32::try_from(yielded).expect("batch fits u32"));

            // Per sibling: the two-phase probe, then branchless
            // compaction. `probe_pass.rs` hosts this pass's pipelined
            // twin, kept line-parallel — a change here needs its mirror
            // there. Extracting the shared pass was refused: the bodies
            // differ in more than parameters (this pass probes one
            // batch-constant cursor per sibling and elides hashing for
            // pinned rows; the twin sources a carried cursor PER ELEMENT
            // and re-resolves value sources per pass), so the honest
            // shape is two commented copies, not one function whose
            // closure parameters reintroduce the difference.
            let value_of = |sources: &[Source],
                            entry_keys: &[u64],
                            bindings: &Bindings,
                            entry: usize,
                            i: usize| match sources[i] {
                Source::Batch(word) => entry_keys[entry * arity + word],
                Source::Slot(slot) => bindings.get(slot),
            };
            for sub_idx in 0..plan.nodes()[node_idx].subatoms.len() {
                if sub_idx == cover_sub || scratch.survivors.is_empty() {
                    continue;
                }
                let subatom = &plan.nodes()[node_idx].subatoms[sub_idx];
                let sub_arity = self.slot_map[node_idx][sub_idx].len();
                let occ = usize::from(subatom.occ.0);
                let (s_cursor, s_level) = self.cursors[occ];
                counters.phase_start(node_idx, JoinPhase::Force);
                colts[occ].ensure_forced(s_cursor, s_level);
                counters.phase_end(node_idx, JoinPhase::Force);

                // Phase 1: gather every probe key and compute every hash —
                // pure ALU, no bucket loads. A pinned sibling
                // (`Cursor::Row`) probes by field equality, never by
                // hash: skip the hash work and its counter, so EXPLAIN's
                // `hashes` counts hashes actually computed for map
                // probes (one branch per sibling per batch).
                let pinned = matches!(s_cursor, Cursor::Row(_));
                counters.phase_start(node_idx, JoinPhase::Hash);
                let n = scratch.survivors.len();
                scratch.hashes.clear();
                scratch.hashes.resize(n, 0);
                // One gather loop for every source shape (the
                // single-batch-word twin measured < 2% and died).
                {
                    let survivors = &scratch.survivors[..n];
                    let entry_keys = &scratch.entry_keys[..];
                    let sources = &scratch.sources[sub_idx];
                    let probe_keys = &mut scratch.probe_keys[..n * sub_arity.max(1)];
                    let hashes = &mut scratch.hashes[..n];
                    for (k, &e) in survivors.iter().enumerate() {
                        let entry = usize::try_from(e).expect("batch fits usize");
                        for i in 0..sub_arity {
                            probe_keys[k * sub_arity + i] =
                                value_of(sources, entry_keys, bindings, entry, i);
                        }
                        if !pinned {
                            counters.probe_hash(node_idx, sub_idx);
                            hashes[k] = crate::exec::colt::hash_key(
                                &probe_keys[k * sub_arity..(k + 1) * sub_arity],
                            );
                        }
                    }
                }
                counters.phase_end(node_idx, JoinPhase::Hash);

                // Phase 1.5: the prefetch pass — every bucket the batch will
                // probe gets its ctrl and bucket lines hinted. Gated on
                // RESIDENCY first (an L2-resident map's prefetch is pure
                // loss) and batch width second (tiny batches never
                // amortize the pass).
                if !pinned && scratch.survivors.len() >= PREFETCH_WIDTH_FLOOR {
                    crate::obs::event(
                        crate::obs::names::PREFETCH_PASS,
                        crate::obs::Category::Execute,
                        scratch.survivors.len() as u64,
                        colts[occ].probe_footprint_bytes() as u64,
                    );
                    for &hash in &scratch.hashes {
                        colts[occ].prefetch_bucket(s_cursor, hash);
                    }
                }

                // Phase 2: all bucket loads — independent chains the
                // out-of-order window overlaps — then kernel compaction.
                // Alias-hoisted locals.
                counters.phase_start(node_idx, JoinPhase::Probe);
                scratch.mask.clear();
                scratch.mask.resize(n, 0);
                {
                    let survivors = &scratch.survivors[..n];
                    let probe_keys = &scratch.probe_keys[..n * sub_arity.max(1)];
                    let hashes = &scratch.hashes[..n];
                    let sibling_children = &mut scratch.sibling_children[sub_idx][..];
                    let mask = &mut scratch.mask[..n];
                    let colt = &mut colts[occ];
                    for k in 0..n {
                        let entry = usize::try_from(survivors[k]).expect("batch fits usize");
                        let hit = colt.get_prehashed(
                            s_cursor,
                            s_level,
                            &probe_keys[k * sub_arity..(k + 1) * sub_arity],
                            hashes[k],
                        );
                        counters.probe(node_idx, sub_idx, hit.is_some());
                        sibling_children[entry] = hit.unwrap_or(Cursor::Row(0));
                        mask[k] = u8::from(hit.is_some());
                    }
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
                counters.phase_end(node_idx, JoinPhase::Probe);
            }

            // Residuals run as batch survivor compaction after the probes.
            counters.phase_start(node_idx, JoinPhase::Residual);
            for (r_idx, (lhs_src, rhs_src)) in scratch.residual_sources.iter().enumerate() {
                let (residual, _, _, width) = &self.residual_slots[node_idx][r_idx];
                let op = residual.op;
                let n = scratch.survivors.len();
                scratch.mask.clear();
                scratch.mask.resize(n, 0);
                for k in 0..n {
                    let e = scratch.survivors[k];
                    let entry = usize::try_from(e).expect("batch fits usize");
                    let value = |src: &Source, offset: usize| match *src {
                        Source::Batch(word) => scratch.entry_keys[entry * arity + word + offset],
                        Source::Slot(slot) => bindings.get(slot + offset),
                    };
                    let pass = super::compare_wide(
                        op,
                        *width,
                        |offset| value(lhs_src, offset),
                        |offset| value(rhs_src, offset),
                    );
                    counters.residual(node_idx, pass);
                    scratch.mask[k] = u8::from(pass);
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            }
            // Word residuals: the decomposed interval compositions —
            // single-word compares over pre-offset slot pairs, compacted
            // exactly like the whole-value residuals above
            // (docs/architecture/20-query-ir.md, § normalization).
            for (r_idx, (lhs_src, rhs_src)) in scratch.word_residual_sources.iter().enumerate() {
                let op = self.word_residual_slots[node_idx][r_idx].0.op;
                let n = scratch.survivors.len();
                scratch.mask.clear();
                scratch.mask.resize(n, 0);
                for k in 0..n {
                    let e = scratch.survivors[k];
                    let entry = usize::try_from(e).expect("batch fits usize");
                    let value = |src: &Source| match *src {
                        Source::Batch(word) => scratch.entry_keys[entry * arity + word],
                        Source::Slot(slot) => bindings.get(slot),
                    };
                    let pass = op.compare(&value(lhs_src), &value(rhs_src));
                    counters.residual(node_idx, pass);
                    scratch.mask[k] = u8::from(pass);
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            }
            // Allen residuals: gather the four endpoint streams per
            // survivor (the gathered shape — batch key words or binding
            // slots), classify the whole batch through the configuration
            // kernel (8 predicate lanes, the 64-byte `tbl` nibble table,
            // the broadcast mask), and compact on the branchless
            // cursor-write like every residual — no per-element
            // classify, no scalar flag chain
            // (docs/architecture/40-execution.md, § vectorized
            // execution).
            for (r_idx, (lhs_src, rhs_src)) in scratch.allen_sources.iter().enumerate() {
                let mask = self.allen_masks[node_idx][r_idx];
                let n = scratch.survivors.len();
                scratch.allen_gather.clear();
                scratch.allen_gather.resize(4 * n, 0);
                let (a_starts, rest) = scratch.allen_gather.split_at_mut(n);
                let (a_ends, rest) = rest.split_at_mut(n);
                let (b_starts, b_ends) = rest.split_at_mut(n);
                for k in 0..n {
                    let e = scratch.survivors[k];
                    let entry = usize::try_from(e).expect("batch fits usize");
                    let value = |src: &Source, offset: usize| match *src {
                        Source::Batch(word) => scratch.entry_keys[entry * arity + word + offset],
                        Source::Slot(slot) => bindings.get(slot + offset),
                    };
                    a_starts[k] = value(lhs_src, 0);
                    a_ends[k] = value(lhs_src, 1);
                    b_starts[k] = value(rhs_src, 0);
                    b_ends[k] = value(rhs_src, 1);
                }
                crate::exec::kernel::allen_code_batch(
                    a_starts,
                    a_ends,
                    b_starts,
                    b_ends,
                    &mut scratch.allen_codes,
                );
                crate::exec::kernel::allen_filter_batch(
                    &scratch.allen_codes,
                    mask,
                    &mut scratch.mask,
                );
                for &keep in &scratch.mask {
                    counters.residual(node_idx, keep != 0);
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            }
            // Measure residuals: per survivor, read the interval pair at
            // the word base, test the ray — `end == MAX` poisons the
            // execution with the offending words (`execute` raises the
            // typed `MeasureOfRay`) — subtract, and compare the u64 word;
            // survivors compact on the same cursor-write. The gathered
            // shape stays scalar per the standing rule (the dense
            // stride-1 twin is the view kernel,
            // `exec::kernel::filter_duration_range_u64`).
            for (r_idx, (interval_src, scalar_src)) in scratch.duration_sources.iter().enumerate() {
                let op = self.duration_residual_slots[node_idx][r_idx].0.op;
                let n = scratch.survivors.len();
                scratch.mask.clear();
                scratch.mask.resize(n, 0);
                for k in 0..n {
                    let e = scratch.survivors[k];
                    let entry = usize::try_from(e).expect("batch fits usize");
                    let value = |src: &Source, offset: usize| match *src {
                        Source::Batch(word) => scratch.entry_keys[entry * arity + word + offset],
                        Source::Slot(slot) => bindings.get(slot + offset),
                    };
                    let (start, end) = (value(interval_src, 0), value(interval_src, 1));
                    if end == u64::MAX {
                        self.measure_of_ray = Some([start, end]);
                        self.all_cancelled = true;
                        break;
                    }
                    let pass = op.compare(&(end - start), &value(scalar_src, 0));
                    counters.residual(node_idx, pass);
                    scratch.mask[k] = u8::from(pass);
                }
                if self.measure_of_ray.is_some() {
                    counters.phase_end(node_idx, JoinPhase::Residual);
                    break 'outer;
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            }
            // Membership probes (docs/architecture/40-execution.md, the
            // point-membership scan): per surviving binding, scan the
            // occurrence's remaining positions for one fact satisfying
            // every var-sourced membership; misses compact away.
            for spec in &self.point_probe_slots[node_idx] {
                let n = scratch.survivors.len();
                scratch.mask.clear();
                scratch.mask.resize(n, 0);
                for k in 0..n {
                    let e = scratch.survivors[k];
                    let entry = usize::try_from(e).expect("batch fits usize");
                    scratch.point_checks.clear();
                    for (start_col, end_col, var, slot) in &spec.parts {
                        let point = super::word_base(cover_vars, *var, |v| self.width_of(v))
                            .map_or_else(
                                || bindings.get(*slot),
                                |base| scratch.entry_keys[entry * arity + base],
                            );
                        scratch.point_checks.push((*start_col, *end_col, point));
                    }
                    let cursor = if spec.occ == cover_occ {
                        scratch.children[entry]
                    } else if let Some(sub_idx) = plan.nodes()[node_idx]
                        .subatoms
                        .iter()
                        .position(|sub| usize::from(sub.occ.0) == spec.occ)
                    {
                        scratch.sibling_children[sub_idx][entry]
                    } else {
                        self.cursors[spec.occ].0
                    };
                    let pass = colts[spec.occ].any_position_matches(cursor, &scratch.point_checks);
                    counters.residual(node_idx, pass);
                    scratch.mask[k] = u8::from(pass);
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            }
            counters.phase_end(node_idx, JoinPhase::Residual);

            // Anti-probes: the residual step's sibling (docs/architecture/
            // 40-execution.md, § anti-probe filters) — this node's lowered
            // negated atoms probe per surviving binding; hits are
            // compacted away on the same cursor-write. Slot reads come
            // from the outer bindings (constant across the batch).
            anti_probe_pass(
                &self.anti_probe_slots[node_idx],
                node_idx,
                cover_vars,
                &self.var_widths,
                arity,
                colts,
                &scratch.entry_keys,
                &mut scratch.survivors,
                &mut scratch.probe_keys,
                &mut scratch.hashes,
                &mut scratch.mask,
                &mut scratch.anti_sources,
                &mut scratch.point_checks,
                |_, slot| bindings.get(slot),
                counters,
            );

            // The leaf: the batch is handed to the sink whole (this IS
            // the last plan node — the entry assert). No recursion, no
            // journal, no cursor writes — nothing below reads them — and
            // no binding stores for the leaf's own vars (the batch
            // carries them). `stop_on_skip` folds this node's
            // sink-relevance into the batch call: when the leaf binds
            // nothing sink-relevant, the sink stops at its first emit and
            // the skip unwinds here exactly as the recursive path's
            // absorption arm did.
            if scratch.survivors.is_empty() {
                continue;
            }
            counters.phase_start(node_idx, JoinPhase::Descend);
            let batch = LeafBatch {
                keys: &scratch.entry_keys,
                arity,
                survivors: &scratch.survivors,
                key_slots: &self.slot_map[node_idx][cover_sub],
                bindings,
            };
            let stop_on_skip = !plan.nodes()[node_idx].sink_relevant && sink.may_skip();
            let batch_flow = sink.emit_batch(&batch, stop_on_skip);
            // EXPLAIN's `emits` counts rows the sink consumed: the
            // whole batch, or exactly one when the first emit's skip
            // stopped it (identical to the recursive path's counts).
            let emitted = if batch_flow == Flow::SkipSuffix {
                1
            } else {
                scratch.survivors.len()
            };
            for _ in 0..emitted {
                counters.emit();
            }
            counters.phase_end(node_idx, JoinPhase::Descend);
            if batch_flow == Flow::SkipSuffix {
                debug_assert!(
                    sink.may_skip(),
                    "a SkipSuffix crossed a node under a non-skipping sink"
                );
                counters.skip(node_idx);
                flow = Flow::SkipSuffix;
                break 'outer;
            }
        }

        self.scratch[node_idx] = scratch;
        flow
    }

    /// Chooses the cover with the smallest magnitude; `Exact` wins only
    /// a magnitude tie, and a full tie keeps the lowest subatom index
    /// (v0 rule, 40-execution).
    fn choose_cover(&self, plan: &ValidatedPlan, node_idx: usize, colts: &[Colt]) -> usize {
        let node = &plan.nodes()[node_idx];
        let mut best: Option<(usize, KeyCount)> = None;
        for &cover in &node.covers {
            let sub_idx = usize::from(cover);
            let occ = usize::from(node.subatoms[sub_idx].occ.0);
            let count = colts[occ].key_count(self.cursors[occ].0);
            let better = match &best {
                None => true,
                Some((_, incumbent)) => better_cover(count, *incumbent),
            };
            if better {
                best = Some((sub_idx, count));
            }
        }
        best.expect("validated plans have non-empty cover sets").0
    }
}
