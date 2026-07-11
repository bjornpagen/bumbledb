use crate::exec::sink::{measure, word_to_i64, Acc, AggregateSink, ArgSpec, FindSpec, FoldOp};

impl AggregateSink {
    /// Folds the full binding currently in `binding_scratch`: the
    /// measure words first (the derived-slot rewrite's one computation
    /// site — a ray poisons the sink and the row is dropped), then dedup
    /// (unless elided), group resolution, accumulator update. The
    /// per-row paths land here — the scratch row is the one
    /// representation.
    pub(super) fn fold_scratch_row(&mut self) {
        // The measure computation: two-slot read, ray test, one exact
        // subtraction into the derived word (see `FindSpec::Duration`).
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

        if let Some(arg) = self.arg {
            self.fold_arg(group_idx, arg);
            return; // validated: Arg terms and folds never mix
        }

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
                    // The value's 1–8 word span inserts into the group's
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
        // (docs/architecture/40-execution.md).
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

/// The binding-dedup key for the row in `binding_scratch`
/// (docs/architecture/40-execution.md § the rule loop): the head
/// projection under the multi-rule union regime — the words each head
/// position reads, gathered in head order into `scratch` — or the whole
/// slot array for a single-rule program. Head projections are
/// rule-independent by construction; full slot arrays are not, which is
/// why the union regime never keys them.
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
