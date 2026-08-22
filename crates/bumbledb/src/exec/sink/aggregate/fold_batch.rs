use crate::exec::run::{LeafBatch, LeafSource};
use crate::exec::sink::{Acc, AggSpec, AggregateSink, FoldOp, GroupState, SinkSpec, word_to_i64};

impl AggregateSink {

    pub(super) fn fold_batch_rows(&mut self, batch: &LeafBatch<'_>) {
        for &slot in &self.cached_outer_slots {
            self.binding_scratch[slot] = batch.bindings.get(slot);
        }
        for &entry in batch.survivors {
            for (word, slot) in batch.key_slots.iter().enumerate() {
                self.binding_scratch[*slot] = batch.key(entry, word);
            }
            self.fold_scratch_row();
        }
    }

    pub(super) fn fold_batch_dedup_constant_group(&mut self, batch: &LeafBatch<'_>) {

        for &slot in &self.cached_outer_slots {
            self.binding_scratch[slot] = batch.bindings.get(slot);
        }

        let key_sourced = self.finds.iter().any(|find| match find {
            SinkSpec::Agg(AggSpec::Fold { slot, .. }) => {
                matches!(batch.source_of(*slot), LeafSource::Key(_))
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

    /// every `Key` arm below asserts it non-empty before gathering.
    fn fold_constant_group(&mut self, batch: &LeafBatch<'_>, count: u64, survivors: &[u32]) {
        super::groups::load_group_key(&mut self.key_scratch, &self.group_spans, |slot| {
            batch.bindings.get(slot)
        });

        let group_idx = self.probe_group();

        let n_aggs = match &self.group_state {
            GroupState::Folds { accs, n_aggs } => {
                let range = group_idx * *n_aggs..(group_idx + 1) * *n_aggs;
                self.acc_scratch.clear();
                self.acc_scratch.extend_from_slice(&accs[range]);
                *n_aggs
            }
            GroupState::Pack { .. } => unreachable!("constant-group fold is the Folds arm"),
        };
        let range = group_idx * n_aggs..(group_idx + 1) * n_aggs;
        let mut cursor = 0;
        for find in &self.finds {
            let SinkSpec::Agg(spec) = find else {
                continue;
            };
            let acc = &mut self.acc_scratch[cursor];
            cursor += 1;
            match spec {
                AggSpec::Count => {
                    let Acc::Count(n) = acc else {
                        unreachable!("accumulators are seeded per op");
                    };
                    *n += count;
                }
                AggSpec::Fold {
                    op, slot, signed, ..
                } => match (op, acc) {
                    (FoldOp::Sum, Acc::SumSigned(total)) => {
                        debug_assert!(*signed);
                        *total += match batch.source_of(*slot) {
                            LeafSource::Outer => {
                                i128::from(word_to_i64(batch.bindings.get(*slot)))
                                    * i128::from(count)
                            }
                            LeafSource::Key(word) => {
                                debug_assert!(
                                    !survivors.is_empty(),
                                    "count-only folds never gather"
                                );
                                gather_sum_signed(batch.keys, batch.arity, word, survivors)
                            }
                        };
                    }
                    (FoldOp::Sum, Acc::SumUnsigned(total)) => {
                        *total += match batch.source_of(*slot) {
                            LeafSource::Outer => {
                                u128::from(batch.bindings.get(*slot)) * u128::from(count)
                            }
                            LeafSource::Key(word) => {
                                debug_assert!(
                                    !survivors.is_empty(),
                                    "count-only folds never gather"
                                );
                                gather_sum_unsigned(batch.keys, batch.arity, word, survivors)
                            }
                        };
                    }
                    (FoldOp::Min, Acc::Min(best)) => {
                        let word = match batch.source_of(*slot) {
                            LeafSource::Outer => batch.bindings.get(*slot),
                            LeafSource::Key(word) => {
                                debug_assert!(
                                    !survivors.is_empty(),
                                    "count-only folds never gather"
                                );
                                gather_min(batch.keys, batch.arity, word, survivors)
                            }
                        };
                        *best = (*best).min(word);
                    }
                    (FoldOp::Max, Acc::Max(best)) => {
                        let word = match batch.source_of(*slot) {
                            LeafSource::Outer => batch.bindings.get(*slot),
                            LeafSource::Key(word) => {
                                debug_assert!(
                                    !survivors.is_empty(),
                                    "count-only folds never gather"
                                );
                                gather_max(batch.keys, batch.arity, word, survivors)
                            }
                        };
                        *best = (*best).max(word);
                    }
                    _ => unreachable!("accumulators are seeded per op"),
                },
            }
        }
        let GroupState::Folds { accs, .. } = &mut self.group_state else {
            unreachable!("constant-group fold is the Folds arm");
        };
        accs[range].copy_from_slice(&self.acc_scratch);
    }
}

fn dense_run(survivors: &[u32]) -> Option<u32> {
    let (first, last) = (survivors[0], survivors[survivors.len() - 1]);
    (last as usize - first as usize + 1 == survivors.len()).then_some(first)
}

fn gather_sum_signed(keys: &[u64], arity: usize, word: usize, survivors: &[u32]) -> i128 {
    match dense_run(survivors) {
        Some(first) => crate::exec::kernel::fold_sum_biased_i64(
            keys,
            arity,
            first as usize * arity + word,
            survivors.len(),
        ),
        None => crate::exec::kernel::fold_sum_biased_i64_idx(keys, arity, word, survivors),
    }
}

fn gather_sum_unsigned(keys: &[u64], arity: usize, word: usize, survivors: &[u32]) -> u128 {
    match dense_run(survivors) {
        Some(first) => crate::exec::kernel::fold_sum_u64(
            keys,
            arity,
            first as usize * arity + word,
            survivors.len(),
        ),
        None => crate::exec::kernel::fold_sum_u64_idx(keys, arity, word, survivors),
    }
}

fn gather_min(keys: &[u64], arity: usize, word: usize, survivors: &[u32]) -> u64 {
    gather_min_max(keys, arity, word, survivors).0
}

fn gather_max(keys: &[u64], arity: usize, word: usize, survivors: &[u32]) -> u64 {
    gather_min_max(keys, arity, word, survivors).1
}

fn gather_min_max(keys: &[u64], arity: usize, word: usize, survivors: &[u32]) -> (u64, u64) {
    match dense_run(survivors) {
        Some(first) => crate::exec::kernel::fold_min_max_u64(
            keys,
            arity,
            first as usize * arity + word,
            survivors.len(),
        ),
        None => crate::exec::kernel::fold_min_max_u64_idx(keys, arity, word, survivors),
    }
}
