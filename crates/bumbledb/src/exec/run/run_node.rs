//! The leaf pass over one node's cover batch (single-node and last-node).
use super::anti_probe::anti_probe_pass;
use super::{
    BatchToken, Bindings, Colt, Counters, Cursor, Executor, Flow, JoinPhase, KeyCount, LeafBatch,
    PREFETCH_WIDTH_FLOOR, Sink, Source, ValidatedPlan, better_cover, grow_scratch,
};

impl Executor {
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )]
    pub(super) fn run_node<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Flow {
        assert!(
            node_idx + 1 == plan.nodes().len(),
            "run_node is the leaf pass; middle nodes pump"
        );

        if matches!(self.leaf, super::LeafPrecompute::Fast { .. })
            && let Some(flow) = self.run_leaf_fast(plan, node_idx, colts, bindings, sink, counters)
        {
            return flow;
        }
        counters.node_entry(node_idx);

        let cover_sub = self.choose_cover(plan, node_idx, colts);
        let node = &plan.nodes()[node_idx];
        let cover_occ = usize::from(node.subatoms[cover_sub].occ.0);
        let (cover_cursor, cover_level) = self.cursors[cover_occ];
        counters.cover_choice(
            node_idx,
            cover_sub,
            colts[cover_occ].key_count(cover_cursor),
        );

        let arity = self.slot_map[node_idx][cover_sub].len();

        let gate_cover = arity == 0 && !self.point_probed[cover_occ];
        let mut scratch = std::mem::take(&mut self.scratch[node_idx]);

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
        for spec in &self.precompute[node_idx].residual_slots {
            let resolve = |var: crate::ir::VarId, slot: usize| {
                super::word_base(cover_vars, var, |v| self.width_of(v))
                    .map_or(Source::Slot(slot), Source::Batch)
            };
            scratch.residual_sources.push((
                resolve(spec.lhs, spec.lhs_slot),
                resolve(spec.rhs, spec.rhs_slot),
            ));
        }

        scratch.word_residual_sources.clear();
        for spec in &self.precompute[node_idx].word_residual_slots {
            let resolve = |side: crate::image::view::OperandAddr, slot: usize| {
                super::word_base(cover_vars, side.var(), |v| self.width_of(v))
                    .map_or(Source::Slot(slot), |base| {
                        Source::Batch(base + side.offset())
                    })
            };
            scratch.word_residual_sources.push((
                resolve(spec.left, spec.lhs_slot),
                resolve(spec.right, spec.rhs_slot),
            ));
        }

        scratch.allen_sources.clear();
        for spec in &self.precompute[node_idx].allen_residual_slots {
            let resolve = |var: crate::ir::VarId, slot: usize| {
                super::word_base(cover_vars, var, |v| self.width_of(v))
                    .map_or(Source::Slot(slot), Source::Batch)
            };
            scratch.allen_sources.push((
                resolve(spec.lhs, spec.lhs_slot),
                resolve(spec.rhs, spec.rhs_slot),
            ));
        }

        counters.phase_start(node_idx, JoinPhase::Iter);
        let overlap = self.overlap_enumerate(
            plan,
            node_idx,
            cover_occ,
            cover_cursor,
            cover_level,
            &colts[cover_occ],
            bindings,
            &scratch.allen_sources,
        );
        counters.phase_end(node_idx, JoinPhase::Iter);
        let mut overlap_drained = 0usize;

        let mut token = BatchToken::default();
        let mut flow = Flow::Continue;

        'outer: loop {
            counters.phase_start(node_idx, JoinPhase::Iter);
            let (yielded, next_token) = if overlap {
                let take = (self.overlap_hits.len() - overlap_drained).min(self.batch);
                super::overlap_leaf::overlap_gather(
                    &colts[cover_occ],
                    cover_level,
                    arity,
                    &self.overlap_hits[overlap_drained..overlap_drained + take],
                    &mut scratch.entry_keys,
                    &mut scratch.children,
                );
                overlap_drained += take;
                (take, token)
            } else {
                colts[cover_occ].iter_batch(
                    cover_cursor,
                    cover_level,
                    token,
                    &mut scratch.entry_keys,
                    &mut scratch.children,
                    if gate_cover { 1 } else { self.batch },
                )
            };
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

            // Residuals run BEFORE the sibling probes — the cost-class

            counters.phase_start(node_idx, JoinPhase::Residual);
            for (r_idx, (lhs_src, rhs_src)) in scratch.residual_sources.iter().enumerate() {
                let spec = &self.precompute[node_idx].residual_slots[r_idx];
                let n = scratch.survivors.len();
                grow_scratch(&mut scratch.mask, n);
                for k in 0..n {
                    let e = scratch.survivors[k];
                    let entry = usize::try_from(e).expect("batch fits usize");
                    let value = |src: &Source, offset: usize| match *src {
                        Source::Batch(word) => scratch.entry_keys[entry * arity + word + offset],
                        Source::Slot(slot) => bindings.get(slot + offset),
                    };
                    let pass = super::compare_wide(
                        spec.op,
                        spec.width,
                        |offset| value(lhs_src, offset),
                        |offset| value(rhs_src, offset),
                    );
                    counters.residual(node_idx, pass);
                    scratch.mask[k] = u8::from(pass);
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            }

            for (r_idx, (lhs_src, rhs_src)) in scratch.word_residual_sources.iter().enumerate() {
                let op = self.precompute[node_idx].word_residual_slots[r_idx].op;
                let n = scratch.survivors.len();
                grow_scratch(&mut scratch.mask, n);
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

            for (r_idx, (lhs_src, rhs_src)) in scratch.allen_sources.iter().enumerate() {
                let mask = self.precompute[node_idx].allen_masks[r_idx];
                let n = scratch.survivors.len();
                let filter_mask = match (*lhs_src, *rhs_src) {
                    (Source::Batch(lw), Source::Batch(rw)) => {
                        grow_scratch(&mut scratch.allen_gather, 4 * n);
                        let (a_starts, rest) = scratch.allen_gather[..4 * n].split_at_mut(n);
                        let (a_ends, rest) = rest.split_at_mut(n);
                        let (b_starts, b_ends) = rest.split_at_mut(n);
                        for (k, &e) in scratch.survivors[..n].iter().enumerate() {
                            let entry = usize::try_from(e).expect("batch fits usize");
                            a_starts[k] = scratch.entry_keys[entry * arity + lw];
                            a_ends[k] = scratch.entry_keys[entry * arity + lw + 1];
                            b_starts[k] = scratch.entry_keys[entry * arity + rw];
                            b_ends[k] = scratch.entry_keys[entry * arity + rw + 1];
                        }
                        crate::exec::kernel::allen_code_batch(
                            a_starts,
                            a_ends,
                            b_starts,
                            b_ends,
                            &mut scratch.allen_codes,
                        );
                        Some(mask)
                    }
                    (Source::Batch(word), Source::Slot(slot)) => {
                        allen_classify_const(
                            &scratch.survivors[..n],
                            &scratch.entry_keys,
                            arity,
                            word,
                            (bindings.get(slot), bindings.get(slot + 1)),
                            &mut scratch.allen_gather,
                            &mut scratch.allen_codes,
                        );
                        Some(mask)
                    }
                    (Source::Slot(slot), Source::Batch(word)) => {
                        allen_classify_const(
                            &scratch.survivors[..n],
                            &scratch.entry_keys,
                            arity,
                            word,
                            (bindings.get(slot), bindings.get(slot + 1)),
                            &mut scratch.allen_gather,
                            &mut scratch.allen_codes,
                        );
                        Some(mask.converse())
                    }
                    (Source::Slot(ls), Source::Slot(rs)) => {
                        let code = crate::allen::classify_bounds(
                            &bindings.get(ls),
                            &bindings.get(ls + 1),
                            &bindings.get(rs),
                            &bindings.get(rs + 1),
                        );
                        scratch.mask.clear();
                        scratch.mask.resize(n, u8::from(mask.contains(code)));
                        None
                    }
                };
                if let Some(filter_mask) = filter_mask {
                    crate::exec::kernel::allen_filter_batch(
                        &scratch.allen_codes,
                        filter_mask,
                        &mut scratch.mask,
                    );
                }
                for &keep in &scratch.mask[..n] {
                    counters.residual(node_idx, keep != 0);
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            }
            counters.phase_end(node_idx, JoinPhase::Residual);

            // there. Extracting the shared pass was refused: the bodies

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

                let pinned = matches!(s_cursor, Cursor::Row(_));
                counters.phase_start(node_idx, JoinPhase::Hash);
                let n = scratch.survivors.len();

                grow_scratch(&mut scratch.hashes, n);

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

                if !pinned && scratch.survivors.len() >= PREFETCH_WIDTH_FLOOR {
                    crate::obs::event(
                        crate::obs::names::PREFETCH_PASS,
                        crate::obs::TraceArgs::Pair(
                            scratch.survivors.len() as u64,
                            colts[occ].probe_footprint_bytes() as u64,
                        ),
                    );
                    for &hash in &scratch.hashes[..n] {
                        colts[occ].prefetch_bucket(s_cursor, hash);
                    }
                }

                counters.phase_start(node_idx, JoinPhase::Probe);
                grow_scratch(&mut scratch.mask, n);
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

            // stay AFTER the sibling probes (unlike the ALU residuals

            if !self.precompute[node_idx].point_probes.is_empty() {
                counters.phase_start(node_idx, JoinPhase::Residual);
            }
            for spec in &self.precompute[node_idx].point_probes {
                scratch.point_sources.clear();
                for (start_col, end_col, var, slot) in &spec.parts {
                    let src = super::word_base(cover_vars, *var, |v| self.width_of(v))
                        .map_or(Source::Slot(*slot), Source::Batch);
                    scratch.point_sources.push((*start_col, *end_col, src));
                }
                let cursor_src = if spec.occ == cover_occ {
                    super::CursorSrc::Cover
                } else if let Some(sub_idx) = plan.nodes()[node_idx]
                    .subatoms
                    .iter()
                    .position(|sub| usize::from(sub.occ.0) == spec.occ)
                {
                    super::CursorSrc::Sibling(sub_idx)
                } else {
                    super::CursorSrc::Const(self.cursors[spec.occ].0)
                };
                let n = scratch.survivors.len();
                grow_scratch(&mut scratch.mask, n);
                for k in 0..n {
                    let e = scratch.survivors[k];
                    let entry = usize::try_from(e).expect("batch fits usize");
                    scratch.point_checks.clear();
                    for &(start_col, end_col, src) in &scratch.point_sources {
                        let point = match src {
                            Source::Batch(base) => scratch.entry_keys[entry * arity + base],
                            Source::Slot(slot) => bindings.get(slot),
                        };
                        scratch.point_checks.push((start_col, end_col, point));
                    }
                    let cursor = match cursor_src {
                        super::CursorSrc::Cover => scratch.children[entry],
                        super::CursorSrc::Sibling(sub_idx) => {
                            scratch.sibling_children[sub_idx][entry]
                        }
                        super::CursorSrc::Carried(_) => {
                            unreachable!("the leaf pass carries no pending cursors")
                        }
                        super::CursorSrc::Const(outer) => outer,
                    };
                    let pass = colts[spec.occ].any_position_matches(cursor, &scratch.point_checks);
                    counters.residual(node_idx, pass);
                    scratch.mask[k] = u8::from(pass);
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            }
            if !self.precompute[node_idx].point_probes.is_empty() {
                counters.phase_end(node_idx, JoinPhase::Residual);
            }

            anti_probe_pass(
                &self.precompute[node_idx].anti_probes,
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
                &mut scratch.point_sources,
                |_, slot| bindings.get(slot),
                counters,
            );

            if scratch.survivors.is_empty() {
                if gate_cover {
                    break;
                }
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
            let batch_flow =
                super::emit_node_batch(sink, plan.nodes()[node_idx].suffix_skip, &batch);

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
                    sink.skip_capability() == super::SkipCapability::Licensed,
                    "a SkipSuffix crossed a node under a non-skipping sink"
                );
                counters.skip(node_idx);
                flow = Flow::SkipSuffix;
                break 'outer;
            }
            if gate_cover {
                break;
            }
        }

        self.scratch[node_idx] = scratch;
        flow
    }

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

fn allen_classify_const(
    survivors: &[u32],
    entry_keys: &[u64],
    arity: usize,
    word: usize,
    (b_start, b_end): (u64, u64),
    gather: &mut Vec<u64>,
    codes: &mut Vec<u8>,
) {
    let n = survivors.len();
    grow_scratch(gather, 2 * n);
    let (starts, ends) = gather[..2 * n].split_at_mut(n);
    for (k, &e) in survivors.iter().enumerate() {
        let entry = usize::try_from(e).expect("batch fits usize");
        starts[k] = entry_keys[entry * arity + word];
        ends[k] = entry_keys[entry * arity + word + 1];
    }
    crate::exec::kernel::allen_code_batch_const(starts, ends, b_start, b_end, codes);
}
