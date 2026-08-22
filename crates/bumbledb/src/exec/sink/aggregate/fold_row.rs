use crate::exec::sink::{Acc, AggSpec, AggregateSink, FoldOp, GroupState, SinkSpec, word_to_i64};

impl AggregateSink {

    pub(super) fn fold_scratch_row(&mut self) {

        if !self
            .dedup
            .consider(&self.binding_scratch, &mut self.union_scratch)
        {
            return;
        }

        super::groups::load_group_key(&mut self.key_scratch, &self.group_spans, |slot| {
            self.binding_scratch[slot]
        });
        let group_idx = self.probe_group();

        match &mut self.group_state {
            GroupState::Pack { slot, claims } => {
                claims[group_idx]
                    .push([self.binding_scratch[*slot], self.binding_scratch[*slot + 1]]);
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
                        AggSpec::Count => {
                            let Acc::Count(n) = acc else {
                                unreachable!("accumulators are seeded per op");
                            };
                            *n += 1;
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
