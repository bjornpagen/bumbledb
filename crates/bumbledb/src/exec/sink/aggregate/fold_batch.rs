use crate::exec::run::{LeafBatch, LeafSource};
use crate::exec::sink::{Acc, AggregateSink, FoldOp, SinkSpec, word_to_i64};

impl AggregateSink {
    /// The per-row batch arm: outer slots prefilled once, leaf key slots
    /// overwritten per row, each full binding folded through the scratch
    /// (dedup, varying-group, and row-fold — `CountDistinct`/Arg —
    /// regimes). `key_slots` is word-level (an interval variable's pair
    /// appears as two entries), so the scratch fill is layout-correct by
    /// construction.
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

    /// The constant-group fast path: one group probe
    /// per batch (memoized across consecutive batches of the same run),
    /// accumulators staged out of the group row, per-op dispatch outside
    /// the row loop, and the row loops themselves shaped as the
    /// kernelized gather folds.
    /// The dedup-regime batch arm: the seen-set pass
    /// runs per row (semantically required — prepare could not prove the
    /// key stream duplicate-free), collecting first-seen entries; those then
    /// gather-fold through the same constant-group core as the elided
    /// path, group probe hoisted and all.
    pub(super) fn fold_batch_dedup_constant_group(&mut self, batch: &LeafBatch<'_>) {
        // The binding fills as ever: outer slots constant, prefilled once
        // (cached shape); key slots overwritten per row. The dedup key is
        // the full binding — or its head projection under the multi-rule
        // union regime (`dedup_key`).
        // Direct per-row insert — NO hash-ahead pipeline (the
        // pipeline measured a strict loss in this
        // exact shape, including on mixed hit/miss streams, once the
        // window probe landed).
        for &slot in &self.cached_outer_slots {
            self.binding_scratch[slot] = batch.bindings.get(slot);
        }
        let mut survivors = std::mem::take(&mut self.dedup_survivors);
        survivors.clear();
        let seen = self.seen.as_mut().expect("dedup regime");
        // Alias-hoisted: `binding_scratch` reborrowed
        // once — the survivor pushes and seen-set writes can no longer
        // alias its header.
        let binding_scratch = &mut self.binding_scratch[..];
        let union_spans = self.union_spans.as_deref();
        let union_scratch = &mut self.union_scratch;
        for &entry in batch.survivors {
            for (word, slot) in batch.key_slots.iter().enumerate() {
                binding_scratch[*slot] = batch.key(entry, word);
            }
            let key = super::fold_row::dedup_key(union_spans, union_scratch, binding_scratch);
            if seen.insert(key) {
                survivors.push(entry);
            }
        }
        if !survivors.is_empty() {
            self.fold_batch_constant_group(batch, &survivors);
        }
        self.dedup_survivors = survivors;
    }

    pub(super) fn fold_batch_constant_group(&mut self, batch: &LeafBatch<'_>, survivors: &[u32]) {
        super::groups::load_group_key(&mut self.key_scratch, &self.group_spans, |slot| {
            batch.bindings.get(slot)
        });
        // Once per batch (the group-run memo that
        // skipped this probe measured < 2% under the const-arity map
        // and was deleted — the probe IS the fast path now).
        let group_idx = self.probe_group();

        let range = group_idx * self.n_aggs..(group_idx + 1) * self.n_aggs;
        self.acc_scratch.clear();
        self.acc_scratch
            .extend_from_slice(&self.accs[range.clone()]);
        let count = survivors.len() as u64;
        let mut cursor = 0;
        for find in &self.finds {
            let SinkSpec::Agg {
                op,
                over_slot,
                signed,
                ..
            } = find
            else {
                continue;
            };
            let acc = &mut self.acc_scratch[cursor];
            cursor += 1;
            match (op, acc) {
                // Count is arithmetic, never a loop.
                (FoldOp::Count, Acc::Count(n)) => *n += count,
                (FoldOp::Sum, Acc::SumSigned(total)) => {
                    debug_assert!(*signed);
                    let slot = over_slot.expect("validated: Sum has a variable");
                    *total += match batch.source_of(slot) {
                        // Constant over the batch: value × count, i128 —
                        // identical to `count` additions.
                        LeafSource::Outer => {
                            i128::from(word_to_i64(batch.bindings.get(slot))) * i128::from(count)
                        }
                        LeafSource::Key(word) => {
                            gather_sum_signed(batch.keys, batch.arity, word, survivors)
                        }
                    };
                }
                (FoldOp::Sum, Acc::SumUnsigned(total)) => {
                    let slot = over_slot.expect("validated: Sum has a variable");
                    *total += match batch.source_of(slot) {
                        LeafSource::Outer => {
                            u128::from(batch.bindings.get(slot)) * u128::from(count)
                        }
                        LeafSource::Key(word) => {
                            gather_sum_unsigned(batch.keys, batch.arity, word, survivors)
                        }
                    };
                }
                (FoldOp::Min, Acc::Min(best)) => {
                    let slot = over_slot.expect("validated: Min has a variable");
                    let word = match batch.source_of(slot) {
                        LeafSource::Outer => batch.bindings.get(slot),
                        LeafSource::Key(word) => {
                            gather_min(batch.keys, batch.arity, word, survivors)
                        }
                    };
                    *best = (*best).min(word);
                }
                (FoldOp::Max, Acc::Max(best)) => {
                    let slot = over_slot.expect("validated: Max has a variable");
                    let word = match batch.source_of(slot) {
                        LeafSource::Outer => batch.bindings.get(slot),
                        LeafSource::Key(word) => {
                            gather_max(batch.keys, batch.arity, word, survivors)
                        }
                    };
                    *best = (*best).max(word);
                }
                (FoldOp::CountDistinct, _) => {
                    unreachable!("row-fold ops take the per-row path (emit_batch gates)")
                }
                _ => unreachable!("accumulators are seeded per op"),
            }
        }
        self.accs[range].copy_from_slice(&self.acc_scratch);
    }
}

/// The batch gather folds, kerneled: dense survivor
/// runs (ascending with no gaps — the common all-survived batch) take
/// the contiguous strided kernels with zero index loads; everything
/// else takes the `_idx` gather kernels. All take non-empty survivor
/// lists (the executor skips empty batches).
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
