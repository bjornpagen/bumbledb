use crate::exec::sink::{Acc, AggSpec, AggregateSink, FoldOp, SinkSpec, measure, word_to_i64};

impl AggregateSink {
    /// Folds the full binding currently in `binding_scratch`: the
    /// measure words first (the derived-slot parse's one computation
    /// site — a ray poisons the sink and the row is dropped), then dedup
    /// (unless elided), group resolution, accumulator update. The
    /// per-row paths land here — the scratch row is the one
    /// representation.
    pub(super) fn fold_scratch_row(&mut self) {
        // The measure computation: two-slot read, ray test, one exact
        // subtraction into the parsed spec's derived word.
        // A poisoned sink folds nothing more — the execution's answer is
        // the typed `MeasureOfRay`, and the error path owes no speed.
        if matches!(self.ray, crate::exec::sink::RayPoison::Hit(_)) {
            return;
        }
        for i in 0..self.measures.len() {
            let (derived, slot) = self.measures[i];
            let (start, end) = (self.binding_scratch[slot], self.binding_scratch[slot + 1]);
            let Some(duration) = measure(start, end) else {
                self.ray = crate::exec::sink::RayPoison::Hit([start, end]);
                return;
            };
            self.binding_scratch[derived] = duration;
        }
        // Binding dedup: fold only the first occurrence of each distinct
        // key — unless the elision proved the stream duplicate-free
        // (single-rule: distinct bindings; multi-rule: the rule-
        // disjointness composition, docs/architecture/40-execution.md
        // § set semantics). Single-rule key: the whole slot array, so an
        // interval variable's two words are both hashed (the SlotWidth
        // layout). Multi-rule key: the head projection — rule-independent
        // by construction, so the seen-set spanning rules folds each
        // element of the union exactly once (20-query-ir § aggregation).
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

        if let Some(slot) = self.pack_slot() {
            // One coalescing-fold step: append the claim raw — identical
            // and overlapping claims collapse in the finalize sweep,
            // never here (20-query-ir § aggregation).
            self.pack_claims[group_idx]
                .push([self.binding_scratch[slot], self.binding_scratch[slot + 1]]);
            return; // validated: Pack mixes with no other aggregate
        }

        let mut acc_cursor = 0;
        for find in &self.finds {
            let SinkSpec::Agg(spec) = find else {
                continue;
            };
            let acc = &mut self.accs[group_idx * self.n_aggs + acc_cursor];
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
                            *total += u128::from(word);
                        }
                        (FoldOp::Min, Acc::Min(best)) => {
                            *best = (*best).min(word);
                        }
                        (FoldOp::Max, Acc::Max(best)) => {
                            *best = (*best).max(word);
                        }
                        _ => unreachable!("accumulators are seeded per op"),
                    }
                }
            }
        }
    }
}
