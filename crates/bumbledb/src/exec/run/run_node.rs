//! The leaf pass over one node's cover batch (single-node and last-node).
use super::anti_probe::anti_probe_pass;
use super::filter::{allen_pass, gather_probe_keys, residual_pass, resolve_points};
use super::{
    BatchToken, Colt, Counters, CoverAt, Cursor, Executor, Flow, JoinCtx, KeyCount, LeafBatch,
    LeafRows, NodeScratch, ProbeCursor, SiblingProbe, Sink, ValidatedPlan, better_cover,
    grow_scratch,
};

impl Executor {
    pub(super) fn run_node<S: Sink, C: Counters>(
        &mut self,
        cx: &mut JoinCtx<'_, S, C>,
        node_idx: usize,
        scratch: &mut NodeScratch,
    ) -> Flow {
        let plan = cx.plan;
        assert!(
            node_idx + 1 == plan.nodes().len(),
            "run_node is the leaf pass; middle nodes pump"
        );

        if matches!(self.leaf, super::LeafPrecompute::Fast { .. })
            && let Some(flow) = self.run_leaf_fast(cx, node_idx)
        {
            return flow;
        }
        cx.counters.node_entry(node_idx);

        let cover_sub = self.choose_cover(plan, node_idx, cx.colts);
        let node = &plan.nodes()[node_idx];
        let cover_occ = usize::from(node.subatoms[cover_sub].occ.0);
        let (cover_cursor, cover_level) = self.cursors[cover_occ];
        if S::may_use_distinct_traversal()
            && self.physical_distinct.is_some()
            && self
                .colt_ok(cx.colts[cover_occ].force_distinct_iteration(cover_cursor, cover_level))
                .is_none()
        {
            return Flow::Error;
        }
        cx.counters.cover_choice(
            node_idx,
            cover_sub,
            cx.colts[cover_occ].key_count(cover_cursor),
        );

        let arity = self.slot_map[node_idx][cover_sub].len();
        // Leaves do not recurse. Only a membership probe on this cover can
        // consume its children; all other work reads the batch's key words.
        let needs_children = !scratch.children[cover_sub].is_empty();

        let gate_cover = arity == 0 && !self.point_probed[cover_occ];
        scratch.layout.prepare(
            &self.slot_map[node_idx],
            &self.precompute[node_idx],
            cover_sub,
        );

        let overlap = self.overlap_enumerate(
            plan,
            node_idx,
            CoverAt {
                occ: cover_occ,
                cursor: cover_cursor,
                level: cover_level,
            },
            &cx.colts[cover_occ],
            cx.bindings,
            &scratch.layout.allen_sources,
        );
        let mut overlap_drained = 0usize;

        let mut token = BatchToken::default();
        let mut flow = Flow::Continue;

        'outer: loop {
            let (yielded, next_token) = if overlap {
                let take = (self.overlap_hits.len() - overlap_drained).min(self.batch);
                super::overlap_leaf::overlap_gather(
                    &cx.colts[cover_occ],
                    cover_level,
                    arity,
                    &self.overlap_hits[overlap_drained..overlap_drained + take],
                    &mut scratch.entry_keys,
                    needs_children.then_some(scratch.children[cover_sub].as_mut_slice()),
                );
                overlap_drained += take;
                (take, token)
            } else {
                let max = if gate_cover { 1 } else { self.batch };
                let batch = if needs_children {
                    cx.colts[cover_occ].iter_batch(
                        cover_cursor,
                        cover_level,
                        token,
                        &mut scratch.entry_keys,
                        &mut scratch.children[cover_sub],
                        max,
                    )
                } else {
                    cx.colts[cover_occ].iter_keys_batch(
                        cover_cursor,
                        cover_level,
                        token,
                        &mut scratch.entry_keys,
                        max,
                    )
                };
                let Some(batch) = self.colt_ok(batch) else {
                    break 'outer;
                };
                batch
            };
            if yielded == 0 {
                break;
            }
            cx.counters.batch(node_idx, yielded);
            token = next_token;
            // Poll cancellation during exploration, even when no row
            // survives to the sink.
            if !self.note_explored(yielded) {
                break 'outer;
            }
            scratch.batch.survivors.clear();
            scratch
                .batch
                .survivors
                .extend(0..u32::try_from(yielded).expect("batch fits u32"));

            let rows = LeafRows {
                keys: &scratch.entry_keys,
                arity,
                bindings: cx.bindings,
            };
            let pre = &self.precompute[node_idx];
            // Reject comparison failures before probing sibling tries.
            residual_pass(
                &pre.residual_slots,
                &scratch.layout.residual_sources,
                &rows,
                &mut scratch.batch,
                node_idx,
                cx.counters,
            );
            allen_pass(
                &pre.allen_residual_slots,
                &scratch.layout.allen_sources,
                &rows,
                &mut scratch.batch,
                node_idx,
                cx.counters,
            );

            // Probe siblings only for bindings that survived the residuals.
            for sub_idx in 0..node.subatoms.len() {
                if sub_idx == cover_sub || scratch.batch.survivors.is_empty() {
                    continue;
                }
                let sub_arity = self.slot_map[node_idx][sub_idx].len();
                let occ = usize::from(node.subatoms[sub_idx].occ.0);
                let (s_cursor, s_level) = self.cursors[occ];
                if self
                    .colt_ok(cx.colts[occ].ensure_forced(s_cursor, s_level))
                    .is_none()
                {
                    break 'outer;
                }

                let pinned = matches!(s_cursor, Cursor::Row(_));
                let n = scratch.batch.survivors.len();
                grow_scratch(&mut scratch.batch.hashes, n);
                gather_probe_keys(
                    &LeafRows {
                        keys: &scratch.entry_keys,
                        arity,
                        bindings: cx.bindings,
                    },
                    &scratch.layout.sources[sub_idx],
                    &scratch.batch.survivors,
                    &mut scratch.batch.probe_keys[..n * sub_arity.max(1)],
                    &mut scratch.batch.hashes[..n],
                    !pinned,
                );
                if !pinned {
                    for _ in 0..n {
                        cx.counters.probe_hash(node_idx, sub_idx);
                    }
                }

                cx.counters.probe_batch(node_idx, sub_idx, n);
                grow_scratch(&mut scratch.batch.mask, n);
                self.probe_sibling_batch::<0, C>(
                    scratch,
                    &mut cx.colts[occ],
                    SiblingProbe {
                        node: node_idx,
                        sub: sub_idx,
                        level: s_level,
                        arity: sub_arity,
                        cursor: ProbeCursor::Shared(s_cursor),
                    },
                    cx.counters,
                );
                if !matches!(self.drive_state, super::DriveState::Running) {
                    break 'outer;
                }
                crate::exec::kernel::compact_u32_by_mask(
                    &mut scratch.batch.survivors,
                    &scratch.batch.mask,
                );
            }

            let rows = LeafRows {
                keys: &scratch.entry_keys,
                arity,
                bindings: cx.bindings,
            };
            // Only surviving sibling matches need interval membership checks.
            for (spec, point_sources) in self.precompute[node_idx]
                .point_probes
                .iter()
                .zip(&scratch.layout.point_sources)
            {
                let sub_idx = node
                    .subatoms
                    .iter()
                    .position(|sub| usize::from(sub.occ.0) == spec.occ);
                let buf = &mut scratch.batch;
                let n = buf.survivors.len();
                grow_scratch(&mut buf.mask, n);
                for k in 0..n {
                    let entry = buf.survivors[k] as usize;
                    resolve_points(&rows, entry, point_sources, &mut buf.point_checks);
                    let cursor = sub_idx.map_or(self.cursors[spec.occ].0, |sub_idx| {
                        scratch.children[sub_idx][entry]
                    });
                    let pass = cx.colts[spec.occ].any_position_matches(cursor, &buf.point_checks);
                    cx.counters.residual(node_idx, pass);
                    buf.mask[k] = u8::from(pass);
                }
                crate::exec::kernel::compact_u32_by_mask(&mut buf.survivors, &buf.mask);
            }

            if let Err(error) = anti_probe_pass(
                &self.precompute[node_idx].anti_probes,
                &scratch.layout,
                &rows,
                &mut scratch.batch,
                cx.colts,
                node_idx,
                cx.counters,
            ) {
                self.poison(super::Poison::Work(error));
                break 'outer;
            }

            if scratch.batch.survivors.is_empty() {
                if gate_cover {
                    break;
                }
                continue;
            }
            let batch = LeafBatch {
                keys: &scratch.entry_keys,
                arity,
                survivors: &scratch.batch.survivors,
                key_slots: &self.slot_map[node_idx][cover_sub],
                bindings: cx.bindings,
            };
            let batch_flow = super::emit_node_batch(cx.sink, node.suffix_skip, &batch);

            let emitted = if batch_flow == Flow::SkipSuffix {
                1
            } else {
                scratch.batch.survivors.len()
            };
            for _ in 0..emitted {
                cx.counters.emit();
            }
            if batch_flow.is_terminal() {
                self.poison(match batch_flow {
                    Flow::Stop => super::Poison::SinkStop,
                    _ => super::Poison::SinkError,
                });
                flow = batch_flow;
                break 'outer;
            }
            if batch_flow == Flow::SkipSuffix {
                debug_assert!(
                    cx.sink.skip_capability() == super::SkipCapability::Licensed,
                    "a SkipSuffix crossed a node under a non-skipping sink"
                );
                cx.counters.skip(node_idx);
                flow = Flow::SkipSuffix;
                break 'outer;
            }
            if gate_cover {
                break;
            }
        }

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
