use crate::exec::sink::{Acc, AggregateSink, FoldOp, SinkSpec, measure, word_to_i64};

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
        if self.ray.is_some() {
            return;
        }
        for i in 0..self.measures.len() {
            let (derived, slot) = self.measures[i];
            let (start, end) = (self.binding_scratch[slot], self.binding_scratch[slot + 1]);
            let Some(duration) = measure(start, end) else {
                self.ray = Some([start, end]);
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
        if let Some(seen) = &mut self.seen {
            let key = dedup_key(
                self.union_spans.as_deref(),
                &mut self.union_scratch,
                &self.binding_scratch,
            );
            if !seen.insert(key) {
                return;
            }
        }

        super::groups::load_group_key(&mut self.key_scratch, &self.group_spans, |slot| {
            self.binding_scratch[slot]
        });
        let group_idx = self.probe_group();

        if let Some(slot) = self.pack {
            // One coalescing-fold step: append the claim raw — identical
            // and overlapping claims collapse in the finalize sweep,
            // never here (20-query-ir § aggregation).
            self.pack_claims[group_idx]
                .push([self.binding_scratch[slot], self.binding_scratch[slot + 1]]);
            return; // validated: Pack mixes with no other aggregate
        }

        let mut acc_cursor = 0;
        for find in &self.finds {
            let SinkSpec::Agg {
                op,
                over_slot,
                over_width: _,
                signed,
            } = find
            else {
                continue;
            };
            let acc = &mut self.accs[group_idx * self.n_aggs + acc_cursor];
            acc_cursor += 1;
            match (op, acc) {
                (FoldOp::Count, Acc::Count(n)) => *n += 1,
                (FoldOp::Sum, Acc::SumSigned(total)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Sum has a variable")];
                    debug_assert!(*signed);
                    *total += i128::from(word_to_i64(word));
                }
                (FoldOp::Sum, Acc::SumUnsigned(total)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Sum has a variable")];
                    *total += u128::from(word);
                }
                (FoldOp::Min, Acc::Min(best)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Min has a variable")];
                    *best = (*best).min(word);
                }
                (FoldOp::Max, Acc::Max(best)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Max has a variable")];
                    *best = (*best).max(word);
                }
                _ => unreachable!("accumulators are seeded per op"),
            }
        }
    }
}

/// The binding-dedup key for the row in `binding_scratch`
/// (docs/architecture/40-execution.md § the rule loop): the union
/// spans' gathered words under the multi-rule regime — the head
/// projection for a hand-written rule set, the `VarId`-ordered shared
/// slot arrays for a DNF-derived one (R2) — or the whole slot array
/// verbatim for a single-rule query. Both span shapes are
/// rule-independent: the head is the hand-written rules' only shared
/// vocabulary, and DNF clones share one variable scope.
pub(super) fn dedup_key<'k>(
    union_spans: Option<&[(usize, usize)]>,
    scratch: &'k mut Vec<u64>,
    binding_scratch: &'k [u64],
) -> &'k [u64] {
    match union_spans {
        Some(spans) => {
            scratch.clear();
            for &(slot, width) in spans {
                scratch.extend_from_slice(&binding_scratch[slot..slot + width]);
            }
            scratch
        }
        None => binding_scratch,
    }
}
