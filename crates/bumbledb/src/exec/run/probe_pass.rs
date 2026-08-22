//! One cross-parent probe pass.

use super::anti_probe::anti_probe_pass;
use super::{
    Bindings, Colt, Counters, Cursor, Executor, Flow, JoinPhase, NodeScratch, PREFETCH_WIDTH_FLOOR,
    PipeTables, Sink, Source, ValidatedPlan, grow_scratch,
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
        if !matches!(self.drive_state, super::DriveState::Running) {
            scratch.parents.clear();
            scratch.element_origins.clear();
            return;
        }
        let n_nodes = plan.nodes().len();
        let slot_count = bindings.slot_count();
        let carried_w = tables.carried[node_idx].len();
        let node = &plan.nodes()[node_idx];
        let cover_occ = usize::from(node.subatoms[cover_sub].occ.0);

        counters.phase_start(node_idx, JoinPhase::Gather);
        scratch.survivors.clear();
        scratch
            .survivors
            .extend(0..u32::try_from(fill).expect("batch fits u32"));
        counters.phase_end(node_idx, JoinPhase::Gather);

        // Residuals run BEFORE the sibling probes — the cost-class

        counters.phase_start(node_idx, JoinPhase::Residual);
        for spec in &self.precompute[node_idx].residual_slots {
            let cover_vars = &node.subatoms[cover_sub].vars;
            let lhs_word = super::word_base(cover_vars, spec.lhs, |v| self.width_of(v));
            let rhs_word = super::word_base(cover_vars, spec.rhs, |v| self.width_of(v));
            let n = scratch.survivors.len();
            grow_scratch(&mut scratch.mask, n);
            for k in 0..n {
                let element = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
                let parent = scratch.parents[element] as usize;
                let value = |word: Option<usize>, slot: usize, offset: usize| match word {
                    Some(word) => scratch.entry_keys[element * arity + word + offset],
                    None => scratch.pending_bindings[parent * slot_count + slot + offset],
                };
                let pass = super::compare_wide(
                    spec.op,
                    spec.width,
                    |offset| value(lhs_word, spec.lhs_slot, offset),
                    |offset| value(rhs_word, spec.rhs_slot, offset),
                );
                counters.residual(node_idx, pass);
                scratch.mask[k] = u8::from(pass);
            }
            crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
        }

        for spec in &self.precompute[node_idx].word_residual_slots {
            let cover_vars = &node.subatoms[cover_sub].vars;
            let side = |addr: crate::image::view::OperandAddr| {
                super::word_base(cover_vars, addr.var(), |v| self.width_of(v))
                    .map(|base| base + addr.offset())
            };
            let (lhs_word, rhs_word) = (side(spec.left), side(spec.right));
            let n = scratch.survivors.len();
            grow_scratch(&mut scratch.mask, n);
            for k in 0..n {
                let element = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
                let parent = scratch.parents[element] as usize;
                let value = |word: Option<usize>, slot: usize| match word {
                    Some(word) => scratch.entry_keys[element * arity + word],
                    None => scratch.pending_bindings[parent * slot_count + slot],
                };
                let pass = spec.op.compare(
                    &value(lhs_word, spec.lhs_slot),
                    &value(rhs_word, spec.rhs_slot),
                );
                counters.residual(node_idx, pass);
                scratch.mask[k] = u8::from(pass);
            }
            crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
        }

        for (r_idx, spec) in self.precompute[node_idx]
            .allen_residual_slots
            .iter()
            .enumerate()
        {
            let mask = self.precompute[node_idx].allen_masks[r_idx];
            let cover_vars = &node.subatoms[cover_sub].vars;
            let lhs_word = super::word_base(cover_vars, spec.lhs, |v| self.width_of(v));
            let rhs_word = super::word_base(cover_vars, spec.rhs, |v| self.width_of(v));
            let n = scratch.survivors.len();
            grow_scratch(&mut scratch.allen_gather, 4 * n);
            let (a_starts, rest) = scratch.allen_gather[..4 * n].split_at_mut(n);
            let (a_ends, rest) = rest.split_at_mut(n);
            let (b_starts, b_ends) = rest.split_at_mut(n);
            for k in 0..n {
                let element = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
                let parent = scratch.parents[element] as usize;
                let value = |word: Option<usize>, slot: usize, offset: usize| match word {
                    Some(word) => scratch.entry_keys[element * arity + word + offset],
                    None => scratch.pending_bindings[parent * slot_count + slot + offset],
                };
                a_starts[k] = value(lhs_word, spec.lhs_slot, 0);
                a_ends[k] = value(lhs_word, spec.lhs_slot, 1);
                b_starts[k] = value(rhs_word, spec.rhs_slot, 0);
                b_ends[k] = value(rhs_word, spec.rhs_slot, 1);
            }
            crate::exec::kernel::allen_code_batch(
                a_starts,
                a_ends,
                b_starts,
                b_ends,
                &mut scratch.allen_codes,
            );
            crate::exec::kernel::allen_filter_batch(&scratch.allen_codes, mask, &mut scratch.mask);
            for &keep in &scratch.mask[..n] {
                counters.residual(node_idx, keep != 0);
            }
            crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
        }
        counters.phase_end(node_idx, JoinPhase::Residual);

        // extraction refusal is recorded at that loop's head).

        for sub_idx in 0..node.subatoms.len() {
            if sub_idx == cover_sub || scratch.survivors.is_empty() {
                continue;
            }
            let subatom = &node.subatoms[sub_idx];
            let sub_arity = self.slot_map[node_idx][sub_idx].len();
            let occ = usize::from(subatom.occ.0);
            let s_level = tables.entry_level[node_idx][occ];
            let cover_vars = &node.subatoms[cover_sub].vars;
            counters.phase_start(node_idx, JoinPhase::Hash);

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
            let n = scratch.survivors.len();
            grow_scratch(&mut scratch.hashes, n);

            {
                let survivors = &scratch.survivors[..n];
                let entry_keys = &scratch.entry_keys[..];
                let parents = &scratch.parents[..];
                let pending_bindings = &scratch.pending_bindings[..];
                let sources = &scratch.sources[sub_idx][..];
                let probe_keys = &mut scratch.probe_keys[..n * sub_arity];
                let hashes = &mut scratch.hashes[..n];

                match sub_arity {
                    1 => gather_hash_core::<1, C>(
                        survivors,
                        parents,
                        entry_keys,
                        pending_bindings,
                        sources,
                        arity,
                        slot_count,
                        probe_keys,
                        hashes,
                        node_idx,
                        sub_idx,
                        counters,
                    ),
                    2 => gather_hash_core::<2, C>(
                        survivors,
                        parents,
                        entry_keys,
                        pending_bindings,
                        sources,
                        arity,
                        slot_count,
                        probe_keys,
                        hashes,
                        node_idx,
                        sub_idx,
                        counters,
                    ),
                    3 => gather_hash_core::<3, C>(
                        survivors,
                        parents,
                        entry_keys,
                        pending_bindings,
                        sources,
                        arity,
                        slot_count,
                        probe_keys,
                        hashes,
                        node_idx,
                        sub_idx,
                        counters,
                    ),
                    4 => gather_hash_core::<4, C>(
                        survivors,
                        parents,
                        entry_keys,
                        pending_bindings,
                        sources,
                        arity,
                        slot_count,
                        probe_keys,
                        hashes,
                        node_idx,
                        sub_idx,
                        counters,
                    ),
                    _ => {
                        for (k, &e) in survivors.iter().enumerate() {
                            let element = usize::try_from(e).expect("batch fits usize");
                            let parent = parents[element] as usize;
                            for i in 0..sub_arity {
                                probe_keys[k * sub_arity + i] = match sources[i] {
                                    Source::Batch(word) => entry_keys[element * arity + word],
                                    Source::Slot(slot) => {
                                        pending_bindings[parent * slot_count + slot]
                                    }
                                };
                            }
                            counters.probe_hash(node_idx, sub_idx);
                            hashes[k] = crate::exec::colt::hash_key(
                                &probe_keys[k * sub_arity..(k + 1) * sub_arity],
                            );
                        }
                    }
                }
            }
            counters.phase_end(node_idx, JoinPhase::Hash);
            let carried = tables.carried_index(node_idx, occ);
            let start_cursor = colts[occ].start();

            if carried.is_none() {
                counters.phase_start(node_idx, JoinPhase::Force);
                colts[occ].ensure_forced(start_cursor, s_level);
                counters.phase_end(node_idx, JoinPhase::Force);
            }

            if scratch.survivors.len() >= PREFETCH_WIDTH_FLOOR {
                crate::obs::event(
                    crate::obs::names::PREFETCH_PASS,
                    crate::obs::TraceArgs::Pair(
                        scratch.survivors.len() as u64,
                        colts[occ].probe_footprint_bytes() as u64,
                    ),
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
            grow_scratch(&mut scratch.mask, n);

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
        }

        counters.phase_start(node_idx, JoinPhase::Gather);
        scratch.cursor_srcs.clear();
        for (occ, colt) in colts.iter().enumerate() {
            scratch.cursor_srcs.push(if occ == cover_occ {
                super::CursorSrc::Cover
            } else if let Some(sub_idx) = node
                .subatoms
                .iter()
                .position(|sub| usize::from(sub.occ.0) == occ)
            {
                debug_assert_ne!(sub_idx, cover_sub, "distinct occs per node");
                super::CursorSrc::Sibling(sub_idx)
            } else {
                match tables.carried_index(node_idx, occ) {
                    Some(col) => super::CursorSrc::Carried(col),
                    None => super::CursorSrc::Const(colt.start()),
                }
            });
        }
        counters.phase_end(node_idx, JoinPhase::Gather);

        // sources. They stay AFTER the sibling probes (unlike the ALU

        if !self.precompute[node_idx].point_probes.is_empty() {
            counters.phase_start(node_idx, JoinPhase::Residual);
        }
        for spec in &self.precompute[node_idx].point_probes {
            let cover_vars = &node.subatoms[cover_sub].vars;
            scratch.point_sources.clear();
            for (start_col, end_col, var, slot) in &spec.parts {
                let src = super::word_base(cover_vars, *var, |v| self.width_of(v))
                    .map_or(Source::Slot(*slot), Source::Batch);
                scratch.point_sources.push((*start_col, *end_col, src));
            }
            let cursor_src = scratch.cursor_srcs[spec.occ];
            let n = scratch.survivors.len();
            grow_scratch(&mut scratch.mask, n);

            scratch.point_rows.clear();
            scratch.point_row_ks.clear();
            for k in 0..n {
                let element = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
                let parent = scratch.parents[element] as usize;
                let cursor = match cursor_src {
                    super::CursorSrc::Cover => scratch.children[element],
                    super::CursorSrc::Sibling(sub_idx) => {
                        scratch.sibling_children[sub_idx][element]
                    }
                    super::CursorSrc::Carried(col) => {
                        scratch.pending_cursors[parent * carried_w + col]
                    }
                    super::CursorSrc::Const(start) => start,
                };
                if let Cursor::Row(position) = cursor {
                    scratch.point_rows.push(position);
                    scratch
                        .point_row_ks
                        .push(u32::try_from(k).expect("batch fits u32"));
                    scratch.mask[k] = 1;
                    continue;
                }
                scratch.point_checks.clear();
                for &(start_col, end_col, src) in &scratch.point_sources {
                    let point = match src {
                        Source::Batch(base) => scratch.entry_keys[element * arity + base],
                        Source::Slot(slot) => scratch.pending_bindings[parent * slot_count + slot],
                    };
                    scratch.point_checks.push((start_col, end_col, point));
                }
                scratch.mask[k] =
                    u8::from(colts[spec.occ].any_position_matches(cursor, &scratch.point_checks));
            }

            let m = scratch.point_rows.len();
            if m > 0 {
                grow_scratch(&mut scratch.allen_gather, 2 * m);
                let (starts, ends) = scratch.allen_gather[..2 * m].split_at_mut(m);
                for &(start_col, end_col, src) in &scratch.point_sources {
                    colts[spec.occ].gather_interval_pair(
                        start_col,
                        end_col,
                        &scratch.point_rows,
                        starts,
                        ends,
                    );
                    for j in 0..m {
                        let k = scratch.point_row_ks[j] as usize;
                        let element =
                            usize::try_from(scratch.survivors[k]).expect("batch fits usize");
                        let parent = scratch.parents[element] as usize;
                        let point = match src {
                            Source::Batch(base) => scratch.entry_keys[element * arity + base],
                            Source::Slot(slot) => {
                                scratch.pending_bindings[parent * slot_count + slot]
                            }
                        };
                        scratch.mask[k] &= u8::from(starts[j] <= point) & u8::from(point < ends[j]);
                    }
                }
            }
            for &keep in &scratch.mask[..n] {
                counters.residual(node_idx, keep != 0);
            }
            crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
        }
        if !self.precompute[node_idx].point_probes.is_empty() {
            counters.phase_end(node_idx, JoinPhase::Residual);
        }

        anti_probe_pass(
            &self.precompute[node_idx].anti_probes,
            node_idx,
            &node.subatoms[cover_sub].vars,
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
            |element, slot| {
                let parent = scratch.parents[element] as usize;
                scratch.pending_bindings[parent * slot_count + slot]
            },
            counters,
        );

        let leaf = node_idx + 2 == n_nodes;
        let child_carried = &tables.carried[node_idx + 1];
        let mints_origins = tables.absorb == super::SkipAbsorb::Node(node_idx);

        // WRONG origin, and silently drop valid rows — beyond the scale

        if mints_origins
            && self
                .next_origin
                .checked_add(u32::try_from(scratch.survivors.len()).expect("batch fits u32"))
                .is_none()
        {
            self.poison(super::Poison::OriginOverflow);
            scratch.parents.clear();
            scratch.element_origins.clear();
            return;
        }
        // The window opens AFTER the poison return above: every

        // invariant), so the cold path may not return out of an open

        counters.phase_start(node_idx, JoinPhase::Descend);

        // above it must never match a minted id. Resolved once per pass,

        let below_absorb = matches!(tables.absorb, super::SkipAbsorb::Node(a) if node_idx > a);
        for k in 0..scratch.survivors.len() {
            if !matches!(self.drive_state, super::DriveState::Running) {
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
            if below_absorb && self.origin_cancelled(origin) {
                continue;
            }

            let assemble = |occ: usize| -> Cursor {
                match scratch.cursor_srcs[occ] {
                    super::CursorSrc::Cover => scratch.children[element],
                    super::CursorSrc::Sibling(sub_idx) => {
                        scratch.sibling_children[sub_idx][element]
                    }
                    super::CursorSrc::Carried(col) => {
                        scratch.pending_cursors[parent * carried_w + col]
                    }
                    super::CursorSrc::Const(start) => start,
                }
            };
            if leaf {
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

                for probe in &leaf_node.point_probes {
                    let occ = usize::from(probe.occ.0);
                    self.cursors[occ] = (assemble(occ), tables.entry_level[node_idx + 1][occ]);
                }
                let flow = self.run_node(plan, node_idx + 1, colts, bindings, sink, counters);
                if flow == Flow::SkipSuffix {
                    counters.skip(node_idx);
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
                let child = &mut self.scratch[node_idx + 1];
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
        counters.phase_end(node_idx, JoinPhase::Descend);
        scratch.parents.clear();
        scratch.element_origins.clear();

        // plus one pass's appends before the next check). The 2×-batch

        if !leaf && self.scratch[node_idx + 1].pending_len >= self.batch {
            self.pump(tables, plan, node_idx + 1, colts, bindings, sink, counters);
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the split borrows and execution context are clearer unpacked"
)]
#[expect(
    clippy::inline_always,
    reason = "a monomorphized pure-ALU leaf of the probe hot loop — the \
              swar module's contract (its `bl` would be the cost the \
              dispatch exists to remove)"
)]
#[inline(always)]
fn gather_hash_core<const K: usize, C: Counters>(
    survivors: &[u32],
    parents: &[u32],
    entry_keys: &[u64],
    pending_bindings: &[u64],
    sources: &[Source],
    arity: usize,
    slot_count: usize,
    probe_keys: &mut [u64],
    hashes: &mut [u64],
    node_idx: usize,
    sub_idx: usize,
    counters: &mut C,
) {
    // The width is a dispatch invariant; the array view kills the

    let sources: &[Source; K] = sources.try_into().unwrap_or_else(|_| {
        panic!(
            "hash dispatch width K={K} does not match sources.len()={}",
            sources.len()
        )
    });
    for (k, &e) in survivors.iter().enumerate() {
        let element = usize::try_from(e).expect("batch fits usize");
        let parent = parents[element] as usize;
        let mut key = [0_u64; K];
        for (i, word) in key.iter_mut().enumerate() {
            *word = match sources[i] {
                Source::Batch(word) => entry_keys[element * arity + word],
                Source::Slot(slot) => pending_bindings[parent * slot_count + slot],
            };
        }
        probe_keys[k * K..(k + 1) * K].copy_from_slice(&key);
        counters.probe_hash(node_idx, sub_idx);
        hashes[k] = crate::exec::colt::hash_key_core::<K>(&key);
    }
}
