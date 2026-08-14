use crate::exec::run::{LeafBatch, LeafSource};
use crate::exec::sink::{AggSpec, AggregateSink, GroupTable, SinkSpec};

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
    /// seeding a fresh accumulator row (and, per regime, the group's
    /// Pack claim list) on
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
                    SinkSpec::Agg(spec) => {
                        self.accs.push(spec.seed_acc());
                    }
                    SinkSpec::Var { .. } | SinkSpec::Pack { .. } => {}
                }
            }
            if self.pack.is_some() {
                self.init_pack_group(group_idx);
            }
        }
        group_idx
    }

    /// Seeds a fresh group's Pack state: an empty claim list, pooled by
    /// group index (capacity retained across executions).
    fn init_pack_group(&mut self, group_idx: usize) {
        if group_idx < self.pack_claims.len() {
            self.pack_claims[group_idx].clear();
        } else {
            self.pack_claims.push(Vec::new());
        }
    }
}
