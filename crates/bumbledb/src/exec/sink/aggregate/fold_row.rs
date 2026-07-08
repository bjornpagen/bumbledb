use crate::exec::sink::{word_to_i64, Acc, AggregateSink, FindSpec};
use crate::ir::AggOp;

impl AggregateSink {
    /// Folds the full binding currently in `binding_scratch`: dedup
    /// (unless elided), group resolution, accumulator update. The
    /// per-row paths land here — the scratch row is the one
    /// representation.
    pub(super) fn fold_scratch_row(&mut self) {
        // Binding dedup: fold only the first occurrence of each distinct
        // full binding — unless the plan proved distinctness (elision).
        if let Some(seen) = &mut self.seen {
            if !seen.insert(&self.binding_scratch) {
                return;
            }
        }

        for (i, slot) in self.group_slots.iter().enumerate() {
            self.key_scratch[i] = self.binding_scratch[*slot];
        }
        let group_idx = self.probe_group();

        let accs = &mut self.accs[group_idx * self.n_aggs..(group_idx + 1) * self.n_aggs];
        let mut acc_cursor = 0;
        for find in &self.finds {
            let FindSpec::Agg {
                op,
                over_slot,
                signed,
            } = find
            else {
                continue;
            };
            let acc = &mut accs[acc_cursor];
            acc_cursor += 1;
            match (op, acc) {
                (AggOp::Count, Acc::Count(n)) => *n += 1,
                (AggOp::Sum, Acc::SumSigned(total)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Sum has a variable")];
                    debug_assert!(*signed);
                    *total += i128::from(word_to_i64(word));
                }
                (AggOp::Sum, Acc::SumUnsigned(total)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Sum has a variable")];
                    *total += u128::from(word);
                }
                (AggOp::Min, Acc::Min(best)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Min has a variable")];
                    *best = (*best).min(word);
                }
                (AggOp::Max, Acc::Max(best)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Max has a variable")];
                    *best = (*best).max(word);
                }
                _ => unreachable!("accumulators are seeded per op"),
            }
        }
    }
}
