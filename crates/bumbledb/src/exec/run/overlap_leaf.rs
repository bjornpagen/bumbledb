//! The leaf overlap enumeration (ruled 2026-07-23; finding 012 — the
//! 40-execution range-accelerator OPEN item, discharged): a leaf Allen
//! residual whose mask is *touching* (⊆ INTERSECTS ∪ MEETS ∪ MET_BY:
//! every admitted configuration shares a point or abuts) and whose one
//! side is an outer-binding constant licenses enumerating, per key
//! group, only the cover positions whose interval pair lies in the
//! residual's window around that constant — the start-sorted max-end
//! index (`interval::overlap`) replaces the `Σ n_k²` all-pairs walk
//! with `~O(log n_k + out)` per outer row.
//!
//! The window is the mask made geometry: the INTERSECTS components are
//! exactly `start < q_end ∧ end > q_start` (the half-open shared-point
//! law), and the two abutment components each relax one bound by one
//! word — MEETS (`end == q_start`) is `end > q_start − 1`, MET_BY
//! (`start == q_end`) is `start < q_end + 1` — so a disconnected
//! composite like `DURING ∪ MEETS` rides the same one-query index with
//! a ±1-widened window instead of declining to the all-pairs classify.
//! No per-component passes, no candidate dedup: the widening admits
//! only the boundary stabbing sets, and the kernel filters them. The
//! word-space ±1 is exact because encoded words are order-faithful
//! (`10-data-model.md`): `end > q_start − 1 ⇔ end ≥ q_start`, with
//! saturation honest at both extremes (`end == 0` and `start == MAX`
//! are unrepresentable for non-empty half-open intervals).
//!
//! Only the *enumeration* changes representation: the yielded batch
//! flows through the same probes, residuals, and sink as the generic
//! iterator, and the driving mask stays data — the uniform classify
//! kernels still filter the candidates, so completeness is the whole
//! correctness obligation: `code ∈ mask ⊆ TOUCHES` implies the pair
//! shares a point (half-open, non-empty by construction — the
//! point-domain law) or abuts at the exact boundary the widening
//! admits. Orientation is normalized to `classify(batch, constant)`
//! via the converse mask (the theory crate's reversal law) — the pure
//! INTERSECTS gate never needed it, the abutment components do.

use super::{Bindings, Colt, Cursor, Executor, Source, ValidatedPlan};
use crate::exec::colt::SuffixRun;
use crate::image::ColumnView;

/// The group-size floor for the index: below it the plain suffix
/// enumeration + batch classify stays (the n² of a tiny group is
/// cheaper than the sort + per-query walk it would amortize).
/// CONSTRAINT: 16 is provisional from the finding's arithmetic (a
/// 16-group's all-pairs classify ≈ the sort the index pays once); the
/// measurement rig is `tests/intervals.rs::overlap_profile` (release,
/// quiet machine, sweep group sizes with this floor lowered to 2) —
/// re-pin this number from that sweep, never by inspection.
pub(super) const OVERLAP_CROSSOVER: u64 = 16;

/// The accelerable mask cover: every configuration that shares a point
/// (INTERSECTS) or abuts (MEETS/MET_BY) — exactly the codes a one-query
/// window over the start-sorted max-end index can bound. BEFORE/AFTER
/// stay out: their windows are unbounded on the axis the index sorts.
fn touches() -> crate::allen::AllenMask {
    use crate::allen::AllenMask;
    AllenMask::INTERSECTS | AllenMask::MEETS | AllenMask::MET_BY
}

impl Executor {
    /// Resolves the leaf call's overlap driver and, when it fires,
    /// fills `self.overlap_hits` with the cover positions inside the
    /// residual's window around the outer constant (start-ordered).
    /// Declines — `false`, the generic iterator runs — for unqualified
    /// residual shapes, non-touching masks (any BEFORE/AFTER
    /// component), sub-crossover groups, and non-scannable cursors
    /// (pinned rows, forced suffixes).
    #[expect(
        clippy::too_many_arguments,
        reason = "the split borrows and execution context are clearer unpacked"
    )]
    pub(super) fn overlap_enumerate(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        cover_occ: usize,
        cover_cursor: Cursor,
        cover_level: usize,
        colt: &Colt,
        bindings: &Bindings,
        allen_sources: &[(Source, Source)],
    ) -> bool {
        // The driver: the first Allen residual pairing a cover-word
        // interval against an outer-constant side under a touching
        // mask, normalized to `classify(batch, constant)` — a
        // `(Slot, Batch)` residual drives under its converse mask (the
        // reversal law), because the abutment components are oriented
        // even though overlap itself is symmetric.
        let driver = allen_sources
            .iter()
            .enumerate()
            .find_map(|(r_idx, (lhs, rhs))| {
                let ((Source::Batch(word), Source::Slot(slot))
                | (Source::Slot(slot), Source::Batch(word))) = (*lhs, *rhs)
                else {
                    return None;
                };
                let mask = self.allen_masks[node_idx][r_idx];
                let mask = if matches!(lhs, Source::Slot(_)) {
                    mask.converse()
                } else {
                    mask
                };
                let touching = mask.bits() & !touches().bits() == 0;
                touching.then(|| {
                    // The mask made geometry: base window `start <
                    // q_end ∧ end > q_start`, each abutment component
                    // relaxing its one bound by one order-faithful
                    // word (module docs walk the equivalences).
                    let meets = mask.bits() & crate::allen::AllenMask::MEETS.bits() != 0;
                    let met_by = mask.bits() & crate::allen::AllenMask::MET_BY.bits() != 0;
                    let q_start = bindings.get(slot);
                    let q_end = bindings.get(slot + 1);
                    (
                        word,
                        if meets {
                            q_start.saturating_sub(1)
                        } else {
                            q_start
                        },
                        if met_by { q_end.saturating_add(1) } else { q_end },
                    )
                })
            });
        let Some((word, q_start, q_end)) = driver else {
            return false;
        };
        if colt.key_count(cover_cursor).magnitude() < OVERLAP_CROSSOVER
            || !colt.suffix_scannable(cover_cursor)
        {
            return false;
        }
        // Interval endpoints are word columns by construction; the
        // check is the honest decline for anything else.
        let (ColumnView::Words(start_words), ColumnView::Words(end_words)) = (
            colt.suffix_column(cover_level, word),
            colt.suffix_column(cover_level, word + 1),
        ) else {
            return false;
        };
        // The cache key: the cover occurrence plus the bound words of
        // its pre-entry trie levels — the exact trie path that minted
        // this cursor, so equal keys mean the same group within the
        // execution (the cache resets per execute).
        self.overlap_key.clear();
        self.overlap_key
            .push(u64::try_from(cover_occ).expect("occurrence ids are small"));
        for level_vars in &plan.occurrences()[cover_occ].trie_schema[..cover_level] {
            for var in level_vars {
                let slot = plan.slot_of(*var);
                for offset in 0..self.width_of(*var) {
                    self.overlap_key.push(bindings.get(slot + offset));
                }
            }
        }
        let dir = self.overlap.get_or_build(&self.overlap_key, |triples| {
            let walked = colt.for_each_suffix_run(cover_cursor, |run| match run {
                SuffixRun::Identity { start, len } => {
                    for position in start..start + len {
                        let position = u32::try_from(position).expect("positions fit u32");
                        triples.push((
                            start_words[position as usize],
                            end_words[position as usize],
                            position,
                        ));
                    }
                }
                SuffixRun::Positions(positions) => {
                    for &position in positions {
                        triples.push((
                            start_words[position as usize],
                            end_words[position as usize],
                            position,
                        ));
                    }
                }
            });
            debug_assert!(walked, "suffix_scannable gated the walk");
        });
        // A group grown between build and query would enumerate stale
        // positions — impossible within an execution (a level forces
        // whole), and tripwired here.
        debug_assert_eq!(
            self.overlap.len_of(dir) as u64,
            colt.key_count(cover_cursor).magnitude(),
            "a group's positions are stable within an execution"
        );
        self.overlap
            .query_into(dir, q_start, q_end, &mut self.overlap_hits);
        true
    }
}

/// Materializes matched positions as one leaf batch — key words per
/// level word plus pinned-row children, the exact `iter_batch` shape.
pub(super) fn overlap_gather(
    colt: &Colt,
    level: usize,
    arity: usize,
    hits: &[u32],
    keys_out: &mut [u64],
    children_out: &mut [Cursor],
) {
    for word in 0..arity {
        match colt.suffix_column(level, word) {
            ColumnView::Words(words) => {
                for (k, &position) in hits.iter().enumerate() {
                    keys_out[k * arity + word] = words[position as usize];
                }
            }
            ColumnView::Bytes(bytes) => {
                for (k, &position) in hits.iter().enumerate() {
                    keys_out[k * arity + word] = u64::from(bytes[position as usize]);
                }
            }
        }
    }
    for (k, &position) in hits.iter().enumerate() {
        children_out[k] = Cursor::Row(position);
    }
}
