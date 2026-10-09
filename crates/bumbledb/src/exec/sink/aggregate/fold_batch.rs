use crate::exec::colt::SuffixRun;
use crate::exec::run::{Bindings, LeafBatch};
use crate::exec::sink::{AggSpec, AggregateSink, SinkSpec};
use std::num::NonZeroU32;

impl AggregateSink {
    pub(super) fn fold_batch_rows(&mut self, batch: &LeafBatch<'_>) {
        if self.distinct_bindings() {
            for &entry in batch.survivors {
                self.fold_row(&[], |slot, leaf_words| match leaf_words[slot] {
                    None => batch.bindings.get(slot),
                    Some(word) => batch.key(entry, word.get() as usize - 1),
                });
            }
            return;
        }

        // Dedup needs the full binding. Keep the outer copy amortized over
        // the batch and borrow this owner while mutating group state.
        let mut row = std::mem::take(&mut self.binding_scratch);
        copy_outer(&self.cached_leaf_words, batch.bindings, &mut row);
        for &entry in batch.survivors {
            for (word, slot) in batch.key_slots.iter().enumerate() {
                row[*slot] = batch.key(entry, word);
            }
            self.fold_row(&row, |slot, _| row[slot]);
        }
        self.binding_scratch = row;
    }

    pub(super) fn fold_batch_dedup_constant_group(&mut self, batch: &LeafBatch<'_>) {
        copy_outer(
            &self.cached_leaf_words,
            batch.bindings,
            &mut self.binding_scratch,
        );

        let key_sourced = self.finds.iter().any(|find| match find {
            SinkSpec::Agg(AggSpec::Fold { slot, .. } | AggSpec::Float { slot, .. }) => {
                self.cached_leaf_words[*slot].is_some()
            }
            _ => false,
        });

        let binding_scratch = &mut self.binding_scratch[..];
        if !key_sourced {
            let mut fresh = 0u64;
            for &entry in batch.survivors {
                for (word, slot) in batch.key_slots.iter().enumerate() {
                    binding_scratch[*slot] = batch.key(entry, word);
                }
                fresh += u64::from(
                    self.dedup
                        .consider(binding_scratch, &mut self.union_scratch),
                );
            }
            if fresh > 0 {
                self.fold_constant_group(batch, fresh, &[]);
            }
            return;
        }
        let mut survivors = std::mem::take(&mut self.dedup_survivors);
        survivors.clear();
        for &entry in batch.survivors {
            for (word, slot) in batch.key_slots.iter().enumerate() {
                binding_scratch[*slot] = batch.key(entry, word);
            }
            if self
                .dedup
                .consider(binding_scratch, &mut self.union_scratch)
            {
                survivors.push(entry);
            }
        }
        if !survivors.is_empty() {
            self.fold_batch_constant_group(batch, &survivors);
        }
        self.dedup_survivors = survivors;
    }

    pub(super) fn fold_batch_constant_group(&mut self, batch: &LeafBatch<'_>, survivors: &[u32]) {
        self.fold_constant_group(batch, survivors.len() as u64, survivors);
    }

    /// Count-only folds may omit survivors; key reductions require them.
    fn fold_constant_group(&mut self, batch: &LeafBatch<'_>, count: u64, survivors: &[u32]) {
        if self.error.is_some() || self.cardinality_overflow {
            return;
        }
        super::groups::load_group_key(&mut self.key_scratch, &self.group_spans, |slot| {
            batch.bindings.get(slot)
        });

        let Some(group_idx) = self.probe_group() else {
            return;
        };
        if !self.advance_group(group_idx, count) {
            return;
        }

        if !self.fold_inputs.is_empty() {
            debug_assert!(!survivors.is_empty(), "count-only folds never gather");
            let run = batch_run(survivors);
            for input in &mut self.fold_inputs {
                input.partial.reset();
                input.partial.fold(batch.keys, batch.arity, input.word, run);
            }
        }
        self.merge_partials(group_idx, count, |slot| batch.bindings.get(slot));
    }
}

fn copy_outer(leaf_words: &[Option<NonZeroU32>], bindings: &Bindings, row: &mut [u64]) {
    for (slot, word) in leaf_words.iter().enumerate() {
        if word.is_none() {
            row[slot] = bindings.get(slot);
        }
    }
}

/// Density describes the whole selection, not just its two endpoints.
/// Resolve it once for every shared column reduction in this batch.
fn batch_run(survivors: &[u32]) -> SuffixRun<'_> {
    let start = survivors[0] as usize;
    if survivors
        .iter()
        .enumerate()
        .all(|(i, &entry)| entry as usize == start + i)
    {
        SuffixRun::Identity {
            start,
            len: survivors.len(),
        }
    } else {
        SuffixRun::Positions(survivors)
    }
}
