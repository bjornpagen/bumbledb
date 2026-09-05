use crate::exec::run::{LeafBatch, LeafSource};
use crate::exec::sink::{Acc, AggSpec, AggregateSink, GroupState, GroupTable, SinkSpec};

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
    /// Check before accumulation: for at most u64::MAX inputs all I64/U64
    /// exact totals fit the existing i128/u128 hot-path accumulators.
    pub(super) fn advance_group(&mut self, group: usize, count: u64) -> bool {
        let Some(total) = self.group_counts[group].checked_add(count) else {
            self.cardinality_overflow = true;
            return false;
        };
        self.group_counts[group] = total;
        true
    }

    pub(super) fn refresh_shape_cache(&mut self, batch: &LeafBatch<'_>) {
        self.cached_outer_slots.clear();

        for slot in 0..self.real_slots {
            if matches!(batch.source_of(slot), LeafSource::Outer) {
                self.cached_outer_slots.push(slot);
            }
        }

        self.cached_constant_group = self.group_spans.iter().all(|(slot, width)| {
            (*slot..slot + width).all(|word| matches!(batch.source_of(word), LeafSource::Outer))
        });
    }

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
            match &mut self.group_state {
                GroupState::Folds { accs, .. } => {
                    self.group_counts.push(0);
                    let first_acc = accs.len();
                    for i in 0..self.finds.len() {
                        if let SinkSpec::Agg(spec) = self.finds[i] {
                            if let AggSpec::Float { slot, .. } = spec {
                                let alias = self.share_float_inputs.then(|| {
                                    self.finds[..i].iter().filter_map(|find| {
                                        if let SinkSpec::Agg(spec) = find { Some(spec) } else { None }
                                    }).position(|previous| matches!(previous, AggSpec::Float { slot: previous, .. } if *previous == slot))
                                }).flatten();
                                let (index, primary) = if let Some(alias) = alias {
                                    let Acc::Float { index, .. } = accs[first_acc + alias] else {
                                        unreachable!("alias references an earlier float accumulator")
                                    };
                                    (index, false)
                                } else {
                                    let index = self.float_accs.len();
                                    self.float_accs.push(Default::default());
                                    (index, true)
                                };
                                accs.push(Acc::Float { index, primary });
                            } else {
                                accs.push(spec.seed_acc());
                            }
                        }
                    }
                }
                GroupState::Pack { claims, .. } => {
                    if group_idx < claims.len() {
                        claims[group_idx].clear();
                    } else {
                        claims.push(Vec::new());
                    }
                }
            }
        }
        group_idx
    }
}
