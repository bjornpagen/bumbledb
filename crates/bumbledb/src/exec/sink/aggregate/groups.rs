use crate::exec::run::{LeafBatch, LeafSource};
use crate::exec::sink::{Acc, AggregateSink, FindSpec, FoldOp};
use crate::exec::wordmap::WordMap;

/// Loads a group key, span-wise (the `SlotWidth` layout): each group
/// variable contributes its full word span — never a bare width-1 read.
/// A free function so callers can borrow the scratch and the slot reader
/// from disjoint sink fields.
pub(super) fn load_group_key(
    key_scratch: &mut [u64],
    group_spans: &[(usize, usize)],
    get: impl Fn(usize) -> u64,
) {
    let mut word = 0;
    for (slot, width) in group_spans {
        for offset in 0..*width {
            key_scratch[word] = get(slot + offset);
            word += 1;
        }
    }
}

impl AggregateSink {
    /// Refreshes the leaf-shape cache (outer slots + group constancy) —
    /// pointer-keyed on `key_slots`, so pinned batch-of-one leaves pay
    /// nothing after the first batch.
    pub(super) fn refresh_shape_cache(&mut self, batch: &LeafBatch<'_>) {
        self.cached_outer_slots.clear();
        for slot in 0..self.binding_scratch.len() {
            if matches!(batch.source_of(slot), LeafSource::Outer) {
                self.cached_outer_slots.push(slot);
            }
        }
        // Every word of every group span outer — spans, never a bare
        // slot: an interval group variable is constant only if both its
        // words are.
        self.cached_constant_group = self.group_spans.iter().all(|(slot, width)| {
            (*slot..slot + width).all(|word| matches!(batch.source_of(word), LeafSource::Outer))
        });
    }

    /// Probes the group map with the key currently in `key_scratch`,
    /// seeding a fresh accumulator row (and, per regime, a `CountDistinct`
    /// value set per `CountDistinct` find or the group's Arg state) on
    /// first sight. The one place a group probe happens — the batch path
    /// memoizes around it.
    pub(super) fn probe_group(&mut self) -> usize {
        #[cfg(test)]
        {
            self.group_probes += 1;
        }
        let next = self.groups.len();
        let (idx, inserted) = self.groups.get_or_insert_with(&self.key_scratch, || next);
        let group_idx = *idx;
        if inserted {
            // Fresh accumulator row, seeded per op (finds copied out —
            // the value-set allocation below takes `&mut self`).
            for i in 0..self.finds.len() {
                let find = self.finds[i];
                match find {
                    FindSpec::Agg { op, signed, .. } => {
                        let acc = match (op, signed) {
                            (FoldOp::Sum, true) => Acc::SumSigned(0),
                            (FoldOp::Sum, false) => Acc::SumUnsigned(0),
                            (FoldOp::Min, _) => Acc::Min(u64::MAX),
                            (FoldOp::Max, _) => Acc::Max(u64::MIN),
                            (FoldOp::Count, _) => Acc::Count(0),
                            (FoldOp::CountDistinct, _) => {
                                Acc::CountDistinct(self.alloc_value_set(find))
                            }
                        };
                        self.accs.push(acc);
                    }
                    FindSpec::Var { .. } | FindSpec::Arg { .. } => {}
                }
            }
            if self.arg.is_some() {
                self.seed_arg_group(group_idx);
            }
        }
        group_idx
    }

    /// Takes a value set from the pool (or grows it): allocation order is
    /// (group, `CountDistinct` find), so a reused map's arity always equals
    /// the find's span width (the map's own insert-time arity assert
    /// backs this).
    fn alloc_value_set(&mut self, find: FindSpec) -> usize {
        let FindSpec::Agg {
            over_slot,
            over_width,
            ..
        } = find
        else {
            unreachable!("callers pass CountDistinct finds")
        };
        debug_assert!(
            over_slot.is_some(),
            "validated: CountDistinct has a variable"
        );
        let idx = self.value_sets_live;
        if idx < self.value_sets.len() {
            self.value_sets[idx].clear();
        } else {
            self.value_sets.push(WordMap::new(over_width));
        }
        self.value_sets_live += 1;
        idx
    }

    /// Seeds a fresh group's Arg state: the identity extreme (any first
    /// key compares equal-or-better against it, so the first binding
    /// always lands) and an empty row set — pooled by group index.
    fn seed_arg_group(&mut self, group_idx: usize) {
        debug_assert_eq!(group_idx, self.arg_best.len(), "groups are dense");
        let arg = self.arg.expect("callers check");
        self.arg_best
            .push(if arg.max { u64::MIN } else { u64::MAX });
        if group_idx < self.arg_rows.len() {
            self.arg_rows[group_idx].clear();
        } else {
            self.arg_rows.push(WordMap::new(self.carry_words));
        }
    }
}
