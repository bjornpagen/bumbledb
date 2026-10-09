//! One cross-parent probe pass.
use super::anti_probe::anti_probe_pass;
use super::filter::{allen_pass, gather_probe_keys, residual_pass, resolve_points};
use super::{
    BatchRows, Colt, Counters, CoverBatch, Cursor, Executor, Flow, JoinCtx, NodeScratch,
    PREFETCH_WIDTH_FLOOR, PendingRows, PipeTables, ProbeCursor, SiblingProbe, Sink, grow_scratch,
};

impl Executor {
    pub(super) fn probe_pass<S: Sink, C: Counters>(
        &mut self,
        tables: &PipeTables,
        cx: &mut JoinCtx<'_, S, C>,
        batch: CoverBatch,
        scratch: &mut NodeScratch,
        below: &mut [NodeScratch],
    ) {
        let CoverBatch {
            node: node_idx,
            cover_sub,
            arity,
            fill,
        } = batch;
        if !matches!(self.drive_state, super::DriveState::Running) {
            scratch.parents.clear();
            scratch.element_origins.clear();
            return;
        }
        let plan = cx.plan;
        let n_nodes = plan.nodes().len();
        let slot_count = cx.bindings.slot_count();
        let carried_w = tables.carried[node_idx].len();
        let node = &plan.nodes()[node_idx];
        scratch.layout.prepare(
            &self.slot_map[node_idx],
            &self.precompute[node_idx],
            cover_sub,
        );

        scratch.batch.survivors.clear();
        scratch
            .batch
            .survivors
            .extend(0..u32::try_from(fill).expect("batch fits u32"));

        let rows = PendingRows {
            keys: &scratch.entry_keys,
            arity,
            parents: &scratch.parents,
            bindings: &scratch.pending_bindings,
            slot_count,
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

        for sub_idx in 0..node.subatoms.len() {
            if sub_idx == cover_sub || scratch.batch.survivors.is_empty() {
                continue;
            }
            let sub_arity = self.slot_map[node_idx][sub_idx].len();
            let occ = usize::from(node.subatoms[sub_idx].occ.0);
            let s_level = tables.entry_level[node_idx][occ];
            let n = scratch.batch.survivors.len();
            grow_scratch(&mut scratch.batch.hashes, n);
            gather_probe_keys(
                &PendingRows {
                    keys: &scratch.entry_keys,
                    arity,
                    parents: &scratch.parents,
                    bindings: &scratch.pending_bindings,
                    slot_count,
                },
                &scratch.layout.sources[sub_idx],
                &scratch.batch.survivors,
                &mut scratch.batch.probe_keys[..n * sub_arity],
                &mut scratch.batch.hashes[..n],
                true,
            );
            for _ in 0..n {
                cx.counters.probe_hash(node_idx, sub_idx);
            }
            let cursor = if let Some(column) = tables.carried_index(node_idx, occ) {
                ProbeCursor::Carried {
                    column,
                    width: carried_w,
                }
            } else {
                let start = cx.colts[occ].start();
                if self
                    .colt_ok(cx.colts[occ].ensure_forced(start, s_level))
                    .is_none()
                {
                    scratch.parents.clear();
                    scratch.element_origins.clear();
                    return;
                }
                ProbeCursor::Shared(start)
            };

            cx.counters.probe_batch(node_idx, sub_idx, n);
            grow_scratch(&mut scratch.batch.mask, n);
            let probe = SiblingProbe {
                node: node_idx,
                sub: sub_idx,
                level: s_level,
                arity: sub_arity,
                cursor,
            };
            let colt = &mut cx.colts[occ];
            match sub_arity {
                1 => self.probe_sibling_batch::<1, C>(scratch, colt, probe, cx.counters),
                2 => self.probe_sibling_batch::<2, C>(scratch, colt, probe, cx.counters),
                3 => self.probe_sibling_batch::<3, C>(scratch, colt, probe, cx.counters),
                4 => self.probe_sibling_batch::<4, C>(scratch, colt, probe, cx.counters),
                _ => self.probe_sibling_batch::<0, C>(scratch, colt, probe, cx.counters),
            }
            if !matches!(self.drive_state, super::DriveState::Running) {
                scratch.parents.clear();
                scratch.element_origins.clear();
                return;
            }
            crate::exec::kernel::compact_u32_by_mask(
                &mut scratch.batch.survivors,
                &scratch.batch.mask,
            );
        }

        let rows = PendingRows {
            keys: &scratch.entry_keys,
            arity,
            parents: &scratch.parents,
            bindings: &scratch.pending_bindings,
            slot_count,
        };
        // Membership checks use surviving bindings and their resolved cursors.
        for (spec, point_sources) in self.precompute[node_idx]
            .point_probes
            .iter()
            .zip(&scratch.layout.point_sources)
        {
            let cursor_src = tables.outgoing[node_idx][spec.occ];
            let buf = &mut scratch.batch;
            let n = buf.survivors.len();
            grow_scratch(&mut buf.mask, n);

            buf.point_rows.clear();
            buf.point_row_ks.clear();
            for k in 0..n {
                let element = buf.survivors[k] as usize;
                let parent = scratch.parents[element] as usize;
                let cursor = match cursor_src {
                    super::CursorSrc::Subatom(sub_idx) => scratch.children[sub_idx][element],
                    super::CursorSrc::Carried(col) => {
                        scratch.pending_cursors[parent * carried_w + col]
                    }
                    super::CursorSrc::Start => cx.colts[spec.occ].start(),
                };
                if let Cursor::Row(position) = cursor {
                    buf.point_rows.push(position);
                    buf.point_row_ks
                        .push(u32::try_from(k).expect("batch fits u32"));
                    buf.mask[k] = 1;
                    continue;
                }
                resolve_points(&rows, element, point_sources, &mut buf.point_checks);
                buf.mask[k] =
                    u8::from(cx.colts[spec.occ].any_position_matches(cursor, &buf.point_checks));
            }

            let m = buf.point_rows.len();
            if m > 0 {
                grow_scratch(&mut buf.allen_gather, 2 * m);
                let (starts, ends) = buf.allen_gather[..2 * m].split_at_mut(m);
                for &(start_col, end_col, src, dense) in point_sources {
                    cx.colts[spec.occ].gather_interval_pair(
                        start_col,
                        end_col,
                        &buf.point_rows,
                        starts,
                        ends,
                    );
                    for j in 0..m {
                        let k = buf.point_row_ks[j] as usize;
                        let element = buf.survivors[k] as usize;
                        let point = rows.word(element, src, 0);
                        // The dense finite-probe guard.
                        let point = if dense {
                            crate::image::view::dense_probe_word(point)
                        } else {
                            point
                        };
                        buf.mask[k] &= u8::from(starts[j] <= point) & u8::from(point < ends[j]);
                    }
                }
            }
            for &keep in &buf.mask[..n] {
                cx.counters.residual(node_idx, keep != 0);
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
            scratch.parents.clear();
            scratch.element_origins.clear();
            return;
        }

        let leaf = node_idx + 2 == n_nodes;
        let child_carried = &tables.carried[node_idx + 1];
        let mints_origins = tables.absorb == super::SkipAbsorb::Node(node_idx);

        // Wrapping an origin could alias a cancelled subtree and drop valid
        // rows. Refuse before minting any origins for this batch.

        if mints_origins
            && self
                .next_origin
                .checked_add(u32::try_from(scratch.batch.survivors.len()).expect("batch fits u32"))
                .is_none()
        {
            self.poison(super::Poison::OriginOverflow);
            scratch.parents.clear();
            scratch.element_origins.clear();
            return;
        }
        // Only descendants of the absorbing node carry cancellable origins.
        let below_absorb = matches!(tables.absorb, super::SkipAbsorb::Node(a) if node_idx > a);
        for k in 0..scratch.batch.survivors.len() {
            if !matches!(self.drive_state, super::DriveState::Running) {
                break;
            }
            let element = scratch.batch.survivors[k] as usize;
            let parent = scratch.parents[element] as usize;
            let origin = if mints_origins {
                let minted = self.next_origin;
                self.next_origin += 1;
                minted
            } else {
                scratch.element_origins[element]
            };
            if below_absorb && self.origin_cancelled(origin) {
                continue;
            }

            let assemble = |occ: usize| -> Cursor {
                match tables.outgoing[node_idx][occ] {
                    super::CursorSrc::Subatom(sub_idx) => scratch.children[sub_idx][element],
                    super::CursorSrc::Carried(col) => {
                        scratch.pending_cursors[parent * carried_w + col]
                    }
                    super::CursorSrc::Start => cx.colts[occ].start(),
                }
            };
            if leaf {
                cx.bindings.load_row(
                    &scratch.pending_bindings[parent * slot_count..(parent + 1) * slot_count],
                );
                for (i, slot) in self.slot_map[node_idx][cover_sub].iter().enumerate() {
                    cx.bindings
                        .set(*slot, scratch.entry_keys[element * arity + i]);
                }
                let leaf_node = &plan.nodes()[node_idx + 1];
                for subatom in &leaf_node.subatoms {
                    let occ = usize::from(subatom.occ.0);
                    self.cursors[occ] = (assemble(occ), tables.entry_level[node_idx + 1][occ]);
                }

                for probe in &leaf_node.point_probes {
                    let occ = usize::from(probe.occ.0);
                    self.cursors[occ] = (assemble(occ), tables.entry_level[node_idx + 1][occ]);
                }
                let flow = self.run_node(cx, node_idx + 1, &mut below[0]);
                if flow.is_terminal() {
                    self.poison(match flow {
                        super::Flow::Stop => super::Poison::SinkStop,
                        _ => super::Poison::SinkError,
                    });
                    return;
                }
                if flow == Flow::SkipSuffix {
                    cx.counters.skip(node_idx);
                    match tables.absorb {
                        super::SkipAbsorb::Node(a) if node_idx >= a => self.cancel_origin(origin),
                        super::SkipAbsorb::Node(_) => {}
                        super::SkipAbsorb::Root => {
                            if matches!(self.drive_state, super::DriveState::Running) {
                                self.drive_state = super::DriveState::SkipDone;
                            }
                        }
                    }
                }
            } else {
                let cover_slots = &self.slot_map[node_idx][cover_sub];
                let child = &mut below[0];
                let start = child.pending_bindings.len();
                child.pending_bindings.extend_from_slice(
                    &scratch.pending_bindings[parent * slot_count..(parent + 1) * slot_count],
                );
                for (i, slot) in cover_slots.iter().enumerate() {
                    child.pending_bindings[start + slot] = scratch.entry_keys[element * arity + i];
                }
                child
                    .pending_cursors
                    .extend(child_carried.iter().map(|&occ| assemble(occ)));
                child.pending_origins.push(origin);
                child.pending_len += 1;
            }
        }
        scratch.parents.clear();
        scratch.element_origins.clear();

        if !leaf && below[0].pending_len >= self.batch {
            self.pump(tables, cx, node_idx + 1, below);
        }
    }

    pub(super) fn probe_sibling_batch<const K: usize, C: Counters>(
        &mut self,
        scratch: &mut NodeScratch,
        colt: &mut Colt,
        probe: SiblingProbe,
        counters: &mut C,
    ) {
        // Resolve child liveness once per batch, not once per key. Both leaf
        // and pipeline calls use these same kernels and refusal handling.
        if scratch.children[probe.sub].is_empty() {
            self.probe_sibling_keys::<K, false, C>(scratch, colt, probe, counters);
        } else {
            self.probe_sibling_keys::<K, true, C>(scratch, colt, probe, counters);
        }
    }

    #[inline(never)]
    fn probe_sibling_keys<const K: usize, const CHILDREN: bool, C: Counters>(
        &mut self,
        scratch: &mut NodeScratch,
        colt: &mut Colt,
        probe: SiblingProbe,
        counters: &mut C,
    ) {
        debug_assert!(K == 0 || probe.arity == K);
        let width = if K == 0 { probe.arity } else { K };
        let buf = &mut scratch.batch;
        let n = buf.survivors.len();
        let survivors = &buf.survivors[..n];
        let parents = &scratch.parents[..];
        let pending_cursors = &scratch.pending_cursors[..];
        let probe_keys = &buf.probe_keys[..n * width];
        let hashes = &buf.hashes[..n];
        let children = &mut scratch.children[probe.sub][..];
        let mask = &mut buf.mask[..n];
        let mut probe_one = |trie: &crate::exec::colt::Probe<'_>, k: usize, element: usize| {
            let key = &probe_keys[k * width..(k + 1) * width];
            let hit = if CHILDREN {
                let hit = trie.get_prehashed_width::<K>(key, hashes[k]);
                if let Some(child) = hit {
                    children[element] = child;
                }
                hit.is_some()
            } else {
                trie.contains_prehashed_width::<K>(key, hashes[k])
            };
            counters.probe(probe.node, probe.sub, hit);
            mask[k] = u8::from(hit);
        };
        match probe.cursor {
            ProbeCursor::Carried { column, width } => {
                let cursor_at = |element: usize| {
                    let parent = parents[element] as usize;
                    pending_cursors[parent * width + column]
                };
                if n >= PREFETCH_WIDTH_FLOOR {
                    for (k, &element) in survivors.iter().enumerate() {
                        colt.prefetch_bucket_with_children::<CHILDREN>(
                            cursor_at(element as usize),
                            hashes[k],
                        );
                    }
                }
                for (k, &element) in survivors.iter().enumerate() {
                    let element = element as usize;
                    // A carried cursor may force a different node. Drop its
                    // borrowed view before preparing the next cursor's probe.
                    let Some(trie) =
                        self.colt_ok(colt.prepare_probe(cursor_at(element), probe.level))
                    else {
                        break;
                    };
                    probe_one(&trie, k, element);
                }
            }
            ProbeCursor::Shared(cursor) => {
                let Some(trie) = self.colt_ok(colt.prepare_probe(cursor, probe.level)) else {
                    return;
                };
                if n >= PREFETCH_WIDTH_FLOOR {
                    trie.prefetch_batch(hashes, CHILDREN);
                }
                for (k, &element) in survivors.iter().enumerate() {
                    probe_one(&trie, k, element as usize);
                }
            }
        }
    }
}
