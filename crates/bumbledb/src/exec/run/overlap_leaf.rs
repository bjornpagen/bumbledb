//! The leaf overlap enumeration (ruled 2026-07-23; finding 012 — the
//! 40-execution range-accelerator OPEN item, discharged): a leaf Allen
//! residual whose mask is *touching* (⊆ INTERSECTS ∪ MEETS ∪ `MET_BY`:
//! every admitted configuration shares a point or abuts) and whose one
//! side is an outer-binding constant licenses enumerating, per key
//! group, only the cover positions whose interval pair lies in the
use super::{Bindings, Colt, Cursor, Executor, Source, ValidatedPlan};
use crate::exec::colt::SuffixRun;
use crate::image::ColumnView;
use crate::interval::overlap::Probe;

pub(super) const OVERLAP_CROSSOVER: u64 = 16;

/// BEFORE/AFTER stay out: their windows are unbounded on the axis the index
/// sorts.
fn touches() -> crate::allen::AllenMask {
    use crate::allen::AllenMask;
    AllenMask::INTERSECTS | AllenMask::MEETS | AllenMask::MET_BY
}

impl Executor {
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
        let mut driver: Option<(usize, u64, u64)> = None;
        for (r_idx, (lhs, rhs)) in allen_sources.iter().enumerate() {
            let ((Source::Batch(word), Source::Slot(slot))
            | (Source::Slot(slot), Source::Batch(word))) = (*lhs, *rhs)
            else {
                continue;
            };
            let mask = self.precompute[node_idx].allen_masks[r_idx];
            let mask = if matches!(lhs, Source::Slot(_)) {
                mask.converse()
            } else {
                mask
            };
            if mask.bits() & !touches().bits() != 0 {
                continue;
            }

            let meets = mask.bits() & crate::allen::AllenMask::MEETS.bits() != 0;
            let met_by = mask.bits() & crate::allen::AllenMask::MET_BY.bits() != 0;
            let q_start = bindings.get(slot);
            let q_start = if meets {
                q_start.saturating_sub(1)
            } else {
                q_start
            };
            let q_end = bindings.get(slot + 1);
            let q_end = if met_by {
                q_end.saturating_add(1)
            } else {
                q_end
            };
            match &mut driver {
                None => driver = Some((word, q_start, q_end)),
                Some((anchor, lo, hi)) if *anchor == word => {
                    *lo = (*lo).max(q_start);
                    *hi = (*hi).min(q_end);
                }
                Some(_) => {}
            }
        }
        let Some((word, q_start, q_end)) = driver else {
            return false;
        };
        if colt.key_count(cover_cursor).magnitude() < OVERLAP_CROSSOVER
            || !colt.suffix_scannable(cover_cursor)
        {
            return false;
        }

        let (ColumnView::Words(start_words), ColumnView::Words(end_words)) = (
            colt.suffix_column(cover_level, word),
            colt.suffix_column(cover_level, word + 1),
        ) else {
            return false;
        };

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

        let Probe::Ready(dir) = self.overlap.probe(&self.overlap_key, |triples| {
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
        }) else {
            return false;
        };

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
