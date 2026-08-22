use crate::exec::run::{LeafBatch, LeafSource};
use crate::exec::sink::{AggregateSink, GroupState, GroupTable, SinkSpec};

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
                    for i in 0..self.finds.len() {
                        if let SinkSpec::Agg(spec) = self.finds[i] {
                            accs.push(spec.seed_acc());
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
