use crate::exec::sink::{Acc, AggSpec, AggregateSink, FoldOp, GroupState, SinkSpec, word_to_i64};

impl AggregateSink {
    pub(super) fn fold_scratch_row(&mut self) {
        // The group-state pressure check runs BEFORE the row folds: past
        // the allowance the RAM partition flushes into the scratch tier
        // (merging per group), and this row starts a fresh partition.
        self.maybe_spill_groups();
        if self.error.is_some()
            || self.cardinality_overflow
            || !self
                .dedup
                .consider(&self.binding_scratch, &mut self.union_scratch)
        {
            return;
        }

        super::groups::load_group_key(&mut self.key_scratch, &self.group_spans, |slot| {
            self.binding_scratch[slot]
        });
        let group_idx = self.probe_group();
        if matches!(self.group_state, GroupState::Folds { .. }) && !self.advance_group(group_idx, 1)
        {
            return;
        }

        match &mut self.group_state {
            GroupState::Pack { slot, claims } => {
                claims[group_idx]
                    .push([self.binding_scratch[*slot], self.binding_scratch[*slot + 1]]);
                self.pack_bytes += 16;
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
                                let value = bumbledb_theory::F64::from_order_key(
                                    self.binding_scratch[*slot],
                                )
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
                            let word = self.binding_scratch[*slot];
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
