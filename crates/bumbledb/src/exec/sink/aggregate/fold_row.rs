use crate::exec::sink::{Acc, AggSpec, AggregateSink, FoldOp, GroupState, SinkSpec, word_to_i64};
use std::num::NonZeroU32;

impl AggregateSink {
    /// Fold borrowed inputs; full bindings are needed only for the seen set.
    /// A checked distinctness witness permits an empty dedup input.
    /// The reader borrows routing at each use, leaving the cache in place
    /// while group state changes. Scalar and staged readers ignore routing.
    pub(super) fn fold_row(
        &mut self,
        bindings: &[u64],
        get: impl Fn(usize, &[Option<NonZeroU32>]) -> u64,
    ) {
        // A full dense-index representation flushes before another group
        // could be inserted. This is not an estimated-byte threshold.
        self.maybe_spill_groups();
        if self.error.is_some()
            || self.cardinality_overflow
            || (!self.distinct_bindings()
                && !self.dedup.consider(bindings, &mut self.union_scratch))
        {
            return;
        }

        super::groups::load_group_key(&mut self.key_scratch, &self.group_spans, |slot| {
            get(slot, &self.cached_leaf_words)
        });
        let group_idx = self.probe_group();
        if matches!(self.group_state, GroupState::Folds { .. }) && !self.advance_group(group_idx, 1)
        {
            return;
        }

        match &mut self.group_state {
            GroupState::Pack { slot, claims } => {
                claims[group_idx].push([
                    get(*slot, &self.cached_leaf_words),
                    get(*slot + 1, &self.cached_leaf_words),
                ]);
            }
            GroupState::Folds { accs, n_aggs } => {
                let n_aggs = *n_aggs;
                let mut acc_cursor = 0;
                for find in &self.finds {
                    let SinkSpec::Agg(spec) = find else {
                        continue;
                    };
                    let acc = &mut accs[group_idx * n_aggs + acc_cursor];
                    acc_cursor += 1;
                    match spec {
                        AggSpec::Float { slot, .. } => {
                            let Acc::Float { index, primary } = acc else {
                                unreachable!("float accumulator handle")
                            };
                            if *primary {
                                let value = bumbledb_theory::F64::from_order_key(get(
                                    *slot,
                                    &self.cached_leaf_words,
                                ))
                                .expect("validated canonical F64 binding");
                                if self.float_accs[*index].push(value).is_err() {
                                    self.cardinality_overflow = true;
                                    return;
                                }
                            }
                        }
                        AggSpec::Count => {
                            let Acc::Count(n) = acc else {
                                unreachable!("accumulators are seeded per op");
                            };
                            *n = n.saturating_add(1);
                        }
                        AggSpec::Fold {
                            op, slot, signed, ..
                        } => {
                            let word = get(*slot, &self.cached_leaf_words);
                            match (op, acc) {
                                (FoldOp::Sum, Acc::SumSigned(total)) => {
                                    debug_assert!(*signed);
                                    *total += i128::from(word_to_i64(word));
                                }
                                (FoldOp::Sum, Acc::SumUnsigned(total)) => {
                                    debug_assert!(!*signed);
                                    *total += u128::from(word);
                                }
                                (FoldOp::Min, Acc::Min(best)) => *best = (*best).min(word),
                                (FoldOp::Max, Acc::Max(best)) => *best = (*best).max(word),
                                _ => unreachable!("accumulators are seeded per op"),
                            }
                        }
                    }
                }
            }
        }
    }
}
