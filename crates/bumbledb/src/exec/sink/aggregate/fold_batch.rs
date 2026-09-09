use crate::exec::colt::SuffixRun;
use crate::exec::run::{Bindings, LeafBatch};
use crate::exec::sink::{Acc, AggSpec, AggregateSink, FoldSource, GroupState, SinkSpec};
use std::num::NonZeroU32;

use super::reduce::{Partial, merge};

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
        self.maybe_spill_groups();
        if self.error.is_some() || self.cardinality_overflow {
            return;
        }
        super::groups::load_group_key(&mut self.key_scratch, &self.group_spans, |slot| {
            batch.bindings.get(slot)
        });

        let group_idx = self.probe_group();
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
        let GroupState::Folds { accs, n_aggs } = &mut self.group_state else {
            unreachable!("constant-group fold is the Folds arm");
        };
        let range = group_idx * *n_aggs..(group_idx + 1) * *n_aggs;
        let mut accumulators = accs[range].iter_mut();
        let mut fold_i = 0;
        for find in &self.finds {
            let SinkSpec::Agg(spec) = find else {
                continue;
            };
            let acc = accumulators.next().expect("one accumulator per aggregate");
            match spec {
                AggSpec::Float { slot, .. } => {
                    let Acc::Float { index, primary } = acc else {
                        unreachable!("float accumulator handle")
                    };
                    if !*primary {
                        continue;
                    }
                    let accumulator = &mut self.float_accs[*index];
                    let result = match self.cached_leaf_words[*slot] {
                        None => accumulator.push_repeated(
                            bumbledb_theory::F64::from_order_key(batch.bindings.get(*slot))
                                .expect("validated canonical F64 binding"),
                            count,
                        ),
                        Some(word) => {
                            let word = word.get() as usize - 1;
                            debug_assert!(!survivors.is_empty(), "count-only folds never gather");
                            survivors.iter().try_for_each(|&entry| {
                                accumulator.push(
                                    bumbledb_theory::F64::from_order_key(batch.key(entry, word))
                                        .expect("validated canonical F64 binding"),
                                )
                            })
                        }
                    };
                    if result.is_err() {
                        self.cardinality_overflow = true;
                        return;
                    }
                }
                AggSpec::Count => {
                    let Acc::Count(n) = acc else {
                        unreachable!("accumulators are seeded per op");
                    };
                    *n = n.saturating_add(count);
                }
                AggSpec::Fold { slot, .. } => {
                    let partial = match self.fold_sources[fold_i] {
                        FoldSource::Column(input) => self.fold_inputs[input].partial,
                        FoldSource::Outer => {
                            Partial::seed(*spec).repeated(batch.bindings.get(*slot), count)
                        }
                    };
                    fold_i += 1;
                    merge(acc, partial.output(*spec, count));
                }
            }
        }
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
