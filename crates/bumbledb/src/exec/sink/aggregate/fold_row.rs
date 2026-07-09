use crate::exec::sink::{word_to_i64, Acc, AggregateSink, ArgSpec, FindSpec, FoldOp};

impl AggregateSink {
    /// Folds the full binding currently in `binding_scratch`: dedup
    /// (unless elided), group resolution, accumulator update. The
    /// per-row paths land here — the scratch row is the one
    /// representation.
    pub(super) fn fold_scratch_row(&mut self) {
        // Binding dedup: fold only the first occurrence of each distinct
        // full binding — unless the plan proved distinctness (elision).
        // The key is the whole slot array, so an interval variable's two
        // words are both hashed (the SlotWidth layout).
        if let Some(seen) = &mut self.seen {
            if !seen.insert(&self.binding_scratch) {
                return;
            }
        }

        super::groups::load_group_key(&mut self.key_scratch, &self.group_spans, |slot| {
            self.binding_scratch[slot]
        });
        let group_idx = self.probe_group();

        if let Some(arg) = self.arg {
            self.fold_arg(group_idx, arg);
            return; // validated: Arg terms and folds never mix
        }

        let mut acc_cursor = 0;
        for find in &self.finds {
            let FindSpec::Agg {
                op,
                over_slot,
                over_width,
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
                (FoldOp::CountDistinct, Acc::CountDistinct(set)) => {
                    // The value's 1–2 word span inserts into the group's
                    // word-set — the projection-dedup mechanism scoped
                    // per group. The binding-level dedup above (or its
                    // elision) stays correct beneath this: distinct
                    // bindings ⊇ distinct values, so dropping duplicate
                    // *bindings* can never drop a value this set doesn't
                    // already hold — the value set dedups the rest down
                    // to value identity (20-query-ir § aggregation).
                    let slot = over_slot.expect("validated: CountDistinct has a variable");
                    self.value_sets[*set].insert(&self.binding_scratch[slot..slot + over_width]);
                }
                _ => unreachable!("accumulators are seeded per op"),
            }
        }
    }

    /// One binding's Arg-restriction step (20-query-ir § aggregation,
    /// restrict-then-project): the group keeps the bindings attaining the
    /// key's extreme and the rows projected from them — whole, never
    /// per-term, which is what makes multi-carry coherence automatic.
    fn fold_arg(&mut self, group_idx: usize, arg: ArgSpec) {
        // Encoded words compare correctly unsigned for both orderable
        // key types: a U64 word is the value itself, an I64 word is the
        // sign-flipped biased form — both order-preserving encodings
        // (docs/architecture/30-execution.md).
        let key = self.binding_scratch[arg.key_slot];
        let best = self.arg_best[group_idx];
        let better = if arg.max { key > best } else { key < best };
        if !better && key != best {
            return; // worse: this binding cannot attain the extreme
        }
        // Project the row whole from this surviving binding: every Arg
        // carry's slot span, in find order (restrict-then-project — all
        // carried values come from the same binding).
        self.carry_scratch.clear();
        for find in &self.finds {
            if let FindSpec::Arg { slot, width, .. } = find {
                self.carry_scratch
                    .extend_from_slice(&self.binding_scratch[*slot..slot + width]);
            }
        }
        let rows = &mut self.arg_rows[group_idx];
        if better {
            // Strictly better: the previous extreme's rows are no longer
            // attained — clear, then store this row.
            self.arg_best[group_idx] = key;
            rows.clear();
        }
        // Equal (or the strictly-better first row): push with row-level
        // dedup — ties are set-honest, and two DISTINCT bindings may
        // project EQUAL rows (the answer is a set of rows, so they
        // collapse; this dedup is never elided — it is not the binding
        // seen-set).
        rows.insert(&self.carry_scratch);
    }
}
