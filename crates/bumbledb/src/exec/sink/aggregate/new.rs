use crate::exec::sink::{
    AggSpec, AggregateSink, DENSE_GROUPS_CAP, DedupState, FindSpec, GroupState, GroupTable,
    SinkSpec,
};
use crate::exec::wordmap::WordMap;

pub(in crate::exec::sink) fn parse_finds(finds: &[FindSpec], slot_count: usize) -> Vec<SinkSpec> {
    let mut parsed = Vec::with_capacity(finds.len());
    parse_finds_into(finds, slot_count, &mut parsed);
    parsed
}

pub(in crate::exec::sink) fn parse_finds_into(
    finds: &[FindSpec],
    _slot_count: usize,
    parsed: &mut Vec<SinkSpec>,
) {
    parsed.clear();
    for find in finds {
        let spec = match *find {
            FindSpec::Var { slot, width } => SinkSpec::Var { slot, width },
            FindSpec::Agg(spec) => SinkSpec::Agg(spec),
            FindSpec::Pack { slot } => SinkSpec::Pack { slot },
        };
        parsed.push(spec);
    }
}

fn pack_slot(finds: &[SinkSpec]) -> Option<usize> {
    let mut packs = finds.iter().filter_map(|f| match f {
        SinkSpec::Pack { slot } => Some(*slot),
        _ => None,
    });
    let slot = packs.next();
    debug_assert!(packs.next().is_none(), "validated: at most one Pack");
    slot
}

impl AggregateSink {
    #[cfg(test)]
    #[must_use]
    pub fn new(finds: impl AsRef<[FindSpec]>, slot_count: usize) -> Self {
        Self::build(finds.as_ref(), slot_count, DedupRegime::Bindings, 0, &[])
    }

    #[cfg(test)]
    #[must_use]
    pub fn new_dense(
        finds: impl AsRef<[FindSpec]>,
        slot_count: usize,
        dense_groups: &[u16],
    ) -> Self {
        Self::build(
            finds.as_ref(),
            slot_count,
            DedupRegime::Bindings,
            0,
            dense_groups,
        )
    }

    #[cfg(test)]
    #[must_use]
    pub fn new_distinct(
        finds: impl AsRef<[FindSpec]>,
        slot_count: usize,
        witness: crate::plan::fj::DistinctWitness,
    ) -> Self {
        Self::build(
            finds.as_ref(),
            slot_count,
            DedupRegime::Elided(witness),
            0,
            &[],
        )
    }

    #[must_use]
    pub fn with_capacity_hint(
        finds: &[FindSpec],
        slot_count: usize,
        hint: usize,
        dense_groups: &[u16],
    ) -> Self {
        Self::build(finds, slot_count, DedupRegime::Bindings, hint, dense_groups)
    }

    #[must_use]
    pub fn for_union(finds: &[FindSpec], slot_count: usize, hint: usize) -> Self {
        Self::build(finds, slot_count, DedupRegime::Union, hint, &[])
    }

    /// 2026-07-23, R2): the union seen-set re-keys on the **shared slot
    /// law, `lean/Bumbledb/Exec/Dedup.lean: dnf_rekey_transparent`).
    #[must_use]
    pub fn for_dnf_union(
        finds: &[FindSpec],
        slot_count: usize,
        spans: &[(usize, usize)],
        hint: usize,
    ) -> Self {
        Self::build(finds, slot_count, DedupRegime::DnfUnion(spans), hint, &[])
    }

    #[must_use]
    pub fn without_seen_set(
        finds: &[FindSpec],
        slot_count: usize,
        witness: crate::plan::fj::DistinctWitness,
        hint: usize,
        dense_groups: &[u16],
    ) -> Self {
        Self::build(
            finds,
            slot_count,
            DedupRegime::Elided(witness),
            hint,
            dense_groups,
        )
    }

    fn build(
        finds: &[FindSpec],
        slot_count: usize,
        regime: DedupRegime<'_>,
        hint: usize,
        dense_groups: &[u16],
    ) -> Self {
        let finds = parse_finds(finds, slot_count);
        let scratch_words = slot_count;
        let group_spans: Vec<(usize, usize)> = finds
            .iter()
            .filter_map(|f| match f {
                SinkSpec::Var { slot, width } => Some((*slot, *width)),
                SinkSpec::Agg(_) | SinkSpec::Pack { .. } => None,
            })
            .collect();
        let key_words: usize = group_spans.iter().map(|(_, width)| width).sum();
        let n_aggs = finds
            .iter()
            .filter(|f| matches!(f, SinkSpec::Agg(_)))
            .count();

        let (dedup, union_words) = match regime {
            DedupRegime::Bindings => (
                DedupState::Bindings {
                    seen: WordMap::with_capacity_hint(scratch_words, hint),
                },
                0,
            ),
            DedupRegime::Union => {
                let spans = union_key_spans(&finds);
                let words: usize = spans.iter().map(|(_, width)| width).sum();
                (
                    DedupState::Union {
                        seen: WordMap::with_capacity_hint(words, hint),
                        spans,
                    },
                    words,
                )
            }
            DedupRegime::DnfUnion(spans) => {
                let spans = spans.to_vec();
                let words: usize = spans.iter().map(|(_, width)| width).sum();
                (
                    DedupState::DnfUnion {
                        seen: WordMap::with_capacity_hint(words, hint),
                        spans,
                    },
                    words,
                )
            }
            DedupRegime::Elided(witness) => (DedupState::Elided { witness }, 0),
        };

        let groups = if dense_groups.is_empty() {
            GroupTable::Hashed(WordMap::with_capacity_hint(key_words, hint.min(4096)))
        } else {
            debug_assert_eq!(
                dense_groups.len(),
                key_words,
                "one radix per group-key word"
            );
            let product: u32 = dense_groups.iter().map(|radix| u32::from(*radix)).product();
            debug_assert!(
                0 < product && product <= DENSE_GROUPS_CAP,
                "the caller caps the dense product"
            );
            GroupTable::Dense {
                radixes: dense_groups.into(),
                table: vec![0; usize::try_from(product).expect("capped")].into_boxed_slice(),
                ordinals: Vec::new(),
            }
        };
        let group_state = if let Some(slot) = pack_slot(&finds) {
            GroupState::Pack {
                slot,
                claims: Vec::new(),
            }
        } else {
            GroupState::Folds {
                accs: Vec::new(),
                n_aggs,
            }
        };
        Self {
            dedup,
            groups,
            key_scratch: vec![0; key_words],
            binding_scratch: vec![0; scratch_words],
            union_scratch: vec![0; union_words],
            acc_scratch: Vec::with_capacity(n_aggs),
            dedup_survivors: Vec::new(),
            scan_sources: Vec::with_capacity(n_aggs),
            scan_count: 0,
            cached_outer_slots: Vec::new(),
            cached_constant_group: false,
            #[cfg(test)]
            group_probes: 0,
            group_spans,
            finds,
            real_slots: slot_count,
            group_state,
        }
    }

    pub fn aim(&mut self, finds: &[FindSpec], slot_count: usize, shared_slots: &[(usize, usize)]) {
        debug_assert_eq!(finds.len(), self.finds.len(), "one head, fixed arity");

        parse_finds_into(finds, slot_count, &mut self.finds);
        self.real_slots = slot_count;
        self.group_spans.clear();
        self.group_spans
            .extend(self.finds.iter().filter_map(|f| match f {
                SinkSpec::Var { slot, width } => Some((*slot, *width)),
                SinkSpec::Agg(_) | SinkSpec::Pack { .. } => None,
            }));
        match &mut self.dedup {
            DedupState::DnfUnion { spans, .. } => {
                spans.clear();
                spans.extend_from_slice(shared_slots);
            }
            DedupState::Union { spans, .. } => {
                spans.clear();
                spans.extend(self.finds.iter().filter_map(union_span));
            }
            DedupState::Bindings { .. } | DedupState::Elided { .. } => {}
        }
        self.binding_scratch.clear();
        self.binding_scratch.resize(slot_count, 0);
        if let GroupState::Pack { slot, .. } = &mut self.group_state {
            *slot = pack_slot(&self.finds).expect("Pack heads stay Pack across rules");
        }
    }

    #[must_use]
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    #[must_use]
    pub fn distinct_seen(&self) -> Option<usize> {
        self.dedup.seen().map(WordMap::len)
    }

    #[cfg(test)]
    #[must_use]
    pub fn seen_elided(&self) -> bool {
        matches!(self.dedup, DedupState::Elided { .. })
    }

    #[cfg(test)]
    #[must_use]
    pub fn dense_group_table(&self) -> bool {
        matches!(self.groups, GroupTable::Dense { .. })
    }

    pub fn reset(&mut self) {
        self.groups.clear();
        if let GroupState::Folds { accs, .. } = &mut self.group_state {
            accs.clear();
        }
        if let Some(seen) = self.dedup.seen_mut() {
            seen.clear();
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DedupRegime<'k> {
    Bindings,

    Union,

    DnfUnion(&'k [(usize, usize)]),

    Elided(crate::plan::fj::DistinctWitness),
}

fn union_span(find: &SinkSpec) -> Option<(usize, usize)> {
    match find {
        SinkSpec::Var { slot, width } | SinkSpec::Agg(AggSpec::Fold { slot, width, .. }) => {
            Some((*slot, *width))
        }
        SinkSpec::Pack { slot } => Some((*slot, 2)),
        SinkSpec::Agg(AggSpec::Count) => None,
    }
}

fn union_key_spans(finds: &[SinkSpec]) -> Vec<(usize, usize)> {
    finds.iter().filter_map(union_span).collect()
}
