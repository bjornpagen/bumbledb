use crate::exec::run::{LeafBatch, LeafSource};
use crate::exec::sink::{Acc, AggregateSink, FoldOp, GroupTable, SinkSpec};
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
    /// Recomputes the leaf-shape classification (outer slots + group
    /// constancy) at batch entry — per-slot work, never per-row.
    pub(super) fn refresh_shape_cache(&mut self, batch: &LeafBatch<'_>) {
        self.cached_outer_slots.clear();
        // Real slots only: the derived measure words past `real_slots`
        // are the sink's own (computed per row in `fold_scratch_row`),
        // never a binding to prefill.
        for slot in 0..self.real_slots {
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
        let (group_idx, inserted) = match &mut self.groups {
            GroupTable::Hashed(map) => {
                let next = map.len();
                let (idx, inserted) = map.get_or_insert_with(&self.key_scratch, || next);
                (*idx, inserted)
            }
            // The dense regime (finding 049): mixed-radix arithmetic —
            // no hash, no ctrl-line probe. The schema proves every
            // committed key word below its radix (closed containment;
            // the strict 0/1 bool encoding), so the index is total over
            // committed data.
            GroupTable::Dense {
                radixes,
                table,
                ordinals,
            } => {
                let mut ordinal = 0usize;
                for (word, radix) in self.key_scratch.iter().zip(radixes.iter()) {
                    debug_assert!(
                        *word < u64::from(*radix),
                        "containment keeps dense words in-domain"
                    );
                    ordinal = ordinal * usize::from(*radix)
                        + usize::try_from(*word).expect("dense words are small");
                }
                let entry = &mut table[ordinal];
                if *entry == 0 {
                    ordinals.push(u32::try_from(ordinal).expect("capped product"));
                    *entry = u32::try_from(ordinals.len()).expect("capped product");
                    (ordinals.len() - 1, true)
                } else {
                    (usize::try_from(*entry - 1).expect("capped product"), false)
                }
            }
        };
        if inserted {
            // Fresh accumulator row, seeded per op (finds copied out —
            // the value-set allocation below takes `&mut self`).
            for i in 0..self.finds.len() {
                let find = self.finds[i];
                match find {
                    SinkSpec::Agg { op, signed, .. } => {
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
                    SinkSpec::Var { .. } | SinkSpec::Arg { .. } | SinkSpec::Pack { .. } => {}
                }
            }
            if self.arg.is_some() {
                self.init_arg_group(group_idx);
            }
            if self.pack.is_some() {
                self.init_pack_group(group_idx);
            }
        }
        group_idx
    }

    /// Takes a value set from the pool (or grows it): allocation order is
    /// (group, `CountDistinct` find), so a reused map's arity always equals
    /// the find's span width (the map's own insert-time arity assert
    /// backs this).
    fn alloc_value_set(&mut self, find: SinkSpec) -> usize {
        let SinkSpec::Agg {
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

    /// Seeds a fresh group's Pack state: an empty claim list, pooled by
    /// group index (capacity retained across executions — the Arg row-set
    /// precedent).
    fn init_pack_group(&mut self, group_idx: usize) {
        if group_idx < self.pack_claims.len() {
            self.pack_claims[group_idx].clear();
        } else {
            self.pack_claims.push(Vec::new());
        }
    }

    /// Seeds a fresh group's Arg state: the identity extreme (any first
    /// key compares equal-or-better against it, so the first binding
    /// always lands) and an empty row set — pooled by group index.
    fn init_arg_group(&mut self, group_idx: usize) {
        debug_assert_eq!(group_idx, self.arg_best.len(), "groups are dense");
        let arg = self.arg.expect("callers check");
        self.arg_best
            .push(if arg.max { u64::MIN } else { u64::MAX });
        if group_idx < self.arg_answers.len() {
            self.arg_answers[group_idx].clear();
        } else {
            self.arg_answers.push(WordMap::new(self.carry_words));
        }
    }
}
