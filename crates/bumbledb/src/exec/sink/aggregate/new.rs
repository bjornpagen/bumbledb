use crate::exec::sink::{AggregateSink, FindSpec};
use crate::exec::wordmap::WordMap;

impl AggregateSink {
    /// Builds the sink. `slot_count` is the plan's binding-slot count;
    /// `distinct_bindings` is the plan's elision flag (30-execution): when
    /// set, the seen-set is skipped entirely.
    /// Unhinted construction (tests; production sinks are hint-sized).
    #[cfg(test)]
    #[must_use]
    pub fn new(finds: Vec<FindSpec>, slot_count: usize, distinct_bindings: bool) -> Self {
        Self::with_capacity_hint(finds, slot_count, distinct_bindings, 0)
    }

    /// Presized construction (docs/perf/ PRD 06): the dedup seen-set
    /// takes the plan's output estimate; the group map takes a small
    /// clamp of it (groups are few — the estimate bounds bindings, not
    /// groups).
    #[must_use]
    pub fn with_capacity_hint(
        finds: Vec<FindSpec>,
        slot_count: usize,
        distinct_bindings: bool,
        hint: usize,
    ) -> Self {
        let group_slots: Vec<usize> = finds
            .iter()
            .filter_map(|f| match f {
                FindSpec::Var { slot } => Some(*slot),
                FindSpec::Agg { .. } => None,
            })
            .collect();
        let n_aggs = finds.len() - group_slots.len();
        Self {
            groups: WordMap::with_capacity_hint(group_slots.len(), hint.min(4096)),
            key_scratch: vec![0; group_slots.len()],
            binding_scratch: vec![0; slot_count],
            seen: (!distinct_bindings).then(|| WordMap::with_capacity_hint(slot_count, hint)),
            acc_scratch: Vec::with_capacity(n_aggs),
            dedup_survivors: Vec::new(),
            scan_sources: Vec::with_capacity(n_aggs),
            scan_count: 0,
            cached_outer_slots: Vec::new(),
            cached_constant_group: false,
            #[cfg(test)]
            group_probes: 0,
            group_slots,
            finds,
            accs: Vec::new(),
            n_aggs,
        }
    }

    /// Groups held (finalize's reservation, docs/perf/ PRD 08).
    #[must_use]
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// Empties the sink for the next execution, retaining capacity.
    pub fn reset(&mut self) {
        self.groups.clear();
        self.accs.clear();
        if let Some(seen) = &mut self.seen {
            seen.clear();
        }
    }
}
