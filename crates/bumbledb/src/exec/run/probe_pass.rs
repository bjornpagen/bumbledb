//! One cross-parent probe pass.

use super::anti_probe::anti_probe_pass;
use super::{
    Bindings, Colt, Counters, Cursor, Executor, Flow, JoinPhase, NodeScratch, PREFETCH_WIDTH_FLOOR,
    PipeTables, Sink, Source, ValidatedPlan, grow_scratch,
};

impl Executor {
    /// One cross-parent probe pass: hashes, prefetch,
    /// probes, and residuals run over `fill` elements drawn from many
    /// pending entries; survivors either append to the child's pending
    /// (middle child — flushed when a full batch accumulates) or run the
    /// last node per parent through `run_node`.
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
        // A poisoned execution's tail flushes (pump's remainder, deeper
        // recursion unwinds) skip the whole pipeline — every survivor
        // would be discarded at the routing loop's own check anyway.
        if self.all_cancelled {
            scratch.parents.clear();
            scratch.element_origins.clear();
            return;
        }
        let n_nodes = plan.nodes().len();
        let slot_count = bindings.slot_count();
        let carried_w = tables.carried[node_idx].len();
        let node = &plan.nodes()[node_idx];
        let cover_occ = usize::from(node.subatoms[cover_sub].occ.0);
        scratch.survivors.clear();
        scratch
            .survivors
            .extend(0..u32::try_from(fill).expect("batch fits u32"));

        // Residuals run BEFORE the sibling probes — the cost-class
        // ordering (docs/architecture/40-execution.md, § inputs from
        // normalization): residual operands read only cover batch words
        // and outer bindings, and sibling probes bind no variables, so
        // the pure-ALU rejection legally precedes the memory-bound
        // hash probes and every probe it kills is a bucket load never
        // issued. Per-parent Slot reads, word offsets via the cover's
        // word bases (width 2 = the pairwise interval compare).
        counters.phase_start(node_idx, JoinPhase::Residual);
        for (residual, lhs_slot, rhs_slot, width) in &self.residual_slots[node_idx] {
            let cover_vars = &node.subatoms[cover_sub].vars;
            let lhs_word = super::word_base(cover_vars, residual.lhs, |v| self.width_of(v));
            let rhs_word = super::word_base(cover_vars, residual.rhs, |v| self.width_of(v));
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
                    residual.op,
                    *width,
                    |offset| value(lhs_word, *lhs_slot, offset),
                    |offset| value(rhs_word, *rhs_slot, offset),
                );
                counters.residual(node_idx, pass);
                scratch.mask[k] = u8::from(pass);
            }
            crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
        }
        // Word residuals: the decomposed interval compositions over
        // pre-offset slot pairs — same placement, same compaction
        // (docs/architecture/20-query-ir.md, § normalization).
        for (residual, lhs_slot, rhs_slot) in &self.word_residual_slots[node_idx] {
            let cover_vars = &node.subatoms[cover_sub].vars;
            let side = |var_word: crate::ir::normalize::VarWord| {
                super::word_base(cover_vars, var_word.var, |v| self.width_of(v))
                    .map(|base| base + var_word.word.offset())
            };
            let (lhs_word, rhs_word) = (side(residual.lhs), side(residual.rhs));
            let n = scratch.survivors.len();
            grow_scratch(&mut scratch.mask, n);
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
        // Allen residuals: gather the four endpoint streams per
        // survivor — read at word-base offsets 0/1, batch key words or
        // the element's parent row — classify the whole batch through
        // the configuration kernel, test the resolved broadcast mask,
        // and compact on the branchless cursor-write (the line-parallel
        // twin of `run_node`'s pass; docs/architecture/40-execution.md,
        // § vectorized execution).
        for (r_idx, (residual, lhs_slot, rhs_slot)) in
            self.allen_residual_slots[node_idx].iter().enumerate()
        {
            let mask = self.allen_masks[node_idx][r_idx];
            let cover_vars = &node.subatoms[cover_sub].vars;
            let lhs_word = super::word_base(cover_vars, residual.lhs, |v| self.width_of(v));
            let rhs_word = super::word_base(cover_vars, residual.rhs, |v| self.width_of(v));
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
                a_starts[k] = value(lhs_word, *lhs_slot, 0);
                a_ends[k] = value(lhs_word, *lhs_slot, 1);
                b_starts[k] = value(rhs_word, *rhs_slot, 0);
                b_ends[k] = value(rhs_word, *rhs_slot, 1);
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
        // Measure residuals: the line-parallel twin of `run_node`'s pass
        // — per-parent Slot reads, subtraction feeding the ordinary word
        // compare. A ray never survives the comparison (its verdict is
        // Ray, not Fails; the Kleene verdict algebra, R6 — the prepared
        // query's ray-probe pass renders it).
        for r_idx in 0..self.duration_residual_slots[node_idx].len() {
            let (residual, interval_slot, scalar_slot) =
                self.duration_residual_slots[node_idx][r_idx];
            let cover_vars = &node.subatoms[cover_sub].vars;
            let interval_word =
                super::word_base(cover_vars, residual.interval, |v| self.width_of(v));
            let scalar_word = super::word_base(cover_vars, residual.scalar, |v| self.width_of(v));
            let n = scratch.survivors.len();
            grow_scratch(&mut scratch.mask, n);
            for k in 0..n {
                let element = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
                let parent = scratch.parents[element] as usize;
                let value = |word: Option<usize>, slot: usize, offset: usize| match word {
                    Some(word) => scratch.entry_keys[element * arity + word + offset],
                    None => scratch.pending_bindings[parent * slot_count + slot + offset],
                };
                let start = value(interval_word, interval_slot, 0);
                let end = value(interval_word, interval_slot, 1);
                let pass = end != u64::MAX
                    && residual
                        .op
                        .compare(&(end - start), &value(scalar_word, scalar_slot, 0));
                counters.residual(node_idx, pass);
                scratch.mask[k] = u8::from(pass);
            }
            crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
        }
        counters.phase_end(node_idx, JoinPhase::Residual);

        // Sibling passes: per-parent Slot reads and per-parent cursors —
        // the pipelined twin of run_node's sibling loop, kept
        // line-parallel (a change here needs its mirror there; the
        // extraction refusal is recorded at that loop's head).
        // Instruction diet (measured): value sources resolve once
        // per (pass, subatom) — never a per-element variable search —
        // loop invariants (carried column, start cursor) hoist, and the
        // inner loops write pre-sized buffers by index (a `Vec::push`'s
        // grow branch blocks LICM and unrolling in exactly these loops).
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
            // One source per key WORD (the SlotWidth layout): an interval
            // variable reads two consecutive batch words or slots.
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
            // One gather loop for every source shape (the
            // single-batch-word specialized twin measured < 2% at
            // family level post-bucket-layout and was deleted).
            {
                let survivors = &scratch.survivors[..n];
                let entry_keys = &scratch.entry_keys[..];
                let parents = &scratch.parents[..];
                let pending_bindings = &scratch.pending_bindings[..];
                let sources = &scratch.sources[sub_idx][..];
                let probe_keys = &mut scratch.probe_keys[..n * sub_arity];
                let hashes = &mut scratch.hashes[..n];
                // The const-arity dispatch (the wordmap's `hash_core`
                // precedent): one predictable branch per pass (the same
                // arm every pass of a given subatom) buys the unrolled,
                // gather-fused hash for the key widths in use; exotic
                // widths keep the dyn loop. Measured (interleaved twin,
                // per-draw arm medians): triangle 1.055x at S scale /
                // 1.043x at M, chain 1.022x, spread 1.021x, skew 1.017x;
                // non-probe families unchanged. The falsifier harness
                // (both arms behind one switch + the ab_hash bin) lives
                // in the stripped commit 564da7c6 — check it out to
                // re-falsify.
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
            let carried = tables.carried_col[node_idx][occ];
            let start_cursor = colts[occ].start();
            // A first-appearance sibling probes the batch-constant start
            // cursor: force its map here, under the Force phase, like
            // the twin — otherwise the prefetch sweep below no-ops on
            // the unforced node and the O(positions) ingest lands inside
            // phase 2's first probe, booked as Probe. Carried cursors
            // stay lazy (per-element; a pinned row never needs a map).
            if carried.is_none() {
                counters.phase_start(node_idx, JoinPhase::Force);
                colts[occ].ensure_forced(start_cursor, s_level);
                counters.phase_end(node_idx, JoinPhase::Force);
            }
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
            grow_scratch(&mut scratch.mask, n);
            // The measured alias-hoist shape: reads
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
        }

        // The pass's cursor-source table, occ-indexed: advanced at this
        // node (the cover's child or a probed sibling's), inherited from
        // the parent (carried column), or the colt's start when never
        // advanced. Resolved once per pass — the membership loops and
        // the routing arm below index it instead of re-searching the
        // subatom list per element (the instruction diet).
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
                match tables.carried_col[node_idx][occ] {
                    Some(col) => super::CursorSrc::Carried(col),
                    None => super::CursorSrc::Const(colt.start()),
                }
            });
        }

        // Membership probes (docs/architecture/40-execution.md, the
        // point-membership scan): scan the occurrence's remaining
        // positions per surviving binding — cursors read through the
        // pass's source table, point words through per-spec resolved
        // sources. They stay AFTER the sibling probes (unlike the ALU
        // residuals above): a probed occurrence's cursor may be this
        // pass's own sibling child, and the position scan is
        // probe-class work.
        if !self.point_probe_slots[node_idx].is_empty() {
            counters.phase_start(node_idx, JoinPhase::Residual);
        }
        for spec in &self.point_probe_slots[node_idx] {
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
            for k in 0..n {
                let element = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
                let parent = scratch.parents[element] as usize;
                scratch.point_checks.clear();
                for &(start_col, end_col, src) in &scratch.point_sources {
                    let point = match src {
                        Source::Batch(base) => scratch.entry_keys[element * arity + base],
                        Source::Slot(slot) => scratch.pending_bindings[parent * slot_count + slot],
                    };
                    scratch.point_checks.push((start_col, end_col, point));
                }
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
                let pass = colts[spec.occ].any_position_matches(cursor, &scratch.point_checks);
                counters.residual(node_idx, pass);
                scratch.mask[k] = u8::from(pass);
            }
            crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
        }
        if !self.point_probe_slots[node_idx].is_empty() {
            counters.phase_end(node_idx, JoinPhase::Residual);
        }

        // Anti-probes: the residual step's sibling (docs/architecture/
        // 40-execution.md, § anti-probe filters) — hits are compacted
        // away on the same cursor-write. Slot reads go through each
        // element's parent row, exactly like the residuals above.
        anti_probe_pass(
            &self.anti_probe_slots[node_idx],
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

        // Survivor routing. Origins: the absorb node mints one
        // fresh origin per routed survivor — the cancellation unit is
        // exactly "one absorb-element's subtree"; deeper nodes inherit.
        // Refuted (2026-07-16, interleaved A/B at b48dcd51): batching
        // this loop — one `load_row` per run of same-parent survivors
        // on the leaf arm, `extend_from_within` duplication instead of
        // the full-row copy on the middle arm — measured NEUTRAL,
        // min-of-5 over 5 cross-process pairs at L2-resident scale S:
        // spread 0.988, skew 0.997, triangle 0.987, chain mean 0.997,
        // busy_scan 0.985; the 10–25% bar decisively unmet, no family
        // robustly worse either. Same-parent survivor runs are too
        // short in these corpora to amortize the run cache — the added
        // per-survivor parent compare offsets the saved copies. That
        // twin armed only THIS loop; run_node's line-parallel mirror
        // kept full copies — arm both (and confront the extraction
        // refusal) before judging the descend bucket untouchable. The
        // W2 gravestone commit carries the full protocol.
        counters.phase_start(node_idx, JoinPhase::Descend);
        let leaf = node_idx + 2 == n_nodes;
        let child_carried = &tables.carried[node_idx + 1];
        let mints_origins = tables.absorb == Some(node_idx);
        // The origin mint space is checked HERE, at mint granularity —
        // one branch per probe pass (this pass mints at most one origin
        // per survivor), never on the per-tuple path. Past 2³² absorb
        // survivors the u32 counter would wrap in release, cancel the
        // WRONG origin, and silently drop valid rows — beyond the scale
        // axiom, but valid input, so it is the typed `Overflow` error
        // (surfaced by `execute`), never a wrap and never a panic. The
        // representation fix — widening origins to u64 — was rejected:
        // origin ids are stored per pending row in hot scratch arrays
        // (`pending_origins`, `element_origins`, the `cancelled`
        // high-water table), and doubling that width is measured bytes
        // on the hot path spent against a beyond-axiom case; the
        // boundary check at mint granularity is the cheaper honest
        // shape.
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
            // The pass's cursor-source table, indexed — resolved once
            // per pass above, never a per-survivor subatom search.
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
                // The leaf's membership probes read their occurrence's
                // advanced cursor too (fully descended by attachment) —
                // assemble it exactly like a leaf subatom's.
                for probe in &leaf_node.point_probes {
                    let occ = usize::from(probe.occ.0);
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
        // Flush downstream at one accumulated batch. Bounded memory: the child
        // holds at most two batches transiently (the 1×batch trigger
        // plus one pass's appends before the next check). The 2×-batch
        // threshold measured 0.0–0.6% once the per-pass overhead was
        // priced at 11–30 ns — reverted to the simpler contract.
        if !leaf && self.scratch[node_idx + 1].pending_len >= self.batch {
            self.pump(tables, plan, node_idx + 1, colts, bindings, sink, counters);
        }
    }
}

/// Phase-1 gather + hash with the key width fixed at K — the
/// probe-pass twin of the wordmap's `hash_core` dispatch
/// (`exec/swar.rs`): the per-word source match unrolls, the `k * K`
/// indexing strength-reduces, and the hash fold fuses with the gather
/// instead of the rolled ~5-cycle serial chain runtime arity leaves.
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
    // per-word bounds checks inside the loop.
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
