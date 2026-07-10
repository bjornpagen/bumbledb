use crate::exec::sink::{AggregateSink, ArgSpec, FindSpec, FoldOp};
use crate::exec::wordmap::WordMap;

impl AggregateSink {
    /// Builds the sink. `slot_count` is the plan's binding-slot count in
    /// **words** (an interval variable holds two — the `SlotWidth` layout);
    /// `distinct_bindings` is the plan's elision flag (30-execution): when
    /// set, the seen-set is skipped entirely; `union` is the multi-rule
    /// regime (the seen-set keys head projections and spans rules).
    /// Unhinted construction (tests; production sinks are hint-sized).
    #[cfg(test)]
    #[must_use]
    pub fn new(finds: Vec<FindSpec>, slot_count: usize, distinct_bindings: bool) -> Self {
        Self::with_capacity_hint(finds, slot_count, distinct_bindings, false, 0)
    }

    /// Presized construction: the dedup seen-set
    /// takes the plan's output estimate; the group map takes a small
    /// clamp of it (groups are few — the estimate bounds bindings, not
    /// groups).
    ///
    /// Dedup is **per-query-shape**: a single-rule sink (`union` false)
    /// keys the whole slot array and elides it under the plan's
    /// distinct-bindings proof; a multi-rule sink keys the **head
    /// projection** — rule-independent by construction — and KEEPS the
    /// seen-set unconditionally. ALG 08's composition point: rules
    /// provably pairwise-disjoint and each internally distinct ⇒ the
    /// union seen-set elides here too — correct first, elided when
    /// proven.
    #[must_use]
    pub fn with_capacity_hint(
        finds: Vec<FindSpec>,
        slot_count: usize,
        distinct_bindings: bool,
        union: bool,
        hint: usize,
    ) -> Self {
        let group_spans: Vec<(usize, usize)> = finds
            .iter()
            .filter_map(|f| match f {
                FindSpec::Var { slot, width } => Some((*slot, *width)),
                FindSpec::Agg { .. } | FindSpec::Arg { .. } => None,
            })
            .collect();
        let key_words: usize = group_spans.iter().map(|(_, width)| width).sum();
        let n_aggs = finds
            .iter()
            .filter(|f| matches!(f, FindSpec::Agg { .. }))
            .count();
        let carry_words: usize = finds
            .iter()
            .filter_map(|f| match f {
                FindSpec::Arg { width, .. } => Some(*width),
                FindSpec::Var { .. } | FindSpec::Agg { .. } => None,
            })
            .sum();
        // Validation guarantees every Arg term names one key and one
        // direction (20-query-ir § aggregation), so the first spec is
        // THE spec.
        let arg = finds.iter().find_map(|f| match f {
            FindSpec::Arg { key_slot, max, .. } => Some(ArgSpec {
                key_slot: *key_slot,
                max: *max,
            }),
            FindSpec::Var { .. } | FindSpec::Agg { .. } => None,
        });
        debug_assert!(
            finds.iter().all(|f| match f {
                FindSpec::Arg { key_slot, max, .. } =>
                    arg.is_some_and(|spec| spec.key_slot == *key_slot && spec.max == *max),
                FindSpec::Var { .. } | FindSpec::Agg { .. } => true,
            }),
            "validated: all Arg terms share one key and one direction"
        );
        let row_fold_only = arg.is_some()
            || finds.iter().any(|f| {
                matches!(
                    f,
                    FindSpec::Agg {
                        op: FoldOp::CountDistinct,
                        ..
                    }
                )
            });
        let union_spans = union.then(|| union_key_spans(&finds));
        let union_words: usize = union_spans
            .as_ref()
            .map_or(0, |spans| spans.iter().map(|(_, width)| width).sum());
        debug_assert!(
            !(union && arg.is_some()),
            "validated: Arg-restriction never crosses rules"
        );
        Self {
            groups: WordMap::with_capacity_hint(key_words, hint.min(4096)),
            key_scratch: vec![0; key_words],
            binding_scratch: vec![0; slot_count],
            // Single-rule: whole-binding key, elidable by the proof.
            // Multi-rule: head-projection key, never elided (ALG 08).
            seen: if union {
                Some(WordMap::with_capacity_hint(union_words, hint))
            } else {
                (!distinct_bindings).then(|| WordMap::with_capacity_hint(slot_count, hint))
            },
            union_scratch: vec![0; union_words],
            union_spans,
            acc_scratch: Vec::with_capacity(n_aggs),
            dedup_survivors: Vec::new(),
            scan_sources: Vec::with_capacity(n_aggs),
            scan_count: 0,
            cached_outer_slots: Vec::new(),
            cached_constant_group: false,
            value_sets: Vec::new(),
            value_sets_live: 0,
            arg,
            arg_best: Vec::new(),
            arg_rows: Vec::new(),
            carry_words,
            carry_scratch: Vec::with_capacity(carry_words),
            row_fold_only,
            #[cfg(test)]
            group_probes: 0,
            group_spans,
            finds,
            accs: Vec::new(),
            n_aggs,
        }
    }

    /// Re-aims the slot tables at one rule's binding layout (the rule
    /// loop, docs/architecture/40-execution.md): the head positions are
    /// fixed — arity, ops, widths, types — but each rule supplies its own
    /// slots, so every slot-addressed table rebuilds in place (capacities
    /// retained; the shared maps — groups, seen, value sets — carry
    /// across rules untouched: the spanning is the union). Single-rule
    /// sinks are built aimed and never call this.
    pub fn aim(&mut self, finds: &[FindSpec], slot_count: usize) {
        debug_assert_eq!(finds.len(), self.finds.len(), "one head, fixed arity");
        self.finds.clear();
        self.finds.extend_from_slice(finds);
        self.group_spans.clear();
        self.group_spans
            .extend(finds.iter().filter_map(|f| match f {
                FindSpec::Var { slot, width } => Some((*slot, *width)),
                FindSpec::Agg { .. } | FindSpec::Arg { .. } => None,
            }));
        if let Some(spans) = &mut self.union_spans {
            spans.clear();
            spans.extend(finds.iter().filter_map(union_span));
        }
        debug_assert!(
            self.arg.is_none() && !finds.iter().any(|f| matches!(f, FindSpec::Arg { .. })),
            "validated: Arg-restriction never crosses rules"
        );
        self.binding_scratch.clear();
        self.binding_scratch.resize(slot_count, 0);
    }

    /// Groups held (finalize's reservation).
    #[must_use]
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// Distinct bindings the seen-set holds — the union observable the
    /// rule loop reads per rule (absorbed = emitted − newly-seen).
    /// `None` when the seen-set is elided (the single-rule
    /// distinct-bindings proof: every emit is first-seen).
    #[must_use]
    pub fn distinct_seen(&self) -> Option<usize> {
        self.seen.as_ref().map(WordMap::len)
    }

    /// Whether the binding seen-set is elided (the plan proved distinct
    /// bindings) — the elision observable. `CountDistinct`'s value sets and
    /// the Arg row-dedup are different sets and are NEVER elided.
    #[cfg(test)]
    #[must_use]
    pub fn seen_elided(&self) -> bool {
        self.seen.is_none()
    }

    /// Distinct values held across every live `CountDistinct` set — the
    /// value-dedup observable the elision fixture asserts alongside
    /// [`Self::seen_elided`].
    #[cfg(test)]
    #[must_use]
    pub fn distinct_values_held(&self) -> usize {
        self.value_sets[..self.value_sets_live]
            .iter()
            .map(WordMap::len)
            .sum()
    }

    /// Empties the sink for the next execution, retaining capacity —
    /// value sets and Arg row sets stay pooled (cleared on reuse at
    /// group creation, never dropped). Called once per execution, never
    /// per rule: the seen-set spanning rules IS the union
    /// (docs/architecture/40-execution.md § the rule loop).
    pub fn reset(&mut self) {
        self.groups.clear();
        self.accs.clear();
        self.value_sets_live = 0;
        self.arg_best.clear();
        if let Some(seen) = &mut self.seen {
            seen.clear();
        }
    }
}

/// One head position's contribution to the union dedup key: the slot
/// span the position reads — a group variable's span or a fold input's
/// span; the nullary `Count` reads nothing and contributes nothing (the
/// naive model's "constant filler", represented as absence). Arg terms
/// are unreachable here (validation refuses Arg-restriction across
/// rules — 20-query-ir § aggregation).
fn union_span(find: &FindSpec) -> Option<(usize, usize)> {
    match find {
        FindSpec::Var { slot, width } => Some((*slot, *width)),
        FindSpec::Agg {
            over_slot: Some(slot),
            over_width,
            ..
        } => Some((*slot, *over_width)),
        FindSpec::Agg {
            over_slot: None, ..
        } => None,
        FindSpec::Arg { .. } => unreachable!("validated: no Arg across rules"),
    }
}

/// The multi-rule dedup-key spans over one rule's binding layout — the
/// head projection, position by position.
fn union_key_spans(finds: &[FindSpec]) -> Vec<(usize, usize)> {
    finds.iter().filter_map(union_span).collect()
}
